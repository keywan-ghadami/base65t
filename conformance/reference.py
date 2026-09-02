# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Base65t v0.4, written from ``docs/spec-v0.4.de.md``.

This is the second implementation §16.3 asks for, and it is deliberately not a
translation of the Rust one: it was written from the specification, it walks
the literal edges of §9.2 one at a time instead of keeping the two sliding
windows, its forward pass enumerates rather than reconstructs, and it shares no
code, no tables and no structure with it. Where the two disagree, one of them
has misread the document -- which is the entire point of asking for two.

The one place it does follow §9.2 closely is the three-state recurrence for the
base64 edges, and not for elegance: taken as "try every end" it is quadratic,
and then a 64 KiB window -- the unit §9.2.1 makes normative -- is out of reach
here, so `test_large.py` could not test a window seam at all.

What it is not: written by somebody else. That gap stays open and is named in
``docs/history/FINDINGS.md``.

Reference, not production. Readability before speed everywhere.
"""

from __future__ import annotations

ALPHABET = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
TILDE = 0x7E
MAX_LITERAL = 4158        # §6.1: 63 + 4095
WINDOW_BYTES = 65536      # §9.2.1
SAMPLE_BYTES = 4096              # §9.6
ENTROPY_LIMIT_MILLIBITS = 7400   # §9.6

_VALUE = {}
for _i, _c in enumerate(ALPHABET):
    _VALUE[_c] = _i
_VALUE[ord("+")] = 62     # §5.2: permissive on both alphabets
_VALUE[ord("/")] = 63


class Base65tError(Exception):
    """One of the ten codes of §10.4."""

    def __init__(self, code: str):
        super().__init__(code)
        self.code = code


def _unreserved(b: int) -> bool:
    return (
        0x41 <= b <= 0x5A
        or 0x61 <= b <= 0x7A
        or 0x30 <= b <= 0x39
        or b in (0x2D, 0x2E, 0x5F, 0x7E)
    )


def allows(profile: str, b: int) -> bool:
    """§7: what a literal payload may carry."""
    if profile == "U":
        return _unreserved(b)
    if profile == "T":
        return 0x20 <= b <= 0x7E and b not in (0x22, 0x5C)
    raise ValueError("profile is U or T")


# --- decoder, §10 ---------------------------------------------------------


class Decoded:
    def __init__(self, data, alphabet_seen, padding_seen):
        self.bytes = data
        self.alphabet_seen = alphabet_seen      # "none" | "url" | "classic"
        self.padding_seen = padding_seen

    def __repr__(self):
        return f"Decoded({self.bytes!r}, {self.alphabet_seen}, {self.padding_seen})"


class _Decoder:
    def __init__(self, profile: str, strict_url: bool):
        self.profile = profile
        self.strict_url = strict_url
        self.alphabet = "none"
        self.padding = False
        self.out = bytearray()

    def note(self, c: int) -> None:
        """Rule A (§5.4), and the strict variant of §5.5 in the same place."""
        if c in (ord("+"), ord("/")):
            if self.strict_url:
                raise Base65tError("E_NON_URL_ALPHABET")
            if self.alphabet == "url":
                raise Base65tError("E_MIXED_ALPHABET")
            self.alphabet = "classic"
        elif c in (ord("-"), ord("_")):
            if self.alphabet == "classic":
                raise Base65tError("E_MIXED_ALPHABET")
            self.alphabet = "url"

    def read(self, c: int) -> int:
        """One alphabet position: check first (§10.1 trap 1), then read."""
        if c not in _VALUE:
            raise Base65tError("E_CHARSET")
        self.note(c)
        return _VALUE[c]

    def plain(self, stream: bytes, padding_allowed: bool) -> None:
        pos, n = 0, len(stream)
        while pos < n:
            if stream[pos] == TILDE:
                if pos + 2 > n:
                    raise Base65tError("E_TRAILING_TILDE")
                l1 = self.read(stream[pos + 1])
                if l1 == 0:
                    raise Base65tError("E_RESERVED_LEN")
                if l1 == 63:
                    if pos + 4 > n:
                        raise Base65tError("E_TRUNCATED")
                    hi = self.read(stream[pos + 2])
                    lo = self.read(stream[pos + 3])
                    length = 63 + (hi << 6 | lo)
                    pos += 4
                else:
                    length = l1
                    pos += 2
                if pos + length > n:
                    raise Base65tError("E_TRUNCATED")
                payload = stream[pos:pos + length]
                for b in payload:
                    if not allows(self.profile, b):
                        raise Base65tError("E_PROFILE")
                self.out += payload      # no Rule A here -- §5.4, TV7
                pos += length
            else:
                end = pos
                while end < n and stream[end] != TILDE:
                    end += 1
                self.segment(stream[pos:end], end == n and padding_allowed)
                pos = end

    def segment(self, seg: bytes, at_stream_end: bool) -> None:
        k = 0
        if at_stream_end:
            while k < 2 and len(seg) - k > 0 and seg[len(seg) - 1 - k] == ord("="):
                k += 1
        m = len(seg) - k
        if not (k == 0 or (k == 1 and m % 4 == 3) or (k == 2 and m % 4 == 2)):
            raise Base65tError("E_PADDING")
        if k:
            self.padding = True
        if m % 4 == 1:
            raise Base65tError("E_ALIGN")
        acc = bits = 0
        for c in seg[:m]:
            acc = acc << 6 | self.read(c)
            bits += 6
            if bits == 24:
                self.out += bytes(((acc >> 16) & 255, (acc >> 8) & 255, acc & 255))
                acc = bits = 0
        if bits == 12:
            if acc & 0x0F:
                raise Base65tError("E_NONZERO_TAIL")
            self.out.append((acc >> 4) & 255)
        elif bits == 18:
            if acc & 0x03:
                raise Base65tError("E_NONZERO_TAIL")
            self.out += bytes(((acc >> 10) & 255, (acc >> 2) & 255))


def _run(stream, profile, strict_url=False) -> Decoded:
    d = _Decoder(profile, strict_url)
    d.plain(stream, padding_allowed=True)
    return Decoded(bytes(d.out), d.alphabet, d.padding)


def decode(stream: bytes, profile: str = "U") -> Decoded:
    """§10.2. One entry point: v0.2 had three plus Rule F in front of them,
    and Rule F went with the framed mode (§5.6)."""
    return _run(stream, profile)


def decode_url_strict(stream: bytes, profile: str = "U") -> Decoded:
    return _run(stream, profile, strict_url=True)


# --- encoder, §9 ----------------------------------------------------------


def _header(m: int) -> int:
    return 2 if m <= 62 else 4


def _b64(chunk: bytes) -> bytes:
    out = bytearray()
    for i in range(0, len(chunk), 3):
        g = chunk[i:i + 3]
        n = g[0] << 16 | (g[1] << 8 if len(g) > 1 else 0) | (g[2] if len(g) > 2 else 0)
        for k in range(len(g) + 1):
            out.append(ALPHABET[(n >> (18 - 6 * k)) & 63])
    return bytes(out)


def _segments(data: bytes, profile: str, lmin):
    """Length-optimal segmentation under §9.2, written out rather than tuned.

    Returns a list of ("B" | "L", start, end). `lmin` of None never takes a
    literal, which is `encode_base64url`. The cost is the character count and
    nothing else.
    """
    n = len(data)
    INF = float("inf")

    def literal_ok(i, j):
        if lmin is None or j - i > MAX_LITERAL or j - i < lmin:
            return False
        if any(not allows(profile, b) for b in data[i:j]):
            return False
        return True

    def lit_cost(m):
        return m + _header(m)

    # r_l[j]: a base64 segment may open at j; r_b[j]: it may not (§4).
    # g[j][p]: cheapest finish from inside a base64 segment with p bytes in the
    # open quantum. §9.2 writes the base64 edges as a three-state recurrence
    # for exactly this reason -- taken literally as "try every end" it is
    # quadratic, and then a 64 KiB window is out of reach for this
    # implementation.
    r_l = [INF] * (n + 1)
    r_b = [INF] * (n + 1)
    g = [[INF, INF, INF] for _ in range(n + 1)]
    r_l[n] = r_b[n] = 0
    g[n] = [0, 0, 0]
    for j in range(n - 1, -1, -1):
        best = INF
        # Literal edges. The run has to be admissible over its whole length,
        # so the walk stops at the first byte the profile rejects rather than
        # asking again for every end past it.
        if lmin is not None:
            t = j + 1
            while t <= n and t - j <= MAX_LITERAL and allows(profile, data[t - 1]):
                if t - j >= lmin and r_l[t] != INF:
                    best = min(best, lit_cost(t - j) + r_l[t])
                t += 1
        r_b[j] = best
        # Base64 edges: +2 characters opening a quantum, +1 for each further
        # byte, and a segment may end at any p.
        g[j] = [
            min(r_b[j], 2 + g[j + 1][1]),
            min(r_b[j], 1 + g[j + 1][2]),
            min(r_b[j], 1 + g[j + 1][0]),
        ]
        r_l[j] = g[j][0]

    def opens_b64(j):
        """Can a base64 segment start at j and still be length-optimal?"""
        return j < n and r_l[j] == 2 + g[j + 1][1]

    segs, pos, may_open = [], 0, True
    while pos < n:
        if may_open and opens_b64(pos):
            # `B` is the smallest symbol, so the run is as long as optimality
            # allows.
            t, p = pos + 1, 1
            while t < n and g[t][p] == (2 if p == 0 else 1) + g[t + 1][(p + 1) % 3]:
                p = (p + 1) % 3
                t += 1
            segs.append(("B", pos, t))
            pos, may_open = t, False
            continue
        target = r_b[pos]
        ends = []
        t = pos + 1
        while t <= n and t - pos <= MAX_LITERAL and allows(profile, data[t - 1]):
            if t - pos >= lmin and r_l[t] != INF and lit_cost(t - pos) + r_l[t] == target:
                ends.append(t)
            t += 1
        assert ends, "the cost table promised a literal"
        # `B` beats `L` beats `S`: the first end where base64 can optimally
        # open wins, otherwise carry on to the longest optimal end.
        chosen = ends[-1]
        for t in ends:
            if opens_b64(t) and lit_cost(t - pos) + (2 + g[t + 1][1]) == target:
                chosen = t
                break
        segs.append(("L", pos, chosen))
        pos, may_open = chosen, True
    return segs


def _emit(data: bytes, segs) -> bytes:
    out = bytearray()
    for kind, i, j in segs:
        if kind == "B":
            out += _b64(data[i:j])
        else:
            m = j - i
            out.append(TILDE)
            if m <= 62:
                out.append(ALPHABET[m])
            else:
                v = m - 63
                out += bytes((ALPHABET[63], ALPHABET[(v >> 6) & 63], ALPHABET[v & 63]))
            out += data[i:j]
    return bytes(out)


def _encode_one(data, profile, lmin) -> bytes:
    return _emit(data, _segments(data, profile, lmin))


# --- §9.6: one decision at the head of the stream --------------------------

MAGIC = (
    b"\x1f\x8b",              # gzip
    b"\x28\xb5\x2f\xfd",      # zstd
    b"\xfd7zXZ",              # xz
    b"BZh",                   # bzip2
    b"PK\x03\x04",            # zip
    b"\xff\xd8\xff",          # JPEG
    b"\x89PNG",               # PNG
    b"OggS",                  # Ogg
    b"\x1aE\xdf\xa3",         # Matroska / WebM
)


def _log2_millibits(a: int, b: int) -> int:
    """1000 * log2(a / b) for a >= b > 0, by integer bisection.

    Integer on purpose (§9.6): this decides which bytes the encoder writes, and
    two implementations have to agree on it exactly. Written the way the
    specification describes it rather than the way the Rust does -- the whole
    part by halving, the fraction by squaring -- so that agreement is evidence
    and not a shared bug.
    """
    whole = 0
    x = a
    while x >= 2 * b:
        x //= 2
        whole += 1
    frac = 0
    num, den = x, b
    for bit in range(16):
        while num > (1 << 31) or den > (1 << 31):
            num >>= 1
            den >>= 1
        num *= num
        den *= den
        if num >= 2 * den:
            num //= 2
            frac |= 1 << (15 - bit)
    return whole * 1000 + (frac * 1000) // (1 << 16)


def entropy_millibits(data: bytes) -> int:
    """Shannon entropy in thousandths of a bit per byte (§9.6)."""
    n = len(data)
    if n == 0:
        return 0
    count = [0] * 256
    for b in data:
        count[b] += 1
    total = 0
    for k in count:
        if k:
            total += k * _log2_millibits(n, k)
    return total // n


def classify(data: bytes) -> str:
    """§9.6: "base64" writes without looking, "exact" runs the programme."""
    for m in MAGIC:
        if data.startswith(m):
            return "base64"
    if len(data) < SAMPLE_BYTES:
        return "exact"
    if entropy_millibits(data[:SAMPLE_BYTES]) > ENTROPY_LIMIT_MILLIBITS:
        return "base64"
    return "exact"


# --- §9: the encoder -------------------------------------------------------


def encode_base64url(data: bytes, profile: str = "U") -> bytes:
    """§9.3, §14: base64url and nothing else, whatever the input looks like."""
    return _b64(data)


def encode_with(data: bytes, profile: str = "U") -> bytes:
    """§9.0, §9.2.1, §9.6: the encoding.

    The windows of §9.2.1 are part of the definition, not of the
    implementation: the programme runs per window at absolute offsets, and
    adjacent base64 segments across a seam are one segment (§4). The Rust does
    this too and the two have to reach the same bytes on inputs past 64 KiB,
    which `conformance/test_large.py` is for.
    """
    if not data:
        return b""
    if classify(data) == "base64":
        return _b64(data)
    segs = []
    for start in range(0, len(data), WINDOW_BYTES):
        end = min(start + WINDOW_BYTES, len(data))
        for kind, i, j in _segments(data[start:end], profile, 1):
            i, j = start + i, start + j
            if segs and kind == "B" and segs[-1][0] == "B" and segs[-1][2] == i:
                segs[-1] = ("B", segs[-1][1], j)
            else:
                segs.append((kind, i, j))
    return _emit(data, segs)


def encode(data: bytes) -> bytes:
    """§9.3: no parameter means profile U."""
    return encode_with(data, "U")


KINDS = {
    "encode": encode_with,
    "base64url": encode_base64url,
}
