//! EngineError 错误模型（`docs/technical-plan.md` §4.7）。
//!
//! 命令层将 `Result<T, EngineError>` 序列化为 `{code, message}` JSON，
//! message 为中文文案；Internal 不向用户暴露细节（`docs/specs/s1-matrix.md` §9）。

/// 错误序列化形状：`{ code: 变体名, message: 中文文案 }`。
impl serde::Serialize for EngineError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("EngineError", 2)?;
        s.serialize_field("code", &self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

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

impl EngineError {
    /// 错误码 = 变体名（前端契约：`{code, message}`，code 即本值）。
    pub fn code(&self) -> &'static str {
        match self {
            EngineError::NotFound(_) => "NotFound",
            EngineError::InvalidState(_) => "InvalidState",
            EngineError::Io(_) => "Io",
            EngineError::Config(_) => "Config",
            EngineError::InvalidSkill(_) => "InvalidSkill",
            EngineError::Unsupported(_) => "Unsupported",
            EngineError::Internal(_) => "Internal",
        }
    }
}

pub type EngineResult<T> = Result<T, EngineError>;

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;

    #[test]
    fn 序列化为code与message() {
        let err = EngineError::NotFound("Skill 不存在".to_string());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "NotFound");
        assert_eq!(json["message"], "Skill 不存在");
    }

    #[test]
    fn internal不暴露细节() {
        let err = EngineError::Internal("底层细节".to_string());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["message"], "内部错误", "Internal 兜底不向用户暴露细节");
    }

    #[test]
    fn 各变体code正确() {
        let cases: Vec<(EngineError, &str)> = vec![
            (EngineError::NotFound(String::new()), "NotFound"),
            (EngineError::InvalidState(String::new()), "InvalidState"),
            (EngineError::Io(String::new()), "Io"),
            (EngineError::Config(String::new()), "Config"),
            (EngineError::InvalidSkill(String::new()), "InvalidSkill"),
            (EngineError::Unsupported(String::new()), "Unsupported"),
            (EngineError::Internal(String::new()), "Internal"),
        ];
        for (err, code) in cases {
            assert_eq!(err.code(), code);
        }
    }
}
