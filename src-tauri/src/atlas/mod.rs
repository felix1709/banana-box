pub mod repository;
pub mod ingest;
pub mod http_service;

const ATLAS_DIMENSIONS: [&str; 10] = [
    "01-scene-concept",
    "02-style",
    "03-character-pose",
    "04-composition",
    "05-lighting-atmosphere",
    "06-fx-effects",
    "07-color-texture",
    "08-emotion-mood",
    "09-camera-lens",
    "10-model-constraints",
];

pub fn ensure_atlas_root() -> Result<std::path::PathBuf, String> {
    let home = std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "HOME_NOT_FOUND".to_string())?;
    let config = home.join(".banana-box").join("atlas-config.json");
    let default_root = home.join("Documents").join("AAA-Aesthetic-Atlas");
    let root = if config.exists() {
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let configured = std::path::PathBuf::from(
            value
                .get("rootPath")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
        );
        if configured.as_os_str().is_empty() {
            default_root
        } else {
            configured
        }
    } else {
        default_root
    };

    if !config.exists() {
        if let Some(parent) = config.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let payload = serde_json::json!({
            "version": 1,
            "rootPath": root.to_string_lossy(),
            "createdAt": chrono::Local::now().date_naive().to_string(),
        });
        let tmp = config.with_extension("tmp");
        std::fs::write(&tmp, payload.to_string()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &config).map_err(|e| e.to_string())?;
    }

    for base in ["entries", "assets", "index"] {
        for dimension in ATLAS_DIMENSIONS {
            std::fs::create_dir_all(root.join(base).join(dimension))
                .map_err(|e| e.to_string())?;
        }
    }
    std::fs::create_dir_all(root.join("entries"))
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(root.join("assets"))
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(root.join("index"))
        .map_err(|e| e.to_string())?;
    Ok(root)
}
