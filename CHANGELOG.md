# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
