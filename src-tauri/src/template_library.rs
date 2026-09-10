use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn template_library_dir(resource_dir: &Path) -> PathBuf {
    let candidates = [
        resource_dir.join("template-library"),
        resource_dir.join("resources").join("template-library"),
    ];
    for candidate in candidates {
        if candidate.join("cases.json").is_file() {
            return candidate;
        }
    }
    resource_dir.join("template-library")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateCategoryDto {
    pub id: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateCaseDto {
    pub id: i64,
    pub title: String,
    pub category: String,
    pub image: String,
    pub tags: Vec<String>,
    pub prompt_en: String,
    pub prompt_zh: String,
    pub source_label: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateLibraryDto {
    pub version: i64,
    pub categories: Vec<TemplateCategoryDto>,
    pub cases: Vec<TemplateCaseDto>,
}

pub fn load_template_library(resource_dir: &Path) -> Result<TemplateLibraryDto, String> {
    let path = template_library_dir(resource_dir).join("cases.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("TEMPLATE_LIBRARY_NOT_FOUND: {error} ({})", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("TEMPLATE_LIBRARY_INVALID: {error}"))
}

pub fn load_template_image(resource_dir: &Path, image_path: &str) -> Result<Vec<u8>, String> {
    let relative = image_path.trim_start_matches('/');
    if relative.is_empty()
        || relative.contains("..")
        || !relative.starts_with("images/")
        || relative.contains('\\')
    {
        return Err("TEMPLATE_IMAGE_PATH_INVALID".to_string());
    }
    let full = template_library_dir(resource_dir).join(relative);
    std::fs::read(&full).map_err(|error| format!("TEMPLATE_IMAGE_NOT_FOUND: {error}"))
}
