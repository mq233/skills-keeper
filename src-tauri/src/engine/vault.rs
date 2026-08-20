//! Vault 读取：清单、frontmatter 解析、Sidecar 容错读写。
//!
//! S1 实现（`docs/specs/s1-matrix.md` §2）：
//! - `invalid` 是数据不是错误：读取返回 `SkillEntry { skill, invalid: Option<String> }`，
//!   SKILL.md 缺失、frontmatter 解析失败、缺 name/description、Sidecar JSON 损坏 → invalid + 原因；
//!   仅 Io 级错误才传播错误
//! - Sidecar 缺失 → 容错合成默认（source = null、targets = 全部已接入工具）

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::error::{EngineError, EngineResult};

/// Skill 实体（slug = 目录名；S2 扩展 render 输入，序列化契约不含扩展字段——
/// `#[serde(skip)]` 保持 S1 前端契约不变）。
#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub sidecar: Sidecar,
    /// SKILL.md 正文（frontmatter 块之后；S2 render 重写时拼接）
    #[serde(skip)]
    pub body: String,
    /// frontmatter 原键保序保留（S2 render 注入/剥离的输入，解析不丢原键）
    #[serde(skip)]
    pub frontmatter: Vec<(String, serde_yaml_ng::Value)>,
    /// 资源文件相对清单（S2 分发复制；排除 SKILL.md 与隐藏文件，与扫描口径一致）
    #[serde(skip)]
    pub resources: Vec<PathBuf>,
}

/// Sidecar（伴生元数据）S1 只读子集：source / targets（schemaVersion 1，见技术规划 §3.3）。
#[derive(Debug, Clone, Serialize)]
pub struct Sidecar {
    /// 导入来源工具 id；本应用新建为 null
    pub source: Option<String>,
    /// 分发目标标记；新建默认全部已接入工具
    pub targets: Vec<String>,
}

/// Vault 读取结果：`invalid` 携带行级标记原因（中文文案），skill 为容错值仍可展示。
#[derive(Debug, Clone, Serialize)]
pub struct SkillEntry {
    pub skill: Skill,
    pub invalid: Option<String>,
}

/// frontmatter 解析结果（S2 扩展：raw 原键保序，render 注入/剥离的输入）。
pub(crate) struct Frontmatter {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) version: Option<String>,
    pub(crate) raw: Vec<(String, serde_yaml_ng::Value)>,
}

/// 读取 Vault 中全部 Skill（按 slug 排序）；`<vault>/skills/` 不存在时为空清单。
/// `connected_targets` = 已接入目标工具 id（Sidecar 缺失时合成默认 targets）。
pub fn list_skills(
    vault_root: &Path,
    connected_targets: &[String],
) -> EngineResult<Vec<SkillEntry>> {
    let skills_root = vault_root.join("skills");
    if !skills_root.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(&skills_root).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        if !entry.file_type().map_err(io_err)?.is_dir() {
            continue; // 根下散文件忽略
        }
        entries.push(read_skill(&entry.path(), connected_targets)?);
    }
    entries.sort_by(|a, b| a.skill.slug.cmp(&b.skill.slug));
    Ok(entries)
}

/// 读取单个 Skill 目录：SKILL.md 缺失 / frontmatter 解析失败 / 缺 name/description /
/// Sidecar 损坏 → `invalid` + 原因；仅 Io 级错误传播。
pub fn read_skill(skill_dir: &Path, connected_targets: &[String]) -> EngineResult<SkillEntry> {
    let slug = skill_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut name = String::new();
    let mut description = String::new();
    let mut version = None;
    let mut frontmatter: Vec<(String, serde_yaml_ng::Value)> = Vec::new();
    let mut body = String::new();
    let mut reasons: Vec<String> = Vec::new();

    let skill_md = skill_dir.join("SKILL.md");
    if skill_md.exists() {
        let content = fs::read_to_string(&skill_md).map_err(io_err)?;
        match parse_frontmatter(&content) {
            Ok(fm) => {
                name = fm.name;
                description = fm.description;
                version = fm.version;
                frontmatter = fm.raw;
                body = split_body(&content);
                match (name.is_empty(), description.is_empty()) {
                    (true, true) => reasons.push("缺少 name 与 description".to_string()),
                    (true, false) => reasons.push("缺少 name".to_string()),
                    (false, true) => reasons.push("缺少 description".to_string()),
                    (false, false) => {}
                }
            }
            Err(reason) => reasons.push(format!("frontmatter 解析失败：{reason}")),
        }
    } else {
        reasons.push("SKILL.md 缺失".to_string());
    }

    let (sidecar, sidecar_reason) = read_sidecar(skill_dir, connected_targets)?;
    if let Some(reason) = sidecar_reason {
        reasons.push(reason);
    }

    let resources = collect_resources(skill_dir).map_err(io_err)?;

    Ok(SkillEntry {
        skill: Skill {
            slug,
            name,
            description,
            version,
            sidecar,
            body,
            frontmatter,
            resources,
        },
        invalid: if reasons.is_empty() {
            None
        } else {
            Some(reasons.join("；"))
        },
    })
}

/// 解析 SKILL.md frontmatter：首尾 `---` 分隔行扫描 + serde_yaml_ng `Value` 提取
/// name/description/version + 原键保序保留（raw，render 注入/剥离输入；适配器复用）。
/// 返回 Err(原因) = 无 frontmatter 块或 YAML 解析失败（属数据问题，不是 Io 错误）。
pub(crate) fn parse_frontmatter(content: &str) -> Result<Frontmatter, String> {
    let block =
        extract_frontmatter_block(content).ok_or("缺少 frontmatter 块（首尾 `---` 分隔行）")?;
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&block).map_err(|e| format!("YAML 解析失败：{e}"))?;
    let map = value.as_mapping().ok_or("frontmatter 顶层不是映射")?;
    // 原键保序（serde_yaml_ng Mapping 为 preserve_order 语义）
    let raw = map
        .iter()
        .map(|(k, v)| (k.as_str().unwrap_or_default().to_string(), v.clone()))
        .collect();
    Ok(Frontmatter {
        name: scalar_to_string(map.get(serde_yaml_ng::Value::String("name".to_string())))
            .unwrap_or_default(),
        description: scalar_to_string(
            map.get(serde_yaml_ng::Value::String("description".to_string())),
        )
        .unwrap_or_default(),
        version: scalar_to_string(map.get(serde_yaml_ng::Value::String("version".to_string()))),
        raw,
    })
}

/// 提取首尾 `---` 之间的 frontmatter 块内容（无块时返回 None）。
fn extract_frontmatter_block(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.iter().position(|l| l.trim() == "---")?;
    let end = (start + 1..lines.len()).find(|&i| lines[i].trim() == "---")?;
    Some(lines[start + 1..end].join("\n"))
}

/// frontmatter 块之后的正文（第二个 `---` 行后剩余内容，去掉首部空行）。
fn split_body(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.iter().position(|l| l.trim() == "---").unwrap_or(0);
    let end = (start + 1..lines.len())
        .find(|&i| lines[i].trim() == "---")
        .unwrap_or(lines.len());
    let body = lines[end + 1..].join("\n");
    let trimmed = body.trim_start();
    if trimmed.is_empty() {
        String::new()
    } else {
        trimmed.to_string()
    }
}

/// 资源文件清单：Skill 目录内相对路径（递归），排除 SKILL.md 与隐藏文件
/// （与扫描/分发口径一致：`.skill-meta.json` 等元数据不复制）。Io 错误传播。
fn collect_resources(skill_dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    fn walk(dir: &Path, prefix: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "SKILL.md" || name.starts_with('.') {
                continue; // 排除 SKILL.md 与隐藏元数据文件
            }
            let rel = prefix.join(&name);
            if entry.file_type()?.is_dir() {
                walk(&entry.path(), &rel, out)?;
            } else {
                out.push(rel);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(skill_dir, Path::new(""), &mut out)?;
    Ok(out)
}

/// 标量统一转字符串原文（规避 YAML 1.1 标量推断陷阱，如 `version: 1.0` → "1.0"）；
/// 缺失 / Null / 非标量（映射、序列）→ None。
fn scalar_to_string(v: Option<&serde_yaml_ng::Value>) -> Option<String> {
    match v {
        None | Some(serde_yaml_ng::Value::Null) => None,
        Some(serde_yaml_ng::Value::String(s)) => Some(s.clone()),
        Some(serde_yaml_ng::Value::Bool(b)) => Some(b.to_string()),
        Some(serde_yaml_ng::Value::Number(n)) => Some(number_to_string(n)),
        Some(_) => None,
    }
}

/// 数值转字符串：整数直接；浮点整数（如 1.0）保留 `.0` 后缀还原 YAML 原文观感。
fn number_to_string(n: &serde_yaml_ng::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    if let Some(f) = n.as_f64() {
        if f.is_finite() && f.fract() == 0.0 {
            return format!("{f:.1}");
        }
        return f.to_string();
    }
    String::new()
}

/// Sidecar 文件结构（schemaVersion 1；S1 只读 source/targets，其余字段忽略）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SidecarFile {
    source: Option<String>,
    targets: Option<Vec<String>>,
}

/// 读 Sidecar：缺失 → 容错合成默认（source = null、targets = 传入的已接入工具）；
/// JSON 损坏 → 合成默认 + invalid 原因（不静默丢弃）。已接入列表由调用方注入
/// （S2 适配器化：接入判定收敛到 AdapterRegistry，vault 不感知具体工具）。
fn read_sidecar(
    skill_dir: &Path,
    connected_targets: &[String],
) -> EngineResult<(Sidecar, Option<String>)> {
    let path = skill_dir.join(".skill-meta.json");
    if !path.exists() {
        return Ok((default_sidecar(connected_targets), None));
    }
    let content = fs::read_to_string(&path).map_err(io_err)?;
    match serde_json::from_str::<SidecarFile>(&content) {
        Ok(sf) => Ok((
            Sidecar {
                source: sf.source,
                // targets 是必填字段：字段缺失同样按容错合成默认（与 Sidecar 缺失口径一致）
                targets: sf.targets.unwrap_or_else(|| connected_targets.to_vec()),
            },
            None,
        )),
        Err(e) => Ok((
            default_sidecar(connected_targets),
            Some(format!("Sidecar 损坏：{e}")),
        )),
    }
}

/// Sidecar 缺失 / 损坏时的容错默认：source = null、targets = 全部已接入工具。
fn default_sidecar(connected_targets: &[String]) -> Sidecar {
    Sidecar {
        source: None,
        targets: connected_targets.to_vec(),
    }
}

fn io_err(e: std::io::Error) -> EngineError {
    EngineError::Io(format!("读取文件失败：{e}"))
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;
    use tempfile::TempDir;

    /// 已接入目标工具（S2 适配器化后由注册表注入；测试与 S1 口径一致：workbuddy 断开）。
    fn connected() -> Vec<String> {
        vec![
            "claude-code".to_string(),
            "codex".to_string(),
            "trae".to_string(),
        ]
    }

    /// 在临时目录建 `<root>/skills/<slug>/` 并返回 root。
    fn make_vault() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("skills")).unwrap();
        dir
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    const GOOD_SKILL_MD: &str =
        "---\nname: 问候助手\ndescription: 生成友好问候语\nversion: 1.0\n---\n# 内容\n";

    #[test]
    fn frontmatter_正常解析三字段() {
        let fm = parse_frontmatter(GOOD_SKILL_MD).unwrap();
        assert_eq!(fm.name, "问候助手");
        assert_eq!(fm.description, "生成友好问候语");
        assert_eq!(fm.version.as_deref(), Some("1.0"), "数值标量应转字符串原文");
    }

    #[test]
    fn frontmatter_缺name_description_或version() {
        let fm = parse_frontmatter("---\ndescription: d\n---\n").unwrap();
        assert_eq!(fm.name, "");
        let fm = parse_frontmatter("---\nname: n\nversion: \"2\"\n---\n").unwrap();
        assert_eq!(fm.description, "");
        assert_eq!(fm.version.as_deref(), Some("2"), "引号字符串原样");
        let fm = parse_frontmatter("---\nname: n\ndescription: d\n---\n").unwrap();
        assert_eq!(fm.version, None, "无 version 应为 None");
    }

    #[test]
    fn frontmatter_yaml11标量陷阱() {
        // 数字有 int/float 之分、true/false 是布尔——一律转字符串原文
        // （libyaml 对 yes/no/on/off 按字符串解析，原样保留，反而安全）
        for (yaml, expected) in [
            ("version: 1.0", "1.0"),
            ("version: 1", "1"),
            ("version: 3.14", "3.14"),
            ("version: 1000", "1000"),
            ("version: no", "no"),
            ("version: true", "true"),
            ("name: 123", "123"),
        ] {
            // 夹具模板自带 name/description；用例行若带 name 键则跳过预置
            let body = if yaml.starts_with("name:") {
                format!("description: d\n{yaml}")
            } else {
                format!("name: n\ndescription: d\n{yaml}")
            };
            let content = format!("---\n{body}\n---\n");
            let fm = parse_frontmatter(&content).unwrap_or_else(|e| panic!("{yaml} 应可解析：{e}"));
            let got = fm.version.unwrap_or(fm.name);
            assert_eq!(got, expected, "{yaml} → {expected}");
        }
    }

    #[test]
    fn frontmatter_未知字段忽略_注释容忍() {
        let content = "---\nname: n\ndescription: d\nallowed-tools: [foo]\n# 注释\n---\n";
        let fm = parse_frontmatter(content).unwrap();
        assert_eq!(fm.name, "n");
    }

    #[test]
    fn frontmatter_块标量描述() {
        let content = "---\nname: n\ndescription: |\n  多行\n  描述\n---\n";
        let fm = parse_frontmatter(content).unwrap();
        assert_eq!(fm.description, "多行\n描述");
    }

    #[test]
    fn frontmatter_解析失败分支() {
        // 无 frontmatter 块
        assert!(parse_frontmatter("# 只有正文\n").is_err());
        assert!(parse_frontmatter("").is_err());
        assert!(parse_frontmatter("---\n只有开头\n").is_err());
        // 非法 YAML
        assert!(parse_frontmatter("---\nname: [未闭合\n---\n").is_err());
        // 顶层非映射
        assert!(parse_frontmatter("---\n- a\n- b\n---\n").is_err());
    }

    #[test]
    fn 读取skill_正常_含sidecar() {
        let dir = make_vault();
        let slug = "greeting";
        let skill_dir = dir.path().join("skills").join(slug);
        write(&skill_dir.join("SKILL.md"), GOOD_SKILL_MD);
        write(
            &skill_dir.join(".skill-meta.json"),
            r#"{"schemaVersion": 1, "source": "codex", "targets": ["codex", "trae"], "createdAt": "2026-08-01T00:00:00Z"}"#,
        );
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        assert_eq!(entry.invalid, None);
        assert_eq!(entry.skill.slug, "greeting");
        assert_eq!(entry.skill.name, "问候助手");
        assert_eq!(entry.skill.sidecar.source.as_deref(), Some("codex"));
        assert_eq!(entry.skill.sidecar.targets, vec!["codex", "trae"]);
    }

    #[test]
    fn 读取skill_缺name标记invalid() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("no-name");
        write(
            &skill_dir.join("SKILL.md"),
            "---\ndescription: 只有描述\n---\n",
        );
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        assert!(entry.invalid.is_some());
        assert!(
            entry.invalid.as_deref().unwrap().contains("name"),
            "原因应点名缺 name"
        );
        assert!(entry.skill.name.is_empty(), "缺 name 时占位空串仍可展示");
    }

    #[test]
    fn 读取skill_缺skill_md标记invalid() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("no-md");
        write(&skill_dir.join(".skill-meta.json"), "{}");
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        assert!(entry.invalid.as_deref().unwrap().contains("SKILL.md 缺失"));
    }

    #[test]
    fn 读取skill_frontmatter解析失败标记invalid() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("bad-fm");
        write(&skill_dir.join("SKILL.md"), "---\nname: [未闭合\n---\n");
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        assert!(entry
            .invalid
            .as_deref()
            .unwrap()
            .contains("frontmatter 解析失败"));
    }

    #[test]
    fn 读取skill_sidecar缺失合成默认() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("no-sidecar");
        write(&skill_dir.join("SKILL.md"), GOOD_SKILL_MD);
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        assert_eq!(entry.invalid, None, "Sidecar 缺失不是 invalid");
        assert_eq!(entry.skill.sidecar.source, None);
        // 全部已接入工具（S1：workbuddy 未接入 → 三个）
        assert_eq!(
            entry.skill.sidecar.targets,
            vec!["claude-code", "codex", "trae"]
        );
    }

    #[test]
    fn 读取skill_sidecar存在但缺targets字段_同样合成默认() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("no-targets");
        write(&skill_dir.join("SKILL.md"), GOOD_SKILL_MD);
        // targets 是必填字段，缺失时按容错口径合成默认（与 Sidecar 缺失一致）
        write(
            &skill_dir.join(".skill-meta.json"),
            r#"{"schemaVersion": 1, "source": "codex"}"#,
        );
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        assert_eq!(entry.invalid, None, "字段缺失不属 JSON 损坏");
        assert_eq!(entry.skill.sidecar.source.as_deref(), Some("codex"));
        assert_eq!(
            entry.skill.sidecar.targets,
            vec!["claude-code", "codex", "trae"]
        );
    }

    #[test]
    fn 读取skill_sidecar损坏标记invalid() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("bad-sidecar");
        write(&skill_dir.join("SKILL.md"), GOOD_SKILL_MD);
        write(&skill_dir.join(".skill-meta.json"), "{不是JSON");
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        let reason = entry.invalid.unwrap();
        assert!(
            reason.contains("Sidecar 损坏"),
            "原因应点名 Sidecar 损坏：{reason}"
        );
        // 损坏时仍合成默认，不静默丢弃
        assert_eq!(entry.skill.sidecar.source, None);
    }

    #[test]
    fn 读取skill_多重invalid原因合并() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("multi");
        write(&skill_dir.join(".skill-meta.json"), "{坏");
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        let reason = entry.invalid.unwrap();
        assert!(reason.contains("SKILL.md 缺失"));
        assert!(reason.contains("Sidecar 损坏"));
    }

    #[test]
    fn list_skills_排序_忽略散文件_缺skills目录为空() {
        let dir = make_vault();
        let skills_root = dir.path().join("skills");
        write(&skills_root.join("b-skill").join("SKILL.md"), GOOD_SKILL_MD);
        write(&skills_root.join("a-skill").join("SKILL.md"), GOOD_SKILL_MD);
        write(&skills_root.join("loose.md"), "根下散文件");
        let entries = list_skills(dir.path(), &connected()).unwrap();
        let slugs: Vec<&str> = entries.iter().map(|e| e.skill.slug.as_str()).collect();
        assert_eq!(
            slugs,
            vec!["a-skill", "b-skill"],
            "应按 slug 排序且忽略散文件"
        );

        // skills/ 缺失 → 空清单
        let empty = TempDir::new().unwrap();
        assert!(list_skills(empty.path(), &connected()).unwrap().is_empty());
    }

    #[test]
    fn 中文目录名slug正常() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("中文技能");
        write(&skill_dir.join("SKILL.md"), GOOD_SKILL_MD);
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        assert_eq!(entry.skill.slug, "中文技能");
    }

    #[test]
    fn 读取skill_frontmatter原键保序保留() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("ordered");
        write(
            &skill_dir.join("SKILL.md"),
            "---\nname: n\ndescription: d\nlicense: MIT\nmetadata:\n  k: v\n---\n正文\n",
        );
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        let keys: Vec<&str> = entry
            .skill
            .frontmatter
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(
            keys,
            vec!["name", "description", "license", "metadata"],
            "原键保序保留（含嵌套映射值）"
        );
        // 嵌套值不丢（metadata 是映射）
        let metadata = &entry.skill.frontmatter[3].1;
        assert_eq!(
            metadata.get("k"),
            Some(&serde_yaml_ng::Value::String("v".to_string()))
        );
    }

    #[test]
    fn 读取skill_body为frontmatter之后正文() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("body-skill");
        write(
            &skill_dir.join("SKILL.md"),
            "---\nname: n\ndescription: d\n---\n\n# 标题\n\n正文内容\n",
        );
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        assert_eq!(
            entry.skill.body, "# 标题\n\n正文内容",
            "body 应去掉前导空行"
        );
        // 无正文 / 无 frontmatter 的边界
        let skill_dir2 = dir.path().join("skills").join("body-empty");
        write(
            &skill_dir2.join("SKILL.md"),
            "---\nname: n\ndescription: d\n---\n",
        );
        assert_eq!(
            read_skill(&skill_dir2, &connected()).unwrap().skill.body,
            ""
        );
    }

    #[test]
    fn 读取skill_resources收集_排除skillmd与隐藏文件() {
        let dir = make_vault();
        let skill_dir = dir.path().join("skills").join("res-skill");
        write(&skill_dir.join("SKILL.md"), GOOD_SKILL_MD);
        write(&skill_dir.join(".skill-meta.json"), "{}");
        write(&skill_dir.join("resources"), "脚本资源");
        write(&skill_dir.join("sub").join("data.txt"), "子目录资源");
        let entry = read_skill(&skill_dir, &connected()).unwrap();
        let mut rels: Vec<String> = entry
            .skill
            .resources
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        rels.sort();
        assert_eq!(
            rels,
            vec!["resources", "sub/data.txt"],
            "应收集资源文件（递归）且排除 SKILL.md 与隐藏文件"
        );
    }
}
