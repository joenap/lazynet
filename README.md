# lazynet

Are you lazy? Do you need to make lots of HTTP requests?

Well, look no further. lazynet is for you!

lazynet performs lazy-evaluated, asynchronous HTTP requests. It is ideal for making many HTTP requests for data at rest, fast and efficiently in a functional manner.

It uses the `asyncio` and `aiohttp` libraries, using coroutines under the hood. It also hides this complexity from you, keeping you in a synchronous mindset.

## Usage

```python
import lazynet

# This is a generator
urls = (f'https://my.domain/object/{id}' for id in range(10))

# responses is also a generator
responses = lazynet.get(urls)

# nothing is evaluated until this loop
for response in responses:
    print(response)
```

## Performance
lazynet can currently achieve a rate of about 1300 requests per second.

## History

2019: Alpha status. lazynet was developed as an experiment and proof of concept.

2023: Beta Status. Updated to use current asyncio interface.
