# lazyhttp
lazyhttp is a library that performs lazy-evaluated, asynchronous HTTP requests. It is ideal for making many HTTP requests for data at rest, fast and efficiently in a functional manner.

It is built using Python's new `asyncio` and `aiohttp` libraries, using coroutines under the hood.

## Usage

Suppose you are lazily reading IDs from a flat file, to be used in HTTP requests:

```
def ids():
    with open('file.txt') as fin:
        for line in fin:
        yield line.strip()
```

You create a url using each individual ID:

```
def my_url(id):
    return f'http://localhost/object/{id}'
```

Now you can make the HTTP request lazily using Python generators:

```
import lazyhttp

urls = (my_url(id) for id in lazy_ids())
responses = lazyhttp.get(urls) # responses is a generator

for response in responses: # nothing is evaluated until this loop
    print(response.code)
```


## Performance
lazyhttp can currently achieve a rate of about 1300 requests per second.

## Status

lazyhttp was developed over 3 days as an experiment and proof of concept. It is considered pre-alpha status. It is not used in any production system today.
