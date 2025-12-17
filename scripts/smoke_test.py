#!/usr/bin/env python3
"""Quick smoke test for lazynet."""

import sys
import lazynet

URL = "http://127.0.0.1:8080/"
NUM_REQUESTS = 100


def main():
    urls = (URL for _ in range(NUM_REQUESTS))
    responses = list(lazynet.get(urls))
    print(f"{len(responses)}/{NUM_REQUESTS} responses")
    return 0 if len(responses) == NUM_REQUESTS else 1


if __name__ == "__main__":
    sys.exit(main())
