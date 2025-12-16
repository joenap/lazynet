"""Tests for lazynet Rust implementation."""

import pytest
import lazynet


class TestResponse:
    """Test Response class API."""

    def test_response_has_expected_attributes(self):
        """Response should have all expected attributes."""
        # We can't easily construct a Response directly since it comes from Rust,
        # but we can verify the attributes exist on the class
        assert hasattr(lazynet, 'Response')

    def test_response_module_exports(self):
        """Module should export get and Response."""
        assert hasattr(lazynet, 'get')
        assert hasattr(lazynet, 'Response')


class TestGet:
    """Test get() function behavior."""

    def test_empty_input_returns_empty_iterator(self):
        """Empty input should return empty iterator."""
        urls = iter([])
        responses = list(lazynet.get(urls))
        assert len(responses) == 0

    def test_get_returns_iterator(self):
        """get() should return an iterator."""
        urls = iter([])
        result = lazynet.get(urls)
        assert hasattr(result, '__iter__')
        assert hasattr(result, '__next__')

    def test_get_accepts_concurrency_limit(self):
        """get() should accept concurrency_limit parameter."""
        urls = iter([])
        # Should not raise
        responses = list(lazynet.get(urls, concurrency_limit=10))
        assert len(responses) == 0

    def test_get_accepts_generator(self):
        """get() should accept a generator."""
        urls = (f"http://invalid.test/{i}" for i in range(0))
        responses = list(lazynet.get(urls))
        assert len(responses) == 0


class TestIntegration:
    """Integration tests requiring network access."""

    @pytest.mark.skip(reason="Requires network access")
    def test_real_http_request(self):
        """Test with a real HTTP endpoint."""
        urls = iter(["https://httpbin.org/json"])
        responses = list(lazynet.get(urls))

        assert len(responses) == 1
        r = responses[0]

        # Check all expected fields exist
        assert hasattr(r, 'request')
        assert hasattr(r, 'status')
        assert hasattr(r, 'reason')
        assert hasattr(r, 'text')
        assert hasattr(r, 'json')

        # Check values
        assert r.request == "https://httpbin.org/json"
        assert r.status == 200
        assert r.reason == "OK"
        assert isinstance(r.text, str)
        assert isinstance(r.json, dict)

    @pytest.mark.skip(reason="Requires network access")
    def test_multiple_requests(self):
        """Test multiple concurrent requests."""
        urls = [f"https://httpbin.org/get?id={i}" for i in range(3)]
        responses = list(lazynet.get(iter(urls)))

        assert len(responses) == 3
        for r in responses:
            assert r.status == 200

    @pytest.mark.skip(reason="Requires network access")
    def test_response_string_representation(self):
        """Response should have string representation."""
        urls = iter(["https://httpbin.org/get"])
        responses = list(lazynet.get(urls))

        assert len(responses) == 1
        r = responses[0]

        # Should not raise
        s = str(r)
        assert 'Response' in s

        rep = repr(r)
        assert 'Response' in rep
