# lazynet

Are you lazy? Do you need to make lots of HTTP requests?

Well, look no further. lazynet is for you!

lazynet performs lazy-evaluated, asynchronous HTTP requests. It is ideal for making many HTTP requests for data at rest, fast and efficiently in a functional manner.

It uses Rust's async runtime (tokio + reqwest) under the hood. It also hides this complexity from you, keeping you in a synchronous mindset.

## Usage

Suppose you are lazily reading IDs from a flat file, to be used in HTTP requests:

```python
def lazy_ids():
    with open('file.txt') as fin:
        for line in fin:
            yield line.strip()
```

You create a url using each individual ID:

```python
def my_url(id):
    return f'http://localhost/object/{id}'
```

Now you can make the HTTP request lazily using Python generators:

```python
import lazynet

urls = (my_url(id) for id in lazy_ids())
responses = lazynet.get(urls)  # responses is a generator

for response in responses:  # nothing is evaluated until this loop
    print(response.status)
```

## Performance

2015-2023: An initial implementation <=0.4.0 was written in pure python and achieved 1300 requests per second on a single core.

2024: Rewrote the core logic in Rust, getting ~7000 requests per second on a single core.

2025: Vibecoded to perfection, getting ~11k requests per second on a single core. Also added multi-core support,
tested on a 16 core AM5 cpu (32 HT cores), getting a whopping ~360k requests per second.

All performance tests are performed against a local nginx server. On Macos you will hit file limits
long before the performance bottleneck.

## History

2015: The original concept was explored [here](https://stackoverflow.com/questions/31869593/yielding-a-value-from-a-coroutine-in-python-a-k-a-convert-callback-to-generato)

2019: lazynet was revisited and developed further over 3 days in a hackathon as a proof of concept.

2023: Updated to use current asyncio interface. Removed Tornado as a dependency.

2024: Rewritten in Rust using PyO3 for maximum performance.

2025: Brought up to BETA standards and will be put into a production service.
