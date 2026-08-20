//! 目标工具层：ToolId（纯标识）+ ToolAdapter trait + AdapterRegistry（S2 适配器层）。
//!
//! 设计取向：行为入 trait、路径入配置（`docs/technical-plan.md` §4.2）；
//! S2 决议「适配器 trait 与 render_skill 范围」——路径与接入判定移入适配器，
//! S1 的最小路径层（`ToolId::default_skills_dir` / `connected`）收敛为注册表查询。
//! S5 设置页换配置驱动（用户路径覆盖 = 覆盖适配器路径），本层是天然扩展点。

pub mod adapters;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::engine::error::EngineError;
use crate::engine::vault::Skill;

/// 目标工具标识（契约序列化：kebab-case，如 `claude-code`）。
/// S2 收敛为纯标识：路径与接入判定在适配器，本类型只负责 id 语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
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

/// 渲染产物：render 输出、validate 校验、落盘复制的完整内容。
/// `resources` 为 Skill 目录内资源文件（Vault 读取时收集，render 原样透传）；
/// `extra_files` 为渲染生成的伴生文件（如 Codex `agents/openai.yaml`，S2 恒空预留）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedSkill {
    /// 重写后的完整 SKILL.md（frontmatter 注入/剥离后 + 正文）
    pub skill_md: String,
    /// 需随 Skill 复制的资源文件（相对 Skill 目录；不含 SKILL.md 与隐藏文件）
    pub resources: Vec<PathBuf>,
    /// 渲染生成的伴生文件（相对 Skill 目录 + 内容）
    pub extra_files: Vec<(PathBuf, String)>,
}

/// 目标工具适配器 trait（S2 决议 A）：行为入 trait、路径入适配器。
/// `default_skills_dir() -> None` = 未接入（引擎不扫描、不可分发）。
/// `Send + Sync`：注册表作为 Tauri state 注入（命令层共享）。
pub trait ToolAdapter: Send + Sync {
    fn id(&self) -> ToolId;
    fn default_skills_dir(&self) -> Option<PathBuf>;
    /// 渲染 Skill 为分发产物（纯函数，无文件系统访问——资源清单来自 Skill）。
    fn render_skill(&self, skill: &Skill) -> Result<RenderedSkill, EngineError>;
    /// 落盘前校验（只查必败项；不合规 → `InvalidSkill`）。
    fn validate(&self, rendered: &RenderedSkill) -> Result<(), EngineError>;
}

/// 工具端根目录解析结果（命令层 / 引擎门面使用；None = 未接入，引擎不扫描）。
pub type ToolRoots = Vec<(ToolId, Option<PathBuf>)>;

/// 适配器注册表：静态构造四适配器（含 WorkBuddy），connected 过滤由调用方按
/// 目录可得判定。路径覆盖表 = S5 设置页用户配置的天然扩展点（覆盖适配器默认路径；
/// S2 静态构造，测试注入自定义工具目录用）。
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn ToolAdapter>>,
    overrides: HashMap<ToolId, PathBuf>,
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AdapterRegistry {
    /// 全注册四适配器（Claude Code / Codex / WorkBuddy / Trae，矩阵列顺序）。
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(adapters::ClaudeCodeAdapter),
                Box::new(adapters::CodexAdapter),
                Box::new(adapters::WorkbuddyAdapter),
                Box::new(adapters::TraeAdapter),
            ],
            overrides: HashMap::new(),
        }
    }

    /// 带路径覆盖构造（测试 / S5 用户配置）：覆盖表命中时以覆盖路径为准。
    pub fn with_overrides(overrides: HashMap<ToolId, PathBuf>) -> Self {
        let mut registry = Self::new();
        registry.overrides = overrides;
        registry
    }

    /// 全部适配器（矩阵列顺序）。
    pub fn adapters(&self) -> &[Box<dyn ToolAdapter>] {
        &self.adapters
    }

    /// 按 id 取适配器（未知 id → None）。
    pub fn get(&self, id: ToolId) -> Option<&dyn ToolAdapter> {
        self.adapters
            .iter()
            .find(|a| a.id() == id)
            .map(|a| a.as_ref())
    }

    /// 目录解析：覆盖表优先（S5 用户配置），否则适配器默认路径。
    fn dir_of(&self, adapter: &dyn ToolAdapter) -> Option<PathBuf> {
        self.overrides
            .get(&adapter.id())
            .cloned()
            .or_else(|| adapter.default_skills_dir())
    }

    /// 已接入适配器（路径可得；未接入 = 引擎不扫描、矩阵列显示「未接入」）。
    pub fn connected(&self) -> Vec<&dyn ToolAdapter> {
        self.adapters
            .iter()
            .filter(|a| self.dir_of(a.as_ref()).is_some())
            .map(|a| a.as_ref())
            .collect()
    }

    /// 工具端根目录全集（(id, Option<dir>)；None = 未接入）。
    pub fn tool_roots(&self) -> ToolRoots {
        self.adapters
            .iter()
            .map(|a| (a.id(), self.dir_of(a.as_ref())))
            .collect()
    }

    /// 已接入工具 id 列表（Sidecar 默认 targets 合成等）。
    pub fn all_connected_targets(&self) -> Vec<String> {
        self.connected()
            .iter()
            .map(|a| a.id().as_str().to_string())
            .collect()
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
