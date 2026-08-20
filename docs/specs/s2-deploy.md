# S2 切片规格：分发

> 来源：wayfinder 地图「S2 分发（适配器 + 分发事务 + 矩阵分发交互）」（已关闭）四份决议——[S2 调研：四目标工具 SKILL.md 字段要求核查](https://github.com/mq233/skills-keeper/issues/27)（产出 `docs/research/s2-render-fields.md`）、[S2 决议：适配器 trait 与 render_skill 范围](https://github.com/mq233/skills-keeper/issues/28)、[S2 决议：分发事务与自动快照细节](https://github.com/mq233/skills-keeper/issues/29)、[S2 决议：deploy 命令契约与前端分发交互](https://github.com/mq233/skills-keeper/issues/30)
> 状态：ready-for-agent。权威规划见 [docs/technical-plan.md](../technical-plan.md) §8 的 S2 行；术语见 [CONTEXT.md](../../CONTEXT.md)

## Problem Statement

S1 让用户**看到** Skill 库矩阵（一致 / 待分发 / 被工具修改 / 缺失），但矩阵不可操作——引擎只有读取能力（Vault 读取、扫描器、状态判定），没有渲染与落盘能力；命令层全同步；前端没有分发交互。用户无法把 Vault 中的 Skill 复制到目标工具目录，「待分发」只能看不能消。

S2 让矩阵可操作化：引擎层新增适配器层（四目标工具 trait 化渲染与校验）与分发事务（分发前重扫 → 自动快照 → staging → 原子落盘 → 记录），命令层新增 `deploy` 并全命令迁移 async，前端矩阵获得分发交互（行勾选 / 底部批量条 / 列头分发全部），分发成功后矩阵直接置「一致」。

## Solution

S2 交付一个**可操作的分发闭环**：用户在矩阵中勾选 Skill 行（或点击工具列头「分发全部」）→ 引擎按目标工具依次执行分发事务（渲染 → 校验 → 分发前重扫 → 自动快照 → staging → 落盘 → 记录）→ 前端把分发结果写回矩阵（成功置「一致」，失败给出中文原因）。分发前发现工具端被外部修改则中止并提示清单；Codex 同时写入新版与旧版目录（旧版存在时）；不支持回滚与快照时间线（S3）。

## User Stories

1. 作为用户，我想勾选 Skill 行并从底部批量条「分发所选」，把所选 Skill 分发给全部已接入目标工具，以便一键分发
2. 作为用户，我想点击已接入工具列头的「分发全部」，把 Vault 中全部 Skill 分发给该工具，以便整列分发
3. 作为用户，我想在分发前自动重扫目标工具，若发现工具端被外部修改则中止并提示被修改 Skill 清单，以便不覆盖未知内容
4. 作为用户，我想在分发前自动保存目标工具端当前状态的快照，以便 S3 回滚有据
5. 作为用户，我想看到分发的部分成功反馈（成功 N 项 / 失败 M 项 + 每个失败的中文原因），以便知道哪些 Skill 没分发成功及为什么
6. 作为用户，我想在分发成功后矩阵对应单元格直接显示「一致」，以便立即确认结果
7. 作为用户，我想在分发进行中看到加载反馈，以便知道操作正在进行
8. 作为用户，我想 Codex 分发同时写入新版与旧版目录（旧版目录存在时），以便旧版 Codex 也能使用
9. 作为用户，我想不合规 Skill（invalid）分发时被拦截并收到原因反馈，以便修复 Vault 数据
10. 作为用户，我想缺失 Skill（从未分发或已删除）也能勾选分发以重建恢复，以便从工具端删除中恢复
11. 作为用户，我想未接入目标工具不可分发（无「分发全部」入口），以便符合接入语义
12. 作为用户，我想操作失败时看到可读的中文错误信息，以便理解失败原因并重试

## Implementation Decisions

### 1. 适配器层（「S2 决议：适配器 trait 与 render_skill 范围」）

- `ToolAdapter` trait 四方法：`id() -> ToolId`、`default_skills_dir() -> Option<PathBuf>`（`None` = 未接入）、`render_skill(&Skill) -> Result<RenderedSkill, EngineError>`、`validate(&RenderedSkill) -> Result<(), EngineError>`；`default_instruction_target` 移除（说明文件分发留 S5）、`DirTemplate` 留 S5（S2 保留 `Option<PathBuf>`）
- `ToolId` 收敛为纯标识（`as_str` / `FromStr` / `ALL` 保留）；路径与接入判定移入适配器，S1 调用点（`vault.rs::all_connected_targets`、矩阵命令）改注册表查询
- `Skill` 扩展 `body`（SKILL.md 正文）+ frontmatter 结构化保序保留（解析不丢原键）；`RenderedSkill { skill_md: String, resources: Vec<PathBuf>, extra_files: Vec<(PathBuf, String)> }`；render 纯函数（无文件系统访问）
- render 注入两级（通用最小集 → 工具特有补丁）：通用 = `name` 以 slug 覆写、`description` / `version` 保留自原 frontmatter、标准字段（license / compatibility / metadata / allowed-tools）保留；**行为字段按目标剥离**：Claude Code 保留（用户合法行为配置），Codex / Trae / WorkBuddy 剥离（清单 = 调研 §2.1 的 14 个 Claude Code 扩展字段）
- Claude Code / Codex / Trae 无额外注入；Codex `agents/openai.yaml` S2 无数据来源不生成（`extra_files` 预留恒空）；WorkBuddy 按调研 §5.4 全集预留注入逻辑（description_zh/en、display_name、allowed-tools、version），「注入即真实数据」——S2 无数据不注入键
- 排除：`.skill-meta.json` 与隐藏元数据文件不复制（沿用 S1 口径）
- validate 只查必败项：name / description 非空 + name 字符集（小写字母/数字/连字符、无连续连字符、首尾非连字符）+ name == slug 兜底；不合规 → `EngineError::InvalidSkill`（中文错误）
- `AdapterRegistry`：全注册四适配器（含 WorkBuddy）+ connected 过滤；`Vec<Box<dyn ToolAdapter>>` 静态构造；WorkBuddy `default_skills_dir() -> None`、引擎从不调用（connected 过滤）；S5 设置页换配置驱动（用户路径覆盖 = 覆盖适配器路径）
- **Codex 双目录**：同一渲染产物复制两份（新版 `~/.agents/skills/` + 旧版 `~/.codex/skills/`）；旧版目录存在才写、不存在跳过；旧版写入失败 → 告警不失败（不影响主分发结果、不触发回滚，告警承载见 §3 deploy 契约）；旧版不参与扫描 / 状态判定 / 「分发全部」

### 2. 分发事务（「S2 决议：分发事务与自动快照细节」）

- **分发前重扫中止**：判据 = 仅「被工具修改」中止（「缺失」「待分发」「一致」不中止——缺失是防呆：工具端删除 Skill 后勾选分发应能重建恢复）；中止返回被修改 Skill 清单（slug + tool_id）+ 可读提示；整工具重扫（与规划 §4.4 一致），不在本次分发集的被修改 Skill 同样中止（保护工具端整体一致性）；扫描失败 → 中止分发、返回 `Io` 错误
- **自动快照落盘子集**：整工具、工具端 Skill 目录**全量复制**（含隐藏元数据文件——回滚需完整恢复原样）；逐文件复制时计算 blake3 hash（与扫描器同款算法）；不复用重扫结果（口径不同，独立复制计算）；时序 = 插 `snapshots` 行（tool_id / reason='auto_pre_deploy' / created_at）拿 AUTOINCREMENT id → 复制到 `snapshots/<id>/` + 写 `.manifest.json` 冗余清单 → 写 `snapshot_files`（rel_path + content_hash）；SQLite 事务覆盖；失败 → 中止分发（无快照即无回滚能力），错误码 `Io`；S3 保留策略（按 tool_id 保留最近 N 份）S2 不实现、只留接口留痕
- **staging 与原子落盘**：staging 放目标 skills 目录的**父目录**（与目标同盘保证 rename 原子），**禁止放 skills/ 根下**（S1 扫描器把根下任意子目录都算 Skill 候选，staging 残留会被判定成「被工具修改」的脏 Skill）；命名 `.skills-keeper-staging/`；步骤 7 清理 + 失败路径兜底清理；覆盖策略 = 两阶段备份（旧目录 rename 到 staging 内备份位 → 新目录 rename 入位 → 成功后删备份；失败则备份 rename 回原位）；跨盘回退保留（防御性兜底，内容 = 复制 + 逐文件 blake3 校验 + 失败回退清理）
- **deploy_records 写入**：v（vault_hash）= **分发时渲染产物目录 hash**（判定基准从「Vault 原始 hash」变更为「渲染后期望 hash」——render 注入/剥离会改写内容，若 v 存 Vault 原始 hash 则分发后恒「待分发」）；r（tool_hash）= 落盘后工具端实际目录 hash（扫描器口径）；判定调用方 `compute(t, r, v)` 与判定矩阵不变，v 改为喂「当前渲染产物 hash」；写记录时机 = 每 Skill 落盘成功后独立 SQLite 事务提交（部分成功语义），失败回滚该事务、已提交记录保留；**自愈** = 已落盘但记录未写（事务回滚）→ 下次扫描 `t == v` → 「一致」；重试幂等（覆盖为最新渲染产物）
- **部分成功结构（引擎侧）**：`DeployResult { ok: Vec<{tool_id, skill_slug}>, failed: Vec<{tool_id, skill_slug, code, message}> }`；code = EngineError 变体名、message 中文；Skill 级失败（渲染 / 校验 / 落盘）→ 入 failed 继续分发后续 Skill；分发级失败（重扫中止、快照失败）→ 整体 `Err(EngineError)`，不入部分成功

### 3. deploy 命令契约（「S2 决议：deploy 命令契约与前端分发交互」）

- 输入：`{ tool_id: string, skill_slugs: string[] }`，单工具一次调用；引擎无特判——「分发全部」由前端把该工具列全部行（含 invalid 行）的 slug 算出显式传入；「分发所选」= 前端对每个已接入目标工具**串行循环调用**；「分发全部」= 列头单工具一次
- 输出：`DeployResult { ok: [{tool_id, skill_slug}], failed: [{tool_id, skill_slug, code, message}] }`（引擎侧结构原样序列化）
- 错误语义：未接入工具分发 → 分发级 `Err(Config)`；invalid Skill → `Err(InvalidSkill)` 入 failed 继续；重扫中止 → `Err(InvalidState)`，message 文本含被修改清单（slug + tool_id + 可读提示，S2 占位文案）；快照失败 → 分发级 `Err(Io)`
- 循环行为：某工具整体 `Err` → 前端**立即停止循环**并展示错误（重扫中止是工具端整体一致性问题，其余工具大概率同样中止；Io 是环境性问题，停下让用户处理后再整批重发）
- Codex 旧版写入失败告警：告警承载于 failed 结构（沿用 code + message 契约，`code` 可区分）

### 4. async 迁移（「S2 决议：deploy 命令契约与前端分发交互」）

- 形态：仅命令层 async（Tauri async command），**引擎保持同步**，命令内 `spawn_blocking` 包装引擎调用——阻塞 IO 不占 tokio worker、引擎同步纯函数可测性不变
- 单任务队列：`tokio::sync::Mutex<()>` 操作级锁，scan / deploy 共用（防并发文件系统操作）；`get_status_matrix` / `list_skills` 只读不加锁；DB 仍 `std::sync::Mutex<Connection>`（在 spawn_blocking 闭包内获取，锁不跨 await）
- 范围：scan / get_status_matrix / list_skills / deploy 四命令全迁 async；前端 api 层统一 await（invoke 天然异步，`invokeCommand` 扩展参数支持）
- 取消：S2 不支持（分发事务不可中断；loading 态无取消按钮，S6 评估）

### 5. 前端交互（「S2 决议：deploy 命令契约与前端分发交互」）

- **行勾选**：checkbox 列，行级勾选（该 Skill → 全部已接入目标工具），会话态存 store（`Set<slug>`）不落库；invalid 行可勾选（validate 兜底入 failed）、缺失行可勾选（重建恢复）
- **底部批量条**：表格下方，仅勾选非空时显示——「已选 N 项 / 取消 / 分发所选」
- **列头「分发全部」**：仅已接入目标工具列显示按钮（未接入列维持「未接入」提示）
- **loading 态**：分发中批量条按钮与勾选禁用 + loading 文案；循环分发逐工具展示
- **分发后置一致**：ok 条目 → 对应行×列直接置 `consistent`（引擎侧 ok = 落盘 + deploy_records 提交成功，下次扫描必一致，不重扫）；failed 条目保持原状态，下次手动扫描刷新；循环中每工具分发完即置该工具列，不等全部结束
- **部分成功反馈**：批量条原地变结果条「成功 N 项 / 失败 M 项」，失败项展开列表（skill_slug + 中文错误），任意交互清除
- **重扫中止提示**：modal 展示 message + 被修改清单文本，确认关闭后用户再次触发分发
- **UX 文案占位清单**（定稿留 S6）：批量条「已选 N 项 / 取消 / 分发所选」、列头「分发全部」、loading「正在分发…」、结果条「成功 N 项 / 失败 M 项」（失败项错误 message 已中文直接展示）、中止提示「分发前扫描发现以下 Skill 已被目标工具修改：<清单>。请处理后重试」等占位

## Testing Decisions

- **接缝（沿用 S1）**：① 引擎门面——Rust 单元测试 + tempdir 集成测试直接驱动引擎；② api 层——前端组件测试 mock invoke（`vi.mock('@tauri-apps/api/core')`），用契约镜像 fixtures 验证渲染与交互
- **Rust 单测**：render 注入各工具分支（通用覆写 / 行为字段剥离清单 / WorkBuddy 全集预留空注入）、validate 必败项全分支（缺 name / 缺 description / name 字符集 / name != slug）、快照子集（表插入 + 全量复制 + manifest + 失败回滚）、staging / 两阶段备份 / 跨盘回退（tempdir）、deploy_records v/r 写入（每 Skill 独立事务、自愈）
- **Rust 集成测试（tempdir）**：分发端到端——样例 Vault + 模拟工具目录 → deploy 成功（工具端出现渲染产物、deploy_records 正确、矩阵判定「一致」）；**任一步失败回滚**（重扫中止 / 快照失败 / 落盘失败 → staging 清理、已落盘保持）；**幂等重试**（重复分发覆盖为最新渲染产物）；**Codex 双目录分发**（新版 + 旧版各一份，旧版不存在跳过）；部分成功结构（混合 invalid + 合规 Skill）
- **前端组件测试**：行勾选（会话态、取消）、底部批量条（计数 / 分发所选触发循环调用）、列头「分发全部」、loading 禁用态、分发后 ok 行置「一致」、部分成功结果条（失败展开）、中止 modal（确认关闭）
- **验收演示**：`SKILLS_KEEPER_VAULT=examples/vault pnpm tauri dev` → 勾选分发 → 工具端出现渲染产物 + 矩阵变「一致」；`examples/vault` 增加可用于分发的样例分支（验收标准见「S2 端到端验收」ticket）

## Out of Scope

- 回滚引擎、`.trash`、快照时间线页 —— S3
- 快照保留策略与手动快照触发 —— S3
- 行级 diff（Vault vs 工具端）—— S3
- targets 编辑（`update_skill_targets` 命令与 UI）—— S4/S5
- WorkBuddy 路径配置与接入 —— S5 设置页
- 说明文件（Instruction）分发 —— S5
- 分发确认对话框与 UX 文案定稿、分发中取消 —— S6
- Codex 旧版目录中非本应用写入的存量 Skill 管理（不扫不提示不迁移）
- Codex `agents/openai.yaml` 数据生成（S2 无数据源）

## Further Notes

- 术语表：CONTEXT.md（Skill、Sidecar、目标工具、未接入、扫描、状态、Vault、分发、快照、回滚）；快照定义已修正为「目标工具端状态副本，文件存入快照目录、元数据写入 SQLite」
- 实施期已知边界（原地图 Not yet specified）：文件系统边缘情况在分发落盘时的表现（符号链接、只读文件、中文文件名）
- §4.1 模块结构若微调需同步更新 technical-plan.md；`engine/deploy.rs` / `snapshot.rs` / `target/adapters.rs` 现为占位，适配器层落地后 `engine/target/` 的 S1 最小路径层（`ToolId::default_skills_dir` / `connected`）收敛入注册表
- 决议全文存档：GitHub issue #27（调研，产出 `docs/research/s2-render-fields.md`）、#28（适配器 trait 与 render_skill 范围）、#29（分发事务与自动快照细节）、#30（deploy 命令契约与前端分发交互）
