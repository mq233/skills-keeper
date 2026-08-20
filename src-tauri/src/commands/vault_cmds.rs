//! 命令：list_skills（`docs/technical-plan.md` §4.7；S1 前端不调用，S4 导入器使用）。

use tauri::State;

use crate::commands::AppPaths;
use crate::engine;
use crate::engine::error::EngineError;
use crate::engine::target::AdapterRegistry;
use crate::engine::vault::SkillEntry;

/// 全部 Skill 列表（契约：`SkillEntry[]`，invalid 含行级标记原因）。
#[tauri::command]
pub fn list_skills(
    paths: State<'_, AppPaths>,
    registry: State<'_, AdapterRegistry>,
) -> Result<Vec<SkillEntry>, EngineError> {
    engine::list_skills(&paths.vault_root, &registry)
}
