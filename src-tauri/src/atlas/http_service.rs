use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::atlas::ingest;

pub struct AtlasHttpService {
    running: Arc<AtomicBool>,
}

impl Default for AtlasHttpService {
    fn default() -> Self {
        Self::new()
    }
}

impl AtlasHttpService {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn start(&self, root: std::path::PathBuf, token: String) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let running = self.running.clone();
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
                    Ok(Some(request)) => handle_request(&root, request),
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

fn handle_request(root: &std::path::Path, mut request: tiny_http::Request) {
    let path = request.url().to_string();
    if path == "/v1/health" {
        let _ = request.respond(tiny_http::Response::from_string("ok"));
        return;
    }

    if path == "/v1/ingest" && request.method() == &tiny_http::Method::Post {
        let mut body = String::new();
        let _ = request.as_reader().read_to_string(&mut body);
        let payload: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(_) => {
                let _ = request.respond(
                    tiny_http::Response::from_string("invalid_json").with_status_code(400),
                );
                return;
            }
        };

        match ingest_from_payload(root, &payload) {
            Ok(entry_id) => {
                let body = serde_json::json!({ "status": "queued", "entryId": entry_id });
                let _ = request.respond(tiny_http::Response::from_string(body.to_string()));
            }
            Err(message) => {
                let _ = request.respond(
                    tiny_http::Response::from_string(message).with_status_code(500),
                );
            }
        }
        return;
    }

    let _ = request.respond(tiny_http::Response::from_string("not_found").with_status_code(404));
}

fn ingest_from_payload(root: &std::path::Path, payload: &serde_json::Value) -> Result<String, String> {
    let bytes = if let Some(local_path) = payload.get("localPath").and_then(|v| v.as_str()) {
        std::fs::read(local_path).map_err(|e| e.to_string())?
    } else if let Some(source_url) = payload.get("sourceUrl").and_then(|v| v.as_str()) {
        reqwest::blocking::get(source_url)
            .and_then(|response| response.bytes())
            .map(|bytes| bytes.to_vec())
            .map_err(|e| e.to_string())?
    } else {
        return Err("missing_local_path_or_source_url".into());
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
    let frontmatter = ingest::build_frontmatter(&entry_id, "pending", dimension, &[], "pending_review");
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
            Err("missing_local_path_or_source_url".into())
        );
    }
}
