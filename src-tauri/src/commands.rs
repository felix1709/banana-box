pub(crate) mod backup_commands;
pub(crate) mod provider_commands;
pub(crate) mod startup_commands;

// src-tauri/src/commands.rs
// IPC 命令：前端通过 src/lib/ipc.ts 调用这些函数。
// 所有系统操作（文件、剪贴板）在此封装，前端不直接碰系统。

use crate::{
    app_state::{AppServices, StartupGate},
    cloud_config::{
        load_cloud_config as load_cloud_config_from_db,
        load_cloud_runtime_config as load_cloud_runtime_config_from_db,
        save_cloud_config as save_cloud_config_to_db,
        CloudConfigDto,
        CloudRuntimeConfigDto,
        SaveCloudConfigInput,
    },
    command_auth::MainArgs,
    library::{self, Library},
};
use chrono::{DateTime, Datelike, Local, Timelike};
use futures_util::StreamExt;
use image::codecs::jpeg::JpegEncoder;
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
use std::process::Command;
use tauri::{Emitter, Manager};
use tokio_util::sync::CancellationToken;
use url::Url;

const MAX_DOWNLOADED_IMAGE_BYTES: usize = 15 * 1024 * 1024;
const MAX_UPDATE_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_FFMPEG_ARCHIVE_BYTES: usize = 220 * 1024 * 1024;
const APP_SERVICES_UNAVAILABLE: &str = "STARTUP_NOT_READY";
const FFMPEG_DOWNLOAD_URL: &str = "https://ffmpeg.org/download.html";
const FFMPEG_WINDOWS_ESSENTIALS_ZIP_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
const DEPTH_VIDEO_ENGINE_ENV: &str = "BANANA_BOX_DEPTH_VIDEO_ENGINE";
const DEPTH_VIDEO_ENGINE_COMMAND: &str = "banana-depth-video";

pub(crate) fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("no app data dir")
}

fn require_startup_ready(gate: &StartupGate) -> Result<(), String> {
    gate.require_ready()
}

#[tauri::command]
pub fn load_library(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
) -> Result<Library, String> {
    require_startup_ready(&gate)?;
    Ok(library::load_library(&data_dir(&app)))
}

#[tauri::command]
pub fn save_library(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    library: Library,
) -> Result<(), String> {
    require_startup_ready(&gate)?;
    library::save_library(&data_dir(&app), &library).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_cloud_config(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    _args: MainArgs<LoadCloudConfigCommandArgs>,
) -> Result<CloudConfigDto, String> {
    require_startup_ready(&gate)?;
    let services = app
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    load_cloud_config_from_db(&services.database)
}

#[tauri::command]
pub fn load_cloud_runtime_config(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    _args: MainArgs<LoadCloudConfigCommandArgs>,
) -> Result<CloudRuntimeConfigDto, String> {
    require_startup_ready(&gate)?;
    let services = app
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    load_cloud_runtime_config_from_db(&services.database)
}

#[tauri::command]
pub fn save_cloud_config(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    args: MainArgs<SaveCloudConfigCommandArgs>,
) -> Result<CloudConfigDto, String> {
    require_startup_ready(&gate)?;
    let services = app
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    save_cloud_config_to_db(&services.database, args.0.input)
}

#[tauri::command]
pub fn copy_to_clipboard(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    text: String,
) -> Result<(), String> {
    require_startup_ready(&gate)?;
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_image(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    bytes: Vec<u8>,
    ext: String,
) -> Result<String, String> {
    require_startup_ready(&gate)?;
    let dir = data_dir(&app).join("images");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let name = format!("{}.{}", id, ext);
    let path = dir.join(&name);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(format!("images/{}", name))
}

#[tauri::command]
pub fn delete_image(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    path: String,
) -> Result<(), String> {
    require_startup_ready(&gate)?;
    let full = data_dir(&app).join(&path);
    if full.exists() {
        std::fs::remove_file(&full).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn read_image_bytes(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    path: String,
) -> Result<Vec<u8>, String> {
    require_startup_ready(&gate)?;
    let full = data_dir(&app).join(&path);
    std::fs::read(&full).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_library(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    dest: String,
) -> Result<(), String> {
    require_startup_ready(&gate)?;
    let zip_path = std::path::PathBuf::from(&dest);
    library::export_library(&data_dir(&app), &zip_path).map_err(|e| e.to_string())
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
pub struct MediaToolOperationInput {
    pub operation_id: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthVideoInput {
    pub source_path: String,
    pub output_path: String,
    pub engine_path: Option<String>,
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

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegSetupResult {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub bin_dir: String,
    pub message: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaToolProgressEvent {
    pub operation_id: String,
    pub tool: String,
    pub phase: String,
    pub progress: u8,
    pub message: String,
    pub detail: Option<String>,
    pub level: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestDepthVideoOutputPathInput {
    pub source_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepthVideoResult {
    pub output_path: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DepthVideoEngineSetupResult {
    pub engine_path: String,
    pub engine_dir: String,
    pub message: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DepthVideoPythonSetupResult {
    pub python_version: String,
    pub message: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveCloudConfigCommandArgs {
    input: SaveCloudConfigInput,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoadCloudConfigCommandArgs {}

#[derive(serde::Deserialize)]
pub(crate) struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(serde::Deserialize)]
pub(crate) struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(serde::Deserialize)]
pub(crate) struct ChatCompletionMessage {
    content: ChatCompletionContent,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum ChatCompletionContent {
    Text(String),
    Parts(Vec<ChatCompletionContentPart>),
}

#[derive(serde::Deserialize)]
pub(crate) struct ChatCompletionContentPart {
    text: Option<String>,
}

pub(crate) fn reverse_image_prompt_instruction() -> &'static str {
    r#"请根据这张图片反推出适合 AI 绘图或设计复现的中文提示词。请严格按下面十个维度输出结构化长模板，所有内容都围绕图片本身描述；无法从图片判断的项目写“未明显体现”。不要输出解释、寒暄、免责声明或模板外的额外内容。

一、基础画面属性（生成的底层框架）
- 画幅比例：横版/竖版/正方形，具体比例（如16:9、3:4、2:3）
- 视图布局：单视图/三视图/多视图拼接，视图排列方式与间距
- 画面大类：人像/风光/静物/创意合成，画面核心题材定位

二、核心主体信息（画面核心内容）
- 身份特征：人物的性别、年龄感、五官特点、发型发色、身形特征；物体的品类、数量、形态状态
- 姿态神态：身体朝向、面部朝向、肢体动作幅度、表情细节、眼神状态
- 穿着配饰：服装款式、面料品类、主辅色、配饰细节（如领结、首饰）

三、构图与镜头语言（摄影技术框架）
- 景别：面部特写/胸像/中景/全景/全身景
- 镜头视角：标准镜头/长焦/广角，平视/仰视/俯视，透视强弱
- 景深对焦：景深深浅、清晰对焦位置、背景虚化程度、虚实过渡节奏
- 空间层次：前景/中景/远景的排布，主体在画面中的位置，构图逻辑（三分法/对称/框架式）

四、光影体系（写实感核心）
- 光源属性：自然光（窗光/天光/逆光）/人造影棚光/环境漫射光
- 光位光质：主光方向（顺光/侧光/侧逆光/伦勃朗光）、光线软硬（柔光/硬光/漫射光）
- 光影特征：明暗对比强度、阴影过渡软硬、高光形态、是否有特殊光影（光斑/丁达尔/投影纹理）
- 整体影调：亮调/中间调/暗调，画面整体明暗基调

五、色彩与色调（氛围底色）
- 整体色调：冷调/暖调/中性调，画面主色调与辅助色
- 色彩质感：饱和度高低（高饱和/低饱和/莫兰迪）、通透度、色彩浓郁度
- 色彩风格：胶片色/日系清透/德系厚重/原生直出感
- 主要色彩：给出前5种按照占比排列的准确或近似准确的 HEX 色值

六、材质与质感（精致度与真实感）
- 皮肤质感：毛孔清晰度、血色感、细纹/瑕疵保留度、通透度、是否有特殊肤质表现（如泪膜、眼下阴影）
- 毛发质感：发丝清晰度、蓬松度、毛躁感、光泽度、发丝边缘虚实
- 物体面料：布料纹理褶皱、金属反光、水体折射、石材肌理等各类材质的真实度表现
- 整体质感：超写实质感/胶片颗粒感/柔焦朦胧感/磨砂哑光感

七、环境与背景（场景支撑）
- 背景类型：纯色背景/实景背景/重度虚化背景
- 场景空间：室内/室外/水下/自然场景等具体空间属性
- 氛围元素：环境道具、烘托元素（如气泡、尘埃、植物、光斑）
- 背景与主体的关联：衬托方式、空间互动感、干扰元素多少

八、风格与情绪调性（画面灵魂）
- 摄影风格：大师级人像/时尚大片/纪实摄影/复古胶片/创意摄影
- 整体氛围：清冷/温柔/静谧/热烈/疏离/治愈/肃穆
- 情绪传递：主体传递的情绪、画面整体给人的感受基调

九、特殊效果与细节（还原度加分项）
- 画面特效：动态模糊、柔焦朦胧、光晕眩光、胶片颗粒、色散、暗角
- 微观细节：水珠、毛絮、尘埃、皮肤纹理、衣物细微褶皱等微元素
- 专属特征：画面独有的视觉标识（如水下折射、玻璃反光、雨丝、烟雾）

十、反向约束维度（负面提示词依据）
- 画面原生瑕疵：跑焦、过曝死白、暗部死黑、明显色散、畸变、噪点过多
- AI常见通病：肢体变形、面部崩坏、塑料假肤、穿模、多余肢体、比例失调
- 需规避元素：文字、水印、logo、多余杂物、不符合场景的违和元素"#
}

pub(crate) fn mime_from_path(path: &str) -> &'static str {
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

pub(crate) fn parse_chat_completion_prompt(body: &str) -> Result<String, String> {
    let parsed: ChatCompletionResponse = serde_json::from_str(body).map_err(|e| e.to_string())?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| match choice.message.content {
            ChatCompletionContent::Text(content) => content.trim().to_string(),
            ChatCompletionContent::Parts(parts) => parts
                .into_iter()
                .filter_map(|part| part.text.map(|text| text.trim().to_string()))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        })
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

fn emit_media_tool_progress(
    app: &tauri::AppHandle,
    operation_id: Option<&str>,
    tool: &str,
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
        "media-tool-progress",
        MediaToolProgressEvent {
            operation_id: operation_id.to_string(),
            tool: tool.to_string(),
            phase: phase.to_string(),
            progress,
            message: message.into(),
            detail,
            level: level.to_string(),
        },
    );
}

fn ffmpeg_tool_filename(tool_name: &str) -> String {
    if cfg!(windows) {
        format!("{tool_name}.exe")
    } else {
        tool_name.to_string()
    }
}

fn managed_ffmpeg_bin_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("ffmpeg").join("bin")
}

fn default_ffmpeg_bin_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|path| path.join("ffmpeg").join("bin"))
    } else {
        None
    }
}

fn resolve_ffmpeg_tool(
    tool_name: &str,
    managed_bin_dir: Option<&Path>,
    common_bin_dir: Option<&Path>,
) -> PathBuf {
    if let Some(dir) = managed_bin_dir {
        let candidate = dir.join(ffmpeg_tool_filename(tool_name));
        if candidate.exists() {
            return candidate;
        }
    }
    if let Some(dir) = common_bin_dir {
        let candidate = dir.join(ffmpeg_tool_filename(tool_name));
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(tool_name)
}

fn ffmpeg_tool_path(tool_name: &str, data_dir: Option<&Path>) -> PathBuf {
    let managed_bin_dir = data_dir.map(managed_ffmpeg_bin_dir);
    resolve_ffmpeg_tool(
        tool_name,
        managed_bin_dir.as_deref(),
        default_ffmpeg_bin_dir().as_deref(),
    )
}

fn ffmpeg_missing_message(tool_name: &str) -> String {
    format!(
        "未找到 {tool_name}，请安装完整 FFmpeg（需要 ffmpeg 和 ffprobe）并加入 PATH。下载地址：{FFMPEG_DOWNLOAD_URL}"
    )
}

fn configure_hidden_child_process(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}

fn find_child_file(root: &Path, filename: &str) -> Result<PathBuf, String> {
    let expected = filename.to_ascii_lowercase();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name == expected {
            return Ok(entry.path().to_path_buf());
        }
    }
    Err(format!("FFMPEG_ARCHIVE_MISSING_TOOL: {filename}"))
}

fn extract_zip_safely(bytes: &[u8], destination: &Path) -> Result<(), String> {
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|error| error.to_string())?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        let enclosed = file
            .enclosed_name()
            .ok_or_else(|| "FFMPEG_ARCHIVE_UNSAFE_PATH".to_string())?
            .to_path_buf();
        let output_path = destination.join(enclosed);
        if file.is_dir() {
            fs::create_dir_all(&output_path).map_err(|error| error.to_string())?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = fs::File::create(&output_path).map_err(|error| error.to_string())?;
        std::io::copy(&mut file, &mut output).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn verify_ffmpeg_tool(path: &Path) -> Result<(), String> {
    let mut command = Command::new(path);
    configure_hidden_child_process(&mut command);
    let output = command
        .arg("-version")
        .output()
        .map_err(|_| "FFMPEG_TOOL_VERIFY_FAILED".to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err("FFMPEG_TOOL_VERIFY_FAILED".to_string())
    }
}

async fn download_with_progress(
    app: &tauri::AppHandle,
    operation_id: Option<&str>,
    url: &str,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(url)
        .header("User-Agent", "banana-box")
        .send()
        .await
        .map_err(|_| "FFMPEG_DOWNLOAD_FAILED".to_string())?;
    if !response.status().is_success() {
        return Err("FFMPEG_DOWNLOAD_FAILED".to_string());
    }
    let total = response.content_length().unwrap_or(0);
    if total > MAX_FFMPEG_ARCHIVE_BYTES as u64 {
        return Err("FFMPEG_ARCHIVE_TOO_LARGE".to_string());
    }
    let mut downloaded = 0_u64;
    let mut bytes = Vec::with_capacity(total.min(MAX_FFMPEG_ARCHIVE_BYTES as u64) as usize);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "FFMPEG_DOWNLOAD_FAILED".to_string())?;
        downloaded += chunk.len() as u64;
        if downloaded > MAX_FFMPEG_ARCHIVE_BYTES as u64 {
            return Err("FFMPEG_ARCHIVE_TOO_LARGE".to_string());
        }
        bytes.extend_from_slice(&chunk);
        if total > 0 {
            let progress = 12 + ((downloaded.saturating_mul(48) / total).min(48) as u8);
            emit_media_tool_progress(
                app,
                operation_id,
                "ffmpeg",
                "download",
                progress,
                format!("正在下载 FFmpeg：{}%", downloaded.saturating_mul(100) / total),
                None,
                "info",
            );
        }
    }
    Ok(bytes)
}

fn ffprobe_duration_secs(source: &Path, data_dir: Option<&Path>) -> Result<f64, String> {
    let output = Command::new(ffmpeg_tool_path("ffprobe", data_dir))
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
        .map_err(|_| ffmpeg_missing_message("ffprobe"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim()
        .parse::<f64>()
        .map_err(|_| "无法读取视频时长".to_string())
}

fn compress_video_with_ffmpeg(
    source: &Path,
    output: &Path,
    target_mb: f64,
    data_dir: Option<&Path>,
) -> Result<(), String> {
    let duration = ffprobe_duration_secs(source, data_dir)?;
    let audio_kbps = 128_u32;
    let video_kbps = video_bitrate_kbps(target_mb, duration, audio_kbps);
    let status = Command::new(ffmpeg_tool_path("ffmpeg", data_dir))
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
        .map_err(|_| ffmpeg_missing_message("ffmpeg"))?;
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

fn depth_video_output_path(source: &Path, suffix: &str) -> Result<PathBuf, String> {
    let dir = source
        .parent()
        .ok_or_else(|| "无法识别源文件目录".to_string())?;
    let stem = source
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "无法识别源文件名".to_string())?;
    Ok(dir.join(format!("{}_depth_{}.mp4", stem, suffix)))
}

fn resolve_depth_video_engine(configured_path: Option<&str>, env_path: Option<PathBuf>) -> PathBuf {
    configured_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env_path.filter(|value| !value.as_os_str().is_empty()))
        .unwrap_or_else(|| PathBuf::from(DEPTH_VIDEO_ENGINE_COMMAND))
}

fn depth_video_engine_command(configured_path: Option<&str>) -> Command {
    let env_path = std::env::var_os(DEPTH_VIDEO_ENGINE_ENV).map(PathBuf::from);
    let engine = resolve_depth_video_engine(configured_path, env_path);
    let extension = engine
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension == "ps1" {
        let mut command = Command::new("powershell.exe");
        command
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(engine);
        return command;
    }
    if extension == "bat" || extension == "cmd" {
        let mut command = Command::new("cmd.exe");
        command.arg("/C").arg(engine);
        return command;
    }
    Command::new(engine)
}

fn convert_video_with_depth_engine(
    source: &Path,
    output: &Path,
    configured_path: Option<&str>,
) -> Result<(), String> {
    let mut command = depth_video_engine_command(configured_path);
    configure_hidden_child_process(&mut command);
    let status = command
        .arg("--input")
        .arg(source)
        .arg("--output")
        .arg(output)
        .status()
        .map_err(|_| "DEPTH_VIDEO_ENGINE_MISSING".to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("DEPTH_VIDEO_CONVERSION_FAILED".to_string())
    }
}

fn depth_video_setup_script() -> &'static str {
    r#"$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoUrl = 'https://github.com/DepthAnything/Video-Depth-Anything/archive/refs/heads/main.zip'
$WeightUrl = 'https://huggingface.co/depth-anything/Video-Depth-Anything-Small/resolve/main/video_depth_anything_vits.pth'
$RepoDir = Join-Path $Root 'Video-Depth-Anything-main'
$ArchivePath = Join-Path $Root 'Video-Depth-Anything-main.zip'
$VenvDir = Join-Path $Root '.venv'
$CheckpointDir = Join-Path $RepoDir 'checkpoints'
$SmallCheckpoint = Join-Path $CheckpointDir 'video_depth_anything_vits.pth'

function Update-TextFileNoBom {
  param([string]$Path, [string]$Content)
  $Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($Path, $Content, $Utf8NoBom)
}

function Repair-DepthVideoCpuFallback {
  if (!(Test-Path $RepoDir)) {
    return
  }

  $AttentionPath = Join-Path $RepoDir 'video_depth_anything\dinov2_layers\attention.py'
  if (Test-Path $AttentionPath) {
    $AttentionText = Get-Content -Raw $AttentionPath
    $UpdatedAttentionText = $AttentionText.Replace(
      'if not XFORMERS_AVAILABLE:',
      'if (not XFORMERS_AVAILABLE) or (not x.is_cuda):'
    )
    if ($UpdatedAttentionText -ne $AttentionText) {
      Update-TextFileNoBom -Path $AttentionPath -Content $UpdatedAttentionText
    }
  }

  $MotionModulePath = Join-Path $RepoDir 'video_depth_anything\motion_module\motion_module.py'
  if (Test-Path $MotionModulePath) {
    $MotionModuleText = Get-Content -Raw $MotionModulePath
    $UpdatedMotionModuleText = $MotionModuleText.Replace(
      'use_memory_efficient = XFORMERS_AVAILABLE and self._use_memory_efficient_attention_xformers',
      'use_memory_efficient = XFORMERS_AVAILABLE and self._use_memory_efficient_attention_xformers and query.is_cuda'
    )
    if ($UpdatedMotionModuleText -ne $MotionModuleText) {
      Update-TextFileNoBom -Path $MotionModulePath -Content $UpdatedMotionModuleText
    }
  }

  $MotionAttentionPath = Join-Path $RepoDir 'video_depth_anything\motion_module\attention.py'
  if (Test-Path $MotionAttentionPath) {
    $MotionAttentionText = Get-Content -Raw $MotionAttentionPath
    $UpdatedMotionAttentionText = $MotionAttentionText.Replace(
      'if XFORMERS_AVAILABLE and self._use_memory_efficient_attention_xformers:',
      'if XFORMERS_AVAILABLE and self._use_memory_efficient_attention_xformers and query.is_cuda:'
    )
    if ($UpdatedMotionAttentionText -ne $MotionAttentionText) {
      Update-TextFileNoBom -Path $MotionAttentionPath -Content $UpdatedMotionAttentionText
    }
  }
}

function Get-PythonMinorVersion {
  param([string]$CommandName, [string[]]$CommandPrefix)
  $PreviousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  try {
    $VersionText = & $CommandName @CommandPrefix -c "import sys; print(str(sys.version_info.major) + '.' + str(sys.version_info.minor))" 2>$null
    $ExitCode = $LASTEXITCODE
  } catch {
    return $null
  } finally {
    $ErrorActionPreference = $PreviousErrorActionPreference
  }
  if (($ExitCode -eq 0) -and $VersionText) {
    return $VersionText.Trim()
  }
  return $null
}

function Test-CompatiblePythonMinorVersion {
  param([string]$VersionText)
  return (($VersionText -eq '3.11') -or ($VersionText -eq '3.10'))
}

function Get-ManagedPython310Path {
  if (!$env:LocalAppData) {
    return $null
  }
  $ManagedPython = Join-Path $env:LocalAppData 'Programs\Python\Python310\python.exe'
  if (Test-Path $ManagedPython) {
    return $ManagedPython
  }
  return $null
}

function Get-CompatiblePython {
  if (Get-Command py -ErrorAction SilentlyContinue) {
    foreach ($Version in @('3.11', '3.10')) {
      $VersionText = Get-PythonMinorVersion -CommandName 'py' -CommandPrefix @("-$Version")
      if ($VersionText -eq $Version) {
        return @{ Command = 'py'; Prefix = @("-$Version") }
      }
    }
  }

  $ManagedPython310 = Get-ManagedPython310Path
  if ($ManagedPython310) {
    $VersionText = Get-PythonMinorVersion -CommandName $ManagedPython310 -CommandPrefix @()
    if ($VersionText -eq '3.10') {
      return @{ Command = $ManagedPython310; Prefix = @() }
    }
  }

  if (Get-Command python -ErrorAction SilentlyContinue) {
    $VersionText = Get-PythonMinorVersion -CommandName 'python' -CommandPrefix @()
    if (Test-CompatiblePythonMinorVersion $VersionText) {
      return @{ Command = 'python'; Prefix = @() }
    }
    if ($VersionText) {
      throw "PYTHON_VERSION_UNSUPPORTED: $VersionText"
    }
  }

  if (Get-Command py -ErrorAction SilentlyContinue) {
    $VersionText = Get-PythonMinorVersion -CommandName 'py' -CommandPrefix @('-3')
    if ($VersionText) {
      throw "PYTHON_VERSION_UNSUPPORTED: $VersionText"
    }
  }

  throw 'PYTHON_NOT_FOUND'
}

function Invoke-HostPython {
  param([string[]]$Arguments)
  $Python = Get-CompatiblePython
  $PythonCommand = $Python.Command
  $PythonArgs = @($Python.Prefix) + $Arguments
  & $PythonCommand @PythonArgs
  if ($LASTEXITCODE -ne 0) {
    throw "PYTHON_COMMAND_FAILED: $($Arguments -join ' ')"
  }
}

if (!(Test-Path (Join-Path $RepoDir 'run.py'))) {
  if (!(Test-Path $ArchivePath)) {
    Invoke-WebRequest -Uri $RepoUrl -OutFile $ArchivePath -UseBasicParsing
  }
  $ExtractDir = Join-Path $Root 'repo-extract'
  if (Test-Path $ExtractDir) { Remove-Item -LiteralPath $ExtractDir -Recurse -Force }
  Expand-Archive -LiteralPath $ArchivePath -DestinationPath $ExtractDir -Force
  $ExtractedRepo = Join-Path $ExtractDir 'Video-Depth-Anything-main'
  if (Test-Path $RepoDir) { Remove-Item -LiteralPath $RepoDir -Recurse -Force }
  Move-Item -LiteralPath $ExtractedRepo -Destination $RepoDir
  Remove-Item -LiteralPath $ExtractDir -Recurse -Force
}

Repair-DepthVideoCpuFallback

$VenvPython = Join-Path $VenvDir 'Scripts\python.exe'
if (Test-Path $VenvPython) {
  $VenvVersionText = Get-PythonMinorVersion -CommandName $VenvPython -CommandPrefix @()
  if (!(Test-CompatiblePythonMinorVersion $VenvVersionText)) {
    Remove-Item -LiteralPath $VenvDir -Recurse -Force
  }
}

if (!(Test-Path $VenvPython)) {
  Invoke-HostPython @('-m', 'venv', $VenvDir)
}

& $VenvPython -m pip install --upgrade pip
if ($LASTEXITCODE -ne 0) { throw 'PIP_UPGRADE_FAILED' }
& $VenvPython -m pip install -r (Join-Path $RepoDir 'requirements.txt')
if ($LASTEXITCODE -ne 0) { throw 'PYTHON_REQUIREMENTS_INSTALL_FAILED' }

New-Item -ItemType Directory -Path $CheckpointDir -Force | Out-Null
if (!(Test-Path $SmallCheckpoint)) {
  Invoke-WebRequest -Uri $WeightUrl -OutFile $SmallCheckpoint -UseBasicParsing
}

Write-Output 'DEPTH_VIDEO_ENGINE_READY'
"#
}

fn depth_video_launcher_script() -> &'static str {
    r#"$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoDir = Join-Path $Root 'Video-Depth-Anything-main'
$VenvPython = Join-Path $Root '.venv\Scripts\python.exe'
$InputPath = $null
$OutputPath = $null

for ($Index = 0; $Index -lt $args.Count; $Index++) {
  if ($args[$Index] -eq '--input') {
    $Index += 1
    $InputPath = $args[$Index]
  } elseif ($args[$Index] -eq '--output') {
    $Index += 1
    $OutputPath = $args[$Index]
  }
}

if (!$InputPath -or !$OutputPath) {
  throw 'DEPTH_VIDEO_ARGUMENTS_REQUIRED: expected --input <video> --output <mp4>'
}
if (!(Test-Path $InputPath)) {
  throw "DEPTH_VIDEO_INPUT_NOT_FOUND: $InputPath"
}
if (!(Test-Path $VenvPython) -or !(Test-Path (Join-Path $RepoDir 'run.py'))) {
  throw 'DEPTH_VIDEO_ENGINE_NOT_CONFIGURED'
}

$RunDir = Join-Path (Join-Path $Root 'outputs') ([guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $RunDir -Force | Out-Null
$Stem = [IO.Path]::GetFileNameWithoutExtension($InputPath)
$Generated = Join-Path $RunDir "$($Stem)_vis.mp4"

function Update-TextFileNoBom {
  param([string]$Path, [string]$Content)
  $Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($Path, $Content, $Utf8NoBom)
}

function Repair-DepthVideoCpuFallback {
  if (!(Test-Path $RepoDir)) {
    return
  }

  $AttentionPath = Join-Path $RepoDir 'video_depth_anything\dinov2_layers\attention.py'
  if (Test-Path $AttentionPath) {
    $AttentionText = Get-Content -Raw $AttentionPath
    $UpdatedAttentionText = $AttentionText.Replace(
      'if not XFORMERS_AVAILABLE:',
      'if (not XFORMERS_AVAILABLE) or (not x.is_cuda):'
    )
    if ($UpdatedAttentionText -ne $AttentionText) {
      Update-TextFileNoBom -Path $AttentionPath -Content $UpdatedAttentionText
    }
  }

  $MotionModulePath = Join-Path $RepoDir 'video_depth_anything\motion_module\motion_module.py'
  if (Test-Path $MotionModulePath) {
    $MotionModuleText = Get-Content -Raw $MotionModulePath
    $UpdatedMotionModuleText = $MotionModuleText.Replace(
      'use_memory_efficient = XFORMERS_AVAILABLE and self._use_memory_efficient_attention_xformers',
      'use_memory_efficient = XFORMERS_AVAILABLE and self._use_memory_efficient_attention_xformers and query.is_cuda'
    )
    if ($UpdatedMotionModuleText -ne $MotionModuleText) {
      Update-TextFileNoBom -Path $MotionModulePath -Content $UpdatedMotionModuleText
    }
  }

  $MotionAttentionPath = Join-Path $RepoDir 'video_depth_anything\motion_module\attention.py'
  if (Test-Path $MotionAttentionPath) {
    $MotionAttentionText = Get-Content -Raw $MotionAttentionPath
    $UpdatedMotionAttentionText = $MotionAttentionText.Replace(
      'if XFORMERS_AVAILABLE and self._use_memory_efficient_attention_xformers:',
      'if XFORMERS_AVAILABLE and self._use_memory_efficient_attention_xformers and query.is_cuda:'
    )
    if ($UpdatedMotionAttentionText -ne $MotionAttentionText) {
      Update-TextFileNoBom -Path $MotionAttentionPath -Content $UpdatedMotionAttentionText
    }
  }
}

Repair-DepthVideoCpuFallback

$RunArgs = @(
  (Join-Path $RepoDir 'run.py'),
  '--input_video',
  $InputPath,
  '--output_dir',
  $RunDir,
  '--encoder',
  'vits',
  '--grayscale'
)
& $VenvPython -c "import torch; raise SystemExit(0 if torch.cuda.is_available() else 1)" 2>$null
if ($LASTEXITCODE -ne 0) {
  $RunArgs += '--fp32'
}

Push-Location $RepoDir
try {
  & $VenvPython @RunArgs
  if ($LASTEXITCODE -ne 0) { throw "DEPTH_VIDEO_RUN_FAILED: exit $LASTEXITCODE" }
} finally {
  Pop-Location
}

if (!(Test-Path $Generated)) {
  throw "DEPTH_VIDEO_OUTPUT_NOT_FOUND: $Generated"
}

New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($OutputPath)) -Force | Out-Null
Copy-Item -LiteralPath $Generated -Destination $OutputPath -Force
"#
}

fn depth_video_python_setup_script() -> &'static str {
    r#"$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$InstallerUrl = 'https://www.python.org/ftp/python/3.10.11/python-3.10.11-amd64.exe'
$InstallerPath = Join-Path $Root 'python-3.10.11-amd64.exe'

function Test-Python310Command {
  param([string]$CommandName, [string[]]$CommandPrefix)
  $PreviousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  try {
    $VersionText = & $CommandName @CommandPrefix -c "import sys; print(str(sys.version_info.major) + '.' + str(sys.version_info.minor))" 2>$null
    $ExitCode = $LASTEXITCODE
  } catch {
    return $false
  } finally {
    $ErrorActionPreference = $PreviousErrorActionPreference
  }
  return (($ExitCode -eq 0) -and $VersionText -and ($VersionText.Trim() -eq '3.10'))
}

function Get-ManagedPython310Path {
  if (!$env:LocalAppData) {
    return $null
  }
  $ManagedPython = Join-Path $env:LocalAppData 'Programs\Python\Python310\python.exe'
  if (Test-Path $ManagedPython) {
    return $ManagedPython
  }
  return $null
}

function Test-Python310Ready {
  if ((Get-Command py -ErrorAction SilentlyContinue) -and (Test-Python310Command -CommandName 'py' -CommandPrefix @('-3.10'))) {
    return $true
  }
  $ManagedPython = Get-ManagedPython310Path
  if ($ManagedPython -and (Test-Python310Command -CommandName $ManagedPython -CommandPrefix @())) {
    return $true
  }
  return $false
}

if (Test-Python310Ready) {
  Write-Output 'PYTHON_310_READY'
  exit 0
}

if (!(Test-Path $InstallerPath)) {
  Invoke-WebRequest -Uri $InstallerUrl -OutFile $InstallerPath -UseBasicParsing
}

$InstallArgs = @(
  '/quiet',
  'InstallAllUsers=0',
  'Include_launcher=0',
  'InstallLauncherAllUsers=0',
  'Include_pip=1',
  'PrependPath=1',
  'Include_test=0',
  'SimpleInstall=1'
)
$Process = Start-Process -FilePath $InstallerPath -ArgumentList $InstallArgs -Wait -PassThru
if ($Process.ExitCode -ne 0) {
  throw "PYTHON_310_INSTALL_FAILED: exit $($Process.ExitCode)"
}

if (!(Test-Python310Ready)) {
  throw 'PYTHON_310_VERIFY_FAILED'
}

Write-Output 'PYTHON_310_READY'
"#
}

fn depth_video_cmd_launcher() -> &'static str {
    "@echo off\r\npowershell.exe -NoProfile -ExecutionPolicy Bypass -File \"%~dp0banana-depth-video.ps1\" %*\r\n"
}

fn write_depth_video_engine_scripts(data_dir: &Path) -> Result<DepthVideoEngineSetupResult, String> {
    let engine_dir = data_dir.join("depth-video-engine");
    std::fs::create_dir_all(&engine_dir).map_err(|error| error.to_string())?;
    std::fs::write(
        engine_dir.join("setup-depth-video-engine.ps1"),
        depth_video_setup_script(),
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(
        engine_dir.join("banana-depth-video.ps1"),
        depth_video_launcher_script(),
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(
        engine_dir.join("install-python-3.10.ps1"),
        depth_video_python_setup_script(),
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(
        engine_dir.join("banana-depth-video.cmd"),
        depth_video_cmd_launcher(),
    )
    .map_err(|error| error.to_string())?;
    let engine_path = engine_dir.join("banana-depth-video.cmd");
    Ok(DepthVideoEngineSetupResult {
        engine_path: engine_path.to_string_lossy().to_string(),
        engine_dir: engine_dir.to_string_lossy().to_string(),
        message: "本地深度视频引擎已配置".to_string(),
    })
}

// 读取目录下所有 .md/.txt 文件内容，供前端解析
#[tauri::command]
pub fn read_import_dir(
    gate: tauri::State<StartupGate>,
    dir: String,
) -> Result<Vec<ImportFile>, String> {
    require_startup_ready(&gate)?;
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
pub async fn download_image(
    app: tauri::AppHandle,
    gate: tauri::State<'_, StartupGate>,
    url: String,
) -> Result<String, String> {
    require_startup_ready(&gate)?;
    let services = app
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let request_url = Url::parse(&url).map_err(|_| "INVALID_DOWNLOAD_URL")?;
    let bytes = services
        .provider_http
        .get_public_bounded(
            request_url.clone(),
            MAX_DOWNLOADED_IMAGE_BYTES,
            CancellationToken::new(),
        )
        .await?;
    let dir = data_dir(&app).join("images");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ext = request_url
        .path()
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
pub async fn check_for_update(
    app: tauri::AppHandle,
    gate: tauri::State<'_, StartupGate>,
) -> Result<UpdateCheckResult, String> {
    require_startup_ready(&gate)?;
    let services = app
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let release_url =
        Url::parse("https://api.github.com/repos/felix1709/banana-box/releases/latest")
            .map_err(|_| "INVALID_UPDATE_URL")?;
    let body = services
        .provider_http
        .get_public_bounded(
            release_url,
            MAX_UPDATE_RESPONSE_BYTES,
            CancellationToken::new(),
        )
        .await?;
    let body = String::from_utf8(body).map_err(|_| "INVALID_UPDATE_RESPONSE")?;
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
pub async fn prepare_ffmpeg_tools(
    app: tauri::AppHandle,
    gate: tauri::State<'_, StartupGate>,
    input: MediaToolOperationInput,
) -> Result<FfmpegSetupResult, String> {
    require_startup_ready(&gate)?;
    if !cfg!(windows) {
        return Err("FFMPEG_MANAGED_SETUP_UNSUPPORTED_PLATFORM".to_string());
    }

    let operation_id = input.operation_id.as_deref();
    let data_dir = data_dir(&app);
    let ffmpeg_root = data_dir.join("ffmpeg");
    let bin_dir = managed_ffmpeg_bin_dir(&data_dir);
    let extract_dir = ffmpeg_root.join("extract");
    let ffmpeg_path = bin_dir.join(ffmpeg_tool_filename("ffmpeg"));
    let ffprobe_path = bin_dir.join(ffmpeg_tool_filename("ffprobe"));

    emit_media_tool_progress(
        &app,
        operation_id,
        "ffmpeg",
        "check",
        8,
        "正在检查本地 FFmpeg",
        None,
        "info",
    );
    if ffmpeg_path.exists() && ffprobe_path.exists() {
        verify_ffmpeg_tool(&ffmpeg_path)?;
        verify_ffmpeg_tool(&ffprobe_path)?;
        emit_media_tool_progress(
            &app,
            operation_id,
            "ffmpeg",
            "done",
            100,
            "FFmpeg 已配置完成",
            None,
            "success",
        );
        return Ok(FfmpegSetupResult {
            ffmpeg_path: ffmpeg_path.to_string_lossy().to_string(),
            ffprobe_path: ffprobe_path.to_string_lossy().to_string(),
            bin_dir: bin_dir.to_string_lossy().to_string(),
            message: "FFmpeg 已配置完成，可以开始压缩视频".to_string(),
        });
    }

    fs::create_dir_all(&ffmpeg_root).map_err(|error| error.to_string())?;
    fs::create_dir_all(&bin_dir).map_err(|error| error.to_string())?;
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&extract_dir).map_err(|error| error.to_string())?;

    emit_media_tool_progress(
        &app,
        operation_id,
        "ffmpeg",
        "download",
        12,
        "正在下载 FFmpeg Essentials",
        Some(FFMPEG_WINDOWS_ESSENTIALS_ZIP_URL.to_string()),
        "info",
    );
    let archive = download_with_progress(
        &app,
        operation_id,
        FFMPEG_WINDOWS_ESSENTIALS_ZIP_URL,
    )
    .await?;

    emit_media_tool_progress(
        &app,
        operation_id,
        "ffmpeg",
        "extract",
        68,
        "正在解压 FFmpeg",
        None,
        "info",
    );
    extract_zip_safely(&archive, &extract_dir)?;
    let extracted_ffmpeg = find_child_file(&extract_dir, &ffmpeg_tool_filename("ffmpeg"))?;
    let extracted_ffprobe = find_child_file(&extract_dir, &ffmpeg_tool_filename("ffprobe"))?;
    fs::copy(extracted_ffmpeg, &ffmpeg_path).map_err(|error| error.to_string())?;
    fs::copy(extracted_ffprobe, &ffprobe_path).map_err(|error| error.to_string())?;

    emit_media_tool_progress(
        &app,
        operation_id,
        "ffmpeg",
        "verify",
        88,
        "正在验证 ffmpeg 和 ffprobe",
        None,
        "info",
    );
    verify_ffmpeg_tool(&ffmpeg_path)?;
    verify_ffmpeg_tool(&ffprobe_path)?;

    emit_media_tool_progress(
        &app,
        operation_id,
        "ffmpeg",
        "done",
        100,
        "FFmpeg 已配置完成",
        None,
        "success",
    );
    Ok(FfmpegSetupResult {
        ffmpeg_path: ffmpeg_path.to_string_lossy().to_string(),
        ffprobe_path: ffprobe_path.to_string_lossy().to_string(),
        bin_dir: bin_dir.to_string_lossy().to_string(),
        message: "FFmpeg 已配置完成，可以开始压缩视频".to_string(),
    })
}

#[tauri::command]
pub fn import_image_from_path(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    input: ImportImageFromPathInput,
) -> Result<String, String> {
    require_startup_ready(&gate)?;
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
pub fn compress_media(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    input: CompressMediaInput,
) -> Result<CompressMediaResult, String> {
    require_startup_ready(&gate)?;
    if input.target_mb <= 0.0 {
        return Err("目标大小必须大于 0 MB".to_string());
    }
    let source = PathBuf::from(&input.source_path);
    if !source.exists() {
        return Err("源文件不存在".to_string());
    }
    let output = PathBuf::from(&input.output_path);
    if is_video_path(&source) {
        let app_data_dir = data_dir(&app);
        compress_video_with_ffmpeg(&source, &output, input.target_mb, Some(&app_data_dir))?;
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
    gate: tauri::State<StartupGate>,
    input: SuggestCompressedOutputPathInput,
) -> Result<String, String> {
    require_startup_ready(&gate)?;
    let source = PathBuf::from(&input.source_path);
    let ext = compressed_output_ext(&source);
    let output = compressed_output_path(&source, ext, &timestamp_suffix_now())?;
    Ok(output.to_string_lossy().to_string())
}

#[tauri::command]
pub fn convert_video_to_depth_video(
    gate: tauri::State<StartupGate>,
    input: DepthVideoInput,
) -> Result<DepthVideoResult, String> {
    require_startup_ready(&gate)?;
    let source = PathBuf::from(&input.source_path);
    if !source.exists() {
        return Err("源视频文件不存在".to_string());
    }
    if !is_video_path(&source) {
        return Err("请选择视频文件".to_string());
    }
    let output = PathBuf::from(&input.output_path);
    convert_video_with_depth_engine(&source, &output, input.engine_path.as_deref())?;
    Ok(DepthVideoResult {
        output_path: output.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn prepare_depth_video_python(
    app: tauri::AppHandle,
    gate: tauri::State<'_, StartupGate>,
) -> Result<DepthVideoPythonSetupResult, String> {
    require_startup_ready(&gate)?;
    let data_dir = data_dir(&app);
    let script_result = write_depth_video_engine_scripts(&data_dir)?;
    let setup_script = PathBuf::from(&script_result.engine_dir).join("install-python-3.10.ps1");
    let setup_output = tauri::async_runtime::spawn_blocking(move || {
        let mut command = Command::new("powershell.exe");
        configure_hidden_child_process(&mut command);
        command
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(setup_script)
            .output()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|_| "DEPTH_VIDEO_PYTHON_SETUP_POWERSHELL_MISSING".to_string())?;

    if setup_output.status.success() {
        Ok(DepthVideoPythonSetupResult {
            python_version: "3.10".to_string(),
            message: "Python 3.10 环境已准备好".to_string(),
        })
    } else {
        let stderr = String::from_utf8_lossy(&setup_output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&setup_output.stdout).trim().to_string();
        Err(format!(
            "DEPTH_VIDEO_PYTHON_SETUP_FAILED\n{}{}",
            stdout,
            if stderr.is_empty() {
                String::new()
            } else {
                format!("\n{stderr}")
            }
        ))
    }
}

#[tauri::command]
pub async fn prepare_depth_video_engine(
    app: tauri::AppHandle,
    gate: tauri::State<'_, StartupGate>,
) -> Result<DepthVideoEngineSetupResult, String> {
    require_startup_ready(&gate)?;
    let data_dir = data_dir(&app);
    let script_result = write_depth_video_engine_scripts(&data_dir)?;
    let setup_script = PathBuf::from(&script_result.engine_dir).join("setup-depth-video-engine.ps1");
    let setup_output = tauri::async_runtime::spawn_blocking(move || {
        let mut command = Command::new("powershell.exe");
        configure_hidden_child_process(&mut command);
        command
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(setup_script)
            .output()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|_| "DEPTH_VIDEO_SETUP_POWERSHELL_MISSING".to_string())?;

    if setup_output.status.success() {
        Ok(script_result)
    } else {
        let stderr = String::from_utf8_lossy(&setup_output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&setup_output.stdout).trim().to_string();
        Err(format!(
            "DEPTH_VIDEO_ENGINE_SETUP_FAILED\n{}{}",
            stdout,
            if stderr.is_empty() {
                String::new()
            } else {
                format!("\n{stderr}")
            }
        ))
    }
}

#[tauri::command]
pub fn suggest_depth_video_output_path(
    gate: tauri::State<StartupGate>,
    input: SuggestDepthVideoOutputPathInput,
) -> Result<String, String> {
    require_startup_ready(&gate)?;
    let source = PathBuf::from(&input.source_path);
    let output = depth_video_output_path(&source, &timestamp_suffix_now())?;
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
    fn parse_chat_completion_prompt_reads_array_text_content() {
        let body = r#"{
          "choices": [
            {
              "message": {
                "content": [
                  { "type": "text", "text": "first visual prompt line" },
                  { "type": "text", "text": "second visual prompt line" }
                ]
              }
            }
          ]
        }"#;

        assert_eq!(
            parse_chat_completion_prompt(body).unwrap(),
            "first visual prompt line\nsecond visual prompt line"
        );
    }

    #[test]
    fn reverse_image_prompt_instruction_requires_structured_dimensions() {
        let instruction = reverse_image_prompt_instruction();
        for section in [
            "一、基础画面属性",
            "二、核心主体信息",
            "三、构图与镜头语言",
            "四、光影体系",
            "五、色彩与色调",
            "六、材质与质感",
            "七、环境与背景",
            "八、风格与情绪调性",
            "九、特殊效果与细节",
            "十、反向约束维度",
        ] {
            assert!(instruction.contains(section), "missing section: {section}");
        }

        assert!(instruction.contains("主要色彩"));
        assert!(instruction.contains("HEX"));
        assert!(instruction.contains("未明显体现"));
        assert!(instruction.contains("不要输出解释"));
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
    fn ffmpeg_tool_resolution_uses_common_install_dir_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let tool = dir.path().join(ffmpeg_tool_filename("ffmpeg"));
        std::fs::write(&tool, b"").unwrap();

        assert_eq!(resolve_ffmpeg_tool("ffmpeg", None, Some(dir.path())), tool);
    }

    #[test]
    fn ffmpeg_tool_resolution_prefers_managed_bin_dir_when_present() {
        let common = tempfile::tempdir().unwrap();
        let managed_root = tempfile::tempdir().unwrap();
        let managed = managed_ffmpeg_bin_dir(managed_root.path());
        std::fs::create_dir_all(&managed).unwrap();
        let common_tool = common.path().join(ffmpeg_tool_filename("ffprobe"));
        let managed_tool = managed.join(ffmpeg_tool_filename("ffprobe"));
        std::fs::write(common_tool, b"").unwrap();
        std::fs::write(&managed_tool, b"").unwrap();

        assert_eq!(
            resolve_ffmpeg_tool("ffprobe", Some(&managed), Some(common.path())),
            managed_tool
        );
    }

    #[test]
    fn ffmpeg_tool_resolution_falls_back_to_command_name_when_missing() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            resolve_ffmpeg_tool("ffprobe", None, Some(dir.path())),
            PathBuf::from("ffprobe")
        );
    }

    #[test]
    fn ffmpeg_missing_message_names_tool_and_download_url() {
        let message = ffmpeg_missing_message("ffprobe");

        assert!(message.contains("ffprobe"));
        assert!(message.contains("ffmpeg"));
        assert!(message.contains(FFMPEG_DOWNLOAD_URL));
    }

    #[test]
    fn depth_video_engine_resolution_prefers_user_configured_path() {
        assert_eq!(
            resolve_depth_video_engine(Some("C:/tools/banana-depth-video.exe"), None),
            PathBuf::from("C:/tools/banana-depth-video.exe")
        );
    }

    #[test]
    fn depth_video_engine_scripts_prepare_a_windows_launcher() {
        let directory = tempfile::tempdir().unwrap();
        let result = write_depth_video_engine_scripts(directory.path()).unwrap();

        assert!(result.engine_path.ends_with("banana-depth-video.cmd"));
        assert!(result.engine_dir.ends_with("depth-video-engine"));

        let engine_dir = directory.path().join("depth-video-engine");
        let setup_script = std::fs::read_to_string(engine_dir.join("setup-depth-video-engine.ps1")).unwrap();
        assert!(setup_script.contains("Video-Depth-Anything"));
        assert!(setup_script.contains("video_depth_anything_vits.pth"));

        let launcher = std::fs::read_to_string(engine_dir.join("banana-depth-video.ps1")).unwrap();
        assert!(launcher.contains("--input_video"));
        assert!(launcher.contains("--encoder"));
        assert!(launcher.contains("_vis.mp4"));
    }

    #[test]
    fn depth_video_engine_powershell_scripts_are_ascii_only_for_windows_powershell() {
        let directory = tempfile::tempdir().unwrap();
        write_depth_video_engine_scripts(directory.path()).unwrap();

        let engine_dir = directory.path().join("depth-video-engine");
        let setup_script =
            std::fs::read_to_string(engine_dir.join("setup-depth-video-engine.ps1")).unwrap();
        let launcher =
            std::fs::read_to_string(engine_dir.join("banana-depth-video.ps1")).unwrap();
        let python_setup =
            std::fs::read_to_string(engine_dir.join("install-python-3.10.ps1")).unwrap();

        assert!(setup_script.is_ascii());
        assert!(launcher.is_ascii());
        assert!(python_setup.is_ascii());
    }

    #[test]
    fn depth_video_setup_script_requires_a_dependency_compatible_python_version() {
        let directory = tempfile::tempdir().unwrap();
        write_depth_video_engine_scripts(directory.path()).unwrap();

        let setup_script = std::fs::read_to_string(
            directory
                .path()
                .join("depth-video-engine")
                .join("setup-depth-video-engine.ps1"),
        )
        .unwrap();

        assert!(setup_script.contains("PYTHON_VERSION_UNSUPPORTED"));
        assert!(setup_script.contains("'3.11', '3.10'"));
        assert!(!setup_script.contains("py -3 @Arguments"));
    }

    #[test]
    fn depth_video_python_probe_does_not_stop_when_py_launcher_has_no_matching_runtime() {
        let directory = tempfile::tempdir().unwrap();
        write_depth_video_engine_scripts(directory.path()).unwrap();

        let engine_dir = directory.path().join("depth-video-engine");
        let setup_script =
            std::fs::read_to_string(engine_dir.join("setup-depth-video-engine.ps1")).unwrap();
        let python_setup_script =
            std::fs::read_to_string(engine_dir.join("install-python-3.10.ps1")).unwrap();

        assert!(setup_script.contains("$PreviousErrorActionPreference = $ErrorActionPreference"));
        assert!(setup_script.contains("$ErrorActionPreference = 'Continue'"));
        assert!(setup_script.contains("$ErrorActionPreference = $PreviousErrorActionPreference"));
        assert!(python_setup_script
            .contains("$PreviousErrorActionPreference = $ErrorActionPreference"));
        assert!(python_setup_script.contains("$ErrorActionPreference = 'Continue'"));
        assert!(python_setup_script
            .contains("$ErrorActionPreference = $PreviousErrorActionPreference"));
    }

    #[test]
    fn depth_video_setup_script_rebuilds_an_existing_incompatible_virtualenv() {
        let directory = tempfile::tempdir().unwrap();
        write_depth_video_engine_scripts(directory.path()).unwrap();

        let setup_script = std::fs::read_to_string(
            directory
                .path()
                .join("depth-video-engine")
                .join("setup-depth-video-engine.ps1"),
        )
        .unwrap();

        assert!(setup_script.contains("Get-PythonMinorVersion -CommandName $VenvPython"));
        assert!(setup_script.contains("Remove-Item -LiteralPath $VenvDir -Recurse -Force"));
    }

    #[test]
    fn depth_video_scripts_patch_cpu_runs_away_from_cuda_only_xformers() {
        let directory = tempfile::tempdir().unwrap();
        write_depth_video_engine_scripts(directory.path()).unwrap();

        let engine_dir = directory.path().join("depth-video-engine");
        let setup_script =
            std::fs::read_to_string(engine_dir.join("setup-depth-video-engine.ps1")).unwrap();
        let launcher =
            std::fs::read_to_string(engine_dir.join("banana-depth-video.ps1")).unwrap();

        for script in [&setup_script, &launcher] {
            assert!(script.contains("Repair-DepthVideoCpuFallback"));
            assert!(script.contains("not XFORMERS_AVAILABLE) or (not x.is_cuda"));
            assert!(script.contains("query.is_cuda"));
            assert!(script.contains("UTF8Encoding($false)"));
        }
        assert!(launcher.contains("'--fp32'"));
        assert!(launcher.contains("torch.cuda.is_available()"));
    }

    #[test]
    fn depth_video_python_setup_script_installs_official_python_310_for_current_user() {
        let directory = tempfile::tempdir().unwrap();
        write_depth_video_engine_scripts(directory.path()).unwrap();

        let python_setup_script = std::fs::read_to_string(
            directory
                .path()
                .join("depth-video-engine")
                .join("install-python-3.10.ps1"),
        )
        .unwrap();

        assert!(python_setup_script.contains(
            "https://www.python.org/ftp/python/3.10.11/python-3.10.11-amd64.exe"
        ));
        assert!(python_setup_script.contains("InstallAllUsers=0"));
        assert!(python_setup_script.contains("Include_launcher=0"));
        assert!(python_setup_script.contains("InstallLauncherAllUsers=0"));
        assert!(python_setup_script.contains("Include_pip=1"));
        assert!(python_setup_script.contains("PrependPath=1"));
        assert!(python_setup_script.contains("PYTHON_310_READY"));
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

    #[test]
    fn legacy_business_command_gate_rejects_recovery_before_work_starts() {
        let gate = crate::app_state::StartupGate::new(crate::app_state::StartupStatus::Recovery {
            message: "Recovery required".into(),
            backup_paths: vec![],
        });

        assert_eq!(
            require_startup_ready(&gate).unwrap_err(),
            "STARTUP_NOT_READY"
        );
    }
}
