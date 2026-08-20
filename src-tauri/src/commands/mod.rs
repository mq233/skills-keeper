//! Tauri command 薄层：参数校验与序列化，无业务逻辑。
//!
//! 分层规则：命令层不落业务逻辑，只转发与序列化；前端仅与 commands 对话
//! （`docs/technical-plan.md` §2）。S1 挂载 `list_skills` / `scan` / `get_status_matrix`。

pub mod deploy_cmds;
pub mod import_cmds;
pub mod scan_cmds;
pub mod vault_cmds;

use std::path::PathBuf;

use crate::engine::error::{EngineError, EngineResult};
use crate::engine::target::expand_tilde;

/// 引擎操作级锁（S2 决议：scan / deploy 共用，串行化文件系统操作；
/// `get_status_matrix` / `list_skills` 只读不加锁）。
pub type EngineLock = tokio::sync::Mutex<()>;

/// 引擎调用包装：`spawn_blocking` 包同步引擎函数——阻塞 IO 不占 tokio worker，
/// 引擎保持同步纯函数（可测性不变）。命令层 async 化的统一出口。
pub(crate) async fn spawn_engine<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, EngineError> + Send + 'static,
) -> Result<T, EngineError> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| EngineError::Internal(format!("引擎任务失败：{e}")))?
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;

    #[test]
    fn 操作级锁_持锁互斥_释放可得() {
        tauri::async_runtime::block_on(async {
            let lock = EngineLock::new(());
            let guard = lock.try_lock().expect("无竞争时立即可得");
            assert!(
                lock.try_lock().is_err(),
                "持锁期间不可再得（scan/deploy 串行）"
            );
            drop(guard);
            assert!(lock.try_lock().is_ok(), "释放后可再得");
        });
    }

    #[test]
    fn spawn_engine_透传结果与错误() {
        let value =
            tauri::async_runtime::block_on(spawn_engine(|| Ok::<_, EngineError>(42))).unwrap();
        assert_eq!(value, 42, "引擎结果应透传");

        let err = tauri::async_runtime::block_on(spawn_engine(|| -> Result<(), EngineError> {
            Err(EngineError::Config("测试错误".to_string()))
        }))
        .unwrap_err();
        assert!(matches!(err, EngineError::Config(_)), "引擎错误应透传");

        // 引擎任务 panic → JoinError → Internal 兜底（不向用户暴露堆栈）
        let err = tauri::async_runtime::block_on(spawn_engine(|| -> Result<(), EngineError> {
            panic!("引擎内部 panic")
        }))
        .unwrap_err();
        assert!(
            matches!(err, EngineError::Internal(_)),
            "panic 应映射为 Internal"
        );
    }
}

/// 应用路径（命令层 setup 解析注入；环境变量覆盖用于开发 / 测试 / 演示，
/// `docs/specs/s1-matrix.md` §1）。
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub vault_root: PathBuf,
    pub data_dir: PathBuf,
}

impl AppPaths {
    /// 解析路径：`SKILLS_KEEPER_VAULT` 覆盖 Vault 根、`SKILLS_KEEPER_DATA` 覆盖数据目录
    /// （db 文件位于其下）；未设置时回落默认 `~/.skills-keeper/`（Vault = 其下 `vault/`）。
    ///
    /// 相对路径按进程当前工作目录转绝对（`tauri dev` 下 cwd 为 `src-tauri/`，
    /// 演示时可传 `../examples/vault` 或绝对路径，见 `examples/vault/README.md`）。
    pub fn resolve() -> EngineResult<Self> {
        let data_dir = match std::env::var_os("SKILLS_KEEPER_DATA") {
            Some(p) => to_absolute(PathBuf::from(p)),
            None => expand_tilde("~/.skills-keeper")
                .ok_or_else(|| EngineError::Config("无法解析用户主目录".to_string()))?,
        };
        let vault_root = match std::env::var_os("SKILLS_KEEPER_VAULT") {
            Some(p) => to_absolute(PathBuf::from(p)),
            None => data_dir.join("vault"),
        };
        Ok(Self {
            vault_root,
            data_dir,
        })
    }
}

/// 相对路径按进程 cwd 转绝对（绝对路径原样返回）；cwd 不可得时原样兜底。
fn to_absolute(p: PathBuf) -> PathBuf {
    if p.is_absolute() {
        p
    } else {
        std::env::current_dir().map(|cwd| cwd.join(&p)).unwrap_or(p)
    }
}
