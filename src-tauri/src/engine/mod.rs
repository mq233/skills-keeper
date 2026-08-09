//! Rust 核心引擎门面：组合各模块，供命令层调用。
//!
//! 分层规则：engine 不依赖 Tauri，可纯单元测试（`docs/technical-plan.md` §2）。
//! 各子模块按 §4.1 划分，Phase 1–3 逐个实现。

pub mod deploy;
pub mod error;
pub mod import;
pub mod rollback;
pub mod scanner;
pub mod snapshot;
pub mod status;
pub mod target;
pub mod vault;
