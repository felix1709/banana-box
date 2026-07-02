// src-tauri/src/library.rs
// 数据读写纯逻辑，不依赖 Tauri runtime，便于单元测试。
// struct 用 serde camelCase 与前端 src/types/index.ts 对齐。
// Option 字段不 skip，None 始终序列化为 null（与前端 string | null 一致）。

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
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

fn default_api_base_url() -> String {
    "https://ai.leihuo.netease.com".to_string()
}

fn default_api_key() -> String {
    String::new()
}

fn default_reverse_model() -> String {
    "doubao-seed-1-6-vision-250815".to_string()
}

fn default_available_reverse_models() -> Vec<String> {
    vec![
        "doubao-seed-1-6-vision-250815".to_string(),
        "gpt-5.4-mini".to_string(),
        "qwen3.5-omni-plus".to_string(),
        "qwen3-vl-plus".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub hotkey: String,
    pub theme: String,
    #[serde(default = "default_api_base_url")]
    pub api_base_url: String,
    #[serde(default = "default_api_key")]
    pub api_key: String,
    #[serde(default = "default_reverse_model")]
    pub reverse_model: String,
    #[serde(default = "default_available_reverse_models")]
    pub available_reverse_models: Vec<String>,
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
                api_base_url: default_api_base_url(),
                api_key: default_api_key(),
                reverse_model: default_reverse_model(),
                available_reverse_models: default_available_reverse_models(),
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

// 把 data_dir 的 library.json + images/ 打包成 zip。
pub fn export_library(data_dir: &Path, zip_path: &Path) -> std::io::Result<()> {
    let file = fs::File::create(zip_path)?;
    let mut writer = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();

    let json = serde_json::to_string_pretty(&load_library(data_dir))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    writer.start_file("library.json", opts)?;
    writer.write_all(json.as_bytes())?;

    let images_dir = data_dir.join("images");
    if images_dir.exists() {
        for entry in fs::read_dir(&images_dir)? {
            let entry = entry?;
            let rel = format!("images/{}", entry.file_name().to_string_lossy());
            writer.start_file(&rel, opts)?;
            let mut f = fs::File::open(entry.path())?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            writer.write_all(&buf)?;
        }
    }
    writer.finish()?;
    Ok(())
}

// 解压 zip 到 data_dir（library.json + images/* 覆盖），返回读出的 Library。
pub fn import_library(zip_path: &Path, data_dir: &Path) -> std::io::Result<Library> {
    fs::create_dir_all(data_dir)?;
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let mut json_str: Option<String> = None;
    for i in 0..archive.len() {
        let mut f = archive
            .by_index(i)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let name = f.name().to_string();
        if name == "library.json" {
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            json_str = Some(s);
        } else if let Some(rel) = name.strip_prefix("images/") {
            if !rel.is_empty() {
                let target = data_dir.join("images");
                fs::create_dir_all(&target)?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;
                fs::write(target.join(rel), &buf)?;
            }
        }
    }
    let json = json_str.ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no library.json in zip")
    })?;
    let lib: Library = serde_json::from_str(&json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    save_library(data_dir, &lib)?;
    Ok(lib)
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

    #[test]
    fn export_import_roundtrip() {
        let data_dir = tempdir().unwrap();
        let mut lib = Library::default();
        lib.prompts.push(Prompt {
            id: "p1".into(),
            title: "t".into(),
            content: "c".into(),
            category_id: None,
            tags: vec![],
            image: Some("images/a.png".into()),
            created_at: 1,
            updated_at: 1,
        });
        fs::create_dir_all(data_dir.path().join("images")).unwrap();
        fs::write(data_dir.path().join("images/a.png"), b"fakepng").unwrap();
        save_library(data_dir.path(), &lib).unwrap();

        let zip_path = data_dir.path().join("export.zip");
        export_library(data_dir.path(), &zip_path).unwrap();

        let data_dir2 = tempdir().unwrap();
        let imported = import_library(&zip_path, data_dir2.path()).unwrap();
        assert_eq!(imported.prompts.len(), 1);
        assert_eq!(imported.prompts[0].image, Some("images/a.png".into()));
        // 图片已解包
        let restored = fs::read(data_dir2.path().join("images/a.png")).unwrap();
        assert_eq!(restored, b"fakepng");
    }

    #[test]
    fn load_old_settings_fills_reverse_api_defaults() {
        let dir = tempdir().unwrap();
        let json = r##"{
          "version": 1,
          "categories": [],
          "prompts": [],
          "settings": {
            "hotkey": "Ctrl+Shift+B",
            "theme": "auto"
          }
        }"##;
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(library_path(dir.path()), json).unwrap();

        let lib = load_library(dir.path());

        assert_eq!(lib.settings.api_base_url, "https://ai.leihuo.netease.com");
        assert_eq!(lib.settings.api_key, "");
        assert_eq!(lib.settings.reverse_model, "doubao-seed-1-6-vision-250815");
        assert_eq!(
            lib.settings.available_reverse_models,
            vec![
                "doubao-seed-1-6-vision-250815".to_string(),
                "gpt-5.4-mini".to_string(),
                "qwen3.5-omni-plus".to_string(),
                "qwen3-vl-plus".to_string(),
            ]
        );
    }
}
