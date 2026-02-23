# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Project Overview

Lazynet provides lazy-evaluated, asynchronous HTTP requests. Pass in a generator of URLs, get back a generator of responses. Requests execute concurrently when responses are consumed.

## Build Commands

See `justfile` for all commands (install, test, lint, bump, bench, dist, etc.).

Single test: `uv run pytest tests/test_lazynet.py::TestGet::test_name -v`

Version is defined in `Cargo.toml` and `pyproject.toml`. `__version__` is derived at runtime from package metadata — never edit it manually.

## Architecture

### Source Files (src/)

```
lib.rs       - PyO3 module exposing get() function and Response class
pipeline.rs  - Core async pipeline: Lazynet struct, channel tasks, HTTP client
```

### Pipeline Flow

```
Python: lazynet.get(urls) → Lazynet.send(url) → crossbeam → async_request_task
                                                                  ↓
Python: for r in ...:  ←  Lazynet.recv()  ←  crossbeam  ←  async_response_task
        __next__()                                              ↑
        (GIL released)                                   async_http_client_task
                                                         (semaphore-limited)
```

- `crossbeam-channel`: Bridges sync Python ↔ async Rust
- `TaskTracker`: Ensures graceful shutdown of in-flight requests
- `py.allow_threads()`: Releases GIL while blocking on recv

### Python API

```python
import lazynet

urls = (f"http://example.com/{i}" for i in range(100))
for response in lazynet.get(urls, concurrency_limit=1000):
    print(response.status, response.text[:50])
    data = response.json  # Auto-parsed JSON (or None if invalid)
```

### Response Fields

- `request` - Original URL
- `status` - HTTP status code (u16)
- `reason` - Status text
- `text` - Response body
- `json` - Auto-parsed JSON (dict/list/None)

## Benchmarks

Benchmarks require an HTTP server on localhost:8080 (nginx recommended). See `just bench`.

**Known issue**: pytest-benchmark exhausts ephemeral ports under rapid iteration. See `docs/benchmark-issue.md`.

