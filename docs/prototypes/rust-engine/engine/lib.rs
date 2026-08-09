//! Rust 分发引擎签名草案（PROTOTYPE — 不编译，仅供讨论确认）
//!
//! 模块划分与设计说明见 ../README.md。此处为关键 trait / 类型签名 stub。
//! 注释中的「?」标记待确认点。

use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// engine/error.rs — 错误模型
// ---------------------------------------------------------------------------

/// 引擎错误：命令层序列化为 { code, message } 交给前端
#[derive(Debug)]
pub enum EngineError {
    /// 技能 / 快照 / 工具不存在
    NotFound(String),
    /// 状态不允许该操作（如分发前扫描过期）
    InvalidState(String),
    /// 文件系统错误
    Io(String),
    /// 目标路径未配置（如 WorkBuddy 未配置路径）
    Config(String),
    /// 技能校验失败（如 frontmatter 缺 name）
    InvalidSkill(String),
    /// 工具未接入等
    Unsupported(String),
    /// 兜底，不向用户暴露细节
    Internal(String),
}

impl EngineError {
    /// 序列化为 { code, message }，message 为中文文案（UI 中文先行）
    pub fn to_payload(&self) -> ErrorPayload {
        ErrorPayload {
            code: "not_found".to_string(), // 按变体映射
            message: "".to_string(),
        }
    }
}

pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// engine/target/mod.rs — 适配器层
// ---------------------------------------------------------------------------

/// 目标工具标识；与 sidecar `source` / `targets` 取值一致
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    ClaudeCode,
    Codex,
    WorkBuddy,
    Trae,
}

impl ToolId {
    /// "claude-code" / "codex" / "workbuddy" / "trae"
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolId::ClaudeCode => "claude-code",
            ToolId::Codex => "codex",
            ToolId::WorkBuddy => "workbuddy",
            ToolId::Trae => "trae",
        }
    }
}

/// 路径模板：`$HOME` 占位，展开时按用户配置覆盖
#[derive(Debug, Clone)]
pub struct DirTemplate {
    pub relative: PathBuf, // 如 ".agents/skills"
}

/// 说明文件目标模板（filename + 相对用户目录路径）
#[derive(Debug, Clone)]
pub struct InstructionTargetTemplate {
    pub filename: String,      // "CLAUDE.md" / "AGENTS.md"
    pub relative: PathBuf,     // 如 ".claude/CLAUDE.md"
}

/// 技能在 Vault 中的完整表示（含解析后的 frontmatter 与 sidecar）
#[derive(Debug, Clone)]
pub struct Skill {
    pub slug: String,          // 目录名
    pub name: String,          // frontmatter name（展示名）
    pub description: String,   // frontmatter description
    pub version: Option<String>,
    pub dir: PathBuf,          // Vault 内 skills/<slug>/
}

/// 渲染后的待分发文件集（落盘前校验的对象）
#[derive(Debug)]
pub struct RenderedSkill {
    pub files: Vec<RenderedFile>, // SKILL.md + 资源，不含 sidecar
}

#[derive(Debug)]
pub struct RenderedFile {
    pub rel_path: String,
    pub content: Vec<u8>,
}

/// 目标工具适配器：一个实现 = 一个工具的部署与校验行为
pub trait ToolAdapter {
    fn id(&self) -> ToolId;

    /// 默认用户级技能目录模板（可被用户配置覆盖）
    fn default_skills_dir(&self) -> DirTemplate;

    /// 默认说明文件目标；该工具无说明文件时为 None
    fn default_instruction_target(&self) -> Option<InstructionTargetTemplate>;

    /// 渲染分发到本工具的文件集：拷贝 SKILL.md + 资源，注入工具特有 frontmatter 字段，排除 sidecar
    fn render_skill(&self, skill: &Skill) -> Result<RenderedSkill, EngineError>;

    /// 落盘前校验渲染产物（如 frontmatter 必填项），保证不把坏文件写进工具目录
    fn validate(&self, rendered: &RenderedSkill) -> Result<(), EngineError>;
}

/// 已接入工具注册表：由用户配置决定包含哪些工具
pub struct AdapterRegistry {
    adapters: HashMap<ToolId, Box<dyn ToolAdapter>>,
}

impl AdapterRegistry {
    /// 按用户配置构建；禁用的工具不入表
    pub fn from_config(cfg: &UserConfig) -> Result<Self, EngineError> {
        let _ = cfg;
        todo!()
    }

    pub fn get(&self, id: ToolId) -> Option<&dyn ToolAdapter> {
        self.adapters.get(&id).map(|b| b.as_ref())
    }

    /// 已接入工具列表（状态矩阵的列）
    pub fn connected(&self) -> Vec<ToolId> {
        self.adapters.keys().copied().collect()
    }

    /// 分发实际目标路径 = 用户覆盖（若有）或模板展开；未配置（如 WorkBuddy）为 None
    pub fn resolve_skills_dir(&self, id: ToolId, cfg: &UserConfig) -> Option<PathBuf> {
        let _ = (id, cfg);
        todo!()
    }
}

// ---------------------------------------------------------------------------
// engine/vault.rs — Vault 读取
// ---------------------------------------------------------------------------

pub struct VaultInventory {
    pub skills: Vec<Skill>,
    pub instructions: Vec<InstructionSummary>,
}

pub struct InstructionSummary {
    pub slug: String,
    pub dir: PathBuf,
    pub targets: HashMap<String, InstructionTarget>, // 工具 id → 目标路径映射
}

pub struct InstructionTarget {
    pub filename: String,
    pub path: PathBuf,
}

pub fn load_vault(vault_root: &PathBuf) -> Result<VaultInventory, EngineError> {
    let _ = vault_root;
    todo!()
}

// ---------------------------------------------------------------------------
// engine/scanner.rs — 目录扫描（导入与分发共用前置）
// ---------------------------------------------------------------------------

pub struct ScanReport {
    /// 按工具分组：工具端技能目录扫描结果（含文件 hash）
    pub per_tool: HashMap<ToolId, ToolScan>,
    pub scanned_at: String,
}

pub struct ToolScan {
    pub skills: HashMap<String, SkillScanEntry>, // key = 目录名（slug）
}

pub struct SkillScanEntry {
    /// 目录整体 hash（按文件逐一 hash 汇总）
    pub dir_hash: String,
    pub files: HashMap<String, String>, // rel_path → hash
}

/// 全量扫描所有已接入工具端
pub fn scan_targets(registry: &AdapterRegistry, cfg: &UserConfig) -> Result<ScanReport, EngineError> {
    let _ = (registry, cfg);
    todo!()
}

// ---------------------------------------------------------------------------
// engine/status.rs — 状态判定
// ---------------------------------------------------------------------------

/// 技能 × 目标工具的匹配状态（术语表 Status）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// 已分发且未变化
    Consistent,
    /// Vault 已改未分发
    PendingDeploy,
    /// 工具端文件被外部改动
    ToolModified,
    /// 从未分发或已被删除
    Missing,
}

/// 状态矩阵的一格
pub struct StatusCell {
    pub status: Status,
    pub tool_hash: Option<String>,   // 工具端 hash（未扫描到为 None）
    pub record_hash: Option<String>, // SQLite 分发记录 hash（从未分发为 None）
}

pub struct StatusMatrix {
    pub cells: Vec<(String, ToolId, StatusCell)>, // (skill_slug, tool, cell)
}

/// 判定：v = Vault 当前 hash，r = 分发记录 hash，t = 工具端扫描 hash
///
/// | 条件                          | 状态            |
/// | ----------------------------- | --------------- |
/// | 工具端目录不存在               | Missing         |
/// | t == r 且 v == r              | Consistent      |
/// | t == r 且 v != r              | PendingDeploy   |
/// | t != r 且 t == v              | Consistent      | 记录过期，下次分发刷新
/// | t != r 且 t != v              | ToolModified    |
pub fn build_status_matrix(
    vault: &VaultInventory,
    scan: &ScanReport,
    records: &[DeployRecord],
) -> StatusMatrix {
    let _ = (vault, scan, records);
    todo!()
}

// ---------------------------------------------------------------------------
// engine/deploy.rs — 分发事务
// ---------------------------------------------------------------------------

pub struct DeployRequest {
    pub tool_id: ToolId,
    /// 空 = 一键全发（该工具 targets 全集）
    pub skill_slugs: Vec<String>,
    /// 默认 true：分发前自动快照
    pub take_snapshot: bool,
}

pub enum DeployOutcome {
    Deployed {
        skill_slugs: Vec<String>,
        snapshot_id: Option<i64>,
    },
    /// 分发前重扫发现状态变化，未执行，等用户确认后再次触发
    StaleScan {
        changed: Vec<ChangedItem>,
    },
}

pub struct ChangedItem {
    pub skill_slug: String,
    pub before: Status,
    pub after: Status,
}

/// 流程：重扫 → 渲染+校验 → 快照 → staging → 落盘（rename）→ 记录 → 清理
pub fn deploy(
    conn: &Db,
    registry: &AdapterRegistry,
    cfg: &UserConfig,
    req: &DeployRequest,
) -> Result<DeployOutcome, EngineError> {
    let _ = (conn, registry, cfg, req);
    todo!()
}

// ---------------------------------------------------------------------------
// engine/snapshot.rs + db — SQLite 快照模型
// ---------------------------------------------------------------------------

pub struct Db {
    /* rusqlite Connection */
}

/// 快照（表 snapshots）
pub struct Snapshot {
    pub id: i64,
    pub tool_id: ToolId,
    pub reason: SnapshotReason, // 'auto_pre_deploy' | 'manual'
    pub created_at: String,     // ISO 8601
}

pub enum SnapshotReason {
    AutoPreDeploy,
    Manual,
}

/// 快照文件副本存快照目录 ~/.skills-keeper/snapshots/<id>/，SQLite 只记元数据
/// （表 snapshot_files：rel_path + content_hash）

/// 手动触发快照；保留策略：按 tool_id 保留最近 N 份（默认 10，用户可配）
pub fn take_snapshot(
    conn: &Db,
    registry: &AdapterRegistry,
    cfg: &UserConfig,
    tool_id: ToolId,
    reason: SnapshotReason,
    retention: usize,
) -> Result<Snapshot, EngineError> {
    let _ = (conn, registry, cfg, tool_id, reason, retention);
    todo!()
}

/// 分发记录（表 deploy_records，主键 tool_id + skill_slug，每技能×工具一条最新）
pub struct DeployRecord {
    pub tool_id: ToolId,
    pub skill_slug: String,
    pub vault_hash: String, // 分发时 Vault 内容 hash（v）
    pub tool_hash: String,  // 分发后工具端 hash（r，状态比对基准）
    pub deployed_at: String,
}

// ---------------------------------------------------------------------------
// engine/rollback.rs — 回滚
// ---------------------------------------------------------------------------

/// 回滚：目标端当前文件 → .trash（可恢复）→ 快照内容复制回 → 校验 hash → 更新记录
pub fn rollback(
    conn: &Db,
    registry: &AdapterRegistry,
    cfg: &UserConfig,
    snapshot_id: i64,
) -> Result<RollbackReport, EngineError> {
    let _ = (conn, registry, cfg, snapshot_id);
    todo!()
}

pub struct RollbackReport {
    pub snapshot_id: i64,
    pub restored_slugs: Vec<String>,
}

// ---------------------------------------------------------------------------
// engine/import.rs — 导入（MVP 仅技能）
// ---------------------------------------------------------------------------

/// 导入识别结果：来源工具端一层目录扫描
pub struct ImportSourceReport {
    pub tool_id: ToolId,
    /// slug → 识别结果（合规 / 不合规禁勾选 / 非技能跳过）
    pub candidates: Vec<ImportCandidate>,
}

pub enum ImportCandidateKind {
    /// 合规，可勾选
    Ok,
    /// frontmatter 缺 name/description，禁勾选，提示去源工具修复
    Invalid,
    /// 无 SKILL.md，静默跳过（结果页汇总提示）
    NotSkill,
}

pub struct ImportCandidate {
    pub slug: String,
    pub kind: ImportCandidateKind,
    /// 与 Vault 内既有技能的冲突信息（slug 基准去重后）
    pub conflict: Option<ImportConflict>,
}

pub enum ImportConflict {
    /// 同内容：自动跳过
    Duplicate,
    /// 同名不同内容：默认跳过，可改 覆盖（警告旧内容不可恢复）/ 改名导入（后缀自动递增）
    Conflicting,
}

/// 勾选后的导入计划
pub struct ImportPlan {
    pub tool_id: ToolId,
    pub items: Vec<ImportItem>,
}

pub struct ImportItem {
    pub slug: String,
    pub mode: ImportMode, // 导入 / 覆盖 / 改名（新 slug）
}

pub enum ImportMode {
    Import,
    Overwrite,
    Renamed(String),
}

/// Vault 侧先复制到临时目录、成功后整体移入；失败回滚清理
pub fn import(
    conn: &Db,
    registry: &AdapterRegistry,
    cfg: &UserConfig,
    plan: &ImportPlan,
) -> Result<ImportReport, EngineError> {
    let _ = (conn, registry, cfg, plan);
    todo!()
}

pub struct ImportReport {
    pub imported: Vec<String>,
    pub overwritten: Vec<String>,
    pub renamed: Vec<(String, String)>, // (旧 slug, 新 slug)
    pub skipped: Vec<String>,
}

// ---------------------------------------------------------------------------
// commands/ — Tauri command 薄层（仅转发，签名见 README §6）
// ---------------------------------------------------------------------------

pub struct UserConfig {
    /// 工具 → 用户覆盖的目标路径（None = 用模板默认）
    pub tool_dirs: HashMap<ToolId, Option<PathBuf>>,
    pub snapshot_retention: usize,
}
