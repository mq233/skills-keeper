//! 命令：scan / get_status_matrix（`docs/technical-plan.md` §4.7）。
//!
//! S2 async 迁移（决议「deploy 命令契约与前端分发交互」B）：四命令全迁 async，
//! `spawn_blocking` 包同步引擎调用（引擎保持同步）；scan / deploy 共用操作级锁
//! （`EngineLock`）串行化文件系统操作；get_status_matrix 只读不加锁。

use std::sync::Arc;

use tauri::State;

use crate::commands::{spawn_engine, AppPaths, EngineLock};
use crate::db::Db;
use crate::engine;
use crate::engine::error::EngineError;
use crate::engine::target::AdapterRegistry;
use crate::engine::StatusMatrix;

/// 手动触发扫描并返回最新矩阵（契约：与 `get_status_matrix` 同形状，一次往返）。
#[tauri::command]
pub async fn scan(
    paths: State<'_, AppPaths>,
    registry: State<'_, Arc<AdapterRegistry>>,
    lock: State<'_, EngineLock>,
    db: State<'_, Arc<Db>>,
) -> Result<StatusMatrix, EngineError> {
    let _guard = lock.lock().await;
    let paths = paths.inner().clone();
    let registry = Arc::clone(registry.inner());
    let db = Arc::clone(db.inner());
    spawn_engine(move || status_matrix(&paths, &registry, &db)).await
}

/// 当前状态矩阵：Vault × 已接入目标工具 × 状态徽章（契约：
/// `{ tools: [{id, connected}], rows: [{skill, statuses}] }`）。
#[tauri::command]
pub async fn get_status_matrix(
    paths: State<'_, AppPaths>,
    registry: State<'_, Arc<AdapterRegistry>>,
    db: State<'_, Arc<Db>>,
) -> Result<StatusMatrix, EngineError> {
    let paths = paths.inner().clone();
    let registry = Arc::clone(registry.inner());
    let db = Arc::clone(db.inner());
    spawn_engine(move || status_matrix(&paths, &registry, &db)).await
}

/// 组合门面：注册表提供工具路径与接入判定（S5 设置页用户配置覆盖适配器路径）。
fn status_matrix(
    paths: &AppPaths,
    registry: &AdapterRegistry,
    db: &Db,
) -> Result<StatusMatrix, EngineError> {
    engine::get_status_matrix(&paths.vault_root, registry, db)
}
