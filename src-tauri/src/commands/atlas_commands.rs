use crate::{
    app_state::StartupGate,
    atlas::http_service::{ingest_from_payload, AtlasHttpService},
    atlas::repository::{AtlasEntryDto, AtlasRepository},
    command_auth::MainArgs,
};
use tauri_plugin_opener::OpenerExt;

fn atlas_root() -> Result<std::path::PathBuf, String> {
    crate::atlas::ensure_atlas_root()
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

#[tauri::command]
pub fn load_atlas_image(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    id: String,
) -> Result<Vec<u8>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    let image_path = repo
        .get_image_path(&id)?
        .ok_or_else(|| "ATLAS_IMAGE_NOT_FOUND".to_string())?;
    std::fs::read(image_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_atlas_service_status(
    service: tauri::State<'_, AtlasHttpService>,
) -> Result<bool, String> {
    Ok(service.is_running())
}

#[tauri::command]
pub fn start_atlas_service(
    service: tauri::State<'_, AtlasHttpService>,
    _gate: tauri::State<'_, StartupGate>,
    _args: MainArgs<EmptyAtlasArgs>,
) -> Result<(), String> {
    service.start(atlas_root()?, String::new())
}

#[tauri::command]
pub fn stop_atlas_service(
    service: tauri::State<'_, AtlasHttpService>,
    _gate: tauri::State<'_, StartupGate>,
    _args: MainArgs<EmptyAtlasArgs>,
) -> Result<(), String> {
    service.stop();
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasIngestInput {
    pub local_path: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub dimension: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[tauri::command]
pub fn ingest_atlas_image(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    input: MainArgs<AtlasIngestInput>,
) -> Result<String, String> {
    gate.require_ready()?;
    let input = input.0;
    let mut payload = serde_json::json!({
        "localPath": input.local_path,
    });
    if let Some(title) = input.title {
        payload["title"] = serde_json::Value::String(title);
    }
    if let Some(dimension) = input.dimension {
        payload["dimension"] = serde_json::Value::String(dimension);
    }
    if let Some(tags) = input.tags {
        payload["tags"] = serde_json::Value::Array(
            tags.into_iter()
                .map(serde_json::Value::String)
                .collect(),
        );
    }
    ingest_from_payload(&atlas_root()?, &payload)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyAtlasArgs {}
