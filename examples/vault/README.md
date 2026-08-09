# 样例 Vault

S1 验收演示用样例 Vault（`docs/specs/s1-matrix.md` §验收演示）：
在不污染真实 `~/.skills-keeper/vault/` 的前提下体验真实矩阵。

## 用法

```bash
SKILLS_KEEPER_VAULT=examples/vault pnpm tauri dev
```

（Windows PowerShell：`$env:SKILLS_KEEPER_VAULT="examples/vault"; pnpm tauri dev`）

数据目录仍走默认 `~/.skills-keeper/`（db 文件与真实数据隔离，仅 Vault 指向样例）。
矩阵中工具端状态来自本机真实工具目录（如实显示），WorkBuddy 显示「未接入」列。

## 覆盖分支

| Skill            | slug           | 分支                                                  |
| ---------------- | -------------- | ----------------------------------------------------- |
| `greeting`       | 问候助手       | 正常英文名 + 版本（`version: 1.0` YAML 标量陷阱）     |
| `中文技能`       | 中文写作助手   | 中文名目录 + Sidecar 来源标记（codex）                |
| `code-assistant` | Code Assistant | 含资源文件（`scripts/helper.py`），目录 hash 含子目录 |
| `broken-skill`   | （缺 name）    | invalid 分支：缺 name → 行级标记                      |
