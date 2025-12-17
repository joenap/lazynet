//! HTTP client abstraction for lazynet.
//!
//! This module provides a trait for HTTP GET operations, enabling
//! dependency injection and mocking for tests.

use crate::pipeline::Response;
use std::future::Future;
use std::time::Duration;

/// Trait for HTTP clients that can perform GET requests.
///
/// Implementations must be `Send + Sync + Clone` to work with
/// the async pipeline across thread boundaries.
pub trait HttpClient: Send + Sync + Clone + 'static {
    /// Perform an HTTP GET request to the given URL.
    ///
    /// Returns a `Response` containing either success data or error information.
    fn get(&self, url: &str) -> impl Future<Output = Response> + Send;
}

/// Production HTTP client implementation using reqwest.
#[derive(Clone)]
pub struct ReqwestClient {
    client: reqwest::Client,
}

impl ReqwestClient {
    /// Create a new ReqwestClient with the given timeout.
    pub fn new(timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        Self { client }
    }

    /// Create from an existing reqwest::Client.
    pub fn from_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl HttpClient for ReqwestClient {
    async fn get(&self, url: &str) -> Response {
        match self.client.get(url).send().await {
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
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Configuration for a mock response.
    #[derive(Clone)]
    pub struct MockResponse {
        pub status: u16,
        pub reason: String,
        pub text: String,
        pub error: Option<String>,
        pub delay: Option<Duration>,
    }

    impl MockResponse {
        /// Create a successful mock response.
        pub fn success(status: u16, text: &str) -> Self {
            Self {
                status,
                reason: status_reason(status).to_string(),
                text: text.to_string(),
                error: None,
                delay: None,
            }
        }

        /// Create an error mock response.
        pub fn error(error: &str) -> Self {
            Self {
                status: 0,
                reason: String::new(),
                text: String::new(),
                error: Some(error.to_string()),
                delay: None,
            }
        }

        /// Add a delay before returning this response.
        pub fn with_delay(mut self, delay: Duration) -> Self {
            self.delay = Some(delay);
            self
        }
    }

    fn status_reason(status: u16) -> &'static str {
        match status {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Unknown",
        }
    }

    /// Mock HTTP client for testing.
    #[derive(Clone)]
    pub struct MockHttpClient {
        /// Default response for any URL not explicitly configured.
        default_response: MockResponse,
        /// URL-specific responses.
        responses: Arc<Mutex<HashMap<String, MockResponse>>>,
        /// Record of all URLs requested.
        requests: Arc<Mutex<Vec<String>>>,
    }

    impl MockHttpClient {
        /// Create a new mock client with a default 200 OK response.
        pub fn new() -> Self {
            Self {
                default_response: MockResponse::success(200, ""),
                responses: Arc::new(Mutex::new(HashMap::new())),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        /// Set the default response for unconfigured URLs.
        pub fn with_default(mut self, response: MockResponse) -> Self {
            self.default_response = response;
            self
        }

        /// Configure a specific response for a URL.
        pub fn with_response(self, url: &str, response: MockResponse) -> Self {
            self.responses
                .lock()
                .unwrap()
                .insert(url.to_string(), response);
            self
        }

        /// Get all URLs that were requested.
        #[allow(dead_code)]
        pub fn get_requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }

        /// Get the number of requests made.
        pub fn request_count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }
    }

    impl Default for MockHttpClient {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HttpClient for MockHttpClient {
        async fn get(&self, url: &str) -> Response {
            // Record the request
            self.requests.lock().unwrap().push(url.to_string());

            // Find the appropriate response
            let mock_resp = self
                .responses
                .lock()
                .unwrap()
                .get(url)
                .cloned()
                .unwrap_or_else(|| self.default_response.clone());

            // Simulate latency if configured
            if let Some(delay) = mock_resp.delay {
                tokio::time::sleep(delay).await;
            }

            // Return the response
            match mock_resp.error {
                Some(err) => Response::error(url.to_string(), err),
                None => Response::success(
                    url.to_string(),
                    mock_resp.status,
                    mock_resp.reason,
                    mock_resp.text,
                ),
            }
        }
    }
}
