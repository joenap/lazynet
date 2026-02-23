__author__ = 'Joe Nap'
__email__ = 'joenap@gmail.com'

from importlib.metadata import version as _version
__version__ = _version('lazynet')

from ._lazynet import get, Response, Client

__all__ = ['get', 'Response', 'Client']
