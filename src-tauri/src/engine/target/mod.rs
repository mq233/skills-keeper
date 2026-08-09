//! 目标工具层：S1 最小路径层（ToolId + 默认 Skill 目录），适配器 trait 留 S2。
//!
//! 设计取向：行为入 trait、路径入配置（`docs/technical-plan.md` §4.2）。
//! S1 只读矩阵：接入判定 = 路径可得（`connected ⇔ default_skills_dir().is_some()`）；
//! trait 化（render/validate）与用户配置覆盖留 S2/S5，本层是 S5 设置页的天然扩展点
//! （见 `docs/specs/s1-matrix.md` §5）。

pub mod adapters;

use std::path::{Path, PathBuf};

/// 目标工具标识（契约序列化：kebab-case，如 `claude-code`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolId {
    /// Claude Code：`~/.claude/skills/`
    ClaudeCode,
    /// Codex（新版）：`~/.agents/skills/`
    Codex,
    /// WorkBuddy：官方未公开路径 → 未接入（None）
    Workbuddy,
    /// Trae（默认国际版）：`~/.trae/skills/`
    Trae,
}

impl ToolId {
    /// 四目标工具全集（矩阵列顺序）。
    pub const ALL: [ToolId; 4] = [
        ToolId::ClaudeCode,
        ToolId::Codex,
        ToolId::Workbuddy,
        ToolId::Trae,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ToolId::ClaudeCode => "claude-code",
            ToolId::Codex => "codex",
            ToolId::Workbuddy => "workbuddy",
            ToolId::Trae => "trae",
        }
    }

    /// 默认用户级 Skill 目录（`~` 已展开）；`None` = 未接入（S1 无用户配置覆盖）。
    pub fn default_skills_dir(&self) -> Option<PathBuf> {
        match self {
            ToolId::ClaudeCode => expand_tilde("~/.claude/skills"),
            ToolId::Codex => expand_tilde("~/.agents/skills"),
            ToolId::Workbuddy => None,
            ToolId::Trae => expand_tilde("~/.trae/skills"),
        }
    }

    /// 是否已接入（S1：路径可得即接入；未接入 = 引擎不扫描、矩阵列显示「未接入」）。
    pub fn connected(&self) -> bool {
        self.default_skills_dir().is_some()
    }
}

/// 字符串 → ToolId（契约 id 解析；未知 id → Err）。
impl std::str::FromStr for ToolId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "claude-code" => Ok(ToolId::ClaudeCode),
            "codex" => Ok(ToolId::Codex),
            "workbuddy" => Ok(ToolId::Workbuddy),
            "trae" => Ok(ToolId::Trae),
            _ => Err(()),
        }
    }
}

/// 把 `~` 前缀展开为用户主目录（Windows `USERPROFILE`，其他平台 `HOME`）；
/// 非 `~` 路径原样返回。`~` 目录不可得时返回 None。
pub fn expand_tilde(p: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    Some(expand_tilde_with(p, &home))
}

/// 内部实现（home 注入，便于测试）；`~` 只认 `~`、`~/`、`~\` 三种形式。
fn expand_tilde_with(p: &str, home: &Path) -> PathBuf {
    if p == "~" {
        return home.to_path_buf();
    }
    for sep in ["~/", "~\\"] {
        if let Some(rest) = p.strip_prefix(sep) {
            return home.join(rest);
        }
    }
    PathBuf::from(p)
}

fn home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    } else {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;
    use std::str::FromStr;

    #[test]
    fn tool_id_序列化与解析往返() {
        for id in ToolId::ALL {
            let s = id.as_str();
            assert_eq!(ToolId::from_str(s).ok(), Some(id), "{s}");
            assert_eq!(
                serde_json::to_string(&id).unwrap(),
                format!("\"{s}\""),
                "序列化应为 kebab-case"
            );
        }
        assert!(ToolId::from_str("unknown").is_err(), "未知 id 应解析失败");
    }

    #[test]
    fn 默认路径_workbuddy未接入_trae国际版() {
        for id in ToolId::ALL {
            let dir = id.default_skills_dir();
            assert_eq!(
                dir.is_some(),
                id.connected(),
                "接入判定应与路径可得一致：{id:?}"
            );
        }
        assert_eq!(
            ToolId::Workbuddy.default_skills_dir(),
            None,
            "WorkBuddy 官方未公开路径 → 未接入"
        );
        for (id, tail) in [
            (ToolId::ClaudeCode, ".claude/skills"),
            (ToolId::Codex, ".agents/skills"),
            (ToolId::Trae, ".trae/skills"),
        ] {
            let dir = id.default_skills_dir().unwrap();
            let s = dir.to_string_lossy().replace('\\', "/");
            assert!(s.ends_with(tail), "{id:?} 默认路径应为 ...{tail}，实际 {s}");
        }
    }

    #[test]
    fn expand_tilde_展开与保留() {
        let home = Path::new("C:/Users/test");
        assert_eq!(expand_tilde_with("~", home), PathBuf::from("C:/Users/test"));
        assert_eq!(
            expand_tilde_with("~/skills", home),
            PathBuf::from("C:/Users/test/skills")
        );
        assert_eq!(
            expand_tilde_with("~\\skills", home),
            PathBuf::from("C:/Users/test/skills")
        );
        // 非 ~ 路径原样
        assert_eq!(
            expand_tilde_with("relative/path", home),
            PathBuf::from("relative/path")
        );
        assert_eq!(
            expand_tilde_with("/abs/path", home),
            PathBuf::from("/abs/path")
        );
        // ~user 形式不展开（S1 不支持）
        assert_eq!(
            expand_tilde_with("~other/x", home),
            PathBuf::from("~other/x")
        );
    }
}
