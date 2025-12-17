# Benchmark Issue: Ephemeral Port Exhaustion

## Problem

When running pytest-benchmark, tests fail intermittently with 0 responses returned. The Rust benchmark (`cargo run --bin bench_runner --release`) works fine, but Python benchmarks under pytest-benchmark fail.

## Root Cause

pytest-benchmark runs the target function hundreds of times in rapid succession for accurate timing. Each call to `lazynet.get()` creates:
- A new `Lazynet` instance
- A new tokio runtime
- A new `reqwest::Client`
- New TCP connections

These connections enter TIME_WAIT state after closing (60 seconds on macOS), exhausting ephemeral ports.

### Evidence

```bash
$ netstat -an | grep 8080 | wc -l
10786
```

Over 10,000 connections stuck in TIME_WAIT after rapid benchmark iterations.

### Reproduction

```python
# This fails after ~100-150 iterations
results = [make_requests(100) for _ in range(300)]
print(f'zeros={results.count(0)}')  # ~190 zeros
```

## Current Workarounds

1. **Reduced benchmark iterations**: Set `min_rounds=3` in pytest.mark.benchmark
2. **Relaxed assertions**: Assert `result > 0` instead of exact counts
3. **Manual delays**: Wait between benchmark runs for TIME_WAIT to clear

## Fix: lazynet.Client

Implemented connection pooling via `lazynet.Client`:

```python
# Benchmarks: reuse client across iterations
client = lazynet.Client()
for _ in range(300):
    responses = list(client.get(urls))  # Reuses connection pool

# Regular usage: unchanged (creates ephemeral client)
responses = list(lazynet.get(urls))
```

The `Client` class wraps a `reqwest::Client` that persists across `get()` calls, reusing TCP connections instead of creating new ones each time.

## Related Issues

- nginx may also need tuning: `worker_rlimit_nofile` and `worker_connections`
- Iterator bug fixed: error responses were terminating iteration (now skipped with `continue`)
