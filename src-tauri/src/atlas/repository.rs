use serde::{Deserialize, Serialize};
use fs2::FileExt;
use std::fs;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasEntryDto {
    pub id: String,
    pub title: String,
    pub dimension: String,
    pub tags: Vec<String>,
    pub image: String,
    pub thumbnail: String,
    pub score: f64,
    pub status: String,
    pub file: String,
    #[serde(default)]
    pub category_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasCategoryDto {
    pub id: String,
    pub name: String,
    pub entry_ids: Vec<String>,
    pub created_at: String,
}

pub struct AtlasRepository {
    root: std::path::PathBuf,
}

impl AtlasRepository {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self { root }
    }

    pub fn list_entries(&self) -> Result<Vec<AtlasEntryDto>, String> {
        let categories = self.load_category_file()?;
        if let Some(mut cached) = self.load_entry_index_if_fresh()? {
            for entry in &mut cached {
                entry.category_ids = entry_category_ids(&categories, &entry.id);
            }
            return Ok(cached);
        }

        let entries = self.scan_entry_dtos(&categories)?;
        let fingerprint = self.current_entry_fingerprint()?;
        self.save_entry_index(&fingerprint, &entries)?;
        Ok(entries)
    }

    fn scan_entry_dtos(&self, categories: &AtlasCategoryFile) -> Result<Vec<AtlasEntryDto>, String> {
        let entries_root = self.root.join("entries");
        let mut result = Vec::new();
        if !entries_root.exists() {
            return Ok(result);
        }
        for entry in walkdir::WalkDir::new(&entries_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
        {
            let content = std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
            let Some(front) = parse_frontmatter(&content) else {
                continue;
            };
            let relative = entry
                .path()
                .strip_prefix(&self.root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let image = self.resolve_image(&front)?;
            let thumbnail = self.resolve_or_build_thumbnail(&front.id, &image);
            result.push(AtlasEntryDto {
                id: front.id.clone(),
                title: front.title.clone(),
                dimension: front.dimension.clone(),
                tags: front.tags.clone(),
                image,
                thumbnail,
                score: front.score,
                status: front.status.clone(),
                file: relative,
                category_ids: entry_category_ids(&categories, &front.id),
            });
        }
        Ok(result)
    }

    pub fn get_entry(&self, id: &str) -> Result<Option<String>, String> {
        for entry in self.list_entries()? {
            if entry.id == id {
                let path = self.root.join(&entry.file);
                return std::fs::read_to_string(path)
                    .map(Some)
                    .map_err(|e| e.to_string());
            }
        }
        Ok(None)
    }

    fn resolve_image(&self, front: &AtlasFrontmatter) -> Result<String, String> {
        let folder = dimension_folder(&front.dimension);
        let assets_dir = self.root.join("assets").join(folder);
        for ext in ["jpg", "jpeg", "png", "webp", "gif"] {
            let candidate = assets_dir.join(format!("{}.{}", front.id, ext));
            if candidate.exists() {
                return Ok(candidate.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(String::new())
    }

    fn resolve_thumbnail(&self, id: &str) -> String {
        let candidate = self
            .root
            .join("assets")
            .join("thumbs")
            .join(format!("{id}.webp"));
        candidate
            .exists()
            .then(|| candidate.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default()
    }

    fn resolve_or_build_thumbnail(&self, id: &str, image: &str) -> String {
        if image.is_empty() {
            return String::new();
        }
        let existing = self.resolve_thumbnail(id);
        if !existing.is_empty() {
            return existing;
        }
        match self.ensure_thumbnail(id, image) {
            Ok(path) => path,
            Err(_) => String::new(),
        }
    }

    fn ensure_thumbnail(&self, id: &str, image: &str) -> Result<String, String> {
        let bytes = fs::read(image).map_err(|e| e.to_string())?;
        let thumbnail = super::ingest::compress_image_to_webp(&bytes, 320)?;
        super::ingest::save_thumbnail_asset(&self.root, id, &thumbnail)
    }

    fn entry_index_path(&self) -> std::path::PathBuf {
        self.root.join("index").join("entries.json")
    }

    fn load_entry_index_if_fresh(&self) -> Result<Option<Vec<AtlasEntryDto>>, String> {
        let path = self.entry_index_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => return Ok(None),
        };
        let index: AtlasEntryIndex = match serde_json::from_str(&content) {
            Ok(index) => index,
            Err(_) => return Ok(None),
        };
        if index.version != 1 {
            return Ok(None);
        }
        let fingerprint = self.current_entry_fingerprint()?;
        if index.fingerprint != fingerprint {
            return Ok(None);
        }
        Ok(Some(index.entries))
    }

    fn save_entry_index(&self, fingerprint: &str, entries: &[AtlasEntryDto]) -> Result<(), String> {
        let path = self.entry_index_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let index = AtlasEntryIndex {
            version: 1,
            fingerprint: fingerprint.to_string(),
            entries: entries.to_vec(),
        };
        let payload = serde_json::to_string(&index).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, payload).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }

    fn current_entry_fingerprint(&self) -> Result<String, String> {
        let mut stamps = self.entry_file_stamps()?;
        stamps.sort_by(|left, right| left.path.cmp(&right.path));
        serde_json::to_string(&stamps).map_err(|e| e.to_string())
    }

    fn entry_file_stamps(&self) -> Result<Vec<AtlasIndexFileStamp>, String> {
        let entries_root = self.root.join("entries");
        let mut stamps = Vec::new();
        if !entries_root.exists() {
            return Ok(stamps);
        }
        for entry in walkdir::WalkDir::new(&entries_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
        {
            let relative = entry
                .path()
                .strip_prefix(&self.root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            let modified = metadata
                .modified()
                .map(|time| system_time_secs(time))
                .unwrap_or(0);
            stamps.push(AtlasIndexFileStamp {
                path: relative,
                modified,
                len: metadata.len(),
            });
        }
        Ok(stamps)
    }

    pub fn get_image_path(&self, id: &str) -> Result<Option<String>, String> {
        for entry in self.list_entries()? {
            if entry.id == id && !entry.image.is_empty() {
                return Ok(Some(entry.image));
            }
        }
        Ok(None)
    }

    pub fn get_thumbnail_path(&self, id: &str) -> Result<Option<String>, String> {
        for entry in self.list_entries()? {
            if entry.id == id && !entry.thumbnail.is_empty() {
                return Ok(Some(entry.thumbnail));
            }
        }
        Ok(None)
    }

    pub fn find_entry_by_source_url(&self, source_url: &str) -> Result<Option<String>, String> {
        let entries_root = self.root.join("entries");
        if !entries_root.exists() {
            return Ok(None);
        }
        for entry in walkdir::WalkDir::new(&entries_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
        {
            let content = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
            let Some(front) = parse_frontmatter(&content) else {
                continue;
            };
            if front.source_url.as_deref() == Some(source_url) {
                return Ok(Some(front.id));
            }
        }
        Ok(None)
    }

    pub fn delete_entry(&self, id: &str) -> Result<(), String> {
        let entry = self
            .list_entries()?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| "ATLAS_ENTRY_NOT_FOUND".to_string())?;

        let md_path = self.root.join(&entry.file);
        if md_path.exists() {
            fs::remove_file(&md_path).map_err(|e| e.to_string())?;
        }
        if !entry.image.is_empty() {
            let image_path = std::path::PathBuf::from(&entry.image);
            if image_path.exists() {
                fs::remove_file(image_path).map_err(|e| e.to_string())?;
            }
        }
        if !entry.thumbnail.is_empty() {
            let thumbnail_path = std::path::PathBuf::from(&entry.thumbnail);
            if thumbnail_path.exists() {
                fs::remove_file(thumbnail_path).map_err(|e| e.to_string())?;
            }
        }

        let _lock = self.lock_category_file()?;
        let mut file = self.load_category_file_unlocked()?;
        file.associations.retain(|association| association.entry_id != id);
        self.save_category_file_unlocked(&file)?;
        Ok(())
    }

    pub fn update_entry_analysis(
        &self,
        id: &str,
        title: &str,
        tags: &[String],
        analysis_body: &str,
    ) -> Result<(), String> {
        let entry = self
            .list_entries()?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| "ATLAS_ENTRY_NOT_FOUND".to_string())?;
        let path = self.root.join(&entry.file);
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let trimmed = content.trim_start();
        let Some(rest) = trimmed.strip_prefix("---\n") else {
            return Err("ATLAS_ENTRY_INVALID".to_string());
        };
        let Some(end) = rest.find("\n---") else {
            return Err("ATLAS_ENTRY_INVALID".to_string());
        };
        let frontmatter = &rest[..end];
        let mut value: serde_json::Value =
            serde_yaml::from_str(frontmatter).map_err(|e| e.to_string())?;
        value["title"] = serde_json::Value::String(title.to_string());
        value["tags"] = serde_json::Value::Array(
            tags.iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        );
        value["status"] = serde_json::Value::String("confirmed".to_string());
        let new_frontmatter = serde_yaml::to_string(&value).map_err(|e| e.to_string())?;
        let body = format!(
            "---\n{}\n---\n\n## 分析\n{}\n",
            new_frontmatter.trim_end(),
            analysis_body
        );
        let tmp = path.with_extension("md.tmp");
        fs::write(&tmp, body).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }

    pub fn list_categories(&self) -> Result<Vec<AtlasCategoryDto>, String> {
        let file = self.load_category_file()?;
        Ok(category_dtos(&file))
    }

    pub fn create_category(&self, name: String) -> Result<Vec<AtlasCategoryDto>, String> {
        let name = normalized_category_name(&name)?;
        let _lock = self.lock_category_file()?;
        let mut file = self.load_category_file_unlocked()?;
        if file
            .categories
            .iter()
            .any(|category| category.name.eq_ignore_ascii_case(&name))
        {
            return Err("ATLAS_CATEGORY_DUPLICATE".to_string());
        }
        let now = chrono::Local::now().to_rfc3339();
        file.categories.push(AtlasCategoryRecord {
            id: format!("cat-{}", Uuid::new_v4()),
            name,
            created_at: now,
        });
        self.save_category_file_unlocked(&file)?;
        Ok(category_dtos(&file))
    }

    pub fn rename_category(
        &self,
        id: String,
        name: String,
    ) -> Result<Vec<AtlasCategoryDto>, String> {
        let name = normalized_category_name(&name)?;
        let _lock = self.lock_category_file()?;
        let mut file = self.load_category_file_unlocked()?;
        let Some(index) = file.categories.iter().position(|item| item.id == id) else {
            return Err("ATLAS_CATEGORY_NOT_FOUND".to_string());
        };
        if file
            .categories
            .iter()
            .any(|item| item.id != id && item.name.eq_ignore_ascii_case(&name))
        {
            return Err("ATLAS_CATEGORY_DUPLICATE".to_string());
        }
        file.categories[index].name = name;
        self.save_category_file_unlocked(&file)?;
        Ok(category_dtos(&file))
    }

    pub fn delete_category(&self, id: String) -> Result<Vec<AtlasCategoryDto>, String> {
        let _lock = self.lock_category_file()?;
        let mut file = self.load_category_file_unlocked()?;
        file.categories.retain(|category| category.id != id);
        file.associations
            .retain(|association| association.category_id != id);
        self.save_category_file_unlocked(&file)?;
        Ok(category_dtos(&file))
    }

    pub fn set_entry_categories(
        &self,
        entry_id: String,
        category_ids: Vec<String>,
    ) -> Result<Vec<AtlasCategoryDto>, String> {
        let _lock = self.lock_category_file()?;
        let mut file = self.load_category_file_unlocked()?;
        let valid_ids: std::collections::HashSet<&str> = file
            .categories
            .iter()
            .map(|category| category.id.as_str())
            .collect();
        let unique_ids: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            category_ids
                .into_iter()
                .filter(|category_id| valid_ids.contains(category_id.as_str()))
                .filter(|category_id| seen.insert(category_id.clone()))
                .collect()
        };
        file.associations
            .retain(|association| association.entry_id != entry_id);
        for category_id in unique_ids {
            file.associations.push(AtlasAssociationRecord {
                entry_id: entry_id.clone(),
                category_id,
            });
        }
        self.save_category_file_unlocked(&file)?;
        Ok(category_dtos(&file))
    }

    fn category_file_path(&self) -> std::path::PathBuf {
        self.root.join("index").join("categories.json")
    }

    fn lock_category_file(&self) -> Result<fs::File, String> {
        let path = self
            .category_file_path()
            .with_extension("json.lock");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.lock_exclusive().map_err(|e| e.to_string())?;
        Ok(file)
    }

    fn load_category_file(&self) -> Result<AtlasCategoryFile, String> {
        self.load_category_file_unlocked()
    }

    fn load_category_file_unlocked(&self) -> Result<AtlasCategoryFile, String> {
        let path = self.category_file_path();
        if !path.exists() {
            return Ok(AtlasCategoryFile::default());
        }
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content)
            .or_else(|_| Ok(AtlasCategoryFile::default()))
            .map_err(|e: serde_json::Error| e.to_string())
    }

    fn save_category_file_unlocked(&self, file: &AtlasCategoryFile) -> Result<(), String> {
        let path = self.category_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let payload = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, payload).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasCategoryFile {
    #[serde(default = "default_category_file_version")]
    version: u32,
    #[serde(default)]
    categories: Vec<AtlasCategoryRecord>,
    #[serde(default)]
    associations: Vec<AtlasAssociationRecord>,
}

impl Default for AtlasCategoryFile {
    fn default() -> Self {
        Self {
            version: default_category_file_version(),
            categories: Vec::new(),
            associations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasCategoryRecord {
    id: String,
    name: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasAssociationRecord {
    entry_id: String,
    category_id: String,
}

fn default_category_file_version() -> u32 {
    1
}

fn normalized_category_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("ATLAS_CATEGORY_NAME_EMPTY".to_string());
    }
    if name.chars().count() > 40 {
        return Err("ATLAS_CATEGORY_NAME_TOO_LONG".to_string());
    }
    Ok(name.to_string())
}

fn entry_category_ids(file: &AtlasCategoryFile, entry_id: &str) -> Vec<String> {
    file.associations
        .iter()
        .filter(|association| association.entry_id == entry_id)
        .map(|association| association.category_id.clone())
        .collect()
}

fn category_dtos(file: &AtlasCategoryFile) -> Vec<AtlasCategoryDto> {
    file.categories
        .iter()
        .map(|category| AtlasCategoryDto {
            id: category.id.clone(),
            name: category.name.clone(),
            entry_ids: file
                .associations
                .iter()
                .filter(|association| association.category_id == category.id)
                .map(|association| association.entry_id.clone())
                .collect(),
            created_at: category.created_at.clone(),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasFrontmatter {
    id: String,
    title: String,
    dimension: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    score: f64,
    #[serde(default = "default_status")]
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasEntryIndex {
    version: u32,
    fingerprint: String,
    entries: Vec<AtlasEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasIndexFileStamp {
    path: String,
    modified: u64,
    len: u64,
}

fn system_time_secs(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn default_status() -> String {
    "pending_review".to_string()
}

fn dimension_folder(dimension: &str) -> &str {
    match dimension {
        "scene-concept" => "01-scene-concept",
        "style" => "02-style",
        "character-pose" => "03-character-pose",
        "composition" => "04-composition",
        "lighting-atmosphere" => "05-lighting-atmosphere",
        "fx-effects" => "06-fx-effects",
        "color-texture" => "07-color-texture",
        "emotion-mood" => "08-emotion-mood",
        "camera-lens" => "09-camera-lens",
        "model-constraints" => "10-model-constraints",
        _ => dimension,
    }
}

fn parse_frontmatter(content: &str) -> Option<AtlasFrontmatter> {
    let rest = content.trim_start().strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    serde_yaml::from_str(&rest[..end]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn list_entries_reads_markdown_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("atlas");
        fs::create_dir_all(root.join("entries/05-lighting-atmosphere")).unwrap();
        fs::write(
            root.join("entries/05-lighting-atmosphere/a.md"),
            "---\nid: a\ntitle: 霓虹逆光\ndimension: lighting-atmosphere\ntags: [霓虹]\nstatus: confirmed\n---\n",
        )
        .unwrap();
        let repo = AtlasRepository::new(root);
        let entries = repo.list_entries().unwrap();
        assert_eq!(entries[0].id, "a");
    }

    #[test]
    fn category_favorites_are_persisted_and_delete_keeps_original_entry() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("atlas");
        fs::create_dir_all(root.join("entries/01-scene-concept")).unwrap();
        fs::write(
            root.join("entries/01-scene-concept/a.md"),
            "---\nid: a\ntitle: 测试参考\ndimension: scene-concept\ntags: []\nstatus: confirmed\n---\n",
        )
        .unwrap();
        let repo = AtlasRepository::new(root.clone());

        let categories = repo.create_category("项目A".to_string()).unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].name, "项目A");

        let categories = repo
            .set_entry_categories("a".to_string(), vec![categories[0].id.clone()])
            .unwrap();
        assert_eq!(categories[0].entry_ids, vec!["a".to_string()]);

        let entries = repo.list_entries().unwrap();
        assert_eq!(entries[0].category_ids, vec![categories[0].id.clone()]);

        let categories = repo.delete_category(categories[0].id.clone()).unwrap();
        assert!(categories.is_empty());
        let entries = repo.list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].category_ids.is_empty());
    }

    #[test]
    fn rename_category_keeps_its_favorites() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("atlas");
        let repo = AtlasRepository::new(root);
        let categories = repo.create_category("旧名称".to_string()).unwrap();
        let categories = repo
            .set_entry_categories("a".to_string(), vec![categories[0].id.clone()])
            .unwrap();
        let renamed = repo
            .rename_category(categories[0].id.clone(), "新名称".to_string())
            .unwrap();
        assert_eq!(renamed[0].name, "新名称");
        assert_eq!(renamed[0].entry_ids, vec!["a".to_string()]);
    }

    #[test]
    fn delete_entry_removes_markdown_asset_and_associations() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("atlas");
        fs::create_dir_all(root.join("entries/01-scene-concept")).unwrap();
        fs::create_dir_all(root.join("assets/01-scene-concept")).unwrap();
        fs::write(
            root.join("entries/01-scene-concept/a.md"),
            "---\nid: a\ntitle: 待删除\ndimension: scene-concept\ntags: []\nstatus: confirmed\n---\n",
        )
        .unwrap();
        fs::write(root.join("assets/01-scene-concept/a.webp"), b"image").unwrap();
        let repo = AtlasRepository::new(root.clone());
        let categories = repo.create_category("项目A".to_string()).unwrap();
        repo.set_entry_categories("a".to_string(), vec![categories[0].id.clone()])
            .unwrap();

        repo.delete_entry("a").unwrap();

        assert!(!root.join("entries/01-scene-concept/a.md").exists());
        assert!(!root.join("assets/01-scene-concept/a.webp").exists());
        assert!(repo.list_entries().unwrap().is_empty());
        assert!(repo
            .list_categories()
            .unwrap()
            .into_iter()
            .all(|category| category.entry_ids.is_empty()));
    }

    #[test]
    fn update_entry_analysis_replaces_body_and_confirms_status() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("atlas");
        fs::create_dir_all(root.join("entries/05-lighting-atmosphere")).unwrap();
        fs::write(
            root.join("entries/05-lighting-atmosphere/a.md"),
            "---\nid: a\ntitle: pending\ndimension: 05-lighting-atmosphere\ntags: []\nstatus: pending_review\n---\n\n## 分析\n待识图\n",
        )
        .unwrap();
        let repo = AtlasRepository::new(root.clone());
        repo.update_entry_analysis("a", "参考图 a", &["霓虹".to_string()], "逆光人像")
            .unwrap();
        let content =
            fs::read_to_string(root.join("entries/05-lighting-atmosphere/a.md")).unwrap();
        assert!(content.contains("title: 参考图 a"));
        assert!(content.contains("status: confirmed"));
        assert!(content.contains("逆光人像"));
    }
}
