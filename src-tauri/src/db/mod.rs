//! SQLite 数据层：rusqlite 连接（Mutex 串行）与事务 helper。
//!
//! Phase 1 实现，见 `docs/technical-plan.md` §3.5 与 §4.1。
//! 分层规则：本模块只被 engine 使用，命令层与前端不直接接触。

pub mod migrations;
