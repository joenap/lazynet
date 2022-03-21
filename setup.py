#!/usr/bin/env python

import os
import sys

try:
    from setuptools import setup
except ImportError:
    from distutils.core import setup


history = open('HISTORY.rst').read().replace('.. :changelog:', '')

requirements = [
    'aiohttp',
    # 'asyncio',
    'async-timeout<4',
    'cchardet',
    'aiodns',
    'pycares',
    'tornado',
]

extras = [
    'asynctest',
    'pytest-asyncio',
    'pytest-mock',
    'pytest-aiohttp',
    'aioresponses',

    'simplestopwatch',
    'coverage',
    'ipython',
    'flake8',
    'pprintpp',
    'pytest-runner',
    'pytest',
    'tox',
    'twine',
    'bumpversion',
]

description = 'Map HTTP requests over a generator'

setup(
    name='httpstream',
    version='0.1.0',
    description=description,
    long_description=description,
    author='Joe Nap',
    author_email='joenap@gmail.com',
    url='https://github.com/joenap/httpstream',
    packages=[
        'httpstream',
    ],
    package_dir={'httpstream': 'httpstream'},
    include_package_data=True,
    install_requires=requirements,
    license='MIT',
    zip_safe=False,
    keywords='httpstream',
    classifiers=[
        'Development Status :: 2 - Pre-Alpha',
        'Intended Audience :: Developers',
        'License :: OSI Approved :: MIT License',
        'Natural Language :: English',
        # 'Programming Language :: Python :: 2',
        # 'Programming Language :: Python :: 2.6',
        # 'Programming Language :: Python :: 2.7',
        # 'Programming Language :: Python :: 3',
        # 'Programming Language :: Python :: 3.3',
        # 'Programming Language :: Python :: Implementation :: PyPy',
    ],
)
