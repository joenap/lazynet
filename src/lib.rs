//! Lazynet Python module - lazy-evaluated HTTP requests.
//!
//! Usage:
//!     import lazynet
//!     urls = (f"http://example.com/{i}" for i in range(100))
//!     for response in lazynet.get(urls):
//!         print(response.status, response.text[:50])

mod http_client;
pub mod pipeline;

pub use pipeline::{Lazynet, Response as RustResponse, DEFAULT_TIMEOUT_SECS};
use pipeline::SharedClient;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator, PyList, PyModule};

/// HTTP response returned from lazynet.
#[pyclass(skip_from_py_object)]
pub struct Response {
    #[pyo3(get)]
    pub request: String,
    #[pyo3(get)]
    pub status: u16,
    #[pyo3(get)]
    pub reason: String,
    #[pyo3(get)]
    pub text: String,
    /// Raw response bytes.
    #[pyo3(get)]
    pub bytes: Vec<u8>,
    /// Error message if the request failed, None if successful.
    #[pyo3(get)]
    pub error: Option<String>,
    /// Response headers from the server.
    #[pyo3(get)]
    pub headers: Option<std::collections::HashMap<String, String>>,
    // Store parsed JSON as a Python object
    json_value: Option<Py<PyAny>>,
}

impl Clone for Response {
    fn clone(&self) -> Self {
        Python::attach(|py| Response {
            request: self.request.clone(),
            status: self.status,
            reason: self.reason.clone(),
            text: self.text.clone(),
            bytes: self.bytes.clone(),
            error: self.error.clone(),
            headers: self.headers.clone(),
            json_value: self.json_value.as_ref().map(|v| v.clone_ref(py)),
        })
    }
}

#[pymethods]
impl Response {
    /// Get the parsed JSON response body.
    #[getter]
    fn json(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match &self.json_value {
            Some(val) => Ok(val.clone_ref(py)),
            None => Ok(py.None()),
        }
    }

    fn __repr__(&self) -> String {
        // Use char-based truncation to avoid panic on multi-byte UTF-8
        let text_preview: String = self.text.chars().take(50).collect();
        let ellipsis = if self.text.chars().count() > 50 { "..." } else { "" };
        let header_count = self.headers.as_ref().map(|h| h.len()).unwrap_or(0);
        format!(
            "Response(request='{}', status={}, reason='{}', text='{}{}', error={:?}, headers={})",
            self.request,
            self.status,
            self.reason,
            text_preview,
            ellipsis,
            self.error,
            header_count
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __eq__(&self, other: &Response) -> bool {
        self.request == other.request
            && self.status == other.status
            && self.reason == other.reason
            && self.text == other.text
            && self.error == other.error
            && self.headers == other.headers
    }
}

impl Response {
    /// Create a Response from the Rust pipeline response, parsing JSON.
    fn from_rust_response(py: Python<'_>, r: RustResponse) -> PyResult<Self> {
        // Try to parse the response text as JSON (only if no error)
        let json_value = if r.error.is_none() && !r.text.is_empty() {
            match serde_json::from_str::<serde_json::Value>(&r.text) {
                Ok(value) => Some(json_to_py(py, &value)?),
                Err(_) => None, // Not valid JSON, that's fine
            }
        } else {
            None
        };

        Ok(Response {
            request: r.request,
            status: r.status,
            reason: r.reason,
            text: r.text,
            bytes: r.bytes,
            error: r.error,
            headers: r.headers,
            json_value,
        })
    }
}

/// Convert a serde_json::Value to a Python object.
fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any().unbind())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any().unbind())
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_py(py, item)?)?;
            }
            Ok(list.unbind().into_any())
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            Ok(dict.unbind().into_any())
        }
    }
}

/// Iterator that yields HTTP responses.
#[pyclass]
pub struct LazynetIterator {
    lazynet: Lazynet,
}

#[pymethods]
impl LazynetIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Response>> {
        // Release the GIL while blocking on the channel
        let rust_response = py.detach(|| self.lazynet.recv());

        match rust_response {
            Some(r) => Ok(Some(Response::from_rust_response(py, r)?)),
            None => Ok(None),
        }
    }
}

/// Reusable HTTP client for connection pooling.
///
/// Use this when making multiple batches of requests to avoid
/// ephemeral port exhaustion from TIME_WAIT connections.
///
/// Args:
///     timeout_secs: Request timeout in seconds (default: 30)
///     default_headers: Default headers to send with every request
///
/// Example:
///     client = lazynet.Client(default_headers={"User-Agent": "MyBot/1.0"})
///     for batch in batches:
///         for response in client.get(batch):
///             print(response.status, response.headers)
#[pyclass]
pub struct Client {
    shared_client: SharedClient,
    default_headers: Option<std::collections::HashMap<String, String>>,
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (timeout_secs=None, default_headers=None))]
    fn new(
        timeout_secs: Option<u64>,
        default_headers: Option<std::collections::HashMap<String, String>>,
    ) -> Self {
        let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
        Client {
            shared_client: SharedClient::with_timeout(timeout),
            default_headers,
        }
    }

    /// Make HTTP GET requests using the shared connection pool.
    ///
    /// Args:
    ///     urls: An iterable of URL strings, or (url, headers_dict) tuples
    ///     concurrency_limit: Maximum concurrent requests (default: 1000)
    ///     headers: Optional headers to override defaults for this batch
    ///
    /// Returns:
    ///     An iterator of Response objects
    #[pyo3(signature = (urls, concurrency_limit=1000, headers=None))]
    fn get(
        &self,
        urls: &Bound<'_, PyIterator>,
        concurrency_limit: usize,
        headers: Option<std::collections::HashMap<String, String>>,
    ) -> PyResult<ClientIterator> {
        // Merge batch headers with default headers
        let batch_headers = match (&self.default_headers, &headers) {
            (Some(defaults), Some(overrides)) => {
                let mut merged = defaults.clone();
                merged.extend(overrides.clone());
                Some(merged)
            }
            (Some(defaults), None) => Some(defaults.clone()),
            (None, Some(overrides)) => Some(overrides.clone()),
            (None, None) => None,
        };

        // Check if any items are tuples with per-request headers
        let mut has_per_request_headers = false;
        let mut per_request: Vec<(String, Option<std::collections::HashMap<String, String>>)> = Vec::new();

        for item_result in urls.try_iter()? {
            let item = item_result?;

            if let Ok(tuple) = item.extract::<(String, std::collections::HashMap<String, String>)>() {
                has_per_request_headers = true;
                per_request.push((tuple.0, Some(tuple.1)));
            } else {
                let url: String = item.extract()?;
                per_request.push((url, None));
            }
        }

        if has_per_request_headers {
            // Use Lazynet pipeline directly to support per-request headers
            let lazynet = Lazynet::with_client(
                Some(&self.shared_client),
                100,
                concurrency_limit,
                DEFAULT_TIMEOUT_SECS,
            );

            for (url, pr_headers) in per_request {
                let merged = merge_headers(&batch_headers, pr_headers);
                lazynet.send_with_headers(url, merged);
            }
            lazynet.send_end();

            Ok(ClientIterator::from_lazy(lazynet))
        } else {
            // Fast path: no per-request headers, use SharedClient directly
            let url_vec: Vec<String> = per_request.into_iter().map(|(url, _)| url).collect();

            let receiver =
                self.shared_client
                    .get_with_headers(url_vec, concurrency_limit, batch_headers);

            Ok(ClientIterator::from_shared(receiver))
        }
    }
}

/// Internal enum for client iterator backends.
enum ClientIteratorInner {
    Shared(crossbeam_channel::Receiver<pipeline::ResponseMsg>),
    Lazy(Lazynet),
}

/// Iterator that yields HTTP responses from a Client.
#[pyclass]
pub struct ClientIterator {
    inner: ClientIteratorInner,
}

impl ClientIterator {
    fn from_shared(receiver: crossbeam_channel::Receiver<pipeline::ResponseMsg>) -> Self {
        ClientIterator { inner: ClientIteratorInner::Shared(receiver) }
    }

    fn from_lazy(lazynet: Lazynet) -> Self {
        ClientIterator { inner: ClientIteratorInner::Lazy(lazynet) }
    }
}

#[pymethods]
impl ClientIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Response>> {
        match &self.inner {
            ClientIteratorInner::Shared(receiver) => {
                let msg = py.detach(|| receiver.recv());
                match msg {
                    Ok(pipeline::ResponseMsg::Element(r)) => {
                        Ok(Some(Response::from_rust_response(py, r)?))
                    }
                    Ok(pipeline::ResponseMsg::End) => Ok(None),
                    Err(_) => Ok(None),
                }
            }
            ClientIteratorInner::Lazy(lazynet) => {
                let rust_response = py.detach(|| lazynet.recv());
                match rust_response {
                    Some(r) => Ok(Some(Response::from_rust_response(py, r)?)),
                    None => Ok(None),
                }
            }
        }
    }
}

/// Merge batch-level headers with per-request headers.
/// Per-request headers override batch-level headers.
fn merge_headers(
    batch: &Option<std::collections::HashMap<String, String>>,
    per_request: Option<std::collections::HashMap<String, String>>,
) -> Option<std::collections::HashMap<String, String>> {
    match (batch, per_request) {
        (Some(b), Some(pr)) => {
            let mut merged = b.clone();
            merged.extend(pr);
            Some(merged)
        }
        (Some(b), None) => Some(b.clone()),
        (None, Some(pr)) => Some(pr),
        (None, None) => None,
    }
}

/// Make HTTP GET requests for the given URLs.
///
/// Args:
///     urls: An iterable of URL strings, or (url, headers_dict) tuples
///     concurrency_limit: Maximum concurrent requests (default: 1000)
///     timeout_secs: Request timeout in seconds (default: 30)
///     headers: Optional headers to send with every request
///
/// Returns:
///     An iterator of Response objects
///
/// Example:
///     urls = (f"http://example.com/{i}" for i in range(100))
///     for response in lazynet.get(urls, headers={"User-Agent": "MyBot/1.0"}):
///         print(response.status, response.headers)
///
///     # Per-request headers via tuples:
///     requests = ((url, {"Range": f"bytes={s}-{e}"}) for url, s, e in manifest)
///     for response in lazynet.get(requests):
///         print(len(response.bytes))
#[pyfunction]
#[pyo3(signature = (urls, concurrency_limit=1000, timeout_secs=None, headers=None))]
fn get(
    urls: &Bound<'_, PyIterator>,
    concurrency_limit: usize,
    timeout_secs: Option<u64>,
    headers: Option<std::collections::HashMap<String, String>>,
) -> PyResult<LazynetIterator> {
    let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
    let lazynet = Lazynet::with_config(100, concurrency_limit, timeout);

    // Consume the Python iterator and send URLs to the pipeline
    // Each item can be a plain URL string or a (url, headers_dict) tuple
    for url_result in urls.try_iter()? {
        let item = url_result?;

        if let Ok(tuple) = item.extract::<(String, std::collections::HashMap<String, String>)>() {
            let merged = merge_headers(&headers, Some(tuple.1));
            lazynet.send_with_headers(tuple.0, merged);
        } else {
            let url: String = item.extract()?;
            lazynet.send_with_headers(url, headers.clone());
        }
    }
    lazynet.send_end();

    Ok(LazynetIterator { lazynet })
}

/// Lazynet Python module - lazy-evaluated HTTP requests.
#[pymodule]
fn _lazynet(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Response>()?;
    m.add_class::<LazynetIterator>()?;
    m.add_class::<Client>()?;
    m.add_class::<ClientIterator>()?;
    m.add_function(wrap_pyfunction!(get, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    mod merge_headers_tests {
        use super::*;

        #[test]
        fn both_none_returns_none() {
            assert!(merge_headers(&None, None).is_none());
        }

        #[test]
        fn batch_only_returns_batch() {
            let batch: HashMap<String, String> =
                [("X-Batch".into(), "val".into())].into_iter().collect();
            let result = merge_headers(&Some(batch.clone()), None).unwrap();
            assert_eq!(result, batch);
        }

        #[test]
        fn per_request_only_returns_per_request() {
            let pr: HashMap<String, String> =
                [("X-Per".into(), "val".into())].into_iter().collect();
            let result = merge_headers(&None, Some(pr.clone())).unwrap();
            assert_eq!(result, pr);
        }

        #[test]
        fn per_request_overrides_batch_for_same_key() {
            let batch: HashMap<String, String> =
                [("X-Key".into(), "batch".into())].into_iter().collect();
            let pr: HashMap<String, String> =
                [("X-Key".into(), "per-request".into())].into_iter().collect();
            let result = merge_headers(&Some(batch), Some(pr)).unwrap();
            assert_eq!(result.get("X-Key").unwrap(), "per-request");
        }

        #[test]
        fn disjoint_headers_are_merged() {
            let batch: HashMap<String, String> =
                [("X-Batch".into(), "b".into())].into_iter().collect();
            let pr: HashMap<String, String> =
                [("X-Per".into(), "p".into())].into_iter().collect();
            let result = merge_headers(&Some(batch), Some(pr)).unwrap();
            assert_eq!(result.len(), 2);
            assert_eq!(result.get("X-Batch").unwrap(), "b");
            assert_eq!(result.get("X-Per").unwrap(), "p");
        }
    }
}
