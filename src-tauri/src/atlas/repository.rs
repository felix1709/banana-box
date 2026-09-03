use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasEntryDto {
    pub id: String,
    pub title: String,
    pub dimension: String,
    pub tags: Vec<String>,
    pub image: String,
    pub score: f64,
    pub status: String,
    pub file: String,
}

pub struct AtlasRepository {
    root: std::path::PathBuf,
}

impl AtlasRepository {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self { root }
    }

    pub fn list_entries(&self) -> Result<Vec<AtlasEntryDto>, String> {
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
            result.push(AtlasEntryDto {
                id: front.id.clone(),
                title: front.title.clone(),
                dimension: front.dimension.clone(),
                tags: front.tags.clone(),
                image,
                score: front.score,
                status: front.status.clone(),
                file: relative,
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
                let relative = candidate
                    .strip_prefix(&self.root)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                return Ok(relative);
            }
        }
        Ok(String::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasFrontmatter {
    id: String,
    title: String,
    dimension: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    score: f64,
    #[serde(default = "default_status")]
    status: String,
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
}
