# lazynet

[![PyPI version](https://badge.fury.io/py/lazynet.svg)](https://badge.fury.io/py/lazynet)
[![CI](https://github.com/joenap/lazynet/workflows/CI/badge.svg)](https://github.com/joenap/lazynet/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Lazy-evaluated, asynchronous HTTP requests powered by Rust.

Pass in a generator of URLs, get back a generator of responses. Requests execute concurrently when responses are consumed.

## Features

- **Lazy evaluation** - Requests only execute when you iterate over responses
- **High concurrency** - Configurable limit (default: 1000 concurrent requests)
- **Connection pooling** - `Client` class for reusing connections across batches
- **Fast** - Rust async runtime (tokio + reqwest) for maximum throughput
- **Simple API** - Synchronous Python interface, async complexity hidden

## Installation

```bash
pip install lazynet
```

## Quick Start

```python
import lazynet

# Generator of URLs
urls = (f"https://api.example.com/item/{i}" for i in range(100))

# Lazy iteration - requests execute as you consume
for response in lazynet.get(urls):
    print(response.status, response.text[:50])
    if response.json:  # Auto-parsed JSON
        print(response.json["key"])
```

## API Reference

### `lazynet.get(urls, concurrency_limit=1000, timeout_secs=30)`

Make HTTP GET requests lazily.

**Parameters:**
- `urls` - Iterable of URL strings
- `concurrency_limit` - Maximum concurrent requests (default: 1000)
- `timeout_secs` - Request timeout in seconds (default: 30)

**Returns:** Iterator of `Response` objects

```python
# Basic usage
for response in lazynet.get(urls):
    process(response)

# With options
for response in lazynet.get(urls, concurrency_limit=100, timeout_secs=60):
    process(response)
```

### `lazynet.Client(timeout_secs=30)`

Reusable HTTP client for connection pooling across multiple request batches.

```python
client = lazynet.Client(timeout_secs=30)

for batch in url_batches:
    for response in client.get(batch, concurrency_limit=500):
        process(response)
```

### `Response`

Response object returned for each request.

**Attributes:**
- `request` - Original URL string
- `status` - HTTP status code (int)
- `reason` - Status reason phrase (e.g., "OK", "Not Found")
- `text` - Response body as string
- `json` - Auto-parsed JSON (dict/list) or `None` if not valid JSON
- `error` - Error message if request failed, otherwise `None`

```python
for response in lazynet.get(urls):
    if response.error:
        print(f"Failed: {response.request} - {response.error}")
    elif response.status == 200:
        data = response.json or response.text
        print(f"Success: {data}")
```

## Performance

lazynet achieves high throughput by using Rust's async runtime (tokio) with the reqwest HTTP client. On localhost benchmarks:

- **360,000+ requests/second** peak throughput
- **~11,000 requests/second** single-core estimate

Actual performance depends on network latency, server response time, and system resources.

## License

MIT
