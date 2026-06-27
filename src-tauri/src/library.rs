// src-tauri/src/library.rs
// 数据读写纯逻辑，不依赖 Tauri runtime，便于单元测试。
// struct 用 serde camelCase 与前端 src/types/index.ts 对齐。
// Option 字段不 skip，None 始终序列化为 null（与前端 string | null 一致）。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: String,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category_id: Option<String>,
    pub tags: Vec<String>,
    pub image: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub hotkey: String,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub version: i32,
    pub categories: Vec<Category>,
    pub prompts: Vec<Prompt>,
    pub settings: Settings,
}

impl Default for Library {
    fn default() -> Self {
        Library {
            version: 1,
            categories: vec![],
            prompts: vec![],
            settings: Settings {
                hotkey: "Ctrl+Shift+B".to_string(),
                theme: "auto".to_string(),
            },
        }
    }
}

pub fn library_path(dir: &Path) -> PathBuf {
    dir.join("library.json")
}

pub fn load_library(dir: &Path) -> Library {
    let path = library_path(dir);
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Library::default()),
        Err(_) => Library::default(),
    }
}

pub fn save_library(dir: &Path, lib: &Library) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = library_path(dir);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(lib).expect("serialize library");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_missing_returns_default() {
        let dir = tempdir().unwrap();
        let lib = load_library(dir.path());
        assert_eq!(lib.version, 1);
        assert!(lib.prompts.is_empty());
        assert_eq!(lib.settings.hotkey, "Ctrl+Shift+B");
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempdir().unwrap();
        let mut lib = Library::default();
        lib.categories.push(Category {
            id: "c1".into(),
            name: "写作".into(),
            color: "#f59e0b".into(),
            order: 0,
        });
        lib.prompts.push(Prompt {
            id: "p1".into(),
            title: "总结".into(),
            content: "总结三点".into(),
            category_id: Some("c1".into()),
            tags: vec!["中文".into()],
            image: None,
            created_at: 1,
            updated_at: 1,
        });
        save_library(dir.path(), &lib).unwrap();
        let loaded = load_library(dir.path());
        assert_eq!(loaded, lib);
    }
}
