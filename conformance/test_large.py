#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""A long stream, checked across implementations.

    cargo run --release --example large_sample -- in.bin in.b65
    python3 conformance/test_large.py in.bin in.b65

The published vectors stop below a kilobyte. Everything a segmentation mistake
needs is above that: many boundaries, and no vector watching them. One
implementation encodes the input, this one encodes it again and decodes what
the other wrote, and the three have to agree.

Blocks are independent, so there is no seam to watch any more; what a long
stream tests is the tail rule and the mask on every mix of bytes a generated
input produces -- some five thousand blocks of it.

The Python encoder is plain and slow, and a quarter of a megabyte is a few
seconds. That is enough.
"""

import hashlib
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import reference as base65t  # noqa: E402


def main(argv) -> int:
    if len(argv) != 2:
        print(__doc__)
        return 2
    data = pathlib.Path(argv[0]).read_bytes()
    stream = pathlib.Path(argv[1]).read_bytes()

    try:
        got = base65t.decode(stream)
    except base65t.Base65tError as e:
        print(f"FAIL the other implementation's stream does not decode: {e.code}")
        return 1
    if got != data:
        print(f"FAIL decoded {len(got)} bytes, expected {len(data)}")
        return 1
    if base65t.decode_detailed(stream).padding_seen:
        print("FAIL an encoder wrote padding, which §5.3 forbids")
        return 1
    if len(stream) > -(-4 * len(data) // 3):
        print(f"FAIL {len(stream)} chars is longer than base64 would be")
        return 1

    mine = base65t.encode(data)
    if mine != stream:
        at = next(i for i, (a, b) in enumerate(zip(mine, stream)) if a != b)
        print(f"FAIL the two encoders disagree at character {at}")
        return 1

    print(
        f"{len(data)} bytes, sha256 {hashlib.sha256(data).hexdigest()[:16]}: "
        f"both encoders write the same {len(stream)} chars, they decode back "
        f"exactly, and they are no longer than base64"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
