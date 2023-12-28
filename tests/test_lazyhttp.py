import asyncio
from unittest.mock import AsyncMock, patch

import pytest
from aiohttp import ClientSession, ClientResponse

from lazyhttp import lazyhttp as _lazyhttp
import lazyhttp

REQUEST = 'url'
DATA = b'data'

RESPONSE_ATTRS = {
    'request': REQUEST,
    'status': 200,
    'reason': 'ok',
    'text.return_value': '',
    'json.return_value': {}
}

expected_response = lazyhttp.Response(
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
        result = await _lazyhttp._send(client, REQUEST)

    mock_get.assert_called_once_with(client, REQUEST)
    assert result == expected_response


@pytest.mark.parametrize("input_list, expected_length", [([], 0), ([1], 1), ([1, 2], 2)])
def test_should_return_expected_length_for_response(input_list, expected_length):
    with patch('aiohttp.ClientSession.get', autospec=True) as mock_get:
        mock_get.return_value = get_response_mock()
        assert len(list(lazyhttp.get(input_list))) == expected_length
        assert mock_get.call_count == expected_length


@pytest.mark.parametrize("input_list, expected_length", [([], 0), ([1], 1), ([1, 2], 2)])
def test_should_return_expected_length_for_chained_response(input_list, expected_length):
    with patch('aiohttp.ClientSession.get', autospec=True) as mock_get:
        mock_get.return_value = get_response_mock()
        responses = lazyhttp.get(input_list)
        chained_requests = (r.request for r in responses)
        responses2 = lazyhttp.get(chained_requests)
        assert len(list(responses2)) == expected_length
        assert mock_get.call_count == expected_length * 2
