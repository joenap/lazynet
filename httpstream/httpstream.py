from queue import Queue
from collections import namedtuple
from tornado.platform.asyncio import AnyThreadEventLoopPolicy

import asyncio
# import aiohttp
# import itertools
# import threading

# # todo check out aiodns resolver
# # https://stackoverflow.com/a/45169094/1102470
#
# Response = namedtuple('Response', ['request', 'status', 'reason', 'text'])
#
# # Used to flush the response queue and stop the iterator.
# STOP_SENTINEL = {}
#
#
# def grouper(n, iterable):
#     """ Yields successive lists of size n from iterable """
#     it = iter(iterable)
#     while True:
#         chunk = tuple(itertools.islice(it, n))
#         if not chunk:
#             return
#         yield chunk
#
#
# async def send(client, request):
#     """ Handles a single request """
#     async with client.get(request) as response:
#         return Response(
#             request=request,
#             status=response.status,
#             reason=response.reason,
#             text=await response.text(),
#             json=await response.json(),
#         )
#
#
# async def send_chunk(client, requests):
#     """ Handles a chunk of requests asynchronously """
#     tasks = (asyncio.ensure_future(send(client, r)) for r in requests)
#     return await asyncio.gather(*tasks)
#
#
# async def send_stream(requests, sync_queue, concurrency_limit):
#     """ Handles a stream of requests and pushes responses to a queue """
#     async with aiohttp.ClientSession() as client:
#         # Gather responses in chunks of size concurrency_limit
#         for request_chunk in grouper(concurrency_limit, requests):
#             for response in await send_chunk(client, request_chunk):
#                 sync_queue.put(response)
#         sync_queue.put(STOP_SENTINEL)
#
#
# def response_generator(sync_queue):
#     """ Wrap a standard queue with a generator """
#     while True:
#         response = sync_queue.get()
#         if response is STOP_SENTINEL:
#             return
#         yield response
#
#
# def worker(loop, pending_tasks):
#     loop.run_until_complete(asyncio.gather(*pending_tasks))
#     loop.close()
#
#
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

    asyncio.set_event_loop_policy(AnyThreadEventLoopPolicy())
    # loop = asyncio.get_event_loop()
    # loop.create_task(send_stream(requests, sync_queue, concurrency_limit))
    # pending_tasks = asyncio.all_tasks()
    # threading.Thread(name='worker', target=worker, args=(loop, pending_tasks)).start()
    # return response_generator(sync_queue)
    return [1,2,3]


NUM_URLS = 1000


def urls_gen():
    for _ in range(NUM_URLS):
        yield 'http://localhost:8080/'


if __name__ == '__main__':
    print("Running main")
    responses = streamer(urls_gen())
    for r in responses:
        pass
    print()
    # print("Time elapsed:", timer.elapsed)
    # print("Human time:", timer.elapsed_human)
    # print("Rate:", NUM_URLS / timer.elapsed)
    print("Ending main")
