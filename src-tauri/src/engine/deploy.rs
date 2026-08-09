//! 分发事务：渲染 → 快照 → staging → 落盘 → 记录。
//!
//! Phase 2 实现（tempdir 集成测试：失败回滚），见 `docs/technical-plan.md` §4.4 与 §4.1。
