from aiohttp import ClientSession, ClientResponse
from asynctest import CoroutineMock, patch

from httpstream import httpstream

REQUEST = 'url'
DATA = b'data'

RESPONSE_ATTRS = {
    'request': REQUEST,
    'status': 200,
    'reason': 'ok',
    'text.return_value': '',
    'json.return_value': {}
}


@patch('aiohttp.ClientSession.get', autospec=True)
async def test_send_should_call_client_get_with_request(mock_get, event_loop):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(spec=ClientResponse)
    async with ClientSession(loop=event_loop) as client:
        await httpstream.send(client, REQUEST)
    mock_get.assert_called_once_with(client, REQUEST)


@patch('aiohttp.ClientSession.get', autospec=True)
async def test_response_should_have_request(mock_get, event_loop):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(**RESPONSE_ATTRS, spec=ClientResponse)
    async with ClientSession(loop=event_loop) as client:
        response = await httpstream.send(client, REQUEST)
    assert response.request is RESPONSE_ATTRS['request']


@patch('aiohttp.ClientSession.get', autospec=True)
async def test_response_should_have_status(mock_get, event_loop):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(**RESPONSE_ATTRS, spec=ClientResponse)
    async with ClientSession(loop=event_loop) as client:
        response = await httpstream.send(client, REQUEST)
    assert response.status == RESPONSE_ATTRS['status']


@patch('aiohttp.ClientSession.get', autospec=True)
async def test_response_should_have_reason(mock_get, event_loop):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(**RESPONSE_ATTRS, spec=ClientResponse)
    async with ClientSession(loop=event_loop) as client:
        response = await httpstream.send(client, REQUEST)
    assert response.reason == RESPONSE_ATTRS['reason']


@patch('aiohttp.ClientSession.get', autospec=True)
async def test_response_should_have_text(mock_get, event_loop):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(**RESPONSE_ATTRS, spec=ClientResponse)
    async with ClientSession(loop=event_loop) as client:
        response = await httpstream.send(client, REQUEST)
    assert response.text == RESPONSE_ATTRS['text.return_value']


@patch('aiohttp.ClientSession.get', autospec=True)
async def test_response_should_have_json(mock_get, event_loop):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(**RESPONSE_ATTRS, spec=ClientResponse)
    async with ClientSession(loop=event_loop) as client:
        response = await httpstream.send(client, REQUEST)
    assert response.json is RESPONSE_ATTRS['json.return_value']


@patch('aiohttp.ClientSession.get', autospec=True)
def test_should_return_0_for_0_response(mock_get):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(**RESPONSE_ATTRS, spec=ClientResponse)
    assert len(list(httpstream.streamer([]))) == 0


@patch('aiohttp.ClientSession.get', autospec=True)
def test_should_return_1_for_1_response(mock_get):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(**RESPONSE_ATTRS, spec=ClientResponse)
    assert len(list(httpstream.streamer(['url']))) == 1


@patch('aiohttp.ClientSession.get', autospec=True)
def test_should_return_1_for_1_response(mock_get):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(**RESPONSE_ATTRS, spec=ClientResponse)
    assert len(list(httpstream.streamer(['url']))) == 1
