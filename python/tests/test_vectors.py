# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""The published vectors, through the binding.

This is not a second implementation and cannot discharge §16.3 -- it calls the
same Rust the vectors were written from, so it can only agree. What it does
check is that every entry point reaches the argument it is
supposed to, which is the one thing a binding can get wrong on its own.
"""

import json
import pathlib

import pytest

import base65t

VECTORS = pathlib.Path(__file__).resolve().parents[2] / "docs" / "vectors.json"

KINDS = {
    "encode": base65t.encode,
    "base64url": base65t.encode_base64url,
}


def load():
    if not VECTORS.exists():
        pytest.skip("docs/vectors.json is not in this tree")
    return json.loads(VECTORS.read_text())["vectors"]


def test_every_vector_encodes_and_decodes():
    checked = 0
    for v in load():
        data = bytes.fromhex(v["input"])
        want = bytes.fromhex(v["stream"])
        assert KINDS[v["kind"]](data) == want, v["name"]
        assert base65t.decode(want) == data, v["name"]
        checked += 1
    assert checked >= 100, f"only {checked} vectors"
