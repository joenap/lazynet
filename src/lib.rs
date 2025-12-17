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
#[pyclass]
pub struct Response {
    #[pyo3(get)]
    pub request: String,
    #[pyo3(get)]
    pub status: u16,
    #[pyo3(get)]
    pub reason: String,
    #[pyo3(get)]
    pub text: String,
    /// Error message if the request failed, None if successful.
    #[pyo3(get)]
    pub error: Option<String>,
    // Store parsed JSON as a Python object
    json_value: Option<Py<PyAny>>,
}

impl Clone for Response {
    fn clone(&self) -> Self {
        Python::with_gil(|py| Response {
            request: self.request.clone(),
            status: self.status,
            reason: self.reason.clone(),
            text: self.text.clone(),
            error: self.error.clone(),
            json_value: self.json_value.as_ref().map(|v| v.clone_ref(py)),
        })
    }
}

#[pymethods]
impl Response {
    /// Get the parsed JSON response body.
    #[getter]
    fn json(&self, py: Python<'_>) -> PyResult<PyObject> {
        match &self.json_value {
            Some(val) => Ok(val.clone_ref(py)),
            None => Ok(py.None()),
        }
    }

    fn __repr__(&self) -> String {
        // Use char-based truncation to avoid panic on multi-byte UTF-8
        let text_preview: String = self.text.chars().take(50).collect();
        let ellipsis = if self.text.chars().count() > 50 { "..." } else { "" };
        format!(
            "Response(request='{}', status={}, reason='{}', text='{}{}', error={:?})",
            self.request,
            self.status,
            self.reason,
            text_preview,
            ellipsis,
            self.error
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
            error: r.error,
            json_value,
        })
    }
}

/// Convert a serde_json::Value to a Python object.
fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
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
        let rust_response = py.allow_threads(|| self.lazynet.recv());

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
///
/// Example:
///     client = lazynet.Client()
///     for batch in batches:
///         for response in client.get(batch):
///             print(response.status)
#[pyclass]
pub struct Client {
    shared_client: SharedClient,
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (timeout_secs=None))]
    fn new(timeout_secs: Option<u64>) -> Self {
        let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
        Client {
            shared_client: SharedClient::with_timeout(timeout),
        }
    }

    /// Make HTTP GET requests using the shared connection pool.
    ///
    /// Args:
    ///     urls: An iterable of URL strings
    ///     concurrency_limit: Maximum concurrent requests (default: 1000)
    ///
    /// Returns:
    ///     An iterator of Response objects
    #[pyo3(signature = (urls, concurrency_limit=1000))]
    fn get(&self, urls: &Bound<'_, PyIterator>, concurrency_limit: usize) -> PyResult<ClientIterator> {
        // Collect URLs into a Vec
        let url_vec: Vec<String> = urls
            .try_iter()?
            .map(|r| r.and_then(|obj| obj.extract::<String>()))
            .collect::<PyResult<Vec<_>>>()?;

        // Use SharedClient::get which uses the shared runtime
        let receiver = self.shared_client.get(url_vec, concurrency_limit);

        Ok(ClientIterator { receiver })
    }
}

/// Iterator that yields HTTP responses from a Client.
#[pyclass]
pub struct ClientIterator {
    receiver: crossbeam_channel::Receiver<pipeline::ResponseMsg>,
}

#[pymethods]
impl ClientIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Response>> {
        let msg = py.allow_threads(|| self.receiver.recv());

        match msg {
            Ok(pipeline::ResponseMsg::Element(r)) => {
                Ok(Some(Response::from_rust_response(py, r)?))
            }
            Ok(pipeline::ResponseMsg::End) => Ok(None),
            Err(_) => Ok(None),
        }
    }
}

/// Make HTTP GET requests for the given URLs.
///
/// Args:
///     urls: An iterable of URL strings
///     concurrency_limit: Maximum concurrent requests (default: 1000)
///     timeout_secs: Request timeout in seconds (default: 30)
///
/// Returns:
///     An iterator of Response objects
///
/// Example:
///     urls = (f"http://example.com/{i}" for i in range(100))
///     for response in lazynet.get(urls):
///         print(response.status, response.text)
#[pyfunction]
#[pyo3(signature = (urls, concurrency_limit=1000, timeout_secs=None))]
fn get(
    urls: &Bound<'_, PyIterator>,
    concurrency_limit: usize,
    timeout_secs: Option<u64>,
) -> PyResult<LazynetIterator> {
    let timeout = timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
    let lazynet = Lazynet::with_config(100, concurrency_limit, timeout);

    // Consume the Python iterator and send URLs to the pipeline
    for url_result in urls.try_iter()? {
        let url: String = url_result?.extract()?;
        lazynet.send(url);
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
