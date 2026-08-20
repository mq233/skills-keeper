//! Rust 核心引擎门面：组合各模块，供命令层调用。
//!
//! 分层规则：engine 不依赖 Tauri，可纯单元测试（`docs/technical-plan.md` §2）。
//! 工具端路径与接入判定由注册表（`AdapterRegistry`）提供，引擎只读——
//! S5 设置页用户配置覆盖适配器路径的天然扩展点。

pub mod deploy;
pub mod error;
pub mod import;
pub mod rollback;
pub mod scanner;
pub mod snapshot;
pub mod status;
pub mod target;
pub mod vault;

use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

use crate::db::{migrations, Db};

use error::{EngineError, EngineResult};
use scanner::ScanResult;
use status::Status;
use target::{AdapterRegistry, ToolId};
use vault::SkillEntry;

pub use deploy::{DeployFailedItem, DeployOkItem, DeployResult};

/// 初始化数据库：打开连接 + 执行全部迁移（S1 三表，`docs/specs/s1-matrix.md` §7）。
/// 命令层 setup 传入 db 文件路径；引擎不依赖 Tauri。
///
/// 首次运行：应用数据目录可能不存在，先创建（Vault 是用户数据不自动创建——S1 只读）。
pub fn init_db(db_path: &Path) -> EngineResult<Db> {
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| EngineError::Io(format!("创建数据目录失败：{e}")))?;
        }
    }
    let db = Db::open(db_path).map_err(|e| EngineError::Io(format!("打开数据库失败：{e}")))?;
    {
        let mut conn = db.lock();
        migrations::migrate(&mut conn)
            .map_err(|e| EngineError::Internal(format!("数据库迁移失败：{e}")))?;
    }
    Ok(db)
}

/// Skill 列表（契约 `list_skills` → `SkillEntry[]`；S1 前端不调用，S4 导入器使用）。
/// Sidecar 默认 targets 由注册表已接入工具合成（S2 适配器化）。
pub fn list_skills(vault_root: &Path, registry: &AdapterRegistry) -> EngineResult<Vec<SkillEntry>> {
    vault::list_skills(vault_root, &registry.all_connected_targets())
}

/// 状态矩阵（契约 `get_status_matrix` / `scan` 同形状）：
/// 读取 Vault → 逐工具扫描（未接入跳过）→ deploy_records 比对 → 判定矩阵全分支。
/// 工具端路径与接入判定来自注册表（S2 适配器化；S5 设置页用户配置覆盖适配器路径）。
pub fn get_status_matrix(
    vault_root: &Path,
    registry: &AdapterRegistry,
    db: &Db,
) -> EngineResult<StatusMatrix> {
    let entries = vault::list_skills(vault_root, &registry.all_connected_targets())?;
    let tool_roots = registry.tool_roots();

    // 逐工具扫描（S1 全量；v 惰性计算为优化意图，spec §8 从简）
    let mut scans: HashMap<&'static str, Option<ScanResult>> = HashMap::new();
    for (id, root) in &tool_roots {
        let scan = match root {
            Some(dir) => scanner::scan_tool(dir)?,
            None => None, // 未接入：不扫描
        };
        scans.insert(id.as_str(), scan);
    }

    // deploy_records 分发记录（S1 恒空；读取链路走通，S2 分发后填入）
    let records = load_deploy_records(&db.lock())?;

    let tools: Vec<ToolInfo> = tool_roots
        .iter()
        .map(|(id, root)| ToolInfo {
            id: id.as_str().to_string(),
            connected: root.is_some(),
        })
        .collect();

    let mut rows = Vec::with_capacity(entries.len());
    for entry in entries {
        let slug = entry.skill.slug.clone();
        // v 计算：任一已接入工具端存在该 Skill（t 非 None）时全量计算，否则跳过
        let need_v = scans.iter().any(|(id, scan)| {
            tools.iter().any(|t| t.id == *id && t.connected)
                && scan
                    .as_ref()
                    .is_some_and(|s| s.skills.iter().any(|sk| sk.slug == slug))
        });
        // v 按工具计算 = 渲染产物 hash（S2 判定基准变更：render 注入/剥离会改写内容，
        // 用 Vault 原始 hash 则 t == r 时 v != r 恒成立 → 分发后永远「待分发」）。
        // 口径与落盘后实际 hash（hash_dir）一致：SKILL.md 文本 + 资源文件（相对路径）。
        let mut v_by_tool: HashMap<&'static str, String> = HashMap::new();
        if need_v {
            let vault_skill_dir = vault_root.join("skills").join(&slug);
            let mut resources: Vec<(String, String)> = Vec::new();
            for rel in &entry.skill.resources {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                let content = std::fs::read(vault_skill_dir.join(rel))
                    .map_err(|e| EngineError::Io(format!("读取资源文件失败：{e}")))?;
                resources.push((rel_str, blake3::hash(&content).to_hex().to_string()));
            }
            for tool in &tools {
                if !tool.connected {
                    continue;
                }
                let Some(adapter) = registry.get(tool.id.parse().unwrap_or(ToolId::Workbuddy))
                else {
                    continue;
                };
                if let Ok(rendered) = adapter.render_skill(&entry.skill) {
                    let mut files = vec![(
                        "SKILL.md".to_string(),
                        blake3::hash(rendered.skill_md.as_bytes())
                            .to_hex()
                            .to_string(),
                    )];
                    files.extend(resources.clone());
                    // 排序与扫描口径一致（hash_from_files 不排序，collect_files 排序后喂入）
                    files.sort();
                    v_by_tool.insert(adapter.id().as_str(), scanner::hash_from_files(&files));
                }
            }
        }

        // 逐已接入工具判定；未接入列是列级属性，单元格仅四态（spec §6）
        let mut statuses: HashMap<String, Status> = HashMap::new();
        for (id, root) in &tool_roots {
            if root.is_none() {
                continue;
            }
            let t = scans[id.as_str()]
                .as_ref()
                .and_then(|s| s.skills.iter().find(|sk| sk.slug == slug))
                .map(|sk| sk.dir_hash.clone());
            let r = records
                .get(&(id.as_str().to_string(), slug.clone()))
                .map(|rec| rec.tool_hash.clone());
            let v = v_by_tool.get(id.as_str()).map(String::as_str);
            statuses.insert(
                id.as_str().to_string(),
                status::compute(t.as_deref(), r.as_deref(), v),
            );
        }
        rows.push(MatrixRow {
            skill: entry,
            statuses,
        });
    }

    Ok(StatusMatrix { tools, rows })
}

/// 分发事务门面：渲染 → 分发前重扫 → 自动快照 → staging → 落盘 → 记录 →
/// 清理（部分成功结构；分发级失败整体 Err）。见 `deploy.rs` 与规格 §2。
pub fn deploy_tool(
    vault_root: &Path,
    registry: &AdapterRegistry,
    snapshots_root: &Path,
    tool_id: ToolId,
    slugs: &[String],
    db: &Db,
) -> EngineResult<DeployResult> {
    deploy::deploy_tool(vault_root, registry, snapshots_root, tool_id, slugs, db)
}

/// 状态矩阵契约形状：`{ tools: [{id, connected}], rows: [{skill, statuses}] }`。
#[derive(Debug, Clone, Serialize)]
pub struct StatusMatrix {
    pub tools: Vec<ToolInfo>,
    pub rows: Vec<MatrixRow>,
}

/// 列信息：工具 id + 接入标志（未接入列前端渲染「未接入」+ 配置提示）。
#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub id: String,
    pub connected: bool,
}

/// 行：Skill（含 invalid 原因）+ 各已接入工具的状态。
#[derive(Debug, Clone, Serialize)]
pub struct MatrixRow {
    pub skill: SkillEntry,
    pub statuses: HashMap<String, Status>,
}

/// 分发记录（deploy_records 行）。`tool_hash` = 判定基准 r（落盘后工具端 hash）；
/// `vault_hash` = 分发时渲染产物 hash（表结构数据，S3 时间线/审计读取）。
pub(crate) struct DeployRecord {
    #[allow(dead_code)] // S3 快照时间线/审计读取，S2 无消费点
    pub(crate) vault_hash: String,
    pub(crate) tool_hash: String,
}

/// 读取 deploy_records 全部记录（S1 恒空；S2 分发写入后成为判定基准 r）。
pub(crate) fn load_deploy_records(
    conn: &Connection,
) -> EngineResult<HashMap<(String, String), DeployRecord>> {
    let mut stmt = conn
        .prepare("SELECT tool_id, skill_slug, vault_hash, tool_hash FROM deploy_records")
        .map_err(|e| EngineError::Internal(format!("读取分发记录失败：{e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                DeployRecord {
                    vault_hash: row.get(2)?,
                    tool_hash: row.get(3)?,
                },
            ))
        })
        .map_err(|e| EngineError::Internal(format!("读取分发记录失败：{e}")))?;
    let mut map = HashMap::new();
    for row in rows {
        let ((tool_id, skill_slug), record) =
            row.map_err(|e| EngineError::Internal(format!("读取分发记录失败：{e}")))?;
        map.insert((tool_id, skill_slug), record);
    }
    Ok(map)
}
