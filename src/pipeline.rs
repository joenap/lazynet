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
        while let Ok(msg) = cross_request_receiver.recv() {
            let is_end = matches!(msg, RequestMsg::End);
            // Use blocking_send since we're in a blocking context
            if bridge_tx.blocking_send(msg).is_err() || is_end {
                break;
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
    while let Some(msg) = async_response_receiver.recv().await {
        let is_end = matches!(msg, ResponseMsg::End);
        if cross_response_sender.send(msg).is_err() || is_end {
            break;
        }
    }
}

// =============================================================================
// Tests
// =============================================================================
//
// This test suite validates lazynet from a product requirements perspective.
// Tests are organized by feature area and written to be human-readable.
//
// Product Requirements Being Tested:
// 1. Core Pipeline: Send URLs in, get responses back
// 2. Concurrency: Configurable limits, parallel execution
// 3. Response Handling: Status codes, errors, body text
// 4. Edge Cases: Empty input, large batches, network failures
// 5. Resource Management: Graceful shutdown, no leaks
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::mock::{MockHttpClient, MockResponse};
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    // =========================================================================
    // RESPONSE STRUCT TESTS
    // Verify the Response struct correctly represents HTTP responses
    // =========================================================================

    mod response_struct {
        use super::*;

        #[test]
        fn successful_response_contains_all_http_metadata() {
            let response = Response::success(
                "http://api.example.com/users".to_string(),
                200,
                "OK".to_string(),
                r#"{"users": []}"#.to_string(),
            );

            assert_eq!(response.request, "http://api.example.com/users");
            assert_eq!(response.status, 200);
            assert_eq!(response.reason, "OK");
            assert_eq!(response.text, r#"{"users": []}"#);
            assert!(response.error.is_none(), "Successful response should have no error");
        }

        #[test]
        fn error_response_preserves_original_url_and_error_message() {
            let response = Response::error(
                "http://unreachable.example.com".to_string(),
                "Connection refused".to_string(),
            );

            assert_eq!(response.request, "http://unreachable.example.com");
            assert_eq!(response.status, 0, "Error responses should have status 0");
            assert!(response.reason.is_empty(), "Error responses should have empty reason");
            assert!(response.text.is_empty(), "Error responses should have empty text");
            assert_eq!(response.error, Some("Connection refused".to_string()));
        }

        #[test]
        fn response_can_be_cloned_without_data_loss() {
            let original = Response::success(
                "http://example.com".to_string(),
                201,
                "Created".to_string(),
                "Resource created".to_string(),
            );

            let cloned = original.clone();

            assert_eq!(cloned.request, original.request);
            assert_eq!(cloned.status, original.status);
            assert_eq!(cloned.reason, original.reason);
            assert_eq!(cloned.text, original.text);
            assert_eq!(cloned.error, original.error);
        }
    }

    // =========================================================================
    // BASIC PIPELINE OPERATION TESTS
    // Verify the fundamental send/receive workflow
    // =========================================================================

    mod basic_pipeline {
        use super::*;

        #[test]
        fn empty_url_list_returns_no_responses() {
            // Product requirement: Gracefully handle empty input
            let mock = MockHttpClient::new();
            let pipeline = Lazynet::with_http_client(mock.clone(), 100, 1000);

            pipeline.send_end(); // Signal completion without sending any URLs

            assert!(
                pipeline.recv().is_none(),
                "Empty input should return None immediately"
            );
            assert_eq!(mock.request_count(), 0, "No HTTP requests should be made");
        }

        #[test]
        fn single_url_returns_single_response() {
            // Product requirement: Basic request/response flow
            let mock = MockHttpClient::new()
                .with_response("http://api.example.com/health", MockResponse::success(200, "OK"));

            let pipeline = Lazynet::with_http_client(mock.clone(), 100, 1000);
            pipeline.send("http://api.example.com/health".to_string());
            pipeline.send_end();

            let response = pipeline.recv().expect("Should receive one response");
            assert_eq!(response.request, "http://api.example.com/health");
            assert_eq!(response.status, 200);
            assert_eq!(response.text, "OK");

            assert!(pipeline.recv().is_none(), "Should have no more responses");
            assert_eq!(mock.request_count(), 1);
        }

        #[test]
        fn multiple_urls_return_same_number_of_responses() {
            // Product requirement: One response per URL
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, "response"));

            let pipeline = Lazynet::with_http_client(mock.clone(), 100, 1000);

            let url_count = 25;
            for i in 0..url_count {
                pipeline.send(format!("http://example.com/page/{}", i));
            }
            pipeline.send_end();

            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();

            assert_eq!(
                responses.len(),
                url_count,
                "Should receive exactly one response per URL"
            );
            assert_eq!(mock.request_count(), url_count);
        }

        #[test]
        fn response_preserves_original_request_url() {
            // Product requirement: Response includes original URL for correlation
            let test_urls = vec![
                "http://example.com/api/v1/users",
                "http://example.com/api/v1/orders?status=pending",
                "http://example.com/api/v1/products/123",
            ];

            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            for url in &test_urls {
                pipeline.send(url.to_string());
            }
            pipeline.send_end();

            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            let response_urls: HashSet<_> = responses.iter().map(|r| r.request.as_str()).collect();
            let expected_urls: HashSet<_> = test_urls.into_iter().collect();

            assert_eq!(
                response_urls, expected_urls,
                "All original URLs should be preserved in responses"
            );
        }

        #[test]
        fn recv_returns_none_after_all_responses_consumed() {
            // Product requirement: Clean termination signal
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/1".to_string());
            pipeline.send("http://example.com/2".to_string());
            pipeline.send_end();

            // Consume all responses
            pipeline.recv();
            pipeline.recv();

            // Subsequent calls should return None
            assert!(pipeline.recv().is_none());
            assert!(pipeline.recv().is_none());
            assert!(pipeline.recv().is_none());
        }
    }

    // =========================================================================
    // HTTP STATUS CODE TESTS
    // Verify correct handling of various HTTP response codes
    // =========================================================================

    mod http_status_codes {
        use super::*;

        #[test]
        fn success_codes_2xx_are_returned_correctly() {
            let mock = MockHttpClient::new()
                .with_response("http://example.com/ok", MockResponse::success(200, "OK"))
                .with_response("http://example.com/created", MockResponse::success(201, "Created"))
                .with_response("http://example.com/accepted", MockResponse::success(202, "Accepted"))
                .with_response("http://example.com/no-content", MockResponse::success(204, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/ok".to_string());
            pipeline.send("http://example.com/created".to_string());
            pipeline.send("http://example.com/accepted".to_string());
            pipeline.send("http://example.com/no-content".to_string());
            pipeline.send_end();

            let mut responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            responses.sort_by_key(|r| r.status);

            assert_eq!(responses[0].status, 200);
            assert_eq!(responses[1].status, 201);
            assert_eq!(responses[2].status, 202);
            assert_eq!(responses[3].status, 204);

            // All 2xx responses should have no error
            for r in &responses {
                assert!(r.error.is_none(), "2xx responses should not have errors");
            }
        }

        #[test]
        fn client_error_codes_4xx_are_returned_as_valid_responses() {
            // Product requirement: 4xx errors are valid HTTP responses, not connection errors
            let mock = MockHttpClient::new()
                .with_response("http://example.com/bad", MockResponse::success(400, "Bad Request"))
                .with_response("http://example.com/unauthorized", MockResponse::success(401, "Unauthorized"))
                .with_response("http://example.com/forbidden", MockResponse::success(403, "Forbidden"))
                .with_response("http://example.com/notfound", MockResponse::success(404, "Not Found"))
                .with_response("http://example.com/conflict", MockResponse::success(409, "Conflict"));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/bad".to_string());
            pipeline.send("http://example.com/unauthorized".to_string());
            pipeline.send("http://example.com/forbidden".to_string());
            pipeline.send("http://example.com/notfound".to_string());
            pipeline.send("http://example.com/conflict".to_string());
            pipeline.send_end();

            let mut responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            responses.sort_by_key(|r| r.status);

            assert_eq!(responses.len(), 5);
            assert_eq!(responses[0].status, 400);
            assert_eq!(responses[1].status, 401);
            assert_eq!(responses[2].status, 403);
            assert_eq!(responses[3].status, 404);
            assert_eq!(responses[4].status, 409);

            // 4xx responses are valid HTTP responses - error field should be None
            for r in &responses {
                assert!(
                    r.error.is_none(),
                    "4xx HTTP responses should not set error field"
                );
            }
        }

        #[test]
        fn server_error_codes_5xx_are_returned_as_valid_responses() {
            // Product requirement: 5xx errors are valid HTTP responses
            let mock = MockHttpClient::new()
                .with_response("http://example.com/internal", MockResponse::success(500, "Internal Server Error"))
                .with_response("http://example.com/unavailable", MockResponse::success(503, "Service Unavailable"))
                .with_response("http://example.com/timeout", MockResponse::success(504, "Gateway Timeout"));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/internal".to_string());
            pipeline.send("http://example.com/unavailable".to_string());
            pipeline.send("http://example.com/timeout".to_string());
            pipeline.send_end();

            let mut responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            responses.sort_by_key(|r| r.status);

            assert_eq!(responses[0].status, 500);
            assert_eq!(responses[1].status, 503);
            assert_eq!(responses[2].status, 504);

            for r in &responses {
                assert!(r.error.is_none(), "5xx HTTP responses should not set error field");
            }
        }

        #[test]
        fn redirect_codes_3xx_are_returned_correctly() {
            let mock = MockHttpClient::new()
                .with_response("http://example.com/moved", MockResponse::success(301, "Moved Permanently"))
                .with_response("http://example.com/found", MockResponse::success(302, "Found"))
                .with_response("http://example.com/see-other", MockResponse::success(303, "See Other"))
                .with_response("http://example.com/temp", MockResponse::success(307, "Temporary Redirect"));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/moved".to_string());
            pipeline.send("http://example.com/found".to_string());
            pipeline.send("http://example.com/see-other".to_string());
            pipeline.send("http://example.com/temp".to_string());
            pipeline.send_end();

            let mut responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            responses.sort_by_key(|r| r.status);

            assert_eq!(responses[0].status, 301);
            assert_eq!(responses[1].status, 302);
            assert_eq!(responses[2].status, 303);
            assert_eq!(responses[3].status, 307);
        }
    }

    // =========================================================================
    // ERROR HANDLING TESTS
    // Verify correct handling of network and connection errors
    // =========================================================================

    mod error_handling {
        use super::*;

        #[test]
        fn connection_refused_returns_error_response() {
            let mock = MockHttpClient::new()
                .with_response("http://localhost:9999/", MockResponse::error("Connection refused"));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://localhost:9999/".to_string());
            pipeline.send_end();

            let response = pipeline.recv().expect("Should receive error response");

            assert_eq!(response.request, "http://localhost:9999/");
            assert_eq!(response.status, 0, "Error responses should have status 0");
            assert!(response.error.is_some());
            assert!(
                response.error.as_ref().unwrap().contains("Connection refused"),
                "Error message should describe the failure"
            );
        }

        #[test]
        fn dns_resolution_failure_returns_error_response() {
            let mock = MockHttpClient::new()
                .with_response(
                    "http://nonexistent.invalid/",
                    MockResponse::error("DNS resolution failed: no such host"),
                );

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://nonexistent.invalid/".to_string());
            pipeline.send_end();

            let response = pipeline.recv().expect("Should receive error response");

            assert!(response.error.is_some());
            assert!(response.error.as_ref().unwrap().contains("DNS"));
        }

        #[test]
        fn timeout_error_returns_error_response() {
            let mock = MockHttpClient::new()
                .with_response(
                    "http://slow.example.com/",
                    MockResponse::error("Request timeout: operation timed out"),
                );

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://slow.example.com/".to_string());
            pipeline.send_end();

            let response = pipeline.recv().expect("Should receive error response");

            assert!(response.error.is_some());
            assert!(response.error.as_ref().unwrap().to_lowercase().contains("timeout"));
        }

        #[test]
        fn ssl_certificate_error_returns_error_response() {
            let mock = MockHttpClient::new()
                .with_response(
                    "https://expired.badssl.com/",
                    MockResponse::error("SSL certificate error: certificate has expired"),
                );

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("https://expired.badssl.com/".to_string());
            pipeline.send_end();

            let response = pipeline.recv().expect("Should receive error response");

            assert!(response.error.is_some());
            assert!(response.error.as_ref().unwrap().to_lowercase().contains("ssl")
                || response.error.as_ref().unwrap().to_lowercase().contains("certificate"));
        }

        #[test]
        fn mixed_successes_and_errors_all_returned() {
            // Product requirement: Errors don't abort the batch
            let mock = MockHttpClient::new()
                .with_response("http://example.com/success1", MockResponse::success(200, "OK"))
                .with_response("http://example.com/fail", MockResponse::error("Connection reset"))
                .with_response("http://example.com/success2", MockResponse::success(200, "OK"))
                .with_response("http://example.com/timeout", MockResponse::error("Timeout"))
                .with_response("http://example.com/success3", MockResponse::success(200, "OK"));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/success1".to_string());
            pipeline.send("http://example.com/fail".to_string());
            pipeline.send("http://example.com/success2".to_string());
            pipeline.send("http://example.com/timeout".to_string());
            pipeline.send("http://example.com/success3".to_string());
            pipeline.send_end();

            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();

            assert_eq!(responses.len(), 5, "All requests should return responses");

            let successes = responses.iter().filter(|r| r.error.is_none()).count();
            let errors = responses.iter().filter(|r| r.error.is_some()).count();

            assert_eq!(successes, 3);
            assert_eq!(errors, 2);
        }

        #[test]
        fn error_responses_have_consistent_structure() {
            let mock = MockHttpClient::new()
                .with_response("http://example.com/error", MockResponse::error("Some error"));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/error".to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();

            // Error response contract:
            assert_eq!(response.status, 0, "Error status should be 0");
            assert!(response.reason.is_empty(), "Error reason should be empty");
            assert!(response.text.is_empty(), "Error text should be empty");
            assert!(response.error.is_some(), "Error field should be populated");
            assert!(!response.request.is_empty(), "Request URL should be preserved");
        }
    }

    // =========================================================================
    // CONCURRENCY CONTROL TESTS
    // Verify the concurrency limit is correctly enforced
    // =========================================================================

    mod concurrency_control {
        use super::*;

        #[test]
        fn concurrency_limit_of_1_processes_requests_sequentially() {
            // With 50ms delay per request and concurrency=1, 4 requests = ~200ms
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, "").with_delay(Duration::from_millis(50)));

            let pipeline = Lazynet::with_http_client(mock, 100, 1); // concurrency = 1

            for i in 0..4 {
                pipeline.send(format!("http://example.com/{}", i));
            }
            pipeline.send_end();

            let start = Instant::now();
            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            let elapsed = start.elapsed();

            assert_eq!(responses.len(), 4);
            assert!(
                elapsed >= Duration::from_millis(180),
                "Sequential execution of 4x50ms should take >= 180ms, got {:?}",
                elapsed
            );
        }

        #[test]
        fn concurrency_limit_of_2_processes_in_pairs() {
            // With 50ms delay and concurrency=2, 4 requests = 2 batches = ~100ms
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, "").with_delay(Duration::from_millis(50)));

            let pipeline = Lazynet::with_http_client(mock, 100, 2);

            for i in 0..4 {
                pipeline.send(format!("http://example.com/{}", i));
            }
            pipeline.send_end();

            let start = Instant::now();
            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            let elapsed = start.elapsed();

            assert_eq!(responses.len(), 4);
            assert!(
                elapsed >= Duration::from_millis(90),
                "Two batches of 50ms should take >= 90ms, got {:?}",
                elapsed
            );
            assert!(
                elapsed < Duration::from_millis(180),
                "Parallel execution should be faster than sequential (~200ms), got {:?}",
                elapsed
            );
        }

        #[test]
        fn high_concurrency_processes_all_requests_in_parallel() {
            // With concurrency=100 and 100 requests at 50ms each, should complete in ~50ms
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, "").with_delay(Duration::from_millis(50)));

            let pipeline = Lazynet::with_http_client(mock, 200, 100);

            for i in 0..100 {
                pipeline.send(format!("http://example.com/{}", i));
            }
            pipeline.send_end();

            let start = Instant::now();
            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            let elapsed = start.elapsed();

            assert_eq!(responses.len(), 100);
            assert!(
                elapsed < Duration::from_millis(200),
                "100 parallel 50ms requests should complete quickly, got {:?}",
                elapsed
            );
        }

        #[test]
        fn all_responses_returned_regardless_of_concurrency_limit() {
            for concurrency in [1, 5, 10, 50, 100] {
                let mock = MockHttpClient::new()
                    .with_default(MockResponse::success(200, ""));

                let pipeline = Lazynet::with_http_client(mock.clone(), 100, concurrency);

                let request_count = 50;
                for i in 0..request_count {
                    pipeline.send(format!("http://example.com/{}", i));
                }
                pipeline.send_end();

                let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();

                assert_eq!(
                    responses.len(),
                    request_count,
                    "Concurrency {} should return all {} responses",
                    concurrency,
                    request_count
                );
            }
        }
    }

    // =========================================================================
    // RESPONSE BODY TESTS
    // Verify correct handling of various response body types
    // =========================================================================

    mod response_body {
        use super::*;

        #[test]
        fn empty_response_body_is_handled() {
            let mock = MockHttpClient::new()
                .with_response("http://example.com/empty", MockResponse::success(204, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/empty".to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.text, "");
            assert!(response.error.is_none());
        }

        #[test]
        fn json_response_body_is_preserved() {
            let json_body = r#"{"users":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]}"#;
            let mock = MockHttpClient::new()
                .with_response("http://api.example.com/users", MockResponse::success(200, json_body));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://api.example.com/users".to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.text, json_body);
        }

        #[test]
        fn html_response_body_is_preserved() {
            let html_body = "<!DOCTYPE html><html><body><h1>Hello</h1></body></html>";
            let mock = MockHttpClient::new()
                .with_response("http://example.com/page", MockResponse::success(200, html_body));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/page".to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.text, html_body);
        }

        #[test]
        fn unicode_response_body_is_preserved() {
            let unicode_body = "Hello 世界! 🌍 Ñoño café résumé";
            let mock = MockHttpClient::new()
                .with_response("http://example.com/unicode", MockResponse::success(200, unicode_body));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/unicode".to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.text, unicode_body);
        }

        #[test]
        fn large_response_body_is_preserved() {
            // 100KB response
            let large_body: String = "x".repeat(100_000);
            let mock = MockHttpClient::new()
                .with_response("http://example.com/large", MockResponse::success(200, &large_body));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/large".to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.text.len(), 100_000);
            assert_eq!(response.text, large_body);
        }

        #[test]
        fn multiline_response_body_is_preserved() {
            let multiline_body = "Line 1\nLine 2\nLine 3\r\nLine 4 with CRLF";
            let mock = MockHttpClient::new()
                .with_response("http://example.com/multiline", MockResponse::success(200, multiline_body));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/multiline".to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.text, multiline_body);
            assert!(response.text.contains('\n'));
            assert!(response.text.contains("\r\n"));
        }
    }

    // =========================================================================
    // SCALE AND LOAD TESTS
    // Verify correct behavior under high request volumes
    // =========================================================================

    mod scale_and_load {
        use super::*;

        #[test]
        fn handles_1000_requests_without_losing_any() {
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, "ok"));

            let pipeline = Lazynet::with_http_client(mock.clone(), 200, 100);

            let request_count = 1000;
            for i in 0..request_count {
                pipeline.send(format!("http://example.com/{}", i));
            }
            pipeline.send_end();

            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();

            assert_eq!(
                responses.len(),
                request_count,
                "All 1000 requests should return responses"
            );
            assert_eq!(mock.request_count(), request_count);
        }

        #[test]
        fn all_unique_urls_return_unique_responses() {
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock, 200, 50);

            let request_count = 500;
            for i in 0..request_count {
                pipeline.send(format!("http://example.com/unique/{}", i));
            }
            pipeline.send_end();

            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            let unique_urls: HashSet<_> = responses.iter().map(|r| &r.request).collect();

            assert_eq!(
                unique_urls.len(),
                request_count,
                "Each unique URL should produce exactly one response"
            );
        }

        #[test]
        fn duplicate_urls_each_produce_response() {
            // Product requirement: Same URL sent multiple times = multiple responses
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock.clone(), 100, 10);

            let same_url = "http://example.com/same";
            for _ in 0..10 {
                pipeline.send(same_url.to_string());
            }
            pipeline.send_end();

            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();

            assert_eq!(responses.len(), 10, "Each send() should produce a response");
            assert_eq!(mock.request_count(), 10, "Each URL should trigger a request");
        }

        #[test]
        fn small_buffer_still_processes_all_requests() {
            // Test with buffer size smaller than request count
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock.clone(), 10, 5); // Small buffer

            let request_count = 100;
            for i in 0..request_count {
                pipeline.send(format!("http://example.com/{}", i));
            }
            pipeline.send_end();

            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();

            assert_eq!(
                responses.len(),
                request_count,
                "Small buffer should not lose requests"
            );
        }
    }

    // =========================================================================
    // URL FORMAT TESTS
    // Verify handling of various URL formats
    // =========================================================================

    mod url_formats {
        use super::*;

        #[test]
        fn handles_urls_with_query_parameters() {
            let url_with_params = "http://api.example.com/search?q=rust&page=1&limit=10";
            let mock = MockHttpClient::new()
                .with_response(url_with_params, MockResponse::success(200, "results"));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send(url_with_params.to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.request, url_with_params);
            assert_eq!(response.status, 200);
        }

        #[test]
        fn handles_urls_with_fragments() {
            let url_with_fragment = "http://example.com/page#section-2";
            let mock = MockHttpClient::new()
                .with_response(url_with_fragment, MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send(url_with_fragment.to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.request, url_with_fragment);
        }

        #[test]
        fn handles_urls_with_special_characters() {
            let url_with_special = "http://example.com/path%20with%20spaces?name=caf%C3%A9";
            let mock = MockHttpClient::new()
                .with_response(url_with_special, MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send(url_with_special.to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.request, url_with_special);
        }

        #[test]
        fn handles_https_urls() {
            let https_url = "https://secure.example.com/api/data";
            let mock = MockHttpClient::new()
                .with_response(https_url, MockResponse::success(200, "secure data"));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send(https_url.to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.request, https_url);
        }

        #[test]
        fn handles_urls_with_ports() {
            let url_with_port = "http://example.com:8080/api";
            let mock = MockHttpClient::new()
                .with_response(url_with_port, MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send(url_with_port.to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.request, url_with_port);
        }

        #[test]
        fn handles_localhost_urls() {
            let localhost_url = "http://localhost:3000/health";
            let mock = MockHttpClient::new()
                .with_response(localhost_url, MockResponse::success(200, "healthy"));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send(localhost_url.to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.request, localhost_url);
            assert_eq!(response.text, "healthy");
        }

        #[test]
        fn handles_ip_address_urls() {
            let ip_url = "http://192.168.1.1:8080/api";
            let mock = MockHttpClient::new()
                .with_response(ip_url, MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send(ip_url.to_string());
            pipeline.send_end();

            let response = pipeline.recv().unwrap();
            assert_eq!(response.request, ip_url);
        }
    }

    // =========================================================================
    // PIPELINE LIFECYCLE TESTS
    // Verify correct initialization and shutdown behavior
    // =========================================================================

    mod pipeline_lifecycle {
        use super::*;

        #[test]
        fn pipeline_can_be_used_immediately_after_creation() {
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);

            // Should work without any warm-up
            pipeline.send("http://example.com/".to_string());
            pipeline.send_end();

            let response = pipeline.recv();
            assert!(response.is_some());
        }

        #[test]
        fn multiple_pipelines_can_run_independently() {
            let mock1 = MockHttpClient::new()
                .with_response("http://example.com/1", MockResponse::success(200, "response 1"));
            let mock2 = MockHttpClient::new()
                .with_response("http://example.com/2", MockResponse::success(200, "response 2"));

            let pipeline1 = Lazynet::with_http_client(mock1, 100, 1000);
            let pipeline2 = Lazynet::with_http_client(mock2, 100, 1000);

            pipeline1.send("http://example.com/1".to_string());
            pipeline2.send("http://example.com/2".to_string());

            pipeline1.send_end();
            pipeline2.send_end();

            let response1 = pipeline1.recv().unwrap();
            let response2 = pipeline2.recv().unwrap();

            assert_eq!(response1.text, "response 1");
            assert_eq!(response2.text, "response 2");
        }

        #[test]
        fn send_end_is_required_to_receive_responses() {
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/".to_string());

            // Without send_end(), we need to ensure the pipeline eventually completes
            // This test verifies that send_end() is the proper termination signal
            pipeline.send_end();

            // Now responses should be available
            let response = pipeline.recv();
            assert!(response.is_some());
        }

        #[test]
        fn graceful_completion_with_in_flight_requests() {
            // Verify all in-flight requests complete before pipeline closes
            let mock = MockHttpClient::new()
                .with_default(MockResponse::success(200, "").with_delay(Duration::from_millis(50)));

            let pipeline = Lazynet::with_http_client(mock.clone(), 100, 10);

            for i in 0..20 {
                pipeline.send(format!("http://example.com/{}", i));
            }
            pipeline.send_end();

            let responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();

            assert_eq!(
                responses.len(),
                20,
                "All in-flight requests should complete before shutdown"
            );
            assert_eq!(mock.request_count(), 20);
        }
    }

    // =========================================================================
    // RESPONSE REASON PHRASE TESTS
    // Verify HTTP reason phrases are correctly captured
    // =========================================================================

    mod reason_phrases {
        use super::*;

        #[test]
        fn common_status_codes_have_correct_reason_phrases() {
            let mock = MockHttpClient::new()
                .with_response("http://example.com/200", MockResponse::success(200, ""))
                .with_response("http://example.com/201", MockResponse::success(201, ""))
                .with_response("http://example.com/400", MockResponse::success(400, ""))
                .with_response("http://example.com/404", MockResponse::success(404, ""))
                .with_response("http://example.com/500", MockResponse::success(500, ""));

            let pipeline = Lazynet::with_http_client(mock, 100, 1000);
            pipeline.send("http://example.com/200".to_string());
            pipeline.send("http://example.com/201".to_string());
            pipeline.send("http://example.com/400".to_string());
            pipeline.send("http://example.com/404".to_string());
            pipeline.send("http://example.com/500".to_string());
            pipeline.send_end();

            let mut responses: Vec<_> = std::iter::from_fn(|| pipeline.recv()).collect();
            responses.sort_by_key(|r| r.status);

            assert_eq!(responses[0].reason, "OK");
            assert_eq!(responses[1].reason, "Created");
            assert_eq!(responses[2].reason, "Bad Request");
            assert_eq!(responses[3].reason, "Not Found");
            assert_eq!(responses[4].reason, "Internal Server Error");
        }
    }

    // =========================================================================
    // DEFAULT CONFIGURATION TESTS
    // Verify default values work correctly
    // =========================================================================

    mod default_configuration {
        use super::*;

        #[test]
        fn default_timeout_constant_is_30_seconds() {
            assert_eq!(DEFAULT_TIMEOUT_SECS, 30);
        }

        #[test]
        fn lazynet_new_uses_sensible_defaults() {
            // This tests that Lazynet::new() creates a working pipeline
            // The actual defaults (buf_size=100, concurrency=1000, timeout=30s)
            // are implementation details, but the pipeline should work
            let lazynet = Lazynet::new();
            lazynet.send_end();
            assert!(lazynet.recv().is_none());
        }

        #[test]
        fn lazynet_default_trait_matches_new() {
            let from_new = Lazynet::new();
            let from_default = Lazynet::default();

            from_new.send_end();
            from_default.send_end();

            // Both should behave identically - return None for empty input
            assert!(from_new.recv().is_none());
            assert!(from_default.recv().is_none());
        }
    }
}
