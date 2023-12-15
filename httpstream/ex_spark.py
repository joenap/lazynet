import os
import httpstream
import time
from pyspark.sql import SparkSession
import requests

from pyspark.sql.types import StructType, StructField, StringType
from pyspark.sql.functions import lit

import uuid


def make_url(index: int) -> str:
    return f'https://hacker-news.firebaseio.com/v0/item/{index}.json'


def make_web_request(row_range):
    urls = (make_url(i) for i in range(row_range['start'], row_range['end'] + 1))
    print(row_range)
    responses = httpstream.streamer(urls, concurrency_limit=1000)
    for response in responses:
        # if response is not None:
        yield response.text


def main():
    start = time.time()
    spark = SparkSession.builder.appName("WebRequestJob").getOrCreate()

    num_urls = 1000000
    group_size = 100000
    indices = range(0, num_urls, group_size)

    for i in indices:
        print(i, i + group_size - 1)

    df = spark.createDataFrame([(i, i + group_size - 1,) for i in indices], ["start", "end"])

    print(f'Partitions: {df.rdd.getNumPartitions()}')

    responses = df.rdd.flatMap(make_web_request)
    responses = responses.map(lambda x: (x,))
    # results = responses.collect()
    # print(results)

    result_df = responses.toDF()
    result_df.write.mode("overwrite").text('news_data')

    end = time.time()
    print()
    elapsed_time = end - start
    print("Time elapsed:", elapsed_time)
    print("Rate:", num_urls / elapsed_time)


if __name__ == '__main__':
    main()
