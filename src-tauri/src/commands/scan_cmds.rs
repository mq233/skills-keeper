//! 命令：scan / get_status_matrix（`docs/technical-plan.md` §4.7）。
//!
//! 命令全同步签名（Tauri 同步 command 运行于后台线程，前端 invoke 天然异步）；
//! `Mutex<Connection>` 串行化防并发；async + 任务队列留 S2。

use tauri::State;

use crate::commands::AppPaths;
use crate::db::Db;
use crate::engine;
use crate::engine::error::EngineError;
use crate::engine::StatusMatrix;

/// 手动触发扫描并返回最新矩阵（契约：与 `get_status_matrix` 同形状，一次往返）。
#[tauri::command]
pub fn scan(paths: State<'_, AppPaths>, db: State<'_, Db>) -> Result<StatusMatrix, EngineError> {
    status_matrix(&paths, &db)
}

/// 当前状态矩阵：Vault × 已接入目标工具 × 状态徽章（契约：
/// `{ tools: [{id, connected}], rows: [{skill, statuses}] }`）。
#[tauri::command]
pub fn get_status_matrix(
    paths: State<'_, AppPaths>,
    db: State<'_, Db>,
) -> Result<StatusMatrix, EngineError> {
    status_matrix(&paths, &db)
}

/// 组合门面：S1 用默认工具路径（S5 设置页用户配置覆盖 `default_tool_roots`）。
fn status_matrix(paths: &AppPaths, db: &Db) -> Result<StatusMatrix, EngineError> {
    let tool_roots = engine::default_tool_roots();
    engine::get_status_matrix(&paths.vault_root, &tool_roots, db)
}
