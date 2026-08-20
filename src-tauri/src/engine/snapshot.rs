//! 快照：分发前自动快照落盘子集（S2，`docs/technical-plan.md` §3.5/§4.5）。
//!
//! S2 决议「分发事务与自动快照细节」B：
//! - 整工具全量复制（**含隐藏元数据文件**，与扫描口径不同——回滚需完整恢复原样）
//! - 逐文件复制时计算 blake3 hash（与扫描器同款算法，S3 回滚校验可复用）
//! - 时序：插 `snapshots` 行（reason='auto_pre_deploy'）拿 id → 复制到
//!   `snapshots/<id>/` + 写 `.manifest.json` 冗余清单 → 写 `snapshot_files`；
//!   SQLite 事务覆盖
//! - 失败 → 中止分发（无快照即无回滚能力），错误码 `Io`
//! - S3 保留策略留痕：目录与表结构按 §3.5 完整实现（id 可回溯），
//!   保留策略（按 tool_id 保留最近 N 份）S2 不实现、S3 只加淘汰逻辑

use std::fs;
use std::path::Path;

use rusqlite::Connection;

use crate::engine::error::{EngineError, EngineResult};

/// 分发前自动快照：把目标工具端 Skill 目录全量复制到 `<snapshots>/<id>/`
/// 并登记 `snapshots` / `snapshot_files` 表（独立事务，失败整体回滚）。
/// `skills_root` 不存在时快照为空目录（记录「分发前为空」，回滚语义完整）。
pub fn auto_pre_deploy(
    conn: &mut Connection,
    tool_id: &str,
    skills_root: &Path,
    snapshots_root: &Path,
) -> EngineResult<i64> {
    let tx = conn
        .transaction()
        .map_err(|e| EngineError::Internal(format!("快照事务启动失败：{e}")))?;

    // 1. 插 snapshots 行拿 AUTOINCREMENT id（时间用 epoch 秒，S3 时间线 UI 再定格式）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());
    tx.execute(
        "INSERT INTO snapshots (tool_id, reason, created_at) VALUES (?1, 'auto_pre_deploy', ?2)",
        rusqlite::params![tool_id, now],
    )
    .map_err(|e| EngineError::Internal(format!("写入快照记录失败：{e}")))?;
    let id = tx.last_insert_rowid();

    // 2. 复制到快照目录（全量含隐藏文件）+ 逐文件 blake3 + 收集清单
    let dir = snapshots_root.join(id.to_string());
    fs::create_dir_all(&dir).map_err(|e| EngineError::Io(format!("创建快照目录失败：{e}")))?;
    let mut files: Vec<(String, String)> = Vec::new();
    if skills_root.exists() {
        copy_tree(skills_root, &dir, "", &mut files)
            .map_err(|e| EngineError::Io(format!("复制快照失败：{e}")))?;
    }

    // 3. 写 .manifest.json 冗余清单（目录外恢复依据；文件级 hash 与 snapshot_files 同源）
    let manifest: Vec<serde_json::Value> = files
        .iter()
        .map(|(rel, hash)| serde_json::json!({ "rel_path": rel, "content_hash": hash }))
        .collect();
    fs::write(
        dir.join(".manifest.json"),
        serde_json::to_string_pretty(&manifest)
            .map_err(|e| EngineError::Internal(format!("快照清单序列化失败：{e}")))?,
    )
    .map_err(|e| EngineError::Io(format!("写快照清单失败：{e}")))?;

    // 4. 写 snapshot_files 表（同事务，失败整体回滚）
    for (rel, hash) in &files {
        tx.execute(
            "INSERT INTO snapshot_files (snapshot_id, rel_path, content_hash) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, rel, hash],
        )
        .map_err(|e| EngineError::Internal(format!("写快照文件清单失败：{e}")))?;
    }

    tx.commit()
        .map_err(|e| EngineError::Internal(format!("快照事务提交失败：{e}")))?;
    Ok(id)
}

/// 递归复制目录（全量含隐藏文件），相对路径统一 `/` 分隔（与扫描口径一致，
/// 跨平台可回溯）；逐文件计算 blake3 内容 hash 收集到清单。
fn copy_tree(
    src: &Path,
    dst: &Path,
    prefix: &str,
    files: &mut Vec<(String, String)>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let s = src.join(&name);
        let d = dst.join(&name);
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&d)?;
            copy_tree(&s, &d, &rel, files)?;
        } else {
            fs::copy(&s, &d)?;
            let hash = blake3::hash(&fs::read(&s)?).to_hex().to_string();
            files.push((rel, hash));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// 迁移过的内存库连接。
    fn conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::migrate(&mut conn).unwrap();
        conn
    }

    #[test]
    fn 自动快照_整工具全量复制_含隐藏文件与manifest() {
        let temp = TempDir::new().unwrap();
        let tools = temp.path().join("skills");
        let snaps = temp.path().join("snapshots");
        write(&tools.join("greeting/SKILL.md"), "hello");
        write(&tools.join("greeting/.skill-meta.json"), "{}");
        write(&tools.join("greeting/scripts/helper.py"), "x");
        write(&tools.join("loose.txt"), "散文件");

        let mut conn = conn();
        let id = auto_pre_deploy(&mut conn, "codex", &tools, &snaps).unwrap();
        assert_eq!(id, 1, "首个快照 id = 1");

        let dir = snaps.join("1");
        assert!(dir.join("greeting/SKILL.md").exists());
        assert!(
            dir.join("greeting/.skill-meta.json").exists(),
            "隐藏文件应复制"
        );
        assert!(dir.join("greeting/scripts/helper.py").exists());
        assert!(
            dir.join("loose.txt").exists(),
            "散文件应复制（完整恢复原样）"
        );
        assert!(dir.join(".manifest.json").exists());

        // snapshot_files 表与文件清单一致（含隐藏文件）
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM snapshot_files WHERE snapshot_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 4, "4 个文件（含隐藏 + 散文件）");
        let has_hidden: i64 = conn
            .query_row(
                "SELECT count(*) FROM snapshot_files WHERE snapshot_id = 1 AND rel_path = 'greeting/.skill-meta.json'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(has_hidden, 1);
        // hash 与 blake3 一致（回滚校验可复用）
        let stored: String = conn
            .query_row(
                "SELECT content_hash FROM snapshot_files WHERE snapshot_id = 1 AND rel_path = 'greeting/SKILL.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, blake3::hash(b"hello").to_hex().to_string());
    }

    #[test]
    fn 自动快照_工具端不存在_快照为空() {
        let temp = TempDir::new().unwrap();
        let mut conn = conn();
        let id = auto_pre_deploy(
            &mut conn,
            "trae",
            &temp.path().join("no-such-tools"),
            &temp.path().join("snapshots"),
        )
        .unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM snapshot_files WHERE snapshot_id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "工具端不存在 → 空快照（记录分发前为空）");
    }

    #[test]
    fn 自动快照_失败事务回滚_不残留表行() {
        let temp = TempDir::new().unwrap();
        let mut conn = conn();
        // 快照根是文件（不可建子目录）→ 复制失败 → 事务回滚
        let snaps_file = temp.path().join("snapshots-file");
        fs::write(&snaps_file, "占用").unwrap();
        let err = auto_pre_deploy(&mut conn, "codex", &temp.path().join("tools"), &snaps_file)
            .unwrap_err();
        assert!(matches!(err, EngineError::Io(_)), "快照失败应为 Io 错误");
        let count: i64 = conn
            .query_row("SELECT count(*) FROM snapshots", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "失败应整体回滚，不残留 snapshots 行");
    }

    #[test]
    fn 自动快照_逐次递增id() {
        let temp = TempDir::new().unwrap();
        let mut conn = conn();
        let tools = temp.path().join("skills");
        write(&tools.join("a/SKILL.md"), "a");
        write(&tools.join("b/SKILL.md"), "b");
        let snaps = temp.path().join("snapshots");
        let id1 = auto_pre_deploy(&mut conn, "codex", &tools, &snaps).unwrap();
        let id2 = auto_pre_deploy(&mut conn, "codex", &tools, &snaps).unwrap();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id2, id1 + 1, "AUTOINCREMENT 逐次递增（S3 保留策略可回溯）");
        assert!(snaps.join("1").exists() && snaps.join("2").exists());
    }
}
