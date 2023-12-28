from queue import Queue
# from collections import namedtuple

import asyncio
import aiohttp
import itertools
import threading

import time


# todo check out aiodns resolver
# https://stackoverflow.com/a/45169094/1102470

# Response = namedtuple(
#     'Response',
#     ['request', 'status', 'reason', 'text', 'json']
# )


class Response:
    def __init__(self, request, status, reason, text, json):
        self.request = request
        self.status = status
        self.reason = reason
        self.text = text
        self.json = json

    def __str__(self):
        return f"Response(request={self.request}, status={self.status}, reason={self.reason}, text={self.text}, json={self.json})"

    def __eq__(self, other):
        return isinstance(other, Response) and self.__dict__ == other.__dict__


# Used to flush the response queue and stop the iterator.
STOP_SENTINEL = {}


def grouper(n, iterable):
    """ Yields successive lists of size n from iterable """
    it = iter(iterable)
    while True:
        chunk = tuple(itertools.islice(it, n))
        if not chunk:
            return
        yield chunk


async def send(client, request, max_retries=3, retry_interval=1):
    """ Handles a single request """
    for attempt in range(max_retries + 1):
        try:
            async with client.get(request) as response:
                return Response(
                    request=request,
                    status=response.status,
                    reason=response.reason,
                    text=await response.text(),
                    json=await response.json(),
                )
        except aiohttp.ClientError as e:
            if attempt < max_retries:
                print(f"Error: {e} when requesting {request}. Attempt #{attempt}")
                await asyncio.sleep(retry_interval)
            else:
                return None


async def send_chunk(client, requests):
    """ Handles a chunk of requests asynchronously """
    tasks = (asyncio.ensure_future(send(client, r)) for r in requests)
    return await asyncio.gather(*tasks)


async def send_stream(requests, sync_queue, concurrency_limit):
    """ Handles a stream of requests and pushes responses to a queue """
    async with aiohttp.ClientSession() as client:
        # Gather responses in chunks of size concurrency_limit
        for request_chunk in grouper(concurrency_limit, requests):
            for response in await send_chunk(client, request_chunk):
                sync_queue.put(response)
        sync_queue.put(STOP_SENTINEL)


def response_generator(sync_queue, thread):
    """ Wrap a standard queue with a generator """
    while True:
        response = sync_queue.get()
        if response is STOP_SENTINEL:
            thread.join()
            return
        yield response


def worker(requests, sync_queue, concurrency_limit):
    asyncio.run(send_stream(requests, sync_queue, concurrency_limit))


def streamer(requests, concurrency_limit=1000):
    """
    Returns a generator of HTTP responses for the given generator of HTTP requests.

    Results are returned in the same order as received.

    The response generator will block while waiting for the HTTP requests to
        be completed asynchronously. Callers may iterate over the results as
        quickly as they arrive using a standard generator. This enables
        lazy-evaluated HTTP streams.

    Example:
        urls = (f"http://my.company/{i}" for i in range(10))
        responses = streamer(urls)
        data = (my_transform_function(r) for r in responses)
    """
    sync_queue = Queue(concurrency_limit)

    print('Creating thread')
    t = threading.Thread(
        name='worker',
        target=worker,
        args=(requests, sync_queue, concurrency_limit)
    )
    t.start()
    print(f'Thread started: {t}')
    return response_generator(sync_queue, t)
