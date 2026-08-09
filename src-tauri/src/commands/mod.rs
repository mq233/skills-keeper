//! Tauri command 薄层：参数校验与序列化，无业务逻辑。
//!
//! 分层规则：命令层不落业务逻辑，只转发与序列化；前端仅与 commands 对话
//! （`docs/technical-plan.md` §2）。Phase 1–4 按模块逐个挂载到 lib.rs。

pub mod deploy_cmds;
pub mod import_cmds;
pub mod scan_cmds;
pub mod vault_cmds;
