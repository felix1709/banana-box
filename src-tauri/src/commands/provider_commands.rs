use crate::{
    app_state::{AppServices, StartupGate},
    command_auth::MainArgs,
    commands::{
        data_dir, mime_from_path, parse_chat_completion_prompt, reverse_image_prompt_instruction,
    },
    provider_http::{
        MAX_MODEL_ID_BYTES, MAX_PROVIDER_MODELS, MAX_PROVIDER_MODELS_BODY_BYTES,
        MAX_REVERSE_IMAGE_CONTENT_BYTES, MAX_REVERSE_IMAGE_RESPONSE_BYTES, ProviderHttpTimeouts,
    },
    providers::{AiProvider, ProviderKind, SaveProviderInput},
};
use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use std::{
    path::{Component, Path, PathBuf},
    time::Duration,
};
use tauri::{Manager, WebviewWindow};
use tokio_util::sync::CancellationToken;
use url::Url;

const APP_SERVICES_UNAVAILABLE: &str = "STARTUP_NOT_READY";
const MAX_REVERSE_IMAGE_SOURCE_BYTES: u64 = 10 * 1024 * 1024;
const REVERSE_IMAGE_INLINE_TARGET_BYTES: usize = 1536 * 1024;
const REVERSE_IMAGE_PROBE_PNG_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAIAAAD8GO2jAAAAJklEQVR4nO3NMQ0AAAwDoPo33arYsQQMkB6LQCAQCAQCgUAg+BIMi1X0pjxKe0gAAAAASUVORK5CYII=";

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
    if resolved.provider.kind == ProviderKind::ReverseImage {
        let Some(model) = connection_probe_model(&resolved.provider, &models) else {
            return Ok(connection_failure_with_models("INVALID_MODEL", models));
        };
        let chat_url = match Url::parse(&resolved.provider.chat_completions_url) {
            Ok(url) => url,
            Err(_) => {
                return Ok(connection_failure_with_models(
                    "INVALID_PROVIDER_URL",
                    models,
                ))
            }
        };
        if let Err(error) =
            probe_reverse_image_chat(&services.provider_http, chat_url, &resolved.api_key, &model)
                .await
        {
            return Ok(connection_failure_with_models(error, models));
        }
    }
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
    let data_url = reverse_image_data_url_for_request(&input.image_path, &full_image_path, &bytes)?;
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

fn reverse_image_data_url_for_request(
    logical_image_path: &str,
    full_image_path: &Path,
    source_bytes: &[u8],
) -> Result<String, String> {
    let (mime, bytes) = if source_bytes.len() > REVERSE_IMAGE_INLINE_TARGET_BYTES {
        match compress_reverse_image_to_jpeg(full_image_path, REVERSE_IMAGE_INLINE_TARGET_BYTES) {
            Ok(compressed) if compressed.len() < source_bytes.len() => ("image/jpeg", compressed),
            _ => (mime_from_path(logical_image_path), source_bytes.to_vec()),
        }
    } else {
        (mime_from_path(logical_image_path), source_bytes.to_vec())
    };

    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn compress_reverse_image_to_jpeg(source: &Path, target_bytes: usize) -> Result<Vec<u8>, String> {
    let image = image::open(source).map_err(|_| "INVALID_IMAGE")?;
    let rgb = image.to_rgb8();
    let mut best = Vec::new();
    for quality in [88_u8, 76, 64, 52, 40, 32, 24, 16, 10] {
        let mut bytes = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut bytes, quality);
        encoder
            .encode_image(&rgb)
            .map_err(|_| "INVALID_IMAGE".to_string())?;
        best = bytes;
        if best.len() <= target_bytes {
            break;
        }
    }
    Ok(best)
}

fn reverse_image_request_timeouts() -> ProviderHttpTimeouts {
    ProviderHttpTimeouts {
        response_header: Duration::from_secs(150),
        idle: Duration::from_secs(90),
        total_non_streaming: Duration::from_secs(180),
    }
}

fn connection_failure(message: impl Into<String>) -> CheckAiProviderConnectionResult {
    connection_failure_with_models(message, vec![])
}

fn connection_failure_with_models(
    message: impl Into<String>,
    models: Vec<String>,
) -> CheckAiProviderConnectionResult {
    CheckAiProviderConnectionResult {
        ok: false,
        message: message.into(),
        models,
    }
}

fn connection_probe_model(provider: &AiProvider, models: &[String]) -> Option<String> {
    let default_model = provider.default_model.as_deref().map(str::trim);
    if let Some(model) = default_model.filter(|model| {
        !model.is_empty()
            && (models.is_empty() || models.iter().any(|candidate| candidate == model))
    }) {
        return Some(model.to_string());
    }
    let probed_model = provider.probed_model.as_deref().map(str::trim);
    if let Some(model) = probed_model.filter(|model| {
        !model.is_empty()
            && (models.is_empty() || models.iter().any(|candidate| candidate == model))
    }) {
        return Some(model.to_string());
    }
    models.first().cloned()
}

async fn probe_reverse_image_chat(
    client: &crate::provider_http::ProviderHttpClient,
    endpoint: Url,
    api_key: &str,
    model: &str,
) -> Result<(), String> {
    let request = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "请用一句中文描述这张图片。"
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{REVERSE_IMAGE_PROBE_PNG_BASE64}")
                        }
                    }
                ]
            }
        ]
    });
    let body = client
        .post_json_bounded(
            endpoint,
            api_key,
            request,
            MAX_REVERSE_IMAGE_RESPONSE_BYTES,
            CancellationToken::new(),
        )
        .await?;
    let body = String::from_utf8(body).map_err(|_| "INVALID_PROVIDER_RESPONSE")?;
    parse_chat_completion_prompt(&body)
        .map(|_| ())
        .map_err(|_| "INVALID_PROVIDER_RESPONSE".into())
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
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        thread,
        time::Duration,
    };

    struct TestServer {
        url: Url,
        requests: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        worker: Option<thread::JoinHandle<()>>,
    }

    impl TestServer {
        fn start(handler: impl Fn(&mut TcpStream) + Send + Sync + 'static) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let worker_stop = stop.clone();
            let worker_requests = requests.clone();
            let handler: Arc<dyn Fn(&mut TcpStream) + Send + Sync> = Arc::new(handler);
            let worker = thread::spawn(move || {
                while !worker_stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream.set_nonblocking(false).unwrap();
                            if let Some(body) = read_request_body(&mut stream) {
                                worker_requests.lock().unwrap().push(body);
                                handler(&mut stream);
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(1));
                        }
                        Err(_) => break,
                    }
                }
            });

            Self {
                url,
                requests,
                stop,
                worker: Some(worker),
            }
        }

        fn url(&self) -> Url {
            self.url.clone()
        }

        fn request_bodies(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
        }
    }

    fn read_request_body(stream: &mut TcpStream) -> Option<String> {
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .ok()?;
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = stream.read(&mut buffer).ok()?;
            if read == 0 {
                return None;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let headers_end = bytes.windows(4).position(|window| window == b"\r\n\r\n")?;
        let headers = std::str::from_utf8(&bytes[..headers_end]).ok()?;
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
            })
            .unwrap_or_default();
        let expected_bytes = headers_end + 4 + content_length;
        while bytes.len() < expected_bytes {
            let read = stream.read(&mut buffer).ok()?;
            if read == 0 {
                return None;
            }
            bytes.extend_from_slice(&buffer[..read]);
        }
        String::from_utf8(bytes[headers_end + 4..expected_bytes].to_vec()).ok()
    }

    fn write_json_response(stream: &mut TcpStream, status: u16, body: &str) {
        let response = format!(
            "HTTP/1.1 {status} Test\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }

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

    #[tokio::test]
    async fn reverse_image_connection_probe_posts_a_real_vision_request() {
        let server = TestServer::start(|stream| {
            write_json_response(
                stream,
                200,
                r#"{"choices":[{"message":{"content":"probe ok"}}]}"#,
            );
        });
        let client = crate::provider_http::ProviderHttpClient::new().unwrap();

        probe_reverse_image_chat(
            &client,
            server.url(),
            "test-key",
            "doubao-seed-1-6-vision-250815",
        )
        .await
        .unwrap();

        let bodies = server.request_bodies();
        assert_eq!(bodies.len(), 1);
        let body: serde_json::Value = serde_json::from_str(&bodies[0]).unwrap();
        assert_eq!(body["model"], "doubao-seed-1-6-vision-250815");
        assert_eq!(body["messages"][0]["content"][1]["type"], "image_url");
        assert!(body["messages"][0]["content"][1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));
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

    #[test]
    fn reverse_image_request_data_url_compresses_large_png_to_jpeg() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.png");
        let mut image = image::RgbImage::new(1024, 1024);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = image::Rgb([
                ((x * 31 + y * 17) % 256) as u8,
                ((x * 13 + y * 47) % 256) as u8,
                ((x * 61 + y * 7) % 256) as u8,
            ]);
        }
        image.save(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        assert!(original.len() > REVERSE_IMAGE_INLINE_TARGET_BYTES);

        let data_url = reverse_image_data_url_for_request("images/source.png", &source, &original)
            .expect("large PNG should be compressed for provider request");

        assert!(data_url.starts_with("data:image/jpeg;base64,"));
        let encoded = data_url.strip_prefix("data:image/jpeg;base64,").unwrap();
        let compressed = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        assert!(compressed.len() < original.len());
        assert!(compressed.len() <= REVERSE_IMAGE_INLINE_TARGET_BYTES);
    }
}
