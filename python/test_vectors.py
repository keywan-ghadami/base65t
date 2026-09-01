#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Conformance point 3 of §16, as far as one repository can carry it.

`docs/vectors.json` is what the Rust implementation writes. This checks it
against an implementation written from the specification instead, sharing no
code with it: every vector must encode to exactly those bytes and decode back
to exactly that input.

    python3 python/test_vectors.py
"""

import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import base65t  # noqa: E402

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
        for profile in v["profiles"]:
            got = base65t.PRESETS[v["preset"]](data, profile)
            if got != want:
                print(f"FAIL {v['name']} ({profile}) encode")
                print(f"     want {want!r}")
                print(f"     got  {got!r}")
                failed += 1
                continue
            back = base65t.decode(want, profile).bytes
            if back != data:
                print(f"FAIL {v['name']} ({profile}) decode: {back!r} != {data!r}")
                failed += 1
                continue
            checked += 1
    failed += errors()
    print(f"{checked} vector/profile pairs agree, {failed} disagree")
    return 1 if failed else 0


def errors() -> int:
    """The twelve codes of §10.4, on the vectors of §15.

    A second implementation that agrees on every valid stream and disagrees on
    what is invalid has not agreed about the format.
    """
    cases = [
        (b"abc~", "U", "decode", "E_TRAILING_TILDE"),
        (b"~AAAA", "U", "decode_plain", "E_RESERVED_LEN"),
        (b"~_A", "U", "decode", "E_TRUNCATED"),
        (b"~Ca b", "U", "decode", "E_PROFILE"),
        (b"abcde", "U", "decode", "E_ALIGN"),
        (b"YWxpY2V", "U", "decode", "E_NONZERO_TAIL"),
        (b"~~ab", "U", "decode", "E_CHARSET"),
        (b"YWxp==", "U", "decode", "E_PADDING"),
        (b"PDw_Pz8+Pg", "U", "decode", "E_MIXED_ALPHABET"),
        (b"PDw/Pz8+Pg", "U", "decode_url_strict", "E_NON_URL_ALPHABET"),
        (b"~AAAC~A", "U", "decode", "E_FRAME_RULE"),
        (b"YWxpY2U", "U", "decode_framed", "E_FRAME_SYNC"),
        (b"~AAAIYWxpY2U=", "U", "decode", "E_CHARSET"),      # TV15
        (b"~Da=b=", "T", "decode", "E_PADDING"),             # TV10
        (b"~Aabc", "U", "decode", "E_TRUNCATED"),            # TV11
    ]
    bad = 0
    for stream, profile, entry, want in cases:
        fn = getattr(base65t, entry)
        try:
            fn(stream, profile)
            print(f"FAIL {stream!r}: expected {want}, got a value")
            bad += 1
        except base65t.Base65tError as e:
            if e.code != want:
                print(f"FAIL {stream!r}: expected {want}, got {e.code}")
                bad += 1
    # And the ones that must be accepted (TV9b, TV10, TV15).
    for stream, profile, expect in [
        (b"~AAAI~Cx~~Cyz", "U", b"x~yz"),
        (b"~Ea=b=", "T", b"a=b="),
        (b"YWxpY2U=", "U", b"alice"),
        (b"~AAAHYWxpY2U", "U", b"alice"),
    ]:
        got = base65t.decode(stream, profile).bytes
        if got != expect:
            print(f"FAIL {stream!r}: {got!r} != {expect!r}")
            bad += 1
    print(f"{len(cases)} error codes and 4 acceptances checked, {bad} wrong")
    return bad


if __name__ == "__main__":
    raise SystemExit(main())
