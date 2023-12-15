FROM python:3.12

RUN apt update  && apt install -y \
    make \
    default-jre \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --upgrade pip \
    && pip install poetry

RUN mkdir /app
RUN mkdir /app/data
WORKDIR /app

COPY Makefile Makefile
COPY pyproject.toml pyproject.toml
COPY poetry.lock poetry.lock
RUN make install

COPY httpstream httpstream

WORKDIR /app/httpstream
