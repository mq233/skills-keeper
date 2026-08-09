# api

Tauri command 调用封装层：统一 `invoke` 入口、`{code, message}` 错误解析。

前端仅通过本层与 Rust 引擎对话（不直接 `fetch`/裸 `invoke`），见 `docs/technical-plan.md` §4.7 与 §7 代码规范。
