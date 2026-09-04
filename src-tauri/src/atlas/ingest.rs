use std::path::Path;

pub fn compress_image_to_webp(bytes: &[u8], max_dimension: u32) -> Result<Vec<u8>, String> {
    let image = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let resized = image.resize(max_dimension, max_dimension, image::imageops::FilterType::Lanczos3);
    let mut output = std::io::Cursor::new(Vec::new());
    resized
        .write_to(&mut output, image::ImageFormat::WebP)
        .map_err(|e| e.to_string())?;
    Ok(output.into_inner())
}

pub fn save_ingested_asset(
    root: &Path,
    dimension: &str,
    entry_id: &str,
    bytes: &[u8],
) -> Result<String, String> {
    let dir = root.join("assets").join(dimension);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{entry_id}.webp"));
    let tmp = dir.join(format!("{entry_id}.webp.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().replace('\\', "/"))
}

pub fn save_thumbnail_asset(
    root: &Path,
    entry_id: &str,
    bytes: &[u8],
) -> Result<String, String> {
    let dir = root.join("assets").join("thumbs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{entry_id}.webp"));
    let tmp = dir.join(format!("{entry_id}.webp.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().replace('\\', "/"))
}

pub fn write_entry_md(
    root: &Path,
    entry_id: &str,
    dimension: &str,
    frontmatter: &str,
    body: &str,
) -> Result<(), String> {
    let dir = root.join("entries").join(dimension);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{entry_id}.md"));
    let tmp = dir.join(format!("{entry_id}.md.tmp"));
    std::fs::write(&tmp, format!("---\n{frontmatter}\n---\n\n{body}")).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn build_frontmatter(
    entry_id: &str,
    title: &str,
    dimension: &str,
    tags: &[String],
    status: &str,
) -> String {
    build_frontmatter_with_source(entry_id, title, dimension, tags, status, None)
}

pub fn build_frontmatter_with_source(
    entry_id: &str,
    title: &str,
    dimension: &str,
    tags: &[String],
    status: &str,
    source_url: Option<&str>,
) -> String {
    let mut payload = serde_json::json!({
        "id": entry_id,
        "title": title,
        "dimension": dimension,
        "tags": tags,
        "status": status,
    });
    if let Some(source_url) = source_url {
        payload["source_url"] = serde_json::Value::String(source_url.to_string());
    }
    serde_yaml::to_string(&payload).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_entry_md_creates_atomic_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_entry_md(
            root,
            "a",
            "05-lighting-atmosphere",
            "id: a\ntitle: 测试",
            "## 提示词片段\n逆光",
        )
        .unwrap();
        assert!(root.join("entries/05-lighting-atmosphere/a.md").exists());
        assert!(!root.join("entries/05-lighting-atmosphere/a.md.tmp").exists());
    }

    #[test]
    fn build_frontmatter_serializes_tags_as_yaml_list() {
        let frontmatter = build_frontmatter(
            "a",
            "测试",
            "05-lighting-atmosphere",
            &["霓虹".into(), "逆光".into()],
            "pending_review",
        );
        assert!(frontmatter.contains("tags:"));
        assert!(frontmatter.contains("霓虹"));
    }
}
