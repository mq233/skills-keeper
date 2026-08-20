# Skills Keeper

跨平台桌面软件，在本地统一托管 AI 技能（Skills）与工具说明文件，并将其复制分发到 Claude Code、Codex、workbuddy、Trae 等 Agent 工具。

## Language

**托管库（Vault）**：
本地统一存储技能与说明文件的根目录，是应用的单一事实源。
_Avoid_: 仓库、库目录、storage

**Skill（技能）**：
`SKILL.md` + 资源文件组成的技能目录，是 Vault 内的最小托管单元，以 SKILL.md 为规范格式存储。
_Avoid_: skill 包、模板

**Sidecar（伴生元数据）**：
与技能或说明文件本体并存的 JSON 元数据文件（技能为 `.skill-meta.json`、说明文件为 `.instruction-meta.json`），存储意图性扩展元数据（来源、分发目标标记、创建时间等），不污染 SKILL.md 本体；分发时排除，仅留在 Vault 内。
_Avoid_: 配置文件、辅助文件

**说明文件（Instruction）**：
`CLAUDE.md` / `AGENTS.md` 这类工具级说明文本，按内容托管、按目标工具映射路径。
_Avoid_: 提示词文件、配置文件

**目标工具（Target）**：
已接入并可分发到的 Agent 工具（Claude Code、Codex、workbuddy、Trae）。
_Avoid_: 平台、通道、目的地

**未接入（Disconnected）**：
目标工具未配置可用的用户级目录路径（如 workbuddy 官方未公开路径、用户未配置），引擎不扫描、不可分发；状态矩阵中该列显示「未接入」并引导配置。
_Avoid_: 未配置、停用

**分发（Deploy）**：
把技能或说明文件复制到目标工具目录的动作。
_Avoid_: 同步（sync 保留给后续双向模型）

**导入（Import）**：
从目标工具目录将存量技能复制进 Vault 的动作；只读源目录、工具端零副作用，流程为识别 → 勾选 → 导入。
_Avoid_: 吸收、迁移、收纳

**扫描（Scan）**：
读取目标工具目录当前状态（技能目录、SKILL.md、文件 hash）并与 Vault 记录比对的只读动作；导入器打开与分发执行前各执行一次。
_Avoid_: 刷新、重载

**状态（Status）**：
技能 × 目标工具的匹配状态：`一致`（已分发且未变化）、`待分发`（Vault 已改未分发）、`被工具修改`（工具端文件被外部改动）、`缺失`（从未分发或已被删除）。
_Avoid_: 外部改动、冲突

**快照（Snapshot）**：
分发前自动保存（或手动触发）的目标工具端状态副本，文件存入快照目录、元数据写入 SQLite。
_Avoid_: 备份、存档

**回滚（Rollback）**：
从快照恢复目标工具端文件到记录状态。
_Avoid_: 还原、撤销
