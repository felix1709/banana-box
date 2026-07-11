use reqwest::{redirect::Policy, Client, RequestBuilder, Response};
use std::{future::Future, time::Duration};
use tokio_util::sync::CancellationToken;
use url::Url;

pub const MAX_PROVIDER_MODELS_BODY_BYTES: usize = 1024 * 1024;
pub const MAX_PROVIDER_MODELS: usize = 512;
pub const MAX_MODEL_ID_BYTES: usize = 256;
pub const MAX_REVERSE_IMAGE_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_REVERSE_IMAGE_CONTENT_BYTES: usize = 1024 * 1024;

const PROVIDER_TIMEOUT: &str = "PROVIDER_TIMEOUT";
const PROVIDER_REQUEST_CANCELLED: &str = "PROVIDER_REQUEST_CANCELLED";
const PROVIDER_REDIRECT_FORBIDDEN: &str = "PROVIDER_REDIRECT_FORBIDDEN";
const PROVIDER_RESPONSE_TOO_LARGE: &str = "PROVIDER_RESPONSE_TOO_LARGE";
const PROVIDER_HTTP_ERROR: &str = "PROVIDER_HTTP_ERROR";
const PROVIDER_REQUEST_FAILED: &str = "PROVIDER_REQUEST_FAILED";
const PROVIDER_CLIENT_UNAVAILABLE: &str = "PROVIDER_CLIENT_UNAVAILABLE";

#[derive(Clone, Copy, Debug)]
pub struct ProviderHttpTimeouts {
    pub response_header: Duration,
    pub idle: Duration,
    pub total_non_streaming: Duration,
}

impl Default for ProviderHttpTimeouts {
    fn default() -> Self {
        Self {
            response_header: Duration::from_secs(30),
            idle: Duration::from_secs(30),
            total_non_streaming: Duration::from_secs(60),
        }
    }
}

pub struct ProviderHttpClient {
    client: Client,
    timeouts: ProviderHttpTimeouts,
}

pub struct ProviderResponseStream {
    response: Option<Response>,
    terminal_error: Option<String>,
    cancellation: CancellationToken,
    idle_timeout: Duration,
    decoded_limit: usize,
    decoded_bytes: usize,
}

impl ProviderHttpClient {
    pub fn new() -> Result<Self, String> {
        Self::with_timeouts_for_tests(ProviderHttpTimeouts::default())
    }

    pub(crate) fn with_timeouts_for_tests(timeouts: ProviderHttpTimeouts) -> Result<Self, String> {
        let client = Client::builder()
            .redirect(Policy::none())
            // Provider credentials must never be forwarded through a system proxy.
            .no_proxy()
            .connect_timeout(Duration::from_secs(10))
            .retry(reqwest::retry::never())
            .build()
            .map_err(|_| PROVIDER_CLIENT_UNAVAILABLE.to_string())?;
        Ok(Self { client, timeouts })
    }

    pub async fn get_bounded(
        &self,
        url: Url,
        api_key: &str,
        decoded_limit: usize,
        cancellation: CancellationToken,
    ) -> Result<Vec<u8>, String> {
        let request = self.client.get(url).bearer_auth(api_key);
        self.read_non_streaming(request, decoded_limit, cancellation)
            .await
    }

    pub async fn post_json_bounded(
        &self,
        url: Url,
        api_key: &str,
        body: serde_json::Value,
        decoded_limit: usize,
        cancellation: CancellationToken,
    ) -> Result<Vec<u8>, String> {
        let request = self.client.post(url).bearer_auth(api_key).json(&body);
        self.read_non_streaming(request, decoded_limit, cancellation)
            .await
    }

    pub async fn post_json_stream(
        &self,
        url: Url,
        api_key: &str,
        body: serde_json::Value,
        decoded_limit: usize,
        cancellation: CancellationToken,
    ) -> Result<ProviderResponseStream, String> {
        let request = self.client.post(url).bearer_auth(api_key).json(&body);
        let response = self.send_request(request, &cancellation).await?;
        reject_declared_response_that_exceeds_limit(&response, decoded_limit)?;
        Ok(ProviderResponseStream::new(
            response,
            cancellation,
            self.timeouts.idle,
            decoded_limit,
        ))
    }

    async fn read_non_streaming(
        &self,
        request: RequestBuilder,
        decoded_limit: usize,
        cancellation: CancellationToken,
    ) -> Result<Vec<u8>, String> {
        if cancellation.is_cancelled() {
            return Err(PROVIDER_REQUEST_CANCELLED.into());
        }

        let total_cancellation = cancellation.clone();
        let read = async {
            let response = self.send_request(request, &total_cancellation).await?;
            reject_declared_response_that_exceeds_limit(&response, decoded_limit)?;
            collect_response(
                response,
                total_cancellation,
                self.timeouts.idle,
                decoded_limit,
            )
            .await
        };

        select_cancel_or_timeout(&cancellation, self.timeouts.total_non_streaming, read).await?
    }

    async fn send_request(
        &self,
        request: RequestBuilder,
        cancellation: &CancellationToken,
    ) -> Result<Response, String> {
        if cancellation.is_cancelled() {
            return Err(PROVIDER_REQUEST_CANCELLED.into());
        }

        let response =
            select_cancel_or_timeout(cancellation, self.timeouts.response_header, request.send())
                .await?
                .map_err(map_request_error)?;
        if response.status().is_redirection() {
            return Err(PROVIDER_REDIRECT_FORBIDDEN.into());
        }
        if !response.status().is_success() {
            return Err(PROVIDER_HTTP_ERROR.into());
        }
        Ok(response)
    }
}

fn reject_declared_response_that_exceeds_limit(
    response: &Response,
    decoded_limit: usize,
) -> Result<(), String> {
    if response
        .content_length()
        .is_some_and(|content_length| content_length > decoded_limit as u64)
    {
        return Err(PROVIDER_RESPONSE_TOO_LARGE.into());
    }
    Ok(())
}

impl ProviderResponseStream {
    fn new(
        response: Response,
        cancellation: CancellationToken,
        idle_timeout: Duration,
        decoded_limit: usize,
    ) -> Self {
        Self {
            response: Some(response),
            terminal_error: None,
            cancellation,
            idle_timeout,
            decoded_limit,
            decoded_bytes: 0,
        }
    }

    pub async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, String> {
        if let Some(error) = &self.terminal_error {
            return Err(error.clone());
        }
        if self.cancellation.is_cancelled() {
            return Err(self.terminate(PROVIDER_REQUEST_CANCELLED.into()));
        }

        let cancellation = self.cancellation.clone();
        let Some(response) = self.response.as_mut() else {
            return Ok(None);
        };
        let next = response.chunk();
        let next = match select_cancel_or_timeout(&cancellation, self.idle_timeout, next).await {
            Ok(Ok(next)) => next,
            Ok(Err(error)) => return Err(self.terminate(map_request_error(error))),
            Err(error) => return Err(self.terminate(error)),
        };
        match next {
            Some(chunk) => {
                let Some(next_size) = self.decoded_bytes.checked_add(chunk.len()) else {
                    return Err(self.terminate(PROVIDER_RESPONSE_TOO_LARGE.into()));
                };
                if next_size > self.decoded_limit {
                    return Err(self.terminate(PROVIDER_RESPONSE_TOO_LARGE.into()));
                }
                self.decoded_bytes = next_size;
                Ok(Some(chunk.to_vec()))
            }
            None => {
                self.response.take();
                Ok(None)
            }
        }
    }

    fn terminate(&mut self, error: String) -> String {
        self.response.take();
        self.terminal_error = Some(error.clone());
        error
    }
}

async fn collect_response(
    response: Response,
    cancellation: CancellationToken,
    idle_timeout: Duration,
    decoded_limit: usize,
) -> Result<Vec<u8>, String> {
    let mut stream =
        ProviderResponseStream::new(response, cancellation, idle_timeout, decoded_limit);
    let mut body = Vec::new();
    while let Some(chunk) = stream.next_chunk().await? {
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

async fn select_cancel_or_timeout<T>(
    cancellation: &CancellationToken,
    timeout: Duration,
    future: impl Future<Output = T>,
) -> Result<T, String> {
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => Err(PROVIDER_REQUEST_CANCELLED.into()),
        result = tokio::time::timeout(timeout, future) => match result {
            Ok(value) => Ok(value),
            Err(_) => Err(PROVIDER_TIMEOUT.into()),
        },
    }
}

fn map_request_error(error: reqwest::Error) -> String {
    if error.is_redirect() {
        PROVIDER_REDIRECT_FORBIDDEN.into()
    } else if error.is_timeout() {
        PROVIDER_TIMEOUT.into()
    } else {
        PROVIDER_REQUEST_FAILED.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        thread,
        time::Duration,
    };
    use tokio_util::sync::CancellationToken;
    use url::Url;

    const SHORT_TIMEOUT: Duration = Duration::from_millis(10);
    const OBSERVATION_WINDOW: Duration = Duration::from_millis(30);

    #[derive(Clone, Debug)]
    struct RecordedRequest {
        method: String,
        authorization_present: bool,
    }

    type Handler = dyn Fn(&RecordedRequest, &mut TcpStream, &AtomicBool) + Send + Sync;

    struct TestServer {
        url: Url,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        stop: Arc<AtomicBool>,
        worker: Option<thread::JoinHandle<()>>,
    }

    impl TestServer {
        fn start(
            handler: impl Fn(&RecordedRequest, &mut TcpStream, &AtomicBool) + Send + Sync + 'static,
        ) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let recorded_requests = requests.clone();
            let worker_stop = stop.clone();
            let handler: Arc<Handler> = Arc::new(handler);
            let worker = thread::spawn(move || {
                while !worker_stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream.set_nonblocking(false).unwrap();
                            if let Some(request) = read_request(&mut stream) {
                                recorded_requests.lock().unwrap().push(request.clone());
                                handler(&request, &mut stream, &worker_stop);
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(1));
                        }
                        Err(_) => break,
                    }
                }
            });

            Self {
                url,
                requests,
                stop,
                worker: Some(worker),
            }
        }

        fn url(&self) -> Url {
            self.url.clone()
        }

        fn request_count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }

        fn request_methods(&self) -> Vec<String> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .map(|request| request.method.clone())
                .collect()
        }

        fn all_requests_lack_authorization(&self) -> bool {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .all(|request| !request.authorization_present)
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> Option<RecordedRequest> {
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .ok()?;
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = stream.read(&mut buffer).ok()?;
            if read == 0 {
                return None;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let headers_end = bytes.windows(4).position(|window| window == b"\r\n\r\n")?;
        let headers = std::str::from_utf8(&bytes[..headers_end]).ok()?.to_owned();
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
            })
            .unwrap_or_default();
        let chunked = headers.lines().any(|line| {
            line.split_once(':')
                .map(|(name, value)| {
                    name.eq_ignore_ascii_case("transfer-encoding")
                        && value.trim().eq_ignore_ascii_case("chunked")
                })
                .unwrap_or(false)
        });
        let expected_bytes = headers_end + 4 + content_length;
        while bytes.len() < expected_bytes
            || (chunked
                && !bytes[headers_end + 4..]
                    .windows(5)
                    .any(|window| window == b"0\r\n\r\n"))
        {
            let read = stream.read(&mut buffer).ok()?;
            if read == 0 {
                return None;
            }
            bytes.extend_from_slice(&buffer[..read]);
        }
        let mut lines = headers.lines();
        let method = lines.next()?.split_whitespace().next()?.to_string();
        let authorization_present = lines.any(|line| {
            line.split_once(':')
                .map(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                .unwrap_or(false)
        });
        Some(RecordedRequest {
            method,
            authorization_present,
        })
    }

    fn write_response(
        stream: &mut TcpStream,
        status: u16,
        headers: &[(&str, String)],
        body: &[u8],
    ) {
        let mut response = format!("HTTP/1.1 {status} Test\r\nConnection: close\r\n");
        for (name, value) in headers {
            response.push_str(name);
            response.push_str(": ");
            response.push_str(value);
            response.push_str("\r\n");
        }
        response.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
        stream.write_all(response.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
        stream.flush().unwrap();
    }

    fn write_chunked(stream: &mut TcpStream, chunks: &[&[u8]], hold_open: bool, stop: &AtomicBool) {
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        for chunk in chunks {
            stream
                .write_all(format!("{:X}\r\n", chunk.len()).as_bytes())
                .unwrap();
            stream.write_all(chunk).unwrap();
            stream.write_all(b"\r\n").unwrap();
            stream.flush().unwrap();
        }
        if hold_open {
            while !stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(1));
            }
        } else {
            stream.write_all(b"0\r\n\r\n").unwrap();
            stream.flush().unwrap();
        }
    }

    fn write_slow_chunked(
        stream: &mut TcpStream,
        chunks: &[&[u8]],
        pause: Duration,
        stop: &AtomicBool,
    ) {
        if stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .is_err()
        {
            return;
        }
        for chunk in chunks {
            if stop.load(Ordering::SeqCst)
                || stream
                    .write_all(format!("{:X}\r\n", chunk.len()).as_bytes())
                    .is_err()
                || stream.write_all(chunk).is_err()
                || stream.write_all(b"\r\n").is_err()
                || stream.flush().is_err()
            {
                return;
            }
            thread::sleep(pause);
        }
        let _ = stream.write_all(b"0\r\n\r\n");
    }

    fn wait_for_follow_up_window() {
        thread::sleep(OBSERVATION_WINDOW);
    }

    fn test_client(timeouts: ProviderHttpTimeouts) -> ProviderHttpClient {
        ProviderHttpClient::with_timeouts_for_tests(timeouts).unwrap()
    }

    fn short_timeouts() -> ProviderHttpTimeouts {
        ProviderHttpTimeouts {
            response_header: SHORT_TIMEOUT,
            idle: SHORT_TIMEOUT,
            total_non_streaming: SHORT_TIMEOUT,
        }
    }

    #[tokio::test]
    async fn redirects_are_never_followed_for_get_or_post() {
        for status in [301, 302, 307, 308] {
            for post in [false, true] {
                let same_origin = TestServer::start(move |_, stream, _| {
                    write_response(stream, status, &[("Location", "/next".into())], b"");
                });
                let client = test_client(ProviderHttpTimeouts::default());
                let cancellation = CancellationToken::new();
                let error = if post {
                    client
                        .post_json_bounded(
                            same_origin.url(),
                            "test-key",
                            serde_json::json!({"hello": "world"}),
                            1024,
                            cancellation,
                        )
                        .await
                        .unwrap_err()
                } else {
                    client
                        .get_bounded(same_origin.url(), "test-key", 1024, cancellation)
                        .await
                        .unwrap_err()
                };
                assert_eq!(
                    error, "PROVIDER_REDIRECT_FORBIDDEN",
                    "same-origin redirect status={status}, post={post}"
                );
                wait_for_follow_up_window();
                assert_eq!(same_origin.request_count(), 1);
                assert_eq!(
                    same_origin.request_methods(),
                    vec![if post { "POST" } else { "GET" }.to_string()]
                );

                let target = TestServer::start(|_, stream, _| {
                    write_response(stream, 200, &[], b"unexpected");
                });
                let location = target.url().to_string();
                let cross_origin = TestServer::start(move |_, stream, _| {
                    write_response(stream, status, &[("Location", location.clone())], b"");
                });
                let cancellation = CancellationToken::new();
                let error = if post {
                    client
                        .post_json_bounded(
                            cross_origin.url(),
                            "test-key",
                            serde_json::json!({"hello": "world"}),
                            1024,
                            cancellation,
                        )
                        .await
                        .unwrap_err()
                } else {
                    client
                        .get_bounded(cross_origin.url(), "test-key", 1024, cancellation)
                        .await
                        .unwrap_err()
                };
                assert_eq!(
                    error, "PROVIDER_REDIRECT_FORBIDDEN",
                    "cross-origin redirect status={status}, post={post}"
                );
                wait_for_follow_up_window();
                assert_eq!(target.request_count(), 0);
                assert!(target.all_requests_lack_authorization());
            }
        }
    }

    #[tokio::test]
    async fn reads_a_normal_chunked_response_with_a_decoded_limit() {
        let server = TestServer::start(|_, stream, stop| {
            write_chunked(stream, &[b"hello ", b"world"], false, stop);
        });
        let body = test_client(ProviderHttpTimeouts::default())
            .get_bounded(server.url(), "test-key", 64, CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(body, b"hello world");
    }

    #[tokio::test]
    async fn reads_normal_stream_chunks_and_then_eof() {
        let server = TestServer::start(|_, stream, stop| {
            write_chunked(stream, &[b"hello ", b"world"], false, stop);
        });
        let mut stream = test_client(ProviderHttpTimeouts::default())
            .post_json_stream(
                server.url(),
                "test-key",
                serde_json::json!({"hello": "world"}),
                64,
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(stream.next_chunk().await.unwrap(), Some(b"hello ".to_vec()));
        assert_eq!(stream.next_chunk().await.unwrap(), Some(b"world".to_vec()));
        assert_eq!(stream.next_chunk().await.unwrap(), None);
    }

    #[tokio::test]
    async fn rejects_a_response_that_exceeds_the_decoded_limit() {
        let server = TestServer::start(|_, stream, stop| {
            write_chunked(stream, &[b"12345", b"6"], false, stop);
        });
        let error = test_client(ProviderHttpTimeouts::default())
            .get_bounded(server.url(), "test-key", 5, CancellationToken::new())
            .await
            .unwrap_err();

        assert_eq!(error, "PROVIDER_RESPONSE_TOO_LARGE");
    }

    #[tokio::test]
    async fn oversized_stream_is_terminal_after_the_limit_error() {
        let server = TestServer::start(|_, stream, stop| {
            write_chunked(stream, &[b"12345", b"6"], false, stop);
        });
        let mut stream = test_client(ProviderHttpTimeouts::default())
            .post_json_stream(
                server.url(),
                "test-key",
                serde_json::json!({"hello": "world"}),
                5,
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(stream.next_chunk().await.unwrap(), Some(b"12345".to_vec()));
        assert_eq!(
            stream.next_chunk().await.unwrap_err(),
            "PROVIDER_RESPONSE_TOO_LARGE"
        );
        assert_eq!(
            stream.next_chunk().await.unwrap_err(),
            "PROVIDER_RESPONSE_TOO_LARGE"
        );
    }

    #[tokio::test]
    async fn rejects_gzip_content_that_exceeds_the_decoded_limit() {
        let compressed = [
            31_u8, 139, 8, 0, 0, 0, 0, 0, 4, 0, 115, 116, 28, 217, 0, 0, 19, 91, 151, 73, 0, 1, 0,
            0,
        ];
        let server = TestServer::start(move |_, stream, _| {
            write_response(
                stream,
                200,
                &[("Content-Encoding", "gzip".into())],
                &compressed,
            );
        });

        let error = test_client(ProviderHttpTimeouts::default())
            .get_bounded(server.url(), "test-key", 64, CancellationToken::new())
            .await
            .unwrap_err();

        assert_eq!(error, "PROVIDER_RESPONSE_TOO_LARGE");
    }

    #[tokio::test]
    async fn body_errors_are_terminal() {
        let server = TestServer::start(|_, stream, _| {
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nnot-a-chunk\r\n",
                )
                .unwrap();
            stream.flush().unwrap();
        });
        let mut stream = test_client(ProviderHttpTimeouts::default())
            .post_json_stream(
                server.url(),
                "test-key",
                serde_json::json!({"hello": "world"}),
                64,
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(
            stream.next_chunk().await.unwrap_err(),
            "PROVIDER_REQUEST_FAILED"
        );
        assert_eq!(
            stream.next_chunk().await.unwrap_err(),
            "PROVIDER_REQUEST_FAILED"
        );
    }

    #[tokio::test]
    async fn rejects_a_declared_response_that_exceeds_the_decoded_limit() {
        let server = TestServer::start(|_, stream, _| {
            write_response(stream, 200, &[], b"123456");
        });
        let error = test_client(ProviderHttpTimeouts::default())
            .get_bounded(server.url(), "test-key", 5, CancellationToken::new())
            .await
            .unwrap_err();

        assert_eq!(error, "PROVIDER_RESPONSE_TOO_LARGE");
    }

    #[tokio::test]
    async fn cancellation_before_send_does_not_contact_the_server() {
        let server = TestServer::start(|_, stream, _| {
            write_response(stream, 200, &[], b"unexpected");
        });
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let error = test_client(ProviderHttpTimeouts::default())
            .get_bounded(server.url(), "test-key", 64, cancellation)
            .await
            .unwrap_err();
        assert_eq!(error, "PROVIDER_REQUEST_CANCELLED");
        wait_for_follow_up_window();
        assert_eq!(server.request_count(), 0);
    }

    #[tokio::test]
    async fn cancellation_during_stream_read_returns_a_stable_code() {
        let server = TestServer::start(|_, stream, stop| {
            write_chunked(stream, &[b"first"], true, stop);
        });
        let cancellation = CancellationToken::new();
        let client = test_client(ProviderHttpTimeouts {
            response_header: Duration::from_secs(1),
            idle: Duration::from_secs(1),
            total_non_streaming: Duration::from_secs(1),
        });
        let mut stream = client
            .post_json_stream(
                server.url(),
                "test-key",
                serde_json::json!({"hello": "world"}),
                64,
                cancellation.clone(),
            )
            .await
            .unwrap();
        assert_eq!(stream.next_chunk().await.unwrap(), Some(b"first".to_vec()));
        cancellation.cancel();

        assert_eq!(
            stream.next_chunk().await.unwrap_err(),
            "PROVIDER_REQUEST_CANCELLED"
        );
        assert_eq!(
            stream.next_chunk().await.unwrap_err(),
            "PROVIDER_REQUEST_CANCELLED"
        );
    }

    #[tokio::test]
    async fn injected_header_idle_and_total_timeouts_are_stable() {
        let header_stall = TestServer::start(|_, _stream, stop| {
            while !stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(1));
            }
        });
        let client = test_client(short_timeouts());
        assert_eq!(
            client
                .get_bounded(header_stall.url(), "test-key", 64, CancellationToken::new())
                .await
                .unwrap_err(),
            "PROVIDER_TIMEOUT"
        );

        let idle_stall = TestServer::start(|_, stream, stop| {
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            stream.flush().unwrap();
            while !stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(1));
            }
        });
        let mut stream = client
            .post_json_stream(
                idle_stall.url(),
                "test-key",
                serde_json::json!({"hello": "world"}),
                64,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(stream.next_chunk().await.unwrap_err(), "PROVIDER_TIMEOUT");
        assert_eq!(stream.next_chunk().await.unwrap_err(), "PROVIDER_TIMEOUT");

        let total_stall = TestServer::start(|_, stream, stop| {
            write_slow_chunked(
                stream,
                &[b"x".as_slice(); 64],
                Duration::from_millis(1),
                stop,
            );
        });
        let total_timeout_client = test_client(ProviderHttpTimeouts {
            response_header: Duration::from_millis(100),
            idle: Duration::from_millis(200),
            total_non_streaming: Duration::from_millis(30),
        });
        assert_eq!(
            total_timeout_client
                .get_bounded(total_stall.url(), "test-key", 64, CancellationToken::new())
                .await
                .unwrap_err(),
            "PROVIDER_TIMEOUT"
        );
    }

    #[test]
    fn exposes_the_foundation_provider_limits() {
        assert_eq!(MAX_PROVIDER_MODELS_BODY_BYTES, 1024 * 1024);
        assert_eq!(MAX_PROVIDER_MODELS, 512);
        assert_eq!(MAX_MODEL_ID_BYTES, 256);
        assert_eq!(MAX_REVERSE_IMAGE_RESPONSE_BYTES, 2 * 1024 * 1024);
        assert_eq!(MAX_REVERSE_IMAGE_CONTENT_BYTES, 1024 * 1024);
    }
}
