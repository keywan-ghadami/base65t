#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""The block seam, checked across implementations.

    cargo run --release --example large_sample -- in.bin in.b65
    python3 python/test_large.py in.bin in.b65

The published vectors stop below a kilobyte, so the block rule of §9.2.1 is
outside them -- and that is where the mistake this test exists for lived: a
block whose last base64 run leaves a partial quantum is continued by the next
block's run, and the seam decodes to what neither block meant. One
implementation encodes, the other decodes; if they disagree about a boundary,
this says so.
"""

import hashlib
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import base65t  # noqa: E402


def main(argv) -> int:
    if len(argv) != 2:
        print(__doc__)
        return 2
    data = pathlib.Path(argv[0]).read_bytes()
    stream = pathlib.Path(argv[1]).read_bytes()

    try:
        got = base65t.decode(stream, "U")
    except base65t.Base65tError as e:
        print(f"FAIL the other implementation's stream does not decode: {e.code}")
        return 1
    if got.bytes != data:
        print(f"FAIL decoded {len(got.bytes)} bytes, expected {len(data)}")
        return 1
    if got.padding_seen or got.framing_seen != "plain":
        print(f"FAIL unexpected {got.padding_seen=} {got.framing_seen=}")
        return 1
    if len(stream) > -(-4 * len(data) // 3):
        print(f"FAIL {len(stream)} chars is longer than base64 would be")
        return 1

    blocks = -(-len(data) // base65t.BLOCK_BYTES)
    print(
        f"{len(data)} bytes over {blocks} blocks, "
        f"sha256 {hashlib.sha256(data).hexdigest()[:16]}: "
        f"{len(stream)} chars decode back exactly, and are no longer than base64"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
