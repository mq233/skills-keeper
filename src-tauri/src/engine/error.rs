//! EngineError 错误模型（`docs/technical-plan.md` §4.7）。
//!
//! 命令层将 `Result<T, EngineError>` 序列化为 `{code, message}` JSON，
//! message 为中文文案；Internal 不向用户暴露细节。

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Skill/快照/工具不存在
    #[error("{0}")]
    NotFound(String),
    /// 状态不允许该操作（如分发前扫描过期）
    #[error("{0}")]
    InvalidState(String),
    /// 文件系统错误
    #[error("{0}")]
    Io(String),
    /// 路径未配置（如 WorkBuddy）
    #[error("{0}")]
    Config(String),
    /// 校验失败（frontmatter 缺 name 等）
    #[error("{0}")]
    InvalidSkill(String),
    /// 工具未接入等
    #[error("{0}")]
    Unsupported(String),
    /// 兜底，不向用户暴露细节
    #[error("内部错误")]
    Internal(String),
}

pub type EngineResult<T> = Result<T, EngineError>;
