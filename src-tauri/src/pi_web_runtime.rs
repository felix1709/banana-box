//! PI-Web 外部依赖模式：一键下载、更新与清理。
//!
//! 这是纯新增能力：安装包内置的 PI-Web 运行时（resources/pi-web-runtime.zip）
//! 完全不改动，本模块只负责把 npm 上的 @agegr/pi-web 安装到用户数据目录，
//! 并让启动逻辑优先使用这份外部副本。

use crate::command_auth::MainArgs;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

const PI_WEB_PROGRESS_EVENT: &str = "pi-web-progress";
const RUNTIME_ROOT_DIR: &str = "pi-web-runtime";
const MANAGED_DIR_NAME: &str = "current";
const PREVIOUS_DIR_NAME: &str = "current.previous";
const STAGING_DIR_NAME: &str = "staging";
const TOOLS_DIR_NAME: &str = "tools";
const MANIFEST_FILE_NAME: &str = "installed.json";
const LOCATION_FILE_NAME: &str = "location.json";
const LOG_DIR_NAME: &str = "logs";
const LOG_FILE_NAME: &str = "pi-web-install.log";
const DEFAULT_NPM_REGISTRY: &str = "https://mirrors.cloud.tencent.com/npm/";
const FALLBACK_NPM_REGISTRIES: [&str; 2] = [
    "https://registry.npmmirror.com/",
    "https://registry.npmjs.org/",
];
const DEFAULT_BASE_URL: &str = "https://ai.leihuo.netease.com/v1";
/// 解压后约 1.05GB，再给 npm 缓存和换版本留出余量。
const REQUIRED_FREE_BYTES: u64 = 1_600 * 1024 * 1024;
const EXPECTED_INSTALL_BYTES: u64 = 1_100 * 1024 * 1024;
const MAX_NPM_TARBALL_BYTES: usize = 40 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 4 * 1024 * 1024;
const INSTALL_TIMEOUT: Duration = Duration::from_secs(60 * 40);
const PROGRESS_POLL_INTERVAL: Duration = Duration::from_millis(1500);
const METADATA_TIMEOUT: Duration = Duration::from_secs(6);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);
/// Windows 上刚写完的大目录会被杀软或索引服务短暂占用，目录改名/删除可能偶发失败，
/// 所以这类文件操作统一退避重试几次再放弃。
const FILE_OP_RETRY_ATTEMPTS: u32 = 12;
const FILE_OP_RETRY_BASE_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiWebRuntimeStatus {
    /// managed | custom | bundled | none
    source: String,
    /// notInstalled | ready | updateAvailable | error
    state: String,
    installed: bool,
    version: String,
    latest_version: String,
    install_dir: String,
    node_path: String,
    npm_path: String,
    can_install: bool,
    can_update: bool,
    can_clean: bool,
    message: String,
    detail: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PiWebProgressEvent {
    operation_id: String,
    phase: String,
    progress: u8,
    message: String,
    detail: Option<String>,
    level: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PiWebInstallManifest {
    version: String,
    installed_at: String,
    registry: String,
    node_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PiWebLocation {
    directory: Option<String>,
    updated_at: String,
}

#[derive(Default)]
pub struct PiWebRuntimeService {
    installing: AtomicBool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiWebRuntimeArgs {
    #[serde(default)]
    operation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiWebRuntimeDirectoryArgs {
    #[serde(default)]
    directory: Option<String>,
}

fn runtime_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(RUNTIME_ROOT_DIR))
        .map_err(|error| error.to_string())
}

fn node_file_name() -> &'static str {
    if cfg!(windows) {
        "node.exe"
    } else {
        "node"
    }
}

fn pi_web_package_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("node_modules").join("@agegr").join("pi-web")
}

fn pi_web_script_path(install_dir: &Path) -> PathBuf {
    pi_web_package_dir(install_dir).join("bin").join("pi-web.js")
}

fn read_pi_web_version(install_dir: &Path) -> Option<String> {
    let raw = fs::read_to_string(pi_web_package_dir(install_dir).join("package.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value.get("version")?.as_str().map(ToString::to_string)
}

/// 目录里是否有一份完整、可以直接启动的 PI-Web。
fn pi_web_install_is_usable(install_dir: &Path) -> bool {
    pi_web_script_path(install_dir).is_file()
        && pi_web_package_dir(install_dir).join(".next").is_dir()
        && install_dir
            .join("node_modules")
            .join("next")
            .join("dist")
            .join("bin")
            .join("next")
            .is_file()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn custom_install_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    let root = runtime_root(app).ok()?;
    let location: PiWebLocation = read_json(&root.join(LOCATION_FILE_NAME))?;
    let dir = PathBuf::from(location.directory?);
    pi_web_install_is_usable(&dir).then_some(dir)
}

fn managed_install_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    let dir = runtime_root(app).ok()?.join(MANAGED_DIR_NAME);
    pi_web_install_is_usable(&dir).then_some(dir)
}

/// 当前应该被启动的那一份 PI-Web（优先用户手动指定，其次一键下载的）。
fn active_install_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    custom_install_dir(app).or_else(|| managed_install_dir(app))
}

/// 给启动逻辑用：外部安装副本的 node 与脚本路径。
pub(crate) fn managed_pi_web_launch(app: &tauri::AppHandle) -> Option<(PathBuf, PathBuf)> {
    let install_dir = active_install_dir(app)?;
    let script = pi_web_script_path(&install_dir);
    if !script.is_file() {
        return None;
    }
    let node = find_node(app)?;
    Some((node, script))
}

fn resource_pi_web_dirs(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let Ok(resource_dir) = app.path().resource_dir() else {
        return Vec::new();
    };
    vec![
        resource_dir.join("pi-web"),
        resource_dir.join("resources").join("pi-web"),
    ]
}

fn which_node() -> Option<PathBuf> {
    let name = node_file_name();
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// 只做查找，不产生任何副作用。
fn find_node(app: &tauri::AppHandle) -> Option<PathBuf> {
    for dir in resource_pi_web_dirs(app) {
        let candidate = dir.join("node").join(node_file_name());
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if let Ok(root) = runtime_root(app) {
        let tools = root.join(TOOLS_DIR_NAME).join("node").join(node_file_name());
        if tools.is_file() {
            return Some(tools);
        }
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("node").join(node_file_name());
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    which_node()
}

fn resolve_usable_install_dir(input: &Path) -> Option<PathBuf> {
    let mut current = Some(input.to_path_buf());
    for _ in 0..4 {
        let dir = current?;
        if pi_web_install_is_usable(&dir) {
            return Some(dir);
        }
        current = dir.parent().map(Path::to_path_buf);
    }
    None
}

fn find_npm_cli(app: &tauri::AppHandle, node: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for dir in resource_pi_web_dirs(app) {
        candidates.push(dir.join("npm").join("bin").join("npm-cli.js"));
        candidates.push(
            dir.join("node_modules")
                .join("npm")
                .join("bin")
                .join("npm-cli.js"),
        );
    }
    if let Some(node_dir) = node.parent() {
        candidates.push(
            node_dir
                .join("node_modules")
                .join("npm")
                .join("bin")
                .join("npm-cli.js"),
        );
    }
    if let Ok(root) = runtime_root(app) {
        candidates.push(
            root.join(TOOLS_DIR_NAME)
                .join("npm")
                .join("package")
                .join("bin")
                .join("npm-cli.js"),
        );
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn node_search_path(node: &Path) -> Option<String> {
    let dir = node.parent()?;
    let existing = std::env::var_os("PATH").unwrap_or_default();
    let mut parts = vec![dir.to_path_buf()];
    parts.extend(std::env::split_paths(&existing));
    std::env::join_paths(parts)
        .ok()
        .map(|value| value.to_string_lossy().to_string())
}

fn registry_candidates() -> Vec<String> {
    let mut list = vec![DEFAULT_NPM_REGISTRY.to_string()];
    for entry in FALLBACK_NPM_REGISTRIES {
        if !list.iter().any(|item| item == entry) {
            list.push(entry.to_string());
        }
    }
    list.into_iter()
        .map(|mut value| {
            if !value.ends_with('/') {
                value.push('/');
            }
            value
        })
        .collect()
}

fn registry_label(registry: &str) -> String {
    url::Url::parse(registry)
        .ok()
        .and_then(|url| url.host_str().map(ToString::to_string))
        .unwrap_or_else(|| registry.to_string())
}

fn human_size(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = MB * 1024.0;
    let value = bytes as f64;
    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else {
        format!("{:.0} MB", value / MB)
    }
}

fn now_stamp() -> String {
    chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn version_is_newer(candidate: &str, current: &str) -> bool {
    if candidate.trim().is_empty() {
        return false;
    }
    if current.trim().is_empty() {
        return true;
    }
    let parse = |value: &str| -> Vec<u64> {
        value
            .split(['.', '-', '+'])
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let left = parse(candidate);
    let right = parse(current);
    for index in 0..left.len().max(right.len()) {
        let a = left.get(index).copied().unwrap_or(0);
        let b = right.get(index).copied().unwrap_or(0);
        if a != b {
            return a > b;
        }
    }
    false
}

fn directory_size(root: &Path) -> u64 {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(|meta| meta.is_file())
        .map(|meta| meta.len())
        .sum()
}

fn append_log(app: &tauri::AppHandle, line: &str) {
    let Ok(root) = runtime_root(app) else {
        return;
    };
    let dir = root.join(LOG_DIR_NAME);
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(LOG_FILE_NAME))
    else {
        return;
    };
    let _ = writeln!(file, "[{}] {line}", now_stamp());
}

fn emit(
    app: &tauri::AppHandle,
    operation_id: Option<&str>,
    phase: &str,
    progress: u8,
    message: impl Into<String>,
    detail: Option<String>,
    level: &str,
) {
    let Some(operation_id) = operation_id.filter(|value| !value.trim().is_empty()) else {
        return;
    };
    let _ = app.emit(
        PI_WEB_PROGRESS_EVENT,
        PiWebProgressEvent {
            operation_id: operation_id.to_string(),
            phase: phase.to_string(),
            progress,
            message: message.into(),
            detail,
            level: level.to_string(),
        },
    );
}

fn extract_zip_entry(
    archive: &mut zip::ZipArchive<fs::File>,
    name: &str,
    target: &Path,
) -> Result<bool, String> {
    let mut entry = match archive.by_name(name) {
        Ok(entry) => entry,
        Err(_) => return Ok(false),
    };
    if !entry.is_file() {
        return Ok(false);
    }
    let mut output = fs::File::create(target).map_err(|error| error.to_string())?;
    std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    Ok(true)
}

/// 从安装包内置的 zip 里只取出 node.exe，不必解压整个 300MB 压缩包。
fn extract_bundled_node(app: &tauri::AppHandle, target: &Path) -> Result<(), String> {
    if target.is_file() {
        return Ok(());
    }
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    let archive_path = crate::pi_web::find_bundled_archive(&resource_dir)
        .ok_or_else(|| "PI_WEB_RUNTIME_MISSING".to_string())?;
    let file = fs::File::open(&archive_path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    for name in ["./node/node.exe", "node/node.exe"] {
        if extract_zip_entry(&mut archive, name, target)? {
            return Ok(());
        }
    }
    Err("PI_WEB_RUNTIME_MISSING".to_string())
}

fn ensure_node(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Some(node) = find_node(app) {
        return Ok(node);
    }
    let root = runtime_root(app)?;
    let target = root
        .join(TOOLS_DIR_NAME)
        .join("node")
        .join(node_file_name());
    extract_bundled_node(app, &target)?;
    Ok(target)
}

fn extract_tar_gz(bytes: &[u8], destination: &Path) -> Result<(), String> {
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive.entries().map_err(|error| error.to_string())?;
    for entry in entries {
        let mut entry = entry.map_err(|error| error.to_string())?;
        let unpacked = entry
            .unpack_in(destination)
            .map_err(|error| error.to_string())?;
        if !unpacked {
            return Err("PI_WEB_NPM_TARBALL_UNSAFE".to_string());
        }
    }
    Ok(())
}

fn http_client(timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(timeout)
        .build()
        .map_err(|error| error.to_string())
}

async fn download_bytes(
    client: &reqwest::Client,
    url: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .header("User-Agent", "banana-box")
        .send()
        .await
        .map_err(|_| "PI_WEB_DOWNLOAD_FAILED".to_string())?;
    if !response.status().is_success() {
        return Err("PI_WEB_DOWNLOAD_FAILED".to_string());
    }
    let declared = response.content_length().unwrap_or(0);
    if declared > max_bytes as u64 {
        return Err("PI_WEB_DOWNLOAD_TOO_LARGE".to_string());
    }
    let mut buffer: Vec<u8> = Vec::with_capacity(declared.min(max_bytes as u64) as usize);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "PI_WEB_DOWNLOAD_FAILED".to_string())?;
        if buffer.len() + chunk.len() > max_bytes {
            return Err("PI_WEB_DOWNLOAD_TOO_LARGE".to_string());
        }
        buffer.extend_from_slice(&chunk);
    }
    Ok(buffer)
}

/// 区分「这个镜像没有」和「整台机器连不上网」，避免离线时把三个镜像都轮询一遍。
enum MetadataOutcome {
    Found(String),
    Missing,
    Unreachable,
}

async fn fetch_latest_pi_web_version(
    client: &reqwest::Client,
    registry: &str,
) -> MetadataOutcome {
    let url = format!("{registry}@agegr/pi-web/latest");
    let response = match client.get(&url).header("User-Agent", "banana-box").send().await {
        Ok(response) => response,
        Err(_) => return MetadataOutcome::Unreachable,
    };
    if !response.status().is_success() {
        return MetadataOutcome::Missing;
    }
    let Ok(bytes) = download_bytes(client, &url, MAX_METADATA_BYTES).await else {
        return MetadataOutcome::Missing;
    };
    match serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|value| {
            value
                .get("version")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
        }) {
        Some(version) => MetadataOutcome::Found(version),
        None => MetadataOutcome::Missing,
    }
}

async fn fetch_npm_tarball_url(client: &reqwest::Client, registry: &str) -> Option<String> {
    let url = format!("{registry}npm/latest");
    let bytes = download_bytes(client, &url, MAX_METADATA_BYTES).await.ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value
        .get("dist")
        .and_then(|dist| dist.get("tarball"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

/// 系统里没有 npm 时，从镜像下载 npm 官方压缩包解压备用（约 3MB）。
async fn ensure_npm_cli(
    app: &tauri::AppHandle,
    node: &Path,
    operation_id: Option<&str>,
) -> Result<PathBuf, String> {
    if let Some(path) = find_npm_cli(app, node) {
        return Ok(path);
    }

    let root = runtime_root(app)?;
    let tools_npm = root.join(TOOLS_DIR_NAME).join("npm");
    let cli = tools_npm.join("package").join("bin").join("npm-cli.js");
    if cli.is_file() {
        return Ok(cli);
    }

    emit(
        app,
        operation_id,
        "prepare",
        10,
        "正在准备 npm 工具",
        Some("首次使用需要下载约 3MB 的 npm 工具包".to_string()),
        "info",
    );

    let client = http_client(METADATA_TIMEOUT)?;
    let mut tarball_url = None;
    for registry in registry_candidates() {
        if let Some(url) = fetch_npm_tarball_url(&client, &registry).await {
            tarball_url = Some(url);
            break;
        }
    }
    let tarball_url = tarball_url.ok_or_else(|| "PI_WEB_NPM_BOOTSTRAP_FAILED".to_string())?;

    let download_client = http_client(DOWNLOAD_TIMEOUT)?;
    let bytes = download_bytes(&download_client, &tarball_url, MAX_NPM_TARBALL_BYTES).await?;
    if tools_npm.exists() {
        fs::remove_dir_all(&tools_npm).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&tools_npm).map_err(|error| error.to_string())?;
    extract_tar_gz(&bytes, &tools_npm)?;

    if !cli.is_file() {
        return Err("PI_WEB_NPM_BOOTSTRAP_FAILED".to_string());
    }
    Ok(cli)
}

fn spawn_log_pump<R: std::io::Read + Send + 'static>(
    app: tauri::AppHandle,
    operation_id: Option<String>,
    reader: R,
    progress: Arc<AtomicU8>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else {
                break;
            };
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            append_log(&app, &line);
            emit(
                &app,
                operation_id.as_deref(),
                "download",
                progress.load(Ordering::Acquire),
                line,
                None,
                "info",
            );
        }
    })
}

fn write_package_json(staging: &Path, range: &str) -> Result<(), String> {
    let payload = serde_json::json!({
        "private": true,
        "dependencies": { "@agegr/pi-web": range },
    });
    let content = serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?;
    fs::write(staging.join("package.json"), format!("{content}\n"))
        .map_err(|error| error.to_string())
}

fn npm_install_blocking(
    app: &tauri::AppHandle,
    operation_id: Option<&str>,
    node: &Path,
    npm_cli: &Path,
    staging: &Path,
    registry: &str,
) -> Result<(), String> {
    let progress = Arc::new(AtomicU8::new(18));
    let stop = Arc::new(AtomicBool::new(false));

    let poller = {
        let app = app.clone();
        let operation_id = operation_id.map(str::to_string);
        let staging = staging.to_path_buf();
        let progress = Arc::clone(&progress);
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            while !stop.load(Ordering::Acquire) {
                thread::sleep(PROGRESS_POLL_INTERVAL);
                if stop.load(Ordering::Acquire) {
                    break;
                }
                let written = directory_size(&staging);
                let ratio = (written.saturating_mul(56) / EXPECTED_INSTALL_BYTES).min(56) as u8;
                let value = 18 + ratio;
                let current = progress.fetch_max(value, Ordering::AcqRel).max(value);
                emit(
                    &app,
                    operation_id.as_deref(),
                    "download",
                    current,
                    format!("正在下载 PI-WEB：已写入 {}", human_size(written)),
                    None,
                    "info",
                );
            }
        })
    };

    let mut command = Command::new(node);
    command
        .arg(npm_cli)
        .arg("install")
        .arg("--prefix")
        .arg(staging)
        .args(["--omit=dev", "--no-audit", "--no-fund", "--loglevel=notice"])
        .arg(format!("--registry={registry}"))
        .current_dir(staging)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::pi_web::configure_background_command(&mut command);
    if let Some(path) = node_search_path(node) {
        command.env("PATH", path);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            stop.store(true, Ordering::Release);
            let _ = poller.join();
            return Err(format!("PI_WEB_NPM_SPAWN_FAILED: {error}"));
        }
    };

    let out_thread = child.stdout.take().map(|stdout| {
        spawn_log_pump(
            app.clone(),
            operation_id.map(str::to_string),
            stdout,
            Arc::clone(&progress),
        )
    });
    let err_thread = child.stderr.take().map(|stderr| {
        spawn_log_pump(
            app.clone(),
            operation_id.map(str::to_string),
            stderr,
            Arc::clone(&progress),
        )
    });

    let deadline = Instant::now() + INSTALL_TIMEOUT;
    let status = loop {
        if let Ok(Some(status)) = child.try_wait() {
            break Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            break Err("PI_WEB_INSTALL_TIMEOUT".to_string());
        }
        thread::sleep(Duration::from_millis(400));
    };

    stop.store(true, Ordering::Release);
    if let Some(handle) = out_thread {
        let _ = handle.join();
    }
    if let Some(handle) = err_thread {
        let _ = handle.join();
    }
    let _ = poller.join();

    let status = status?;
    if status.success() {
        Ok(())
    } else {
        Err("PI_WEB_NPM_INSTALL_FAILED".to_string())
    }
}

fn retry_file_operation<F>(mut operation: F) -> std::io::Result<()>
where
    F: FnMut() -> std::io::Result<()>,
{
    let mut last_error = None;
    for attempt in 0..FILE_OP_RETRY_ATTEMPTS {
        match operation() {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                thread::sleep(FILE_OP_RETRY_BASE_DELAY * (attempt + 1).min(4));
            }
        }
    }
    Err(last_error.expect("重试循环至少执行一次"))
}

/// 新版本完全装好之后才做切换：先改名成 current.previous，失败时自动回滚。
fn activate_install(root: &Path, staging: &Path) -> Result<(), String> {
    let target = root.join(MANAGED_DIR_NAME);
    let backup = root.join(PREVIOUS_DIR_NAME);
    if backup.exists() {
        retry_file_operation(|| fs::remove_dir_all(&backup)).map_err(|error| error.to_string())?;
    }
    if target.exists() {
        retry_file_operation(|| fs::rename(&target, &backup)).map_err(|error| error.to_string())?;
    }
    match retry_file_operation(|| fs::rename(staging, &target)) {
        Ok(()) => {
            let _ = retry_file_operation(|| fs::remove_dir_all(&backup));
            Ok(())
        }
        Err(error) => {
            if backup.exists() {
                let _ = retry_file_operation(|| fs::rename(&backup, &target));
            }
            Err(format!("PI_WEB_ACTIVATE_FAILED: {error}"))
        }
    }
}

fn write_manifest(root: &Path, manifest: &PiWebInstallManifest) -> Result<(), String> {
    let content = serde_json::to_string_pretty(manifest).map_err(|error| error.to_string())?;
    fs::write(root.join(MANIFEST_FILE_NAME), format!("{content}\n"))
        .map_err(|error| error.to_string())
}

/// 复用用户已经在「设置 - API 设置」里填好的 Key，自动写好 PI-WEB 配置。
fn auto_configure(app: &tauri::AppHandle) -> Result<bool, String> {
    let Some(services) = app.try_state::<crate::app_state::AppServices>() else {
        return Err("STARTUP_NOT_READY".to_string());
    };

    let mut resolved = None;
    for provider_id in ["reverse-image", "storyboard"] {
        if let Ok(found) = services.providers.resolve_for_request(provider_id) {
            if !found.api_key.trim().is_empty() {
                resolved = Some(found);
                break;
            }
        }
    }
    let Some(resolved) = resolved else {
        return Err("没有找到已保存的 API Key，请在下方手动填写后点击一键配置。".to_string());
    };

    let base_url = if resolved.provider.base_url.trim().is_empty() {
        DEFAULT_BASE_URL.to_string()
    } else {
        resolved.provider.base_url.clone()
    };

    crate::pi_web::auto_repair_pi_agent_config(&resolved.api_key, &base_url)
}

async fn perform_install(
    app: &tauri::AppHandle,
    operation_id: Option<&str>,
) -> Result<String, String> {
    let root = runtime_root(app)?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;

    emit(app, operation_id, "check", 4, "正在检查运行环境", None, "info");
    let node = ensure_node(app)?;
    append_log(app, &format!("使用 node：{}", node.display()));
    let npm_cli = ensure_npm_cli(app, &node, operation_id).await?;
    append_log(app, &format!("使用 npm：{}", npm_cli.display()));

    let client = http_client(METADATA_TIMEOUT)?;
    let registries = registry_candidates();
    let mut target_version = String::new();
    let mut reachable = false;
    for registry in &registries {
        match fetch_latest_pi_web_version(&client, registry).await {
            MetadataOutcome::Found(version) => {
                target_version = version;
                reachable = true;
                break;
            }
            MetadataOutcome::Missing => {
                reachable = true;
                continue;
            }
            MetadataOutcome::Unreachable => continue,
        }
    }
    if !reachable {
        return Err("无法连接到 PI-WEB 下载源，请检查网络后重试。".to_string());
    }
    let range = if target_version.trim().is_empty() {
        "latest".to_string()
    } else {
        target_version.clone()
    };

    let existing = managed_install_dir(app)
        .map(|dir| directory_size(&dir))
        .unwrap_or(0);
    let needed = REQUIRED_FREE_BYTES + existing;
    let free = fs2::available_space(&root).map_err(|error| error.to_string())?;
    if free < needed {
        return Err(format!(
            "磁盘空间不足：需要约 {}，当前可用 {}。请清理磁盘后重试。",
            human_size(needed),
            human_size(free)
        ));
    }

    emit(
        app,
        operation_id,
        "prepare",
        14,
        "正在准备下载目录",
        Some(format!("目标版本 {range}")),
        "info",
    );

    let staging = root.join(STAGING_DIR_NAME);
    let mut installed_version = String::new();
    let mut used_registry = String::new();
    let mut last_error = "PI_WEB_INSTALL_FAILED".to_string();

    for registry in &registries {
        if staging.exists() {
            if let Err(error) = retry_file_operation(|| fs::remove_dir_all(&staging)) {
                append_log(app, &format!("旧下载目录未能清理（{error}），继续重新下载。"));
            }
        }
        fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
        write_package_json(&staging, &range)?;

        emit(
            app,
            operation_id,
            "download",
            18,
            format!("正在从 {} 下载 PI-WEB", registry_label(registry)),
            Some(registry.clone()),
            "info",
        );

        match npm_install_blocking(app, operation_id, &node, &npm_cli, &staging, registry) {
            Ok(()) if pi_web_install_is_usable(&staging) => {
                installed_version = read_pi_web_version(&staging).unwrap_or_else(|| range.clone());
                used_registry = registry.clone();
                last_error.clear();
                break;
            }
            Ok(()) => {
                last_error = "PI_WEB_INSTALL_INCOMPLETE".to_string();
                append_log(app, "下载结果不完整，准备换一个镜像重试。");
            }
            Err(error) => {
                last_error = error;
            }
        }
    }

    if !last_error.is_empty() {
        append_log(app, &format!("安装失败：{last_error}"));
        let _ = fs::remove_dir_all(&staging);
        return Err(format!(
            "PI-WEB 下载失败（{last_error}）。请检查网络后重试；详细日志见数据目录 logs/pi-web-install.log。"
        ));
    }

    emit(
        app,
        operation_id,
        "verify",
        78,
        "正在校验下载结果",
        Some(format!("@agegr/pi-web {installed_version}")),
        "info",
    );

    emit(app, operation_id, "activate", 86, "正在切换到最新版本", None, "info");
    activate_install(&root, &staging)?;

    write_manifest(
        &root,
        &PiWebInstallManifest {
            version: installed_version.clone(),
            installed_at: now_stamp(),
            registry: used_registry,
            node_path: node.display().to_string(),
        },
    )?;

    emit(app, operation_id, "config", 92, "正在写入 PI-WEB 配置", None, "info");
    match auto_configure(app) {
        Ok(true) => append_log(app, "已自动写入雷火 API 配置。"),
        Ok(false) => append_log(app, "检测到已有可用配置，保持不变。"),
        Err(error) => {
            append_log(app, &format!("自动配置未完成：{error}"));
            emit(
                app,
                operation_id,
                "config",
                95,
                "自动配置未完成，可在下方手动一键配置",
                Some(error),
                "info",
            );
        }
    }

    emit(
        app,
        operation_id,
        "done",
        100,
        format!("PI-WEB {installed_version} 已就绪"),
        None,
        "success",
    );
    Ok(installed_version)
}

fn build_status(
    source: &str,
    install_dir: Option<&Path>,
    version: String,
    latest_version: String,
    node: Option<&Path>,
    npm: Option<&Path>,
    bundled_available: bool,
) -> PiWebRuntimeStatus {
    let installed = source != "none";
    let can_update =
        matches!(source, "managed" | "bundled") && version_is_newer(&latest_version, &version);
    let state = if !installed {
        "notInstalled"
    } else if can_update {
        "updateAvailable"
    } else {
        "ready"
    };

    let install_dir_label = install_dir
        .map(|dir| dir.display().to_string())
        .unwrap_or_default();

    let (message, detail) = match source {
        "custom" => (
            "正在使用手动指定的 PI-WEB".to_string(),
            format!("目录：{install_dir_label}。这一份由你自己维护，程序不会自动更新或清理。"),
        ),
        "managed" => (
            format!("PI-WEB {version} 已就绪"),
            "这一份由 Banana Box 一键下载维护，可以随时点击「更新 PI-WEB」升级到最新版。".to_string(),
        ),
        "bundled" => (
            format!("当前使用安装包内置版本 {version}"),
            format!(
                "检测到最新版本 {latest_version}，点击「更新 PI-WEB」即可升级到外部安装的最新版；内置文件不会被删除。"
            ),
        ),
        _ => (
            "尚未安装 PI-WEB".to_string(),
            "点击「一键下载配置」会自动下载最新版 PI-WEB、写入 API 配置并完成校验，全程不需要手动操作。".to_string(),
        ),
    };

    PiWebRuntimeStatus {
        source: source.to_string(),
        state: state.to_string(),
        installed,
        version,
        latest_version,
        install_dir: install_dir_label,
        node_path: node.map(|path| path.display().to_string()).unwrap_or_default(),
        npm_path: npm.map(|path| path.display().to_string()).unwrap_or_default(),
        can_install: !installed && (node.is_some() || bundled_available),
        can_update,
        can_clean: source == "managed",
        message,
        detail,
    }
}

async fn collect_status(app: &tauri::AppHandle) -> PiWebRuntimeStatus {
    let node = find_node(app);
    let npm = node.as_deref().and_then(|node| find_npm_cli(app, node));
    let bundled_available = crate::pi_web::bundled_pi_web_available(app);

    let (source, install_dir, version) = if let Some(dir) = custom_install_dir(app) {
        let version = read_pi_web_version(&dir).unwrap_or_default();
        ("custom", Some(dir), version)
    } else if let Some(dir) = managed_install_dir(app) {
        let version = read_pi_web_version(&dir).unwrap_or_default();
        ("managed", Some(dir), version)
    } else if bundled_available {
        ("bundled", None, crate::pi_web::PI_WEB_VERSION.to_string())
    } else {
        ("none", None, String::new())
    };

    let latest_version = match http_client(METADATA_TIMEOUT) {
        Ok(client) => {
            let mut latest = String::new();
            for registry in registry_candidates() {
                match fetch_latest_pi_web_version(&client, &registry).await {
                    MetadataOutcome::Found(version) => {
                        latest = version;
                        break;
                    }
                    MetadataOutcome::Missing => continue,
                    MetadataOutcome::Unreachable => break,
                }
            }
            latest
        }
        Err(_) => String::new(),
    };

    build_status(
        source,
        install_dir.as_deref(),
        version,
        latest_version,
        node.as_deref(),
        npm.as_deref(),
        bundled_available,
    )
}

#[tauri::command]
pub async fn get_pi_web_runtime_status(
    app: tauri::AppHandle,
    _args: MainArgs<PiWebRuntimeArgs>,
) -> Result<PiWebRuntimeStatus, String> {
    Ok(collect_status(&app).await)
}

#[tauri::command]
pub async fn install_pi_web_runtime(
    app: tauri::AppHandle,
    service: tauri::State<'_, PiWebRuntimeService>,
    args: MainArgs<PiWebRuntimeArgs>,
) -> Result<PiWebRuntimeStatus, String> {
    let operation_id = args.0.operation_id;
    if service.installing.swap(true, Ordering::SeqCst) {
        return Err("PI_WEB_INSTALL_IN_PROGRESS".to_string());
    }
    let outcome = perform_install(&app, operation_id.as_deref()).await;
    service.installing.store(false, Ordering::SeqCst);

    if let Err(error) = outcome {
        append_log(&app, &format!("安装中断：{error}"));
        emit(
            &app,
            operation_id.as_deref(),
            "error",
            100,
            error.clone(),
            None,
            "error",
        );
        return Err(error);
    }

    Ok(collect_status(&app).await)
}

#[tauri::command]
pub async fn set_pi_web_runtime_directory(
    app: tauri::AppHandle,
    args: MainArgs<PiWebRuntimeDirectoryArgs>,
) -> Result<PiWebRuntimeStatus, String> {
    let root = runtime_root(&app)?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let path = root.join(LOCATION_FILE_NAME);

    match args
        .0
        .directory
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(directory) => {
            let resolved = resolve_usable_install_dir(Path::new(directory)).ok_or_else(|| {
                "PI_WEB_DIRECTORY_NOT_USABLE：这个目录里没有找到完整的 PI-WEB（需要包含 node_modules/@agegr/pi-web）。"
                    .to_string()
            })?;
            let location = PiWebLocation {
                directory: Some(resolved.display().to_string()),
                updated_at: now_stamp(),
            };
            let content =
                serde_json::to_string_pretty(&location).map_err(|error| error.to_string())?;
            fs::write(&path, format!("{content}\n")).map_err(|error| error.to_string())?;
        }
        None => {
            if path.is_file() {
                fs::remove_file(&path).map_err(|error| error.to_string())?;
            }
        }
    }

    Ok(collect_status(&app).await)
}

#[tauri::command]
pub async fn clean_pi_web_runtime(
    app: tauri::AppHandle,
    args: MainArgs<PiWebRuntimeArgs>,
) -> Result<PiWebRuntimeStatus, String> {
    let root = runtime_root(&app)?;
    for name in [MANAGED_DIR_NAME, PREVIOUS_DIR_NAME, STAGING_DIR_NAME] {
        let path = root.join(name);
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|error| error.to_string())?;
        }
    }
    if root.join(MANIFEST_FILE_NAME).is_file() {
        let _ = fs::remove_file(root.join(MANIFEST_FILE_NAME));
    }
    emit(
        &app,
        args.0.operation_id.as_deref(),
        "clean",
        100,
        "已清理一键下载的 PI-WEB",
        Some("安装包内置的旧版和用户配置都没有改动。".to_string()),
        "success",
    );
    Ok(collect_status(&app).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_usable_install(install: &Path) {
        let package = pi_web_package_dir(install);
        fs::create_dir_all(package.join("bin")).unwrap();
        fs::create_dir_all(package.join(".next")).unwrap();
        fs::create_dir_all(install.join("node_modules").join("next").join("dist").join("bin"))
            .unwrap();
        fs::write(pi_web_script_path(install), "// pi-web").unwrap();
        fs::write(
            install
                .join("node_modules")
                .join("next")
                .join("dist")
                .join("bin")
                .join("next"),
            "// next",
        )
        .unwrap();
        fs::write(
            package.join("package.json"),
            r#"{ "name": "@agegr/pi-web", "version": "0.9.0" }"#,
        )
        .unwrap();
    }

    #[test]
    fn version_compare_treats_missing_current_as_updatable() {
        assert!(version_is_newer("0.9.0", ""));
        assert!(!version_is_newer("", "0.7.16"));
        assert!(!version_is_newer("", ""));
    }

    #[test]
    fn version_compare_orders_numeric_segments() {
        assert!(version_is_newer("0.9.0", "0.7.16"));
        assert!(version_is_newer("0.10.0", "0.9.0"));
        assert!(!version_is_newer("0.7.16", "0.7.16"));
        assert!(!version_is_newer("0.7.1", "0.7.16"));
    }

    #[test]
    fn a_directory_without_pi_web_is_not_usable() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!pi_web_install_is_usable(dir.path()));
        assert!(resolve_usable_install_dir(dir.path()).is_none());
    }

    #[test]
    fn usable_install_directory_is_detected_from_a_parent_selection() {
        let dir = tempfile::tempdir().unwrap();
        let install = dir.path().join("pi-web");
        make_usable_install(&install);

        assert!(pi_web_install_is_usable(&install));
        assert_eq!(read_pi_web_version(&install).as_deref(), Some("0.9.0"));
        assert_eq!(
            resolve_usable_install_dir(&install.join("node_modules")).as_deref(),
            Some(install.as_path())
        );
    }

    #[test]
    fn registry_candidates_always_fall_back_and_end_with_a_slash() {
        let registries = registry_candidates();
        assert_eq!(registries[0], DEFAULT_NPM_REGISTRY);
        assert!(registries.len() >= 3);
        assert!(registries.iter().all(|value| value.ends_with('/')));
        assert_eq!(
            registry_label("https://mirrors.cloud.tencent.com/npm/"),
            "mirrors.cloud.tencent.com"
        );
    }

    #[test]
    fn human_size_switches_to_gigabytes() {
        assert_eq!(human_size(1_100 * 1024 * 1024), "1.07 GB");
        assert_eq!(human_size(320 * 1024 * 1024), "320 MB");
    }

    #[test]
    fn status_flags_describe_the_not_installed_case() {
        let status = build_status(
            "none",
            None,
            String::new(),
            "0.9.0".into(),
            Some(Path::new("C:\\node.exe")),
            None,
            false,
        );
        assert_eq!(status.state, "notInstalled");
        assert!(!status.installed);
        assert!(status.can_install);
        assert!(!status.can_update);
        assert!(!status.can_clean);
    }

    #[test]
    fn status_flags_mark_the_bundled_copy_as_updatable() {
        let status = build_status(
            "bundled",
            None,
            "0.7.16".into(),
            "0.9.0".into(),
            None,
            None,
            true,
        );
        assert_eq!(status.state, "updateAvailable");
        assert!(status.installed);
        assert!(status.can_update);
        assert!(!status.can_clean);
    }

    #[test]
    fn status_flags_keep_a_custom_directory_read_only() {
        let status = build_status(
            "custom",
            Some(Path::new("D:\\pi-web")),
            "0.9.0".into(),
            "0.9.0".into(),
            None,
            None,
            true,
        );
        assert_eq!(status.state, "ready");
        assert!(!status.can_update);
        assert!(!status.can_clean);
        assert!(!status.can_install);
    }

    #[test]
    fn a_managed_install_that_is_behind_is_reported_as_updatable() {
        let status = build_status(
            "managed",
            Some(Path::new("D:\\current")),
            "0.7.16".into(),
            "0.9.0".into(),
            None,
            None,
            true,
        );
        assert_eq!(status.state, "updateAvailable");
        assert!(status.can_update);
        assert!(status.can_clean);
    }

    #[test]
    fn zip_entry_extraction_reports_a_missing_entry_instead_of_failing() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("sample.zip");
        {
            let file = fs::File::create(&archive_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer
                .start_file("node/node.exe", zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"fake-node").unwrap();
            writer.finish().unwrap();
        }

        let file = fs::File::open(&archive_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let target = dir.path().join("out").join("node.exe");
        fs::create_dir_all(target.parent().unwrap()).unwrap();

        assert!(!extract_zip_entry(&mut archive, "./missing.txt", &target).unwrap());
        assert!(extract_zip_entry(&mut archive, "node/node.exe", &target).unwrap());
        assert_eq!(fs::read(&target).unwrap(), b"fake-node");
    }

    #[test]
    fn tar_extraction_writes_the_npm_layout() {
        let dir = tempfile::tempdir().unwrap();
        let mut tar_bytes = Vec::new();
        {
            let encoder =
                flate2::write::GzEncoder::new(&mut tar_bytes, flate2::Compression::default());
            let mut builder = tar::Builder::new(encoder);
            let mut header = tar::Header::new_gnu();
            header.set_size(1);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "package/bin/npm-cli.js", &b"/"[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }

        extract_tar_gz(&tar_bytes, dir.path()).unwrap();
        assert!(dir
            .path()
            .join("package")
            .join("bin")
            .join("npm-cli.js")
            .is_file());
    }

    #[test]
    fn retry_file_operation_retries_until_the_lock_is_gone() {
        let mut attempts = 0;
        let result = retry_file_operation(|| {
            attempts += 1;
            if attempts < 3 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "busy",
                ))
            } else {
                Ok(())
            }
        });

        assert!(result.is_ok());
        assert_eq!(attempts, 3);
    }

    #[test]
    fn activate_install_swaps_the_managed_directory() {
        let root = tempfile::tempdir().unwrap();
        let staging = root.path().join(STAGING_DIR_NAME);
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("marker.txt"), "new").unwrap();

        activate_install(root.path(), &staging).unwrap();

        let current = root.path().join(MANAGED_DIR_NAME);
        assert_eq!(fs::read_to_string(current.join("marker.txt")).unwrap(), "new");
        assert!(!staging.exists());
        assert!(!root.path().join(PREVIOUS_DIR_NAME).exists());
    }
}
