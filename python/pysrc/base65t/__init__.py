# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Base65t: Base64URL plus a 65th character introducing raw runs.

Everything in this package comes from `base65t.base65t`, the extension module
built from Rust -- see `src/lib.rs`. The package exists so that the type stubs
and the PEP 561 marker have somewhere a type checker recognises; it adds no
behaviour of its own and re-exports exactly what the extension lists in its
`__all__`.
"""

from . import base65t as _extension
from .base65t import *  # noqa: F401,F403

__doc__ = _extension.__doc__ or __doc__
__all__ = list(_extension.__all__)
