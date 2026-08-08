# 四目标工具的用户级技能与配置机制调研

> 对应 issue：[mq233/skills-keeper #2 调研四目标工具的技能与配置机制](https://github.com/mq233/skills-keeper/issues/2)
> 范围：Claude Code、Codex、Trae 三个 Agent 工具（workbuddy 由另一子代理单独调研，不在本文覆盖）。
> 调研日期：2026-08-08。所有结论均来自官方文档与官方仓库源码等一手来源，每条结论标注来源；无法证实的内容明确标注"未证实"。

---

## 一、Claude Code（Anthropic）

### 1. 用户级技能目录路径

| 平台 | 路径 |
| --- | --- |
| macOS / Linux | `~/.claude/skills/<skill-name>/SKILL.md` |
| Windows | `%USERPROFILE%\.claude\skills\<skill-name>\SKILL.md` |

- 官方文档原文："Personal skills are available across all your projects"，路径为 `~/.claude/skills/<skill-name>/SKILL.md`；"On Windows, `~/.claude` resolves to `%USERPROFILE%\.claude`"（来源：[skills 文档](https://code.claude.com/docs/en/skills)、[.claude 目录文档](https://code.claude.com/docs/en/claude-directory)）。
- 若设置环境变量 `CLAUDE_CONFIG_DIR`，则所有 `~/.claude` 路径（含 skills）整体重定向到该目录下（来源：[.claude 目录文档](https://code.claude.com/docs/en/claude-directory)）。
- 其他技能层级（供对照）：企业托管级（managed settings 分发）、项目级 `.claude/skills/`、插件级 `<plugin>/skills/`。同名技能优先级：企业级 > 个人级 > 项目级，任何一级都会覆盖同名 bundled 内置技能（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。
- 附加能力：个人/项目技能目录项可以是**符号链接**，Claude Code 会跟随链接读取目标目录的 `SKILL.md`（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。

### 2. SKILL.md 支持度与格式要求

完全支持 SKILL.md 形态技能，并遵循 [Agent Skills](https://agentskills.io) 开放标准（跨工具通用），同时提供若干 Claude Code 扩展字段（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。

- **目录约定**：每个技能是一个目录，`SKILL.md` 为入口（必须），可附带任意支持文件（模板、示例、脚本、参考文档）：
  ```
  my-skill/
  ├── SKILL.md           # 主指令（必须）
  ├── template.md        # 可选的模板
  ├── examples/
  │   └── sample.md      # 示例输出
  └── scripts/
      └── validate.sh    # 可执行脚本
  ```
  官方建议 SKILL.md 控制在 500 行以内，详细材料放到独立文件（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。
- **frontmatter 字段**（YAML，位于 `---` 之间；全部字段可选，仅 `description` 被推荐）：
  - 标准字段（Agent Skills 规范，跨工具可用）：`name`、`description`、`license`、`compatibility`、`metadata`、`allowed-tools`
  - Claude Code 扩展字段：`when_to_use`、`argument-hint`、`arguments`、`disable-model-invocation`、`user-invocable`、`disallowed-tools`、`model`、`effort`、`context`（`fork` 时在子代理中运行）、`agent`、`background`、`hooks`、`paths`（按文件路径门控自动加载）、`shell`
  - `description`（与 `when_to_use` 合并）在技能列表中合计截断于 1536 字符
  - 注意：只有标准六字段可用于 claude.ai 上传、Skills API 与 `package_skill.py` 打包，出现非标准字段会报硬错误（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。
- **命令名规则**：个人/项目技能的命令名来自**目录名**（`.claude/skills/deploy-staging/SKILL.md` → `/deploy-staging`），frontmatter `name` 只影响列表显示名（插件技能除外）（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。
- **内容能力**：支持 `$ARGUMENTS`/`$0` 等字符串替换、`${CLAUDE_SKILL_DIR}` 路径变量、动态上下文注入（`` !`command` `` 行、` ```! ` 代码块）、`context: fork` 子代理执行（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。

### 3. 说明文件（CLAUDE.md）位置与加载语义

CLAUDE.md 是 Claude Code 的说明文件，按加载顺序（范围从大到小）：

| 范围 | 位置 | 说明 |
| --- | --- | --- |
| 托管策略级（组织） | macOS：`/Library/Application Support/ClaudeCode/CLAUDE.md`；Linux/WSL：`/etc/claude-code/CLAUDE.md`；Windows：`C:\Program Files\ClaudeCode\CLAUDE.md` | IT/DevOps 统一部署，个人无法排除 |
| **用户级（个人）** | `~/.claude/CLAUDE.md`（Windows 为 `%USERPROFILE%\.claude\CLAUDE.md`） | 所有项目生效的个人偏好 |
| 项目级 | `./CLAUDE.md` 或 `./.claude/CLAUDE.md` | 团队共享，随 git 提交 |
| 个人项目级 | `./CLAUDE.local.md` | 项目内私有偏好，需加入 .gitignore |

（来源：[memory 文档](https://code.claude.com/docs/en/memory)）

**加载语义（多文件、非覆盖式拼接）**：
- Claude Code 从当前工作目录**向上**逐级查找 `CLAUDE.md` 与 `CLAUDE.local.md`，找到的全部文件**拼接进上下文而非互相覆盖**；顺序为文件系统根 → 工作目录，即越靠近启动目录的文件越后读、冲突时优先
- 同一目录内 `CLAUDE.local.md` 追加在 `CLAUDE.md` 之后
- 工作目录**子目录**中的 CLAUDE.md 不随会话启动加载，而是 Claude 读取该子目录文件时按需加载
- 用户级与项目级文件同时加载；用户级 `~/.claude/rules/` 与项目级 `.claude/rules/*.md` 为按主题拆分、可带 `paths:` 前缀门控的规则文件（规则级：用户规则先读、项目规则优先）
- 支持 `@path/to/import` 导入其他文件（递归最多 4 层）
- 官方建议每份 CLAUDE.md 控制在 200 行以内
（来源：[memory 文档](https://code.claude.com/docs/en/memory)）

### 4. 技能启用机制

- **放入目录即生效，无需任何注册/配置**。官方文档示例即"创建 `~/.claude/skills/<name>/SKILL.md` 后重启会话即可使用"（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。
- 支持**热加载**：Claude Code 监听技能目录文件变化，会话内增删改技能即时生效（新建顶级 skills 目录需重启以启动监听）（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。
- 技能描述在会话启动时注入上下文供 Claude 匹配，**正文按需加载**（触发时才完整读入）（来源：[.claude 目录文档](https://code.claude.com/docs/en/claude-directory)）。
- 可选的控制手段（非启用前提）：
  - frontmatter `disable-model-invocation: true`（仅用户可调）、`user-invocable: false`（仅 Claude 可调）
  - settings 中 `skillOverrides`（`"on" / "name-only" / "user-invocable-only" / "off"`）控制技能对 Claude 与 `/` 菜单的可见性
  - 权限规则 `Skill(name)` / `Skill(name *)` 允许或拒绝 Claude 调用
  - 插件级技能需要先安装插件（`/plugin install`）
- 技能触发方式：`/skill-name` 手动调用，或 Claude 依据 `description` 自动调用（来源：[skills 文档](https://code.claude.com/docs/en/skills)）。

---

## 二、Codex（OpenAI）

> 重要演进提示：Codex 技能目录体系在 2025 年末引入（`~/.codex/skills`，需 experimental flag），2026 年中已迁移至跨工具标准的 **`.agents/skills`** 约定；旧目录仍被源码保留兼容。下文分别列出。

### 1. 用户级技能目录路径

**当前官方文档（learn.chatgpt.com，2026-08 抓取）规定的技能搜索位置**（来源：[Build skills](https://learn.chatgpt.com/docs/build-skills)）：

| 作用域 | 位置 | 说明 |
| --- | --- | --- |
| REPO | `$CWD/.agents/skills` | 当前工作目录 |
| REPO | `$CWD/../.agents/skills` | CWD 上一级（monorepo） |
| REPO | `$REPO_ROOT/.agents/skills` | 仓库根 |
| **USER** | **`$HOME/.agents/skills`** | **用户个人技能** |
| ADMIN | `/etc/codex/skills` | 机器/容器级 |
| SYSTEM | 随 Codex 内置 | Bundled with Codex |

- 官方手册原文："Personal skills are stored in `$HOME/.agents/skills`"；团队技能可检入仓库的 `.agents/skills`（来源：[Codex manual](https://learn.chatgpt.com/docs/codex-manual.md)）。
- **旧版（已废弃但兼容）用户级位置**：源码注释明确 "Deprecated user skills location (`$CODEX_HOME/skills`), kept for backward compatibility"，即默认 `~/.codex/skills` 仍被读取（来源：openai/codex 源码 [host_roots.rs](https://github.com/openai/codex/blob/main/codex-rs/ext/skills/src/host_roots.rs)）。历史版本（v0.39 时代）的官方文档即使用 `~/.codex/skills/**/SKILL.md`（递归扫描）（来源：旧版文档 [docs/skills.md @ 2c6995ca](https://raw.githubusercontent.com/openai/codex/2c6995ca4dfc23b93db311b59c1b4ead464658b1/docs/skills.md)）。
- **Windows 路径**：官方文档未单独列出 Windows 路径（只给 Unix 形式 `$HOME`、`/etc/codex`）；实现按 home 目录语义解析（Rust `dirs::home_dir()`，Windows 下为 `%USERPROFILE%`），因此 Windows 对应为 `%USERPROFILE%\.agents\skills` 与 `%USERPROFILE%\.codex\skills`（来源：源码 [host_roots.rs](https://github.com/openai/codex/blob/main/codex-rs/ext/skills/src/host_roots.rs)；Windows 具体路径官方文档未证实，按 home 语义推导）。
- 同名技能不合并，都会出现在技能选择器中；支持符号链接技能目录（来源：[Build skills](https://learn.chatgpt.com/docs/build-skills)）。

### 2. SKILL.md 支持度与格式要求

完全支持 SKILL.md 形态技能（来源：[Build skills](https://learn.chatgpt.com/docs/build-skills)）：

- **frontmatter 必需字段**：`name` 与 `description` 均为必填；旧版文档限定 `name` ≤100 字符、单行，`description` ≤500 字符、单行（超出/缺失会在启动时弹出阻塞性错误弹窗），其余键忽略（来源：[Build skills](https://learn.chatgpt.com/docs/build-skills)、旧版 [docs/skills.md](https://raw.githubusercontent.com/openai/codex/2c6995ca4dfc23b93db311b59c1b4ead464658b1/docs/skills.md)）。
- **目录结构**（官方文档示例）：
  ```
  my-skill/
  ├── SKILL.md          # 必需：指令 + 元数据
  ├── scripts/          # 可选：可执行代码
  ├── references/       # 可选：文档
  ├── assets/           # 可选：模板、素材
  └── agents/
      └── openai.yaml   # 可选：外观与依赖声明
  ```
  `agents/openai.yaml` 可配置 `interface.display_name`/图标、`policy.allow_implicit_invocation`（默认 true）、`dependencies.tools`（如声明 MCP server 依赖）（来源：[Build skills](https://learn.chatgpt.com/docs/build-skills)）。
- **渐进披露**：启动时只注入每个技能的 name、description 与绝对路径（追加为运行时 `## Skills` 段），正文留在磁盘、用到时才打开（来源：[Build skills](https://learn.chatgpt.com/docs/build-skills)；旧版文档同此机制）。

### 3. 说明文件（AGENTS.md）位置与加载语义

（来源：[AGENTS.md 文档](https://learn.chatgpt.com/docs/agent-configuration/agents-md)）

- **用户级**：`$CODEX_HOME/AGENTS.md`（`CODEX_HOME` 默认指向 `~/.codex`；Windows 下对应 `%USERPROFILE%\.codex`）。该层若存在 `AGENTS.override.md` 则优先读取，且只取第一个非空文件；官方用途是"临时全局覆盖，不想删基础文件时使用"。
- **项目级**：从项目根（通常是 git 根）向当前工作目录**逐级**发现；每层目录**最多取一个文件**（依次检查 `AGENTS.override.md` → `AGENTS.md` → fallback 文件名）；所有文件从根到 CWD 顺序**拼接**（以空行连接），越靠近 CWD 的文件越靠后、冲突时优先（"Files closer to your current directory override earlier guidance"）。无项目根时只检查当前目录。
- **大小限制**：所有加载文件合计上限 `project_doc_max_bytes`，默认 32 KiB，超限的后续文件被跳过。
- **fallback 文件名**：`project_doc_fallback_filenames`（config.toml 中配置）可指定其他文件名（如 `CLAUDE.md`、`TEAM_GUIDE.md`），仅当该目录无 AGENTS.md 时按序尝试。
- **补充**：Codex 另有自动记忆层（memories），为 Codex 自己跨会话总结写入的内容，与 AGENTS.md（静态、用户维护）互补；官方文档存在专门页面（来源：[Codex manual](https://learn.chatgpt.com/docs/codex-manual.md)，其具体存储路径未在本轮抓取的一手页面中确认，未证实）。

### 4. 技能启用机制

- **当前版本（2026-08 官方文档）：技能默认可用，放入目录即被发现，无需注册或 feature flag**。官方文档的启用/禁用入口是 config.toml 中的 `[[skills.config]]`（按 path 引用某个技能的 SKILL.md 并 `enabled = false` 禁用它），修改后需重启 Codex；技能文件本身的改动可被自动检测（来源：[Build skills](https://learn.chatgpt.com/docs/build-skills)、[config reference](https://learn.chatgpt.com/docs/config-file/config-reference.md)）。
- **历史版本（2025 年末，CLI v0.39 起）：技能是 experimental 功能，默认关闭**，需在 `~/.codex/config.toml` 加 `[features] skills = true` 或单次运行 `codex --enable skills`；技能在启动时加载一次，新建技能后需重启（来源：旧版 [docs/skills.md](https://raw.githubusercontent.com/openai/codex/2c6995ca4dfc23b93db311b59c1b4ead464658b1/docs/skills.md)）。
- 触发方式：显式在消息中用 `$<skill-name>` 提及、TUI 中 `/skills` 浏览插入；或当任务与 `description` 匹配时由模型**隐式自动激活**（可通过 `agents/openai.yaml` 的 `policy.allow_implicit_invocation: false` 关闭隐式触发）（来源：[Build skills](https://learn.chatgpt.com/docs/build-skills)）。

---

## 三、Trae / TraeCode（字节跳动）

> Trae 国际版与 Trae CN（中国版）文档站点不同（docs.trae.ai / docs.trae.cn），用户级目录名也不同（`.trae` / `.trae-cn`），以下分开标注。官方文档近期将产品名称为 TraeCode。

### 1. 用户级技能目录路径

官方文档原文（来源：[Trae 技能文档（国际版，en）](https://docs.trae.ai/ide/skills) / [Trae CN 技能文档](https://docs.trae.cn/ide_skills)）：

| 范围 | 国际版 Trae | Trae CN |
| --- | --- | --- |
| **全局（用户级）技能** | macOS/Linux：`~/.trae/skills`；Windows：`%userprofile%/.trae/skills` | macOS/Linux：`~/.trae-cn/skills`；Windows：`%userprofile%/.trae-cn/skills` |
| 项目技能 | `<project>/.trae/skills/`（手动创建或导入时自动在 `.trae/skills/{skill_name}/` 建目录） | 同左 |

- 官方原文："Global skill: macOS/Linux: The local root directory `~/.trae/skills`. Windows: The local root directory `%userprofile%/.trae/skills`."（CN 版同句式，目录为 `~/.trae-cn/skills`）。
- 另支持 **`.agents/skills/` 约定目录**（Agent Skills 规范定义）：把该目录加入项目即可被自动发现加载，但需要先在"设置 > 技能与命令 > 导入设置"中打开 **"启用 .agents 技能目录"** 开关；与 `.trae/skills/` 中技能重名时，`.trae/skills/` 优先（来源：Trae 技能文档，同上）。
- 内置技能（随产品发布）：`TRAE-generate-mini-app`、`TRAE-debugger`、`TRAE-code-review`（来源：同上）。

### 2. SKILL.md 支持度与格式要求

完全支持 SKILL.md 形态技能（来源：Trae 技能文档，同上）：

- **frontmatter**：`name`（技能名称）与 `description`（功能与使用场景简述）两个字段。
- **正文结构**（官方模板）：
  ```
  ---
  name: skill name
  description: briefly describe what the skill does and when to use it
  ---
  # SKill name
  ## Description    描述技能作用
  ## When to use    描述触发条件/使用时机
  ## Instructions   清晰的分步指令
  ## Examples (optional)  输入/输出示例
  ```
- **目录结构**（官方示例）：`SKILL.md` 必须，其余可选：
  ```
  skill-name/
  ├── SKILL.md               #（必须）核心指令
  ├── examples/              #（可选）输入/输出示例
  ├── templates/             #（可选）可复用模板
  └── resources/             #（可选）参考文件、运行脚本、素材
  ```
- **加载模型**：技能**动态按需加载**——会话启动只扫描全部技能的简要描述，判定任务与某技能高度相关时才加载其完整内容（与规则的全量加载相对）（来源：同上）。
- 创建方式：AI 对话自动创建、手动创建（设置 > 技能与命令）、导入外部 SKILL.md 或 .zip（来源：同上）。

### 3. 说明文件（Rules 规则）位置与加载语义

Trae 的说明文件机制名为"规则（Rules）"，分为全局规则与项目规则（来源：[Trae 规则文档](https://docs.trae.cn/ide_rules)）：

- **项目规则**：`<project>/.trae/rules/*.md`（设置中创建规则时系统自动生成该目录）。系统**递归读取，最多 3 层嵌套**；项目中任意子目录下的 `.trae/rules/` 也会被读取，仅当涉及该目录文件时生效。`git-commit-message.md` 为自动生成的提交信息规则文件。
- **用户级（全局）规则**：官方 IDE 文档只描述通过"设置中心 > 规则"UI 创建（选择"全局"），**未明文给出磁盘路径**；TRAE CLI 文档明确 CLI 可读取 `{$HOME}/.trae-cn/rules`（CN 版）目录下的规则（来源：[Trae CLI 记忆文档](https://docs.trae.cn/cli_memories)），据此国际版应为 `~/.trae/rules`（未证实：国际版 IDE 文档未明文，此为按 CLI 文档与技能目录同构推断）。
- **frontmatter 字段**：`alwaysApply`（始终生效）、`globs`（按文件匹配生效，如 `*.js`、`src/**/*.ts`）、`description`（智能生效，由 AI 判断相关性）、`scene: git_message`（提交信息场景规则）；规则之间要求不得互相冲突或覆盖，多条规则叠加生效；`#Rule名` 手动触发优先级最高。
- **兼容导入**：设置 > 规则 > 导入设置中可开关"将 AGENTS.md 包含在上下文中""将 CLAUDE.md 包含在上下文中"；项目从 Claude Code 迁移时兼容 `CLAUDE.md` 与 `CLAUDE.local.md`（来源：Trae 规则文档，同上）。
- **规则 vs 技能**：规则全量注入、常驻上下文；技能按需加载（来源：Trae 技能文档）。

### 4. 技能启用机制

- **创建后默认启用**，可通过技能面板中的**开关**手动启用/禁用（官方原文："After creating a skill, you can enable or disable it by toggling its switch."）（来源：Trae 技能文档）。
- 禁用行为：禁用技能后，TraeCode 会在项目 `.trae/` 目录创建 **`skill-config.json`**，其中**仅罗列被禁用的项目技能**；被禁用的全局技能不写入该文件（来源：Trae 技能文档）。
- `.agents/skills/` 目录需要显式开启"启用 .agents 技能目录"开关后才生效（来源：Trae 技能文档）。
- 触发方式：对话中手动指示 AI 使用某技能，或 AI 依据技能描述中的使用场景自动调用（来源：Trae 技能文档）。

---

## 附：三工具机制对比速览

| 维度 | Claude Code | Codex | Trae |
| --- | --- | --- | --- |
| 用户级技能目录（macOS/Linux） | `~/.claude/skills/` | `~/.agents/skills/`（新）；`~/.codex/skills/`（废弃兼容） | `~/.trae/skills/`（CN 版 `~/.trae-cn/skills/`） |
| 用户级技能目录（Windows） | `%USERPROFILE%\.claude\skills\` | `%USERPROFILE%\.agents\skills\`（按 home 语义推断，官方文档未列 Windows 路径） | `%userprofile%/.trae/skills/`（CN 版 `.trae-cn`） |
| 项目级技能目录 | `.claude/skills/`（含嵌套按需加载） | `.agents/skills/`（各层目录）+ `.codex/skills/` | `.trae/skills/` |
| SKILL.md 必填 frontmatter | 无必填（推荐 description） | name + description（必填，有长度限制） | name + description |
| 说明文件（用户级） | `~/.claude/CLAUDE.md` | `~/.codex/AGENTS.md`（override 文件优先） | 全局规则（UI 创建；CLI 读 `~/.trae-cn/rules`） |
| 说明文件加载语义 | 向上逐级发现、全部拼接、靠 CWD 者优先 | 从 repo root 到 CWD 逐级拼接、靠 CWD 者优先、32KiB 上限 | 项目 `.trae/rules/` 递归最多 3 层、支持 globs/alwaysApply |
| 技能启用 | 放入目录即生效（热加载） | 当前默认可用；旧版需 `[features] skills=true`；可用 `[[skills.config]]` 禁用 | 默认启用，面板开关控制；禁用记录在 `.trae/skill-config.json` |
| 说明文件兼容性 | 可 `@AGENTS.md` 导入 | `project_doc_fallback_filenames` 可读 CLAUDE.md | 可开关导入 AGENTS.md / CLAUDE.md |

---

## 来源列表

### Claude Code（官方文档）
1. https://code.claude.com/docs/en/skills — Extend Claude with skills（技能位置、frontmatter、目录结构、启用机制）
2. https://code.claude.com/docs/en/claude-directory — Explore the .claude directory（~/.claude 目录全览、Windows 路径、CLAUDE_CONFIG_DIR）
3. https://code.claude.com/docs/en/memory — Manage Claude's memory（CLAUDE.md 各级位置、拼接/覆盖语义、rules）

### Codex（OpenAI 官方文档 + 官方仓库源码）
4. https://learn.chatgpt.com/docs/build-skills — Build skills（`.agents/skills` 目录表、SKILL.md 字段、目录结构、[[skills.config]]、触发方式）
5. https://learn.chatgpt.com/docs/agent-configuration/agents-md — Custom instructions with AGENTS.md（用户级/项目级层级、override、大小限制、fallback 文件名）
6. https://learn.chatgpt.com/docs/config-file/config-reference.md — Configuration reference（config.toml 位置、project_doc_max_bytes、skills.config）
7. https://learn.chatgpt.com/docs/codex-manual.md — Codex manual（$HOME/.agents/skills 个人技能、~/.codex AGENTS.md、~/.codex/config.toml）
8. https://github.com/openai/codex/blob/main/codex-rs/ext/skills/src/host_roots.rs — Codex 源码（技能根目录解析：`.agents/skills`、废弃兼容 `$CODEX_HOME/skills`、各作用域）
9. https://raw.githubusercontent.com/openai/codex/2c6995ca4dfc23b93db311b59c1b4ead464658b1/docs/skills.md — 旧版官方文档（`~/.codex/skills`、`[features] skills=true` / `--enable skills`、name/description 长度限制）
10. https://developers.openai.com/plugins/concepts/skills — OpenAI Plugins：Skills 概念页（SKILL.md 组成、触发模型，无路径信息）

### Trae（官方文档）
11. https://docs.trae.ai/ide/skills — Trae 国际版技能文档（全局技能 `~/.trae/skills` / `%userprofile%/.trae/skills`、项目 `.trae/skills/`、SKILL.md 格式、开关启用机制、`.agents/skills/` 开关）
12. https://docs.trae.cn/ide_skills — Trae CN 技能文档（`~/.trae-cn/skills` / `%userprofile%/.trae-cn/skills`）
13. https://docs.trae.cn/ide_rules — Trae 规则文档（项目 `.trae/rules/` 3 层嵌套、frontmatter 字段、AGENTS.md/CLAUDE.md 导入开关）
14. https://docs.trae.cn/cli_memories — Trae CLI 记忆/规则文档（CLI 读取 `{$HOME}/.trae-cn/rules`）
