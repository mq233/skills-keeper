# Skills Keeper

跨平台桌面软件，本地统一托管你的 AI 技能（Skills）与配置，一键分发到 Codex、Claude Code、Trae 等各 Agent 工具。

## 交互要求

- 涉及 `CONTEXT.md` 术语表词汇时，以条目主词 AA 为输出描述词，必要时完整输出「AA(BB)」；不使用括号内附注词或 Avoid 词替代。如「Skill（技能）」条目输出用「Skill」，需要消歧时输出「Skill（技能）」

## Agent skills

### Issue tracker

本仓库的 issue 与规格说明以 GitHub Issues 形式托管，通过 `gh` CLI 操作。参见 `docs/agents/issue-tracker.md`。

### Domain docs

单上下文布局：仓库根目录一份 `CONTEXT.md` + `docs/adr/`。参见 `docs/agents/domain.md`。
