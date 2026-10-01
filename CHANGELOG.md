# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.3] - 2026-10-01

### Fixed
- Intermittent deadlock on large batches when the response buffer filled before the caller started reading
- Hang when an iterator was abandoned with unread responses

### Changed
- Rust unit tests now run in `just test` and CI
- CI test jobs time out after 10 minutes

## [0.8.2] - 2026-10-01

### Fixed
- `just bump` now refreshes `Cargo.lock` and `uv.lock` so the tagged commit carries the new version

### Changed
- Moved nginx benchmark setup details from the README to the wiki

## [0.8.1] - 2026-02-23

### Changed
- `__version__` is derived from package metadata instead of being hardcoded
- Migrated from Makefile to justfile, with a `bump` recipe for releases

### Added
- Tests for header merging and `Client` per-request headers

## [0.8.0] - 2026-02-23

### Added
- `Response.bytes` field with the raw response body, for binary data
- Per-request headers via `(url, headers_dict)` tuples
- GitHub Release creation in the release workflow

## [0.7.0] - 2026-02-22

### Added
- Python 3.14 support
- `headers` parameter on `get()` and `Client` for custom request headers
- `Response.headers` with the response headers
- Automatic decompression of gzip, brotli and deflate responses
- Fallback for SSL certificate name mismatches: discover the redirect over HTTP, then follow it over HTTPS to the correct domain

### Changed
- Upgraded PyO3 from 0.24 to 0.28

## [0.6.0] - 2025-12-17

### Changed
- Restored the previous README

## [0.5.0] - 2025-12-17

### Changed
- Complete rewrite using Rust (PyO3/maturin) for improved performance
- Replaced asyncio/aiohttp with tokio/reqwest async runtime
- Upgraded PyO3 from 0.20 to 0.24
- Replaced flake8 with ruff for Python linting

### Added
- `Client` class for connection pooling across multiple request batches
- `concurrency_limit` parameter to control parallel requests (default: 1000)
- `timeout_secs` parameter for request timeouts (default: 30 seconds)
- `Response.json` property for auto-parsed JSON responses
- `Response.error` field for error information
- `HttpClient` trait abstraction for testable mocking
- Comprehensive test suites: 111 Python tests, 47 Rust tests

### Performance
- Achieved 360,000+ requests/second throughput on localhost benchmarks
- Optimized spawn_blocking to use single task instead of per-message
