import sys

# import httpstream
from httpstream.httpstream import streamer

import time

NUM_URLS = 3000


def urls_gen():
    for _ in range(NUM_URLS):
        yield 'http://ip.jsontest.com/'


if __name__ == '__main__':
    # httpstream.httpstream.streamer()
    print(sys.path)
    start = time.time()
    # responses = httpstream.streamer(urls_gen(), concurrency_limit=1000)
    # for r in responses:
    #     print(r)
        # pass
    end = time.time()
    print()
    elapsed_time = end - start
    print("Time elapsed:", elapsed_time)
    print("Rate:", NUM_URLS / elapsed_time)
    print("Ending main")
