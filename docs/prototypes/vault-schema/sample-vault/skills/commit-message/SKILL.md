---
name: commit-message
description: 根据暂存区变更生成符合 Conventional Commits 规范的提交信息
version: 0.1.0
---

# commit-message

根据 git 暂存区变更，生成符合 Conventional Commits 规范的提交信息草案。

## 用法

- **输入**：`git diff --cached` 的输出
- **输出**：建议的提交信息（type(scope): subject + 正文要点）

## 规则

- type 从 `feat`/`fix`/`refactor`/`docs`/`chore` 中选择
- 变更跨多个关注点时分条列出
