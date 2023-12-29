# lazyhttp

Are you lazy? Do you need to make lots of HTTP requests?

Well, look no further. lazyhttp is for you!

lazyhttp performs lazy-evaluated, asynchronous HTTP requests. It is ideal for making many HTTP requests for data at rest, fast and efficiently in a functional manner.

It uses `asyncio` and `aiohttp` libraries, using coroutines under the hood. It also hides this complexity from you, keeping you in a synchronous mindset.

## Usage

```
import lazyhttp

def my_url(id):
    return f'http://my.domain/object/{id}'

# This is a generator
urls = (my_url(id) for id in range(10))

# responses is also a generator
responses = lazyhttp.get(urls)

# nothing is evaluated until this loop
for response in responses:
    print(response)
```

## Performance
lazyhttp can currently achieve a rate of about 1300 requests per second.

## History

2019: Alpha status. lazyhttp was developed as an experiment and proof of concept.

2023: Beta Status. Updated to use current asyncio interface.
