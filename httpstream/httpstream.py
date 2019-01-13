from queue import Queue
from collections import namedtuple

import asyncio
import aiohttp
import itertools
import threading


Response = namedtuple('Response', ['status', 'reason', 'text', 'json'])


STOP_SENTINEL = {}


def grouper(n, iterable):
    ''' Yields successive lists of size n from iterable'''
    it = iter(iterable)
    while True:
        chunk = tuple(itertools.islice(it, n))
        if not chunk:
            return
        yield chunk


async def send(client, request):
    ''' Handles a single request '''
    async with client.get(request) as response:
        return Response(
            status=response.status,
            reason=response.reason,
            text=await response.text(),
            json=await response.json(),
        )


async def send_chunk(client, requests):
    ''' Handles a chunk of requests asynchronously '''
    tasks = (asyncio.ensure_future(send(client, r)) for r in requests)
    return await asyncio.gather(*tasks)


async def send_stream(requests, async_queue, concurrency_limit):
    ''' Handles a stream of requests and pushes responses to a queue '''
    async with aiohttp.ClientSession() as client:
        for event_chunk in grouper(concurrency_limit, requests):
            responses = await send_chunk(client, event_chunk)
            for response in responses:
                await async_queue.put(response)
        await async_queue.put(STOP_SENTINEL)


async def receive_stream(sync_queue, async_queue):
    ''' Receives asynchronous responses and pushes them to a standard queue '''
    while True:
        response = await async_queue.get()
        sync_queue.put(response)
        if response is STOP_SENTINEL:
            break


def response_generator(sync_queue):
    ''' Wrap a standard queue with a generator '''
    while True:
        response = sync_queue.get()
        if response is STOP_SENTINEL:
            raise StopIteration
        yield response


def worker(loop, pending_tasks):
    loop.run_until_complete(asyncio.gather(*pending_tasks))
    loop.close()


def streamer(requests, concurrency_limit=1000):
    '''
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
    '''
    async_queue = asyncio.Queue(concurrency_limit)
    sync_queue = Queue(concurrency_limit)

    loop = asyncio.get_event_loop()
    loop.create_task(receive_stream(sync_queue, async_queue))
    loop.create_task(send_stream(requests, async_queue, concurrency_limit))

    pending_tasks = asyncio.Task.all_tasks()

    threading.Thread(name='worker', target=worker, args=(loop, pending_tasks)).start()
    return response_generator(sync_queue)


if __name__ == '__main__':
    urls = [
        'https://postman-echo.com/get?foo1=bar1&foo2=bar2',
        'https://postman-echo.com/get?foo3=bar3&foo4=bar4'
    ]
    responses = streamer(urls)
    for r in responses:
        print(r.status, r.reason, r.json)
        print()
