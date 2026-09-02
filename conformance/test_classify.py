#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Conformance point 4 of §16: the two implementations decide §9.6 alike.

The entropy of §9.6 is not observable in a test vector shorter than 4096
bytes, and it decides which bytes the encoder writes. Two implementations that
disagree about it produce different output for the same input while both pass
the vector set -- which is exactly the failure the vector set cannot catch.

So it is checked directly, on inputs chosen to sit on both sides of the
threshold and, deliberately, very close to it.

    python3 conformance/test_classify.py
"""

import os
import random
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import reference as ref  # noqa: E402

RUST = os.path.join(HERE, "..", "rust")


def rust_classify(data: bytes) -> str:
    out = subprocess.run(
        ["cargo", "run", "--release", "--quiet", "--example", "entropy"],
        cwd=RUST, input=data, stdout=subprocess.PIPE, check=True,
    ).stdout.decode()
    return out.split()[0].lower()


def samples():
    rng = random.Random(0x65D)
    yield "gzip header", b"\x1f\x8b\x08\x00" + bytes(rng.randrange(256) for _ in range(8000))
    yield "zstd header", b"\x28\xb5\x2f\xfd" + b"a" * 8000
    yield "short text", b"alice.jones"
    yield "short random", bytes(rng.randrange(256) for _ in range(4095))
    yield "flat text", b"a" * 8000
    yield "uniform random", bytes(rng.randrange(256) for _ in range(8000))
    yield "english-ish", (b"the quick brown fox jumps over the lazy dog. " * 200)[:8000]
    yield "base64 text", (b"YWxpY2Uuam9uZXNzZXNzaW9u" * 400)[:8000]
    # Near the threshold from both sides: an alphabet of k symbols has entropy
    # log2(k), so k = 168 gives 7.39 bits and k = 174 gives 7.44.
    for k in (160, 165, 168, 170, 172, 174, 180, 200, 256):
        pool = bytes(range(k))
        yield f"{k} symbols", bytes(pool[rng.randrange(k)] for _ in range(8000))


def main() -> int:
    bad = 0
    for name, data in samples():
        theirs = rust_classify(data)
        ours = ref.classify(data)
        h = ref.entropy_millibits(data[:ref.SAMPLE_BYTES]) if len(data) >= ref.SAMPLE_BYTES else None
        mark = "ok" if theirs == ours else "DISAGREE"
        if theirs != ours:
            bad += 1
        print(f"{mark:9} {name:<18} {len(data):>6} B  rust={theirs:<6} python={ours:<6} "
              f"H={'-' if h is None else h}")
    if bad:
        print(f"\n{bad} disagreement(s): §9.6 is not implemented the same way twice")
        return 1
    print("\nthe two implementations decide §9.6 alike")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
