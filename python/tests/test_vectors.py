# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""The published vectors through the binding.

This does **not** check the format: the binding wraps the same Rust the
vectors were written from, so agreement here is agreement with itself. What it
does check is that every preset and profile name reaches the argument it is
supposed to reach — a binding that silently encoded everything as `dense`
would pass every test in test_bindings.py that does not compare bytes, and fail
here on the first `canonical` vector.

The check that means something about the format is ../conformance, which runs
an implementation written from the specification instead.
"""

import json
import pathlib

import pytest

import base65t

VECTORS = json.loads(
    (pathlib.Path(__file__).resolve().parents[2] / "docs" / "vectors.json").read_text()
)["vectors"]


@pytest.mark.parametrize("v", VECTORS, ids=lambda v: v["name"])
def test_vector(v):
    data = bytes.fromhex(v["input"])
    want = bytes.fromhex(v["stream"])
    for profile in v["profiles"]:
        assert base65t.encode(data, v["preset"], profile) == want
        assert base65t.decode(want, profile).bytes == data
