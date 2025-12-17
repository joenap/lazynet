//! Lazynet: Lazy-evaluated HTTP requests with async Rust pipeline.
//!
//! This module provides a sync-to-async bridge for making concurrent HTTP requests.
//! URLs are sent into the pipeline, and responses are pulled out one at a time.

use crate::http_client::{HttpClient, ReqwestClient};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::task::TaskTracker;

/// Default request timeout in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Request message sent into the pipeline.
#[derive(Clone)]
pub enum RequestMsg {
    Element(String),
    End,
}

/// Response message received from the pipeline.
pub enum ResponseMsg {
    Element(Response),
    End,
}

/// HTTP response with metadata.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields used by lib.rs, not bench_runner
pub struct Response {
    pub request: String,
    pub status: u16,
    pub reason: String,
    pub text: String,
    pub error: Option<String>,
}

impl Response {
    /// Create a successful response.
    pub fn success(request: String, status: u16, reason: String, text: String) -> Self {
        Response {
            request,
            status,
            reason,
            text,
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(request: String, error: String) -> Self {
        Response {
            request,
            status: 0,
            reason: String::new(),
            text: String::new(),
            error: Some(error),
        }
    }
}

/// Lazynet pipeline manager.
///
/// Owns a tokio runtime and channels for sending requests and receiving responses.
pub struct Lazynet {
    _rt: tokio::runtime::Runtime,
    cross_request_sender: crossbeam_channel::Sender<RequestMsg>,
    cross_response_receiver: crossbeam_channel::Receiver<ResponseMsg>,
}

/// Shared HTTP client with its own runtime for connection pooling.
#[allow(dead_code)] // Used by lib.rs, not bench_runner
pub struct SharedClient {
    rt: tokio::runtime::Runtime,
    client: reqwest::Client,
}

#[allow(dead_code)] // Used by lib.rs, not bench_runner
impl SharedClient {
    /// Create a new shared HTTP client with its own runtime and default timeout.
    pub fn new() -> Self {
        Self::with_timeout(DEFAULT_TIMEOUT_SECS)
    }

    /// Create a new shared HTTP client with a custom timeout in seconds.
    pub fn with_timeout(timeout_secs: u64) -> Self {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        SharedClient { rt, client }
    }

    /// Make requests using this client's runtime. Returns a receiver for responses.
    pub fn get(
        &self,
        urls: Vec<String>,
        concurrency_limit: usize,
    ) -> crossbeam_channel::Receiver<ResponseMsg> {
        let buf_size = 100;

        let (async_request_sender, async_request_receiver) =
            tokio::sync::mpsc::channel::<RequestMsg>(buf_size);
        let (async_response_sender, async_response_receiver) =
            tokio::sync::mpsc::channel::<ResponseMsg>(buf_size);
        let (cross_response_sender, cross_response_receiver) =
            crossbeam_channel::bounded::<ResponseMsg>(buf_size);

        let http_client = ReqwestClient::from_client(self.client.clone());

        // Spawn tasks on the shared runtime
        self.rt.spawn(async move {
            for url in urls {
                if async_request_sender
                    .send(RequestMsg::Element(url))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            let _ = async_request_sender.send(RequestMsg::End).await;
        });

        self.rt.spawn(async_http_client_task(
            async_request_receiver,
            async_response_sender,
            concurrency_limit,
            http_client,
        ));

        self.rt.spawn(async_response_task(
            async_response_receiver,
            cross_response_sender,
        ));

        cross_response_receiver
    }
}

impl Default for SharedClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Lazynet {
    /// Create a new Lazynet instance with default settings (1000 concurrency, 30s timeout).
    pub fn new() -> Self {
        Self::with_config(100, 1000, DEFAULT_TIMEOUT_SECS)
    }

    /// Create a new Lazynet instance with custom buffer size, concurrency limit, and timeout.
    pub fn with_config(buf_size: usize, concurrency_limit: usize, timeout_secs: u64) -> Self {
        Self::with_client(None, buf_size, concurrency_limit, timeout_secs)
    }

    /// Create a new Lazynet instance with an optional shared client.
    /// If no client is provided, creates a new one with the specified timeout.
    pub fn with_client(
        shared_client: Option<&SharedClient>,
        buf_size: usize,
        concurrency_limit: usize,
        timeout_secs: u64,
    ) -> Self {
        // Use provided client or create a new one with timeout
        let http_client = match shared_client {
            Some(sc) => ReqwestClient::from_client(sc.client.clone()),
            None => ReqwestClient::new(timeout_secs),
        };

        Self::with_http_client_internal(http_client, buf_size, concurrency_limit)
    }

    /// Internal constructor that accepts any HttpClient implementation.
    fn with_http_client_internal<C: HttpClient>(
        http_client: C,
        buf_size: usize,
        concurrency_limit: usize,
    ) -> Self {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

        // Create channels for the pipeline
        let (cross_request_sender, cross_request_receiver) =
            crossbeam_channel::bounded::<RequestMsg>(buf_size);
        let (async_request_sender, async_request_receiver) =
            tokio::sync::mpsc::channel::<RequestMsg>(buf_size);
        let (async_response_sender, async_response_receiver) =
            tokio::sync::mpsc::channel::<ResponseMsg>(buf_size);
        let (cross_response_sender, cross_response_receiver) =
            crossbeam_channel::bounded::<ResponseMsg>(buf_size);

        // Spawn the pipeline tasks
        rt.spawn(async_request_task(
            cross_request_receiver,
            async_request_sender,
        ));

        rt.spawn(async_http_client_task(
            async_request_receiver,
            async_response_sender,
            concurrency_limit,
            http_client,
        ));

        rt.spawn(async_response_task(
            async_response_receiver,
            cross_response_sender,
        ));

        Lazynet {
            _rt: rt,
            cross_request_sender,
            cross_response_receiver,
        }
    }

    /// Create a new Lazynet instance with a custom HTTP client (for testing).
    #[cfg(test)]
    pub fn with_http_client<C: HttpClient>(
        client: C,
        buf_size: usize,
        concurrency_limit: usize,
    ) -> Self {
        Self::with_http_client_internal(client, buf_size, concurrency_limit)
    }

    /// Send a URL into the pipeline.
    pub fn send(&self, url: String) {
        let _ = self.cross_request_sender.send(RequestMsg::Element(url));
    }

    /// Signal that no more URLs will be sent.
    pub fn send_end(&self) {
        let _ = self.cross_request_sender.send(RequestMsg::End);
    }

    /// Receive the next response from the pipeline.
    /// Returns None when all responses have been received (after End signal).
    pub fn recv(&self) -> Option<Response> {
        match self.cross_response_receiver.recv() {
            Ok(ResponseMsg::Element(response)) => Some(response),
            Ok(ResponseMsg::End) => None,
            Err(_) => None, // Channel closed
        }
    }
}

impl Default for Lazynet {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Pipeline Tasks
// =============================================================================

/// Bridge from sync crossbeam channel to async tokio channel.
/// Uses a single blocking task to avoid blocking the async runtime's worker threads.
async fn async_request_task(
    cross_request_receiver: crossbeam_channel::Receiver<RequestMsg>,
    async_request_sender: tokio::sync::mpsc::Sender<RequestMsg>,
) {
    // Spawn a single blocking task that reads all messages and forwards them
    // This avoids the overhead of spawn_blocking per message
    let (bridge_tx, mut bridge_rx) = tokio::sync::mpsc::channel::<RequestMsg>(100);

    tokio::task::spawn_blocking(move || {
        loop {
            match cross_request_receiver.recv() {
                Ok(msg) => {
                    let is_end = matches!(msg, RequestMsg::End);
                    // Use blocking_send since we're in a blocking context
                    if bridge_tx.blocking_send(msg).is_err() {
                        break;
                    }
                    if is_end {
                        break;
                    }
                }
                Err(_) => break, // Channel closed
            }
        }
    });

    // Forward messages from the bridge to the async sender
    while let Some(msg) = bridge_rx.recv().await {
        let is_end = matches!(msg, RequestMsg::End);
        if async_request_sender.send(msg).await.is_err() {
            break;
        }
        if is_end {
            break;
        }
    }
}

/// Make HTTP requests concurrently with semaphore-based rate limiting.
async fn async_http_client_task<C: HttpClient>(
    mut async_request_receiver: tokio::sync::mpsc::Receiver<RequestMsg>,
    async_response_sender: tokio::sync::mpsc::Sender<ResponseMsg>,
    concurrency_limit: usize,
    http_client: C,
) {
    let tracker = TaskTracker::new();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency_limit));

    loop {
        // Acquire semaphore permit before receiving to apply backpressure
        let permit = semaphore.clone().acquire_owned().await;

        match async_request_receiver.recv().await {
            Some(RequestMsg::Element(url)) => {
                let client = http_client.clone();
                let sender = async_response_sender.clone();

                tracker.spawn(async move {
                    let response = client.get(&url).await;
                    drop(permit); // Release permit after request completes
                    let _ = sender.send(ResponseMsg::Element(response)).await;
                });
            }
            Some(RequestMsg::End) => {
                drop(permit);
                break;
            }
            None => {
                drop(permit);
                break;
            }
        }
    }

    // Wait for all in-flight requests to complete
    tracker.close();
    tracker.wait().await;

    // Send End signal downstream
    let _ = async_response_sender.send(ResponseMsg::End).await;
}

/// Bridge from async tokio channel back to sync crossbeam channel.
async fn async_response_task(
    mut async_response_receiver: tokio::sync::mpsc::Receiver<ResponseMsg>,
    cross_response_sender: crossbeam_channel::Sender<ResponseMsg>,
) {
    loop {
        match async_response_receiver.recv().await {
            Some(msg) => {
                let is_end = matches!(msg, ResponseMsg::End);
                if cross_response_sender.send(msg).is_err() {
                    break;
                }
                if is_end {
                    break;
                }
            }
            None => break, // Channel closed
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::mock::{MockHttpClient, MockResponse};
    use std::time::Duration;

    #[test]
    fn test_empty_input() {
        let mock = MockHttpClient::new();
        let lazynet = Lazynet::with_http_client(mock, 100, 1000);
        lazynet.send_end();

        assert!(lazynet.recv().is_none());
    }

    #[test]
    fn test_single_request() {
        let mock = MockHttpClient::new()
            .with_response("http://example.com/", MockResponse::success(200, "Hello, World!"));

        let lazynet = Lazynet::with_http_client(mock.clone(), 100, 1000);
        lazynet.send("http://example.com/".to_string());
        lazynet.send_end();

        let response = lazynet.recv();
        assert!(response.is_some());

        let r = response.unwrap();
        assert_eq!(r.request, "http://example.com/");
        assert_eq!(r.status, 200);
        assert_eq!(r.reason, "OK");
        assert_eq!(r.text, "Hello, World!");
        assert!(r.error.is_none());

        assert!(lazynet.recv().is_none());
        assert_eq!(mock.request_count(), 1);
    }

    #[test]
    fn test_error_response() {
        let mock = MockHttpClient::new()
            .with_response("http://fail.example.com/", MockResponse::error("Connection refused"));

        let lazynet = Lazynet::with_http_client(mock, 100, 1000);
        lazynet.send("http://fail.example.com/".to_string());
        lazynet.send_end();

        let response = lazynet.recv().unwrap();
        assert_eq!(response.request, "http://fail.example.com/");
        assert_eq!(response.status, 0);
        assert!(response.error.is_some());
        assert!(response.error.unwrap().contains("Connection refused"));

        assert!(lazynet.recv().is_none());
    }

    #[test]
    fn test_multiple_concurrent_requests() {
        let mock = MockHttpClient::new()
            .with_default(MockResponse::success(200, "response body"));

        let lazynet = Lazynet::with_http_client(mock.clone(), 100, 10);

        for i in 0..100 {
            lazynet.send(format!("http://example.com/{}", i));
        }
        lazynet.send_end();

        let mut responses = Vec::new();
        while let Some(r) = lazynet.recv() {
            responses.push(r);
        }

        assert_eq!(responses.len(), 100);
        assert_eq!(mock.request_count(), 100);

        // All responses should be successful
        for r in &responses {
            assert_eq!(r.status, 200);
            assert!(r.error.is_none());
        }
    }

    #[test]
    fn test_mixed_success_and_errors() {
        let mock = MockHttpClient::new()
            .with_response("http://ok.example.com/", MockResponse::success(200, "ok"))
            .with_response("http://fail.example.com/", MockResponse::error("Timeout"))
            .with_response("http://notfound.example.com/", MockResponse::success(404, ""));

        let lazynet = Lazynet::with_http_client(mock, 100, 1000);
        lazynet.send("http://ok.example.com/".to_string());
        lazynet.send("http://fail.example.com/".to_string());
        lazynet.send("http://notfound.example.com/".to_string());
        lazynet.send_end();

        let mut responses: Vec<_> = std::iter::from_fn(|| lazynet.recv()).collect();
        assert_eq!(responses.len(), 3);

        // Sort by URL for predictable assertions
        responses.sort_by(|a, b| a.request.cmp(&b.request));

        // fail.example.com - error
        let fail = &responses[0];
        assert!(fail.request.contains("fail"));
        assert!(fail.error.is_some());

        // notfound.example.com - 404
        let notfound = &responses[1];
        assert!(notfound.request.contains("notfound"));
        assert_eq!(notfound.status, 404);

        // ok.example.com - 200
        let ok = &responses[2];
        assert!(ok.request.contains("ok."));
        assert_eq!(ok.status, 200);
    }

    #[test]
    fn test_concurrency_limit_respected() {
        // Use delayed responses to verify concurrency limiting
        let mock = MockHttpClient::new()
            .with_default(MockResponse::success(200, "").with_delay(Duration::from_millis(50)));

        let lazynet = Lazynet::with_http_client(mock, 100, 2); // Only 2 concurrent

        // Send 4 requests
        for i in 0..4 {
            lazynet.send(format!("http://example.com/{}", i));
        }
        lazynet.send_end();

        let start = std::time::Instant::now();
        let mut count = 0;
        while lazynet.recv().is_some() {
            count += 1;
        }
        let elapsed = start.elapsed();

        assert_eq!(count, 4);
        // With concurrency 2 and 50ms delay, 4 requests should take ~100ms minimum
        // (2 batches of 2). Allow some timing slack.
        assert!(
            elapsed >= Duration::from_millis(90),
            "Expected >= 90ms, got {:?}",
            elapsed
        );
    }

    #[test]
    fn test_various_status_codes() {
        let mock = MockHttpClient::new()
            .with_response("http://example.com/ok", MockResponse::success(200, ""))
            .with_response("http://example.com/created", MockResponse::success(201, ""))
            .with_response("http://example.com/bad", MockResponse::success(400, ""))
            .with_response("http://example.com/notfound", MockResponse::success(404, ""))
            .with_response("http://example.com/error", MockResponse::success(500, ""));

        let lazynet = Lazynet::with_http_client(mock, 100, 1000);
        lazynet.send("http://example.com/ok".to_string());
        lazynet.send("http://example.com/created".to_string());
        lazynet.send("http://example.com/bad".to_string());
        lazynet.send("http://example.com/notfound".to_string());
        lazynet.send("http://example.com/error".to_string());
        lazynet.send_end();

        let mut responses: Vec<_> = std::iter::from_fn(|| lazynet.recv()).collect();
        responses.sort_by(|a, b| a.request.cmp(&b.request));

        assert_eq!(responses.len(), 5);
        assert_eq!(responses[0].status, 400); // bad
        assert_eq!(responses[1].status, 201); // created
        assert_eq!(responses[2].status, 500); // error
        assert_eq!(responses[3].status, 404); // notfound
        assert_eq!(responses[4].status, 200); // ok
    }
}
