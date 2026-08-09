//! schema 迁移：`PRAGMA user_version` + 递增迁移数组（不引 refinery，
//! 见 `docs/research/s1-deps.md` §3.2）。
//!
//! S1 实现：snapshots / snapshot_files / deploy_records 三表，
//! 见 `docs/technical-plan.md` §3.5。

use rusqlite::Connection;

/// 迁移错误：schema 版本超前与底层 db 错误。
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    /// db 的 schema 版本高于应用支持版本（不自动降级）
    #[error("数据库 schema 版本 {0} 高于应用支持版本 {1}")]
    SchemaVersionAhead(i64, i64),
    /// 底层 SQLite 错误
    #[error("数据库错误：{0}")]
    Db(#[from] rusqlite::Error),
}

/// 递增迁移数组：索引 = 目标版本号 - 1；每项在事务内执行并推进 `user_version`。
const MIGRATIONS: &[&str] = &[
    // 迁移 1（S1）：三表——快照、快照文件清单、分发记录
    r#"
    CREATE TABLE snapshots (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        tool_id     TEXT NOT NULL,
        reason      TEXT NOT NULL,
        created_at  TEXT NOT NULL
    );

    CREATE TABLE snapshot_files (
        snapshot_id  INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        rel_path     TEXT NOT NULL,
        content_hash TEXT NOT NULL,
        PRIMARY KEY (snapshot_id, rel_path)
    );

    CREATE TABLE deploy_records (
        tool_id     TEXT NOT NULL,
        skill_slug  TEXT NOT NULL,
        vault_hash  TEXT NOT NULL,
        tool_hash   TEXT NOT NULL,
        deployed_at TEXT NOT NULL,
        PRIMARY KEY (tool_id, skill_slug)
    );
    "#,
];

/// 当前 schema 版本号（`PRAGMA user_version`）。
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// 执行全部未应用的迁移（幂等）：每项迁移在独立事务内完成并推进 `user_version`。
pub fn migrate(conn: &mut Connection) -> Result<(), MigrationError> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if current < 0 || current as usize > MIGRATIONS.len() {
        // 未来版本回退：不自动降级，按域信号返回（调用方映射为 Internal 兜底）
        return Err(MigrationError::SchemaVersionAhead(current, SCHEMA_VERSION));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;
    use rusqlite::Connection;

    /// 用内存库跑迁移，返回连接。
    fn migrated_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        conn
    }

    fn user_version(conn: &Connection) -> i64 {
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn 空库迁移后版本推进到最新且三表存在() {
        let conn = migrated_conn();
        assert_eq!(user_version(&conn), SCHEMA_VERSION);
        for table in ["snapshots", "snapshot_files", "deploy_records"] {
            let count: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "表 {table} 应存在");
        }
    }

    #[test]
    fn 迁移幂等_重复执行不报错且版本不变() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        let v1 = user_version(&conn);
        migrate(&mut conn).unwrap();
        assert_eq!(user_version(&conn), v1);
    }

    #[test]
    fn deploy_records_主键约束生效() {
        let conn = migrated_conn();
        conn.execute(
            "INSERT INTO deploy_records (tool_id, skill_slug, vault_hash, tool_hash, deployed_at)
             VALUES ('claude-code', 'greeting', 'v1', 't1', '2026-08-09T00:00:00Z')",
            [],
        )
        .unwrap();
        // 同 (tool_id, skill_slug) 重复插入应冲突
        let err = conn.execute(
            "INSERT INTO deploy_records (tool_id, skill_slug, vault_hash, tool_hash, deployed_at)
             VALUES ('claude-code', 'greeting', 'v2', 't2', '2026-08-09T00:00:00Z')",
            [],
        );
        assert!(err.is_err(), "主键 (tool_id, skill_slug) 应阻止重复");
    }

    #[test]
    fn snapshot_files_外键级联删除() {
        let conn = migrated_conn();
        conn.execute(
            "INSERT INTO snapshots (id, tool_id, reason, created_at)
             VALUES (1, 'codex', 'auto_pre_deploy', '2026-08-09T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO snapshot_files (snapshot_id, rel_path, content_hash)
             VALUES (1, 'SKILL.md', 'abc')",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM snapshots WHERE id = 1", [])
            .unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM snapshot_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "删快照应级联删文件清单");
    }
}
