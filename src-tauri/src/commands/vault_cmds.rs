//! 命令：list_skills（`docs/technical-plan.md` §4.7；S1 前端不调用，S4 导入器使用）。
//!
//! S2 async 迁移：命令层 async + spawn_blocking（引擎保持同步）。

use std::sync::Arc;

use tauri::State;

use crate::commands::{spawn_engine, AppPaths};
use crate::engine;
use crate::engine::error::EngineError;
use crate::engine::target::AdapterRegistry;
use crate::engine::vault::SkillEntry;

/// 全部 Skill 列表（契约：`SkillEntry[]`，invalid 含行级标记原因）。
#[tauri::command]
pub async fn list_skills(
    paths: State<'_, AppPaths>,
    registry: State<'_, Arc<AdapterRegistry>>,
) -> Result<Vec<SkillEntry>, EngineError> {
    let paths = paths.inner().clone();
    let registry = Arc::clone(registry.inner());
    spawn_engine(move || engine::list_skills(&paths.vault_root, &registry)).await
}
