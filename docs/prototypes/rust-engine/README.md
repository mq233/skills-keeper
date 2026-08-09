# Rust 分发引擎草案（原型）

> ⚠️ **PROTOTYPE** — 供 wayfinder ticket「设计 Rust 分发引擎：适配器、状态模型、快照与回滚」（#6）讨论确认的粗坯。
> 确认后结论并入最终技术规划文档（`docs/`），本目录是 throwaway 原型，不代表最终交付物。

配套代码草案：`engine/`（模块划分与关键 trait / 类型签名 stub，不编译）。

## 决策基线（继承既有决议）

| # | 约束 | 来源 |
| --- | --- | --- |
| 1 | 核心引擎全 Rust，前端只做 UI 与状态 | map Notes（grilling 2026-08-08） |
| 2 | 技能目录同构（`skills/<slug>/SKILL.md` + 资源），分发=目录拷贝，排除 sidecar | 「设计技能内部 schema 与 Vault 目录结构」 |
| 3 | sidecar 只存意图元数据；name/description 读 frontmatter；hash/状态归引擎 | 同上 |
| 4 | 用户级目录：Claude `~/.claude/skills/`；Codex 新版 `~/.agents/skills/`（旧 `~/.codex/skills/` 兼容读）；Trae `~/.trae/skills/`（CN 版 `~/.trae-cn/skills/`）；WorkBuddy 官方未证实（社区 `~/.workbuddy/skills/`） | 调研决议 #2 #3 |
| 5 | frontmatter 最小集 name+description（必填）+version；工具特有字段分发时注入 | schema 决议 #4 |
| 6 | 分发时排除 sidecar；WorkBuddy 路径用户可配置 | #4 #3 |
| 7 | 扫描=导入与分发共用前置；分发前重扫，有变化 UX 提示 | 导入决议 #5 |
| 8 | 导入：SKILL.md 唯一判定、一层扫描、slug 基准去重；Vault 侧先复制临时目录成功后整体移入 | #5 |
| 9 | 分发粒度：逐项勾选 + 「一键全发」；快照 SQLite、分发前自动 + 手动触发、保留最近 N 份、时间线回滚 | map Notes |

## 1. 模块划分

```
src/
├── main.rs                 # Tauri 入口（无逻辑）
├── lib.rs                  # 引擎库入口（命令层与测试用）
├── db/
│   ├── mod.rs              # rusqlite 连接、事务 helper
│   └── migrations.rs       # schema 迁移（版本表 + 递增迁移）
├── engine/
│   ├── mod.rs              # 引擎门面（facade）：组合各模块，供命令层调用
│   ├── vault.rs            # Vault 读取：技能/说明文件清单、frontmatter 解析、sidecar 读写
│   ├── scanner.rs          # 目录扫描：目标工具端技能目录、文件 hash（导入与分发共用）
│   ├── status.rs           # 状态判定：一致 / 待分发 / 被工具修改 / 缺失
│   ├── deploy.rs           # 分发事务：渲染 → 快照 → staging → 落盘 → 记录
│   ├── snapshot.rs         # 快照：分发前自动 + 手动触发、保留策略、文件副本管理
│   ├── rollback.rs         # 回滚：从快照恢复目标工具端文件
│   ├── import.rs           # 导入：识别 → 去重比对 → 复制入 Vault（MVP 仅技能）
│   ├── target/
│   │   ├── mod.rs          # ToolId、ToolAdapter trait、AdapterRegistry
│   │   └── adapters.rs     # 四工具适配器实现（claude-code / codex / workbuddy / trae）
│   └── error.rs            # EngineError 错误模型
└── commands/               # Tauri command 薄层（只做参数校验与结果序列化）
    ├── mod.rs
    ├── vault_cmds.rs       # Vault 侧：技能列表、sidecar 编辑
    ├── scan_cmds.rs        # 扫描与状态矩阵
    ├── deploy_cmds.rs      # 分发（含一键全发）、快照列表、回滚
    └── import_cmds.rs      # 导入
```

**分层规则**：`commands/` 不落业务逻辑，只转发；`engine/` 不依赖 Tauri；`db/` 只被 `engine/` 使用。前端（Vue）仅与 `commands/` 对话，错误统一走 `EngineError` 序列化。

## 2. 适配器层（target/）

### 2.1 设计取向：行为入 trait，路径入配置

四个适配器的**行为差异**很小（都是「SKILL.md 目录拷贝 + frontmatter 注入」），**路径差异**才是大头（Codex 双目录、Trae CN/国际版、WorkBuddy 未证实路径）。因此 trait 管行为与默认值，路径解析走「模板 + 用户覆盖」：

- 适配器提供**默认路径模板**（如 Codex 优先 `~/.agents/skills/`，旧 `~/.codex/skills/` 仅兼容读取）
- 用户配置可覆盖任何目标路径（WorkBuddy 必须先配置，未配置则禁用分发、状态显示为未接入）
- 说明文件路径同理（`.instruction-meta.json` 的 `filename + path` 映射）

### 2.2 ToolAdapter trait 草案

```rust
/// 目标工具标识；与 sidecar `source`/`targets` 取值一致
pub enum ToolId {
    ClaudeCode,
    Codex,
    WorkBuddy,
    Trae,
}

impl ToolId {
    pub fn as_str(&self) -> &'static str;   // "claude-code" / "codex" / "workbuddy" / "trae"
}

/// 目标工具适配器：一个实现 = 一个工具的部署与校验行为
pub trait ToolAdapter {
    fn id(&self) -> ToolId;

    /// 默认用户级技能目录模板（含 home 占位），可被用户配置覆盖
    fn default_skills_dir(&self) -> DirTemplate;

    /// 默认说明文件目标（filename + 相对用户目录路径）；不支持为 None
    fn default_instruction_target(&self) -> Option<InstructionTargetTemplate>;

    /// 渲染分发到本工具的技能文件集：
    /// 拷贝 SKILL.md + 资源，注入工具特有 frontmatter 字段，排除 sidecar
    fn render_skill(&self, skill: &Skill) -> Result<RenderedSkill, EngineError>;

    /// 落盘前校验渲染产物（如 frontmatter 必填项），保证不把坏文件写进工具目录
    fn validate(&self, rendered: &RenderedSkill) -> Result<(), EngineError>;
}
```

### 2.3 适配器注册表

```rust
/// 已接入工具的注册表：由用户配置决定包含哪些工具
pub struct AdapterRegistry {
    adapters: HashMap<ToolId, Box<dyn ToolAdapter>>,
}

impl AdapterRegistry {
    /// 按用户配置构建；配置里禁用的工具不入表
    pub fn from_config(cfg: &UserConfig) -> Result<Self, EngineError>;
    pub fn get(&self, id: ToolId) -> Option<&dyn ToolAdapter>;
    /// 已接入工具列表（状态矩阵的列）
    pub fn connected(&self) -> Vec<ToolId>;
    /// 分发时实际目标路径 = 用户覆盖（若有）或模板展开
    pub fn resolve_skills_dir(&self, id: ToolId, cfg: &UserConfig) -> Option<PathBuf>;
}
```

**职责边界**：适配器只回答「这个工具长什么样、写到哪、怎么校验」；「哪些技能要发、何时发」由 `deploy.rs` 决定；「工具端现在有什么」由 `scanner.rs` 决定。四工具差异全部收敛在 4 个适配器实现里。

## 3. 状态模型（status.rs）

### 3.1 判定输入：三方 hash

每次判定以「扫描结果 + SQLite 分发记录 + Vault 当前内容」三方为输入：

```
v = Vault 当前技能目录 hash（全局：按文件逐一 hash 汇总）
r = SQLite deploy_records 中上次分发到该工具时的 hash
t = 工具端扫描得到的技能目录 hash
```

### 3.2 判定矩阵

| 条件 | 状态（Status） | 含义 |
| --- | --- | --- |
| 工具端目录不存在 | `Missing` | 从未分发或已被删除 |
| `t == r` 且 `v == r` | `Consistent`（一致） | 与上次分发一致，Vault 未变 |
| `t == r` 且 `v != r` | `PendingDeploy`（待分发） | 上次分发后 Vault 改了，工具端是旧版本 |
| `t != r` 且 `t == v` | `Consistent`（一致） | 工具端内容恰与 Vault 当前一致（如手工重放），记录过期；下次分发时刷新记录即可 |
| `t != r` 且 `t != v` | `ToolModified`（被工具修改） | 工具端被外部改动，与 Vault 与记录都不一致 |

> `Missing` 优先级最高；`t == r` 且 `v == r` 是唯一需要查 Vault hash 的「干净」情形——`v` 仅在 `t == r` 或 `t != r 且 t == v` 时需计算，避免每次扫描全量 hash Vault。

### 3.3 类型草案

```rust
pub enum Status { Consistent, PendingDeploy, ToolModified, Missing }

/// 状态矩阵的一格：技能 × 目标工具
pub struct StatusCell {
    pub status: Status,
    pub tool_hash: Option<String>,   // 工具端 hash（NULL = 未扫描到）
    pub record_hash: Option<String>, // SQLite 分发记录 hash
}

/// 判定入口：扫描结果 + 记录，产出矩阵
pub fn build_status_matrix(
    vault: &VaultInventory,
    scan: &ScanReport,
    records: &[DeployRecord],
) -> StatusMatrix;
```

## 4. 分发事务（deploy.rs）

### 4.1 流程与原子性边界

```
选择技能集（逐项 / 一键全发）
  → 1. 重扫目标工具（分发前共用前置，见决议 #7）
     有状态变化 → 中止并返回提示，待用户确认后再次触发
  → 2. 渲染（render_skill）并校验（validate）
  → 3. 快照（自动）：把目标工具端当前文件复制到快照目录 + 写 snapshots 表
  → 4. staging：渲染产物写入临时目录（Vault 侧，与目标同盘保证 rename 原子性）
  → 5. 落盘：逐个技能 rename 到目标工具目录；跨盘回退为 复制+校验+删除
  → 6. 记录：写 deploy_records（v、t、时间）
  → 7. 清理 staging
```

- **失败恢复**：3–6 任一步失败 → 清理 staging、SQLite 回滚、已落盘的技能保持原样（不做补偿回滚——文件系统无两阶段提交；失败发生在落盘阶段的中间时，重试即可，因为分发是幂等的）
- **原子性边界**：SQLite 事务覆盖步骤 3/6 的写；文件系统以「技能为单位」原子（单个技能目录要么新要么旧）；跨技能无原子保证（失败后状态矩阵如实反映，用户可重试）
- 快照在落盘**前**做，记录的是「分发前的工具端状态」——这正是回滚要恢复的对象

### 4.2 类型草案

```rust
pub struct DeployRequest {
    pub tool_id: ToolId,
    pub skill_slugs: Vec<String>,   // 空 = 一键全发（该工具 targets 全集）
    pub take_snapshot: bool,        // 默认 true（分发前自动快照）
}

pub enum DeployOutcome {
    Deployed { skill_slugs: Vec<String>, snapshot_id: Option<i64> },
    StaleScan { changed: Vec<ChangedItem> },  // 重扫发现变化，未执行
}

pub fn deploy(conn: &Db, registry: &AdapterRegistry, req: &DeployRequest) -> Result<DeployOutcome, EngineError>;
```

## 5. SQLite 快照模型（db/migrations.rs）

### 5.1 表结构草案

```sql
-- 快照（分发前自动 / 手动触发）
CREATE TABLE snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    tool_id     TEXT NOT NULL,          -- 回滚粒度：单工具
    reason      TEXT NOT NULL,          -- 'auto_pre_deploy' | 'manual'
    created_at  TEXT NOT NULL           -- ISO 8601
);

-- 快照文件清单：内容副本存快照目录，表只记元数据
CREATE TABLE snapshot_files (
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    rel_path    TEXT NOT NULL,          -- 相对技能目录的路径（SKILL.md、资源…）
    content_hash TEXT NOT NULL,         -- 校验完整性
    PRIMARY KEY (snapshot_id, rel_path)
);

-- 分发记录：每 技能×工具 一条最新记录（状态比对基准 r）
CREATE TABLE deploy_records (
    tool_id     TEXT NOT NULL,
    skill_slug  TEXT NOT NULL,
    vault_hash  TEXT NOT NULL,          -- 分发时 Vault 内容 hash（v）
    tool_hash   TEXT NOT NULL,          -- 分发后工具端 hash（r）
    deployed_at TEXT NOT NULL,
    PRIMARY KEY (tool_id, skill_slug)
);
```

- **快照目录**：应用数据目录 `~/.skills-keeper/snapshots/<snapshot_id>/`（skill 端文件副本 + 一份 `.manifest.json` 冗余清单，便于不依赖 DB 的人工排查）
- **内容不进 SQLite**：文件副本放磁盘目录，SQLite 只存元数据（二进制 blob 进 SQLite 会让备份与增量管理变重）
- **保留策略**：按 `tool_id` 分组、按 `created_at` 保留最近 N 份（默认 10，用户可配），淘汰 = 删目录 + 删行

### 5.2 回滚语义（rollback.rs）

```
选择快照（时间线列表）→ 确认（可能覆盖工具端现有内容）
  → 1. 当前工具端文件移动到回收目录（.trash，可恢复，防误删）
  → 2. 快照目录内容复制回目标技能目录
  → 3. 校验 content_hash
  → 4. 更新 deploy_records（vault_hash 不变、tool_hash 改为恢复后 hash）—— 状态回归「一致」
  → 5. 清理回收目录（若第 2 步失败，从回收目录恢复原状）
```

```rust
pub fn rollback(conn: &Db, registry: &AdapterRegistry, snapshot_id: i64) -> Result<RollbackReport, EngineError>;
```

## 6. 命令层（commands/）

### 6.1 暴露面草案

```rust
// Vault
#[tauri::command] async fn list_skills(state) -> Result<Vec<SkillSummary>, String>;
#[tauri::command] async fn list_instructions(state) -> Result<Vec<InstructionSummary>, String>;
#[tauri::command] async fn update_skill_targets(slug, targets) -> Result<(), String>;

// 扫描与状态
#[tauri::command] async fn scan(state) -> Result<ScanReport, String>;          // 手动刷新
#[tauri::command] async fn get_status_matrix(state) -> Result<StatusMatrix, String>;

// 分发与快照
#[tauri::command] async fn deploy(req: DeployRequest) -> Result<DeployOutcome, String>;
#[tauri::command] async fn list_snapshots(tool_id: Option<String>) -> Result<Vec<SnapshotInfo>, String>;
#[tauri::command] async fn rollback(snapshot_id: i64) -> Result<RollbackReport, String>;

// 导入
#[tauri::command] async fn scan_import_sources() -> Result<ImportSourceReport, String>;
#[tauri::command] async fn import(plan: ImportPlan) -> Result<ImportReport, String>;
```

### 6.2 错误模型

```rust
pub enum EngineError {
    NotFound(String),            // 技能/快照/工具不存在
    InvalidState(String),        // 状态不允许该操作（如分发前扫描过期）
    Io(String),                  // 文件系统错误
    Config(String),              // 路径未配置（如 WorkBuddy 未配置路径）
    InvalidSkill(String),        // 校验失败（如 frontmatter 缺 name）
    Unsupported(String),         // 工具未接入等
    Internal(String),            // 兜底，不向用户暴露细节
}

impl EngineError {
    /// 序列化为 { code, message } JSON；message 中文文案（UI 中文先行）
    pub fn to_payload(&self) -> ErrorPayload;
}
```

- 命令层 `Result<T, EngineError>` → Tauri 序列化时统一转 `String`（前端按 `code` 分支提示，不做类型恢复）
- 后台慢操作（扫描、分发）走 `async` + Tauri `State` 串行化（MVP 单任务队列，避免并发写 SQLite）

## 7. 确认记录（2026-08-09 与用户确认）

| # | 决策点 | 结论 |
| --- | --- | --- |
| 1 | 快照粒度 | **按工具整体**：每次分发前快照整个工具端技能目录；回滚时间线简单直观，MVP 技能量小可接受 |
| 2 | 回滚后是否留新快照 | **不留**，回滚前工具端内容移入 `.trash` 兜底（可恢复），避免快照无限膨胀 |
| 3 | 一键全发范围 | **targets 全集**：分发该工具 targets 标记的全部技能；与逐项勾选正交 |
| 4 | 说明文件分发 | 复用 deploy 流程（内容实体 + 工具映射路径），独立渲染（无 frontmatter 注入） |
| 5 | 保留 N 默认值 | 默认 10，用户可配 |
| 6 | WorkBuddy 未配置路径 | 状态列显示「未接入」且不可分发；用户配置路径后接入 |
| 7 | 分发失败恢复 | **技能级原子 + 可重试**：单技能 rename 原子，跨技能无原子保证，失败如实反映、重试即幂等 |
| 8 | 状态判定 | 三方 hash 判定矩阵（见 §3.2）；「t == v 但 t != r」判 Consistent，记录过期下次分发刷新 |
