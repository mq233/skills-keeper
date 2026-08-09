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
