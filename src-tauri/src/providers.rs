use crate::{
    db::Database,
    provider_http::ProviderHttpClient,
    secrets::{CredentialMutationCoordinator, CredentialStore},
};
use rusqlite::{params, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

const INVALID_PROVIDER_URL: &str = "INVALID_PROVIDER_URL";
const INSECURE_PROVIDER_URL: &str = "INSECURE_PROVIDER_URL";
const CROSS_ORIGIN_CONFIRMATION_REQUIRED: &str = "CROSS_ORIGIN_CONFIRMATION_REQUIRED";
const PROVIDER_KIND_MISMATCH: &str = "PROVIDER_KIND_MISMATCH";
const PROVIDER_NOT_FOUND: &str = "PROVIDER_NOT_FOUND";
const PROVIDER_STORAGE_UNAVAILABLE: &str = "PROVIDER_STORAGE_UNAVAILABLE";
const PROVIDER_CREDENTIALS_REQUIRED: &str = "PROVIDER_CREDENTIALS_REQUIRED";
const PROVIDER_CREDENTIAL_STORE_UNAVAILABLE: &str = "PROVIDER_CREDENTIAL_STORE_UNAVAILABLE";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    ReverseImage,
    Storyboard,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredMode {
    JsonSchema,
    StrictJson,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProvider {
    pub id: String,
    pub kind: ProviderKind,
    pub display_name: String,
    pub base_url: String,
    pub models_url: String,
    pub chat_completions_url: String,
    pub default_model: Option<String>,
    pub available_models: Vec<String>,
    pub probed_model: Option<String>,
    pub structured_mode: Option<StructuredMode>,
    pub interactive_compatible: Option<bool>,
    pub bound_host: Option<String>,
    pub needs_credentials: bool,
    pub config_revision: i64,
    pub capability_revision: i64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveProviderInput {
    pub id: String,
    pub kind: ProviderKind,
    pub display_name: String,
    pub base_url: String,
    pub models_url: String,
    pub chat_completions_url: String,
    pub default_model: Option<String>,
    #[serde(default)]
    pub confirm_cross_origin: bool,
}

pub struct ResolvedProvider {
    pub provider: AiProvider,
    pub api_key: String,
}

pub enum SafeCredentialError {
    MissingBinding,
    MissingSecret,
    StoreUnavailable,
}

pub struct ProviderPreflight {
    pub provider: AiProvider,
    pub credential: Result<String, SafeCredentialError>,
}

pub struct ProviderService {
    database: Arc<Database>,
    credential_store: Arc<dyn CredentialStore>,
    provider_http: Arc<ProviderHttpClient>,
    credential_mutations: Arc<CredentialMutationCoordinator>,
}

struct CanonicalEndpoints {
    base_url: String,
    models_url: String,
    chat_completions_url: String,
    origin_fingerprint: String,
}

struct RawProvider {
    id: String,
    kind: String,
    display_name: String,
    base_url: String,
    models_url: String,
    chat_completions_url: String,
    default_model: Option<String>,
    available_models_json: String,
    probed_model: Option<String>,
    structured_mode: Option<String>,
    interactive_compatible: Option<i64>,
    bound_host: Option<String>,
    needs_credentials: i64,
    credential_ref: Option<String>,
    config_revision: i64,
    capability_revision: i64,
}

struct StoredProvider {
    provider: AiProvider,
    credential_ref: Option<String>,
}

pub(crate) fn validated_host_fingerprint(input: &SaveProviderInput) -> Result<String, String> {
    canonicalize_endpoints(
        &input.base_url,
        &input.models_url,
        &input.chat_completions_url,
        input.confirm_cross_origin,
    )
    .map(|endpoints| endpoints.origin_fingerprint)
}

impl ProviderService {
    pub fn new(
        database: Arc<Database>,
        credential_store: Arc<dyn CredentialStore>,
        provider_http: Arc<ProviderHttpClient>,
        credential_mutations: Arc<CredentialMutationCoordinator>,
    ) -> Self {
        Self {
            database,
            credential_store,
            provider_http,
            credential_mutations,
        }
    }

    pub fn list(&self, kind: ProviderKind) -> Result<Vec<AiProvider>, String> {
        let rows = database_result(self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT
                        id, kind, display_name, base_url, models_url, chat_completions_url,
                        default_model, available_models_json, probed_model, structured_mode,
                        interactive_compatible, bound_host, needs_credentials, credential_ref,
                        config_revision, capability_revision
                     FROM ai_providers WHERE kind = ?1 ORDER BY id",
                )
                .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
            let rows = statement
                .query_map([kind.database_value()], raw_provider_from_row)
                .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
            let mut collected = Vec::new();
            for row in rows {
                collected.push(row.map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?);
            }
            Ok(collected)
        }))?;

        rows.into_iter()
            .map(stored_provider_from_raw)
            .map(|result| result.map(|stored| stored.provider))
            .collect()
    }

    pub fn get(&self, id: &str) -> Result<AiProvider, String> {
        self.load_stored_provider(id)?
            .map(|stored| stored.provider)
            .ok_or_else(|| PROVIDER_NOT_FOUND.to_string())
    }

    pub fn save(
        &self,
        input: SaveProviderInput,
        api_key: Option<&str>,
    ) -> Result<AiProvider, String> {
        let endpoints = canonicalize_endpoints(
            &input.base_url,
            &input.models_url,
            &input.chat_completions_url,
            input.confirm_cross_origin,
        )?;
        let _mutation_guard = self
            .credential_mutations
            .acquire()
            .map_err(|_| PROVIDER_CREDENTIAL_STORE_UNAVAILABLE.to_string())?;
        let current = self
            .load_stored_provider(&input.id)?
            .ok_or_else(|| PROVIDER_KIND_MISMATCH.to_string())?;
        if current.provider.kind != input.kind {
            return Err(PROVIDER_KIND_MISMATCH.into());
        }

        let candidate_ref = api_key
            .filter(|key| !key.trim().is_empty())
            .map(|key| {
                self.create_verified_candidate(&input.id, &endpoints.origin_fingerprint, key)
            })
            .transpose()?;
        let provider_id = input.id.clone();
        let mut retired_ref = None;
        database_result(self.database.with_immediate_transaction(|transaction| {
            let live = load_stored_provider_from_transaction(transaction, &provider_id)?
                .ok_or_else(|| PROVIDER_KIND_MISMATCH.to_string())?;
            if live.provider.kind != input.kind {
                return Err(PROVIDER_KIND_MISMATCH.into());
            }
            let endpoints_changed = live.provider.base_url != endpoints.base_url
                || live.provider.models_url != endpoints.models_url
                || live.provider.chat_completions_url != endpoints.chat_completions_url;
            let origin_changed =
                live.provider.bound_host.as_deref() != Some(endpoints.origin_fingerprint.as_str());
            let next_credential_ref = candidate_ref.clone().or_else(|| {
                if origin_changed {
                    None
                } else {
                    live.credential_ref.clone()
                }
            });
            retired_ref = (live.credential_ref != next_credential_ref)
                .then_some(live.credential_ref.clone())
                .flatten();
            let changed_rows = if endpoints_changed {
                transaction.execute(
                    "UPDATE ai_providers SET
                        display_name = ?2,
                        base_url = ?3,
                        models_url = ?4,
                        chat_completions_url = ?5,
                        default_model = NULL,
                        available_models_json = '[]',
                        probed_model = NULL,
                        structured_mode = NULL,
                        interactive_compatible = NULL,
                        bound_host = ?6,
                        needs_credentials = ?7,
                        credential_ref = ?8,
                        config_revision = ?9,
                        capability_revision = ?10,
                        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?1",
                    params![
                        provider_id,
                        input.display_name,
                        endpoints.base_url,
                        endpoints.models_url,
                        endpoints.chat_completions_url,
                        endpoints.origin_fingerprint,
                        i64::from(next_credential_ref.is_none()),
                        next_credential_ref,
                        live.provider.config_revision + 1,
                        live.provider.capability_revision + 1,
                    ],
                )
            } else {
                transaction.execute(
                    "UPDATE ai_providers SET
                        display_name = ?2,
                        base_url = ?3,
                        models_url = ?4,
                        chat_completions_url = ?5,
                        default_model = ?6,
                        bound_host = ?7,
                        needs_credentials = ?8,
                        credential_ref = ?9,
                        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?1",
                    params![
                        provider_id,
                        input.display_name,
                        endpoints.base_url,
                        endpoints.models_url,
                        endpoints.chat_completions_url,
                        input.default_model,
                        endpoints.origin_fingerprint,
                        i64::from(next_credential_ref.is_none()),
                        next_credential_ref,
                    ],
                )
            }
            .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
            if changed_rows != 1 {
                return Err(PROVIDER_STORAGE_UNAVAILABLE.into());
            }
            if let Some(candidate_ref) = &candidate_ref {
                transaction
                    .execute(
                        "DELETE FROM credential_cleanup WHERE credential_ref = ?1",
                        [candidate_ref],
                    )
                    .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
            }
            if let Some(retired_ref) = &retired_ref {
                insert_cleanup_reference(transaction, retired_ref, "retired")?;
            }
            Ok(())
        }))?;

        if let Some(retired_ref) = retired_ref {
            self.try_remove_retired_credential(&retired_ref);
        }
        self.get(&provider_id)
    }

    pub fn clear_credential(&self, id: &str) -> Result<(), String> {
        let _mutation_guard = self
            .credential_mutations
            .acquire()
            .map_err(|_| PROVIDER_CREDENTIAL_STORE_UNAVAILABLE.to_string())?;
        let current = self
            .load_stored_provider(id)?
            .ok_or_else(|| PROVIDER_NOT_FOUND.to_string())?;
        let retired_ref = current.credential_ref;

        database_result(self.database.with_immediate_transaction(|transaction| {
            let changed_rows = transaction
                .execute(
                    "UPDATE ai_providers SET
                        credential_ref = NULL,
                        needs_credentials = 1,
                        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?1",
                    [id],
                )
                .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
            if changed_rows != 1 {
                return Err(PROVIDER_STORAGE_UNAVAILABLE.into());
            }
            if let Some(retired_ref) = &retired_ref {
                insert_cleanup_reference(transaction, retired_ref, "retired")?;
            }
            Ok(())
        }))?;

        if let Some(retired_ref) = retired_ref {
            self.try_remove_retired_credential(&retired_ref);
        }
        Ok(())
    }

    pub(crate) fn with_request_preflight<T>(
        &self,
        id: &str,
        operation: impl FnOnce(ProviderPreflight) -> Result<T, String>,
    ) -> Result<T, String> {
        let _mutation_guard = self
            .credential_mutations
            .acquire()
            .map_err(|_| PROVIDER_CREDENTIAL_STORE_UNAVAILABLE.to_string())?;
        let stored = self
            .load_stored_provider(id)?
            .ok_or_else(|| PROVIDER_NOT_FOUND.to_string())?;
        let credential = if stored_provider_is_valid(&stored) {
            match stored.credential_ref.as_deref() {
                None => Err(SafeCredentialError::MissingBinding),
                Some(credential_ref) => match self.credential_store.get(credential_ref) {
                    Ok(Some(secret)) => Ok(secret),
                    Ok(None) => Err(SafeCredentialError::MissingSecret),
                    Err(_) => Err(SafeCredentialError::StoreUnavailable),
                },
            }
        } else {
            Err(SafeCredentialError::StoreUnavailable)
        };

        operation(ProviderPreflight {
            provider: stored.provider,
            credential,
        })
    }

    pub fn with_resolved_for_request<T>(
        &self,
        id: &str,
        operation: impl FnOnce(ResolvedProvider) -> Result<T, String>,
    ) -> Result<T, String> {
        self.with_request_preflight(id, |preflight| match preflight.credential {
            Ok(api_key) => operation(ResolvedProvider {
                provider: preflight.provider,
                api_key,
            }),
            Err(error) => Err(error.public_code().to_string()),
        })
    }

    pub fn resolve_for_request(&self, id: &str) -> Result<ResolvedProvider, String> {
        self.with_resolved_for_request(id, Ok)
    }

    fn create_verified_candidate(
        &self,
        provider_id: &str,
        origin_fingerprint: &str,
        api_key: &str,
    ) -> Result<String, String> {
        let credential_ref = credential_reference(provider_id, origin_fingerprint);
        database_result(self.database.with_immediate_transaction(|transaction| {
            insert_cleanup_reference(transaction, &credential_ref, "candidate")
        }))?;
        self.credential_store
            .set(&credential_ref, api_key)
            .map_err(|_| PROVIDER_CREDENTIAL_STORE_UNAVAILABLE.to_string())?;
        match self.credential_store.get(&credential_ref) {
            Ok(Some(stored_key)) if stored_key == api_key => Ok(credential_ref),
            Ok(_) | Err(_) => Err(PROVIDER_CREDENTIAL_STORE_UNAVAILABLE.into()),
        }
    }

    fn try_remove_retired_credential(&self, credential_ref: &str) {
        let removed = self
            .credential_store
            .delete(credential_ref)
            .and_then(|()| self.credential_store.get(credential_ref))
            .map(|remaining| remaining.is_none())
            .unwrap_or(false);
        if removed {
            let _ = self.database.with_immediate_transaction(|transaction| {
                transaction
                    .execute(
                        "DELETE FROM credential_cleanup WHERE credential_ref = ?1",
                        [credential_ref],
                    )
                    .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
                Ok(())
            });
        }
    }

    fn load_stored_provider(&self, id: &str) -> Result<Option<StoredProvider>, String> {
        let raw = database_result(self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT
                        id, kind, display_name, base_url, models_url, chat_completions_url,
                        default_model, available_models_json, probed_model, structured_mode,
                        interactive_compatible, bound_host, needs_credentials, credential_ref,
                        config_revision, capability_revision
                     FROM ai_providers WHERE id = ?1",
                    [id],
                    raw_provider_from_row,
                )
                .optional()
                .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())
        }))?;
        raw.map(stored_provider_from_raw).transpose()
    }
}

impl SafeCredentialError {
    fn public_code(&self) -> &'static str {
        match self {
            Self::MissingBinding | Self::MissingSecret => PROVIDER_CREDENTIALS_REQUIRED,
            Self::StoreUnavailable => PROVIDER_CREDENTIAL_STORE_UNAVAILABLE,
        }
    }
}

impl ProviderKind {
    fn database_value(self) -> &'static str {
        match self {
            Self::ReverseImage => "reverse-image",
            Self::Storyboard => "storyboard",
        }
    }

    fn from_database(value: &str) -> Option<Self> {
        match value {
            "reverse-image" => Some(Self::ReverseImage),
            "storyboard" => Some(Self::Storyboard),
            _ => None,
        }
    }
}

impl StructuredMode {
    fn from_database(value: &str) -> Option<Self> {
        match value {
            "json_schema" => Some(Self::JsonSchema),
            "strict_json" => Some(Self::StrictJson),
            _ => None,
        }
    }
}

fn canonicalize_endpoints(
    base_url: &str,
    models_url: &str,
    chat_completions_url: &str,
    confirm_cross_origin: bool,
) -> Result<CanonicalEndpoints, String> {
    let base = parse_provider_url(base_url)?;
    let models = parse_provider_url(models_url)?;
    let chat = parse_provider_url(chat_completions_url)?;

    for endpoint in [&base, &models, &chat] {
        validate_provider_url(endpoint)?;
    }

    let origins = canonical_origins([&base, &models, &chat]);

    if (base.origin() != models.origin() || base.origin() != chat.origin()) && !confirm_cross_origin
    {
        return Err(format!(
            "{CROSS_ORIGIN_CONFIRMATION_REQUIRED}: {}",
            origins.join("|")
        ));
    }

    Ok(CanonicalEndpoints {
        base_url: base.to_string(),
        models_url: models.to_string(),
        chat_completions_url: chat.to_string(),
        origin_fingerprint: origins.join("|"),
    })
}

fn raw_provider_from_row(row: &Row<'_>) -> rusqlite::Result<RawProvider> {
    Ok(RawProvider {
        id: row.get(0)?,
        kind: row.get(1)?,
        display_name: row.get(2)?,
        base_url: row.get(3)?,
        models_url: row.get(4)?,
        chat_completions_url: row.get(5)?,
        default_model: row.get(6)?,
        available_models_json: row.get(7)?,
        probed_model: row.get(8)?,
        structured_mode: row.get(9)?,
        interactive_compatible: row.get(10)?,
        bound_host: row.get(11)?,
        needs_credentials: row.get(12)?,
        credential_ref: row.get(13)?,
        config_revision: row.get(14)?,
        capability_revision: row.get(15)?,
    })
}

fn load_stored_provider_from_transaction(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<Option<StoredProvider>, String> {
    let raw = transaction
        .query_row(
            "SELECT
                id, kind, display_name, base_url, models_url, chat_completions_url,
                default_model, available_models_json, probed_model, structured_mode,
                interactive_compatible, bound_host, needs_credentials, credential_ref,
                config_revision, capability_revision
             FROM ai_providers WHERE id = ?1",
            [id],
            raw_provider_from_row,
        )
        .optional()
        .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
    raw.map(stored_provider_from_raw).transpose()
}

fn stored_provider_from_raw(raw: RawProvider) -> Result<StoredProvider, String> {
    let kind = ProviderKind::from_database(&raw.kind)
        .ok_or_else(|| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
    let available_models = serde_json::from_str(&raw.available_models_json)
        .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
    let structured_mode = match raw.structured_mode.as_deref() {
        None => None,
        Some(value) => Some(
            StructuredMode::from_database(value)
                .ok_or_else(|| PROVIDER_STORAGE_UNAVAILABLE.to_string())?,
        ),
    };
    let interactive_compatible = match raw.interactive_compatible {
        None => None,
        Some(0) => Some(false),
        Some(1) => Some(true),
        Some(_) => return Err(PROVIDER_STORAGE_UNAVAILABLE.into()),
    };
    if !matches!(raw.needs_credentials, 0 | 1)
        || raw.config_revision < 1
        || raw.capability_revision < 1
    {
        return Err(PROVIDER_STORAGE_UNAVAILABLE.into());
    }

    Ok(StoredProvider {
        provider: AiProvider {
            id: raw.id,
            kind,
            display_name: raw.display_name,
            base_url: raw.base_url,
            models_url: raw.models_url,
            chat_completions_url: raw.chat_completions_url,
            default_model: raw.default_model,
            available_models,
            probed_model: raw.probed_model,
            structured_mode,
            interactive_compatible,
            bound_host: raw.bound_host,
            needs_credentials: raw.needs_credentials == 1,
            config_revision: raw.config_revision,
            capability_revision: raw.capability_revision,
        },
        credential_ref: raw.credential_ref,
    })
}

fn stored_provider_is_valid(stored: &StoredProvider) -> bool {
    canonicalize_endpoints(
        &stored.provider.base_url,
        &stored.provider.models_url,
        &stored.provider.chat_completions_url,
        true,
    )
    .map(|endpoints| {
        stored.provider.bound_host.as_deref() == Some(endpoints.origin_fingerprint.as_str())
    })
    .unwrap_or(false)
}

fn credential_reference(provider_id: &str, origin_fingerprint: &str) -> String {
    let origin_hash = format!("{:x}", Sha256::digest(origin_fingerprint.as_bytes()));
    format!("provider/{provider_id}/{origin_hash}/{}", Uuid::new_v4())
}

fn insert_cleanup_reference(
    transaction: &Transaction<'_>,
    credential_ref: &str,
    reason: &str,
) -> Result<(), String> {
    transaction
        .execute(
            "INSERT OR REPLACE INTO credential_cleanup (credential_ref, reason, created_at)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![credential_ref, reason],
        )
        .map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())?;
    Ok(())
}

fn database_result<T>(result: Result<T, String>) -> Result<T, String> {
    result.map_err(|_| PROVIDER_STORAGE_UNAVAILABLE.to_string())
}

fn canonical_origins(endpoints: [&Url; 3]) -> Vec<String> {
    let mut origins = endpoints
        .into_iter()
        .map(|endpoint| endpoint.origin().ascii_serialization().to_ascii_lowercase())
        .collect::<Vec<_>>();
    origins.sort();
    origins.dedup();
    origins
}

fn parse_provider_url(value: &str) -> Result<Url, String> {
    Url::parse(value).map_err(|_| INVALID_PROVIDER_URL.into())
}

fn validate_provider_url(endpoint: &Url) -> Result<(), String> {
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || endpoint.host_str().is_none()
    {
        return Err(INVALID_PROVIDER_URL.into());
    }

    let host = endpoint.host_str().expect("host was checked above");
    let is_loopback = matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]");
    let uses_allowed_scheme =
        endpoint.scheme() == "https" || (endpoint.scheme() == "http" && is_loopback);

    uses_allowed_scheme
        .then_some(())
        .ok_or_else(|| INSECURE_PROVIDER_URL.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::Database,
        provider_http::ProviderHttpClient,
        secrets::{CredentialMutationCoordinator, CredentialStore},
    };
    use rusqlite::params;
    use std::{
        collections::HashMap,
        sync::{mpsc, Arc, Mutex},
        thread,
        time::Duration,
    };
    use tempfile::{tempdir, TempDir};

    const SENTINEL: &str = "TEST_ONLY_DO_NOT_USE";

    #[derive(Default)]
    struct RecordingCredentialStore {
        entries: Mutex<HashMap<String, String>>,
        mutations: Mutex<usize>,
        fail_set: Mutex<bool>,
        fail_delete: Mutex<bool>,
        on_next_set: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    }

    impl RecordingCredentialStore {
        fn mutation_count(&self) -> usize {
            *self.mutations.lock().unwrap()
        }

        fn stored_secret(&self, credential_ref: &str) -> Option<String> {
            self.entries.lock().unwrap().get(credential_ref).cloned()
        }

        fn fail_next_set(&self) {
            *self.fail_set.lock().unwrap() = true;
        }

        fn fail_deletes(&self) {
            *self.fail_delete.lock().unwrap() = true;
        }

        fn run_on_next_set(&self, callback: Arc<dyn Fn() + Send + Sync>) {
            *self.on_next_set.lock().unwrap() = Some(callback);
        }
    }

    impl CredentialStore for RecordingCredentialStore {
        fn set(&self, credential_ref: &str, secret: &str) -> Result<(), String> {
            *self.mutations.lock().unwrap() += 1;
            if std::mem::take(&mut *self.fail_set.lock().unwrap()) {
                return Err("credential store unavailable".into());
            }
            if let Some(callback) = self.on_next_set.lock().unwrap().take() {
                callback();
            }
            self.entries
                .lock()
                .unwrap()
                .insert(credential_ref.into(), secret.into());
            Ok(())
        }

        fn get(&self, credential_ref: &str) -> Result<Option<String>, String> {
            Ok(self.entries.lock().unwrap().get(credential_ref).cloned())
        }

        fn delete(&self, credential_ref: &str) -> Result<(), String> {
            *self.mutations.lock().unwrap() += 1;
            if *self.fail_delete.lock().unwrap() {
                return Err("credential store unavailable".into());
            }
            self.entries.lock().unwrap().remove(credential_ref);
            Ok(())
        }
    }

    struct TestContext {
        _dir: TempDir,
        db: Arc<Database>,
        credentials: Arc<RecordingCredentialStore>,
        mutations: Arc<CredentialMutationCoordinator>,
        service: Arc<ProviderService>,
    }

    #[derive(Debug, Eq, PartialEq)]
    struct DatabaseProviderState {
        display_name: String,
        base_url: String,
        models_url: String,
        chat_completions_url: String,
        default_model: Option<String>,
        available_models_json: String,
        probed_model: Option<String>,
        structured_mode: Option<String>,
        interactive_compatible: Option<i64>,
        bound_host: Option<String>,
        needs_credentials: i64,
        credential_ref: Option<String>,
        config_revision: i64,
        capability_revision: i64,
    }

    fn test_context() -> TestContext {
        let dir = tempdir().unwrap();
        let db = Arc::new(Database::open(dir.path().join("banana.db")).unwrap());
        seed_provider(
            &db,
            "reverse-image",
            ProviderKind::ReverseImage,
            "https://reverse.example.test",
        );
        seed_provider(
            &db,
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        let credentials = Arc::new(RecordingCredentialStore::default());
        let mutations = Arc::new(CredentialMutationCoordinator::default());
        let service = Arc::new(ProviderService::new(
            db.clone(),
            credentials.clone(),
            Arc::new(ProviderHttpClient::new().unwrap()),
            mutations.clone(),
        ));
        TestContext {
            _dir: dir,
            db,
            credentials,
            mutations,
            service,
        }
    }

    fn seed_provider(db: &Database, id: &str, kind: ProviderKind, origin: &str) {
        let kind = match kind {
            ProviderKind::ReverseImage => "reverse-image",
            ProviderKind::Storyboard => "storyboard",
        };
        db.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO ai_providers (
                        id, kind, display_name, base_url, models_url, chat_completions_url,
                        default_model, available_models_json, probed_model, structured_mode,
                        interactive_compatible, bound_host, needs_credentials, credential_ref,
                        config_revision, capability_revision, created_at, updated_at
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6,
                        'seed-model', '[\"seed-model\"]', 'seed-model', 'json_schema',
                        1, ?7, 1, NULL, 4, 9, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z'
                    )",
                    params![
                        id,
                        kind,
                        format!("Seed {id}"),
                        format!("{origin}/v1"),
                        format!("{origin}/v1/models"),
                        format!("{origin}/v1/chat/completions"),
                        origin,
                    ],
                )
                .map_err(|_| "TEST_DATABASE_ERROR".to_string())?;
            Ok(())
        })
        .unwrap();
    }

    fn save_input(id: &str, kind: ProviderKind, origin: &str) -> SaveProviderInput {
        SaveProviderInput {
            id: id.into(),
            kind,
            display_name: format!("Saved {id}"),
            base_url: format!("{origin}/v1"),
            models_url: format!("{origin}/v1/models"),
            chat_completions_url: format!("{origin}/v1/chat/completions"),
            default_model: Some("saved-model".into()),
            confirm_cross_origin: false,
        }
    }

    fn database_state(db: &Database, id: &str) -> DatabaseProviderState {
        db.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT
                        display_name, base_url, models_url, chat_completions_url,
                        default_model, available_models_json, probed_model, structured_mode,
                        interactive_compatible, bound_host, needs_credentials, credential_ref,
                        config_revision, capability_revision
                     FROM ai_providers WHERE id = ?1",
                    [id],
                    |row| {
                        Ok(DatabaseProviderState {
                            display_name: row.get(0)?,
                            base_url: row.get(1)?,
                            models_url: row.get(2)?,
                            chat_completions_url: row.get(3)?,
                            default_model: row.get(4)?,
                            available_models_json: row.get(5)?,
                            probed_model: row.get(6)?,
                            structured_mode: row.get(7)?,
                            interactive_compatible: row.get(8)?,
                            bound_host: row.get(9)?,
                            needs_credentials: row.get(10)?,
                            credential_ref: row.get(11)?,
                            config_revision: row.get(12)?,
                            capability_revision: row.get(13)?,
                        })
                    },
                )
                .map_err(|_| "TEST_DATABASE_ERROR".to_string())
        })
        .unwrap()
    }

    fn active_credential_ref(db: &Database, id: &str) -> Option<String> {
        database_state(db, id).credential_ref
    }

    fn cleanup_reason(db: &Database, credential_ref: &str) -> Option<String> {
        db.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT reason FROM credential_cleanup WHERE credential_ref = ?1",
                    [credential_ref],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| "TEST_DATABASE_ERROR".to_string())
        })
        .unwrap()
    }

    #[test]
    fn provider_service_uses_the_injected_credential_coordinator() {
        let context = test_context();

        assert!(Arc::ptr_eq(
            &context.service.credential_mutations,
            &context.mutations
        ));
    }

    #[test]
    fn list_and_get_only_return_the_requested_public_provider_kind() {
        let context = test_context();

        let storyboard = context.service.list(ProviderKind::Storyboard).unwrap();
        let reverse_image = context.service.get("reverse-image").unwrap();

        assert_eq!(storyboard.len(), 1);
        assert_eq!(storyboard[0].id, "storyboard");
        assert_eq!(storyboard[0].kind, ProviderKind::Storyboard);
        assert_eq!(reverse_image.kind, ProviderKind::ReverseImage);
    }

    #[test]
    fn save_and_resolve_provider_keeps_secret_out_of_public_data_and_database() {
        let context = test_context();
        let secret = "provider-secret-not-in-sqlite";

        let saved = context
            .service
            .save(
                save_input(
                    "storyboard",
                    ProviderKind::Storyboard,
                    "https://story.example.test",
                ),
                Some(secret),
            )
            .unwrap();
        let resolved = context.service.resolve_for_request("storyboard").unwrap();
        let serialized = serde_json::to_string(&saved).unwrap();
        let state = database_state(&context.db, "storyboard");

        assert!(!saved.needs_credentials);
        assert_eq!(resolved.api_key, secret);
        assert!(!serialized.contains(secret));
        assert!(!serialized.contains("credentialRef"));
        assert!(!format!("{state:?}").contains(secret));
    }

    #[test]
    fn unknown_or_mismatched_provider_save_never_mutates_credentials_or_rows() {
        let context = test_context();
        let before = database_state(&context.db, "storyboard");
        let before_mutations = context.credentials.mutation_count();

        for input in [
            save_input(
                "storyboard",
                ProviderKind::ReverseImage,
                "https://story.example.test",
            ),
            save_input(
                "reverse-image",
                ProviderKind::Storyboard,
                "https://reverse.example.test",
            ),
            save_input(
                "unknown-provider",
                ProviderKind::Storyboard,
                "https://unknown.example.test",
            ),
        ] {
            assert_eq!(
                context
                    .service
                    .save(input, Some("unexpected-key"))
                    .unwrap_err(),
                "PROVIDER_KIND_MISMATCH"
            );
        }

        assert_eq!(database_state(&context.db, "storyboard"), before);
        assert_eq!(context.credentials.mutation_count(), before_mutations);
    }

    #[test]
    fn save_uses_copy_on_write_references_without_mixing_old_and_new_keys() {
        let context = test_context();
        let input = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        context
            .service
            .save(input.clone(), Some("old-key"))
            .unwrap();
        let old_ref = active_credential_ref(&context.db, "storyboard").unwrap();

        context.service.save(input, Some("new-key")).unwrap();
        let new_ref = active_credential_ref(&context.db, "storyboard").unwrap();
        let resolved = context.service.resolve_for_request("storyboard").unwrap();

        assert_ne!(old_ref, new_ref);
        assert!(new_ref.starts_with("provider/storyboard/"));
        assert_eq!(context.credentials.stored_secret(&old_ref), None);
        assert_eq!(
            context.credentials.stored_secret(&new_ref),
            Some("new-key".into())
        );
        assert_eq!(resolved.api_key, "new-key");
    }

    #[test]
    fn failed_retired_key_cleanup_does_not_roll_back_an_accepted_save() {
        let context = test_context();
        let input = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        context
            .service
            .save(input.clone(), Some("old-key"))
            .unwrap();
        let old_ref = active_credential_ref(&context.db, "storyboard").unwrap();
        context.credentials.fail_deletes();

        context.service.save(input, Some("new-key")).unwrap();
        let new_ref = active_credential_ref(&context.db, "storyboard").unwrap();
        let resolved = context.service.resolve_for_request("storyboard").unwrap();

        assert_ne!(new_ref, old_ref);
        assert_eq!(resolved.api_key, "new-key");
        assert_eq!(
            context.credentials.stored_secret(&old_ref),
            Some("old-key".into())
        );
        assert_eq!(
            cleanup_reason(&context.db, &old_ref),
            Some("retired".into())
        );
    }

    #[test]
    fn whitespace_only_key_does_not_replace_an_existing_credential() {
        let context = test_context();
        let input = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        context
            .service
            .save(input.clone(), Some("old-key"))
            .unwrap();
        let old_ref = active_credential_ref(&context.db, "storyboard").unwrap();

        context.service.save(input, Some(" \t ")).unwrap();
        let active_ref = active_credential_ref(&context.db, "storyboard").unwrap();
        let resolved = context.service.resolve_for_request("storyboard").unwrap();

        assert_eq!(active_ref, old_ref);
        assert_eq!(resolved.api_key, "old-key");
    }

    #[test]
    fn candidate_keyring_failure_keeps_the_old_provider_binding_and_journaled_candidate() {
        let context = test_context();
        let input = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        context
            .service
            .save(input.clone(), Some("old-key"))
            .unwrap();
        let before = database_state(&context.db, "storyboard");
        context.credentials.fail_next_set();

        assert_eq!(
            context.service.save(input, Some("new-key")).unwrap_err(),
            "PROVIDER_CREDENTIAL_STORE_UNAVAILABLE"
        );
        let after = database_state(&context.db, "storyboard");
        let resolved = context.service.resolve_for_request("storyboard").unwrap();

        assert_eq!(after, before);
        assert_eq!(resolved.api_key, "old-key");
        let candidate_count: i64 = context
            .db
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM credential_cleanup WHERE reason = 'candidate'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|_| "TEST_DATABASE_ERROR".to_string())
            })
            .unwrap();
        assert_eq!(candidate_count, 1);
    }

    #[test]
    fn non_endpoint_save_does_not_overwrite_a_concurrent_capability_update() {
        let context = test_context();
        let input = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        context
            .service
            .save(input.clone(), Some("old-key"))
            .unwrap();
        let db = context.db.clone();
        context.credentials.run_on_next_set(Arc::new(move || {
            db.with_immediate_transaction(|transaction| {
                transaction
                    .execute(
                        "UPDATE ai_providers SET
                            available_models_json = '[\"fresh-model\"]',
                            probed_model = 'fresh-model',
                            structured_mode = 'strict_json',
                            interactive_compatible = 0,
                            capability_revision = capability_revision + 1
                         WHERE id = 'storyboard'",
                        [],
                    )
                    .map_err(|_| "TEST_DATABASE_ERROR".to_string())?;
                Ok(())
            })
            .unwrap();
        }));

        context.service.save(input, Some("new-key")).unwrap();
        let after = database_state(&context.db, "storyboard");

        assert_eq!(after.available_models_json, "[\"fresh-model\"]");
        assert_eq!(after.probed_model, Some("fresh-model".into()));
        assert_eq!(after.structured_mode, Some("strict_json".into()));
        assert_eq!(after.interactive_compatible, Some(0));
        assert_eq!(after.capability_revision, 10);
    }

    #[test]
    fn endpoint_save_resets_capabilities_from_the_live_revision() {
        let context = test_context();
        let input = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        context.service.save(input, Some("old-key")).unwrap();
        let db = context.db.clone();
        context.credentials.run_on_next_set(Arc::new(move || {
            db.with_immediate_transaction(|transaction| {
                transaction
                    .execute(
                        "UPDATE ai_providers SET capability_revision = capability_revision + 1
                         WHERE id = 'storyboard'",
                        [],
                    )
                    .map_err(|_| "TEST_DATABASE_ERROR".to_string())?;
                Ok(())
            })
            .unwrap();
        }));
        let mut changed = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        changed.base_url = "https://story.example.test/v2".into();
        changed.models_url = "https://story.example.test/v2/models".into();
        changed.chat_completions_url = "https://story.example.test/v2/chat/completions".into();

        context.service.save(changed, Some("new-key")).unwrap();
        let after = database_state(&context.db, "storyboard");

        assert_eq!(after.config_revision, 5);
        assert_eq!(after.capability_revision, 11);
        assert_eq!(after.available_models_json, "[]");
        assert_eq!(after.probed_model, None);
    }

    #[test]
    fn same_origin_path_change_resets_capabilities_and_preserves_the_credential() {
        let context = test_context();
        let original = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        context.service.save(original, Some("old-key")).unwrap();
        let before = database_state(&context.db, "storyboard");
        let mut changed = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://story.example.test",
        );
        changed.base_url = "https://story.example.test/v2".into();
        changed.models_url = "https://story.example.test/v2/models".into();
        changed.chat_completions_url = "https://story.example.test/v2/chat/completions".into();

        let saved = context.service.save(changed, None).unwrap();
        let after = database_state(&context.db, "storyboard");

        assert_eq!(after.credential_ref, before.credential_ref);
        assert!(!saved.needs_credentials);
        assert_eq!(after.available_models_json, "[]");
        assert_eq!(after.probed_model, None);
        assert_eq!(after.structured_mode, None);
        assert_eq!(after.interactive_compatible, None);
        assert_eq!(after.default_model, None);
        assert_eq!(after.config_revision, before.config_revision + 1);
        assert_eq!(after.capability_revision, before.capability_revision + 1);
    }

    #[test]
    fn non_endpoint_save_uses_canonical_urls_without_bumping_revisions() {
        let context = test_context();
        let before = database_state(&context.db, "storyboard");
        let mut input = save_input(
            "storyboard",
            ProviderKind::Storyboard,
            "https://STORY.example.test",
        );
        input.display_name = "Renamed storyboard".into();
        input.default_model = Some("chosen-model".into());

        context.service.save(input, None).unwrap();
        let after = database_state(&context.db, "storyboard");

        assert_eq!(after.base_url, "https://story.example.test/v1");
        assert_eq!(after.config_revision, before.config_revision);
        assert_eq!(after.capability_revision, before.capability_revision);
        assert_eq!(after.default_model, Some("chosen-model".into()));
    }

    #[test]
    fn origin_change_without_a_new_key_detaches_the_old_credential() {
        let context = test_context();
        context
            .service
            .save(
                save_input(
                    "storyboard",
                    ProviderKind::Storyboard,
                    "https://story.example.test",
                ),
                Some("old-key"),
            )
            .unwrap();
        let old_ref = active_credential_ref(&context.db, "storyboard").unwrap();

        let changed = context
            .service
            .save(
                save_input(
                    "storyboard",
                    ProviderKind::Storyboard,
                    "https://new.example.test",
                ),
                None,
            )
            .unwrap();
        let after = database_state(&context.db, "storyboard");

        assert!(changed.needs_credentials);
        assert_eq!(after.credential_ref, None);
        assert_eq!(after.bound_host, Some("https://new.example.test".into()));
        assert_eq!(context.credentials.stored_secret(&old_ref), None);
        let error = match context.service.resolve_for_request("storyboard") {
            Err(error) => error,
            Ok(_) => panic!("changed provider unexpectedly resolved"),
        };
        assert_eq!(error, "PROVIDER_CREDENTIALS_REQUIRED");
    }

    #[test]
    fn clear_credential_commits_the_missing_binding_before_best_effort_cleanup() {
        let context = test_context();
        context
            .service
            .save(
                save_input(
                    "storyboard",
                    ProviderKind::Storyboard,
                    "https://story.example.test",
                ),
                Some("old-key"),
            )
            .unwrap();
        let old_ref = active_credential_ref(&context.db, "storyboard").unwrap();

        context.service.clear_credential("storyboard").unwrap();
        let after = database_state(&context.db, "storyboard");

        assert_eq!(after.credential_ref, None);
        assert_eq!(after.needs_credentials, 1);
        assert_eq!(context.credentials.stored_secret(&old_ref), None);
        let error = match context.service.resolve_for_request("storyboard") {
            Err(error) => error,
            Ok(_) => panic!("cleared provider unexpectedly resolved"),
        };
        assert_eq!(error, "PROVIDER_CREDENTIALS_REQUIRED");
    }

    #[test]
    fn failed_retired_key_cleanup_does_not_roll_back_a_clear() {
        let context = test_context();
        context
            .service
            .save(
                save_input(
                    "storyboard",
                    ProviderKind::Storyboard,
                    "https://story.example.test",
                ),
                Some("old-key"),
            )
            .unwrap();
        let old_ref = active_credential_ref(&context.db, "storyboard").unwrap();
        context.credentials.fail_deletes();

        context.service.clear_credential("storyboard").unwrap();
        let after = database_state(&context.db, "storyboard");

        assert_eq!(after.credential_ref, None);
        assert_eq!(after.needs_credentials, 1);
        assert_eq!(
            context.credentials.stored_secret(&old_ref),
            Some("old-key".into())
        );
        assert_eq!(
            cleanup_reason(&context.db, &old_ref),
            Some("retired".into())
        );
    }

    #[test]
    fn preflight_calls_the_closure_for_missing_credentials_but_resolved_does_not() {
        let context = test_context();
        let mut preflight_called = false;
        context
            .service
            .with_request_preflight("storyboard", |preflight| {
                preflight_called = true;
                assert!(matches!(
                    preflight.credential,
                    Err(SafeCredentialError::MissingBinding)
                ));
                Ok(())
            })
            .unwrap();

        let mut resolved_called = false;
        let error = context
            .service
            .with_resolved_for_request("storyboard", |_| {
                resolved_called = true;
                Ok(())
            })
            .unwrap_err();

        assert!(preflight_called);
        assert!(!resolved_called);
        assert_eq!(error, "PROVIDER_CREDENTIALS_REQUIRED");
    }

    #[test]
    fn missing_bound_host_fails_closed_but_still_calls_preflight() {
        let context = test_context();
        context
            .service
            .save(
                save_input(
                    "storyboard",
                    ProviderKind::Storyboard,
                    "https://story.example.test",
                ),
                Some("old-key"),
            )
            .unwrap();
        context
            .db
            .with_immediate_transaction(|transaction| {
                transaction
                    .execute(
                        "UPDATE ai_providers SET bound_host = NULL WHERE id = 'storyboard'",
                        [],
                    )
                    .map_err(|_| "TEST_DATABASE_ERROR".to_string())?;
                Ok(())
            })
            .unwrap();

        let mut preflight_called = false;
        context
            .service
            .with_request_preflight("storyboard", |preflight| {
                preflight_called = true;
                assert!(matches!(
                    preflight.credential,
                    Err(SafeCredentialError::StoreUnavailable)
                ));
                Ok(())
            })
            .unwrap();
        let error = match context.service.resolve_for_request("storyboard") {
            Err(error) => error,
            Ok(_) => panic!("missing host binding unexpectedly resolved"),
        };

        assert!(preflight_called);
        assert_eq!(error, "PROVIDER_CREDENTIAL_STORE_UNAVAILABLE");
    }

    #[test]
    fn preflight_closure_can_reenter_an_immediate_database_transaction() {
        let context = test_context();
        let service = context.service.clone();
        let db = context.db.clone();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let result = service.with_request_preflight("storyboard", |_| {
                db.with_immediate_transaction(|transaction| {
                    transaction
                        .execute(
                            "UPDATE ai_providers SET display_name = display_name WHERE id = 'storyboard'",
                            [],
                        )
                        .map_err(|_| "TEST_DATABASE_ERROR".to_string())?;
                    Ok(())
                })
            });
            sender.send(result).unwrap();
        });

        assert!(receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .is_ok());
        worker.join().unwrap();
    }

    fn provider_input() -> SaveProviderInput {
        SaveProviderInput {
            id: "storyboard".into(),
            kind: ProviderKind::Storyboard,
            display_name: "Storyboards".into(),
            base_url: "https://api.example.com/v1".into(),
            models_url: "https://api.example.com/v1/models".into(),
            chat_completions_url: "https://api.example.com/v1/chat/completions".into(),
            default_model: Some("glm-5.2".into()),
            confirm_cross_origin: false,
        }
    }

    #[test]
    fn accepts_same_origin_https_paths_and_returns_a_normalized_fingerprint() {
        let mut input = provider_input();
        input.base_url = "https://API.Example.com/v1".into();

        assert_eq!(
            validated_host_fingerprint(&input).unwrap(),
            "https://api.example.com"
        );
    }

    #[test]
    fn rejects_unconfirmed_cross_origin_with_sanitized_canonical_origins() {
        let mut input = provider_input();
        input.base_url = format!("https://api.example.com/{SENTINEL}");
        input.models_url = "https://models.example.net/v1/models".into();

        let error = validated_host_fingerprint(&input).unwrap_err();

        assert_eq!(
            error,
            "CROSS_ORIGIN_CONFIRMATION_REQUIRED: https://api.example.com|https://models.example.net"
        );
        assert!(!error.contains(SENTINEL));
    }

    #[test]
    fn accepts_confirmed_cross_origins_and_returns_sorted_deduplicated_origins() {
        let mut input = provider_input();
        input.base_url = "https://z.example.com/v1".into();
        input.models_url = "https://a.example.com/v1/models".into();
        input.chat_completions_url = "https://z.example.com/v1/chat/completions".into();
        input.confirm_cross_origin = true;

        assert_eq!(
            validated_host_fingerprint(&input).unwrap(),
            "https://a.example.com|https://z.example.com"
        );
    }

    #[test]
    fn rejects_credentials_queries_and_fragments_without_echoing_sensitive_input() {
        for field in ["base", "models", "chat"] {
            for suffix in [
                format!("user:{SENTINEL}@api.example.com/v1"),
                format!("{SENTINEL}@api.example.com/v1"),
                format!("api.example.com/v1?api_key={SENTINEL}"),
                format!("api.example.com/v1#{SENTINEL}"),
            ] {
                let mut input = provider_input();
                let url = format!("https://{suffix}");

                match field {
                    "base" => input.base_url = url,
                    "models" => input.models_url = url,
                    "chat" => input.chat_completions_url = url,
                    _ => unreachable!(),
                }

                let error = validated_host_fingerprint(&input).unwrap_err();
                assert!(
                    !error.contains(SENTINEL),
                    "{field} validation error leaked sensitive input: {error}"
                );
            }
        }
    }

    #[test]
    fn rejects_non_https_non_loopback_urls() {
        let mut input = provider_input();
        input.base_url = "http://api.example.com/v1".into();

        assert_eq!(
            validated_host_fingerprint(&input).unwrap_err(),
            INSECURE_PROVIDER_URL
        );
    }

    #[test]
    fn rejects_malformed_and_hostless_urls_with_a_stable_error_code() {
        for invalid_url in ["not a url", "https://", "mailto:api@example.com"] {
            let mut input = provider_input();
            input.base_url = invalid_url.into();

            assert_eq!(
                validated_host_fingerprint(&input).unwrap_err(),
                INVALID_PROVIDER_URL
            );
        }
    }

    #[test]
    fn accepts_http_only_for_loopback_hosts() {
        for host in ["localhost", "127.0.0.1", "[::1]"] {
            let mut input = provider_input();
            input.base_url = format!("http://{host}/v1");
            input.models_url = format!("http://{host}/v1/models");
            input.chat_completions_url = format!("http://{host}/v1/chat/completions");

            assert!(
                validated_host_fingerprint(&input).is_ok(),
                "expected loopback host {host} to be allowed"
            );
        }
    }

    #[test]
    fn save_provider_input_rejects_unknown_fields() {
        let input = r#"{
            "id": "storyboard",
            "kind": "storyboard",
            "displayName": "Storyboards",
            "baseUrl": "https://api.example.com/v1",
            "modelsUrl": "https://api.example.com/v1/models",
            "chatCompletionsUrl": "https://api.example.com/v1/chat/completions",
            "defaultModel": "glm-5.2",
            "unexpected": true
        }"#;

        assert!(serde_json::from_str::<SaveProviderInput>(input).is_err());
    }
}
