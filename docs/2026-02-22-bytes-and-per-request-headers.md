# Add `bytes` Property and Per-Request Headers

Date: 2026-02-22

## Problem

### Binary data corruption via `response.text`

Lazynet called `resp.text().await` (reqwest) to read response bodies, which decodes as UTF-8. For binary content like GRIB2 files, compressed archives, or images, this silently corrupts data — invalid UTF-8 sequences get replaced with U+FFFD, and the resulting string can't be round-tripped back to the original bytes. A 1.2MB binary file might decode to 1.17M characters with data loss.

There was no way to get the raw bytes from a response.

### No per-request headers

`lazynet.get(urls, headers={...})` applies the same headers to every request in a batch. This doesn't work for use cases like HTTP Range requests, where each request in a batch needs a unique `Range: bytes=X-Y` header. Callers had to fall back to a ThreadPoolExecutor+requests pattern to work around this.

## Solution

### 1. `response.bytes` — raw binary content

The Rust pipeline now reads response bodies as raw bytes (`resp.bytes().await`) and derives text from them via `String::from_utf8_lossy`. Both are stored:

- `response.bytes` — `bytes` in Python, the exact bytes from the server
- `response.text` — `str` in Python, the lossy UTF-8 decode (same behavior as before for text content, safe for binary)

Error responses have `bytes = b""`.

PyO3 automatically converts Rust `Vec<u8>` to Python `bytes`, so there's no serialization overhead.

### 2. Per-request headers via `(url, headers)` tuples

`get()` and `Client.get()` now accept an iterable of either:
- Plain URL strings (existing behavior)
- `(url, headers_dict)` tuples (new)

Both forms can be mixed in the same iterable. Per-request headers merge with batch-level headers, with per-request values taking precedence for duplicate keys.

```python
# Old API still works — same headers for all
lazynet.get(iter(urls), headers={"Authorization": "Bearer token"})

# New API — per-request headers
requests = ((url, {"Range": f"bytes={start}-{end}"}) for url, start, end in manifest)
lazynet.get(requests)

# Mixed — batch headers merged with per-request headers
requests = ((url, {"Range": f"bytes={s}-{e}"}) for url, s, e in manifest)
lazynet.get(requests, headers={"User-Agent": "myapp/1.0"})
```

This is fully backward compatible. The implementation tries `item.extract::<(String, HashMap)>()` first, falling back to `item.extract::<String>()`.

## Files changed

| File | Change |
|------|--------|
| `src/pipeline.rs` | Added `bytes: Vec<u8>` to `Response` struct and constructors |
| `src/http_client.rs` | `resp.bytes().await` instead of `resp.text().await`; `MockResponse` carries bytes |
| `src/lib.rs` | `bytes` on PyO3 Response; tuple extraction in `get()` and `Client.get()` |
| `tests/test_lazynet.py` | 11 new tests for bytes and per-request headers |

## Design notes

- **Why `from_utf8_lossy` instead of `from_utf8`?** Lossy conversion never fails, so `response.text` is always available. For text responses this produces identical results. For binary responses, text is garbled but that's expected — callers should use `.bytes` instead. This preserves backward compatibility for existing code that only reads `.text`.

- **Why tuple extraction instead of a new method?** Adding a separate `get_with_per_request_headers()` would fragment the API. Tuple extraction keeps a single entry point and composes naturally with generators. The `(url, headers)` pattern is also familiar from libraries like `grequests`.

- **Client.get() uses two code paths.** When no tuples are detected, it uses the fast `SharedClient` path (shared runtime, no extra channel overhead). When tuples are present, it falls back to a `Lazynet` pipeline instance that supports per-request headers. This avoids penalizing the common case.
