//! HTTP client abstraction for lazynet.
//!
//! This module provides a trait for HTTP GET operations, enabling
//! dependency injection and mocking for tests.

use crate::pipeline::Response;
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

/// Trait for HTTP clients that can perform GET requests.
///
/// Implementations must be `Send + Sync + Clone` to work with
/// the async pipeline across thread boundaries.
pub trait HttpClient: Send + Sync + Clone + 'static {
    /// Perform an HTTP GET request to the given URL with optional headers.
    ///
    /// Returns a `Response` containing either success data or error information.
    fn get(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
    ) -> impl Future<Output = Response> + Send;
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
            .gzip(true)
            .brotli(true)
            .deflate(true)
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
    async fn get(&self, url: &str, headers: Option<&HashMap<String, String>>) -> Response {
        // Build request with optional headers
        let mut request = self.client.get(url);
        if let Some(hdrs) = headers {
            for (key, value) in hdrs {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        match request.send().await {
            Ok(resp) => self.handle_success(url, resp).await,
            Err(e) => {
                // Check if this is an SSL certificate name mismatch error
                // This happens when a domain's cert doesn't match (e.g., redirect target differs)
                let error_str = e.to_string();
                if error_str.contains("NotValidForName") && url.starts_with("https://") {
                    // Try HTTP to discover the redirect, then follow it
                    self.try_http_redirect_fallback(url, headers).await
                } else {
                    Response::error(url.to_string(), error_str)
                }
            }
        }
    }
}

impl ReqwestClient {
    /// Handle a successful response, extracting status, headers, and body.
    async fn handle_success(&self, url: &str, resp: reqwest::Response) -> Response {
        let final_url = resp.url().to_string();
        let status = resp.status().as_u16();
        let reason = resp
            .status()
            .canonical_reason()
            .unwrap_or("Unknown")
            .to_string();

        // Capture response headers
        let response_headers: HashMap<String, String> = resp
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|val| (k.as_str().to_string(), val.to_string()))
            })
            .collect();

        match resp.bytes().await {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                Response::success(
                    final_url,
                    status,
                    reason,
                    text,
                    bytes.to_vec(),
                    Some(response_headers),
                )
            }
            Err(e) => Response::error(url.to_string(), e.to_string()),
        }
    }

    /// When HTTPS fails with a cert name mismatch, try HTTP to discover redirect.
    ///
    /// This handles cases where a domain redirects to another host whose cert
    /// doesn't cover the original domain name.
    async fn try_http_redirect_fallback(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
    ) -> Response {
        // Convert https:// to http:// to discover the redirect
        let http_url = url.replacen("https://", "http://", 1);

        // Build a client that doesn't follow redirects - we just want the Location header
        let no_redirect_client = match reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(c) => c,
            Err(e) => return Response::error(url.to_string(), format!("fallback client error: {}", e)),
        };

        let mut request = no_redirect_client.get(&http_url);
        if let Some(hdrs) = headers {
            for (key, value) in hdrs {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        match request.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();

                // Check for redirect status codes
                if (301..=308).contains(&status) {
                    if let Some(location) = resp.headers().get("location") {
                        if let Ok(location_str) = location.to_str() {
                            // Follow the redirect with HTTPS
                            let redirect_url = if location_str.starts_with("http") {
                                location_str.to_string()
                            } else if location_str.starts_with("/") {
                                // Relative redirect - construct full URL
                                // http_url is like "http://host/path", find the / after "http://"
                                let after_scheme = &http_url[7..]; // skip "http://"
                                if let Some(path_start) = after_scheme.find('/') {
                                    let host = &after_scheme[..path_start];
                                    format!("https://{}{}", host, location_str)
                                } else {
                                    // No path, just host
                                    format!("https://{}{}", after_scheme, location_str)
                                }
                            } else {
                                location_str.to_string()
                            };

                            // Ensure we use HTTPS for the final request
                            let https_redirect = if redirect_url.starts_with("http://") {
                                redirect_url.replacen("http://", "https://", 1)
                            } else {
                                redirect_url
                            };

                            // Now make the actual request to the redirect target
                            let mut final_request = self.client.get(&https_redirect);
                            if let Some(hdrs) = headers {
                                for (key, value) in hdrs {
                                    final_request = final_request.header(key.as_str(), value.as_str());
                                }
                            }

                            match final_request.send().await {
                                Ok(final_resp) => return self.handle_success(url, final_resp).await,
                                Err(e) => return Response::error(url.to_string(), format!("redirect follow error: {}", e)),
                            }
                        }
                    }
                }

                // Not a redirect or couldn't parse - return original error context
                Response::error(
                    url.to_string(),
                    format!("SSL cert mismatch, HTTP fallback got status {} (expected redirect)", status),
                )
            }
            Err(e) => Response::error(
                url.to_string(),
                format!("SSL cert mismatch, HTTP fallback failed: {}", e),
            ),
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
        pub bytes: Vec<u8>,
        pub error: Option<String>,
        pub delay: Option<Duration>,
    }

    impl MockResponse {
        /// Create a successful mock response.
        pub fn success(status: u16, text: &str) -> Self {
            Self {
                status,
                reason: status_reason(status).to_string(),
                bytes: text.as_bytes().to_vec(),
                text: text.to_string(),
                error: None,
                delay: None,
            }
        }

        /// Create a successful mock response with raw bytes.
        pub fn success_bytes(status: u16, bytes: Vec<u8>) -> Self {
            let text = String::from_utf8_lossy(&bytes).to_string();
            Self {
                status,
                reason: status_reason(status).to_string(),
                text,
                bytes,
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
                bytes: Vec::new(),
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
        async fn get(&self, url: &str, _headers: Option<&HashMap<String, String>>) -> Response {
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
                    mock_resp.bytes,
                    None, // Mock doesn't return headers
                ),
            }
        }
    }
}
