from aiohttp import ClientSession, ClientResponse
from asynctest import CoroutineMock, patch

from httpstream import httpstream

REQUEST = 'url'
DATA = b'data'

RESPONSE = httpstream.Response(
    request=REQUEST,
    status=200,
    reason='ok',
    text='',
    json={},
)


@patch('aiohttp.ClientSession.get', autospec=True)
async def test_send_should_call_client_get_with_request(mock_get, event_loop):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(status=200, spec=ClientResponse)

    async with ClientSession(loop=event_loop) as client:
        await httpstream.send(client, REQUEST)

    mock_get.assert_called_once_with(client, REQUEST)

# @patch('aiohttp.ClientSession.post')
# async def test_should_return_response_on_200(mock_post, event_loop):
#     mock_post.return_value.__aenter__.return_value = CoroutineMock(status=200, spec=ClientResponse)
#
#     async with ClientSession(loop=event_loop) as session:
#         await httpstream.post2(session, URL, DATA)
#
#     mock_post.assert_called_once_with(URL, data=DATA)

# @patch('aiohttp.ClientSession.post')
# async def test_should_print_error_when_not_200(mock_post, event_loop):
#     mock_post.return_value.__aenter__.return_value = CoroutineMock(status=404, spec=ClientResponse)
#     mock_post.return_value.__aenter__.return_value.read = CoroutineMock(return_value={'message': 'a message'})
#
#     async with ClientSession(loop=event_loop) as session:
#         await httpstream.post2(session, URL, DATA)
#
#     mock_post.assert_called_once_with(URL, data=DATA)
#     # Assert that something was done with the error
