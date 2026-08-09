//! SQLite 数据层：rusqlite 连接（Mutex 串行）与事务 helper。
//!
//! S1 实现，见 `docs/technical-plan.md` §3.5 与 §4.1。
//! 分层规则：本模块只被 engine 使用，命令层与前端不直接接触。

pub mod migrations;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

/// 全局共享的 SQLite 连接：`Mutex` 串行化防止并发写（技术规划 §4.7）。
pub struct Db(pub Mutex<Connection>);

impl Db {
    /// 打开（不存在则创建）数据库文件并返回连接包装。
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self(Mutex::new(conn)))
    }

    /// 获取内部连接（Mutex 串行化）；锁中毒时继续使用（连接本身仍可用）。
    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
