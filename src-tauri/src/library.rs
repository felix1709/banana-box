// src-tauri/src/library.rs
// 数据读写纯逻辑，不依赖 Tauri runtime，便于单元测试。
// struct 用 serde camelCase 与前端 src/types/index.ts 对齐。
// Option 字段不 skip，None 始终序列化为 null（与前端 string | null 一致）。

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const LIBRARY_VERSION: i32 = 2;

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
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub order: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

pub(crate) fn default_api_base_url() -> String {
    "https://ai.leihuo.netease.com".to_string()
}

pub(crate) fn default_reverse_model() -> String {
    "doubao-seed-1-6-vision-250815".to_string()
}

pub(crate) fn default_available_reverse_models() -> Vec<String> {
    vec![
        "doubao-seed-1-6-vision-250815".to_string(),
        "gpt-5.4-mini".to_string(),
        "qwen3.5-omni-plus".to_string(),
        "qwen3-vl-plus".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
            version: LIBRARY_VERSION,
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
    load_library_strict(dir).unwrap_or_else(|_| Library::default())
}

pub struct LegacySecrets {
    pub api_base_url: String,
    pub api_key: Option<String>,
    pub reverse_model: String,
    pub available_reverse_models: Vec<String>,
}

pub fn load_library_strict(dir: &Path) -> Result<Library, String> {
    let path = library_path(dir);
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("读取 {} 失败：{error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("解析 {} 失败：{error}", path.display()))
}

pub fn normalize_legacy_json(raw: &str) -> Result<(Library, LegacySecrets, Vec<String>), String> {
    let (library, secrets, warnings, _, _) = normalize_legacy_json_with_counts(raw)?;
    Ok((library, secrets, warnings))
}

pub(crate) fn normalize_legacy_json_with_counts(
    raw: &str,
) -> Result<(Library, LegacySecrets, Vec<String>, usize, usize), String> {
    let mut value: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| "legacy library JSON 无法解析".to_string())?;
    if value.get("version").and_then(serde_json::Value::as_i64) != Some(1) {
        return Err("legacy library 版本不受支持".into());
    }
    let prompts = value
        .get_mut("prompts")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| "legacy library 缺少 prompts 数组".to_string())?;
    let mut favorites_defaulted = 0;
    let mut orders_rebuilt = 0;
    for (index, prompt) in prompts.iter_mut().enumerate() {
        let object = prompt
            .as_object_mut()
            .ok_or_else(|| "legacy library 的 prompt 不是对象".to_string())?;
        if !object.contains_key("favorite") {
            object.insert("favorite".into(), serde_json::Value::Bool(false));
            favorites_defaulted += 1;
        }
        if !object.contains_key("order") {
            object.insert("order".into(), serde_json::Value::from(index as i64));
            orders_rebuilt += 1;
        }
    }

    let settings = value
        .get_mut("settings")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| "legacy library 缺少 settings 对象".to_string())?;
    let secrets = LegacySecrets {
        api_base_url: settings
            .remove("apiBaseUrl")
            .and_then(|item| item.as_str().map(str::to_owned))
            .unwrap_or_else(default_api_base_url),
        api_key: settings
            .remove("apiKey")
            .and_then(|item| item.as_str().map(str::to_owned))
            .filter(|value| !value.trim().is_empty()),
        reverse_model: settings
            .remove("reverseModel")
            .and_then(|item| item.as_str().map(str::to_owned))
            .unwrap_or_else(default_reverse_model),
        available_reverse_models: settings
            .remove("availableReverseModels")
            .and_then(|item| serde_json::from_value(item).ok())
            .unwrap_or_else(default_available_reverse_models),
    };
    value["version"] = serde_json::Value::from(LIBRARY_VERSION);
    let library =
        serde_json::from_value(value).map_err(|_| "legacy library 的数据结构无效".to_string())?;

    let mut warnings = Vec::new();
    if favorites_defaulted != 0 {
        warnings.push("缺失的 favorite 已按 false 迁移，历史上已经丢失的收藏无法恢复".into());
    }
    if orders_rebuilt != 0 {
        warnings.push(format!("{orders_rebuilt} 条提示词缺少排序，已按原顺序迁移"));
    }
    Ok((
        library,
        secrets,
        warnings,
        favorites_defaulted,
        orders_rebuilt,
    ))
}

pub(crate) fn serialize_sanitized_library(library: &Library) -> Result<Vec<u8>, String> {
    let mut value =
        serde_json::to_value(library).map_err(|_| "无法序列化迁移后的 library".to_string())?;
    let settings = value
        .get_mut("settings")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| "迁移后的 library 缺少 settings 对象".to_string())?;
    for key in [
        "apiBaseUrl",
        "apiKey",
        "reverseModel",
        "availableReverseModels",
    ] {
        settings.remove(key);
    }
    serde_json::to_vec_pretty(&value).map_err(|_| "无法序列化迁移后的 library".to_string())
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
    fn normalizes_legacy_fields_without_retaining_credential_in_sanitized_json() {
        const LEGACY_KEY: &str = "test-only-legacy-key";
        let raw = format!(
            r#"{{
              "version": 1,
              "categories": [],
              "prompts": [
                {{
                  "id": "p1",
                  "title": "旧提示词",
                  "content": "内容",
                  "categoryId": null,
                  "tags": [],
                  "image": null,
                  "createdAt": 1,
                  "updatedAt": 2
                }}
              ],
              "settings": {{
                "hotkey": "Ctrl+Shift+B",
                "theme": "auto",
                "apiBaseUrl": "https://legacy.example.test/v1",
                "apiKey": "{LEGACY_KEY}",
                "reverseModel": "legacy-model",
                "availableReverseModels": ["legacy-model"]
              }}
            }}"#
        );

        let (library, secrets, warnings) = normalize_legacy_json(&raw).unwrap();
        let sanitized = serialize_sanitized_library(&library).unwrap();
        let sanitized_text = String::from_utf8(sanitized).unwrap();

        assert_eq!(library.version, LIBRARY_VERSION);
        assert!(!library.prompts[0].favorite);
        assert_eq!(library.prompts[0].order, 0);
        assert_eq!(secrets.api_base_url, "https://legacy.example.test/v1");
        assert_eq!(secrets.api_key.as_deref(), Some(LEGACY_KEY));
        assert_eq!(secrets.reverse_model, "legacy-model");
        assert_eq!(secrets.available_reverse_models, vec!["legacy-model"]);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("缺失的 favorite 已按 false 迁移")));
        assert!(!sanitized_text.contains(LEGACY_KEY));
        assert!(!sanitized_text.contains("apiKey"));
        assert!(!sanitized_text.contains("apiBaseUrl"));
        assert!(!sanitized_text.contains("reverseModel"));
        assert!(!sanitized_text.contains("availableReverseModels"));
    }

    #[test]
    fn legacy_normalization_rejects_a_missing_prompt_array() {
        let error = match normalize_legacy_json(r#"{"version":1,"settings":{}}"#) {
            Ok(_) => panic!("缺少 prompts 的旧数据不应被接受"),
            Err(error) => error,
        };

        assert!(error.contains("prompts"));
    }

    #[test]
    fn load_missing_returns_default() {
        let dir = tempdir().unwrap();
        let lib = load_library(dir.path());
        assert_eq!(lib.version, LIBRARY_VERSION);
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
            favorite: false,
            order: 0,
            created_at: 1,
            updated_at: 1,
        });
        save_library(dir.path(), &lib).unwrap();
        let loaded = load_library(dir.path());
        assert_eq!(loaded, lib);
    }

    #[test]
    fn prompt_favorite_and_order_round_trip() {
        let dir = tempdir().unwrap();
        let mut library = Library::default();
        library.prompts.push(Prompt {
            id: "p1".into(),
            title: "镜头".into(),
            content: "内容".into(),
            category_id: None,
            tags: vec![],
            image: None,
            favorite: true,
            order: 7,
            created_at: 1,
            updated_at: 2,
        });

        save_library(dir.path(), &library).unwrap();
        let saved: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(library_path(dir.path())).unwrap()).unwrap();
        assert_eq!(
            saved["prompts"][0]["favorite"],
            serde_json::Value::Bool(true)
        );
        assert_eq!(saved["prompts"][0]["order"], serde_json::Value::from(7));
        let loaded = load_library_strict(dir.path()).unwrap();

        assert!(loaded.prompts[0].favorite);
        assert_eq!(loaded.prompts[0].order, 7);
    }

    #[test]
    fn strict_load_v1_prompt_defaults_favorite_and_order() {
        let dir = tempdir().unwrap();
        let json = r#"{
          "version": 1,
          "categories": [],
          "prompts": [
            {
              "id": "p1",
              "title": "旧提示词",
              "content": "兼容旧数据",
              "categoryId": null,
              "tags": [],
              "image": null,
              "createdAt": 1,
              "updatedAt": 2
            }
          ],
          "settings": {
            "hotkey": "Ctrl+Shift+B",
            "theme": "auto"
          }
        }"#;
        fs::write(library_path(dir.path()), json).unwrap();

        let loaded = load_library_strict(dir.path()).unwrap();

        assert!(!loaded.prompts[0].favorite);
        assert_eq!(loaded.prompts[0].order, 0);
    }

    #[test]
    fn missing_library_is_strict_error_and_lenient_default() {
        let dir = tempdir().unwrap();

        assert!(load_library_strict(dir.path()).is_err());
        assert_eq!(load_library(dir.path()), Library::default());
    }

    #[test]
    fn corrupt_library_json_is_strict_error_and_lenient_default() {
        let dir = tempdir().unwrap();
        fs::write(library_path(dir.path()), "{not valid JSON").unwrap();

        assert!(load_library_strict(dir.path()).is_err());
        assert_eq!(load_library(dir.path()), Library::default());
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
            favorite: false,
            order: 0,
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
    fn load_legacy_settings_keeps_only_non_sensitive_fields() {
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

        assert_eq!(lib.settings.hotkey, "Ctrl+Shift+B");
        assert_eq!(lib.settings.theme, "auto");
    }

    #[test]
    fn v2_settings_reject_each_reintroduced_legacy_api_field() {
        for (field, value) in [
            ("apiBaseUrl", "\"https://legacy.example.test\""),
            ("apiKey", "\"legacy-key\""),
            ("reverseModel", "\"legacy-model\""),
            ("availableReverseModels", "[\"legacy-model\"]"),
        ] {
            let json = format!(
                r#"{{
                  "version": 2,
                  "categories": [],
                  "prompts": [],
                  "settings": {{
                    "hotkey": "Ctrl+Shift+B",
                    "theme": "auto",
                    "{field}": {value}
                  }}
                }}"#
            );

            let error = serde_json::from_str::<Library>(&json).unwrap_err();
            assert!(error.to_string().contains(field));
        }
    }

    #[test]
    fn default_library_serializes_only_non_sensitive_settings() {
        let serialized = serde_json::to_value(Library::default()).unwrap();

        assert_eq!(
            serialized["settings"],
            serde_json::json!({
                "hotkey": "Ctrl+Shift+B",
                "theme": "auto",
            })
        );
    }
}
