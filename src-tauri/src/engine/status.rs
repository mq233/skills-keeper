//! 状态判定：一致 / 待分发 / 被工具修改 / 缺失（`docs/technical-plan.md` §4.3 判定矩阵）。
//!
//! S1 实现：纯函数 `compute(t, r, v)` 覆盖判定矩阵全分支（spec §8）；
//! r 缺失时走 t == v 分支（无分发记录 + 内容一致 → 一致）。

/// 状态值（契约序列化：snake_case）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// 与上次分发一致，Vault 未变
    Consistent,
    /// 上次分发后 Vault 改了
    Pending,
    /// 工具端被外部改动
    Modified,
    /// 从未分发或已被删除（工具端目录不存在）
    Missing,
}

/// 判定输入：`t` = 工具端当前目录 hash，`r` = SQLite 分发记录 hash，`v` = Vault 当前目录 hash。
///
/// 判定矩阵（§4.3）：
/// - `t` 不存在（工具端目录不存在）→ 缺失
/// - `t == r` 且 `v == r` → 一致
/// - `t == r` 且 `v != r` → 待分发
/// - `t != r` 且 `t == v` → 一致（记录过期，下次分发刷新即可）
/// - `t != r` 且 `t != v` → 被工具修改
/// - `r` 缺失（无分发记录）→ 走 `t == v` 分支
pub fn compute(t: Option<&str>, r: Option<&str>, v: Option<&str>) -> Status {
    let Some(t) = t else {
        return Status::Missing;
    };
    match r {
        Some(r) if t == r => {
            if v == Some(r) {
                Status::Consistent
            } else {
                Status::Pending
            }
        }
        _ => {
            // t != r，或 r 缺失：走 t == v 分支
            if Some(t) == v {
                Status::Consistent
            } else {
                Status::Modified
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;

    /// 用例表：`(t, r, v) → 预期状态`，覆盖判定矩阵全分支（spec §Testing Decisions）。
    type Case = (
        Option<&'static str>,
        Option<&'static str>,
        Option<&'static str>,
        Status,
    );

    #[test]
    fn 判定矩阵全分支() {
        let cases: Vec<Case> = vec![
            // t 不存在 → 缺失（与 r/v 无关）
            (None, None, None, Status::Missing),
            (None, Some("r"), Some("v"), Status::Missing),
            // t == r 且 v == r → 一致
            (Some("h"), Some("h"), Some("h"), Status::Consistent),
            // t == r 且 v != r → 待分发
            (Some("h"), Some("h"), Some("other"), Status::Pending),
            // t != r 且 t == v → 一致（记录过期分支）
            (Some("h"), Some("old"), Some("h"), Status::Consistent),
            // t != r 且 t != v → 被工具修改
            (Some("h"), Some("old"), Some("other"), Status::Modified),
            // r 缺失（无分发记录）：
            //   t == v → 一致（spec User Story 15）
            (Some("h"), None, Some("h"), Status::Consistent),
            //   t != v → 被工具修改
            (Some("h"), None, Some("other"), Status::Modified),
            // 边界：v 缺失（t 存在时 v 通常已计算，防御分支）
            (Some("h"), Some("h"), None, Status::Pending),
            (Some("h"), Some("old"), None, Status::Modified),
            (Some("h"), None, None, Status::Modified),
        ];
        for (t, r, v, expected) in cases {
            assert_eq!(
                compute(t, r, v),
                expected,
                "compute({t:?}, {r:?}, {v:?}) 应得 {expected:?}"
            );
        }
    }

    #[test]
    fn 状态序列化为snake_case() {
        assert_eq!(
            serde_json::to_string(&Status::Consistent).unwrap(),
            "\"consistent\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Modified).unwrap(),
            "\"modified\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Missing).unwrap(),
            "\"missing\""
        );
    }
}
