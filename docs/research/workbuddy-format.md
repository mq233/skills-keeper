# 调研：WorkBuddy 的技能格式

> 调研日期：2026-08-08
> 调研方式：仅以一手来源（腾讯官方文档、官方 CDN 技能市场包、官方新闻稿、官方更新日志）为结论依据；二手来源仅作交叉印证，并在文中标注。
> 对应 Issue：wayfinder research ticket #3

---

## 1. WorkBuddy 身份确认

| 项目 | 结论 |
|------|------|
| 厂商 | **腾讯（腾讯云）**，由腾讯云代码助手 CodeBuddy 团队研发，与 CodeBuddy 同属腾讯"Buddy"家族 |
| 产品类型 | **独立桌面 Agent 工具**（全场景职场 AI 智能体桌面工作台），非 IDE、非 IDE 插件；有独立客户端（Windows / macOS），另有 Web 端与移动端 |
| 国内官网 | https://www.workbuddy.cn/ |
| 国际版官网 | https://www.workbuddy.ai/（2026-05-28 发布海外版） |
| 官方文档 | https://www.codebuddy.cn/docs/workbuddy/ （文档挂载于 CodeBuddy 官网域名下） |
| 与 OpenClaw 的关系 | 官方更新日志 v4.5.12（2026-03-14）与 v4.6.1（2026-03-20）明确记载「**兼容 Vercel/OpenClaw 生态安装 Skill**」；媒体称其为"腾讯版小龙虾"，完全兼容 OpenClaw skills |

发布历程（官方更新日志 + 腾讯官方新闻稿）：
- 2026-02-06 宣布启动内测（媒体口径，来源见文末交叉印证）
- 2026-03-04 官方更新日志记 v4.5.0「WorkBuddy 正式发布」
- 2026-05-28 发布海外国际版（workbuddy.ai）
- 2026-06-05 腾讯云 AI 产业应用大会发布 WorkBuddy 企业版（腾讯官方新闻稿）

一手来源：
- 官方更新日志：https://www.codebuddy.cn/docs/workbuddy/Changelog
- 腾讯官方新闻稿：https://www.tencent.com/zh-cn/tencent-cloud-debuts-productivity-agent-suite-creating-a-new-gateway-to-ai-for-users-and-enterprises/

---

## 2. 技能格式支持度与规范格式样例

### 2.1 支持度：完整支持 SKILL.md 形态

WorkBuddy **完整支持 SKILL.md 形态的技能目录**，与 Claude Code / Codex / OpenClaw 的 Agent Skills 标准同源（官方更新日志明确「兼容 Vercel/OpenClaw 生态安装 Skill」；官方文档的 Skills 定义与 OpenClaw/Anthropic Agent Skills 一致）。

一手验证：腾讯官方 CDN 技能市场包 `https://download.codebuddy.cn/skill-marketplace/skill-marketplace.zip`（已验证可访问，2026-06-25 更新）内含 **295 个技能包，全部为 `skills/<skill-name>/SKILL.md` 形态**；市场清单 `.codebuddy-skill/marketplace.json` 的 `owner` 为 `{name: "CodeBuddy", email: "codebuddy@tencent.com"}`，确认官方身份。

### 2.2 官方市场技能包目录约定

```
skills/<skill-name>/
├── SKILL.md          # 必需：YAML frontmatter + Markdown 指令正文
├── README.md         # 可选：人类可读说明
├── scripts/          # 可选：可执行代码（Python/JS/Shell）
├── references/       # 可选：按需加载的参考文档
└── assets/           # 可选：输出资源（模板、品牌素材）
```

实测官方市场 295 个包中：`references/` 129 个、`scripts/` 123 个、`assets/` 19 个；另有部分包含 `.clawhub/`（ClawHub 安装来源记录，`origin.json` 记 registry/slug/installedVersion）。

### 2.3 SKILL.md frontmatter 字段

**实际官方市场技能包**（统计 294 个 SKILL.md 的 frontmatter 出现频率）：

| 字段 | 出现数 | 说明 |
|------|--------|------|
| `name` | 294（全部） | 技能名，目录名须与之对应 |
| `description` | 288 | 触发描述（做什么 + 何时用），必填核心 |
| `version` | 278 | 版本号（市场管理/更新依赖） |
| `description_zh` / `description_en` | 274 | WorkBuddy 市场扩展字段，按系统语言切换描述（官方更新日志 v4.6.1：「Skill 描述国际化」） |
| `allowed-tools` | 96 | 工具白名单，如 `Bash, Read` |
| `display_name` / `display_name_en` | 87 / 83 | 显示名（市场 UI） |
| `homepage` | 85 | 项目主页 |
| `metadata` | 80 | 元数据，常含 OpenClaw 兼容配置 |
| `visibility` | 71 | 市场可见性 |
| `author` / `license` / `category` / `icon` 等 | 少量 | 其他可选字段 |

**实测样例（官方市场包 skills/12306/SKILL.md）**：

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

# 12306 Train Query
...（Markdown 指令正文）
```

另一个样例（skills/obsidian/SKILL.md）展示了 `homepage` 与 `metadata.clawdbot`（install 指令数组，声明 brew 安装依赖）：

```yaml
name: obsidian
description: "Work with Obsidian vaults (plain Markdown notes) and automate via obsidian-cli."
version: 1.0.0
homepage: https://help.obsidian.md
metadata: {"clawdbot":{"emoji":"💎","requires":{"bins":["obsidian-cli"]},"install":[{"id":"brew","kind":"brew","formula":"yakitrak/yakitrak/obsidian-cli","bins":["obsidian-cli"],"label":"Install obsidian-cli (brew)"}]}}
```

**CodeBuddy CLI 官方文档（同团队产品，WorkBuddy 同源）补充字段**：`disable-model-invocation`、`user-invocable`、`context: fork`（独立 subagent 上下文）、`agent`、`model`、`hooks`；正文支持占位符 `${CODEBUDDY_SKILL_DIR}`、`${CODEBUDDY_SESSION_ID}`、环境变量 `${MY_ENV_VAR}`（含默认值 `:-`）、`$ARGUMENTS`，并兼容 `CLAUDE_SKILL_DIR` 等 Claude 别名；支持 `` !`command` `` 内联执行 Shell。来源：https://www.codebuddy.ai/docs/cli/skills

### 2.4 市场清单格式（官方）

`.codebuddy-skill/marketplace.json`（官方市场安装包级清单）：顶层字段 `name` / `description` / `owner` / `tags_zh` / `tags_en` / `skills[]`；`skills` 数组每项含 `name`、`version`、`description`、`description_zh`、`description_en`、`source`、`examples_zh`、`examples_en` 等。

---

## 3. 技能目录路径（按平台）

### 3.1 官方确认的路径（WorkBuddy 官方文档 FAQ）

| 路径 | Windows | macOS |
|------|---------|-------|
| 工作空间（workspace）目录 | `C:\Users\<用户名>\workbuddy` | `/Users/<用户名>/WorkBuddy` |
| 应用安装目录 | `C:\Users\{用户名}\AppData\Local\Programs\WorkBuddy` | `/Applications/WorkBuddy.app` |
| 登录/权限数据目录 | `%APPDATA%\CodeBuddyExtension` | `~/Library/Application Support/CodeBuddyExtension/` |

来源：https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/FAQ（原文含上述路径）

### 3.2 技能子目录位置（官方未公开，社区归纳，标注为"未证实"）

官方文档仅确认技能经「技能市场（SkillHub）/上传安装包/对话创建」安装，**未公开技能文件在磁盘上的确切目录**。以下为社区来源（基于官方市场包结构归纳 + 社区实测）的一致口径：

| 级别 | 路径（Unix 形式） | Windows 展开 | macOS 展开 |
|------|-------------------|--------------|------------|
| 用户级（全局） | `~/.workbuddy/skills/<skill-name>/` | `C:\Users\<用户名>\.workbuddy\skills\<name>\` | `/Users/<用户名>/.workbuddy/skills/<name>/` |
| 项目/工作区级 | `{workspace}/.workbuddy/skills/` 及 `{workspace}/Claw/skills/<skill-id>/` | `C:\Users\<用户名>\workbuddy\Claw\skills\`（工作区下） | `/Users/<用户名>/WorkBuddy/Claw/skills/` |

说明：
- 官方更新日志 v4.22.4 记载修复「套件 skill **嵌套目录结构无法扫描**的问题」，可反推技能按目录扫描、要求技能目录直接位于扫描目录下（`skills/<name>/SKILL.md`，不支持更深嵌套）。
- 社区称 WorkBuddy 兼容读取 `~/.workbuddy/skills/` 与 `~/.codebuddy/skills/`（旧目录）。
- 加载优先级（社区口径）：项目级 > 用户级 > 内置。

> ⚠️ 3.2 节整体属于"未证实"范畴，详见第 6 节。

---

## 4. 启用机制与配置格式

### 4.1 官方确认的安装/启用流程

官方文档「技能」页（Skills-Market）与更新日志确认：

- **安装方式**（官方文档原文）：技能市场浏览/搜索 → 一键安装；「添加技能」支持三种方式——**上传技能**（导入本地技能包，拖拽或选文件，"导入后系统自动完成配置，无需额外操作"）、**查找技能**（输入任务描述自动查找）、**创建技能**（输入任务描述自动生成）。
- **安全检测**（官方更新日志 v4.7.5 起）：安装前自动安全扫描，检测恶意脚本；设置页有「非高风险自动安装」开关，高风险技能始终需手动确认（官方「系统设置」文档）。
- **启用/关闭**（官方文档原文）：已安装技能可随时关闭或重新启用，无需卸载；「关闭后该技能不会被调用」。
- **其他**：v4.7.1 起支持 `/` 命令唤起技能列表；v4.7.5 技能版本管理（有新版本提示批量更新）；v4.22.4 支持 CBC deeplink 安装；卸载支持批量。

### 4.2 目录即识别 or 需注册？

- **官方口径**：上传导入"自动完成配置，无需额外操作"；更新日志反推技能按目录扫描识别（嵌套扫描修复项）。官方未出现"注册/manifest 注册"流程的描述。
- **社区口径**（未证实）：放入目录后需重启应用或执行 `/reload skills` 才生效；个别资料称技能须处于"已启用且已发布"状态并依赖 SQLite 全文索引（`fts_index.db`）识别——此说法未获官方证实，且在官方市场包中未发现任何 manifest.json / 索引文件。
- 官方市场包中实际存在的是 `.clawhub/origin.json`（ClawHub 安装记录：registry/slug/installedVersion），仅作来源记录，非注册凭据。

### 4.3 配置文件格式

| 文件 | 层级 | 内容 |
|------|------|------|
| `.codebuddy-skill/marketplace.json` | 官方市场安装包根级 | 市场清单（见 2.4），官方 owner 为 codebuddy@tencent.com |
| `.clawhub/origin.json` | 单个技能包内 | ClawHub 安装来源记录（registry: https://clawhub.ai, slug, installedVersion） |
| `mcp.json` | 连接器（connectors）包内 | MCP 连接器配置（与技能并列的独立机制） |
| `.codebuddy-plugin/plugin.json` | 专家/插件包内 | 插件清单（WorkBuddy 的"专家"形态） |

---

## 5. 说明文件对应物（相当于 CLAUDE.md / AGENTS.md）

**官方文档未披露**任何类似 CLAUDE.md / AGENTS.md 的说明文件机制；官方「记忆」文档仅描述应用内对话式记忆（「设置-记忆」，云端提取会话事实，无文件形态）。

**社区来源（腾讯云开发者社区、人人都是产品经理等）一致口径**——WorkBuddy 工作区/用户目录存在 OpenClaw 同款"人格层"文件体系：

| 文件 | 位置 | 职责 |
|------|------|------|
| `SOUL.md` | `~/.workbuddy/`（用户级，全局生效） | AI 人格与工作原则（对应 CLAUDE.md 的角色/原则部分） |
| `IDENTITY.md` | `~/.workbuddy/` | 名字、emoji、角色定位等身份信息 |
| `USER.md` | `~/.workbuddy/` | 用户信息 |
| `AGENTS.md` | 需自建，位置同上 | 多 Agent 协作规则、输出规范（对应 AGENTS.md） |
| `MEMORY.md` | `~/.workbuddy/MEMORY.md`（用户级）；`{workspace}/.workbuddy/memory/MEMORY.md`（项目级） | 长期记忆 |

社区称：SOUL.md / IDENTITY.md / USER.md 在首次启动时引导生成，AGENTS.md 需手动编写；人格层文件只在用户级路径生效，项目级会被忽略；每次新对话自动读取。此文件体系与 OpenClaw 生态（WorkBuddy 明确兼容）一致，但**文件名称、位置均未经腾讯官方文档证实**，列为未证实项。

---

## 6. 未证实项清单

以下各项的可靠一手来源（官方文档/官方代码/官方发布说明）未能找到，均已注明已尝试的检索途径，未编造：

1. **技能文件的确切磁盘目录（Windows/macOS）**：官方文档（文档目录全站检索：入门指南/功能说明/系统设置/数据管理/FAQ/实践案例/零成本 Skill 10 选/更新日志）未公开技能文件落盘路径；仅 FAQ 公开工作空间与安装目录。社区口径（`~/.workbuddy/skills/` 用户级、`{workspace}/.workbuddy/skills/` 与 `{workspace}/Claw/skills/` 工作区级、`%APPDATA%\WorkBuddy\skills\`）存在版本差异，建议以应用内实测为准。
2. **放入目录即生效 vs 需重载/重启**：官方仅确认"上传导入自动完成配置"与目录扫描（嵌套修复条目），未确认手动放置目录后的生效条件；社区称需 `/reload skills` 或重启。
3. **SOUL.md / IDENTITY.md / USER.md / AGENTS.md / MEMORY.md 文件体系**：官方文档零提及（官方记忆文档仅描述应用内对话式记忆）；文件名称与位置均来自社区文章（腾讯云开发者社区、人人都是产品经理等），虽与 OpenClaw 生态一致，仍属未证实。
4. **manifest.json + SQLite 索引（fts_index.db）**：个别二手资料称技能元数据写入 manifest.json（id/name/version）并有 SQLite 全文索引；在官方 CDN 市场包 295 个技能包中逐一检索未发现任何 manifest.json 或索引文件，判定该说法可信度低。
5. **WorkBuddy 自身的说明文件对应物**：CodeBuddy CLI 文档有 CODEBUDDY.md 记忆文件体系（`~/.codebuddy/CODEBUDDY.md`、`./CODEBUDDY.local.md`、rules 目录），但 WorkBuddy 桌面应用是否沿用同套文件未获官方证实。
6. **macOS 用户级技能目录**：所有社区路径说明以 Unix `~` 形式给出，未见到 macOS 实测的具体路径。

已尝试但未获官方信息的主要检索途径：WorkBuddy 官方文档站全目录（codebuddy.cn / workbuddy.ai 两域名）、官方 CDN 市场包（download.codebuddy.cn）、腾讯官方新闻稿、Tencent GitHub 组织仓库（仅 workbuddy-bench 评测基准，无产品源码/文档仓库）。

---

## 7. 来源列表

### 一手来源（结论依据）

- WorkBuddy 国内官网：https://www.workbuddy.cn/
- WorkBuddy 国际版官网：https://www.workbuddy.ai/
- 官方文档站（WorkBuddy 全部章节）：https://www.codebuddy.cn/docs/workbuddy/
- 官方更新日志（v4.5.0 起，含 OpenClaw 兼容/SkillHub/技能安全检测条目）：https://www.codebuddy.cn/docs/workbuddy/Changelog
- 官方文档-技能市场（安装/上传/启用/关闭）：https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Skills-Market
- 官方文档-系统设置（非高风险自动安装）：https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Setting
- 官方文档-记忆（应用内记忆，无文件形态）：https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Memory
- 官方文档-项目（云上项目级技能）：https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Function-Description/Project
- 官方文档-实践八：创建自己的 Skills：https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Practice-Cases/Practice-Eight
- 官方文档-FAQ（工作空间/安装目录路径）：https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/FAQ
- 官方文档-零成本 Skill 系列（Find Skills / Skill Scanner 等）：https://www.codebuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/WorkBuddy-Zero-Cost-Skill-Top-10/
- 官方技能市场 CDN 包（295 个技能包，本调研已下载验证）：https://download.codebuddy.cn/skill-marketplace/skill-marketplace.zip
- 腾讯官方新闻稿（Buddy 家族 / 企业版发布）：https://www.tencent.com/zh-cn/tencent-cloud-debuts-productivity-agent-suite-creating-a-new-gateway-to-ai-for-users-and-enterprises/
- CodeBuddy CLI 技能系统文档（同团队，SKILL.md 字段/占位符规范）：https://www.codebuddy.ai/docs/cli/skills
- CodeBuddy IDE Skills 文档（渐进式加载/三级上下文）：https://www.codebuddy.ai/docs/ide/Features/Skills
- 腾讯官方开源评测基准（产品身份佐证）：https://github.com/Tencent/workbuddy-bench

### 二手来源（仅交叉印证，不作依据）

- 官方市场归档仓库（含装回路径 `~/.workbuddy/skills/`）：https://github.com/infometa/workbuddyskills
- 腾讯 WorkBuddy 生态清单：https://github.com/staruhub/awesome-workbuddy
- OpenClaw 技能注册表 ClawHub（WorkBuddy 兼容生态，非腾讯运营）：https://clawhub.ai/
- 腾讯云开发者社区 WorkBuddy 系列文章（技能路径、记忆文件、SOUL.md/AGENTS.md 说明）：https://cloud.tencent.com/developer/article/2638618 、https://cloud.tencent.com/developer/article/2686207
- 人人都是产品经理《WorkBuddy 记忆完全指南》（SOUL.md/IDENTITY.md/AGENTS.md/MEMORY.md）：http://www.woshipm.com/ai/6441979.html
- 媒体（产品发布/定位背景）：https://technode.com/2026/05/29/tencent-launches-workbuddy-productivity-ai-agent-for-global-users/ 、https://www.cnstock.com/commonDetail/660162
