//! 分发事务：渲染 → 分发前重扫 → 自动快照 → staging → 落盘 → 记录 → 清理。
//!
//! S2 决议「分发事务与自动快照细节」+「deploy 命令契约与前端分发交互」：
//! - 重扫仅「被工具修改」中止（含不在本次分发集的，保护工具端整体一致性）
//! - 快照失败 / 重扫失败 = 分发级失败 → 整体 `Err`
//! - Skill 级失败（渲染 / 校验 / 落盘 / 记录）→ 入 `failed` 继续（部分成功）
//! - 每 Skill 独立事务写 deploy_records（v = 渲染产物 hash、r = 落盘实际 hash），
//!   已落盘不回滚（自愈：下次扫描 t == v → 一致）
//! - staging 放目标父目录（禁放 skills/ 根下——S1 扫描器把根下子目录全算 Skill 候选）
//! - Codex 旧版跟随写入（目录存在才写；失败告警不失败，告警承载于 failed 结构）

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::Serialize;

use crate::db::Db;
use crate::engine::error::{EngineError, EngineResult};
use crate::engine::scanner;
use crate::engine::snapshot;
use crate::engine::status::{self, Status};
use crate::engine::target::adapters::CodexAdapter;
use crate::engine::target::{AdapterRegistry, ToolId};
use crate::engine::vault::{list_skills, SkillEntry};
use crate::engine::{load_deploy_records, DeployRecord};

/// 分发结果（部分成功结构，契约：`{ ok, failed }`，code = EngineError 变体名）。
#[derive(Debug, Clone, Serialize)]
pub struct DeployResult {
    pub ok: Vec<DeployOkItem>,
    pub failed: Vec<DeployFailedItem>,
}

/// 成功条目：工具 + Skill 已落盘且记录写入。
#[derive(Debug, Clone, Serialize)]
pub struct DeployOkItem {
    pub tool_id: String,
    pub skill_slug: String,
}

/// 失败条目：Skill 级失败原因（message 中文可直接展示）。
#[derive(Debug, Clone, Serialize)]
pub struct DeployFailedItem {
    pub tool_id: String,
    pub skill_slug: String,
    pub code: String,
    pub message: String,
}

/// 分发事务主体（引擎门面 `engine::deploy_tool` 转发）。
/// 空 `slugs` → 空结果（无操作，不重扫不快照）。
pub fn deploy_tool(
    vault_root: &Path,
    registry: &AdapterRegistry,
    snapshots_root: &Path,
    tool_id: ToolId,
    slugs: &[String],
    db: &Db,
) -> EngineResult<DeployResult> {
    if slugs.is_empty() {
        return Ok(DeployResult {
            ok: Vec::new(),
            failed: Vec::new(),
        });
    }

    // 0. 适配器与目录：未接入 → 分发级 Err(Config)（无法分发）
    if registry.get(tool_id).is_none() {
        return Err(EngineError::Config(format!(
            "未知目标工具：{}",
            tool_id.as_str()
        )));
    }
    let skills_root = registry
        .tool_roots()
        .into_iter()
        .find(|(id, _)| *id == tool_id)
        .and_then(|(_, dir)| dir)
        .ok_or_else(|| {
            EngineError::Config(format!("目标工具「{}」未接入，无法分发", tool_id.as_str()))
        })?;

    // 1. 读 Vault Skill 全集（slug 索引；Sidecar 默认 targets 由注册表合成）
    let by_slug: HashMap<String, SkillEntry> =
        list_skills(vault_root, &registry.all_connected_targets())?
            .into_iter()
            .map(|e| (e.skill.slug.clone(), e))
            .collect();

    // 2. 分发前重扫（整工具）：仅「被工具修改」中止（返回被修改清单 + 可读提示）
    let scan = scanner::scan_tool(&skills_root)?; // None = 根不存在，视为空工具端
    let records = {
        let conn = db.lock();
        load_deploy_records(&conn)?
    };
    let modified = find_modified(&scan, &records, &tool_id, vault_root, &by_slug);
    if !modified.is_empty() {
        let list = modified
            .iter()
            .map(|s| format!("- {}/{}", tool_id.as_str(), s))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(EngineError::InvalidState(format!(
            "分发前重扫发现目标工具「{}」的以下 Skill 已被外部修改：\n{}\n为避免覆盖未知内容，分发已中止。请先在工具端处理这些 Skill 后再重试。",
            tool_id.as_str(),
            list
        )));
    }

    // 3. 自动快照（整工具全量，独立事务；失败 = 分发级 Io，无快照即无回滚能力）
    {
        let mut conn = db.lock();
        snapshot::auto_pre_deploy(&mut conn, tool_id.as_str(), &skills_root, snapshots_root)?;
    }

    // 3.5 工具端根不存在时创建（首次分发；重扫已按空工具端放行，落盘需父目录存在）
    fs::create_dir_all(&skills_root)
        .map_err(|e| EngineError::Io(format!("创建目标工具目录失败：{e}")))?;

    // 4. staging 根：目标父目录（与目标同盘保证 rename 原子；禁放 skills/ 根下）
    let staging_root = skills_root
        .parent()
        .map(|p| p.join(".skills-keeper-staging"))
        .ok_or_else(|| {
            EngineError::Io(format!(
                "无法解析目标工具目录的父路径：{}",
                skills_root.display()
            ))
        })?;
    fs::create_dir_all(&staging_root).map_err(|_| {
        EngineError::Io(format!("创建 staging 目录失败：{}", staging_root.display()))
    })?;

    // 5. 逐 Skill：渲染 → 校验 → staging 写入 → 落盘 → 记录（部分成功）
    let mut ok: Vec<DeployOkItem> = Vec::new();
    let mut failed: Vec<DeployFailedItem> = Vec::new();
    for slug in slugs {
        let Some(entry) = by_slug.get(slug) else {
            failed.push(failed_item(
                &tool_id,
                slug,
                &EngineError::NotFound(format!("Vault 中不存在 Skill「{slug}」")),
            ));
            continue;
        };
        let vault_skill_dir = vault_root.join("skills").join(slug);

        // 渲染（纯函数；失败 → Skill 级失败入 failed）
        let rendered = match registry.get(tool_id).unwrap().render_skill(&entry.skill) {
            Ok(r) => r,
            Err(e) => {
                failed.push(failed_item(&tool_id, slug, &e));
                continue;
            }
        };
        // 校验（只查必败项 → InvalidSkill 兜底拦截）
        if let Err(e) = registry.get(tool_id).unwrap().validate(&rendered) {
            failed.push(failed_item(&tool_id, slug, &e));
            continue;
        }

        // staging 写入 + 渲染产物 hash（v = 判定基准：渲染后期望内容）
        let stage = staging_root.join(slug);
        let v = match stage_skill(&stage, &vault_skill_dir, &rendered) {
            Ok(hash) => hash,
            Err(e) => {
                failed.push(failed_item(&tool_id, slug, &e));
                let _ = fs::remove_dir_all(&stage);
                continue;
            }
        };

        // 落盘（两阶段备份覆盖 / 跨盘复制校验回退）
        let target = skills_root.join(slug);
        match deploy_one(&stage, &target, &staging_root) {
            Ok(()) => {
                // 记录：v = 渲染产物 hash、r = 落盘实际 hash（扫描口径），独立事务提交
                let r = match scanner::hash_dir(&target) {
                    Ok(h) => h,
                    Err(e) => {
                        failed.push(failed_item(&tool_id, slug, &e));
                        continue;
                    }
                };
                let write_result = {
                    let conn = db.lock();
                    conn.execute(
                        "INSERT OR REPLACE INTO deploy_records
                         (tool_id, skill_slug, vault_hash, tool_hash, deployed_at)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![tool_id.as_str(), slug, v, r, now_secs()],
                    )
                    .map_err(|e| EngineError::Internal(format!("写入分发记录失败：{e}")))
                };
                match write_result {
                    Ok(_) => ok.push(DeployOkItem {
                        tool_id: tool_id.as_str().to_string(),
                        skill_slug: slug.clone(),
                    }),
                    Err(e) => {
                        // 已落盘未记录 → 自愈（下次扫描 t == v → 一致）；提示重试幂等覆盖
                        failed.push(failed_item(&tool_id, slug, &e));
                    }
                }
            }
            Err(e) => {
                failed.push(failed_item(&tool_id, slug, &e));
                let _ = fs::remove_dir_all(&stage);
            }
        }
    }

    // 6. Codex 旧版跟随写入：主分发成功后复制到旧版目录（存在才写；失败告警不失败）
    if tool_id == ToolId::Codex {
        if let Some(legacy_root) = CodexAdapter::legacy_skills_dir() {
            if legacy_root.exists() {
                for item in &ok {
                    let src = skills_root.join(&item.skill_slug);
                    let dst = legacy_root.join(&item.skill_slug);
                    if let Err(e) = copy_overwrite(&src, &dst) {
                        failed.push(DeployFailedItem {
                            tool_id: tool_id.as_str().to_string(),
                            skill_slug: item.skill_slug.clone(),
                            code: "Io".to_string(),
                            message: format!(
                                "主目录已分发成功；旧版目录（~/.codex/skills）写入失败：{e}"
                            ),
                        });
                    }
                }
            }
        }
    }

    // 7. 清理 staging（尽力而为；staging 在目标父目录，不属扫描口径，残留不影响判定）
    let _ = fs::remove_dir_all(&staging_root);

    Ok(DeployResult { ok, failed })
}

/// 重扫判定「被工具修改」：遍历工具端现有 Skill，`compute(t, r, v) == Modified` 收集。
/// v = Vault 当前目录 hash（重扫用 Vault 基准；工具端存在而 Vault 无该 Skill → v None）。
fn find_modified(
    scan: &Option<scanner::ScanResult>,
    records: &HashMap<(String, String), DeployRecord>,
    tool_id: &ToolId,
    vault_root: &Path,
    by_slug: &HashMap<String, SkillEntry>,
) -> Vec<String> {
    let Some(scan) = scan else {
        return Vec::new(); // 工具端根不存在 → 无「被修改」
    };
    let mut modified = Vec::new();
    for skill in &scan.skills {
        let t = Some(skill.dir_hash.as_str());
        let r = records
            .get(&(tool_id.as_str().to_string(), skill.slug.clone()))
            .map(|rec| rec.tool_hash.as_str());
        let v = by_slug
            .get(&skill.slug)
            .and_then(|_| scanner::hash_dir(&vault_root.join("skills").join(&skill.slug)).ok());
        if status::compute(t, r, v.as_deref()) == Status::Modified {
            modified.push(skill.slug.clone());
        }
    }
    modified
}

/// staging 写入：SKILL.md + 资源（从 Vault 复制）+ extra_files，返回渲染产物目录 hash（v）。
fn stage_skill(
    stage: &Path,
    vault_skill_dir: &Path,
    rendered: &crate::engine::target::RenderedSkill,
) -> EngineResult<String> {
    fs::create_dir_all(stage)
        .map_err(|e| EngineError::Io(format!("创建 staging 目录失败：{e}")))?;
    fs::write(stage.join("SKILL.md"), &rendered.skill_md)
        .map_err(|e| EngineError::Io(format!("写入 staging SKILL.md 失败：{e}")))?;
    for rel in &rendered.resources {
        let src = vault_skill_dir.join(rel);
        let dst = stage.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| EngineError::Io(format!("创建 staging 子目录失败：{e}")))?;
        }
        fs::copy(&src, &dst)
            .map_err(|e| EngineError::Io(format!("复制资源到 staging 失败：{e}")))?;
    }
    for (rel, content) in &rendered.extra_files {
        let dst = stage.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| EngineError::Io(format!("创建 staging 子目录失败：{e}")))?;
        }
        fs::write(&dst, content)
            .map_err(|e| EngineError::Io(format!("写入 staging 伴生文件失败：{e}")))?;
    }
    scanner::hash_dir(stage)
}

/// 单 Skill 落盘：两阶段备份覆盖（旧目录 rename 到 staging 备份位 → 新目录 rename 入位
/// → 成功删备份；失败备份回原位）；跨盘回退为 复制 + blake3 校验 + 删源。
fn deploy_one(stage: &Path, target: &Path, staging_root: &Path) -> EngineResult<()> {
    let slug = stage
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let backup = staging_root.join(".backup").join(&slug);

    // 旧目录挪到备份位（目标已存在时；Windows rename 不覆盖已存在目录）
    if target.exists() {
        if let Some(parent) = backup.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| EngineError::Io(format!("创建备份目录失败：{e}")))?;
        }
        fs::rename(target, &backup).map_err(|e| EngineError::Io(format!("备份旧目录失败：{e}")))?;
    }

    match fs::rename(stage, target) {
        Ok(()) => {
            // 落盘成功：删除备份（旧内容已由新内容取代）
            if backup.exists() {
                fs::remove_dir_all(&backup)
                    .map_err(|e| EngineError::Io(format!("清理备份目录失败：{e}")))?;
            }
            Ok(())
        }
        Err(e) if e.kind() == ErrorKind::CrossesDevices => {
            // 跨盘回退：复制 + 逐文件 blake3 校验（通过才删源）；失败回退清理
            match copy_verified(stage, target) {
                Ok(()) => {
                    let _ = fs::remove_dir_all(stage);
                    if backup.exists() {
                        fs::remove_dir_all(&backup)
                            .map_err(|e| EngineError::Io(format!("清理备份目录失败：{e}")))?;
                    }
                    Ok(())
                }
                Err(e) => {
                    restore_backup(&backup, target);
                    Err(EngineError::Io(format!("跨盘复制落盘失败：{e}")))
                }
            }
        }
        Err(e) => {
            // 其他 rename 失败：备份回原位，原 staging 残留由调用方清理
            restore_backup(&backup, target);
            Err(EngineError::Io(format!("落盘失败：{e}")))
        }
    }
}

/// 复制目录 + 逐文件 blake3 校验（内容一致才算成功）；失败删除目标残留。
fn copy_verified(src: &Path, dst: &Path) -> std::io::Result<()> {
    let mut stack = vec![(src.to_path_buf(), dst.to_path_buf())];
    while let Some((s, d)) = stack.pop() {
        for entry in fs::read_dir(&s)? {
            let entry = entry?;
            let s2 = entry.path();
            let d2 = d.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                fs::create_dir_all(&d2)?;
                stack.push((s2, d2));
            } else {
                fs::copy(&s2, &d2)?;
                let src_hash = blake3::hash(&fs::read(&s2)?);
                let dst_hash = blake3::hash(&fs::read(&d2)?);
                if src_hash != dst_hash {
                    let _ = fs::remove_dir_all(dst);
                    return Err(std::io::Error::other(format!(
                        "文件校验不一致：{}",
                        d2.display()
                    )));
                }
            }
        }
    }
    Ok(())
}

/// 复制覆盖（Codex 旧版跟随写入）：先删旧目录再复制（无原子要求，失败告警语义）。
fn copy_overwrite(src: &Path, dst: &Path) -> EngineResult<()> {
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|e| EngineError::Io(format!("删除旧版旧目录失败：{e}")))?;
    }
    fs::create_dir_all(dst).map_err(|e| EngineError::Io(format!("创建旧版目录失败：{e}")))?;
    copy_verified(src, dst).map_err(|e| EngineError::Io(format!("复制旧版失败：{e}")))?;
    Ok(())
}

/// 备份回原位（尽力而为；失败时备份残留由下次 staging 清理兜底）。
fn restore_backup(backup: &Path, target: &Path) {
    if backup.exists() {
        let _ = fs::rename(backup, target);
    }
}

/// 失败条目构造：code = EngineError 变体名、message = 中文文案（契约同口径）。
fn failed_item(tool_id: &ToolId, slug: &str, e: &EngineError) -> DeployFailedItem {
    DeployFailedItem {
        tool_id: tool_id.as_str().to_string(),
        skill_slug: slug.to_string(),
        code: e.code().to_string(),
        message: e.to_string(),
    }
}

/// 时间戳（epoch 秒；S3 时间线 UI 再定展示格式）。
fn now_secs() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn 落盘失败_备份回原位_不残留() {
        let temp = TempDir::new().unwrap();
        let staging_root = temp.path().join("staging");
        let stage = staging_root.join("skill-a");
        let target = temp.path().join("tools").join("skill-a");
        write(&stage.join("SKILL.md"), "新内容");
        write(&target.join("SKILL.md"), "旧内容");

        // 模拟：stage 在 rename 前被外部删除 → rename 失败（非 EXDEV）
        fs::remove_dir_all(&stage).unwrap();
        let err = deploy_one(&stage, &target, &staging_root).unwrap_err();
        assert!(matches!(err, EngineError::Io(_)), "落盘失败应为 Io");
        // 备份回原位：目标仍是旧内容、备份内容无残留（.backup 目录本身保留由 staging 清理兜底）
        assert!(target.join("SKILL.md").exists(), "目标应恢复旧内容");
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "旧内容"
        );
        assert!(
            !staging_root.join(".backup").join("skill-a").exists(),
            "备份内容应已移回原位"
        );
    }

    #[test]
    fn 备份目录创建失败_目标不动() {
        let temp = TempDir::new().unwrap();
        let staging_root = temp.path().join("staging");
        let stage = staging_root.join("skill-a");
        let target = temp.path().join("tools").join("skill-a");
        write(&stage.join("SKILL.md"), "新内容");
        write(&target.join("SKILL.md"), "旧内容");
        // 备份位被文件占用 → 备份目录创建失败 → 目标保持原样
        write(&staging_root.join(".backup"), "占用");

        let err = deploy_one(&stage, &target, &staging_root).unwrap_err();
        assert!(matches!(err, EngineError::Io(_)));
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "旧内容",
            "备份失败时目标不动"
        );
    }
}
