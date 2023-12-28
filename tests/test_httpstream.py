import asyncio
from unittest.mock import AsyncMock, patch

import pytest
from aiohttp import ClientSession, ClientResponse

from httpstream import httpstream
from httpstream.httpstream import Response

REQUEST = 'url'
DATA = b'data'

RESPONSE_ATTRS = {
    'request': REQUEST,
    'status': 200,
    'reason': 'ok',
    'text.return_value': '',
    'json.return_value': {}
}

expected_response = Response(
    request=REQUEST,
    status=RESPONSE_ATTRS['status'],
    reason=RESPONSE_ATTRS['reason'],
    text=RESPONSE_ATTRS['text.return_value'],
    json=RESPONSE_ATTRS['json.return_value'],
)


def get_response_mock():
    response_mock = AsyncMock(**RESPONSE_ATTRS, spec=ClientResponse)
    response_mock.__aenter__.return_value = response_mock
    return response_mock


@pytest.mark.asyncio
@patch('aiohttp.ClientSession.get', autospec=True)
async def test_send_should_call_client_get_with_request(mock_get):
    mock_get.return_value = get_response_mock()
    event_loop = asyncio.get_running_loop()

    async with ClientSession(loop=event_loop) as client:
        result = await httpstream.send(client, REQUEST)

    mock_get.assert_called_once_with(client, REQUEST)
    assert result == expected_response


@patch('aiohttp.ClientSession.get', autospec=True)
def test_should_return_0_for_0_response(mock_get):
    mock_get.return_value = get_response_mock()
    assert len(list(httpstream.streamer([]))) == 0


@patch('aiohttp.ClientSession.get', autospec=True)
def test_should_return_1_for_1_response(mock_get):
    mock_get.return_value = get_response_mock()
    assert len(list(httpstream.streamer(['url']))) == 1


@patch('aiohttp.ClientSession.get', autospec=True)
def test_should_return_0_for_0_when_chained(mock_get):
    mock_get.return_value = get_response_mock()
    responses = httpstream.streamer([])
    requests = (r.request for r in httpstream.streamer(responses))
    responses2 = httpstream.streamer(requests)
    assert len(list(responses2)) == 0


@patch('aiohttp.ClientSession.get', autospec=True)
def test_should_return_1_for_1_when_chained(mock_get):
    mock_get.return_value = get_response_mock()
    responses = httpstream.streamer(['url'])
    requests = (r.request for r in httpstream.streamer(responses))
    responses2 = httpstream.streamer(requests)
    assert len(list(responses2)) == 1
