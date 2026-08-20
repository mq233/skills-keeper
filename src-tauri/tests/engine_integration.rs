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
use skills_keeper_lib::engine::status::Status;
use skills_keeper_lib::engine::target::{AdapterRegistry, ToolId};

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

    // 模拟工具端：claude-code 存在且内容与 Vault 一致（无分发记录 → r 缺失 → t==v → 一致）
    let cc = temp.path().join("cc");
    write(&cc.join("greeting/SKILL.md"), GOOD_MD);
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
    write(&cc.join("greeting/SKILL.md"), GOOD_MD);
    let other = temp.path().join("other");

    let db = engine::init_db(&temp.path().join("skills-keeper.db")).unwrap();
    let registry = make_registry(&cc, &other);

    // 插入分发记录：模拟 S2 分发后状态（r = 工具端 hash，v_rec = 分发时 Vault hash）
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
    write(&cc.join("greeting/SKILL.md"), GOOD_MD);
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
