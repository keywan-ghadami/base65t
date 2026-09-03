#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Conformance point 3 of §16, as far as one repository can carry it.

`docs/vectors.json` is what the Rust implementation writes. This checks it
against an implementation written from the specification instead, sharing no
code with it: every vector must encode to exactly those bytes and decode back
to exactly that input.

    python3 conformance/test_vectors.py
"""

import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import reference as base65t  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent


def main() -> int:
    vectors = json.loads((ROOT / "docs" / "vectors.json").read_text())["vectors"]
    checked = failed = 0
    for v in vectors:
        data = bytes.fromhex(v["input"])
        want = bytes.fromhex(v["stream"])
        if "stream_ascii" in v and v["stream_ascii"].encode() != want:
            print(f"FAIL {v['name']}: stream_ascii disagrees with the hex")
            failed += 1
        got = base65t.KINDS[v["kind"]](data)
        if got != want:
            print(f"FAIL {v['name']} encode")
            print(f"     want {want!r}")
            print(f"     got  {got!r}")
            failed += 1
            continue
        back = base65t.decode(want)
        if back != data:
            print(f"FAIL {v['name']} decode: {back!r} != {data!r}")
            failed += 1
            continue
        checked += 1
    failed += errors()
    print(f"{checked} vectors agree, {failed} disagree")
    return 1 if failed else 0


def errors() -> int:
    """The nine codes of §10.4, on the vectors of §15.

    A second implementation that agrees on every valid stream and disagrees on
    what is invalid has not agreed about the format.
    """
    cases = [
        (b"YWJj" * 16 + b"~", "decode", "E_TRAILING_TILDE"),
        (b"~", "decode", "E_TRAILING_TILDE"),
        (b"~Aabc", "decode", "E_RESERVED"),
        (b"~7abc", "decode", "E_RESERVED"),
        (b"~~a b", "decode", "E_PROFILE"),
        (b"~~abcd=", "decode", "E_PROFILE"),                   # TV10
        (b"~~a+b/c-d_e", "decode", "E_PROFILE"),
        (b"abcde", "decode", "E_ALIGN"),
        (b"YWxpY2V", "decode", "E_NONZERO_TAIL"),
        (b"YW*j", "decode", "E_CHARSET"),
        (b"YW~x", "decode", "E_CHARSET"),
        (b"~=", "decode", "E_CHARSET"),
        (b"YWxp==", "decode", "E_PADDING"),
        (b"YWxpY2U==", "decode", "E_PADDING"),
        (b"PDw_Pz8+Pg", "decode", "E_MIXED_ALPHABET"),
        (b"PDw/Pz8+Pg", "decode_url_strict", "E_NON_URL_ALPHABET"),
    ]
    bad = 0
    for stream, entry, want in cases:
        fn = getattr(base65t, entry)
        try:
            fn(stream)
            print(f"FAIL {stream!r}: expected {want}, got a value")
            bad += 1
        except base65t.Base65tError as e:
            if e.code != want:
                print(f"FAIL {stream!r}: expected {want}, got {e.code}")
                bad += 1
    # And the ones that must be accepted (TV6, TV7, TV9, TV10).
    for stream, expect in [
        (b"~~abcd", b"abcd"),
        (b"YWxpY2U=", b"alice"),
        (b"YWxpY2Uu", b"alice."),
        (b"~~a-b_c-d_e", b"a-b_c-d_e"),   # TV7: raw bytes do not count
        (b"~~", b""),
    ]:
        got = base65t.decode(stream)
        if got != expect:
            print(f"FAIL {stream!r}: {got!r} != {expect!r}")
            bad += 1
    print(f"{len(cases)} error cases and 5 acceptances checked, {bad} wrong")
    return bad


if __name__ == "__main__":
    raise SystemExit(main())
