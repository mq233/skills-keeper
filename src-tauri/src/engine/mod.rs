//! Rust 核心引擎门面：组合各模块，供命令层调用。
//!
//! 分层规则：engine 不依赖 Tauri，可纯单元测试（`docs/technical-plan.md` §2）。
//! 工具端路径由命令层注入（`ToolRoots`），引擎只读——S5 设置页用户配置覆盖的天然扩展点。

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
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::db::{migrations, Db};

use error::{EngineError, EngineResult};
use scanner::ScanResult;
use status::Status;
use target::ToolId;
use vault::SkillEntry;

/// 初始化数据库：打开连接 + 执行全部迁移（S1 三表，`docs/specs/s1-matrix.md` §7）。
/// 命令层 setup 传入 db 文件路径；引擎不依赖 Tauri。
pub fn init_db(db_path: &Path) -> EngineResult<Db> {
    let db = Db::open(db_path).map_err(|e| EngineError::Io(format!("打开数据库失败：{e}")))?;
    {
        let mut conn = db.lock();
        migrations::migrate(&mut conn)
            .map_err(|e| EngineError::Internal(format!("数据库迁移失败：{e}")))?;
    }
    Ok(db)
}

/// 工具端根目录解析结果（命令层注入；None = 未接入，引擎不扫描）。
pub type ToolRoots = Vec<(ToolId, Option<PathBuf>)>;

/// S1 默认工具端根目录：`ToolId::default_skills_dir()`（`~` 展开；
/// WorkBuddy 官方未公开路径 → None = 未接入）。S5 设置页用户配置覆盖此函数。
pub fn default_tool_roots() -> ToolRoots {
    ToolId::ALL
        .iter()
        .map(|id| (*id, id.default_skills_dir()))
        .collect()
}

/// Skill 列表（契约 `list_skills` → `SkillEntry[]`；S1 前端不调用，S4 导入器使用）。
pub fn list_skills(vault_root: &Path) -> EngineResult<Vec<SkillEntry>> {
    vault::list_skills(vault_root)
}

/// 状态矩阵（契约 `get_status_matrix` / `scan` 同形状）：
/// 读取 Vault → 逐工具扫描（未接入跳过）→ deploy_records 比对 → 判定矩阵全分支。
pub fn get_status_matrix(
    vault_root: &Path,
    tool_roots: &ToolRoots,
    db: &Db,
) -> EngineResult<StatusMatrix> {
    let entries = vault::list_skills(vault_root)?;

    // 逐工具扫描（S1 全量；v 惰性计算为优化意图，spec §8 从简）
    let mut scans: HashMap<&'static str, Option<ScanResult>> = HashMap::new();
    for (id, root) in tool_roots {
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
        let v = if need_v {
            Some(scanner::hash_dir(&vault_root.join("skills").join(&slug))?)
        } else {
            None
        };

        // 逐已接入工具判定；未接入列是列级属性，单元格仅四态（spec §6）
        let mut statuses: HashMap<String, Status> = HashMap::new();
        for (id, root) in tool_roots {
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
            statuses.insert(
                id.as_str().to_string(),
                status::compute(t.as_deref(), r.as_deref(), v.as_deref()),
            );
        }
        rows.push(MatrixRow {
            skill: entry,
            statuses,
        });
    }

    Ok(StatusMatrix { tools, rows })
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

/// 分发记录（deploy_records 行）。S1 只用 `tool_hash`（判定基准 r）；
/// `vault_hash`（分发时 Vault hash）留 S2 分发记录校验使用。
struct DeployRecord {
    #[allow(dead_code)]
    vault_hash: String,
    tool_hash: String,
}

/// 读取 deploy_records 全部记录（S1 恒空；S2 分发写入后成为判定基准 r）。
fn load_deploy_records(conn: &Connection) -> EngineResult<HashMap<(String, String), DeployRecord>> {
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
