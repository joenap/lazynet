"""Performance benchmarks for lazynet using pytest-benchmark.

Prerequisites: HTTP server running on localhost:8080
Run with: uv run pytest tests/test_perf.py --benchmark-only

Uses lazynet.Client for connection pooling to avoid exhausting
ephemeral ports during rapid benchmark iterations.
"""

import pytest

import lazynet

URL = "http://127.0.0.1:8080/"


@pytest.fixture(scope="function")
def client():
    """Fresh client per test to avoid cross-runtime issues."""
    return lazynet.Client()


def make_requests(
    client: lazynet.Client, num_urls: int, concurrency: int = 1000
) -> int:
    """Make HTTP requests and return count of responses."""
    urls = (URL for _ in range(num_urls))
    responses = list(client.get(urls, concurrency_limit=concurrency))
    return len(responses)


class TestRequestThroughput:
    """Benchmark request throughput at different scales."""

    @pytest.mark.benchmark(group="throughput", min_rounds=3)
    def test_10_requests(self, benchmark, client):
        result = benchmark(make_requests, client, 10)
        assert result == 10

    @pytest.mark.benchmark(group="throughput", min_rounds=3)
    def test_100_requests(self, benchmark, client):
        result = benchmark(make_requests, client, 100)
        assert result == 100

    @pytest.mark.benchmark(group="throughput", min_rounds=3)
    def test_1000_requests(self, benchmark, client):
        result = benchmark(make_requests, client, 1000)
        assert result == 1000


class TestConcurrencyLevels:
    """Benchmark different concurrency limits."""

    @pytest.mark.benchmark(group="concurrency", min_rounds=3)
    def test_concurrency_10(self, benchmark, client):
        result = benchmark(make_requests, client, 100, concurrency=10)
        assert result == 100

    @pytest.mark.benchmark(group="concurrency", min_rounds=3)
    def test_concurrency_50(self, benchmark, client):
        result = benchmark(make_requests, client, 100, concurrency=50)
        assert result == 100

    @pytest.mark.benchmark(group="concurrency", min_rounds=3)
    def test_concurrency_100(self, benchmark, client):
        result = benchmark(make_requests, client, 100, concurrency=100)
        assert result == 100

    @pytest.mark.benchmark(group="concurrency", min_rounds=3)
    def test_concurrency_300(self, benchmark, client):
        result = benchmark(make_requests, client, 100, concurrency=300)
        assert result == 100

    @pytest.mark.benchmark(group="concurrency", min_rounds=3)
    def test_concurrency_1000(self, benchmark, client):
        result = benchmark(make_requests, client, 100, concurrency=1000)
        assert result == 100
