use crate::{
    app_state::StartupGate,
    atlas::repository::{AtlasEntryDto, AtlasRepository},
    command_auth::MainArgs,
};
use tauri_plugin_opener::OpenerExt;

fn atlas_root() -> Result<std::path::PathBuf, String> {
    let home = dirs_home_dir().ok_or_else(|| "HOME_NOT_FOUND".to_string())?;
    let config = home.join(".banana-box").join("atlas-config.json");
    let root = if config.exists() {
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(config).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        std::path::PathBuf::from(
            value
                .get("rootPath")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
        )
    } else {
        home.join("Documents").join("AAA-Aesthetic-Atlas")
    };
    Ok(root)
}

fn dirs_home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE").map(std::path::PathBuf::from)
}

#[tauri::command]
pub fn load_atlas_entries(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    _args: MainArgs<EmptyAtlasArgs>,
) -> Result<Vec<AtlasEntryDto>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.list_entries()
}

#[tauri::command]
pub fn load_atlas_entry(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    id: String,
) -> Result<Option<String>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.get_entry(&id)
}

#[tauri::command]
pub fn open_atlas_folder(
    app: tauri::AppHandle,
    _gate: tauri::State<'_, StartupGate>,
) -> Result<(), String> {
    let root = atlas_root()?;
    app.opener()
        .open_path(root.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyAtlasArgs {}
