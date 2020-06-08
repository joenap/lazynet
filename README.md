# HTTPStream
HTTPStream is a library that performs lazy-evaluated, asynchronous HTTP requests. It is ideal for making many HTTP requests for data at rest, fast and efficiently in a functional manner.

It is built using Python's new `asyncio` and `aiohttp` libraries, using coroutines under the hood.

## Usage

Suppose you are lazily reading IDs from a source, to be used in HTTP requests:

```
def lazy_ids():
    for id in range(10):
        yield id
```

You create a url using each individual ID:

```
def my_url(id):
    return f'http://localhost/object/{id}'
```

Now you can make the HTTP lazily using Python generators:

```
import httpstream

urls = (my_url(id) for id in lazy_ids())
responses = httpstream.streamer(urls) # responses is a generator

for response in responses: # nothing is evaluated until this loop
    print(response.code)
```


## Performance
HTTPStream can currently achieve a rate of about 1300 requests per second.

## Status

HTTPStream was developed over 3 days as an experiment and proof of concept. It is considered pre-alpha status. It is not used in any production system today.