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
use image::codecs::jpeg::JpegEncoder;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;
use tokio_util::sync::CancellationToken;
use url::Url;

const MAX_DOWNLOADED_IMAGE_BYTES: usize = 15 * 1024 * 1024;
const MAX_UPDATE_RESPONSE_BYTES: usize = 1024 * 1024;
const APP_SERVICES_UNAVAILABLE: &str = "STARTUP_NOT_READY";

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
pub struct SuggestCompressedOutputPathInput {
    pub source_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressMediaResult {
    pub output_path: String,
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
    content: String,
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

fn ffmpeg_tool_filename(tool_name: &str) -> String {
    if cfg!(windows) {
        format!("{tool_name}.exe")
    } else {
        tool_name.to_string()
    }
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

fn resolve_ffmpeg_tool(tool_name: &str, common_bin_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = common_bin_dir {
        let candidate = dir.join(ffmpeg_tool_filename(tool_name));
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(tool_name)
}

fn ffmpeg_tool_path(tool_name: &str) -> PathBuf {
    resolve_ffmpeg_tool(tool_name, default_ffmpeg_bin_dir().as_deref())
}

fn ffprobe_duration_secs(source: &Path) -> Result<f64, String> {
    let output = Command::new(ffmpeg_tool_path("ffprobe"))
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
        .map_err(|_| "未找到 ffprobe，请安装完整 FFmpeg（需要 ffmpeg 和 ffprobe）并加入 PATH".to_string())?;
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
    let status = Command::new(ffmpeg_tool_path("ffmpeg"))
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
        .map_err(|_| "未找到 ffmpeg，请安装完整 FFmpeg（需要 ffmpeg 和 ffprobe）并加入 PATH".to_string())?;
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
    gate: tauri::State<StartupGate>,
    input: SuggestCompressedOutputPathInput,
) -> Result<String, String> {
    require_startup_ready(&gate)?;
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

        assert_eq!(resolve_ffmpeg_tool("ffmpeg", Some(dir.path())), tool);
    }

    #[test]
    fn ffmpeg_tool_resolution_falls_back_to_command_name_when_missing() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            resolve_ffmpeg_tool("ffprobe", Some(dir.path())),
            PathBuf::from("ffprobe")
        );
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
