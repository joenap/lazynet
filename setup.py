#!/usr/bin/env python

import os
import sys

try:
    from setuptools import setup
except ImportError:
    from distutils.core import setup


if sys.argv[-1] == 'publish':
    os.system('python setup.py sdist upload')
    sys.exit()

readme = open('README.md').read()
doclink = """
Documentation
-------------

The full documentation is at http://httpstream.rtfd.org."""
history = open('HISTORY.rst').read().replace('.. :changelog:', '')

requirements = [
    'aiohttp<4',
    'asyncio',
    'async-timeout',
    'cchardet',
    'aiodns<2',
    'pycares<3',
    'tornado<6',
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
    'pex',
    'pytest-runner',
    'pytest',
    'tox',
    'twine',
    'bumpversion',
]

setup(
    name='httpstream',
    version='0.1.0',
    description='Map HTTP requests over a generator',
    long_description=readme + '\n\n' + doclink + '\n\n' + history,
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
