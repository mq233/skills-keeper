# S1 切片规格：Skill 库矩阵（只读）

> 来源：wayfinder 地图「S1 库矩阵（只读）实施规划」（已关闭）三份决议——#18（依赖选型）、#19（引擎契约）、#20（命令契约与前端架构）
> 状态：ready-for-agent。权威规划见 [docs/technical-plan.md](../technical-plan.md) §8 的 S1 行；术语见 [CONTEXT.md](../../CONTEXT.md)

## Problem Statement

用户把 Skill（技能）统一托管在本地 Vault（单一事实源）中，但当前**看不到**每个 Skill 在四个目标工具（Claude Code、Codex、workbuddy、Trae）里的分发状态：哪些已分发、哪些 Vault 改了还没分发、哪些被工具端外部修改、哪些从未分发。没有状态可见性，用户就无法判断"该分发了"或"工具端被改坏了"。此外 Vault 中可能混入不合规的 Skill（缺 name/description、SKILL.md 缺失），用户需要知道问题出在哪。

## Solution

S1 交付一个**只读的 Skill 库矩阵视图**：行 = Skill（含名称、描述、版本），列 = 目标工具（含未接入列），单元格 = 状态徽章（一致 / 待分发 / 被工具修改 / 缺失）。矩阵顶部有手动扫描按钮与状态摘要（被工具修改 / 待分发 / 缺失计数）。引擎层完成 SQLite 三表迁移、Vault 读取（frontmatter 解析、Sidecar 读写、invalid 标记）、扫描器与状态判定（判定矩阵全分支），通过三个 Tauri 命令（`list_skills` / `scan` / `get_status_matrix`）与前端 `src/api/` 封装对接。仓库内置样例 Vault，可通过环境变量指向后直接跑出真实矩阵。

## User Stories

1. 作为用户，我想看到 Vault 中全部 Skill 的列表（slug、名称、描述、版本），以便了解我托管了哪些技能
2. 作为用户，我想看到每个 Skill 在**每个已接入目标工具**中的状态（一致 / 待分发 / 被工具修改 / 缺失），以便判断哪里需要分发
3. 作为用户，我想看到未接入的目标工具（如未配置路径的 workbuddy）在矩阵中显示"未接入"列并给出配置提示，以便知道如何启用它
4. 作为用户，我想看到矩阵顶部的状态摘要（被工具修改 / 待分发 / 缺失计数），以便快速了解整体健康度
5. 作为用户，我想手动触发扫描刷新矩阵，以便在工具端或 Vault 变化后获得最新状态
6. 作为用户，我想在扫描进行中看到加载反馈，以便知道操作正在进行而不是界面卡死
7. 作为用户，我想看到不合规 Skill（缺 name/description、frontmatter 解析失败、SKILL.md 缺失、Sidecar 损坏）的行级标记与原因，以便修复 Vault 数据
8. 作为用户，我想看到 Sidecar 缺失的 Skill 仍能正常显示（按默认语义合成），以便旧 Vault 也能迁移使用
9. 作为用户，我想看到含资源文件的 Skill 状态被正确判定，以便确认整个 Skill 目录的一致性
10. 作为用户，我想看到中文名 / 特殊字符名的 Skill 正常显示，以便不受命名限制
11. 作为用户，我想在操作失败时看到可读的中文错误信息（而不是技术堆栈），以便理解失败原因并采取行动
12. 作为用户，我想通过左侧栏导航在四个页面间切换（S1 仅 Skill 库可用，其余占位），以便后续切片逐页填充
13. 作为用户，我想用环境变量把 Vault 指向样例目录，以便在不污染真实数据的前提下体验真实矩阵
14. 作为用户，我想在工具端完全不存在时看到所有 Skill 显示"缺失"，以便理解工具端从未初始化
15. 作为用户，我想在工具端目录内容与 Vault 当前一致但无分发记录时看到"一致"，以便不重复操作（判定矩阵 r 缺失分支）

## Implementation Decisions

### 1. 目录与数据位置

- Vault 根：`~/.skills-keeper/vault/`；SQLite 库：`~/.skills-keeper/skills-keeper.db`（快照目录 `~/.skills-keeper/snapshots/` 属 S3）；db 文件与 Vault 同根
- 命令层路径解析支持环境变量覆盖：`SKILLS_KEEPER_VAULT` / `SKILLS_KEEPER_DATA`（开发 / 测试 / 演示用；生产不设置即默认值）

### 2. Skill 实体与 Vault 读取

- 实体：`Skill { slug, name, description, version: Option<String>, sidecar }`；slug = 目录名（S1 不校验）
- frontmatter 用 serde_yaml_ng 的 `Value` + 手写三字段提取；`version` 对数值 / 布尔标量转字符串原文（规避 YAML 1.1 标量推断陷阱，如 `version: 1.0`）
- **invalid 是数据不是错误**：读取返回 `SkillEntry { skill, invalid: Option<String> }`；SKILL.md 缺失、frontmatter 解析失败、缺 name/description、Sidecar JSON 损坏 → invalid + 原因；仅 Io 级错误才传播错误
- Sidecar 缺失 → 容错合成默认（source = null、targets = 全部已接入工具）；Sidecar 损坏 → invalid 标记，不静默丢弃

### 3. Skill 目录 hash

- blake3；目录 hash = 排序后的（相对路径 + 各文件内容 hash）流式喂入——重命名可感知
- **排除隐藏元数据文件**（`.skill-meta.json`）——与分发排除、导入去重口径一致
- hash 表示为十六进制字符串（对应 SQLite TEXT 列）

### 4. 扫描器

- `scan_tool(ToolId) -> ScanResult` 逐工具接口（单测友好；S4 导入器可复用）；未接入工具不扫描
- 工具端 skills/ 根下**直接子目录全算** Skill 候选（目录同构），根下散文件忽略；目录存在即算（无 SKILL.md 也计入，由 hash 比对自然得出"被工具修改"）
- 工具端根目录不存在 → 空清单（"缺失"由状态层判定）
- ScanResult 含文件清单（相对路径 + 文件 hash，S3 行级 diff 用）+ 目录 hash

### 5. 目标工具路径（最小 target 层）

- `ToolId` 枚举（claude-code / codex / workbuddy / trae）+ `default_skills_dir(ToolId) -> Option<PathBuf>`（`~` 展开；workbuddy 官方未公开路径 → None；Trae 默认国际版 `~/.trae/skills/`）
- 接入判定 = 路径可得（connected ⇔ is_some）；trait 化（render/validate）与用户配置留 S2/S5

### 6. WorkBuddy 未接入

- 引擎判定并输出 `ToolId` 全集 + connected 标志，前端只渲染；"未接入"是列级属性，单元格仅四态

### 7. db 与迁移

- 三表 DDL 按技术规划 §3.5（snapshots / snapshot_files / deploy_records）；`PRAGMA user_version` + 递增迁移数组（不引 refinery）
- 启动时 `engine::init_db(path)`（命令层 setup 取路径传入，引擎不依赖 Tauri）；`Mutex<Connection>` 归 db 模块；S1 读取 deploy_records（恒空）

### 8. 状态判定

- `compute(t: Option<&str>, r: Option<&str>, v: Option<&str>) -> Status` 纯函数，覆盖判定矩阵全分支：缺失（t None）/ 一致 / 待分发 / 被工具修改；r 缺失时走 t==v 分支（无记录 + 内容一致 → 一致）
- v 计算：t None 时跳过，其余全量计算（§4.3 惰性表述为优化意图，S1 从简）

### 9. 命令契约（JSON）

- `list_skills` → `SkillEntry[]`（S1 前端不调用，S4 导入器使用）
- `get_status_matrix` → `{ tools: [{ id, connected }], rows: [{ skill: SkillEntry, statuses: { [toolId]: Status } }] }`；SkillEntry 的 invalid 含原因文案
- `scan` → 返回最新矩阵（同 get_status_matrix 形状），手动扫描一次往返
- 错误：命令层 `Result<T, EngineError>`，EngineError 序列化为 `{code, message}`；code = 变体名（NotFound / InvalidState / Io / Config / InvalidSkill / Internal），message 为中文文案
- 命令全同步签名（Tauri 同步 command 运行于后台线程，前端 invoke 天然异步）；`Mutex<Connection>` 串行化防并发；async + 任务队列留 S2

### 10. 前端架构

- 手写 TS 类型镜像（与命令契约一一对应），不引入类型生成器
- api 封装：`listSkills() / scan() / getStatusMatrix()`；统一错误解析——`{code, message}` → 抛带 `code` 属性的 Error，message 中文直接展示
- 单 Pinia store：state `{ tools, rows, loading, error }`；actions `loadMatrix() / scan()`；getter `summary`（被工具修改 / 待分发 / 缺失计数）
- 组件：页面容器 + 矩阵表格 + 矩阵摘要（扫描按钮 + 计数）+ 状态徽章；列头"分发全部"按钮占位留 S2、行内 diff 展开占位留 S3；文案中文直写（data-i18n 迁移是 S6）
- 路由：vue-router + 左侧栏四页导航（Skill 库实现，导入 / 快照时间线 / 设置占位）；清除脚手架 greet 模板

### 11. 依赖

- Rust：rusqlite 0.40 bundled、serde_yaml_ng 0.10、blake3 1（Tauri 依赖树零冲突，仅自带 sha2）
- 前端：pinia、vue-router、vitest.config.ts（environment: happy-dom）、@vue/test-utils、happy-dom（新增依赖同步 package.json 与锁文件）

## Testing Decisions

- **好测试的标准**：只测外部行为，不测实现细节——判定矩阵以 (t, r, v) 输入 × 预期状态为用例表；解析测试以真实样例文件为夹具
- **接缝（两个，自高而低）**：① 引擎门面——Rust 单元测试 + tempdir 集成测试直接驱动引擎，验证"读取 Vault → 扫描工具 → 判定状态"全流程；② api 层——前端组件测试 mock invoke（`vi.mock('@tauri-apps/api/core')`），用契约镜像 fixtures 验证渲染与交互。前端不 mock 引擎内部的任何东西
- **Rust 单测**：frontmatter / Sidecar 解析（含 invalid 各分支、YAML 标量陷阱）、表迁移（user_version 递增）、判定矩阵全分支、扫描器（hash 构成、根不存在、子目录判定）、target 路径（~ 展开、workbuddy None）
- **Rust 集成测试**：tempdir 下建样例 Vault + 模拟工具目录 → `list_skills` / `scan` / `get_status_matrix` 返回契约形状数据（含未接入列）
- **前端组件测试**：矩阵渲染（行 × 列）、状态徽章四态 + 未接入列、状态摘要计数、加载态、错误态（{code, message} 中文展示）、扫描按钮触发 store action
- **验收演示**：`SKILLS_KEEPER_VAULT=examples/vault pnpm tauri dev` → 真实矩阵（样例 Vault 覆盖正常 / 资源文件 / 中文名 / invalid 分支）
- **既有先例**：Phase 0 已建立 cargo test / clippy / vitest / CI 三段式基线；组件测试为仓库首批前端测试

## Out of Scope

- 分发（S2：适配器 trait、分发事务、矩阵分发交互）
- 差异展示与回滚（S3：行级 diff、快照时间线页、rollback）
- 导入（S4：导入向导、去重冲突处理）
- 说明文件与设置（S5：Instruction 分发、设置页、Trae CN/国际版选择、Vault 路径可改、workbuddy 路径配置）
- 打磨与发布准备（S6：分发前重扫 UX 文案、i18n 抽取、E2E、三平台安装包）
- MVP 之后能力（双向同步、项目级分发、Vault 版本历史、说明文件导入等）

## Further Notes

- 术语表：CONTEXT.md（Skill、Sidecar、目标工具、未接入、扫描、状态、Vault、分发、导入、快照、回滚）
- 实施期已知边界（原地图 Not yet specified）：大 Skill 量 / 大工具量扫描开销、文件系统边缘情况（符号链接、中文文件名、只读文件）；§4.1 模块结构若微调需同步更新 technical-plan.md
- S1 的 target 路径解析是 S5 设置页（用户配置覆盖）的天然扩展点
- 决议全文存档：GitHub issue #18（依赖选型，含调研文档 docs/research/s1-deps.md）、#19（引擎契约）、#20（命令契约与前端架构）
