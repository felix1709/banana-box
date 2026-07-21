use crate::command_auth::MainArgs;
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::Duration,
};
use tauri::Manager;

const PI_WEB_PORT: u16 = 30141;
const PI_WEB_HOST: &str = "127.0.0.1";
const PI_WEB_VERSION: &str = "0.7.16";
const PI_WEB_ARCHIVE: &str = "pi-web-runtime.zip";
const NODE_INSTALL_URL: &str = "https://nodejs.org/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PiWebServiceState {
    MissingRuntime,
    Stopped,
    Checking,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiWebDiagnosticLink {
    label: String,
    url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiWebStatus {
    state: PiWebServiceState,
    url: String,
    port: u16,
    message: String,
    detail: String,
    missing_dependency: String,
    install_links: Vec<PiWebDiagnosticLink>,
    can_start: bool,
    can_open: bool,
    can_stop: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PiWebChatHealthState {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiWebChatHealth {
    state: PiWebChatHealthState,
    message: String,
    detail: String,
    provider: String,
    model_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiWebRepairResult {
    changed: bool,
    message: String,
    detail: String,
}

pub struct PiWebService {
    child: Mutex<Option<Child>>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiWebEmptyCommandArgs {}

impl Default for PiWebService {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
        }
    }
}

impl Drop for PiWebService {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            if let Some(mut process) = child.take() {
                let _ = process.kill();
                let _ = process.wait();
            }
        }
    }
}

impl PiWebStatus {
    fn url(port: u16) -> String {
        format!("http://{PI_WEB_HOST}:{port}")
    }

    fn stopped(port: u16) -> Self {
        Self {
            state: PiWebServiceState::Stopped,
            url: Self::url(port),
            port,
            message: "PI-Web 未启动".into(),
            detail: "点击启动后，Banana Box 会在后台启动 PI-Web，并在独立页面中打开。首次启动可能需要先解压内置运行时，请稍等片刻。".into(),
            missing_dependency: String::new(),
            install_links: Vec::new(),
            can_start: true,
            can_open: false,
            can_stop: false,
        }
    }

    fn running(port: u16) -> Self {
        Self {
            state: PiWebServiceState::Running,
            url: Self::url(port),
            port,
            message: "PI-Web 正在运行".into(),
            detail: "PI-Web 已经在本机启动，可以打开独立页面继续使用。".into(),
            missing_dependency: String::new(),
            install_links: Vec::new(),
            can_start: false,
            can_open: true,
            can_stop: true,
        }
    }

    fn external_running(port: u16, can_stop: bool) -> Self {
        Self {
            state: PiWebServiceState::Running,
            url: Self::url(port),
            port,
            message: "PI-Web 端口已有服务".into(),
            detail: "检测到本机 PI-Web 默认端口已经有服务在运行。可以直接打开；如果这不是 PI-Web，请关闭占用端口的程序后重试。".into(),
            missing_dependency: String::new(),
            install_links: Vec::new(),
            can_start: false,
            can_open: true,
            can_stop,
        }
    }

    fn missing_dependency(name: &str, message: &str) -> Self {
        let install_links = if name == "Node.js" {
            vec![PiWebDiagnosticLink {
                label: "下载 Node.js".into(),
                url: NODE_INSTALL_URL.into(),
            }]
        } else {
            Vec::new()
        };

        Self {
            state: PiWebServiceState::MissingRuntime,
            url: Self::url(PI_WEB_PORT),
            port: PI_WEB_PORT,
            message: message.into(),
            detail: format!("{name} 是 PI-Web 当前运行方式需要的本地环境。安装后重新打开 Banana Box 或点击重新检查。"),
            missing_dependency: name.into(),
            install_links,
            can_start: false,
            can_open: false,
            can_stop: false,
        }
    }

    fn error(message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            state: PiWebServiceState::Error,
            url: Self::url(PI_WEB_PORT),
            port: PI_WEB_PORT,
            message: message.into(),
            detail: detail.into(),
            missing_dependency: String::new(),
            install_links: Vec::new(),
            can_start: true,
            can_open: false,
            can_stop: false,
        }
    }
}

impl PiWebChatHealth {
    fn ok(detail: impl Into<String>, provider: String, model_id: String) -> Self {
        Self {
            state: PiWebChatHealthState::Ok,
            message: "对话检测通过".into(),
            detail: detail.into(),
            provider,
            model_id,
        }
    }

    fn warning(message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            state: PiWebChatHealthState::Warning,
            message: message.into(),
            detail: detail.into(),
            provider: String::new(),
            model_id: String::new(),
        }
    }

    fn error(
        message: impl Into<String>,
        detail: impl Into<String>,
        provider: String,
        model_id: String,
    ) -> Self {
        Self {
            state: PiWebChatHealthState::Error,
            message: message.into(),
            detail: detail.into(),
            provider,
            model_id,
        }
    }
}

impl PiWebService {
    fn status(&self, app: Option<&tauri::AppHandle>) -> Result<PiWebStatus, String> {
        if self.child_is_running()? {
            return Ok(PiWebStatus::running(PI_WEB_PORT));
        }

        if port_accepts_connections(PI_WEB_PORT) {
            return Ok(PiWebStatus::external_running(
                PI_WEB_PORT,
                external_pi_web_process_id(PI_WEB_PORT).is_some(),
            ));
        }

        if app.is_some_and(bundled_pi_web_available) {
            return Ok(PiWebStatus::stopped(PI_WEB_PORT));
        }

        if !command_available(node_command(), &["--version"]) {
            return Ok(PiWebStatus::missing_dependency("Node.js", "缺少 Node.js"));
        }

        if !command_available(npx_command(), &["--version"]) {
            return Ok(PiWebStatus::missing_dependency("Node.js", "缺少 npm/npx"));
        }

        Ok(PiWebStatus::stopped(PI_WEB_PORT))
    }

    async fn start(&self, app: tauri::AppHandle) -> Result<PiWebStatus, String> {
        if self.child_is_running()? {
            let status = PiWebStatus::running(PI_WEB_PORT);
            open_url(&status.url)?;
            return Ok(status);
        }

        let initial_status = self.status(Some(&app))?;
        if initial_status.state == PiWebServiceState::MissingRuntime {
            return Ok(initial_status);
        }
        if initial_status.can_open && !initial_status.can_stop {
            open_url(&initial_status.url)?;
            return Ok(initial_status);
        }

        let mut command = build_launch_command(&app)?;
        let child = command.spawn().map_err(|error| {
            format!(
                "PI_WEB_START_FAILED: {}",
                sanitize_process_error(error.to_string())
            )
        })?;

        {
            let mut guard = self
                .child
                .lock()
                .map_err(|_| "PI_WEB_LOCK_FAILED".to_string())?;
            *guard = Some(child);
        }

        match wait_until_ready(PI_WEB_PORT, Duration::from_secs(45)).await {
            Ok(()) => {
                let status = PiWebStatus::running(PI_WEB_PORT);
                open_url(&status.url)?;
                Ok(status)
            }
            Err(error) => {
                let _ = self.stop();
                Ok(PiWebStatus::error("PI-Web 启动超时", error))
            }
        }
    }

    fn stop(&self) -> Result<PiWebStatus, String> {
        let mut guard = self
            .child
            .lock()
            .map_err(|_| "PI_WEB_LOCK_FAILED".to_string())?;
        if let Some(mut child) = guard.take() {
            terminate_process_tree(child.id());
            let _ = child.wait();
        } else if let Some(process_id) = external_pi_web_process_id(PI_WEB_PORT) {
            terminate_process_tree(process_id);
        }
        Ok(PiWebStatus::stopped(PI_WEB_PORT))
    }

    fn child_is_running(&self) -> Result<bool, String> {
        let mut guard = self
            .child
            .lock()
            .map_err(|_| "PI_WEB_LOCK_FAILED".to_string())?;
        let Some(child) = guard.as_mut() else {
            return Ok(false);
        };

        match child.try_wait().map_err(|error| error.to_string())? {
            Some(_) => {
                *guard = None;
                Ok(false)
            }
            None => Ok(true),
        }
    }
}

#[tauri::command]
pub fn get_pi_web_status(
    app: tauri::AppHandle,
    service: tauri::State<PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebStatus, String> {
    service.status(Some(&app))
}

#[tauri::command]
pub async fn start_pi_web(
    app: tauri::AppHandle,
    service: tauri::State<'_, PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebStatus, String> {
    service.start(app).await
}

#[tauri::command]
pub fn open_pi_web(
    app: tauri::AppHandle,
    service: tauri::State<PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebStatus, String> {
    let status = service.status(Some(&app))?;
    if status.can_open {
        open_url(&status.url)?;
    }
    Ok(status)
}

#[tauri::command]
pub fn stop_pi_web(
    service: tauri::State<PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebStatus, String> {
    service.stop()
}

#[tauri::command]
pub async fn get_pi_web_chat_health(
    app: tauri::AppHandle,
    service: tauri::State<'_, PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebChatHealth, String> {
    let status = service.status(Some(&app))?;
    if !status.can_open {
        return Ok(PiWebChatHealth::warning(
            "PI-Web 尚未运行",
            "请先启动 PI-Web，再检测对话功能。",
        ));
    }

    probe_pi_web_chat_health(PI_WEB_PORT).await
}

#[tauri::command]
pub fn repair_pi_web_model_compatibility(
    service: tauri::State<PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebRepairResult, String> {
    let models_path = pi_models_config_path()?;
    let settings_path = pi_settings_config_path()?;
    let mut models_config: Value = serde_json::from_str(
        &fs::read_to_string(&models_path)
            .map_err(|error| format!("无法读取 PI-Web 模型配置：{error}"))?,
    )
    .map_err(|error| format!("PI-Web 模型配置不是有效 JSON：{error}"))?;
    let settings_config: Value = fs::read_to_string(&settings_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or(Value::Null);

    let provider = settings_config
        .get("defaultProvider")
        .and_then(Value::as_str)
        .unwrap_or("");
    let model_id = settings_config
        .get("defaultModel")
        .and_then(Value::as_str)
        .unwrap_or("");
    let changed = apply_developer_role_compat(&mut models_config, provider, model_id);
    if !changed {
        return Ok(PiWebRepairResult {
            changed: false,
            message: "没有找到可修复的模型配置".into(),
            detail: "请在 PI-Web 的 Models 页面检查当前模型是否为 OpenAI 兼容接口。".into(),
        });
    }

    let content = serde_json::to_string_pretty(&models_config).map_err(|error| error.to_string())?;
    fs::write(&models_path, format!("{content}\n"))
        .map_err(|error| format!("无法写入 PI-Web 模型配置：{error}"))?;
    let _ = service.stop();

    Ok(PiWebRepairResult {
        changed: true,
        message: "已写入兼容配置".into(),
        detail: "已关闭 developer 角色和 reasoning_effort 兼容项，并停止 PI-Web。请重新启动后再检测对话。".into(),
    })
}

fn build_launch_command(app: &tauri::AppHandle) -> Result<Command, String> {
    if let Some((node, script)) = ensure_bundled_pi_web_launch(app)? {
        let mut command = Command::new(node);
        command.arg(script);
        command.args([
            "--no-open",
            "-p",
            &PI_WEB_PORT.to_string(),
            "-H",
            PI_WEB_HOST,
        ]);
        configure_background_command(&mut command);
        return Ok(command);
    }

    let mut command = Command::new(npx_command());
    command.arg("@agegr/pi-web@latest");
    command.args([
        "--no-open",
        "-p",
        &PI_WEB_PORT.to_string(),
        "-H",
        PI_WEB_HOST,
    ]);
    configure_background_command(&mut command);
    Ok(command)
}

fn bundled_pi_web_available(app: &tauri::AppHandle) -> bool {
    bundled_runtime_dir(app)
        .and_then(|dir| find_bundled_pi_web_paths(&dir))
        .is_some()
        || bundled_archive_path(app).is_some_and(|path| path.is_file())
}

fn ensure_bundled_pi_web_launch(
    app: &tauri::AppHandle,
) -> Result<Option<(PathBuf, PathBuf)>, String> {
    let Some(runtime_dir) = bundled_runtime_dir(app) else {
        return Ok(None);
    };

    if let Some(paths) = find_bundled_pi_web_paths(&runtime_dir) {
        return Ok(Some(paths));
    }

    let Some(archive) = bundled_archive_path(app).filter(|path| path.is_file()) else {
        return Ok(None);
    };

    extract_bundled_runtime(&archive, &runtime_dir)?;
    Ok(find_bundled_pi_web_paths(&runtime_dir))
}

fn bundled_runtime_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("pi-web-runtime").join(PI_WEB_VERSION))
}

fn bundled_archive_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    Some(app.path().resource_dir().ok()?.join(PI_WEB_ARCHIVE))
}

fn find_bundled_pi_web_paths(resource_dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let node = resource_dir
        .join("node")
        .join(if cfg!(windows) { "node.exe" } else { "node" });
    let script = resource_dir
        .join("node_modules")
        .join("@agegr")
        .join("pi-web")
        .join("bin")
        .join("pi-web.js");

    (node.is_file() && script.is_file()).then_some((node, script))
}

fn extract_bundled_runtime(archive_path: &Path, runtime_dir: &Path) -> Result<(), String> {
    let parent = runtime_dir
        .parent()
        .ok_or_else(|| "PI_WEB_RUNTIME_PARENT_MISSING".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;

    let temp_dir = parent.join(format!("{PI_WEB_VERSION}.tmp"));
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&temp_dir).map_err(|error| error.to_string())?;

    let extract_result = extract_zip_archive(archive_path, &temp_dir);
    if let Err(error) = extract_result {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(error);
    }

    if runtime_dir.exists() {
        fs::remove_dir_all(runtime_dir).map_err(|error| error.to_string())?;
    }
    fs::rename(&temp_dir, runtime_dir).map_err(|error| error.to_string())?;
    Ok(())
}

fn extract_zip_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let Some(enclosed_name) = entry.enclosed_name() else {
            return Err("PI_WEB_ARCHIVE_UNSAFE_PATH".into());
        };
        let relative_path = strip_archive_dot_prefix(&enclosed_name);
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let output_path = destination.join(relative_path);
        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(|error| error.to_string())?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut output = File::create(&output_path).map_err(|error| error.to_string())?;
            io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn strip_archive_dot_prefix(path: &Path) -> PathBuf {
    path.strip_prefix(".").unwrap_or(path).to_path_buf()
}

fn pi_models_config_path() -> Result<PathBuf, String> {
    Ok(pi_agent_config_dir()?.join("models.json"))
}

fn pi_settings_config_path() -> Result<PathBuf, String> {
    Ok(pi_agent_config_dir()?.join("settings.json"))
}

fn pi_agent_config_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "无法定位用户主目录。".to_string())?;
    Ok(home.join(".pi").join("agent"))
}

fn apply_developer_role_compat(config: &mut Value, provider: &str, model_id: &str) -> bool {
    let Some(providers) = config.get_mut("providers").and_then(Value::as_object_mut) else {
        return false;
    };

    if let Some(provider_config) = providers.get_mut(provider) {
        set_compat_flag(provider_config);
        return true;
    }

    for provider_config in providers.values_mut() {
        let Some(models) = provider_config.get_mut("models").and_then(Value::as_array_mut) else {
            continue;
        };
        if let Some(model) = models
            .iter_mut()
            .find(|model| model.get("id").and_then(Value::as_str) == Some(model_id))
        {
            set_compat_flag(model);
            return true;
        }
    }

    for provider_config in providers.values_mut() {
        if provider_config.get("api").and_then(Value::as_str) == Some("openai-completions") {
            set_compat_flag(provider_config);
            return true;
        }
    }

    false
}

fn set_compat_flag(target: &mut Value) {
    if !target.get("compat").is_some_and(Value::is_object) {
        target["compat"] = json!({});
    }
    target["compat"]["supportsDeveloperRole"] = Value::Bool(false);
    target["compat"]["supportsReasoningEffort"] = Value::Bool(false);
}

async fn probe_pi_web_chat_health(port: u16) -> Result<PiWebChatHealth, String> {
    let base_url = PiWebStatus::url(port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|error| error.to_string())?;

    let cwd = fetch_default_cwd(&client, &base_url).await?;
    let (provider, model_id) = fetch_default_model(&client, &base_url, &cwd).await?;
    let session_id = create_health_session(&client, &base_url, &cwd).await?;
    let event_response = client
        .get(format!("{base_url}/api/agent/{session_id}/events"))
        .send()
        .await
        .map_err(|error| format!("无法连接 PI-Web 事件流：{error}"))?;

    if !event_response.status().is_success() {
        return Ok(PiWebChatHealth::error(
            "事件流连接失败",
            format!("PI-Web 事件流返回 HTTP {}", event_response.status()),
            provider,
            model_id,
        ));
    }

    let prompt_response = client
        .post(format!("{base_url}/api/agent/{session_id}"))
        .json(&json!({
            "type": "prompt",
            "message": "你好，请只回复 OK"
        }))
        .send()
        .await
        .map_err(|error| format!("无法发送 PI-Web 测试消息：{error}"))?;

    if !prompt_response.status().is_success() {
        return Ok(PiWebChatHealth::error(
            "测试消息发送失败",
            format!("PI-Web 返回 HTTP {}", prompt_response.status()),
            provider,
            model_id,
        ));
    }

    read_health_events(event_response, provider, model_id).await
}

async fn fetch_default_cwd(client: &reqwest::Client, base_url: &str) -> Result<String, String> {
    let response = client
        .post(format!("{base_url}/api/default-cwd"))
        .json(&json!({}))
        .send()
        .await
        .map_err(|error| format!("无法读取 PI-Web 默认目录：{error}"))?;
    let body: Value = response.json().await.map_err(|error| error.to_string())?;
    body.get("cwd")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "PI-Web 没有返回默认工作目录。".to_string())
}

async fn fetch_default_model(
    client: &reqwest::Client,
    base_url: &str,
    cwd: &str,
) -> Result<(String, String), String> {
    let response = client
        .get(format!("{base_url}/api/models"))
        .query(&[("cwd", cwd)])
        .send()
        .await
        .map_err(|error| format!("无法读取 PI-Web 模型配置：{error}"))?;
    let body: Value = response.json().await.map_err(|error| error.to_string())?;
    let default_model = body.get("defaultModel").unwrap_or(&Value::Null);
    let provider = default_model
        .get("provider")
        .and_then(Value::as_str)
        .or_else(|| {
            body.get("modelList")
                .and_then(Value::as_array)
                .and_then(|models| models.first())
                .and_then(|model| model.get("provider"))
                .and_then(Value::as_str)
        })
        .unwrap_or("")
        .to_string();
    let model_id = default_model
        .get("modelId")
        .and_then(Value::as_str)
        .or_else(|| {
            body.get("modelList")
                .and_then(Value::as_array)
                .and_then(|models| models.first())
                .and_then(|model| model.get("id"))
                .and_then(Value::as_str)
        })
        .unwrap_or("")
        .to_string();
    Ok((provider, model_id))
}

async fn create_health_session(
    client: &reqwest::Client,
    base_url: &str,
    cwd: &str,
) -> Result<String, String> {
    let response = client
        .post(format!("{base_url}/api/agent/new"))
        .json(&json!({
            "cwd": cwd,
            "type": "ensure_session"
        }))
        .send()
        .await
        .map_err(|error| format!("无法创建 PI-Web 测试会话：{error}"))?;

    let body: Value = response.json().await.map_err(|error| error.to_string())?;
    body.get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "PI-Web 没有返回测试会话 ID。".to_string())
}

async fn read_health_events(
    response: reqwest::Response,
    provider: String,
    model_id: String,
) -> Result<PiWebChatHealth, String> {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut saw_assistant_text = false;

    loop {
        let next_chunk = tokio::time::timeout(Duration::from_secs(20), stream.next()).await;
        let Some(chunk) = (match next_chunk {
            Ok(chunk) => chunk,
            Err(_) => {
                return Ok(PiWebChatHealth::warning(
                    "对话检测超时",
                    "PI-Web 已收到测试消息，但 20 秒内没有返回模型结果。请检查模型配置或网络。",
                ));
            }
        }) else {
            break;
        };

        let chunk = chunk.map_err(|error| error.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(index) = buffer.find("\n\n") {
            let block = buffer[..index].to_string();
            buffer = buffer[index + 2..].to_string();
            if let Some(event) = parse_sse_json(&block) {
                if let Some(error) = extract_event_error(&event) {
                    let message = if error.contains("developer is not one of") {
                        "模型接口不兼容"
                    } else if error.contains("internal_server_error") || error.contains("<500>") {
                        "模型服务内部错误"
                    } else {
                        "模型调用失败"
                    };
                    return Ok(PiWebChatHealth::error(
                        message,
                        error,
                        provider,
                        model_id,
                    ));
                }
                if assistant_text_present(&event) {
                    saw_assistant_text = true;
                }
                if event.get("type").and_then(Value::as_str) == Some("prompt_done") {
                    if saw_assistant_text {
                        return Ok(PiWebChatHealth::ok(
                            "PI-Web 可以正常收到模型回复。",
                            provider,
                            model_id,
                        ));
                    }
                    return Ok(PiWebChatHealth::warning(
                        "未收到模型正文",
                        "PI-Web 完成了测试流程，但没有收到助手文本。请打开 PI-Web 查看模型配置。",
                    ));
                }
            }
        }
    }

    Ok(PiWebChatHealth::warning(
        "事件流提前结束",
        "PI-Web 的事件流在返回模型结果前结束了。请重新检测一次。",
    ))
}

fn parse_sse_json(block: &str) -> Option<Value> {
    block.lines().find_map(|line| {
        line.strip_prefix("data:")
            .and_then(|data| serde_json::from_str(data.trim()).ok())
    })
}

fn extract_event_error(event: &Value) -> Option<String> {
    event
        .get("errorMessage")
        .and_then(Value::as_str)
        .or_else(|| {
            event
                .get("message")
                .and_then(|message| message.get("errorMessage"))
                .and_then(Value::as_str)
        })
        .map(str::to_string)
}

fn assistant_text_present(event: &Value) -> bool {
    let Some(message) = event.get("message") else {
        return false;
    };
    if message.get("role").and_then(Value::as_str) != Some("assistant") {
        return false;
    }

    match message.get("content") {
        Some(Value::String(text)) => !text.trim().is_empty(),
        Some(Value::Array(items)) => items.iter().any(|item| {
            item.get("type").and_then(Value::as_str) == Some("text")
                && item
                    .get("text")
                    .and_then(Value::as_str)
                    .is_some_and(|text| !text.trim().is_empty())
        }),
        _ => false,
    }
}

fn configure_background_command(command: &mut Command) {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}

fn command_available(program: &str, args: &[&str]) -> bool {
    let mut command = Command::new(program);
    command.args(args);
    configure_background_command(&mut command);
    command.status().is_ok_and(|status| status.success())
}

fn external_pi_web_process_id(port: u16) -> Option<u32> {
    platform_external_pi_web_process_id(port)
}

#[cfg(windows)]
fn platform_external_pi_web_process_id(port: u16) -> Option<u32> {
    let script = format!(
        r#"
$connection = Get-NetTCPConnection -LocalPort {port} -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $connection) {{ exit 1 }}
$listenerPid = [int]$connection.OwningProcess
$currentPid = $listenerPid
$killPid = $null
for ($i = 0; $i -lt 10 -and $currentPid; $i++) {{
  $process = Get-CimInstance Win32_Process -Filter "ProcessId = $currentPid" -ErrorAction SilentlyContinue
  if (-not $process) {{ break }}
  $commandLine = [string]$process.CommandLine
  if ($commandLine -match '@agegr[\\/]+pi-web|pi-web\.js|npx.*@agegr/pi-web') {{
    $killPid = [int]$process.ProcessId
  }} elseif (-not $killPid -and $commandLine -match 'next.*start.*{port}') {{
    $killPid = $listenerPid
  }}
  $currentPid = [int]$process.ParentProcessId
}}
if ($killPid) {{ Write-Output $killPid; exit 0 }}
exit 1
"#
    );

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().parse().ok())
        .flatten()
}

#[cfg(not(windows))]
fn platform_external_pi_web_process_id(_port: u16) -> Option<u32> {
    None
}

fn terminate_process_tree(process_id: u32) {
    platform_terminate_process_tree(process_id);
}

#[cfg(windows)]
fn platform_terminate_process_tree(process_id: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(windows))]
fn platform_terminate_process_tree(process_id: u32) {
    let _ = Command::new("kill")
        .args(["-TERM", &process_id.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn port_accepts_connections(port: u16) -> bool {
    let Ok(address) = format!("{PI_WEB_HOST}:{port}").parse::<SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&address, Duration::from_millis(180)).is_ok()
}

fn node_command() -> &'static str {
    "node"
}

fn npx_command() -> &'static str {
    if cfg!(windows) {
        "npx.cmd"
    } else {
        "npx"
    }
}

fn open_url(url: &str) -> Result<(), String> {
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| error.to_string())
}

fn sanitize_process_error(error: String) -> String {
    error.replace(['\r', '\n'], " ")
}

async fn wait_until_ready(port: u16, timeout: Duration) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + timeout;
    let url = PiWebStatus::url(port);

    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("{url} 在 {} 秒内没有响应。", timeout.as_secs()));
        }

        if let Ok(response) = reqwest::get(&url).await {
            if response.status().is_success() {
                return Ok(());
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn status_flags_match_state() {
        let stopped = PiWebStatus::stopped(30141);
        assert!(stopped.can_start);
        assert!(!stopped.can_open);
        assert!(!stopped.can_stop);

        let running = PiWebStatus::running(30141);
        assert!(!running.can_start);
        assert!(running.can_open);
        assert!(running.can_stop);
    }

    #[test]
    fn missing_node_status_has_install_link() {
        let status = PiWebStatus::missing_dependency("Node.js", "缺少 Node.js");
        assert_eq!(status.state, PiWebServiceState::MissingRuntime);
        assert_eq!(status.missing_dependency, "Node.js");
        assert!(status
            .install_links
            .iter()
            .any(|link| link.url.contains("nodejs.org")));
    }

    #[test]
    fn unknown_external_running_status_can_open_but_not_stop() {
        let status = PiWebStatus::external_running(30141, false);

        assert_eq!(status.state, PiWebServiceState::Running);
        assert!(status.can_open);
        assert!(!status.can_start);
        assert!(!status.can_stop);
    }

    #[test]
    fn pi_web_external_running_status_can_open_and_stop() {
        let status = PiWebStatus::external_running(30141, true);

        assert_eq!(status.state, PiWebServiceState::Running);
        assert!(status.can_open);
        assert!(!status.can_start);
        assert!(status.can_stop);
    }

    #[test]
    fn chat_health_reports_developer_role_incompatibility() {
        let event = json!({
            "type": "message_end",
            "message": {
                "role": "assistant",
                "errorMessage": "400: developer is not one of ['system', 'assistant', 'user']"
            }
        });

        assert_eq!(
            extract_event_error(&event).as_deref(),
            Some("400: developer is not one of ['system', 'assistant', 'user']")
        );
    }

    #[test]
    fn model_repair_adds_provider_level_developer_role_compat() {
        let mut config = json!({
            "providers": {
                "雷火": {
                    "api": "openai-completions",
                    "models": [
                        { "id": "glm-5.2", "api": "openai-completions" }
                    ]
                }
            }
        });

        assert!(apply_developer_role_compat(&mut config, "雷火", "glm-5.2"));
        assert_eq!(
            config["providers"]["雷火"]["compat"]["supportsDeveloperRole"],
            Value::Bool(false)
        );
        assert_eq!(
            config["providers"]["雷火"]["compat"]["supportsReasoningEffort"],
            Value::Bool(false)
        );
    }

    #[test]
    fn bundled_runtime_requires_node_and_pi_web_script() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_bundled_pi_web_paths(dir.path()).is_none());

        let node = dir
            .path()
            .join("node")
            .join(if cfg!(windows) { "node.exe" } else { "node" });
        let script = dir
            .path()
            .join("node_modules")
            .join("@agegr")
            .join("pi-web")
            .join("bin")
            .join("pi-web.js");
        std::fs::create_dir_all(node.parent().unwrap()).unwrap();
        std::fs::write(&node, b"node").unwrap();
        assert!(find_bundled_pi_web_paths(dir.path()).is_none());

        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, b"pi-web").unwrap();
        let (resolved_node, resolved_script) = find_bundled_pi_web_paths(dir.path()).unwrap();

        assert_eq!(resolved_node, node);
        assert_eq!(resolved_script, script);
    }

    #[test]
    fn zip_extraction_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("unsafe.zip");
        let file = File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("../bad.txt", options).unwrap();
        writer.write_all(b"bad").unwrap();
        writer.finish().unwrap();

        let result = extract_zip_archive(&archive_path, &dir.path().join("out"));
        assert!(result.is_err());
    }

    #[test]
    fn zip_extraction_handles_dot_prefixed_entries() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("runtime.zip");
        let file = File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        writer.add_directory("./node/", options).unwrap();
        writer.start_file("./node/node.exe", options).unwrap();
        writer.write_all(b"node").unwrap();
        writer.finish().unwrap();

        let out_dir = dir.path().join("out");
        extract_zip_archive(&archive_path, &out_dir).unwrap();

        assert!(out_dir.join("node").join("node.exe").is_file());
    }
}
