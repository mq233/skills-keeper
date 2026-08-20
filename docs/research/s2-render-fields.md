# S2 调研：四目标工具 SKILL.md 字段要求核查（render 范围）

> 对应 ticket：[mq233/skills-keeper #27 S2 调研：四目标工具 SKILL.md 字段要求核查（render 范围）](https://github.com/mq233/skills-keeper/issues/27)（Part of #26，wayfinder 地图「S2 分发（适配器 + 分发事务 + 矩阵分发交互）」）
> 调研日期：2026-08-20。所有结论来自当日在官方文档 / 官方仓库 / 官方 CDN 抓取的一手来源，逐条标注来源；无法证实的内容明确标注「未证实」。
> 前置基线：`docs/research/target-tools-mechanisms.md`、`docs/research/workbuddy-format.md`（2026-08-08 调研），本文第 4 节给出差异比对。

---

## 0. 调研问题与结论速览

调研问题：`render_skill` 的字段注入范围——当前各目标工具对 SKILL.md frontmatter 的字段要求与注入规范是什么；S2 的 render 应为「排除 Sidecar 原样复制」还是「按工具注入特有字段」。

**结论速览（详见第 5 节）**：

1. **推荐「按工具注入特有字段」中的最小注入集**，而非「原样复制」。原样复制无法保证 Codex/Trae 的硬性要求（`name` + `description` 必填，缺失会被跳过/失败），也无法适配 WorkBuddy 国际化扩展字段。
2. **通用注入（三工具共同底线）**：保证 frontmatter 存在且含 `name`（与分发目录名一致，小写+连字符）与 `description`（what + when，触发面）。Vault 本体已按此校验（决议 #2）。
3. **Codex**：无额外 SKILL.md 字段需注入；可选的 UI/策略元数据放 `agents/openai.yaml`（目录级伴生文件，非 frontmatter）。
4. **Trae**：无额外字段需注入，`name` + `description` 即满足全部要求。
5. **WorkBuddy（未接入，按 trait 全集预留）**：注入 `description_zh`/`description_en`、`display_name`、`allowed-tools`、`version`（官方市场包实证字段，格式见 3.4）。
6. **不要默认注入 Claude Code 行为字段**（`when_to_use`、`user-invocable`、`context`、`model`、`hooks`、`paths` 等）：对 Codex/Trae 是未知键（Codex 启动时严格解析，缺失/非法 frontmatter 会跳过技能），且会改变工具行为。仅当 Sidecar 显式声明且目标为 Claude Code 时才注入。
7. **分发动作排除 Sidecar 与元数据文件**（`.skill-meta.json`、`agents/openai.yaml` 之外仅复制 SKILL.md + 资源）——注意 `agents/openai.yaml` 属于 Codex 分发目标的合法伴生文件，属于「注入」范畴而非 Sidecar。

---

## 1. 公共基准：Agent Skills 开放规范（agentskills.io）

四工具（Claude Code / Codex / Trae / WorkBuddy）均声明兼容 Agent Skills 生态，规范是跨工具分发的安全交集基准。

- 规范位置：https://agentskills.io/specification （Anthropic 维护，2025-12 随 Codex 正式支持 SKILL.md 同期发布；anthropics/skills 仓库 `spec/agent-skills-spec.md` 已重定向到该地址）
- **frontmatter 字段表**（来源：[agentskills.io/specification](https://agentskills.io/specification)，2026-08-20 抓取）：

| 字段 | 必填 | 约束 |
| --- | --- | --- |
| `name` | **是** | 1–64 字符；仅小写字母/数字/连字符；不得以连字符开头或结尾；不得含连续连字符；**必须与父目录名一致** |
| `description` | **是** | 1–1024 字符，非空；描述技能做什么 + 何时用 |
| `license` | 否 | 许可证名或引用许可证文件 |
| `compatibility` | 否 | ≤500 字符；环境要求（目标产品、系统包、网络等） |
| `metadata` | 否 | 字符串键值映射；客户端可存规范外的附加属性，建议键名唯一 |
| `allowed-tools` | 否 | 空格分隔的预授权工具列表（**实验性**，各客户端支持度不一） |

- 渐进披露：`metadata`（约 100 tokens，name+description 启动时注入）→ `instructions`（SKILL.md 正文 <5000 tokens，激活时加载）→ `resources`（按需加载）；SKILL.md 建议 <500 行。
- 校验工具：`skills-ref validate ./my-skill`（官方参考实现）。

---

## 2. Claude Code（Anthropic）

来源：https://code.claude.com/docs/en/skills（2026-08-20 抓取）、https://github.com/anthropics/skills（README + template/SKILL.md，2026-08-20 抓取）。

### 2.1 frontmatter 字段要求

**全部字段可选，仅 `description` 被推荐**（官方原文："All fields are optional. Only description is recommended so Claude knows when to use the skill."）。

| 类别 | 字段 | 说明 |
| --- | --- | --- |
| Agent Skills 标准字段（跨工具） | `name`、`description`、`license`、`compatibility`、`metadata`、`allowed-tools` | 六个字段完整支持；`name` 缺省时列表显示名默认目录名 |
| Claude Code 扩展字段 | `when_to_use`、`argument-hint`、`arguments`、`disable-model-invocation`、`user-invocable`、`disallowed-tools`、`model`、`effort`、`context`（fork）、`agent`、`background`、`hooks`、`paths`、`shell` | 均为可选；其中 `paths` 按文件路径门控自动加载、`model`/`hooks`/`context` 等会改变行为 |

约束与细节：

- `description` + `when_to_use` 合并后列表显示截断 1536 字符；`description` 缺省时取正文第一段。
- 命令名来自**目录名**（个人/项目技能：`.claude/skills/deploy-staging/SKILL.md` → `/deploy-staging`）；frontmatter `name` 只影响列表显示名（插件技能除外）。
- 布尔字段接受 `yes/no/on/off/1/0` 及 `true/false`（v2.1.218 起）。
- **分发限制**：claude.ai 上传、Skills API、`package_skill.py` 打包**只允许标准六字段**，出现任何扩展字段会报硬错误：`Unexpected key(s) in SKILL.md frontmatter: argument-hint. Allowed properties are: allowed-tools, compatibility, description, license, metadata, name`。
- **分发到 Claude Code 本地目录无任何注入要求**：放入 `~/.claude/skills/<name>/SKILL.md`（Windows `%USERPROFILE%\.claude\skills\`）即生效，符号链接亦可，支持热加载。
- anthropics/skills 官方仓库 README：「The frontmatter requires only two fields: `name`, `description`」；官方 template 即最小两字段（来源：[anthropics/skills README](https://github.com/anthropics/skills)）。

### 2.2 对 render 的含义

- 分发到 Claude Code 不需要注入任何字段；`name`+`description` 最小集即可。
- Claude Code 是**唯一会主动读取并执行**扩展行为字段（`model`、`hooks`、`context: fork`、`paths` 等）的工具——Sidecar 若携带此类字段且分发到 Claude Code，行为会被激活；分发到其他工具则被忽略（甚至被 Codex 严格解析告警，见 3.3）。
- 若未来走 Skills API / claude.ai 分发路径，frontmatter 必须限定标准六字段。

---

## 3. Codex（OpenAI）

来源：https://learn.chatgpt.com/docs/build-skills（官方文档，2026-08-20 发布版；developers.openai.com/codex/skills 同文重定向）、https://github.com/openai/skills（官方技能仓库，2026-08-20 抓取）。

### 3.1 frontmatter 字段要求

**`name` 与 `description` 为必填**（官方原文："The `SKILL.md` file must include `name` and `description`"）。

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `name` | **是** | 技能唯一标识；缺失/非法会导致技能被跳过 |
| `description` | **是** | 隐式触发匹配面；官方建议前置关键用例与触发词（描述被缩短时仍能匹配） |
| 其他字段 | — | 新版文档未列允许/禁止清单；Codex 特有元数据应放 `agents/openai.yaml`（见 3.2），而非 frontmatter |

官方示例（最小合法集）：

```markdown
---
name: skill-name
description: Explain exactly when this skill should and should not trigger.
---
```

- **旧版（2025 末 ~/.codex/skills 时代）长度限制**：`name` ≤100 字符单行、`description` ≤500 字符单行（来源：旧版 [docs/skills.md](https://raw.githubusercontent.com/openai/codex/2c6995ca4dfc23b93db311b59c1b4ead464658b1/docs/skills.md)）。**新版文档已不出现该限制**；开放规范约束为 `name` ≤64 / `description` ≤1024（见第 1 节）。
- **严格解析**：Codex 递归扫描 `.agents/skills/` 下全部 `SKILL.md`，缺失 YAML frontmatter 的文件在启动时被跳过并警告（"Skipped loading X skill(s) due to invalid SKILL.md files"）；未知/废弃字段（如 Claude Code 扩展字段 `user-invocable`、`context: fork`）在新版被归为反模式（来源：Codex 官方 build-skills 文档 + 2026-08-20 社区复述，后者仅作交叉印证）。

### 3.2 伴生文件 `agents/openai.yaml`

目录级可选伴生文件（**不是** SKILL.md frontmatter），Codex 官方推荐在此声明 UI 元数据与依赖：

```yaml
interface:
  display_name: "Optional user-facing name"
  short_description: "Optional user-facing description"
  icon_small: "./assets/small-logo.svg"
  icon_large: "./assets/large-logo.png"
  brand_color: "#3B82F6"
  default_prompt: "Optional surrounding prompt to use the skill with"

policy:
  allow_implicit_invocation: false   # 默认 true

dependencies:
  tools:
    - type: "mcp"
      value: "openaiDeveloperDocs"
      description: "OpenAI Docs MCP server"
      transport: "streamable_http"
      url: "https://developers.openai.com/mcp"
```

官方仓库 [openai/skills](https://github.com/openai/skills)（新仓库，Codex 文档引用的技能示例源）实际技能（如 `skills/.curated/cli-creator/`）的 SKILL.md frontmatter 仅 `name` + `description` 两字段，`agents/openai.yaml` 单独存放（2026-08-20 抓取验证）。

### 3.3 目录位置与分发注意

- 技能搜索：`$CWD/.agents/skills` → `$CWD/../.agents/skills` → `$REPO_ROOT/.agents/skills` → `$HOME/.agents/skills`（用户级）→ `/etc/codex/skills` → SYSTEM 内置；同名不合并；支持符号链接。
- 渐进披露：启动注入每个技能的 `name`、`description` 与**绝对路径**（列表预算 ≤2% 上下文窗口或 8000 字符）。
- 禁用：`~/.codex/config.toml` 中 `[[skills.config]]`（`path` + `enabled = false`）。
- **2026 年中后新动向（S1 基线未覆盖）**：官方文档明确 Skills 是 authoring 格式、**plugins 是分发单位**（universal plugin directory，ChatGPT 与 Codex 共享）；「分发可复用技能」的首选路径是打包为 plugin，而非直接散装 SKILL.md。

### 3.4 对 render 的含义

- Codex 目标必须保证 frontmatter 含 `name` + `description`（缺则技能被跳过/警告）——这是「按工具注入」的最少必要项。
- `name` 受开放规范约束须与**分发目录名一致**（小写+连字符、无连续连字符）；render 时若 Sidecar 指定了与目录名不同的 name，需以目录名（slug）为准或显式警告。
- 不得把 Claude Code 行为字段注入到 Codex 目标的 frontmatter；Codex 特有 UI/策略元数据应生成为 `agents/openai.yaml`（属于分发目标的合法伴生文件，不应被当作 Sidecar 排除）。

---

## 4. Trae / TraeCode（字节跳动）

来源：https://docs.trae.ai/ide/skills（国际版官方文档，2026-08-20 抓取）。

### 4.1 frontmatter 字段要求

**frontmatter 仅两个字段：`name` 与 `description`**（官方模板原样）：

```markdown
---
name: skill name
description: briefly describe what the skill does and when to use it
---
```

| 字段 | 必填（按模板语义） | 官方说明原文 |
| --- | --- | --- |
| `name` | 是 | "Give this skill a short and distinctive name." |
| `description` | 是 | "Briefly describe what the skill is and when this skill should be used." |

- 官方文档未列出任何其他 frontmatter 字段，也未提及长度限制或未知字段的处理方式。
- 目录结构：`SKILL.md`（Required: Core instructions）+ `examples/`、`templates/`、`resources/`（均 Optional）。
- 正文结构模板：`# Skill name`、`## Description`、`## When to use`、`## Instructions`、`## Examples (optional)`。
- 加载模型：按需加载——启动只扫描全部技能的简要描述，判定高度相关才加载全文。
- 内置技能：`TRAE-generate-mini-app`、`TRAE-debugger`、`TRAE-code-review`。
- 目录位置：全局 `~/.trae/skills`（Windows `%userprofile%/.trae/skills`；CN 版 `.trae-cn`）；项目 `<project>/.trae/skills/`；`.agents/skills/` 目录需在设置中打开「启用 .agents 技能目录」开关。

### 4.2 对 render 的含义

- Trae 只消费 `name` + `description`，两者齐备即满足；**无需注入任何 Trae 特有字段**（官方文档不存在此类字段）。
- Trae 文档未声明缺字段的失败行为，但按其模板语义应视为必填；保证两字段存在即覆盖。

---

## 5. WorkBuddy（腾讯；当前未接入，按 trait 全集预留）

来源：https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Skills-Market（官方文档，2026-08-20 抓取）、https://www.codebuddy.cn/docs/workbuddy/Changelog（官方更新日志，2026-08-20 抓取，最新 5.3.13/2026-08-13）、官方技能市场 CDN 包 https://download.codebuddy.cn/skill-marketplace/skill-marketplace.zip（2026-08-20 下载实测）、https://www.codebuddy.ai/docs/cli/skills（CodeBuddy CLI 同源文档，2026-08-20 抓取）。

### 5.1 官方文档口径

- 官方「技能」文档只描述安装（上传/查找/创建）、启用/关闭、搜索、卸载与安全提示，**未公开 frontmatter 字段规范**（与 S1 基线一致）。
- 更新日志中与格式相关的条目：「Skill 描述国际化，根据系统语言自动切换中/英文」（v4.6.1）、「优化技能描述优先级，优先采用 SKILL.md frontmatter 中的值」（v4.x）；2026-06 之后（v5.3.11–5.3.13）的技能条目均为 UI/修复，无格式变化。

### 5.2 官方市场包实测字段（一手实证，2026-08-20 下载，295 个技能包）

与 S1 基线（2026-06-25 版包）统计**完全一致**（数量 295 未变）：

| 字段 | 出现数（/295） | 格式要求（实测样例） |
| --- | --- | --- |
| `name` | 294 | 与目录名对应；全小写+连字符（12306 等数字开头亦存在） |
| `description` | 288 | 单行英文触发描述（what + when） |
| `version` | 278 | 语义化版本（如 `1.0.2`） |
| `description_zh` / `description_en` | 274 / 274 | 按系统语言切换的中/英文描述，可带引号 |
| `allowed-tools` | 96 | **空格或逗号分隔**均可（`Bash,Read`、`Read, Write, Edit, Bash, Glob, Grep`） |
| `display_name` / `display_name_en` | 87 / 83 | 显示名，可带引号（`"网页自动化"`） |
| `homepage` | 85 | 项目主页 URL |
| `metadata` | 80 | 任意 YAML 映射（常含 `openclaw` 兼容配置） |
| `visibility` | 71 | 市场可见性 |
| `author` / `license` / `category` / `icon` 等 | 少量 | 其他可选字段 |

实测样例（skills/12306/SKILL.md）：

```markdown
---
name: "12306"
description: Query China Railway 12306 for train schedules, remaining tickets, and station info. Use when user asks about train/高铁/火车 tickets, schedules, or availability within China.
description_zh: "查询 12306 国内列车时刻、余票与站点信息"
description_en: "Query China Railway 12306 train schedules and ticket availability"
version: 1.0.2
allowed-tools: Bash,Read
metadata:
  openclaw:
    emoji: "🚄"
    requires:
      bins:
        - node
---
```

### 5.3 CodeBuddy CLI 同源文档（补充字段语义）

CodeBuddy CLI（同团队、WorkBuddy 同源）技能文档的 frontmatter 字段：`name`、`description`、`allowed-tools`（逗号分隔允许列表）、`disable-model-invocation`、`user-invocable`、`context`（fork）、`agent`、`model`、`hooks`——**全部可选**，并提供 `${CODEBUDDY_SKILL_DIR}`、`${CODEBUDDY_SESSION_ID}` 等占位符与 `CLAUDE_SKILL_DIR` 兼容别名（来源：https://www.codebuddy.ai/docs/cli/skills）。

### 5.4 对 render 的含义

- WorkBuddy 未接入（官方未公开磁盘路径等，见 S1 基线「未证实项」），但 `render_skill` 应**按 trait 全集预留**以下注入字段：`description_zh`、`description_en`、`display_name`（可选 `display_name_en`）、`allowed-tools`、`version`。
- `allowed-tools` 是跨工具安全字段（Agent Skills 标准字段之一，Claude Code 亦读取），可在 WorkBuddy 目标注入；格式按市场惯例用逗号或空格分隔均可。
- WorkBuddy 目标注入应在**其工具特有字段不存在的场合省略对应键**（如无中文描述则不注入 `description_zh`，避免空值），保持「注入即真实数据」。

---

## 6. 与 S1 基线（2026-08-08）的差异比对

| 主题 | S1 基线结论 | 本次核查（2026-08-20 一手来源） | 判定 |
| --- | --- | --- | --- |
| Claude Code 字段 | 全部可选、仅 description 推荐；标准六字段 + 扩展字段 | 一致；新增细节：布尔字段新格式（yes/no/on/off/1/0）、`background` 字段（v2.1.218+）、`name` 缺省默认目录名；claude.ai/Skills API 硬性六字段限制（含报错原文）确认 | **成立** |
| Claude Code 官方仓库 | 未引用 anthropics/skills | anthropics/skills README：「frontmatter 仅要求 name + description 两字段」；template 即最小两字段；spec 已重定向 agentskills.io | 补充 |
| Codex 目录体系 | `.agents/skills` 为用户级位置；旧 `~/.codex/skills` 废弃兼容 | 一致；文档新增 `$CWD/../.agents/skills`、`/etc/codex/skills`、SYSTEM 层的完整作用域表 | **成立**（补全） |
| Codex 字段要求 | name + description 必填；旧版 name ≤100 / description ≤500 字符单行、其余键忽略 | 必填不变；**新版文档已无长度限制表述**（开放规范改为 name ≤64 / description ≤1024）；「其余键忽略」→ 严格解析（缺 frontmatter 启动警告跳过，未知/废弃字段归为反模式） | **需更新**（长度限制与解析行为） |
| Codex 分发 | 未提及 plugin 分发 | **新动向**：官方文档明确 Skills 是 authoring 格式、plugins 是分发单位（ChatGPT+Codex 共享 universal plugin directory）；官方技能仓库变更为 github.com/openai/skills | **需补充** |
| Trae 字段 | name + description 两字段 | 一致（官方文档原文抓取，含字段说明原文）；产品名 TraeCode；无其他字段、无长度限制 | **成立** |
| WorkBuddy 字段 | 市场包 295 包统计（name 294/description 288/version 278/description_zh_en 274/allowed-tools 96/display_name 87 等） | **重新下载最新市场包实测，统计完全一致**（295 包，字段频率同基线）；更新日志 2026-06 后无格式级变化 | **成立** |
| WorkBuddy 接入状态 | 未接入（官方未公开技能目录路径） | 无变化；官方文档仍只描述安装/启用/关闭 | **成立** |
| CodeBuddy CLI 字段 | 全部可选 + 占位符 | 一致（重新抓取确认） | **成立** |
| Agent Skills 开放规范 | 仅提及「标准六字段」名称 | 补全细节：name 必填 ≤64 且**须与目录名一致**、description ≤1024、allowed-tools 实验性、渐进披露 token 预算、`skills-ref` 校验工具 | 补充 |

**总体**：S1 基线结论在新一轮核查后绝大多数仍然成立；需更新的仅两点（Codex 长度限制表述、Codex 严格解析行为），需补充一点（Codex plugin 分发新动向）。四工具对 SKILL.md 的字段要求面在 2026 年中后**无根本性变化**。

---

## 7. 对 render 范围的建议（推荐结论）

### 7.1 结论：推荐「按工具注入特有字段」的最小注入集，不推荐「排除 Sidecar 原样复制」

事实依据：

1. **Codex / Trae 对 `name` + `description` 有硬性要求**（Codex 官方文档 "must include"；Trae 模板两字段；开放规范两者必填）。「原样复制」依赖 Sidecar 技能恰好带齐两字段；缺字段时 Codex 启动警告并跳过技能、Trae 无法正确索引——Vault 本体校验（决议 #2）只能兜底 Vault 侧，不能兜底 render 注入侧。
2. **Claude Code 是唯一「零注入即可用」的工具**（全字段可选），但不能因此否定其他工具的注入需求；同时它是唯一会**执行**行为字段的工具，注入必须区分「中性元数据」与「行为字段」。
3. **WorkBuddy 的扩展字段（description_zh/en、display_name、allowed-tools、version）是官方市场生态的事实规范**（295 包实证），原样复制无法提供这些展示/国际化字段。
4. 分发动作排除 Sidecar（`.skill-meta.json`）与元数据文件、仅复制 SKILL.md + 资源——**此部分保持成立**；但「注入」范围应含 Codex 目标的 `agents/openai.yaml`（合法伴生文件，不是 Sidecar）。

### 7.2 各工具字段注入清单

| 目标工具 | 注入字段 | 来源 | 格式 / 约束 |
| --- | --- | --- | --- |
| **通用（三工具共同底线）** | `name` | Vault Skill 目录名（slug）；Sidecar 如有显示名差异以 slug 为准 | 小写字母/数字/连字符，≤64 字符，无连续连字符，**与分发目录名一致**（开放规范 + Codex） |
| | `description` | Sidecar / 技能正文首段 | what + when 触发面，≤1024 字符（开放规范）；Claude Code 显示截断 1536（description+when_to_use） |
| | `version`（可选） | Sidecar | 语义化版本，标量转字符串规避 YAML 1.1 陷阱（沿用 S1 规则） |
| **Claude Code** | 无必须注入项 | — | 全字段可选；**默认不注入** `when_to_use`/`user-invocable`/`context`/`model`/`hooks`/`paths` 等行为字段，除非 Sidecar 显式声明且目标仅限 Claude Code |
| **Codex** | 无 SKILL.md 额外字段 | — | frontmatter 保持最小集（name+description+可选中性字段）；Codex 特有 UI/策略元数据生成 `agents/openai.yaml`（interface.display_name 等、policy.allow_implicit_invocation、dependencies.tools） |
| **Trae** | 无额外字段 | — | name + description 即满足官方全部要求 |
| **WorkBuddy**（预留，未接入） | `description_zh` / `description_en` | Sidecar 多语言描述（若 Vault 有此数据） | 按系统语言切换；无数据则不注入键 |
| | `display_name`（可选 `display_name_en`） | Sidecar 显示名 | 市场 UI 显示，可带引号 |
| | `allowed-tools` | Sidecar 工具白名单配置 | 空格或逗号分隔均可（官方市场两格式并存） |
| | `version` | Sidecar | 市场版本管理 |
| **所有工具** | 分发排除 | — | 排除 `.skill-meta.json` 等 Sidecar 与索引/元数据文件；仅复制 SKILL.md + 资源 + 上述注入物 |

### 7.3 实现建议（供 S2 规格参考）

- `ToolAdapter::render_skill` 的注入逻辑建议按「**通用最小集（保证 name+description）→ 工具特有补丁（可选）**」两级实现：先合成合法 frontmatter，再按目标工具叠加特有字段（Codex 无叠加、WorkBuddy 叠加扩展字段、Claude Code 仅当 Sidecar 显式声明行为字段时叠加）。
- `name` 一致性：render 输出前校验 `name == 分发目录名`，不一致时以目录名为准（开放规范硬约束），并在 Sidecar 记录差异供 UI 提示。
- WorkBuddy 目标在未接入期间不参与实际渲染，但其 trait 字段清单（5.4）应在适配器接口与类型定义中**预留**，避免接入时改接口。
- 若未来启用 claude.ai 上传 / Skills API 分发，render 需另走「标准六字段」模式（扩展字段剥离），与本地目录分发不同——当前 S2 不涉及，仅记录。
- 来源字段与注入物由 Sidecar 提供时，遵循「注入即真实数据」：不注入空值/默认占位文本。

---

## 8. 来源列表（均为 2026-08-20 抓取的一手来源）

### Claude Code
1. https://code.claude.com/docs/en/skills — Skills 官方文档（frontmatter 全字段表、必填性、分发限制与报错原文、命令名规则、截断规则）
2. https://github.com/anthropics/skills — 官方技能仓库（README「仅要求 name+description」、template/SKILL.md、spec 重定向 agentskills.io）

### Codex（OpenAI）
3. https://learn.chatgpt.com/docs/build-skills — Build skills 官方文档（must include name+description、目录结构、agents/openai.yaml、作用域表、plugin 分发）
4. https://developers.openai.com/codex/skills — 同文重定向入口（openai/codex 仓库 docs/skills.md 亦指向此处）
5. https://github.com/openai/skills — 官方技能仓库（.curated 技能实际 frontmatter 仅 name+description、agents/openai.yaml 样例）
6. https://raw.githubusercontent.com/openai/codex/2c6995ca4dfc23b93db311b59c1b4ead464658b1/docs/skills.md — 旧版官方文档（name ≤100 / description ≤500 历史限制，用于差异比对）

### Trae
7. https://docs.trae.ai/ide/skills — Trae 国际版技能文档（frontmatter 仅 name+description、目录结构、按需加载、内置技能）

### WorkBuddy（腾讯）
8. https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Skills-Market — 官方技能文档（安装/启用/关闭，无字段规范）
9. https://www.codebuddy.cn/docs/workbuddy/Changelog — 官方更新日志（2026-08-20 抓取，最新 5.3.13；技能格式无变化）
10. https://download.codebuddy.cn/skill-marketplace/skill-marketplace.zip — 官方市场 CDN 包（2026-08-20 下载，295 包 frontmatter 字段实测统计）
11. https://www.codebuddy.ai/docs/cli/skills — CodeBuddy CLI 技能文档（同源字段语义、占位符）

### 公共规范
12. https://agentskills.io/specification — Agent Skills 开放规范（name 64/description 1024/目录名一致约束、六字段表、渐进披露）
