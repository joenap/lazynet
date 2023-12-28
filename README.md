# lazyhttp

Are you lazy? Do you need to make lots of HTTP requests?

Well, look no further. lazyhttp is for you!

lazyhttp performs lazy-evaluated, asynchronous HTTP requests. It is ideal for making many HTTP requests for data at rest, fast and efficiently in a functional manner.

It uses `asyncio` and `aiohttp` libraries, using coroutines under the hood. It also hides this complexity from you, keeping you in a synchronous mindset.

## Usage

Suppose you are lazily reading IDs:

```
def lazy_ids():
    for i in range(10):
        yield i
```

You create a url using each individual ID:

```
def my_url(id):
    return f'http://my.domain/object/{id}'
```

Now you can make the HTTP request lazily using Python generators:

```
import lazyhttp

urls = (my_url(id) for id in lazy_ids())
responses = lazyhttp.get(urls) # responses is a generator

for response in responses: # nothing is evaluated until this loop
    print(response)
```


## Performance
lazyhttp can currently achieve a rate of about 1300 requests per second.

## History

2015: The original concept was explored [here](https://stackoverflow.com/questions/31869593/yielding-a-value-from-a-coroutine-in-python-a-k-a-convert-callback-to-generato)

2019: Alpha status. lazyhttp was developed over 3 days in a hackathon as an experiment and proof of concept. It is not used in any production system today.

2023: Beta Status. Updated to use current asyncio interface. Removed Tornado as a dependency, which was only used for the thread policy.
