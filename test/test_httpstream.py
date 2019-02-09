from aiohttp import ClientSession, ClientResponse
from asynctest import CoroutineMock, patch
from httpstream import httpstream

URL = 'url'
DATA = b'data'


@patch('aiohttp.ClientSession.get')
async def test_should_return_response_on_200(mock_get, event_loop):
    mock_get.return_value.__aenter__.return_value = CoroutineMock(status=200, spec=ClientResponse)

    async with ClientSession(loop=event_loop) as client:
        await httpstream.send(client, URL)

    mock_get.assert_called_once_with(URL)

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
