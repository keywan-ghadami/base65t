# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Base66: an output alphabet of 66 characters, RFC 3986 unreserved.

Everything in this package comes from `base66.base66`, the extension module
built from Rust -- see `src/lib.rs`. The package exists so that the type stubs
and the PEP 561 marker have somewhere a type checker recognises; it adds no
behaviour of its own and re-exports exactly what the extension lists in its
`__all__`.
"""

from . import base66 as _extension
from .base66 import *  # noqa: F401,F403

__doc__ = _extension.__doc__ or __doc__
__all__ = list(_extension.__all__)
