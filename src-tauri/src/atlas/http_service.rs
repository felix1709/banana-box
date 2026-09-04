use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use crate::atlas::ingest;
use crate::atlas::repository::AtlasRepository;

pub struct AtlasHttpService {
    running: Arc<AtomicBool>,
    analyzer: Arc<Mutex<Option<AtlasAnalyzer>>>,
}

type AtlasAnalyzer = Arc<dyn Fn(String, std::path::PathBuf) + Send + Sync>;

impl Default for AtlasHttpService {
    fn default() -> Self {
        Self::new()
    }
}

impl AtlasHttpService {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            analyzer: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn start(
        &self,
        root: std::path::PathBuf,
        token: String,
        analyzer: Option<AtlasAnalyzer>,
    ) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let running = self.running.clone();
        *self.analyzer.lock().map_err(|_| "ATLAS_ANALYZER_LOCK".to_string())? = analyzer;
        let analyzer = self.analyzer.lock().map_err(|_| "ATLAS_ANALYZER_LOCK".to_string())?.clone();
        let _token = token;

        std::thread::spawn(move || {
            let server = match tiny_http::Server::http("127.0.0.1:41773") {
                Ok(server) => server,
                Err(_) => {
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            while running.load(Ordering::SeqCst) {
                match server.recv_timeout(Duration::from_millis(250)) {
                    Ok(Some(request)) => handle_request(&root, analyzer.clone(), request),
                    Ok(None) => {}
                    Err(_) => {}
                }
            }
        });
        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

fn handle_request(
    root: &std::path::Path,
    analyzer: Option<AtlasAnalyzer>,
    mut request: tiny_http::Request,
) {
    if request.method() == &tiny_http::Method::Options {
        respond_text(request, String::new(), 204);
        return;
    }

    let path = request.url().to_string();
    if path == "/v1/health" {
        respond_text(request, "ok".to_string(), 200);
        return;
    }

    if path == "/v1/ingest" && request.method() == &tiny_http::Method::Post {
        let mut body = String::new();
        let _ = request.as_reader().read_to_string(&mut body);
        let payload: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(_) => {
                respond_text(request, "invalid_json".to_string(), 400);
                return;
            }
        };

        match ingest_from_payload(root, &payload) {
            Ok(entry_id) => {
                let body = serde_json::json!({ "status": "queued", "entryId": entry_id });
                respond_text(request, body.to_string(), 200);
                if let Some(analyzer) = analyzer {
                    analyzer(entry_id, root.to_path_buf());
                }
            }
            Err(message) => {
                respond_text(request, message, 500);
            }
        }
        return;
    }

    respond_text(request, "not_found".to_string(), 404);
}

fn respond_text(request: tiny_http::Request, body: String, status: u16) {
    let mut response = tiny_http::Response::from_string(body).with_status_code(status);
    for header in cors_headers() {
        response.add_header(header);
    }
    let _ = request.respond(response);
}

fn cors_headers() -> Vec<tiny_http::Header> {
    vec![
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        tiny_http::Header::from_bytes(
            &b"Access-Control-Allow-Methods"[..],
            &b"POST, GET, OPTIONS"[..],
        )
        .unwrap(),
        tiny_http::Header::from_bytes(
            &b"Access-Control-Allow-Headers"[..],
            &b"Content-Type"[..],
        )
        .unwrap(),
    ]
}

pub fn ingest_from_payload(
    root: &std::path::Path,
    payload: &serde_json::Value,
) -> Result<String, String> {
    let source_url = payload.get("sourceUrl").and_then(|v| v.as_str());
    if let Some(source_url) = source_url {
        let repo = AtlasRepository::new(root.to_path_buf());
        if let Some(existing_id) = repo.find_entry_by_source_url(source_url)? {
            return Ok(existing_id);
        }
    }

    let bytes = if let Some(local_path) = payload.get("localPath").and_then(|v| v.as_str()) {
        std::fs::read(local_path).map_err(|e| e.to_string())?
    } else if let Some(source_url) = source_url {
        reqwest::blocking::get(source_url)
            .and_then(|response| response.bytes())
            .map(|bytes| bytes.to_vec())
            .map_err(|e| e.to_string())?
    } else if let Some(image_base64) = payload.get("imageBase64").and_then(|v| v.as_str()) {
        let encoded = image_base64
            .split_once(',')
            .map(|(_, data)| data)
            .unwrap_or(image_base64);
        base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .map_err(|_| "invalid_image_base64".to_string())?
    } else {
        return Err("missing_local_path_or_source_url_or_image_base64".into());
    };

    let dimension = payload
        .get("dimension")
        .and_then(|v| v.as_str())
        .unwrap_or("05-lighting-atmosphere");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    let entry_id = format!("atlas-{}-{}", now.as_secs(), now.subsec_nanos());
    let compressed = ingest::compress_image_to_webp(&bytes, 1280)?;
    ingest::save_ingested_asset(root, dimension, &entry_id, &compressed)?;
    let thumbnail = ingest::compress_image_to_webp(&bytes, 320)?;
    ingest::save_thumbnail_asset(root, &entry_id, &thumbnail)?;
    let frontmatter = ingest::build_frontmatter_with_source(
        &entry_id,
        "pending",
        dimension,
        &[],
        "pending_review",
        source_url,
    );
    ingest::write_entry_md(root, &entry_id, dimension, &frontmatter, "## 分析\n待识图")?;
    Ok(entry_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_path_or_url_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let payload = serde_json::json!({});
        assert_eq!(
            ingest_from_payload(root.path(), &payload),
            Err("missing_local_path_or_source_url_or_image_base64".into())
        );
    }
}
