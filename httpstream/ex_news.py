import os
import httpstream
import time

NUM_URLS = 1000


def urls_gen():
    for i in range(NUM_URLS):
        yield f'https://hacker-news.firebaseio.com/v0/item/{i}.json'


if __name__ == '__main__':
    tmp_file = 'news.jsonl.tmp'
    success_file = 'news.jsonl'

    start = time.time()
    responses = httpstream.streamer(urls_gen(), concurrency_limit=1000)
    with open(tmp_file, 'w') as outf:
        for r in responses:
            outf.write(str({
                'index_id': r.request,
                'status': r.status,
                'reason': r.reason,
                'text': r.text,
            }) + '\n')
    end = time.time()
    print()
    elapsed_time = end - start
    print("Time elapsed:", elapsed_time)
    print("Rate:", NUM_URLS / elapsed_time)

    os.rename(tmp_file, success_file)

    print("Ending main")
