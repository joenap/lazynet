FROM python:3.12

RUN pip install --upgrade pip \
    && pip install poetry

COPY httpstream httpstream
COPY Makefile Makefile
COPY pyproject.toml pyproject.toml
COPY poetry.lock poetry.lock
RUN make install

RUN pip install ipython
