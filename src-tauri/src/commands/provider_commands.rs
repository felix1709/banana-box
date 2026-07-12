use crate::{
    app_state::{AppServices, StartupGate},
    command_auth::MainArgs,
    commands::{
        data_dir, mime_from_path, parse_chat_completion_prompt, reverse_image_prompt_instruction,
    },
    provider_http::{
        MAX_MODEL_ID_BYTES, MAX_PROVIDER_MODELS, MAX_PROVIDER_MODELS_BODY_BYTES,
        MAX_REVERSE_IMAGE_CONTENT_BYTES, MAX_REVERSE_IMAGE_RESPONSE_BYTES,
    },
    providers::{AiProvider, ProviderKind, SaveProviderInput},
};
use base64::Engine;
use std::path::{Component, Path, PathBuf};
use tauri::{Manager, WebviewWindow};
use tokio_util::sync::CancellationToken;
use url::Url;

const APP_SERVICES_UNAVAILABLE: &str = "STARTUP_NOT_READY";
const MAX_REVERSE_IMAGE_SOURCE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ListAiProvidersCommandArgs {
    kind: ProviderKind,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveAiProviderCommandArgs {
    input: SaveProviderInput,
    api_key: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ClearAiProviderCredentialCommandArgs {
    provider_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CheckAiProviderConnectionCommandArgs {
    provider_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReverseImagePromptInput {
    pub provider_id: String,
    pub model: String,
    pub image_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckAiProviderConnectionResult {
    pub ok: bool,
    pub message: String,
    pub models: Vec<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseImagePromptResult {
    pub prompt: String,
}

#[tauri::command]
pub fn list_ai_providers(
    window: WebviewWindow,
    gate: tauri::State<StartupGate>,
    args: MainArgs<ListAiProvidersCommandArgs>,
) -> Result<Vec<AiProvider>, String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    services.providers.list(args.0.kind)
}

#[tauri::command]
pub fn save_ai_provider(
    window: WebviewWindow,
    gate: tauri::State<StartupGate>,
    args: MainArgs<SaveAiProviderCommandArgs>,
) -> Result<AiProvider, String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    let args = args.0;
    services.providers.save(args.input, args.api_key.as_deref())
}

#[tauri::command]
pub fn clear_ai_provider_credential(
    window: WebviewWindow,
    gate: tauri::State<StartupGate>,
    args: MainArgs<ClearAiProviderCredentialCommandArgs>,
) -> Result<(), String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    services.providers.clear_credential(&args.0.provider_id)
}

#[tauri::command]
pub async fn check_ai_provider_connection(
    window: WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<CheckAiProviderConnectionCommandArgs>,
) -> Result<CheckAiProviderConnectionResult, String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    let resolved = match services.providers.resolve_for_request(&args.0.provider_id) {
        Ok(resolved) => resolved,
        Err(error) => return Ok(connection_failure(error)),
    };
    let models_url = match Url::parse(&resolved.provider.models_url) {
        Ok(url) => url,
        Err(_) => return Ok(connection_failure("INVALID_PROVIDER_URL")),
    };

    let body = match services
        .provider_http
        .get_bounded(
            models_url,
            &resolved.api_key,
            MAX_PROVIDER_MODELS_BODY_BYTES,
            CancellationToken::new(),
        )
        .await
    {
        Ok(body) => body,
        Err(error) => return Ok(connection_failure(error)),
    };

    let models = parse_model_ids(&body);
    Ok(CheckAiProviderConnectionResult {
        ok: true,
        message: "CONNECTION_SUCCEEDED".into(),
        models,
    })
}

#[tauri::command]
pub async fn reverse_image_prompt(
    window: WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<ReverseImagePromptInput>,
) -> Result<ReverseImagePromptResult, String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    let input = args.0;
    let resolved = services.providers.resolve_for_request(&input.provider_id)?;

    if resolved.provider.kind != ProviderKind::ReverseImage {
        return Err("PROVIDER_KIND_MISMATCH".into());
    }
    let model = input.model.trim();
    if model.is_empty() || model.len() > MAX_MODEL_ID_BYTES {
        return Err("INVALID_MODEL".into());
    }

    let full_image_path = resolve_managed_image_path(&window, &input.image_path)?;
    let image_metadata = std::fs::metadata(&full_image_path).map_err(|_| "IMAGE_NOT_FOUND")?;
    if image_metadata.len() > MAX_REVERSE_IMAGE_SOURCE_BYTES {
        return Err("IMAGE_TOO_LARGE".into());
    }
    let bytes = std::fs::read(&full_image_path).map_err(|_| "IMAGE_NOT_FOUND")?;
    let data_url = format!(
        "data:{};base64,{}",
        mime_from_path(&input.image_path),
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
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
    let endpoint =
        Url::parse(&resolved.provider.chat_completions_url).map_err(|_| "INVALID_PROVIDER_URL")?;
    let body = services
        .provider_http
        .post_json_bounded(
            endpoint,
            &resolved.api_key,
            request,
            MAX_REVERSE_IMAGE_RESPONSE_BYTES,
            CancellationToken::new(),
        )
        .await?;
    if body.len() > MAX_REVERSE_IMAGE_RESPONSE_BYTES {
        return Err("PROVIDER_RESPONSE_TOO_LARGE".into());
    }
    let body = String::from_utf8(body).map_err(|_| "INVALID_PROVIDER_RESPONSE")?;
    let prompt = parse_chat_completion_prompt(&body).map_err(|_| "INVALID_PROVIDER_RESPONSE")?;
    if prompt.len() > MAX_REVERSE_IMAGE_CONTENT_BYTES {
        return Err("PROVIDER_RESPONSE_TOO_LARGE".into());
    }

    Ok(ReverseImagePromptResult { prompt })
}

fn resolve_managed_image_path(window: &WebviewWindow, image_path: &str) -> Result<PathBuf, String> {
    validate_managed_image_path(image_path)?;
    let relative = Path::new(image_path);
    let image_root = data_dir(window.app_handle())
        .join("images")
        .canonicalize()
        .map_err(|_| "IMAGE_NOT_FOUND")?;
    let full_path = data_dir(window.app_handle())
        .join(relative)
        .canonicalize()
        .map_err(|_| "IMAGE_NOT_FOUND")?;
    if !full_path.starts_with(&image_root) {
        return Err("INVALID_IMAGE_PATH".into());
    }
    Ok(full_path)
}

fn validate_managed_image_path(image_path: &str) -> Result<(), String> {
    let relative = Path::new(image_path);
    let mut components = relative.components();
    let Some(Component::Normal(first)) = components.next() else {
        return Err("INVALID_IMAGE_PATH".into());
    };
    if first != "images" || components.clone().next().is_none() {
        return Err("INVALID_IMAGE_PATH".into());
    }
    if components.any(|component| !matches!(component, Component::Normal(_))) {
        return Err("INVALID_IMAGE_PATH".into());
    }
    Ok(())
}

fn connection_failure(message: impl Into<String>) -> CheckAiProviderConnectionResult {
    CheckAiProviderConnectionResult {
        ok: false,
        message: message.into(),
        models: vec![],
    }
}

fn parse_model_ids(body: &[u8]) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct ModelsResponse {
        data: Vec<Model>,
    }

    #[derive(serde::Deserialize)]
    struct Model {
        id: String,
    }

    serde_json::from_slice::<ModelsResponse>(body)
        .map(|response| {
            response
                .data
                .into_iter()
                .map(|model| model.id)
                .filter(|id| !id.trim().is_empty() && id.len() <= MAX_MODEL_ID_BYTES)
                .take(MAX_PROVIDER_MODELS)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_provider_command_accepts_only_the_write_only_input_shape() {
        let parsed = serde_json::from_value::<SaveAiProviderCommandArgs>(serde_json::json!({
            "input": {
                "id": "reverse-image",
                "kind": "reverse-image",
                "displayName": "Reverse image",
                "baseUrl": "https://api.example.test/v1",
                "modelsUrl": "https://api.example.test/v1/models",
                "chatCompletionsUrl": "https://api.example.test/v1/chat/completions",
                "defaultModel": "vision-model",
                "confirmCrossOrigin": false
            },
            "apiKey": "only-written-once"
        }))
        .unwrap();

        assert_eq!(parsed.input.id, "reverse-image");
        assert_eq!(parsed.api_key.as_deref(), Some("only-written-once"));
        assert!(
            serde_json::from_value::<SaveAiProviderCommandArgs>(serde_json::json!({
                "input": {
                    "id": "reverse-image",
                    "kind": "reverse-image",
                    "displayName": "Reverse image",
                    "baseUrl": "https://api.example.test/v1",
                    "modelsUrl": "https://api.example.test/v1/models",
                    "chatCompletionsUrl": "https://api.example.test/v1/chat/completions",
                    "defaultModel": null,
                    "confirmCrossOrigin": false,
                    "credentialRef": "must-never-be-client-controlled"
                }
            }))
            .is_err()
        );
    }

    #[test]
    fn provider_command_args_reject_unknown_and_non_camel_case_fields() {
        assert!(
            serde_json::from_value::<ListAiProvidersCommandArgs>(serde_json::json!({
                "kind": "storyboard"
            }))
            .is_ok()
        );
        assert!(
            serde_json::from_value::<ListAiProvidersCommandArgs>(serde_json::json!({
                "provider_kind": "storyboard"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<ClearAiProviderCredentialCommandArgs>(serde_json::json!({
                "providerId": "reverse-image",
                "unexpected": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CheckAiProviderConnectionCommandArgs>(serde_json::json!({
                "providerId": "reverse-image",
                "probedModel": "server-owned"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<ReverseImagePromptInput>(serde_json::json!({
                "providerId": "reverse-image",
                "model": "vision-model",
                "imagePath": "images/a.png",
                "apiKey": "must-never-be-accepted"
            }))
            .is_err()
        );
    }

    #[test]
    fn connection_model_parser_keeps_only_bounded_model_ids() {
        let oversized_id = "x".repeat(crate::provider_http::MAX_MODEL_ID_BYTES + 1);
        let body = serde_json::json!({
            "data": [
                { "id": "vision-model" },
                { "id": "   " },
                { "id": oversized_id }
            ]
        });

        assert_eq!(
            parse_model_ids(body.to_string().as_bytes()),
            vec!["vision-model"]
        );
    }

    #[test]
    fn reverse_image_paths_must_stay_inside_the_managed_images_directory() {
        assert!(validate_managed_image_path("images/source.png").is_ok());

        for path in [
            "source.png",
            "../images/source.png",
            "images/../library.json",
            "/images/a.png",
        ] {
            assert_eq!(
                validate_managed_image_path(path).unwrap_err(),
                "INVALID_IMAGE_PATH"
            );
        }
    }
}
