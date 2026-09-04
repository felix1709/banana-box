use crate::{
    app_state::{AppServices, StartupGate},
    atlas::http_service::{ingest_from_payload, AtlasHttpService},
    atlas::repository::{AtlasCategoryDto, AtlasEntryDto, AtlasRepository},
    command_auth::MainArgs,
    commands::{
        parse_chat_completion_prompt, reverse_image_prompt_instruction,
        provider_commands::{reverse_image_data_url_for_request, reverse_image_request_timeouts},
    },
    provider_http::{
        MAX_MODEL_ID_BYTES, MAX_REVERSE_IMAGE_CONTENT_BYTES, MAX_REVERSE_IMAGE_RESPONSE_BYTES,
    },
    providers::ProviderKind,
};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_opener::OpenerExt;
use tokio_util::sync::CancellationToken;
use url::Url;
use std::sync::Arc;

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
pub fn load_atlas_categories(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    _args: MainArgs<EmptyAtlasArgs>,
) -> Result<Vec<AtlasCategoryDto>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.list_categories()
}

#[tauri::command]
pub fn create_atlas_category(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    name: String,
) -> Result<Vec<AtlasCategoryDto>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.create_category(name)
}

#[tauri::command]
pub fn rename_atlas_category(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    id: String,
    name: String,
) -> Result<Vec<AtlasCategoryDto>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.rename_category(id, name)
}

#[tauri::command]
pub fn delete_atlas_category(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    id: String,
) -> Result<Vec<AtlasCategoryDto>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.delete_category(id)
}

#[tauri::command]
pub fn set_atlas_entry_categories(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    entry_id: String,
    category_ids: Vec<String>,
) -> Result<Vec<AtlasCategoryDto>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.set_entry_categories(entry_id, category_ids)
}

#[tauri::command]
pub fn delete_atlas_entry(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    id: String,
) -> Result<(), String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.delete_entry(&id)
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
pub fn load_atlas_thumbnail(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    id: String,
) -> Result<Vec<u8>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    let thumbnail_path = repo
        .get_thumbnail_path(&id)?
        .ok_or_else(|| "ATLAS_THUMBNAIL_NOT_FOUND".to_string())?;
    std::fs::read(thumbnail_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_atlas_service_status(
    service: tauri::State<'_, AtlasHttpService>,
) -> Result<bool, String> {
    Ok(service.is_running())
}

#[tauri::command]
pub fn start_atlas_service(
    window: WebviewWindow,
    service: tauri::State<'_, AtlasHttpService>,
    _gate: tauri::State<'_, StartupGate>,
    _args: MainArgs<EmptyAtlasArgs>,
) -> Result<(), String> {
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?
        .inner()
        .clone();
    let analyzer = Arc::new(move |entry_id: String, root: std::path::PathBuf| {
        let services = services.clone();
        tauri::async_runtime::spawn(async move {
            let _ = analyze_atlas_entry_with_services(root, entry_id, &services).await;
        });
    });
    service.start(atlas_root()?, String::new(), Some(analyzer))
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

#[tauri::command]
pub async fn analyze_atlas_entry(
    window: WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    entry_id: String,
) -> Result<String, String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    analyze_atlas_entry_with_services(atlas_root()?, entry_id, &services).await
}

pub async fn analyze_atlas_entry_with_services(
    root: std::path::PathBuf,
    entry_id: String,
    services: &AppServices,
) -> Result<String, String> {
    let _permit = services.operations.enter_user()?;

    let repo = AtlasRepository::new(root);
    let image_path = repo
        .get_image_path(&entry_id)?
        .ok_or_else(|| "ATLAS_IMAGE_NOT_FOUND".to_string())?;
    let image_bytes = std::fs::read(&image_path).map_err(|_| "ATLAS_IMAGE_NOT_FOUND")?;
    if image_bytes.len() > 10 * 1024 * 1024 {
        return Err("IMAGE_TOO_LARGE".to_string());
    }

    let provider = services
        .providers
        .list(ProviderKind::ReverseImage)?
        .into_iter()
        .next()
        .ok_or_else(|| "NO_REVERSE_IMAGE_PROVIDER".to_string())?;
    let model = provider
        .probed_model
        .clone()
        .or_else(|| provider.default_model.clone())
        .or_else(|| provider.available_models.first().cloned())
        .ok_or_else(|| "NO_REVERSE_IMAGE_MODEL".to_string())?;
    if model.trim().is_empty() || model.len() > MAX_MODEL_ID_BYTES {
        return Err("INVALID_MODEL".to_string());
    }
    let resolved = services.providers.resolve_for_request(&provider.id)?;
    if resolved.provider.kind != ProviderKind::ReverseImage {
        return Err("PROVIDER_KIND_MISMATCH".to_string());
    }

    let full_image_path = std::path::PathBuf::from(&image_path);
    let data_url = reverse_image_data_url_for_request(&image_path, &full_image_path, &image_bytes)?;
    let request = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": reverse_image_prompt_instruction()
                    },
                    {
                        "type": "image_url",
                        "image_url": { "url": data_url }
                    }
                ]
            }
        ]
    });
    let endpoint = Url::parse(&resolved.provider.chat_completions_url)
        .map_err(|_| "INVALID_PROVIDER_URL".to_string())?;
    let body = services
        .provider_http
        .post_json_bounded_with_timeouts(
            endpoint,
            &resolved.api_key,
            request,
            MAX_REVERSE_IMAGE_RESPONSE_BYTES,
            CancellationToken::new(),
            reverse_image_request_timeouts(),
        )
        .await?;
    if body.len() > MAX_REVERSE_IMAGE_RESPONSE_BYTES {
        return Err("PROVIDER_RESPONSE_TOO_LARGE".to_string());
    }
    let body = String::from_utf8(body).map_err(|_| "INVALID_PROVIDER_RESPONSE".to_string())?;
    let prompt = parse_chat_completion_prompt(&body).map_err(|_| "INVALID_PROVIDER_RESPONSE")?;
    if prompt.len() > MAX_REVERSE_IMAGE_CONTENT_BYTES {
        return Err("PROVIDER_RESPONSE_TOO_LARGE".to_string());
    }

    let short_id = entry_id.chars().take(12).collect::<String>();
    let title = format!("参考图 {short_id}");
    repo.update_entry_analysis(&entry_id, &title, &[], &prompt)?;
    Ok(prompt)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyAtlasArgs {}
