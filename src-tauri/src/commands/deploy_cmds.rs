//! 命令：deploy（`docs/technical-plan.md` §4.7；S2 决议「deploy 命令契约与前端分发交互」A）。
//!
//! 契约：`{ tool_id: string, skill_slugs: string[] }` → `{ ok, failed }`（部分成功）；
//! 引擎无特判（「分发全部」由前端算全集传入）；未接入工具 → 分发级 `Err(Config)`；
//! Skill 级失败（渲染 / 校验 / 落盘）入 failed；分发级失败（重扫中止 / 快照失败）整体 Err。
//! 操作级锁：与 scan 共用（串行化文件系统操作）。

use std::sync::Arc;

use serde::Deserialize;
use tauri::State;

use crate::commands::{spawn_engine, AppPaths, EngineLock};
use crate::db::Db;
use crate::engine;
use crate::engine::error::EngineError;
use crate::engine::target::{AdapterRegistry, ToolId};

/// deploy 输入契约（前端算全集显式传入；引擎无特判）。
#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    pub tool_id: String,
    pub skill_slugs: Vec<String>,
}

/// 分发：渲染 → 分发前重扫 → 自动快照 → staging → 落盘 → 记录 → 清理。
/// 返回部分成功结构；分发级失败（重扫中止 InvalidState / 快照失败 Io）整体 Err。
#[tauri::command]
pub async fn deploy(
    paths: State<'_, AppPaths>,
    registry: State<'_, Arc<AdapterRegistry>>,
    lock: State<'_, EngineLock>,
    db: State<'_, Arc<Db>>,
    request: DeployRequest,
) -> Result<engine::DeployResult, EngineError> {
    let _guard = lock.lock().await;
    let tool_id = request
        .tool_id
        .parse::<ToolId>()
        .map_err(|_| EngineError::Config(format!("未知目标工具：{}", request.tool_id)))?;
    let paths = paths.inner().clone();
    let registry = Arc::clone(registry.inner());
    let db = Arc::clone(db.inner());
    let slugs = request.skill_slugs;
    let snapshots_root = paths.data_dir.join("snapshots");
    spawn_engine(move || {
        engine::deploy_tool(
            &paths.vault_root,
            &registry,
            &snapshots_root,
            tool_id,
            &slugs,
            &db,
        )
    })
    .await
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;

    #[test]
    fn deploy_request_契约反序列化() {
        let req: DeployRequest =
            serde_json::from_str(r#"{"tool_id": "codex", "skill_slugs": ["a", "b"]}"#).unwrap();
        assert_eq!(req.tool_id, "codex");
        assert_eq!(req.skill_slugs, vec!["a", "b"]);
        // 字段名契约（snake_case）与前端镜像一致；未知 id 解析失败由命令层映射 Config
        assert!(
            serde_json::from_str::<DeployRequest>(r#"{"tool_id": "codex"}"#).is_err(),
            "skill_slugs 为必填字段"
        );
    }
}
