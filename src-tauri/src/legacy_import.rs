use crate::{
    fs_atomic,
    library::{self, LegacySecrets, Library},
    providers::{ProviderKind, ProviderService, SaveProviderInput},
    safe_archive::{self, ArchiveLimits},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime},
};
use uuid::Uuid;

const STAGING_DIRECTORY: &str = ".legacy-import-staging";
const MAX_LIBRARY_JSON_BYTES: u64 = 4 * 1024 * 1024;
const MAX_LEGACY_IMAGE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PREVIEWS: usize = 2;
const PREVIEW_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImportPreview {
    pub token: String,
    pub prompt_count: usize,
    pub category_count: usize,
    pub has_api_key: bool,
    pub credential_conflict: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
struct StagedImage {
    logical_path: String,
    sha256: String,
}

#[derive(Clone)]
struct PreviewRecord {
    source: PathBuf,
    source_hash: String,
    sanitized_hash: String,
    directory: PathBuf,
    images: Vec<StagedImage>,
    expires_at: SystemTime,
    claimed: bool,
}

pub struct BackupStagingCoordinator {
    root: PathBuf,
    previews: Mutex<HashMap<Uuid, PreviewRecord>>,
}

pub(crate) struct ParsedLegacyImport {
    library: Library,
    secrets: LegacySecrets,
    warnings: Vec<String>,
    images: Vec<(String, Vec<u8>, String)>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImportCommit {
    pub library: Library,
    pub prompts_imported: usize,
    pub categories_imported: usize,
    pub warnings: Vec<String>,
}

impl BackupStagingCoordinator {
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        let root = data_dir.join(STAGING_DIRECTORY);
        fs::create_dir_all(&root).map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
        remove_stale_staging_directories(&root)?;
        Ok(Self {
            root,
            previews: Mutex::new(HashMap::new()),
        })
    }

    pub fn inspect(
        &self,
        source: &Path,
        credential_conflict: bool,
    ) -> Result<LegacyImportPreview, String> {
        self.discard_expired()?;
        let source = verified_source_path(source)?;
        let parsed = parse_legacy_source(&source)?;
        let sanitized = library::serialize_sanitized_library(&parsed.library)
            .map_err(|_| "INVALID_LEGACY_LIBRARY")?;
        let source_hash = sha256_file(&source)?;
        let sanitized_hash = sha256_bytes(&sanitized);
        let token = Uuid::new_v4();
        let directory = self.root.join(token.to_string());

        {
            let previews = self
                .previews
                .lock()
                .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
            if previews.len() >= MAX_PREVIEWS {
                return Err("IMPORT_PREVIEW_LIMIT_REACHED".into());
            }
        }

        write_staged_import(&directory, &sanitized, &parsed.images)?;
        let preview = LegacyImportPreview {
            token: token.to_string(),
            prompt_count: parsed.library.prompts.len(),
            category_count: parsed.library.categories.len(),
            has_api_key: parsed.secrets.api_key.is_some(),
            credential_conflict,
            warnings: parsed.warnings,
        };
        let record = PreviewRecord {
            source,
            source_hash,
            sanitized_hash,
            directory: directory.clone(),
            images: parsed
                .images
                .iter()
                .map(|(logical_path, _, sha256)| StagedImage {
                    logical_path: logical_path.clone(),
                    sha256: sha256.clone(),
                })
                .collect(),
            expires_at: SystemTime::now() + PREVIEW_TTL,
            claimed: false,
        };
        let mut previews = self
            .previews
            .lock()
            .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
        if previews.len() >= MAX_PREVIEWS {
            let _ = remove_directory(&directory);
            return Err("IMPORT_PREVIEW_LIMIT_REACHED".into());
        }
        previews.insert(token, record);
        Ok(preview)
    }

    pub fn discard(&self, token: &str) -> Result<(), String> {
        self.discard_expired()?;
        let token = parse_preview_token(token)?;
        let record = self
            .previews
            .lock()
            .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?
            .remove(&token);
        let Some(record) = record else {
            return Ok(());
        };
        if record.claimed {
            self.previews
                .lock()
                .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?
                .insert(token, record);
            return Err("PREVIEW_IN_USE".into());
        }
        remove_directory(&record.directory)
    }

    pub(crate) fn claim_for_commit(&self, token: &str) -> Result<ClaimedPreview, String> {
        self.discard_expired()?;
        let token = parse_preview_token(token)?;
        let mut previews = self
            .previews
            .lock()
            .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
        let record = previews
            .get_mut(&token)
            .ok_or_else(|| "STALE_PREVIEW_TOKEN".to_string())?;
        if record.claimed {
            return Err("STALE_PREVIEW_TOKEN".into());
        }
        record.claimed = true;
        Ok(ClaimedPreview {
            token,
            source: record.source.clone(),
            source_hash: record.source_hash.clone(),
            sanitized_hash: record.sanitized_hash.clone(),
            directory: record.directory.clone(),
            images: record.images.clone(),
        })
    }

    pub(crate) fn release_claim(&self, token: Uuid) -> Result<(), String> {
        let mut previews = self
            .previews
            .lock()
            .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
        let record = previews
            .get_mut(&token)
            .ok_or_else(|| "STALE_PREVIEW_TOKEN".to_string())?;
        record.claimed = false;
        Ok(())
    }

    pub(crate) fn complete_claim(&self, token: Uuid) -> Result<(), String> {
        let record = self
            .previews
            .lock()
            .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?
            .remove(&token)
            .ok_or_else(|| "STALE_PREVIEW_TOKEN".to_string())?;
        remove_directory(&record.directory)
    }

    pub(crate) fn commit(
        &self,
        data_dir: &Path,
        providers: &ProviderService,
        token: &str,
        overwrite_credential: bool,
    ) -> Result<LegacyImportCommit, String> {
        let claimed = self.claim_for_commit(token)?;
        let mut created_images = Vec::new();
        let temporary_library = data_dir.join(format!(".legacy-import-{}.tmp", claimed.token));
        let mut result = (|| {
            let imported = claimed.revalidate()?;
            let live =
                library::load_library_strict(data_dir).map_err(|_| "LIVE_LIBRARY_UNAVAILABLE")?;
            let (mut merged, mut warnings) = merge_libraries(&live, &imported.library)?;
            let image_paths = copy_images_for_commit(data_dir, &claimed, &mut created_images)?;
            for prompt in merged.prompts.iter_mut().skip(live.prompts.len()) {
                if let Some(imported_path) = prompt.image.as_deref() {
                    if let Some(copied_path) = image_paths.get(imported_path) {
                        prompt.image = Some(copied_path.clone());
                    } else {
                        prompt.image = None;
                        warnings.push("一个缺失的导入图片已被忽略".into());
                    }
                }
            }
            let serialized = library::serialize_sanitized_library(&merged)
                .map_err(|_| "INVALID_LEGACY_LIBRARY")?;
            fs::write(&temporary_library, serialized).map_err(|_| "IMPORT_COMMIT_FAILED")?;

            if let Some(api_key) = imported.secrets.api_key.as_deref() {
                let current = providers.get("reverse-image")?;
                if !current.needs_credentials && !overwrite_credential {
                    return Err("CREDENTIAL_OVERWRITE_REQUIRED".into());
                }
                providers.save(
                    legacy_reverse_provider_input(&imported.secrets),
                    Some(api_key),
                )?;
            }
            fs_atomic::replace_file(&temporary_library, &library::library_path(data_dir))?;

            Ok(LegacyImportCommit {
                library: merged,
                prompts_imported: imported.library.prompts.len(),
                categories_imported: imported.library.categories.len(),
                warnings,
            })
        })();

        if let Ok(committed) = &mut result {
            if self.complete_claim(claimed.token).is_err() {
                committed
                    .warnings
                    .push("导入已完成，暂存文件将在下次启动时清理".into());
            }
        } else {
            let _ = fs::remove_file(&temporary_library);
            for image in created_images {
                let _ = fs::remove_file(image);
            }
            let _ = self.release_claim(claimed.token);
        }
        result
    }

    fn discard_expired(&self) -> Result<(), String> {
        let now = SystemTime::now();
        let stale = {
            let previews = self
                .previews
                .lock()
                .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
            previews
                .iter()
                .filter_map(|(token, record)| {
                    (!record.claimed && record.expires_at <= now).then_some(*token)
                })
                .collect::<Vec<_>>()
        };
        for token in stale {
            self.discard(&token.to_string())?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ClaimedPreview {
    pub token: Uuid,
    pub source: PathBuf,
    pub source_hash: String,
    pub sanitized_hash: String,
    pub directory: PathBuf,
    images: Vec<StagedImage>,
}

impl ClaimedPreview {
    pub(crate) fn revalidate(&self) -> Result<ParsedLegacyImport, String> {
        let source = verified_source_path(&self.source)?;
        if sha256_file(&source)? != self.source_hash {
            return Err("SOURCE_CHANGED".into());
        }
        let parsed = parse_legacy_source(&source)?;
        let sanitized = library::serialize_sanitized_library(&parsed.library)
            .map_err(|_| "INVALID_LEGACY_LIBRARY")?;
        if sha256_bytes(&sanitized) != self.sanitized_hash
            || !same_image_manifest(&parsed.images, &self.images)
        {
            return Err("SOURCE_CHANGED".into());
        }
        Ok(parsed)
    }
}

fn parse_legacy_source(source: &Path) -> Result<ParsedLegacyImport, String> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("zip") {
        parse_legacy_zip(source)
    } else if extension.eq_ignore_ascii_case("json") {
        let bytes = read_regular_file_bounded(source, MAX_LIBRARY_JSON_BYTES)?;
        parse_legacy_parts(bytes, Vec::new())
    } else {
        Err("UNSUPPORTED_IMPORT_SOURCE".into())
    }
}

fn parse_legacy_zip(source: &Path) -> Result<ParsedLegacyImport, String> {
    let entries = safe_archive::read_zip(source, ArchiveLimits::default(), legacy_entry_policy)?;
    let mut library_json = None;
    let mut images = Vec::new();
    for entry in entries {
        if entry.path == Path::new("library.json") {
            if library_json.replace(entry.bytes).is_some() {
                return Err("DUPLICATE_LIBRARY_ENTRY".into());
            }
        } else {
            images.push((
                entry.path.to_string_lossy().replace('\\', "/"),
                entry.bytes,
                entry.sha256,
            ));
        }
    }
    let library_json = library_json.ok_or_else(|| "MISSING_LIBRARY_ENTRY".to_string())?;
    parse_legacy_parts(library_json, images)
}

fn parse_legacy_parts(
    library_json: Vec<u8>,
    images: Vec<(String, Vec<u8>, String)>,
) -> Result<ParsedLegacyImport, String> {
    if library_json.len() as u64 > MAX_LIBRARY_JSON_BYTES {
        return Err("IMPORT_LIMIT_EXCEEDED".into());
    }
    let raw = String::from_utf8(library_json).map_err(|_| "INVALID_LEGACY_LIBRARY")?;
    let (library, secrets, warnings) =
        library::normalize_legacy_json(&raw).map_err(|_| "INVALID_LEGACY_LIBRARY")?;
    Ok(ParsedLegacyImport {
        library,
        secrets,
        warnings,
        images,
    })
}

fn legacy_entry_policy(path: &Path, directory: bool, size: u64) -> Result<(), String> {
    if directory {
        return (path == Path::new("images"))
            .then_some(())
            .ok_or_else(|| "UNSUPPORTED_ARCHIVE_ENTRY".into());
    }
    if path == Path::new("library.json") {
        return (size <= MAX_LIBRARY_JSON_BYTES)
            .then_some(())
            .ok_or_else(|| "IMPORT_LIMIT_EXCEEDED".into());
    }
    let valid_image = path
        .parent()
        .is_some_and(|parent| parent == Path::new("images"))
        && path.components().count() == 2
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "png" | "jpg" | "jpeg" | "webp" | "gif"
                )
            });
    if !valid_image {
        return Err("UNSUPPORTED_ARCHIVE_ENTRY".into());
    }
    (size <= MAX_LEGACY_IMAGE_BYTES)
        .then_some(())
        .ok_or_else(|| "IMPORT_LIMIT_EXCEEDED".into())
}

fn write_staged_import(
    directory: &Path,
    sanitized_library: &[u8],
    images: &[(String, Vec<u8>, String)],
) -> Result<(), String> {
    fs::create_dir(directory).map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
    fs::write(directory.join("library.json"), sanitized_library)
        .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
    for (logical_path, bytes, _) in images {
        let name = Path::new(logical_path)
            .file_name()
            .ok_or_else(|| "UNSAFE_ARCHIVE_PATH".to_string())?;
        let image_directory = directory.join("images");
        fs::create_dir_all(&image_directory).map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
        fs::write(image_directory.join(name), bytes).map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
    }
    Ok(())
}

fn same_image_manifest(images: &[(String, Vec<u8>, String)], staged: &[StagedImage]) -> bool {
    images.len() == staged.len()
        && images
            .iter()
            .zip(staged)
            .all(|((path, _, sha256), staged)| {
                path == &staged.logical_path && sha256 == &staged.sha256
            })
}

fn merge_libraries(live: &Library, imported: &Library) -> Result<(Library, Vec<String>), String> {
    let mut merged = live.clone();
    let mut category_ids = live
        .categories
        .iter()
        .map(|category| category.id.clone())
        .collect::<HashSet<_>>();
    let mut imported_category_ids = HashSet::new();
    let mut category_map = HashMap::new();
    let next_category_order = merged
        .categories
        .iter()
        .map(|category| category.order)
        .max()
        .unwrap_or(-1)
        + 1;
    let mut categories = imported.categories.clone();
    categories.sort_by(|left, right| (left.order, &left.id).cmp(&(right.order, &right.id)));
    for (index, mut category) in categories.into_iter().enumerate() {
        if !imported_category_ids.insert(category.id.clone()) {
            return Err("DUPLICATE_IMPORT_ID".into());
        }
        let source_id = category.id.clone();
        if category_ids.contains(&category.id) || Uuid::parse_str(&category.id).is_err() {
            category.id = Uuid::new_v4().to_string();
        }
        category_ids.insert(category.id.clone());
        category.order = next_category_order + index as i32;
        category_map.insert(source_id, category.id.clone());
        merged.categories.push(category);
    }

    let mut prompt_ids = live
        .prompts
        .iter()
        .map(|prompt| prompt.id.clone())
        .collect::<HashSet<_>>();
    let mut imported_prompt_ids = HashSet::new();
    let next_prompt_order = merged
        .prompts
        .iter()
        .map(|prompt| prompt.order)
        .max()
        .unwrap_or(-1)
        + 1;
    let mut prompts = imported.prompts.clone();
    prompts.sort_by(|left, right| (left.order, &left.id).cmp(&(right.order, &right.id)));
    let mut warnings = Vec::new();
    for (index, mut prompt) in prompts.into_iter().enumerate() {
        if !imported_prompt_ids.insert(prompt.id.clone()) {
            return Err("DUPLICATE_IMPORT_ID".into());
        }
        if prompt_ids.contains(&prompt.id) || Uuid::parse_str(&prompt.id).is_err() {
            prompt.id = Uuid::new_v4().to_string();
        }
        prompt_ids.insert(prompt.id.clone());
        prompt.category_id = match prompt.category_id {
            Some(source_id) => Some(
                category_map
                    .get(&source_id)
                    .cloned()
                    .ok_or_else(|| "IMPORT_CATEGORY_REFERENCE_INVALID".to_string())?,
            ),
            None => None,
        };
        if prompt
            .image
            .as_deref()
            .is_some_and(|image| !image.starts_with("images/"))
        {
            prompt.image = None;
            warnings.push("一个不安全的图片引用已被忽略".into());
        }
        prompt.order = next_prompt_order + index as i32;
        merged.prompts.push(prompt);
    }
    Ok((merged, warnings))
}

fn copy_images_for_commit(
    data_dir: &Path,
    claimed: &ClaimedPreview,
    created: &mut Vec<PathBuf>,
) -> Result<HashMap<String, String>, String> {
    let destination_directory = data_dir.join("images");
    fs::create_dir_all(&destination_directory).map_err(|_| "IMPORT_COMMIT_FAILED")?;
    let mut copied = HashMap::new();
    for image in &claimed.images {
        let source = claimed.directory.join(&image.logical_path);
        let bytes = read_regular_file_bounded(&source, MAX_LEGACY_IMAGE_BYTES)?;
        if sha256_bytes(&bytes) != image.sha256 {
            return Err("IMPORT_STAGING_CHANGED".into());
        }
        let extension = Path::new(&image.logical_path)
            .extension()
            .and_then(|extension| extension.to_str())
            .ok_or_else(|| "IMPORT_STAGING_CHANGED".to_string())?;
        let name = format!("{}.{}", Uuid::new_v4(), extension.to_ascii_lowercase());
        let destination = destination_directory.join(&name);
        fs::write(&destination, bytes).map_err(|_| "IMPORT_COMMIT_FAILED")?;
        created.push(destination);
        copied.insert(image.logical_path.clone(), format!("images/{name}"));
    }
    Ok(copied)
}

fn legacy_reverse_provider_input(secrets: &LegacySecrets) -> SaveProviderInput {
    let base = secrets.api_base_url.trim().trim_end_matches('/');
    let api_root = if base.ends_with("/v1") {
        base.to_string()
    } else {
        format!("{base}/v1")
    };
    SaveProviderInput {
        id: "reverse-image".into(),
        kind: ProviderKind::ReverseImage,
        display_name: "图片反推".into(),
        base_url: api_root.clone(),
        models_url: format!("{api_root}/models"),
        chat_completions_url: format!("{api_root}/chat/completions"),
        default_model: Some(secrets.reverse_model.clone()),
        confirm_cross_origin: false,
    }
}

fn verified_source_path(source: &Path) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(source).map_err(|_| "IMPORT_SOURCE_UNAVAILABLE")?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err("UNSAFE_IMPORT_SOURCE".into());
    }
    source
        .canonicalize()
        .map_err(|_| "IMPORT_SOURCE_UNAVAILABLE".into())
}

fn read_regular_file_bounded(source: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(source).map_err(|_| "IMPORT_SOURCE_UNAVAILABLE")?;
    if metadata.len() > limit {
        return Err("IMPORT_LIMIT_EXCEEDED".into());
    }
    let mut file = fs::File::open(source).map_err(|_| "IMPORT_SOURCE_UNAVAILABLE")?;
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| "IMPORT_SOURCE_UNAVAILABLE")?;
        if read == 0 {
            return Ok(bytes);
        }
        let next = bytes
            .len()
            .checked_add(read)
            .ok_or_else(|| "IMPORT_LIMIT_EXCEEDED".to_string())?;
        if next as u64 > limit {
            return Err("IMPORT_LIMIT_EXCEEDED".into());
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
}

fn parse_preview_token(token: &str) -> Result<Uuid, String> {
    let parsed = Uuid::parse_str(token).map_err(|_| "STALE_PREVIEW_TOKEN")?;
    (parsed.to_string() == token)
        .then_some(parsed)
        .ok_or_else(|| "STALE_PREVIEW_TOKEN".into())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|_| "IMPORT_SOURCE_UNAVAILABLE")?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| "IMPORT_SOURCE_UNAVAILABLE")?;
        if read == 0 {
            return Ok(format!("{:x}", hasher.finalize()));
        }
        hasher.update(&buffer[..read]);
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn remove_stale_staging_directories(root: &Path) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|_| "IMPORT_STAGING_UNAVAILABLE")? {
        let entry = entry.map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?;
        if entry
            .file_type()
            .map_err(|_| "IMPORT_STAGING_UNAVAILABLE")?
            .is_dir()
        {
            remove_directory(&entry.path())?;
        }
    }
    Ok(())
}

fn remove_directory(path: &Path) -> Result<(), String> {
    fs::remove_dir_all(path).map_err(|_| "IMPORT_STAGING_UNAVAILABLE".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::Database,
        provider_http::ProviderHttpClient,
        providers::ProviderService,
        secrets::{CredentialMutationCoordinator, MemoryCredentialStore},
        startup,
    };
    use std::io::{Cursor, Write};
    use std::sync::Arc;
    use tempfile::tempdir;
    use zip::{write::SimpleFileOptions, ZipWriter};

    const LEGACY_KEY: &str = "test-only-legacy-key";

    fn legacy_json() -> String {
        format!(
            r##"{{
              "version": 1,
              "categories": [],
              "prompts": [],
              "settings": {{
                "hotkey": "Ctrl+Shift+B",
                "theme": "auto",
                "apiKey": "{LEGACY_KEY}"
              }}
            }}"##
        )
    }

    fn legacy_json_with_collisions(category_id: &str, prompt_id: &str) -> String {
        format!(
            r##"{{
              "version": 1,
              "categories": [{{
                "id": "{category_id}",
                "name": "Imported category",
                "color": "#facc15",
                "order": 0
              }}],
              "prompts": [{{
                "id": "{prompt_id}",
                "title": "Imported prompt",
                "content": "A safe imported prompt",
                "categoryId": "{category_id}",
                "tags": [],
                "image": null,
                "createdAt": 1,
                "updatedAt": 1
              }}],
              "settings": {{
                "hotkey": "Ctrl+Shift+B",
                "theme": "auto",
                "apiKey": "{LEGACY_KEY}"
              }}
            }}"##
        )
    }

    fn provider_service(
        data_dir: &Path,
    ) -> (Arc<Database>, Arc<MemoryCredentialStore>, ProviderService) {
        startup::initialize_fresh(data_dir).unwrap();
        let database = Arc::new(Database::open(data_dir.join("banana.db")).unwrap());
        let credentials = Arc::new(MemoryCredentialStore::default());
        let provider_http = Arc::new(ProviderHttpClient::new().unwrap());
        let providers = ProviderService::new(
            database.clone(),
            credentials.clone(),
            provider_http,
            Arc::new(CredentialMutationCoordinator::default()),
        );
        (database, credentials, providers)
    }

    fn legacy_secrets(api_key: Option<&str>) -> LegacySecrets {
        LegacySecrets {
            api_base_url: "https://ai.leihuo.netease.com".into(),
            api_key: api_key.map(str::to_owned),
            reverse_model: "doubao-seed-1-6-vision-250815".into(),
            available_reverse_models: vec!["doubao-seed-1-6-vision-250815".into()],
        }
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        for (name, bytes) in entries {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
    }

    #[test]
    fn inspect_rejects_parent_path_entry_without_creating_staging() {
        let data_dir = tempdir().unwrap();
        let source = data_dir.path().join("legacy.zip");
        write_zip(
            &source,
            &[
                ("library.json", legacy_json().as_bytes()),
                ("images/../../escape.txt", b"unsafe"),
            ],
        );
        let coordinator = BackupStagingCoordinator::new(data_dir.path()).unwrap();

        assert_eq!(
            coordinator.inspect(&source, false).unwrap_err(),
            "UNSAFE_ARCHIVE_PATH"
        );
        assert!(fs::read_dir(data_dir.path().join(STAGING_DIRECTORY))
            .unwrap()
            .next()
            .is_none());
    }

    #[test]
    fn inspect_stages_only_sanitized_legacy_library() {
        let data_dir = tempdir().unwrap();
        let source = data_dir.path().join("library.json");
        fs::write(&source, legacy_json()).unwrap();
        let coordinator = BackupStagingCoordinator::new(data_dir.path()).unwrap();

        let preview = coordinator.inspect(&source, false).unwrap();
        let staged = fs::read_to_string(
            data_dir
                .path()
                .join(STAGING_DIRECTORY)
                .join(&preview.token)
                .join("library.json"),
        )
        .unwrap();

        assert!(preview.has_api_key);
        assert!(!staged.contains("apiKey"));
        assert!(!staged.contains(LEGACY_KEY));
    }

    #[test]
    fn preview_tokens_are_canonical_single_use_values() {
        let data_dir = tempdir().unwrap();
        let source = data_dir.path().join("library.json");
        fs::write(&source, legacy_json()).unwrap();
        let coordinator = BackupStagingCoordinator::new(data_dir.path()).unwrap();
        let preview = coordinator.inspect(&source, false).unwrap();

        let claimed = coordinator.claim_for_commit(&preview.token).unwrap();
        assert_eq!(claimed.token.to_string(), preview.token);
        assert_eq!(
            coordinator.claim_for_commit(&preview.token).unwrap_err(),
            "STALE_PREVIEW_TOKEN"
        );
        assert_eq!(
            coordinator
                .claim_for_commit(&preview.token.to_uppercase())
                .unwrap_err(),
            "STALE_PREVIEW_TOKEN"
        );
    }

    #[test]
    fn commit_strips_the_legacy_key_and_rewrites_colliding_category_references() {
        let data_dir = tempdir().unwrap();
        let (database, _credentials, providers) = provider_service(data_dir.path());
        let category_id = Uuid::new_v4().to_string();
        let prompt_id = Uuid::new_v4().to_string();
        let mut live = library::load_library_strict(data_dir.path()).unwrap();
        live.categories.push(library::Category {
            id: category_id.clone(),
            name: "Existing category".into(),
            color: "#22c55e".into(),
            order: 0,
        });
        live.prompts.push(library::Prompt {
            id: prompt_id.clone(),
            title: "Existing prompt".into(),
            content: "Keep this prompt".into(),
            category_id: Some(category_id.clone()),
            tags: vec![],
            image: None,
            favorite: false,
            order: 0,
            created_at: 1,
            updated_at: 1,
        });
        library::save_library(data_dir.path(), &live).unwrap();
        let source = data_dir.path().join("legacy-source.json");
        fs::write(
            &source,
            legacy_json_with_collisions(&category_id, &prompt_id),
        )
        .unwrap();
        let coordinator = BackupStagingCoordinator::new(data_dir.path()).unwrap();

        let preview = coordinator.inspect(&source, false).unwrap();
        let committed = coordinator
            .commit(data_dir.path(), &providers, &preview.token, false)
            .unwrap();
        let saved = fs::read_to_string(library::library_path(data_dir.path())).unwrap();

        assert_eq!(committed.prompts_imported, 1);
        assert!(!saved.contains("apiKey"));
        assert!(!saved.contains(LEGACY_KEY));
        assert_eq!(committed.library.categories.len(), 2);
        let imported_category = committed
            .library
            .categories
            .iter()
            .find(|category| category.name == "Imported category")
            .unwrap();
        assert_ne!(imported_category.id, category_id);
        let imported_prompt = committed
            .library
            .prompts
            .iter()
            .find(|prompt| prompt.title == "Imported prompt")
            .unwrap();
        assert_eq!(
            imported_prompt.category_id.as_deref(),
            Some(imported_category.id.as_str())
        );
        assert_eq!(
            providers
                .resolve_for_request("reverse-image")
                .unwrap()
                .api_key,
            LEGACY_KEY
        );
        assert_eq!(
            database
                .with_connection(|connection| {
                    connection
                        .query_row("SELECT COUNT(*) FROM projects", [], |row| {
                            row.get::<_, i64>(0)
                        })
                        .map_err(|_| "DATABASE_TEST_FAILED".to_string())
                })
                .unwrap(),
            0
        );
    }

    #[test]
    fn commit_requires_explicit_credential_overwrite_and_keeps_the_existing_key() {
        let data_dir = tempdir().unwrap();
        let (_database, _credentials, providers) = provider_service(data_dir.path());
        providers
            .save(
                legacy_reverse_provider_input(&legacy_secrets(None)),
                Some("existing-key"),
            )
            .unwrap();
        let source = data_dir.path().join("legacy-source.json");
        fs::write(&source, legacy_json()).unwrap();
        let coordinator = BackupStagingCoordinator::new(data_dir.path()).unwrap();
        let preview = coordinator.inspect(&source, true).unwrap();

        assert_eq!(
            coordinator
                .commit(data_dir.path(), &providers, &preview.token, false)
                .unwrap_err(),
            "CREDENTIAL_OVERWRITE_REQUIRED"
        );
        assert_eq!(
            providers
                .resolve_for_request("reverse-image")
                .unwrap()
                .api_key,
            "existing-key"
        );

        coordinator
            .commit(data_dir.path(), &providers, &preview.token, true)
            .unwrap();
        assert_eq!(
            providers
                .resolve_for_request("reverse-image")
                .unwrap()
                .api_key,
            LEGACY_KEY
        );
    }

    #[test]
    fn commit_rejects_a_source_that_changed_after_preview() {
        let data_dir = tempdir().unwrap();
        let (_database, _credentials, providers) = provider_service(data_dir.path());
        let source = data_dir.path().join("legacy-source.json");
        fs::write(&source, legacy_json()).unwrap();
        let coordinator = BackupStagingCoordinator::new(data_dir.path()).unwrap();
        let preview = coordinator.inspect(&source, false).unwrap();
        fs::write(
            &source,
            legacy_json().replace("Ctrl+Shift+B", "Alt+Shift+P"),
        )
        .unwrap();

        assert_eq!(
            coordinator
                .commit(data_dir.path(), &providers, &preview.token, false)
                .unwrap_err(),
            "SOURCE_CHANGED"
        );
        assert!(library::load_library_strict(data_dir.path())
            .unwrap()
            .prompts
            .is_empty());
    }

    #[test]
    fn commit_copies_imported_images_to_new_logical_paths() {
        let data_dir = tempdir().unwrap();
        let (_database, _credentials, providers) = provider_service(data_dir.path());
        let category_id = Uuid::new_v4().to_string();
        let prompt_id = Uuid::new_v4().to_string();
        let source = data_dir.path().join("legacy-source.zip");
        let legacy = legacy_json_with_collisions(&category_id, &prompt_id)
            .replace("\"image\": null", "\"image\": \"images/sample.png\"");
        write_zip(
            &source,
            &[
                ("library.json", legacy.as_bytes()),
                ("images/sample.png", b"png"),
            ],
        );
        let coordinator = BackupStagingCoordinator::new(data_dir.path()).unwrap();
        let preview = coordinator.inspect(&source, false).unwrap();

        let committed = coordinator
            .commit(data_dir.path(), &providers, &preview.token, false)
            .unwrap();
        let imported = committed
            .library
            .prompts
            .iter()
            .find(|prompt| prompt.title == "Imported prompt")
            .unwrap();
        let image_path = imported.image.as_deref().unwrap();

        assert_ne!(image_path, "images/sample.png");
        assert_eq!(fs::read(data_dir.path().join(image_path)).unwrap(), b"png");
    }
}
