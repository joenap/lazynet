# Unit Testing Analysis

**Date:** 2025-12-17
**Repository:** lazynet

## Test Infrastructure

| Component | Location | Tests |
|-----------|----------|-------|
| Python functional | `tests/test_lazynet.py` | 9 tests |
| Python benchmarks | `tests/test_perf.py` | 10 benchmarks |
| Rust unit tests | `src/pipeline.rs:372-399` | 2 tests |
| Smoke test | `scripts/smoke_test.py` | 1 script |

## Frameworks Used

- **pytest** with pytest-asyncio and pytest-benchmark
- **coverage** for code coverage
- **Rust built-in test framework** (`cargo test`)

## Test File Structure

```
tests/
├── __init__.py (empty)
├── test_lazynet.py (103 lines, 9 tests)
└── test_perf.py (76 lines, 10 benchmark tests)

benches/
└── bench_runner.rs (86 lines, Rust benchmark binary)

scripts/
└── smoke_test.py (20 lines, quick smoke test)

src/
└── pipeline.rs (2 embedded Rust unit tests)
```

## What's Tested

### Python Tests (`test_lazynet.py`)

**TestResponse:**
- Response class has expected attributes
- Module exports (get function and Response class)

**TestGet:**
- Empty input returns empty iterator
- `get()` returns iterator with `__iter__` and `__next__`
- Accepts `concurrency_limit` parameter
- Accepts generator as input

**TestIntegration (skipped):**
- Real HTTP request handling
- Multiple concurrent requests
- Response string representation (`__repr__` and `__str__`)

### Rust Tests (`pipeline.rs`)

- Empty input handling (no URLs sent)
- Single request with response field verification

### Performance Benchmarks

**Python (`test_perf.py`):**
- Throughput tests: 10, 100, 1000 requests
- Concurrency tests: limits of 10, 50, 100, 300, 1000

**Rust (`bench_runner.rs`):**
- Throughput: 100, 500, 1000, 5000, 30000, 100000 requests
- Concurrency: 10, 50, 100, 300, 500, 1000 concurrent

## Test Coverage Gaps

| Category | Missing Coverage |
|----------|------------------|
| Error handling | HTTP errors, invalid URLs, connection failures |
| Timeouts | Timeout behavior and configuration |
| Concurrency | Limit enforcement, race conditions |
| JSON parsing | Various JSON response types, malformed JSON |
| Resource cleanup | Memory leaks, connection cleanup |
| Edge cases | High concurrency (100k+), large responses |
| Status codes | Non-200 responses, redirects |
| Headers | Custom headers, content-type handling |

## Key Issues

### 1. Server Dependency
All tests require nginx on `localhost:8080`. This makes tests:
- Not CI-friendly without additional setup
- Dependent on external service availability
- Subject to network timing variations

### 2. Skipped Tests
Integration tests are marked `@pytest.mark.skip`, reducing actual coverage.

### 3. No Mocking
No mock framework is used. Cannot test:
- Error paths without real failures
- Timeout behavior without slow servers
- Edge cases without complex server setup

### 4. Minimal Rust Tests
Only 2 unit tests for the core pipeline logic in `pipeline.rs`.

### 5. No Shared Fixtures
Missing `conftest.py` for shared pytest fixtures across test modules.

### 6. Port Exhaustion
Documented in `docs/benchmark-issue.md`:
- pytest-benchmark exhausts ephemeral ports during rapid iterations
- Connections enter TIME_WAIT state (60s on macOS)
- Mitigated with `min_rounds=3` and function-scoped client fixtures

## Test Commands

```bash
# Run all Python tests
make test
uv run pytest

# Run single test
uv run pytest tests/test_lazynet.py::TestGet::test_name -v

# Run with coverage
uv run coverage run --source lazynet -m pytest
uv run coverage report -m

# Run Python benchmarks
uv run pytest tests/test_perf.py --benchmark-only -v

# Run Rust unit tests
cargo test --lib

# Run Rust benchmarks
make bench-rust
cargo run --bin bench_runner --release

# Smoke test
uv run python scripts/smoke_test.py

# Linting
make lint
```

## Test Configuration

**pytest** (`pyproject.toml`):
```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
```

**Development Dependencies:**
```toml
dev = [
    "pytest>=7.4",
    "pytest-asyncio>=0.23",
    "pytest-benchmark>=4.0",
    "coverage>=7.3",
    "flake8>=6.1",
    "maturin>=1.4,<2.0",
]
```

## Assessment

### Strengths

- Clean separation between functional and performance tests
- Benchmark infrastructure for performance tracking
- Smoke test script for quick validation
- Known issues documented (`docs/benchmark-issue.md`)
- Function-scoped fixtures prevent cross-test contamination

### Weaknesses

- Very low unit test count (2 Rust + 9 Python functional)
- Heavy reliance on external HTTP server
- No error path testing
- No mocking/stubbing capabilities
- Limited pytest configuration
- Integration tests are skipped by default
- Not suitable for CI without enhancements

### Recommendations

1. **Add mocking**: Use `responses` or `httpx` mock for Python, `mockito` or `wiremock` for Rust
2. **Enable skipped tests**: Create proper fixtures or test server setup
3. **Add conftest.py**: Share fixtures across test modules
4. **Increase Rust coverage**: Add tests for error paths, timeouts, concurrency
5. **CI setup**: Add test server container or mock server to CI pipeline
6. **Error testing**: Test invalid URLs, connection failures, timeout scenarios
7. **JSON edge cases**: Test malformed JSON, empty responses, large payloads
