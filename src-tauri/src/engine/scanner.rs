//! 目录扫描：目标工具端 Skill 目录、文件 hash（导入与分发共用前置）。
//!
//! S1 实现（`docs/specs/s1-matrix.md` §3–§4）：
//! - blake3；目录 hash = 排序后的（相对路径 + 各文件内容 hash）流式喂入——重命名可感知
//! - 排除隐藏元数据文件 `.skill-meta.json`（与分发排除、导入去重口径一致）
//! - 工具端 skills/ 根下直接子目录全算 Skill 候选（目录同构），根下散文件忽略；
//!   目录存在即算（无 SKILL.md 也计入，由 hash 比对自然得出「被工具修改」）
//! - 工具端根目录不存在 → 空清单（「缺失」由状态层判定）

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::engine::error::{EngineError, EngineResult};

/// 文件条目：相对路径 + 内容 hash（S3 行级 diff 用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileEntry {
    pub rel_path: String,
    pub hash: String,
}

/// 单个 Skill 目录扫描结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedSkill {
    pub slug: String,
    pub dir_hash: String,
    pub files: Vec<FileEntry>,
}

/// 工具端整体扫描结果：`skills` 为根下全部 Skill 目录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    pub skills: Vec<ScannedSkill>,
}

/// 逐工具扫描（S4 导入器可复用）；根目录不存在 → `Ok(None)`（「缺失」由状态层判定），
/// Io 级错误传播。
pub fn scan_tool(root: &Path) -> EngineResult<Option<ScanResult>> {
    if !root.exists() {
        return Ok(None);
    }
    if !root.is_dir() {
        return Err(EngineError::Io(format!(
            "扫描目标不是目录：{}",
            root.display()
        )));
    }
    let mut skills = Vec::new();
    let mut dirs: Vec<_> = fs::read_dir(root)
        .map_err(io_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(io_err)?;
    dirs.sort_by_key(|e| e.file_name());
    for entry in dirs {
        let file_type = entry.file_type().map_err(io_err)?;
        if !file_type.is_dir() {
            continue; // 根下散文件忽略
        }
        let files = collect_files(&entry.path())?;
        let dir_hash = hash_from_files(&files);
        skills.push(ScannedSkill {
            slug: entry.file_name().to_string_lossy().into_owned(),
            dir_hash,
            files: files
                .into_iter()
                .map(|(rel_path, hash)| FileEntry { rel_path, hash })
                .collect(),
        });
    }
    Ok(Some(ScanResult { skills }))
}

/// 目录整体 hash：排序后的（相对路径 + 文件内容 hash）流式喂入；排除 `.skill-meta.json`。
pub fn hash_dir(root: &Path) -> EngineResult<String> {
    let files = collect_files(root)?;
    Ok(hash_from_files(&files))
}

/// 递归收集目录内全部文件（排除 `.skill-meta.json`），返回排序后的（相对路径, 内容 hash）。
/// 相对路径统一 `/` 分隔，保证 hash 跨平台一致。
fn collect_files(root: &Path) -> EngineResult<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).map_err(io_err)? {
            let entry = entry.map_err(io_err)?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(io_err)?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                if entry.file_name() == ".skill-meta.json" {
                    continue; // 隐藏元数据文件排除
                }
                let rel = path
                    .strip_prefix(root)
                    .expect("递归路径必在根下")
                    .to_string_lossy()
                    .replace('\\', "/");
                let content = fs::read(&path).map_err(io_err)?;
                let hash = blake3::hash(&content).to_hex().to_string();
                out.push((rel, hash));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// 由（相对路径, 内容 hash）清单流式聚合目录 hash——路径参与哈希，重命名可感知。
fn hash_from_files(files: &[(String, String)]) -> String {
    let mut hasher = blake3::Hasher::new();
    for (rel, hash) in files {
        hasher.update(rel.as_bytes());
        hasher.update(&[0]);
        hasher.update(hash.as_bytes());
        hasher.update(&[0]);
    }
    hasher.finalize().to_hex().to_string()
}

fn io_err(e: std::io::Error) -> EngineError {
    EngineError::Io(format!("扫描失败：{e}"))
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)] // 测试函数名用中文 + 可读性命名（非 snake_case）

    use super::*;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn 目录hash_排除skill_meta_内容参与() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        write(&a.join("SKILL.md"), "hello");
        write(&a.join(".skill-meta.json"), r#"{"source": null}"#);
        write(&b.join("SKILL.md"), "hello");
        write(&b.join(".skill-meta.json"), r#"{"source": "codex"}"#);
        // Sidecar 差异不影响目录 hash（被排除）
        assert_eq!(hash_dir(&a).unwrap(), hash_dir(&b).unwrap());

        // 内容变化 → hash 变化
        write(&a.join("SKILL.md"), "hello world");
        assert_ne!(hash_dir(&a).unwrap(), hash_dir(&b).unwrap());
    }

    #[test]
    fn 目录hash_重命名可感知_子目录文件参与() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        write(&a.join("SKILL.md"), "x");
        write(&a.join("scripts").join("helper.py"), "y");
        write(&b.join("SKILL.md"), "x");
        write(&b.join("helper.py"), "y"); // 同内容但路径不同
        assert_ne!(
            hash_dir(&a).unwrap(),
            hash_dir(&b).unwrap(),
            "重命名（路径变化）应可感知"
        );
    }

    #[test]
    fn 目录hash_与文件写入顺序无关() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        for path in [&a, &b] {
            fs::create_dir_all(path).unwrap();
        }
        write(&a.join("SKILL.md"), "1");
        write(&a.join("extra.txt"), "2");
        write(&b.join("extra.txt"), "2");
        write(&b.join("SKILL.md"), "1");
        assert_eq!(hash_dir(&a).unwrap(), hash_dir(&b).unwrap());
    }

    #[test]
    fn 空目录hash固定值() {
        let dir = TempDir::new().unwrap();
        let empty = dir.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        let h = hash_dir(&empty).unwrap();
        assert_eq!(h.len(), 64, "blake3 hex 64 字符");
        assert_eq!(hash_dir(&empty).unwrap(), h, "空目录 hash 确定");
    }

    #[test]
    fn 扫描_根不存在返回None() {
        let dir = TempDir::new().unwrap();
        assert_eq!(scan_tool(&dir.path().join("missing")).unwrap(), None);
    }

    #[test]
    fn 扫描_根下散文件忽略_子目录全算() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join("loose.md"), "散文件忽略");
        write(&dir.path().join("ok-skill").join("SKILL.md"), "x");
        write(
            &dir.path().join("no-md").join("notes.txt"),
            "无 SKILL.md 也计入",
        );
        let result = scan_tool(dir.path()).unwrap().unwrap();
        let slugs: Vec<&str> = result.skills.iter().map(|s| s.slug.as_str()).collect();
        assert_eq!(
            slugs,
            vec!["no-md", "ok-skill"],
            "排序 + 散文件忽略 + 目录存在即算"
        );
        for s in &result.skills {
            assert_eq!(s.dir_hash.len(), 64);
        }
        // 文件清单：相对路径 + 内容 hash（S3 diff 用）
        let ok = result.skills.iter().find(|s| s.slug == "ok-skill").unwrap();
        assert_eq!(ok.files.len(), 1);
        assert_eq!(ok.files[0].rel_path, "SKILL.md");
        assert_eq!(ok.files[0].hash, blake3::hash(b"x").to_hex().to_string());
    }

    #[test]
    fn 扫描_文件清单不含sidecar() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join("s").join("SKILL.md"), "x");
        write(&dir.path().join("s").join(".skill-meta.json"), "{}");
        let result = scan_tool(dir.path()).unwrap().unwrap();
        let s = &result.skills[0];
        assert_eq!(
            s.files
                .iter()
                .map(|f| f.rel_path.as_str())
                .collect::<Vec<_>>(),
            vec!["SKILL.md"]
        );
    }
}
