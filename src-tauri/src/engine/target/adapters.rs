//! 四目标工具适配器实现（Claude Code / Codex / WorkBuddy / Trae）。
//!
//! S2 决议「适配器 trait 与 render_skill 范围」：render 两级注入（通用最小集 →
//! 工具特有补丁），行为字段按目标剥离；WorkBuddy 全集预留（未接入，S2 无数据不注入）；
//! validate 只查必败项 → `InvalidSkill`。字段依据见 `docs/research/s2-render-fields.md`。
//!
//! 路径基线（`docs/technical-plan.md` §4.2）：Claude Code `~/.claude/skills/`；
//! Codex 新版 `~/.agents/skills/`（旧 `~/.codex/skills/` 仅兼容跟随写入）；
//! Trae 默认国际版 `~/.trae/skills/`；WorkBuddy 官方未公开路径 → 未接入。

use std::path::PathBuf;

use crate::engine::error::{EngineError, EngineResult};
use crate::engine::target::{expand_tilde, RenderedSkill, ToolAdapter, ToolId};
use crate::engine::vault::{parse_frontmatter, Skill};

/// Claude Code 扩展行为字段（调研 §2.1 的 14 个）：分发到 Codex / Trae / WorkBuddy
/// 目标时剥离——Codex 严格解析（未知/废弃字段归为反模式）、其余工具不消费且会改变
/// 行为语义。Claude Code 目标保留（用户合法行为配置）。
const CLAUDE_CODE_BEHAVIOR_FIELDS: [&str; 14] = [
    "when_to_use",
    "argument-hint",
    "arguments",
    "disable-model-invocation",
    "user-invocable",
    "disallowed-tools",
    "model",
    "effort",
    "context",
    "agent",
    "background",
    "hooks",
    "paths",
    "shell",
];

/// Claude Code 适配器：`~/.claude/skills/`；行为字段全保留（唯一会执行行为字段的工具）。
pub struct ClaudeCodeAdapter;

impl ToolAdapter for ClaudeCodeAdapter {
    fn id(&self) -> ToolId {
        ToolId::ClaudeCode
    }

    fn default_skills_dir(&self) -> Option<PathBuf> {
        expand_tilde("~/.claude/skills")
    }

    fn render_skill(&self, skill: &Skill) -> EngineResult<RenderedSkill> {
        render_common(skill, true)
    }

    fn validate(&self, rendered: &RenderedSkill) -> EngineResult<()> {
        validate_rendered(rendered)
    }
}

/// Codex 适配器：新版 `~/.agents/skills/`（主目录）；旧版 `~/.codex/skills/` 仅兼容
/// 跟随写入（存在才写、失败告警不失败、不参与扫描/状态判定——地图决议）。
pub struct CodexAdapter;

impl CodexAdapter {
    /// 旧版兼容目录（`None` = 主目录解析失败）；分发事务查询，仅目录存在时写入第二份。
    pub fn legacy_skills_dir() -> Option<PathBuf> {
        expand_tilde("~/.codex/skills")
    }
}

impl ToolAdapter for CodexAdapter {
    fn id(&self) -> ToolId {
        ToolId::Codex
    }

    fn default_skills_dir(&self) -> Option<PathBuf> {
        expand_tilde("~/.agents/skills")
    }

    fn render_skill(&self, skill: &Skill) -> EngineResult<RenderedSkill> {
        // Codex 无 SKILL.md 额外注入；`agents/openai.yaml` S2 无数据来源不生成（extra_files 恒空）
        render_common(skill, false)
    }

    fn validate(&self, rendered: &RenderedSkill) -> EngineResult<()> {
        validate_rendered(rendered)
    }
}

/// WorkBuddy 适配器：官方未公开路径 → 未接入（`None`），引擎从不调用（connected 过滤）。
/// render 按调研 §5.4 全集预留注入逻辑（description_zh/en、display_name、allowed-tools、
/// version），「注入即真实数据」——S2 无数据不注入键；S5 设置页接入后启用。
pub struct WorkbuddyAdapter;

impl ToolAdapter for WorkbuddyAdapter {
    fn id(&self) -> ToolId {
        ToolId::Workbuddy
    }

    fn default_skills_dir(&self) -> Option<PathBuf> {
        None
    }

    fn render_skill(&self, skill: &Skill) -> EngineResult<RenderedSkill> {
        // 预留：接入后在此注入 description_zh/description_en/display_name/allowed-tools/version
        //（数据源齐备才注入对应键，无数据不注入——避免空值）
        render_common(skill, false)
    }

    fn validate(&self, rendered: &RenderedSkill) -> EngineResult<()> {
        validate_rendered(rendered)
    }
}

/// Trae 适配器：默认国际版 `~/.trae/skills/`（CN 版 `~/.trae-cn/skills/` 留 S5 设置页选择）；
/// name + description 即满足官方全部要求，无额外注入。
pub struct TraeAdapter;

impl ToolAdapter for TraeAdapter {
    fn id(&self) -> ToolId {
        ToolId::Trae
    }

    fn default_skills_dir(&self) -> Option<PathBuf> {
        expand_tilde("~/.trae/skills")
    }

    fn render_skill(&self, skill: &Skill) -> EngineResult<RenderedSkill> {
        render_common(skill, false)
    }

    fn validate(&self, rendered: &RenderedSkill) -> EngineResult<()> {
        validate_rendered(rendered)
    }
}

/// render 公共实现（纯函数，无文件系统访问）：
/// 通用最小集（name 以 slug 覆写、description/version 保留原 frontmatter、标准字段保留）
/// → 行为字段按目标剥离（`keep_behavior` = Claude Code 保留，其余剥离）。
fn render_common(skill: &Skill, keep_behavior: bool) -> EngineResult<RenderedSkill> {
    // 1. 过滤原 frontmatter 键（保序）；行为字段按模式剥离
    let mut keys: Vec<(String, serde_yaml_ng::Value)> = skill
        .frontmatter
        .iter()
        .filter(|(k, _)| keep_behavior || !CLAUDE_CODE_BEHAVIOR_FIELDS.contains(&k.as_str()))
        .cloned()
        .collect();

    // 2. name 以 slug 覆写：原 name 键原位换值；无 name 键则前置插入（保序惯例 name 在首）
    if let Some((_, v)) = keys.iter_mut().find(|(k, _)| k == "name") {
        *v = serde_yaml_ng::Value::String(skill.slug.clone());
    } else {
        keys.insert(
            0,
            (
                "name".to_string(),
                serde_yaml_ng::Value::String(skill.slug.clone()),
            ),
        );
    }

    // 3. description / version 保留自原 frontmatter（空值不写入——「注入即真实数据」，
    //    缺失由 validate 兜底拦截）；version 有值则覆盖旧键值
    if !skill.description.is_empty() {
        if let Some((_, v)) = keys.iter_mut().find(|(k, _)| k == "description") {
            *v = serde_yaml_ng::Value::String(skill.description.clone());
        } else {
            keys.insert(
                1,
                (
                    "description".to_string(),
                    serde_yaml_ng::Value::String(skill.description.clone()),
                ),
            );
        }
    }
    if let Some(version) = &skill.version {
        if let Some((_, v)) = keys.iter_mut().find(|(k, _)| k == "version") {
            *v = serde_yaml_ng::Value::String(version.clone());
        } else {
            keys.push((
                "version".to_string(),
                serde_yaml_ng::Value::String(version.clone()),
            ));
        }
    }

    // 4. 组装 SKILL.md：frontmatter 块（值经 YAML 序列化，语义保真）+ 正文
    let mut md = String::from("---\n");
    for (k, v) in &keys {
        let value = serde_yaml_ng::to_string(v).unwrap_or_default();
        md.push_str(&format!("{k}: {}\n", value.trim_end()));
    }
    md.push_str("---\n");
    if !skill.body.is_empty() {
        md.push_str(&skill.body);
        if !md.ends_with('\n') {
            md.push('\n');
        }
    }

    Ok(RenderedSkill {
        skill_md: md,
        resources: skill.resources.clone(),
        extra_files: Vec::new(), // S2 恒空：Codex agents/openai.yaml 无数据来源不生成
    })
}

/// validate 公共实现：只查必败项——name/description 非空 + name 字符集
/// （小写字母/数字/连字符、无连续连字符、首尾非连字符）；不合规 → `InvalidSkill`。
fn validate_rendered(rendered: &RenderedSkill) -> EngineResult<()> {
    let fm = parse_frontmatter(&rendered.skill_md).map_err(|reason| {
        EngineError::InvalidSkill(format!("渲染产物 frontmatter 解析失败：{reason}"))
    })?;

    if fm.name.is_empty() {
        return Err(EngineError::InvalidSkill("渲染产物缺少 name".to_string()));
    }
    if fm.description.is_empty() {
        return Err(EngineError::InvalidSkill(
            "渲染产物缺少 description".to_string(),
        ));
    }
    if !is_valid_name(&fm.name) {
        return Err(EngineError::InvalidSkill(format!(
            "name 不符合规范（小写字母/数字/连字符，无连续连字符，首尾非连字符）：{}",
            fm.name
        )));
    }
    Ok(())
}

/// name 字符集校验（开放规范与 Codex 硬约束）。
fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;
    use crate::engine::target::AdapterRegistry;

    /// 构造合规 Skill 夹具（frontmatter 保序含行为字段，用于剥离分支测试）。
    fn make_skill() -> Skill {
        let mut frontmatter = vec![
            (
                "name".to_string(),
                serde_yaml_ng::Value::String("Display 名".to_string()),
            ),
            (
                "description".to_string(),
                serde_yaml_ng::Value::String("生成友好问候语".to_string()),
            ),
            (
                "version".to_string(),
                serde_yaml_ng::Value::String("1.0".to_string()),
            ),
            (
                "license".to_string(),
                serde_yaml_ng::Value::String("MIT".to_string()),
            ),
            (
                "allowed-tools".to_string(),
                serde_yaml_ng::Value::String("Bash,Read".to_string()),
            ),
            (
                "when_to_use".to_string(),
                serde_yaml_ng::Value::String("用户要求问候时".to_string()),
            ),
            (
                "model".to_string(),
                serde_yaml_ng::Value::String("opus".to_string()),
            ),
        ];
        // name 覆写测试：frontmatter 无 name 键的独立用例
        frontmatter.remove(0);
        frontmatter.insert(
            0,
            (
                "name".to_string(),
                serde_yaml_ng::Value::String("old-name".to_string()),
            ),
        );
        Skill {
            slug: "greeting".to_string(),
            name: "Display 名".to_string(),
            description: "生成友好问候语".to_string(),
            version: Some("1.0".to_string()),
            sidecar: crate::engine::vault::Sidecar {
                source: None,
                targets: vec!["claude-code".to_string()],
            },
            body: "# 问候助手\n\n生成问候。\n".to_string(),
            frontmatter,
            resources: vec![],
        }
    }

    /// 解析渲染产物的 frontmatter 值（键 → Value）。
    fn rendered_key(rendered: &RenderedSkill, key: &str) -> Option<serde_yaml_ng::Value> {
        let block = rendered
            .skill_md
            .lines()
            .skip(1)
            .take_while(|l| *l != "---")
            .collect::<Vec<_>>()
            .join("\n");
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&block).unwrap();
        value.get(key).cloned()
    }

    #[test]
    fn 适配器_默认路径_workbuddy未接入() {
        let registry = AdapterRegistry::new();
        let expected = [
            (ToolId::ClaudeCode, ".claude/skills"),
            (ToolId::Codex, ".agents/skills"),
            (ToolId::Workbuddy, ""), // 未接入
            (ToolId::Trae, ".trae/skills"),
        ];
        for (id, tail) in expected {
            let adapter = registry.get(id).unwrap();
            match id {
                ToolId::Workbuddy => {
                    assert_eq!(adapter.default_skills_dir(), None, "WorkBuddy 未接入")
                }
                _ => {
                    let dir = adapter.default_skills_dir().unwrap();
                    let s = dir.to_string_lossy().replace('\\', "/");
                    assert!(s.ends_with(tail), "{id:?} 默认路径应为 ...{tail}，实际 {s}");
                }
            }
        }
        // Codex 旧版兼容目录
        let legacy = CodexAdapter::legacy_skills_dir().unwrap();
        assert!(
            legacy
                .to_string_lossy()
                .replace('\\', "/")
                .ends_with(".codex/skills"),
            "Codex 旧版目录应为 ~/.codex/skills"
        );
    }

    #[test]
    fn 注册表_connected过滤与roots() {
        let registry = AdapterRegistry::new();
        let connected = registry.connected();
        assert_eq!(connected.len(), 3, "WorkBuddy 未接入 → 3 个已接入");
        let ids: Vec<&str> = connected.iter().map(|a| a.id().as_str()).collect();
        assert_eq!(ids, vec!["claude-code", "codex", "trae"]);
        assert_eq!(
            registry.all_connected_targets(),
            vec!["claude-code", "codex", "trae"]
        );
        let roots = registry.tool_roots();
        assert_eq!(roots.len(), 4, "roots 含未接入（None）");
        assert!(roots
            .iter()
            .any(|(id, d)| *id == ToolId::Workbuddy && d.is_none()));
        assert_eq!(
            registry.get(ToolId::Workbuddy).unwrap().id(),
            ToolId::Workbuddy
        );
    }

    #[test]
    fn render_通用注入_name以slug覆写_标准字段保留() {
        let skill = make_skill();
        let registry = AdapterRegistry::new();
        for id in [ToolId::ClaudeCode, ToolId::Codex, ToolId::Trae] {
            let rendered = registry.get(id).unwrap().render_skill(&skill).unwrap();
            let fm = parse_frontmatter(&rendered.skill_md).unwrap();
            assert_eq!(fm.name, "greeting", "{id:?} name 应覆写为 slug");
            assert_eq!(fm.description, "生成友好问候语");
            assert_eq!(fm.version.as_deref(), Some("1.0"));
            assert_eq!(
                rendered_key(&rendered, "license"),
                Some(serde_yaml_ng::Value::String("MIT".to_string())),
                "标准字段 license 应保留"
            );
            assert_eq!(
                rendered_key(&rendered, "allowed-tools"),
                Some(serde_yaml_ng::Value::String("Bash,Read".to_string())),
                "标准字段 allowed-tools 应保留"
            );
            assert!(rendered.skill_md.contains("# 问候助手"), "正文应保留");
            assert!(
                rendered.skill_md.starts_with("---\n"),
                "frontmatter 块应完整"
            );
        }
    }

    #[test]
    fn render_行为字段_ClaudeCode保留_其余剥离() {
        let skill = make_skill();
        let registry = AdapterRegistry::new();
        let cc = registry
            .get(ToolId::ClaudeCode)
            .unwrap()
            .render_skill(&skill)
            .unwrap();
        assert_eq!(
            rendered_key(&cc, "when_to_use"),
            Some(serde_yaml_ng::Value::String("用户要求问候时".to_string())),
            "Claude Code 目标应保留行为字段"
        );
        assert_eq!(
            rendered_key(&cc, "model"),
            Some(serde_yaml_ng::Value::String("opus".to_string()))
        );
        for id in [ToolId::Codex, ToolId::Trae, ToolId::Workbuddy] {
            let rendered = registry.get(id).unwrap().render_skill(&skill).unwrap();
            assert_eq!(
                rendered_key(&rendered, "when_to_use"),
                None,
                "{id:?} 应剥离行为字段"
            );
            assert_eq!(
                rendered_key(&rendered, "model"),
                None,
                "{id:?} 应剥离行为字段"
            );
        }
    }

    #[test]
    fn render_workbuddy_预留无数据不注入() {
        let skill = make_skill();
        let registry = AdapterRegistry::new();
        let wb = registry
            .get(ToolId::Workbuddy)
            .unwrap()
            .render_skill(&skill)
            .unwrap();
        // allowed-tools 是标准字段（原 frontmatter 保留）
        assert_eq!(
            rendered_key(&wb, "allowed-tools").unwrap(),
            serde_yaml_ng::Value::String("Bash,Read".to_string())
        );
        // S2 无数据源：WorkBuddy 特有扩展键不注入（注入即真实数据）
        for key in ["description_zh", "description_en", "display_name"] {
            assert_eq!(rendered_key(&wb, key), None, "{key} 无数据不注入");
        }
    }

    #[test]
    fn render_frontmatter保序() {
        let skill = make_skill();
        let registry = AdapterRegistry::new();
        let rendered = registry
            .get(ToolId::Trae)
            .unwrap()
            .render_skill(&skill)
            .unwrap();
        let lines: Vec<&str> = rendered
            .skill_md
            .lines()
            .skip(1)
            .take_while(|l| *l != "---")
            .collect();
        // 原键顺序保留：name（覆写值）、description、version、license、allowed-tools（行为字段已剥离）
        let keys: Vec<&str> = lines.iter().map(|l| l.split(':').next().unwrap()).collect();
        assert_eq!(
            keys,
            vec!["name", "description", "version", "license", "allowed-tools"],
            "保序输出且行为字段剥离"
        );
    }

    #[test]
    fn validate_合规通过() {
        let skill = make_skill();
        let registry = AdapterRegistry::new();
        for id in [ToolId::ClaudeCode, ToolId::Codex, ToolId::Trae] {
            let rendered = registry.get(id).unwrap().render_skill(&skill).unwrap();
            registry.get(id).unwrap().validate(&rendered).unwrap();
        }
    }

    #[test]
    fn validate_必败项拦截() {
        let registry = AdapterRegistry::new();
        let adapter = registry.get(ToolId::Codex).unwrap();

        // 缺 description（Vault invalid 数据面兜底）
        let mut skill = make_skill();
        skill.description = String::new();
        skill.frontmatter.retain(|(k, _)| k != "description");
        let rendered = adapter.render_skill(&skill).unwrap();
        let err = adapter.validate(&rendered).unwrap_err();
        assert!(
            matches!(err, EngineError::InvalidSkill(_)),
            "缺 description → InvalidSkill"
        );
        assert!(err.to_string().contains("description"));

        // name 字符集：大写字母
        let mut skill = make_skill();
        skill.slug = "Bad-Name".to_string();
        let rendered = adapter.render_skill(&skill).unwrap();
        let err = adapter.validate(&rendered).unwrap_err();
        assert!(matches!(err, EngineError::InvalidSkill(_)));
        assert!(err.to_string().contains("name"));

        // name 字符集：连续连字符 / 首尾连字符
        for bad in ["a--b", "-ab", "ab-"] {
            let mut skill = make_skill();
            skill.slug = bad.to_string();
            let rendered = adapter.render_skill(&skill).unwrap();
            assert!(adapter.validate(&rendered).is_err(), "{bad} 应不合规");
        }

        // 合规名通过
        for good in ["abc", "a-b-c", "skill123", "x"] {
            let mut skill = make_skill();
            skill.slug = good.to_string();
            let rendered = adapter.render_skill(&skill).unwrap();
            assert!(adapter.validate(&rendered).is_ok(), "{good} 应合规");
        }
    }
}
