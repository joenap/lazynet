//! Lazynet: Lazy-evaluated HTTP requests with async Rust pipeline.
//!
//! This module provides a sync-to-async bridge for making concurrent HTTP requests.
//! URLs are sent into the pipeline, and responses are pulled out one at a time.

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

        let client = self.client.clone();

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
            client,
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
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

        // Use provided client or create a new one with timeout
        let http_client = match shared_client {
            Some(sc) => sc.client.clone(),
            None => reqwest::Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .expect("Failed to create HTTP client"),
        };

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
/// Uses spawn_blocking to avoid blocking the async runtime's worker threads.
async fn async_request_task(
    cross_request_receiver: crossbeam_channel::Receiver<RequestMsg>,
    async_request_sender: tokio::sync::mpsc::Sender<RequestMsg>,
) {
    loop {
        // Move blocking recv to dedicated thread pool to avoid blocking async workers
        let receiver = cross_request_receiver.clone();
        let recv_result = tokio::task::spawn_blocking(move || receiver.recv())
            .await
            .expect("spawn_blocking task panicked");

        match recv_result {
            Ok(msg) => {
                let is_end = matches!(msg, RequestMsg::End);
                if async_request_sender.send(msg).await.is_err() {
                    break;
                }
                if is_end {
                    break;
                }
            }
            Err(_) => break, // Channel closed
        }
    }
}

/// Make HTTP requests concurrently with semaphore-based rate limiting.
async fn async_http_client_task(
    mut async_request_receiver: tokio::sync::mpsc::Receiver<RequestMsg>,
    async_response_sender: tokio::sync::mpsc::Sender<ResponseMsg>,
    concurrency_limit: usize,
    http_client: reqwest::Client,
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
                    let response = make_request(&client, &url).await;
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

/// Make a single HTTP request and return a Response.
async fn make_request(client: &reqwest::Client, url: &str) -> Response {
    match client.get(url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let reason = resp
                .status()
                .canonical_reason()
                .unwrap_or("Unknown")
                .to_string();

            match resp.text().await {
                Ok(text) => Response::success(url.to_string(), status, reason, text),
                Err(e) => Response::error(url.to_string(), e.to_string()),
            }
        }
        Err(e) => Response::error(url.to_string(), e.to_string()),
    }
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

    #[test]
    fn test_empty_input() {
        let lazynet = Lazynet::new();
        lazynet.send_end();

        assert!(lazynet.recv().is_none());
    }

    #[test]
    fn test_single_request() {
        let lazynet = Lazynet::new();
        lazynet.send("http://127.0.0.1:8080/".to_string());
        lazynet.send_end();

        let response = lazynet.recv();
        assert!(response.is_some());

        let r = response.unwrap();
        assert_eq!(r.request, "http://127.0.0.1:8080/");
        assert!(r.status == 200 || r.error.is_some());

        assert!(lazynet.recv().is_none());
    }
}
