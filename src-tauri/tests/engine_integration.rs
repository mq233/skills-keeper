//! S1 集成测试（`docs/specs/s1-matrix.md` §Testing Decisions 接缝 ①）：
//! tempdir 下建样例 Vault + 模拟工具目录 → 引擎门面返回契约形状数据。
//!
//! 引擎不依赖 Tauri；命令层（scan / get_status_matrix）薄转发至此门面，
//! 环境变量路径解析（AppPaths）一并在此验证。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use skills_keeper_lib::commands::AppPaths;
use skills_keeper_lib::engine;
use skills_keeper_lib::engine::error::EngineError;
use skills_keeper_lib::engine::status::Status;
use skills_keeper_lib::engine::target::{AdapterRegistry, ToolId};
use tempfile::TempDir;

/// 环境变量类测试互斥（set_var 是全局副作用，Rust 测试并发运行）。
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// 测试内临时改 cwd 的 RAII 守卫（与 ENV_LOCK 同锁串行，失败时也恢复）。
struct CwdGuard(PathBuf);

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

const GOOD_MD: &str =
    "---\nname: 问候助手\ndescription: 生成友好问候语\nversion: 1.0\n---\n# 内容\n";

/// GOOD_MD 经 render 后的产物（name 以 slug 覆写；S2 判定基准 = 渲染产物，
/// 模拟工具端应放置渲染后内容才判定「一致」；纯数字字符串序列化带单引号）。
const RENDERED_MD: &str =
    "---\nname: greeting\ndescription: 生成友好问候语\nversion: '1.0'\n---\n# 内容\n";

/// 建样例 Vault：`greeting` 正常 Skill；`broken` 缺 name（invalid 分支）。
fn make_vault(root: &Path) {
    write(&root.join("skills/greeting/SKILL.md"), GOOD_MD);
    write(
        &root.join("skills/greeting/.skill-meta.json"),
        r#"{"schemaVersion": 1, "source": null, "targets": ["claude-code", "codex", "trae"]}"#,
    );
    write(
        &root.join("skills/broken/SKILL.md"),
        "---\ndescription: 只有描述\n---\n",
    );
}

/// 注入式注册表：claude-code 指向模拟目录，codex/trae 指向另一模拟目录，
/// workbuddy 无覆盖 = 未接入（None）。S2 适配器化：路径覆盖 = 测试注入。
fn make_registry(cc_root: &Path, other_root: &Path) -> AdapterRegistry {
    AdapterRegistry::with_overrides(HashMap::from([
        (ToolId::ClaudeCode, cc_root.to_path_buf()),
        (ToolId::Codex, other_root.to_path_buf()),
        (ToolId::Trae, other_root.to_path_buf()),
    ]))
}

#[test]
fn 契约形状_未接入列_r缺失分支() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    make_vault(&vault);

    // 模拟工具端：claude-code 存在且内容为渲染产物（无分发记录 → r 缺失 → t==v → 一致；
    // S2 判定基准 = 渲染产物，非 Vault 原始内容）
    let cc = temp.path().join("cc");
    write(&cc.join("greeting/SKILL.md"), RENDERED_MD);
    // codex / trae 根不存在 → 缺失
    let empty = temp.path().join("empty-tools");

    let db = engine::init_db(&temp.path().join("skills-keeper.db")).unwrap();
    let matrix = engine::get_status_matrix(&vault, &make_registry(&cc, &empty), &db).unwrap();

    // tools：四列全集 + connected 标志（未接入是列级属性）
    let tools: Vec<(&str, bool)> = matrix
        .tools
        .iter()
        .map(|t| (t.id.as_str(), t.connected))
        .collect();
    assert_eq!(
        tools,
        vec![
            ("claude-code", true),
            ("codex", true),
            ("workbuddy", false),
            ("trae", true),
        ],
        "未接入列正确表达"
    );

    // 行 × 状态：greeting 一致（r 缺失 + t==v）、其余缺失；未接入列不在 statuses
    let rows = &matrix.rows;
    assert_eq!(rows.len(), 2);
    let greeting = rows
        .iter()
        .find(|r| r.skill.skill.slug == "greeting")
        .unwrap();
    assert_eq!(greeting.skill.skill.name, "问候助手");
    assert_eq!(greeting.skill.skill.version.as_deref(), Some("1.0"));
    assert_eq!(greeting.skill.invalid, None);
    assert!(
        !greeting.statuses.contains_key("workbuddy"),
        "未接入列单元格无状态"
    );
    assert_eq!(greeting.statuses["claude-code"], Status::Consistent);
    assert_eq!(greeting.statuses["codex"], Status::Missing);
    assert_eq!(greeting.statuses["trae"], Status::Missing);

    // invalid 分支：broken 行级标记原因
    let broken = rows
        .iter()
        .find(|r| r.skill.skill.slug == "broken")
        .unwrap();
    assert!(
        broken.skill.invalid.as_deref().unwrap().contains("name"),
        "invalid 应点名缺 name"
    );
}

#[test]
fn 判定链路_分发记录与变化反映() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    make_vault(&vault);
    let cc = temp.path().join("cc");
    // 工具端为渲染产物（S2 判定基准：分发后工具端内容 = 渲染产物）
    write(&cc.join("greeting/SKILL.md"), RENDERED_MD);
    let other = temp.path().join("other");

    let db = engine::init_db(&temp.path().join("skills-keeper.db")).unwrap();
    let registry = make_registry(&cc, &other);

    // 插入分发记录：模拟 S2 分发后状态（r = 工具端 hash；v 记录值不参与矩阵判定，
    // 矩阵 v 现算渲染产物 hash）
    let v = engine::scanner::hash_dir(&vault.join("skills/greeting")).unwrap();
    let t = engine::scanner::hash_dir(&cc.join("greeting")).unwrap();
    {
        let conn = db.lock();
        conn.execute(
            "INSERT INTO deploy_records (tool_id, skill_slug, vault_hash, tool_hash, deployed_at)
             VALUES ('claude-code', 'greeting', ?1, ?2, '2026-08-09T00:00:00Z')",
            rusqlite::params![v, t],
        )
        .unwrap();
    }

    // t == r 且 v == r → 一致
    let matrix = engine::get_status_matrix(&vault, &registry, &db).unwrap();
    let greeting = matrix
        .rows
        .iter()
        .find(|r| r.skill.skill.slug == "greeting")
        .unwrap();
    assert_eq!(greeting.statuses["claude-code"], Status::Consistent);

    // 工具端被外部修改（t != r 且 t != v）→ 被工具修改
    write(
        &cc.join("greeting/SKILL.md"),
        "---\nname: 被改了\ndescription: x\n---\n",
    );
    let matrix = engine::get_status_matrix(&vault, &registry, &db).unwrap();
    let greeting = matrix
        .rows
        .iter()
        .find(|r| r.skill.skill.slug == "greeting")
        .unwrap();
    assert_eq!(greeting.statuses["claude-code"], Status::Modified);

    // 恢复工具端一致后，Vault 变化（v != r 且 t == r）→ 待分发
    write(&cc.join("greeting/SKILL.md"), RENDERED_MD);
    write(
        &vault.join("skills/greeting/SKILL.md"),
        "---\nname: 问候助手\ndescription: 更新后的描述\nversion: 1.1\n---\n",
    );
    let matrix = engine::get_status_matrix(&vault, &registry, &db).unwrap();
    let greeting = matrix
        .rows
        .iter()
        .find(|r| r.skill.skill.slug == "greeting")
        .unwrap();
    assert_eq!(greeting.statuses["claude-code"], Status::Pending);
}

#[test]
fn list_skills_契约与invalid() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    make_vault(&vault);

    let entries = engine::list_skills(&vault, &AdapterRegistry::new()).unwrap();
    assert_eq!(entries.len(), 2);
    let slugs: Vec<&str> = entries.iter().map(|e| e.skill.slug.as_str()).collect();
    assert_eq!(slugs, vec!["broken", "greeting"], "按 slug 排序");
    let greeting = entries.iter().find(|e| e.skill.slug == "greeting").unwrap();
    assert_eq!(greeting.skill.name, "问候助手");
    assert_eq!(
        greeting.skill.sidecar.targets,
        vec!["claude-code", "codex", "trae"]
    );
    let broken = entries.iter().find(|e| e.skill.slug == "broken").unwrap();
    assert!(broken.invalid.is_some());
}

#[test]
fn init_db_数据目录不存在时自动创建() {
    let temp = tempfile::tempdir().unwrap();
    // 首次运行场景：应用数据目录（如默认 ~/.skills-keeper/）尚不存在
    let data_dir = temp.path().join("nested/data");
    let db_path = data_dir.join("skills-keeper.db");
    let db = engine::init_db(&db_path).unwrap();
    assert!(data_dir.exists(), "数据目录应被创建");
    assert!(db_path.exists(), "db 文件应已创建");

    // 创建后可正常初始化并迁移（空 Vault → 空矩阵）
    let matrix = engine::get_status_matrix(
        &temp.path().join("vault"),
        &make_registry(&temp.path().join("cc"), &temp.path().join("other")),
        &db,
    )
    .unwrap();
    assert!(matrix.rows.is_empty());
    assert_eq!(matrix.tools.len(), 4);
}

#[test]
fn 空vault返回空矩阵() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault"); // 无 skills/ 目录
    let db = engine::init_db(&temp.path().join("skills-keeper.db")).unwrap();
    let matrix = engine::get_status_matrix(
        &vault,
        &make_registry(&temp.path().join("cc"), &temp.path().join("other")),
        &db,
    )
    .unwrap();
    assert_eq!(matrix.tools.len(), 4);
    assert!(matrix.rows.is_empty());
}

#[test]
fn 工具端完全不存在时全部缺失() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    make_vault(&vault);
    let db = engine::init_db(&temp.path().join("skills-keeper.db")).unwrap();
    // 全部已接入工具根均不存在（覆盖注入，不触碰真实用户目录）
    let registry = AdapterRegistry::with_overrides(HashMap::from([
        (ToolId::ClaudeCode, temp.path().join("cc")),
        (ToolId::Codex, temp.path().join("codex")),
        (ToolId::Trae, temp.path().join("trae")),
    ]));
    let matrix = engine::get_status_matrix(&vault, &registry, &db).unwrap();
    for row in &matrix.rows {
        for (id, status) in &row.statuses {
            assert_eq!(
                *status,
                Status::Missing,
                "{} 应缺失（根不存在）",
                row.skill.skill.slug
            );
            assert!(id != "workbuddy", "未接入列无状态");
        }
    }
}

#[test]
fn 环境变量覆盖路径解析() {
    let _guard = ENV_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let vault_dir = temp.path().join("my-vault");
    let data_dir = temp.path().join("my-data");

    // SKILLS_KEEPER_VAULT 覆盖 Vault 根（data 回落默认 ~/.skills-keeper）
    std::env::set_var("SKILLS_KEEPER_VAULT", &vault_dir);
    std::env::remove_var("SKILLS_KEEPER_DATA");
    let paths = AppPaths::resolve().unwrap();
    assert_eq!(paths.vault_root, vault_dir);
    assert!(paths.data_dir.to_string_lossy().contains(".skills-keeper"));

    // SKILLS_KEEPER_DATA 覆盖数据目录 → Vault 跟随其下 vault/
    std::env::remove_var("SKILLS_KEEPER_VAULT");
    std::env::set_var("SKILLS_KEEPER_DATA", &data_dir);
    let paths = AppPaths::resolve().unwrap();
    assert_eq!(paths.data_dir, data_dir);
    assert_eq!(paths.vault_root, data_dir.join("vault"));

    // 两者同时覆盖：相互独立
    std::env::set_var("SKILLS_KEEPER_VAULT", &vault_dir);
    let paths = AppPaths::resolve().unwrap();
    assert_eq!(paths.vault_root, vault_dir);
    assert_eq!(paths.data_dir, data_dir);

    std::env::remove_var("SKILLS_KEEPER_VAULT");
    std::env::remove_var("SKILLS_KEEPER_DATA");
}

#[test]
fn 相对路径按cwd解析为绝对路径() {
    let _guard = ENV_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let _cwd = CwdGuard(std::env::current_dir().unwrap());
    std::env::set_current_dir(temp.path()).unwrap();

    std::env::set_var("SKILLS_KEEPER_VAULT", "examples/vault");
    std::env::remove_var("SKILLS_KEEPER_DATA");
    let paths = AppPaths::resolve().unwrap();
    // macOS 上 /var → /private/var 是符号链接：cwd 规范化路径与 tempdir 字面路径
    // 不同（/private/var/... vs /var/...），故只断言「绝对 + 基于 cwd」而非精确相等
    assert!(paths.vault_root.is_absolute(), "相对路径应转绝对");
    assert!(
        paths.vault_root.ends_with("examples/vault"),
        "相对路径应基于 cwd 解析：{}",
        paths.vault_root.display()
    );

    std::env::remove_var("SKILLS_KEEPER_VAULT");
    std::env::remove_var("SKILLS_KEEPER_DATA");
}

#[test]
fn 契约序列化形状() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    make_vault(&vault);
    let cc = temp.path().join("cc");
    write(&cc.join("greeting/SKILL.md"), GOOD_MD);
    let other = temp.path().join("other");
    let db = engine::init_db(&temp.path().join("skills-keeper.db")).unwrap();
    let matrix = engine::get_status_matrix(&vault, &make_registry(&cc, &other), &db).unwrap();

    // 前端契约镜像：JSON 形状（tools 数组、rows 数组、statuses 对象、invalid 字符串/null）
    let json = serde_json::to_value(&matrix).unwrap();
    assert!(json["tools"].is_array());
    assert!(json["rows"].is_array());
    // rows[].skill = SkillEntry（含嵌套 skill + invalid）
    let first = &json["rows"][0];
    assert!(first["skill"]["skill"]["slug"].is_string());
    assert!(first["skill"]["invalid"].is_null() || first["skill"]["invalid"].is_string());
    assert!(first["statuses"].as_object().is_some());
    // statuses 值为四态之一
    let statuses: HashMap<String, String> =
        serde_json::from_value(first["statuses"].clone()).unwrap();
    for v in statuses.values() {
        assert!(
            ["consistent", "pending", "modified", "missing"].contains(&v.as_str()),
            "状态值应属四态：{v}"
        );
    }
}

/// 仓库内置样例 Vault（examples/vault，`docs/specs/s1-matrix.md` §验收演示）：
/// 保证样例不腐化——4 个 Skill 覆盖正常 / 中文名 / 含资源文件 / invalid 分支。
#[test]
fn 样例vault读取正常_覆盖展示分支() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri 的父目录即仓库根");
    let vault = repo_root.join("examples/vault");
    assert!(vault.exists(), "样例 Vault 应存在：{vault:?}");

    let entries = engine::list_skills(&vault, &AdapterRegistry::new()).unwrap();
    let slugs: Vec<&str> = entries.iter().map(|e| e.skill.slug.as_str()).collect();
    assert_eq!(
        slugs,
        vec!["broken-skill", "code-assistant", "greeting", "中文技能"],
        "样例 Vault 四 Skill 按 slug 排序"
    );

    // 正常 Skill：无 invalid；中文名；含资源文件
    for slug in ["greeting", "中文技能", "code-assistant"] {
        let entry = entries.iter().find(|e| e.skill.slug == slug).unwrap();
        assert_eq!(entry.invalid, None, "{slug} 应合规");
        assert!(!entry.skill.name.is_empty());
    }
    let greeting = entries.iter().find(|e| e.skill.slug == "greeting").unwrap();
    assert_eq!(
        greeting.skill.version.as_deref(),
        Some("1.0"),
        "YAML 标量 1.0 转字符串原文"
    );
    let zh = entries.iter().find(|e| e.skill.slug == "中文技能").unwrap();
    assert_eq!(zh.skill.sidecar.source.as_deref(), Some("codex"));

    // invalid 分支：缺 name
    let broken = entries
        .iter()
        .find(|e| e.skill.slug == "broken-skill")
        .unwrap();
    assert!(broken.invalid.as_deref().unwrap().contains("name"));
}

// ===== S2 分发事务集成测试（#33，规格 §Implementation Decisions 2）=====

/// S2 分发夹具：tempdir + 样例 Vault（greeting 合规 / broken 缺 name）+ 注册表
/// （cc / codex / trae 指向模拟目录，workbuddy 未接入）+ db + 快照根。
/// 工具端根目录不预创建（缺失 = 首次分发场景）。
struct DeployFixture {
    temp: TempDir,
    vault: PathBuf,
    cc: PathBuf,
    codex: PathBuf,
    registry: AdapterRegistry,
    db: skills_keeper_lib::db::Db,
    snaps: PathBuf,
}

impl DeployFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let vault = temp.path().join("vault");
        make_vault(&vault);
        let cc = temp.path().join("cc");
        let codex = temp.path().join("codex");
        let trae = temp.path().join("trae");
        let registry = AdapterRegistry::with_overrides(HashMap::from([
            (ToolId::ClaudeCode, cc.clone()),
            (ToolId::Codex, codex.clone()),
            (ToolId::Trae, trae),
        ]));
        let db = engine::init_db(&temp.path().join("skills-keeper.db")).unwrap();
        let snaps = temp.path().join("snapshots");
        Self {
            temp,
            vault,
            cc,
            codex,
            registry,
            db,
            snaps,
        }
    }

    fn deploy(
        &self,
        tool: ToolId,
        slugs: &[&str],
    ) -> skills_keeper_lib::engine::error::EngineResult<engine::DeployResult> {
        let slugs: Vec<String> = slugs.iter().map(|s| s.to_string()).collect();
        engine::deploy_tool(
            &self.vault,
            &self.registry,
            &self.snaps,
            tool,
            &slugs,
            &self.db,
        )
    }

    fn matrix_status(&self, slug: &str, tool: &str) -> Status {
        let matrix = engine::get_status_matrix(&self.vault, &self.registry, &self.db).unwrap();
        matrix
            .rows
            .iter()
            .find(|r| r.skill.skill.slug == slug)
            .unwrap()
            .statuses[tool]
    }

    fn snapshot_count(&self) -> i64 {
        self.db
            .lock()
            .query_row("SELECT count(*) FROM snapshots", [], |r| r.get(0))
            .unwrap()
    }
}

#[test]
fn 分发成功_渲染产物_记录快照_矩阵一致() {
    let fx = DeployFixture::new();
    let result = fx.deploy(ToolId::ClaudeCode, &["greeting"]).unwrap();
    assert_eq!(result.ok.len(), 1);
    assert!(result.failed.is_empty());

    // 渲染产物：name 以 slug 覆写（通用注入）
    let md = fs::read_to_string(fx.cc.join("greeting/SKILL.md")).unwrap();
    assert!(md.contains("name: greeting"), "name 应覆写为 slug：{md}");

    // deploy_records：v = 渲染产物 hash、r = 落盘实际 hash（同内容 → 相等）
    let (v, r): (String, String) = fx
        .db
        .lock()
        .query_row(
            "SELECT vault_hash, tool_hash FROM deploy_records
             WHERE tool_id = 'claude-code' AND skill_slug = 'greeting'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(v, r, "渲染产物 hash 与落盘 hash 应一致（判定基准变更）");

    // 矩阵判定「一致」（S2 验收：分发后状态变一致）
    assert_eq!(
        fx.matrix_status("greeting", "claude-code"),
        Status::Consistent
    );

    // 自动快照：表行 + 目录（首次分发工具端为空 → 快照记录「分发前状态」空目录）
    assert_eq!(fx.snapshot_count(), 1);
    assert!(fx.snaps.join("1").is_dir(), "快照目录存在（空快照）");

    // staging 已清理（成功路径）
    assert!(!fx.temp.path().join(".skills-keeper-staging").exists());
}

#[test]
fn 缺name的invalid_渲染以slug覆写_分发成功() {
    let fx = DeployFixture::new();
    let result = fx.deploy(ToolId::ClaudeCode, &["broken"]).unwrap();
    assert_eq!(result.ok.len(), 1, "缺 name 由 render 以 slug 覆写修复");
    let md = fs::read_to_string(fx.cc.join("broken/SKILL.md")).unwrap();
    assert!(md.contains("name: broken"), "name 应为 slug：{md}");
    assert_eq!(
        fx.matrix_status("broken", "claude-code"),
        Status::Consistent
    );
}

#[test]
fn 重扫中止_被工具修改_范围外也中止() {
    let fx = DeployFixture::new();
    // 工具端 greeting 被外部修改（与 Vault 内容不同 → t != v、无记录 → Modified）
    write(
        &fx.cc.join("greeting/SKILL.md"),
        "---\nname: 外部修改\ndescription: x\n---\n",
    );
    // 分发集不含 greeting（Vault 无 other）→ 范围外被修改同样中止（保护整体一致性）
    let err = fx.deploy(ToolId::ClaudeCode, &["other"]).unwrap_err();
    assert!(
        matches!(err, EngineError::InvalidState(_)),
        "重扫中止应为 InvalidState"
    );
    assert!(
        err.to_string().contains("greeting"),
        "中止提示应含被修改清单"
    );
    // 未快照、未落盘
    assert_eq!(fx.snapshot_count(), 0);
    assert!(!fx.cc.join("other").exists());
}

#[test]
fn 缺失不中止_工具端根不存在_正常分发创建() {
    let fx = DeployFixture::new();
    let result = fx.deploy(ToolId::ClaudeCode, &["greeting"]).unwrap();
    assert_eq!(result.ok.len(), 1, "工具端缺失不中止（防呆：重建恢复）");
    assert!(fx.cc.join("greeting/SKILL.md").exists(), "工具端根被创建");
}

#[test]
fn 部分成功_invalid拦截入failed_合规入ok() {
    let fx = DeployFixture::new();
    // SKILL.md 缺失的 Skill：render 后缺 description → validate 拦截 → InvalidSkill
    write(&fx.vault.join("skills/no-md/.skill-meta.json"), "{}");
    let result = fx
        .deploy(ToolId::ClaudeCode, &["greeting", "no-md", "broken"])
        .unwrap();
    let ok_slugs: Vec<&str> = result.ok.iter().map(|i| i.skill_slug.as_str()).collect();
    assert_eq!(ok_slugs, vec!["greeting", "broken"], "合规 + 修复项入 ok");
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].skill_slug, "no-md");
    assert_eq!(result.failed[0].code, "InvalidSkill");
    assert!(!result.failed[0].message.is_empty(), "failed 应含中文原因");
    assert!(!fx.cc.join("no-md").exists(), "failed 项不落盘");
}

#[test]
fn 未接入工具_分发级config错误() {
    let fx = DeployFixture::new();
    let err = fx.deploy(ToolId::Workbuddy, &["greeting"]).unwrap_err();
    assert!(
        matches!(err, EngineError::Config(_)),
        "未接入分发应为 Config"
    );
}

#[test]
fn 快照失败_分发级io中止_不落盘() {
    let fx = DeployFixture::new();
    fs::write(&fx.snaps, "占用").unwrap(); // 快照根是文件 → 无法建子目录
    let err = fx.deploy(ToolId::ClaudeCode, &["greeting"]).unwrap_err();
    assert!(matches!(err, EngineError::Io(_)), "快照失败应为 Io");
    assert!(!fx.cc.join("greeting").exists(), "分发级失败不落盘");
}

#[test]
fn 幂等重试_重复分发覆盖最新() {
    let fx = DeployFixture::new();
    fx.deploy(ToolId::ClaudeCode, &["greeting"]).unwrap();
    // Vault 更新 → 待分发 → 重试（幂等覆盖）
    write(
        &fx.vault.join("skills/greeting/SKILL.md"),
        "---\nname: 问候助手\ndescription: 更新后的描述\nversion: 1.1\n---\n",
    );
    let result = fx.deploy(ToolId::ClaudeCode, &["greeting"]).unwrap();
    assert_eq!(result.ok.len(), 1, "重试应成功（幂等）");
    let md = fs::read_to_string(fx.cc.join("greeting/SKILL.md")).unwrap();
    assert!(md.contains("更新后的描述"), "工具端应为最新渲染产物");
    assert_eq!(
        fx.matrix_status("greeting", "claude-code"),
        Status::Consistent
    );
}

#[test]
fn 覆盖已有目录_两阶段备份无残留() {
    let fx = DeployFixture::new();
    // 首次分发（工具端为空）
    fx.deploy(ToolId::ClaudeCode, &["greeting"]).unwrap();
    // 工具端外部写入元数据（隐藏文件不参与 hash 判定 → 不触发重扫中止）
    write(
        &fx.cc.join("greeting/.skill-meta.json"),
        r#"{"external": true}"#,
    );
    // Vault 更新 → 再分发：工具端 = 旧渲染产物（Pending 不中止）→ 两阶段备份覆盖
    write(
        &fx.vault.join("skills/greeting/SKILL.md"),
        "---\nname: 问候助手\ndescription: 更新后的描述\nversion: 1.1\n---\n",
    );
    let result = fx.deploy(ToolId::ClaudeCode, &["greeting"]).unwrap();
    assert_eq!(result.ok.len(), 1);
    let md = fs::read_to_string(fx.cc.join("greeting/SKILL.md")).unwrap();
    assert!(md.contains("更新后的描述"), "目标应为最新渲染产物");
    // 第二次快照全量复制工具端（含外部写入的隐藏文件——回滚需完整恢复原样）
    assert_eq!(fx.snapshot_count(), 2);
    assert!(
        fx.snaps.join("2/greeting/.skill-meta.json").exists(),
        "快照应含隐藏文件"
    );
    // 新落盘取代旧目录（外部元数据不残留）
    assert!(!fx.cc.join("greeting/.skill-meta.json").exists());
    assert!(
        !fx.temp.path().join(".skills-keeper-staging").exists(),
        "备份位与 staging 应清理无残留"
    );
}

#[test]
fn codex双目录_旧版跟随写入_不存在则跳过() {
    let _guard = ENV_LOCK.lock().unwrap();
    let fx = DeployFixture::new();
    let legacy = fx.temp.path().join("codex-legacy");
    fs::create_dir_all(&legacy).unwrap();
    std::env::set_var("SKILLS_KEEPER_CODEX_LEGACY", &legacy);

    // 旧版存在 → 同一渲染产物写两份
    let result = fx.deploy(ToolId::Codex, &["greeting"]).unwrap();
    assert_eq!(result.ok.len(), 1);
    assert!(
        fx.codex.join("greeting/SKILL.md").exists(),
        "新版目录应分发"
    );
    assert!(
        legacy.join("greeting/SKILL.md").exists(),
        "旧版目录应跟随写入"
    );
    assert_eq!(
        fs::read_to_string(fx.codex.join("greeting/SKILL.md")).unwrap(),
        fs::read_to_string(legacy.join("greeting/SKILL.md")).unwrap(),
        "两份内容一致"
    );

    // 旧版不存在 → 跳过（无告警）
    std::env::remove_var("SKILLS_KEEPER_CODEX_LEGACY");
    let legacy2 = fx.temp.path().join("codex-legacy-none");
    std::env::set_var("SKILLS_KEEPER_CODEX_LEGACY", &legacy2);
    let result = fx.deploy(ToolId::Codex, &["greeting"]).unwrap();
    assert_eq!(result.ok.len(), 1);
    assert!(result.failed.is_empty(), "旧版目录不存在 → 跳过不告警");
    std::env::remove_var("SKILLS_KEEPER_CODEX_LEGACY");
}

#[test]
fn codex旧版失败_告警入failed_主分发成功() {
    let _guard = ENV_LOCK.lock().unwrap();
    let fx = DeployFixture::new();
    // 旧版指向文件（存在但不是目录 → 写入失败）
    let legacy_file = fx.temp.path().join("legacy-file");
    fs::write(&legacy_file, "占用").unwrap();
    std::env::set_var("SKILLS_KEEPER_CODEX_LEGACY", &legacy_file);

    let result = fx.deploy(ToolId::Codex, &["greeting"]).unwrap();
    std::env::remove_var("SKILLS_KEEPER_CODEX_LEGACY");
    assert_eq!(result.ok.len(), 1, "主分发成功不受旧版失败影响");
    assert_eq!(result.failed.len(), 1, "告警承载于 failed 结构");
    assert_eq!(result.failed[0].code, "Io");
    assert!(
        result.failed[0].message.contains("旧版"),
        "告警应点名旧版目录：{}",
        result.failed[0].message
    );
    assert!(fx.codex.join("greeting/SKILL.md").exists());
}
