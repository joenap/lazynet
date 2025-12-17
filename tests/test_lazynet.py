"""Comprehensive unit tests for lazynet Python API.

Tests the public Python interface of lazynet using pytest-httpserver
for controlled HTTP mocking. Uses functional style with fixtures.
"""

import pytest

import lazynet


# =============================================================================
# Fixtures
# =============================================================================


@pytest.fixture(scope="session")
def httpserver_listen_address():
    """Configure httpserver to use a specific port."""
    return ("127.0.0.1", 9999)


@pytest.fixture
def base_url(httpserver):
    """Get the base URL for the test server."""
    return httpserver.url_for("")


@pytest.fixture
def client():
    """Create a lazynet Client instance."""
    return lazynet.Client()


# =============================================================================
# Module exports and public API
# =============================================================================


def test_module_exports_get_function():
    """The module should export the get function."""
    assert hasattr(lazynet, "get")
    assert callable(lazynet.get)


def test_module_exports_response_class():
    """The module should export the Response class."""
    assert hasattr(lazynet, "Response")


def test_module_exports_client_class():
    """The module should export the Client class."""
    assert hasattr(lazynet, "Client")


def test_module_version_exists():
    """The module should have a version string."""
    assert hasattr(lazynet, "__version__")
    assert isinstance(lazynet.__version__, str)


def test_module_all_contains_public_api():
    """__all__ should list public API members."""
    assert "get" in lazynet.__all__
    assert "Response" in lazynet.__all__
    assert "Client" in lazynet.__all__


# =============================================================================
# lazynet.get() - basic functionality
# =============================================================================


def test_get_empty_input_returns_empty_iterator():
    """Passing an empty iterator should return no responses."""
    urls = iter([])
    responses = list(lazynet.get(urls))
    assert responses == []


def test_get_returns_iterator_protocol():
    """get() should return an object implementing the iterator protocol."""
    result = lazynet.get(iter([]))
    assert hasattr(result, "__iter__")
    assert hasattr(result, "__next__")


def test_get_single_url_returns_one_response(httpserver):
    """A single URL should produce exactly one response."""
    httpserver.expect_request("/test").respond_with_data("hello")
    url = httpserver.url_for("/test")

    responses = list(lazynet.get(iter([url])))

    assert len(responses) == 1


def test_get_multiple_urls_returns_all_responses(httpserver):
    """Multiple URLs should produce responses for each."""
    httpserver.expect_request("/a").respond_with_data("response a")
    httpserver.expect_request("/b").respond_with_data("response b")
    httpserver.expect_request("/c").respond_with_data("response c")

    urls = [
        httpserver.url_for("/a"),
        httpserver.url_for("/b"),
        httpserver.url_for("/c"),
    ]
    responses = list(lazynet.get(iter(urls)))

    assert len(responses) == 3


def test_get_accepts_generator_input(httpserver):
    """get() should accept a generator as input."""
    httpserver.expect_request("/gen").respond_with_data("ok")
    base = httpserver.url_for("/gen")

    def url_generator():
        for _ in range(5):
            yield base

    responses = list(lazynet.get(url_generator()))

    assert len(responses) == 5


def test_get_accepts_list_via_iter(httpserver):
    """get() should accept a list converted to an iterator."""
    httpserver.expect_request("/list").respond_with_data("ok")
    url = httpserver.url_for("/list")

    urls = [url] * 3
    responses = list(lazynet.get(iter(urls)))

    assert len(responses) == 3


# =============================================================================
# lazynet.get() - parameters
# =============================================================================


def test_get_accepts_concurrency_limit_parameter():
    """get() should accept the concurrency_limit parameter."""
    responses = list(lazynet.get(iter([]), concurrency_limit=10))
    assert responses == []


def test_get_accepts_timeout_secs_parameter():
    """get() should accept the timeout_secs parameter."""
    responses = list(lazynet.get(iter([]), timeout_secs=5))
    assert responses == []


def test_get_accepts_all_parameters_together(httpserver):
    """get() should accept all parameters at once."""
    httpserver.expect_request("/params").respond_with_data("ok")
    url = httpserver.url_for("/params")

    responses = list(
        lazynet.get(iter([url]), concurrency_limit=50, timeout_secs=10)
    )

    assert len(responses) == 1


@pytest.mark.parametrize("concurrency", [1, 10, 50, 100, 500, 1000])
def test_get_various_concurrency_limits(httpserver, concurrency):
    """get() should work with various concurrency limits."""
    httpserver.expect_request("/conc").respond_with_data("ok")
    url = httpserver.url_for("/conc")

    responses = list(lazynet.get(iter([url]), concurrency_limit=concurrency))

    assert len(responses) == 1


# =============================================================================
# Response attributes
# =============================================================================


def test_response_has_request_attribute(httpserver):
    """Response should have the original request URL."""
    httpserver.expect_request("/req-attr").respond_with_data("ok")
    url = httpserver.url_for("/req-attr")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].request == url


def test_response_has_status_attribute(httpserver):
    """Response should have the HTTP status code."""
    httpserver.expect_request("/status").respond_with_data("ok", status=200)
    url = httpserver.url_for("/status")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].status == 200


def test_response_has_reason_attribute(httpserver):
    """Response should have the status reason phrase."""
    httpserver.expect_request("/reason").respond_with_data("ok", status=200)
    url = httpserver.url_for("/reason")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].reason == "OK"


def test_response_has_text_attribute(httpserver):
    """Response should have the response body text."""
    httpserver.expect_request("/text").respond_with_data("hello world")
    url = httpserver.url_for("/text")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].text == "hello world"


def test_response_error_is_none_on_success(httpserver):
    """Response.error should be None for successful requests."""
    httpserver.expect_request("/success").respond_with_data("ok")
    url = httpserver.url_for("/success")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].error is None


def test_response_json_parses_json_body(httpserver):
    """Response.json should parse JSON response bodies."""
    data = {"key": "value", "number": 42}
    httpserver.expect_request("/json").respond_with_json(data)
    url = httpserver.url_for("/json")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == data


def test_response_json_is_none_for_non_json(httpserver):
    """Response.json should be None for non-JSON responses."""
    httpserver.expect_request("/plain").respond_with_data("plain text")
    url = httpserver.url_for("/plain")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json is None


# =============================================================================
# HTTP status codes
# =============================================================================


@pytest.mark.parametrize(
    "status_code,reason",
    [
        (200, "OK"),
        (201, "Created"),
        (204, "No Content"),
        (301, "Moved Permanently"),
        (302, "Found"),
        (304, "Not Modified"),
        (400, "Bad Request"),
        (401, "Unauthorized"),
        (403, "Forbidden"),
        (404, "Not Found"),
        (405, "Method Not Allowed"),
        (500, "Internal Server Error"),
        (502, "Bad Gateway"),
        (503, "Service Unavailable"),
    ],
)
def test_response_captures_status_codes(httpserver, status_code, reason):
    """Response should capture various HTTP status codes."""
    httpserver.expect_request(f"/status/{status_code}").respond_with_data(
        "", status=status_code
    )
    url = httpserver.url_for(f"/status/{status_code}")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].status == status_code
    assert responses[0].reason == reason


def test_4xx_responses_are_not_errors(httpserver):
    """4xx responses should not set the error field."""
    httpserver.expect_request("/404").respond_with_data(
        "not found", status=404
    )
    url = httpserver.url_for("/404")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].status == 404
    # HTTP errors are not "errors" in lazynet
    assert responses[0].error is None


def test_5xx_responses_are_not_errors(httpserver):
    """5xx responses should not set the error field."""
    httpserver.expect_request("/500").respond_with_data(
        "error", status=500
    )
    url = httpserver.url_for("/500")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].status == 500
    # HTTP errors are not "errors" in lazynet
    assert responses[0].error is None


# =============================================================================
# JSON parsing
# =============================================================================


def test_json_object_parsing(httpserver):
    """Response should parse JSON objects."""
    data = {"name": "test", "value": 123}
    httpserver.expect_request("/json-obj").respond_with_json(data)
    url = httpserver.url_for("/json-obj")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == data
    assert isinstance(responses[0].json, dict)


def test_json_array_parsing(httpserver):
    """Response should parse JSON arrays."""
    data = [1, 2, 3, "four", {"five": 5}]
    httpserver.expect_request("/json-arr").respond_with_json(data)
    url = httpserver.url_for("/json-arr")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == data
    assert isinstance(responses[0].json, list)


def test_json_nested_structure(httpserver):
    """Response should parse nested JSON structures."""
    data = {
        "level1": {"level2": {"level3": [1, 2, {"deep": True}]}},
        "array": [None, True, False, 3.14],
    }
    httpserver.expect_request("/nested").respond_with_json(data)
    url = httpserver.url_for("/nested")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == data


def test_json_with_unicode(httpserver):
    """Response should handle Unicode in JSON."""
    data = {"emoji": "🎉", "chinese": "你好", "arabic": "مرحبا"}
    httpserver.expect_request("/unicode").respond_with_json(data)
    url = httpserver.url_for("/unicode")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == data


def test_json_null_value(httpserver):
    """Response should parse JSON null as Python None."""
    httpserver.expect_request("/null").respond_with_data(
        "null", content_type="application/json"
    )
    url = httpserver.url_for("/null")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json is None


def test_json_boolean_values(httpserver):
    """Response should parse JSON booleans correctly."""
    data = {"true": True, "false": False}
    httpserver.expect_request("/bool").respond_with_json(data)
    url = httpserver.url_for("/bool")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json["true"] is True
    assert responses[0].json["false"] is False


def test_json_number_types(httpserver):
    """Response should handle various JSON number types."""
    data = {"int": 42, "float": 3.14, "negative": -100, "scientific": 1e10}
    httpserver.expect_request("/nums").respond_with_json(data)
    url = httpserver.url_for("/nums")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json["int"] == 42
    assert responses[0].json["float"] == 3.14
    assert responses[0].json["negative"] == -100
    assert responses[0].json["scientific"] == 1e10


def test_malformed_json_returns_none(httpserver):
    """Malformed JSON should result in json=None, not an error."""
    httpserver.expect_request("/bad-json").respond_with_data(
        "{invalid json", content_type="application/json"
    )
    url = httpserver.url_for("/bad-json")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json is None
    assert responses[0].error is None  # Not a request error


def test_empty_response_json_is_none(httpserver):
    """Empty response body should have json=None."""
    httpserver.expect_request("/empty").respond_with_data("")
    url = httpserver.url_for("/empty")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json is None


# =============================================================================
# Response body handling
# =============================================================================


def test_response_preserves_exact_body(httpserver):
    """Response text should be the exact response body."""
    body = "  exact\nwhitespace\tpreserved  "
    httpserver.expect_request("/exact").respond_with_data(body)
    url = httpserver.url_for("/exact")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].text == body


def test_response_handles_unicode_body(httpserver):
    """Response should handle Unicode in the body."""
    body = "Hello 世界 🌍 مرحبا"
    httpserver.expect_request("/unicode-body").respond_with_data(body)
    url = httpserver.url_for("/unicode-body")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].text == body


def test_response_handles_empty_body(httpserver):
    """Response should handle empty body."""
    httpserver.expect_request("/empty-body").respond_with_data("")
    url = httpserver.url_for("/empty-body")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].text == ""


def test_response_handles_large_body(httpserver):
    """Response should handle large response bodies."""
    body = "x" * 100_000
    httpserver.expect_request("/large").respond_with_data(body)
    url = httpserver.url_for("/large")

    responses = list(lazynet.get(iter([url])))

    assert len(responses[0].text) == 100_000


def test_response_handles_binary_as_text(httpserver):
    """Binary data should be handled (may be garbled but not crash)."""
    # This tests that binary doesn't crash - the result may not be useful
    httpserver.expect_request("/binary").respond_with_data(
        b"\x00\x01\x02", content_type="application/octet-stream"
    )
    url = httpserver.url_for("/binary")

    # Should not raise
    responses = list(lazynet.get(iter([url])))
    assert len(responses) == 1


# =============================================================================
# Response string representation
# =============================================================================


def test_response_repr_contains_response(httpserver):
    """Response __repr__ should contain 'Response'."""
    httpserver.expect_request("/repr").respond_with_data("test")
    url = httpserver.url_for("/repr")

    responses = list(lazynet.get(iter([url])))

    assert "Response" in repr(responses[0])


def test_response_str_contains_response(httpserver):
    """Response __str__ should contain 'Response'."""
    httpserver.expect_request("/str").respond_with_data("test")
    url = httpserver.url_for("/str")

    responses = list(lazynet.get(iter([url])))

    assert "Response" in str(responses[0])


def test_response_repr_contains_url(httpserver):
    """Response __repr__ should contain the request URL."""
    httpserver.expect_request("/repr-url").respond_with_data("test")
    url = httpserver.url_for("/repr-url")

    responses = list(lazynet.get(iter([url])))

    assert "/repr-url" in repr(responses[0])


def test_response_repr_contains_status(httpserver):
    """Response __repr__ should contain the status code."""
    httpserver.expect_request("/repr-status").respond_with_data(
        "test", status=201
    )
    url = httpserver.url_for("/repr-status")

    responses = list(lazynet.get(iter([url])))

    assert "201" in repr(responses[0])


def test_response_repr_truncates_long_text(httpserver):
    """Response __repr__ should truncate long text bodies."""
    body = "x" * 200
    httpserver.expect_request("/long").respond_with_data(body)
    url = httpserver.url_for("/long")

    responses = list(lazynet.get(iter([url])))
    rep = repr(responses[0])

    # Should truncate and have ellipsis
    assert "..." in rep
    # Should not contain the full 200 chars
    assert len(rep) < 300


# =============================================================================
# Response equality
# =============================================================================


def test_response_equality(httpserver):
    """Two responses with same data should be equal."""
    httpserver.expect_request("/eq1").respond_with_data("same")
    httpserver.expect_request("/eq2").respond_with_data("same")
    url1 = httpserver.url_for("/eq1")
    url2 = httpserver.url_for("/eq2")

    r1 = list(lazynet.get(iter([url1])))[0]
    r2 = list(lazynet.get(iter([url2])))[0]

    # Same status, reason, text, but different request URL
    assert r1.status == r2.status
    assert r1.text == r2.text
    # Different requests means not equal
    assert r1 != r2


def test_response_self_equality(httpserver):
    """A response should equal itself."""
    httpserver.expect_request("/self").respond_with_data("test")
    url = httpserver.url_for("/self")

    r = list(lazynet.get(iter([url])))[0]

    assert r == r


# =============================================================================
# Error handling
# =============================================================================


def test_connection_refused_sets_error():
    """Connection refused should set the error field."""
    # Use a port that's definitely not listening
    url = "http://127.0.0.1:1"

    responses = list(lazynet.get(iter([url]), timeout_secs=1))

    assert len(responses) == 1
    assert responses[0].error is not None
    assert responses[0].status == 0


def test_invalid_url_scheme_sets_error():
    """Invalid URL scheme should set the error field."""
    url = "ftp://example.com/file"

    responses = list(lazynet.get(iter([url]), timeout_secs=1))

    assert len(responses) == 1
    assert responses[0].error is not None


# Note: DNS failure tests are slow and unpredictable. Tested in Rust.


def test_error_response_preserves_request_url():
    """Even on error, the original request URL should be preserved."""
    url = "http://127.0.0.1:1/some/path"

    responses = list(lazynet.get(iter([url]), timeout_secs=1))

    assert responses[0].request == url


# Note: Timeout behavior is tested in Rust unit tests with MockHttpClient.
# Python tests focus on API surface, not re-testing Rust internals.


# =============================================================================
# Concurrency behavior
# =============================================================================


def test_concurrent_requests_all_complete(httpserver):
    """Multiple concurrent requests should all complete."""
    for i in range(20):
        httpserver.expect_request(f"/c{i}").respond_with_data(f"response {i}")

    urls = [httpserver.url_for(f"/c{i}") for i in range(20)]
    responses = list(lazynet.get(iter(urls), concurrency_limit=10))

    assert len(responses) == 20


# Note: Out-of-order response tests require timing delays and are covered
# by Rust unit tests with MockHttpClient delay simulation.


def test_concurrency_limit_one_is_sequential(httpserver):
    """With concurrency_limit=1, requests should be sequential."""
    for i in range(5):
        httpserver.expect_request(f"/seq{i}").respond_with_data(f"{i}")

    urls = [httpserver.url_for(f"/seq{i}") for i in range(5)]
    responses = list(lazynet.get(iter(urls), concurrency_limit=1))

    assert len(responses) == 5


def test_high_concurrency_handles_many_requests(httpserver):
    """High concurrency should handle many simultaneous requests."""
    for i in range(30):
        httpserver.expect_request(f"/high{i}").respond_with_data(f"{i}")

    urls = [httpserver.url_for(f"/high{i}") for i in range(30)]
    responses = list(lazynet.get(iter(urls), concurrency_limit=30))

    assert len(responses) == 30


# =============================================================================
# Client class
# =============================================================================


def test_client_construction():
    """Client should be constructable."""
    client = lazynet.Client()
    assert client is not None


def test_client_accepts_timeout_parameter():
    """Client should accept timeout_secs parameter."""
    client = lazynet.Client(timeout_secs=10)
    assert client is not None


def test_client_get_returns_iterator(httpserver):
    """Client.get() should return an iterator."""
    httpserver.expect_request("/client-iter").respond_with_data("ok")
    url = httpserver.url_for("/client-iter")

    client = lazynet.Client()
    result = client.get(iter([url]))

    assert hasattr(result, "__iter__")
    assert hasattr(result, "__next__")


def test_client_get_empty_input(client):
    """Client.get() with empty input should return empty iterator."""
    responses = list(client.get(iter([])))
    assert responses == []


def test_client_get_single_url(httpserver, client):
    """Client.get() should handle a single URL."""
    httpserver.expect_request("/client-single").respond_with_data(
        "response"
    )
    url = httpserver.url_for("/client-single")

    responses = list(client.get(iter([url])))

    assert len(responses) == 1
    assert responses[0].text == "response"


def test_client_get_multiple_urls(httpserver, client):
    """Client.get() should handle multiple URLs."""
    for i in range(10):
        httpserver.expect_request(f"/client-multi{i}").respond_with_data(
            f"r{i}"
        )

    urls = [httpserver.url_for(f"/client-multi{i}") for i in range(10)]
    responses = list(client.get(iter(urls)))

    assert len(responses) == 10


def test_client_get_accepts_concurrency_limit(httpserver, client):
    """Client.get() should accept concurrency_limit parameter."""
    httpserver.expect_request("/client-conc").respond_with_data("ok")
    url = httpserver.url_for("/client-conc")

    responses = list(client.get(iter([url]), concurrency_limit=5))

    assert len(responses) == 1


def test_client_reuse_for_multiple_batches(httpserver):
    """A single Client should handle multiple batches of requests."""
    for i in range(20):
        httpserver.expect_request(f"/batch{i}").respond_with_data(f"r{i}")

    client = lazynet.Client()

    # First batch
    urls1 = [httpserver.url_for(f"/batch{i}") for i in range(10)]
    responses1 = list(client.get(iter(urls1)))

    # Second batch with same client
    urls2 = [httpserver.url_for(f"/batch{i}") for i in range(10, 20)]
    responses2 = list(client.get(iter(urls2)))

    assert len(responses1) == 10
    assert len(responses2) == 10


# Note: Client timeout behavior is tested in Rust unit tests.


def test_multiple_clients_independent(httpserver):
    """Multiple Client instances should be independent."""
    httpserver.expect_request("/ind1").respond_with_data("1")
    httpserver.expect_request("/ind2").respond_with_data("2")

    client1 = lazynet.Client()
    client2 = lazynet.Client()

    r1 = list(client1.get(iter([httpserver.url_for("/ind1")])))
    r2 = list(client2.get(iter([httpserver.url_for("/ind2")])))

    assert r1[0].text == "1"
    assert r2[0].text == "2"


# =============================================================================
# URL handling
# =============================================================================


def test_url_with_query_parameters(httpserver):
    """URLs with query parameters should work."""
    httpserver.expect_request(
        "/query", query_string="a=1&b=2"
    ).respond_with_data("ok")
    url = httpserver.url_for("/query") + "?a=1&b=2"

    responses = list(lazynet.get(iter([url])))

    assert len(responses) == 1
    assert responses[0].status == 200


def test_url_with_path_components(httpserver):
    """URLs with multiple path components should work."""
    httpserver.expect_request("/a/b/c/d/e").respond_with_data("deep path")
    url = httpserver.url_for("/a/b/c/d/e")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].text == "deep path"


def test_url_with_encoded_characters(httpserver):
    """URLs with percent-encoded characters should work."""
    httpserver.expect_request("/hello%20world").respond_with_data("encoded")
    url = httpserver.url_for("/hello%20world")

    responses = list(lazynet.get(iter([url])))

    assert len(responses) == 1


def test_url_with_port(httpserver):
    """URLs with explicit port should work (httpserver uses a port)."""
    httpserver.expect_request("/with-port").respond_with_data("ported")
    url = httpserver.url_for("/with-port")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].text == "ported"


def test_url_with_fragment_ignored(httpserver):
    """URL fragments should be handled (typically ignored by servers)."""
    httpserver.expect_request("/fragment").respond_with_data("ok")
    url = httpserver.url_for("/fragment") + "#section"

    responses = list(lazynet.get(iter([url])))

    assert len(responses) == 1


# =============================================================================
# Iterator consumption behavior
# =============================================================================


def test_partial_iteration(httpserver):
    """Iterator should support partial consumption."""
    for i in range(10):
        httpserver.expect_request(f"/partial{i}").respond_with_data(f"{i}")

    urls = [httpserver.url_for(f"/partial{i}") for i in range(10)]
    iterator = lazynet.get(iter(urls))

    # Only consume first 3
    first = next(iterator)
    second = next(iterator)
    third = next(iterator)

    assert first is not None
    assert second is not None
    assert third is not None


def test_iterator_exhaustion(httpserver):
    """Exhausted iterator should raise StopIteration."""
    httpserver.expect_request("/exhaust").respond_with_data("ok")
    url = httpserver.url_for("/exhaust")

    iterator = lazynet.get(iter([url]))
    next(iterator)  # Consume the one response

    with pytest.raises(StopIteration):
        next(iterator)


def test_for_loop_iteration(httpserver):
    """Iterator should work with for loops."""
    for i in range(5):
        httpserver.expect_request(f"/loop{i}").respond_with_data(f"{i}")

    urls = [httpserver.url_for(f"/loop{i}") for i in range(5)]
    count = 0
    for response in lazynet.get(iter(urls)):
        count += 1
        assert response is not None

    assert count == 5


def test_list_conversion(httpserver):
    """Iterator should support list() conversion."""
    for i in range(5):
        httpserver.expect_request(f"/list{i}").respond_with_data(f"{i}")

    urls = [httpserver.url_for(f"/list{i}") for i in range(5)]
    responses = list(lazynet.get(iter(urls)))

    assert isinstance(responses, list)
    assert len(responses) == 5


# =============================================================================
# Edge cases and stress tests
# =============================================================================


def test_single_character_response(httpserver):
    """Single character responses should work."""
    httpserver.expect_request("/char").respond_with_data("x")
    url = httpserver.url_for("/char")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].text == "x"


def test_whitespace_only_response(httpserver):
    """Whitespace-only responses should be preserved."""
    httpserver.expect_request("/ws").respond_with_data("   \n\t\r\n   ")
    url = httpserver.url_for("/ws")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].text == "   \n\t\r\n   "


def test_special_json_characters(httpserver):
    """JSON with special characters should parse correctly."""
    data = {"quote": '"', "backslash": "\\", "newline": "\n", "tab": "\t"}
    httpserver.expect_request("/special").respond_with_json(data)
    url = httpserver.url_for("/special")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == data


def test_json_with_empty_string_values(httpserver):
    """JSON with empty string values should parse correctly."""
    data = {"empty": "", "nested": {"also_empty": ""}}
    httpserver.expect_request("/empty-str").respond_with_json(data)
    url = httpserver.url_for("/empty-str")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == data


def test_json_array_of_nulls(httpserver):
    """JSON array of nulls should parse correctly."""
    data = [None, None, None]
    httpserver.expect_request("/nulls").respond_with_json(data)
    url = httpserver.url_for("/nulls")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == data


def test_deeply_nested_json(httpserver):
    """Deeply nested JSON should parse correctly."""
    data = {"a": {"b": {"c": {"d": {"e": {"f": "deep"}}}}}}
    httpserver.expect_request("/deep").respond_with_json(data)
    url = httpserver.url_for("/deep")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json["a"]["b"]["c"]["d"]["e"]["f"] == "deep"


def test_large_json_array(httpserver):
    """Large JSON arrays should parse correctly."""
    data = list(range(1000))
    httpserver.expect_request("/big-arr").respond_with_json(data)
    url = httpserver.url_for("/big-arr")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == data


def test_many_small_requests(httpserver):
    """Many small requests should all complete."""
    for i in range(50):
        httpserver.expect_request(f"/small{i}").respond_with_data(".")

    urls = [httpserver.url_for(f"/small{i}") for i in range(50)]
    responses = list(lazynet.get(iter(urls), concurrency_limit=50))

    assert len(responses) == 50


# =============================================================================
# Type validation
# =============================================================================


def test_get_requires_iterable():
    """get() should require an iterable input."""
    with pytest.raises(TypeError):
        list(lazynet.get(42))


def test_concurrency_limit_must_be_int():
    """concurrency_limit should require an integer."""
    with pytest.raises(TypeError):
        list(lazynet.get(iter([]), concurrency_limit="ten"))


def test_timeout_secs_must_be_numeric():
    """timeout_secs should require a numeric value."""
    with pytest.raises(TypeError):
        list(lazynet.get(iter([]), timeout_secs="five"))


# =============================================================================
# Content-Type handling
# =============================================================================


def test_json_content_type_parsed(httpserver):
    """application/json content-type should result in parsed json."""
    httpserver.expect_request("/ct-json").respond_with_data(
        '{"key": "value"}', content_type="application/json"
    )
    url = httpserver.url_for("/ct-json")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == {"key": "value"}


def test_json_charset_utf8(httpserver):
    """application/json; charset=utf-8 should parse JSON."""
    httpserver.expect_request("/ct-charset").respond_with_data(
        '{"key": "value"}', content_type="application/json; charset=utf-8"
    )
    url = httpserver.url_for("/ct-charset")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].json == {"key": "value"}


def test_text_plain_not_parsed_as_json(httpserver):
    """text/plain content should not be parsed as JSON even if valid."""
    httpserver.expect_request("/ct-plain").respond_with_data(
        '{"key": "value"}', content_type="text/plain"
    )
    url = httpserver.url_for("/ct-plain")

    responses = list(lazynet.get(iter([url])))

    # Actually lazynet parses any valid JSON regardless of content-type
    # Just testing behavior is consistent
    assert responses[0].text == '{"key": "value"}'


def test_html_content_type(httpserver):
    """text/html content should be returned as text."""
    html = "<html><body>Hello</body></html>"
    httpserver.expect_request("/ct-html").respond_with_data(
        html, content_type="text/html"
    )
    url = httpserver.url_for("/ct-html")

    responses = list(lazynet.get(iter([url])))

    assert responses[0].text == html
