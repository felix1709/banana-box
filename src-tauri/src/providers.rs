use serde::{Deserialize, Serialize};
use url::Url;

const INVALID_PROVIDER_URL: &str = "INVALID_PROVIDER_URL";
const INSECURE_PROVIDER_URL: &str = "INSECURE_PROVIDER_URL";
const CROSS_ORIGIN_CONFIRMATION_REQUIRED: &str = "CROSS_ORIGIN_CONFIRMATION_REQUIRED";

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

#[derive(Clone, Debug, Deserialize)]
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

pub(crate) fn validated_host_fingerprint(input: &SaveProviderInput) -> Result<String, String> {
    let base = parse_provider_url(&input.base_url)?;
    let models = parse_provider_url(&input.models_url)?;
    let chat = parse_provider_url(&input.chat_completions_url)?;

    for endpoint in [&base, &models, &chat] {
        validate_provider_url(endpoint)?;
    }

    if (base.origin() != models.origin() || base.origin() != chat.origin())
        && !input.confirm_cross_origin
    {
        return Err(CROSS_ORIGIN_CONFIRMATION_REQUIRED.into());
    }

    let mut origins = [&base, &models, &chat]
        .into_iter()
        .map(|endpoint| endpoint.origin().ascii_serialization().to_ascii_lowercase())
        .collect::<Vec<_>>();
    origins.sort();
    origins.dedup();

    Ok(origins.join("|"))
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

    const SENTINEL: &str = "TEST_ONLY_DO_NOT_USE";

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
    fn rejects_unconfirmed_cross_origin_without_echoing_the_url() {
        let mut input = provider_input();
        input.models_url = format!("https://{SENTINEL}.models.example.net/v1/models");

        let error = validated_host_fingerprint(&input).unwrap_err();

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

        assert!(validated_host_fingerprint(&input).is_err());
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
