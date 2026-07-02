// src-tauri/src/commands.rs
// IPC 命令：前端通过 src/lib/ipc.ts 调用这些函数。
// 所有系统操作（文件、剪贴板）在此封装，前端不直接碰系统。

use crate::library::{self, Library};
use base64::Engine;
use chrono::{DateTime, Datelike, Local, Timelike};
use image::codecs::jpeg::JpegEncoder;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;

fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("no app data dir")
}

#[tauri::command]
pub fn load_library(app: tauri::AppHandle) -> Library {
    library::load_library(&data_dir(&app))
}

#[tauri::command]
pub fn save_library(app: tauri::AppHandle, library: Library) -> Result<(), String> {
    library::save_library(&data_dir(&app), &library).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_image(app: tauri::AppHandle, bytes: Vec<u8>, ext: String) -> Result<String, String> {
    let dir = data_dir(&app).join("images");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let name = format!("{}.{}", id, ext);
    let path = dir.join(&name);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(format!("images/{}", name))
}

#[tauri::command]
pub fn delete_image(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let full = data_dir(&app).join(&path);
    if full.exists() {
        std::fs::remove_file(&full).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn read_image_bytes(app: tauri::AppHandle, path: String) -> Result<Vec<u8>, String> {
    let full = data_dir(&app).join(&path);
    std::fs::read(&full).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_library(app: tauri::AppHandle, dest: String) -> Result<(), String> {
    let zip_path = std::path::PathBuf::from(&dest);
    library::export_library(&data_dir(&app), &zip_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_library(app: tauri::AppHandle, zip_path: String) -> Result<Library, String> {
    library::import_library(std::path::Path::new(&zip_path), &data_dir(&app))
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFile {
    pub filename: String,
    pub content: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_url: String,
    pub download_url: String,
}

#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GithubReleaseAsset>,
}

#[derive(serde::Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckApiConnectionInput {
    pub base_url: String,
    pub api_key: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckApiConnectionResult {
    pub ok: bool,
    pub message: String,
    pub models: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseImagePromptInput {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub image_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseImagePromptResult {
    pub prompt: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportImageFromPathInput {
    pub source_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressMediaInput {
    pub source_path: String,
    pub target_mb: f64,
    pub output_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestCompressedOutputPathInput {
    pub source_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressMediaResult {
    pub output_path: String,
}

#[derive(serde::Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(serde::Deserialize)]
struct OpenAiModel {
    id: String,
}

#[derive(serde::Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(serde::Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(serde::Deserialize)]
struct ChatCompletionMessage {
    content: String,
}

fn openai_url(base_url: &str, path: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    let base = if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{}/v1", trimmed)
    };
    format!("{}/{}", base, path.trim_start_matches('/'))
}

fn mime_from_path(path: &str) -> &'static str {
    match path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/png",
    }
}

fn parse_chat_completion_prompt(body: &str) -> Result<String, String> {
    let parsed: ChatCompletionResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content.trim().to_string())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| "模型没有返回提示词".to_string())
}

fn release_download_url(release: &GithubRelease) -> String {
    release
        .assets
        .iter()
        .find(|asset| asset.name.ends_with("_x64-setup.exe"))
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.name.ends_with(".exe"))
        })
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.name.ends_with(".msi"))
        })
        .map(|asset| asset.browser_download_url.clone())
        .unwrap_or_else(|| release.html_url.clone())
}

fn timestamp_suffix_from_datetime<Tz: chrono::TimeZone>(datetime: DateTime<Tz>) -> String {
    format!(
        "{:02}{:02}{:02}{:02}",
        datetime.month(),
        datetime.day(),
        datetime.hour(),
        datetime.minute()
    )
}

fn timestamp_suffix_now() -> String {
    timestamp_suffix_from_datetime(Local::now())
}

fn compressed_output_ext(source: &Path) -> &'static str {
    match source
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "mp4" | "mov" | "webm" | "avi" | "mkv" => "mp4",
        _ => "jpg",
    }
}

fn is_video_path(source: &Path) -> bool {
    matches!(
        source
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "mp4" | "mov" | "webm" | "avi" | "mkv"
    )
}

fn compress_image_to_jpeg(source: &Path, output: &Path, target_bytes: u64) -> Result<(), String> {
    let img = image::open(source).map_err(|e| e.to_string())?;
    let rgb = img.to_rgb8();
    let mut best = Vec::new();
    for quality in [88_u8, 76, 64, 52, 40, 32, 24, 16] {
        let mut bytes = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut bytes, quality);
        encoder.encode_image(&rgb).map_err(|e| e.to_string())?;
        best = bytes;
        if best.len() as u64 <= target_bytes {
            break;
        }
    }
    std::fs::write(output, best).map_err(|e| e.to_string())
}

fn video_bitrate_kbps(target_mb: f64, duration_secs: f64, audio_kbps: u32) -> u32 {
    if duration_secs <= 0.0 {
        return 500;
    }
    let total_kbits = target_mb * 1024.0 * 8.0;
    let total_kbps = total_kbits / duration_secs;
    total_kbps.max(audio_kbps as f64 + 100.0).round() as u32 - audio_kbps
}

fn ffprobe_duration_secs(source: &Path) -> Result<f64, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(source)
        .output()
        .map_err(|_| "未找到 FFmpeg/ffprobe，请先安装 FFmpeg 并加入 PATH".to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim()
        .parse::<f64>()
        .map_err(|_| "无法读取视频时长".to_string())
}

fn compress_video_with_ffmpeg(source: &Path, output: &Path, target_mb: f64) -> Result<(), String> {
    let duration = ffprobe_duration_secs(source)?;
    let audio_kbps = 128_u32;
    let video_kbps = video_bitrate_kbps(target_mb, duration, audio_kbps);
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(source)
        .args([
            "-c:v",
            "libx264",
            "-b:v",
            &format!("{}k", video_kbps),
            "-maxrate",
            &format!("{}k", video_kbps),
            "-bufsize",
            &format!("{}k", video_kbps * 2),
            "-c:a",
            "aac",
            "-b:a",
            &format!("{}k", audio_kbps),
            "-movflags",
            "+faststart",
        ])
        .arg(output)
        .status()
        .map_err(|_| "未找到 FFmpeg，请先安装 FFmpeg 并加入 PATH".to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("视频压缩失败，请确认 FFmpeg 可用并重试".to_string())
    }
}

fn compressed_output_path(source: &Path, ext: &str, suffix: &str) -> Result<PathBuf, String> {
    let dir = source
        .parent()
        .ok_or_else(|| "无法识别源文件目录".to_string())?;
    let stem = source
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "无法识别源文件名".to_string())?;
    Ok(dir.join(format!("{}_{}.{}", stem, suffix, ext)))
}

// 读取目录下所有 .md/.txt 文件内容，供前端解析
#[tauri::command]
pub fn read_import_dir(dir: String) -> Result<Vec<ImportFile>, String> {
    let mut files = Vec::new();
    let read = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in read {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "md" || ext == "txt" {
                let filename = entry.file_name().to_string_lossy().to_string();
                let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                files.push(ImportFile { filename, content });
            }
        }
    }
    Ok(files)
}

// 下载远程图片到 images/，返回相对路径
#[tauri::command]
pub fn download_image(app: tauri::AppHandle, url: String) -> Result<String, String> {
    use std::io::Read;
    let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let dir = data_dir(&app).join("images");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ext = url
        .split('?')
        .next()
        .unwrap_or(&url)
        .rsplit('.')
        .next()
        .filter(|s| ["png", "jpg", "jpeg", "webp", "gif"].contains(s))
        .unwrap_or("png")
        .to_string();
    let id = uuid::Uuid::new_v4().simple().to_string();
    let name = format!("{}.{}", id, ext);
    std::fs::write(dir.join(&name), &bytes).map_err(|e| e.to_string())?;
    Ok(format!("images/{}", name))
}

#[tauri::command]
pub fn check_for_update() -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let resp = ureq::get("https://api.github.com/repos/felix1709/banana-box/releases/latest")
        .set("User-Agent", "banana-box")
        .call()
        .map_err(|e| e.to_string())?;
    let body = resp.into_string().map_err(|e| e.to_string())?;
    let release: GithubRelease = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let latest_version = release
        .tag_name
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string();
    let update_available = is_newer_version(&latest_version, &current_version);

    Ok(UpdateCheckResult {
        current_version,
        latest_version,
        update_available,
        download_url: release_download_url(&release),
        release_url: release.html_url,
    })
}

#[tauri::command]
pub fn check_api_connection(input: CheckApiConnectionInput) -> CheckApiConnectionResult {
    if input.base_url.trim().is_empty() {
        return CheckApiConnectionResult {
            ok: false,
            message: "Base URL 不能为空".to_string(),
            models: vec![],
        };
    }
    if input.api_key.trim().is_empty() {
        return CheckApiConnectionResult {
            ok: false,
            message: "API Key 不能为空".to_string(),
            models: vec![],
        };
    }

    let url = openai_url(&input.base_url, "models");
    let response = ureq::get(&url)
        .set("Authorization", &format!("Bearer {}", input.api_key.trim()))
        .set("User-Agent", "banana-box")
        .call();

    match response {
        Ok(resp) => match resp.into_string() {
            Ok(body) => {
                let models = serde_json::from_str::<OpenAiModelsResponse>(&body)
                    .map(|parsed| parsed.data.into_iter().map(|model| model.id).collect())
                    .unwrap_or_else(|_| vec![]);
                CheckApiConnectionResult {
                    ok: true,
                    message: "连接成功".to_string(),
                    models,
                }
            }
            Err(err) => CheckApiConnectionResult {
                ok: false,
                message: format!("读取响应失败：{}", err),
                models: vec![],
            },
        },
        Err(err) => CheckApiConnectionResult {
            ok: false,
            message: format!("连接失败：{}", err),
            models: vec![],
        },
    }
}

#[tauri::command]
pub fn reverse_image_prompt(
    app: tauri::AppHandle,
    input: ReverseImagePromptInput,
) -> Result<ReverseImagePromptResult, String> {
    if input.api_key.trim().is_empty() {
        return Err("API Key 不能为空".to_string());
    }
    if input.model.trim().is_empty() {
        return Err("反推模型不能为空".to_string());
    }

    let full = data_dir(&app).join(&input.image_path);
    let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let data_url = format!(
        "data:{};base64,{}",
        mime_from_path(&input.image_path),
        encoded
    );
    let request = serde_json::json!({
        "model": input.model,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "请根据这张图片反推出适合 AI 绘图或设计复现的中文提示词。只输出提示词本身，不要解释。"
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": data_url
                        }
                    }
                ]
            }
        ]
    });

    let url = openai_url(&input.base_url, "chat/completions");
    let resp = ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", input.api_key.trim()))
        .set("User-Agent", "banana-box")
        .set("Content-Type", "application/json")
        .send_string(&request.to_string())
        .map_err(|e| e.to_string())?;
    let body = resp.into_string().map_err(|e| e.to_string())?;
    Ok(ReverseImagePromptResult {
        prompt: parse_chat_completion_prompt(&body)?,
    })
}

#[tauri::command]
pub fn import_image_from_path(
    app: tauri::AppHandle,
    input: ImportImageFromPathInput,
) -> Result<String, String> {
    let source = PathBuf::from(&input.source_path);
    if !source.exists() {
        return Err("图片文件不存在".to_string());
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| ["png", "jpg", "jpeg", "webp", "gif"].contains(&e.as_str()))
        .unwrap_or_else(|| "png".to_string());
    let dir = data_dir(&app).join("images");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let name = format!("{}.{}", id, ext);
    std::fs::copy(&source, dir.join(&name)).map_err(|e| e.to_string())?;
    Ok(format!("images/{}", name))
}

#[tauri::command]
pub fn compress_media(input: CompressMediaInput) -> Result<CompressMediaResult, String> {
    if input.target_mb <= 0.0 {
        return Err("目标大小必须大于 0 MB".to_string());
    }
    let source = PathBuf::from(&input.source_path);
    if !source.exists() {
        return Err("源文件不存在".to_string());
    }
    let output = PathBuf::from(&input.output_path);
    if is_video_path(&source) {
        compress_video_with_ffmpeg(&source, &output, input.target_mb)?;
    } else {
        let target_bytes = (input.target_mb * 1024.0 * 1024.0).round() as u64;
        compress_image_to_jpeg(&source, &output, target_bytes)?;
    }
    Ok(CompressMediaResult {
        output_path: output.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn suggest_compressed_output_path(
    input: SuggestCompressedOutputPathInput,
) -> Result<String, String> {
    let source = PathBuf::from(&input.source_path);
    let ext = compressed_output_ext(&source);
    let output = compressed_output_path(&source, ext, &timestamp_suffix_now())?;
    Ok(output.to_string_lossy().to_string())
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_parts = version_parts(latest);
    let current_parts = version_parts(current);
    for index in 0..latest_parts.len().max(current_parts.len()) {
        let latest_part = *latest_parts.get(index).unwrap_or(&0);
        let current_part = *current_parts.get(index).unwrap_or(&0);
        if latest_part > current_part {
            return true;
        }
        if latest_part < current_part {
            return false;
        }
    }
    false
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|ch| ch.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
                .unwrap_or(0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn openai_url_join_adds_v1_path_once() {
        assert_eq!(
            openai_url("https://ai.leihuo.netease.com", "models"),
            "https://ai.leihuo.netease.com/v1/models"
        );
        assert_eq!(
            openai_url("https://ai.leihuo.netease.com/v1/", "chat/completions"),
            "https://ai.leihuo.netease.com/v1/chat/completions"
        );
    }

    #[test]
    fn parse_chat_completion_prompt_reads_first_text_content() {
        let body = r#"{
          "choices": [
            {
              "message": {
                "content": "a clean generated prompt"
              }
            }
          ]
        }"#;

        assert_eq!(
            parse_chat_completion_prompt(body).unwrap(),
            "a clean generated prompt"
        );
    }

    #[test]
    fn compressed_output_path_uses_source_folder_and_timestamp_suffix() {
        let source = std::path::Path::new("C:/Users/admin/Desktop/photo.png");
        let output = compressed_output_path(source, "jpg", "06301205").unwrap();

        assert_eq!(
            output.to_string_lossy().replace('\\', "/"),
            "C:/Users/admin/Desktop/photo_06301205.jpg"
        );
    }

    #[test]
    fn timestamp_suffix_uses_month_day_hour_and_minute() {
        let timezone = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        let datetime = timezone.with_ymd_and_hms(2026, 7, 1, 9, 30, 45).unwrap();

        assert_eq!(timestamp_suffix_from_datetime(datetime), "07010930");
    }

    #[test]
    fn video_bitrate_uses_target_size_duration_and_audio_budget() {
        assert_eq!(video_bitrate_kbps(10.0, 10.0, 128), 8064);
    }

    #[test]
    fn release_asset_download_prefers_windows_setup_exe() {
        let release = GithubRelease {
            tag_name: "v0.1.2".to_string(),
            html_url: "https://github.com/felix1709/banana-box/releases/tag/v0.1.2".to_string(),
            assets: vec![
                GithubReleaseAsset {
                    name: "banana-box_0.1.2_x64_en-US.msi".to_string(),
                    browser_download_url:
                        "https://github.com/felix1709/banana-box/releases/download/v0.1.2/banana-box_0.1.2_x64_en-US.msi"
                            .to_string(),
                },
                GithubReleaseAsset {
                    name: "banana-box_0.1.2_x64-setup.exe".to_string(),
                    browser_download_url:
                        "https://github.com/felix1709/banana-box/releases/download/v0.1.2/banana-box_0.1.2_x64-setup.exe"
                            .to_string(),
                },
            ],
        };

        assert_eq!(
            release_download_url(&release),
            "https://github.com/felix1709/banana-box/releases/download/v0.1.2/banana-box_0.1.2_x64-setup.exe"
        );
    }
}
