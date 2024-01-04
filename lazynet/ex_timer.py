import time

import lazynet

NUM_URLS = 3


def urls_gen():
    for _ in range(NUM_URLS):
        yield 'http://ip.jsontest.com/'


if __name__ == '__main__':
    start = time.time()
    responses = lazynet.get(urls_gen(), concurrency_limit=1000)
    for r in responses:
        print(r)
    end = time.time()
    elapsed_time = end - start
    print("Time elapsed:", elapsed_time)
    print("Rate:", NUM_URLS / elapsed_time)
    print("Ending main")
