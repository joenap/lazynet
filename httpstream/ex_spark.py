import os
import httpstream
import time

from pyspark.sql import SparkSession
from pyspark.sql.types import StructType, StructField, StringType
from pyspark.sql.functions import lit


def make_url(index: int) -> str:
    return f'https://hacker-news.firebaseio.com/v0/item/{index}.json'


def make_web_request(row_range):
    urls = (make_url(i) for i in range(row_range['start'], row_range['end'] + 1))
    print(row_range)
    responses = httpstream.streamer(urls, concurrency_limit=1000)
    for response in responses:
        yield response.text


def main():
    start = time.time()
    spark = SparkSession.builder.appName("WebRequestJob").getOrCreate()

    num_urls = 39000000
    group_size = 100000
    indices = range(0, num_urls, group_size)

    df = spark.createDataFrame([(i, i + group_size - 1,) for i in indices], ["start", "end"])
    responses = df.rdd.flatMap(make_web_request).map(lambda x: (x,)).toDF()
    responses.write.mode("overwrite").text('/app/data/news_data')

    end = time.time()
    print()
    elapsed_time = end - start
    print("Time elapsed:", elapsed_time)
    print("Rate:", num_urls / elapsed_time)


if __name__ == '__main__':
    main()
