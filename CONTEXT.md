# Skills Keeper

跨平台桌面软件，在本地统一托管 AI 技能（Skills）与工具说明文件，并将其复制分发到 Claude Code、Codex、workbuddy、Trae 等 Agent 工具。

## Language

**托管库（Vault）**：
本地统一存储技能与说明文件的根目录，是应用的单一事实源。
_Avoid_: 仓库、库目录、storage

**技能（Skill）**：
`SKILL.md` + 资源文件组成的技能目录，是 Vault 内的最小托管单元，以 SKILL.md 为规范格式存储。
_Avoid_: skill 包、模板

**说明文件（Instruction）**：
`CLAUDE.md` / `AGENTS.md` 这类工具级说明文本，按内容托管、按目标工具映射路径。
_Avoid_: 提示词文件、配置文件

**目标工具（Target）**：
已接入并可分发到的 Agent 工具（Claude Code、Codex、workbuddy、Trae）。
_Avoid_: 平台、通道、目的地

**分发（Deploy）**：
把技能或说明文件复制到目标工具目录的动作。
_Avoid_: 同步（sync 保留给后续双向模型）

**状态（Status）**：
技能 × 目标工具的匹配状态：`一致`（已分发且未变化）、`待分发`（Vault 已改未分发）、`被工具修改`（工具端文件被外部改动）、`缺失`（从未分发或已被删除）。
_Avoid_: 外部改动、冲突

**快照（Snapshot）**：
分发前自动保存（或手动触发）的 Vault 状态副本，存入 SQLite。
_Avoid_: 备份、存档

**回滚（Rollback）**：
从快照恢复目标工具端文件到记录状态。
_Avoid_: 还原、撤销
