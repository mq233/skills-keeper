# Domain 文档

工程技能在探索代码库时应如何消费本仓库的领域文档。

## 探索前先读这些

- 仓库根目录的 **`CONTEXT.md`**，或
- 若存在根目录的 **`CONTEXT-MAP.md`** —— 它指向每个上下文各自的 `CONTEXT.md`。读取与主题相关的每一份。
- **`docs/adr/`** —— 阅读涉及你即将改动区域的 ADR。多上下文仓库中，也检查 `src/<context>/docs/adr/` 下上下文范围内的决策。

若以上文件均不存在，**静默继续**。不要标记它们的缺失，也不要主动建议创建。`/domain-modeling` 技能（通过 `/grill-with-docs` 与 `/improve-codebase-architecture` 触达）会在术语或决策真正落定时惰性创建它们。

## 文件结构

单上下文仓库（大多数仓库）：

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-event-sourced-orders.md
│   └── 0002-postgres-for-write-model.md
└── src/
```

多上下文仓库（根目录存在 `CONTEXT-MAP.md`）：

```
/
├── CONTEXT-MAP.md
├── docs/adr/                          ← 系统级决策
└── src/
    ├── ordering/
    │   ├── CONTEXT.md
    │   └── docs/adr/                  ← 上下文内决策
    └── billing/
        ├── CONTEXT.md
        └── docs/adr/
```

## 使用词汇表中的术语

当你的输出提到领域概念时（issue 标题、重构提案、假设、测试名），使用 `CONTEXT.md` 中定义的术语，不要漂移到词汇表明确回避的同义词。

若所需概念尚未出现在词汇表中，这是一个信号 —— 要么你在发明项目不使用的语言（重新考虑），要么存在真实缺口（记为 `/domain-modeling` 的任务）。

## 标记 ADR 冲突

若你的输出与现有 ADR 矛盾，明确指出而不是静默覆盖：

> _与 ADR-0007（event-sourced orders）冲突 —— 但值得重新讨论，因为……_
