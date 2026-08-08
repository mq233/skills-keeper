# Vault Schema 草案（原型）

> ⚠️ **PROTOTYPE** — 供 wayfinder ticket「设计技能内部 schema 与 Vault 目录结构」讨论确认的粗坯。
> 确认后结论并入最终技术规划文档（`docs/`），本目录是 throwaway 原型，不代表最终交付物。

配套样例：`sample-vault/`（含技能的完整布局，可对照阅读）。

## 决策基线（2026-08-08 grilling 确认）

1. **目录同构**：技能目录形态与目标工具端一致（`skills/<name>/SKILL.md`），分发=目录拷贝、导入=目录识别
2. **sidecar 格式**：JSON（`.skill-meta.json` / `.instruction-meta.json`）
3. **职责分工**：sidecar 只存意图性扩展元数据；name/description 从 frontmatter 读取（双事实源）；hash/状态归 SQLite 快照与引擎实时比对（双事实源）
4. **分发目标标记**：显式数组，导入/新建时默认全选已接入工具
5. **说明文件**：内容实体目录托管 + 工具映射，不进技能管线
6. **frontmatter 最小兼容集**：工具特有字段分发时注入、导入时清洗进 sidecar `extras` 备查

## 1. Vault 目录结构

```
Vault/
├── skills/                      # 技能库
│   └── <slug>/
│       ├── SKILL.md             # 规范格式本体
│       ├── <资源文件...>         # 与 SKILL.md 同级的辅助文件
│       └── .skill-meta.json     # sidecar
└── instructions/                # 说明文件库
    └── <slug>/
        ├── INSTRUCTION.md       # 内容本体（单一版本，全工具同文）
        └── .instruction-meta.json
```

- **slug 规则**：目录名即 slug，是文件系统标识；新建时从 `name` 自动生成（小写、非字母数字归一化为 `-`），导入时沿用工具端目录名；仅校验全局唯一与文件系统安全，不强制拉丁字符
- **frontmatter `name` 是展示名**，与目录名分离，保留原文

## 2. SKILL.md frontmatter 最小兼容集

```yaml
---
name: <技能名>          # 必填 —— 四工具（Claude Code/Codex/WorkBuddy/Trae）全部要求
description: <描述>    # 必填
version: <语义化版本>    # 可选
---
```

- Vault 内仅保留最小集，保持干净的标准形态
- **分发时**：适配器按目标工具注入特有字段（如 WorkBuddy 的 `allowed-tools`、`display_name`、`description_zh/en`）
- **导入时**：工具特有字段从 frontmatter 剥离，按来源工具存档到 sidecar `extras`（备查，不污染本体）

## 3. `.skill-meta.json`（schemaVersion 1）

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `schemaVersion` | `number` | ✓ | 当前 `1`，为演进留路 |
| `source` | `string \| null` | ✓ | 导入来源工具 id（`claude-code`/`codex`/`workbuddy`/`trae`）；本应用新建为 `null` |
| `targets` | `string[]` | ✓ | 分发目标标记，已接入工具的任意子集；导入/新建时初始化为**全部已接入工具**（默认全选） |
| `createdAt` | `string`（ISO 8601） | ✓ | 创建/导入时间 |
| `updatedAt` | `string`（ISO 8601） | - | `targets` 等意图变更时更新 |
| `extras` | `object` | - | 按来源工具分组的工具特有字段存档（见 §2 导入清洗） |

示例见 `sample-vault/skills/*/.skill-meta.json`。

## 4. `.instruction-meta.json`（schemaVersion 1）

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `schemaVersion` | `number` | ✓ | 当前 `1` |
| `targets` | `object` | ✓ | 工具 id → `{ filename, path }`；未接入/未配置的工具为 `null` |
| `createdAt` | `string`（ISO 8601） | ✓ | 创建时间 |

- 内容本体 `INSTRUCTION.md` 全工具同文（MVP）；per-tool 内容变体为后续能力，`schemaVersion` 预留演进
- 目标路径由适配器提供默认值（如 `~/.claude/CLAUDE.md`），用户可覆盖；WorkBuddy 路径官方未证实，用户配置后写入

示例见 `sample-vault/instructions/*/.instruction-meta.json`。

## 5. 职责分工（防双事实源）

| 载体 | 管什么 |
| --- | --- |
| SKILL.md frontmatter | 工具可见的展示信息：`name`、`description`、`version` |
| sidecar | 意图性扩展元数据：来源、分发目标标记、时间戳 |
| SQLite 快照 + 引擎实时比对 | hash、状态（Status）、分发历史 |
| 分发动作 | 排除 sidecar 与元数据文件，仅复制 SKILL.md + 资源 |

## 6. 关联与留后续

- **导入去重/冲突**：见 ticket「设计导入流程：识别、去重与目标标记」
- **适配器路径模板与目录选择**（Codex 双目录、Trae CN/国际版、WorkBuddy 可配置路径）：见 ticket「设计 Rust 分发引擎：适配器、状态模型、快照与回滚」
- **per-tool 说明文件变体**：后续能力，已留 `schemaVersion` 演进位
- **技能无独立 id**：目录路径即唯一标识（MVP）；重命名/移动由引擎负责同步 SQLite 记录
