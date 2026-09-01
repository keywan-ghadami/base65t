# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Base65t v0.2, written from ``docs/spec-v0.2.de.md``.

This is the second implementation §16.3 asks for, and it is deliberately not a
translation of the Rust one: it was written from the specification, it uses a
plain quadratic dynamic programme instead of the sliding windows of §9.2, and
it shares no code, no tables and no structure with it. Where the two disagree,
one of them has misread the document -- which is the entire point of asking for
two.

What it is not: written by somebody else. That gap stays open and is named in
FINDINGS.md.

Reference, not production. Readability before speed everywhere.
"""

from __future__ import annotations

ALPHABET = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
TILDE = 0x7E
MAX_LITERAL = 4158        # §6.1: 63 + 4095
MAX_FRAME_BODY = 262143   # §8.1: 18 bits
BLOCK_BYTES = 65535       # §9.2.1, a multiple of three
FRAME_BYTES = 65536       # §8.1

_VALUE = {}
for _i, _c in enumerate(ALPHABET):
    _VALUE[_c] = _i
_VALUE[ord("+")] = 62     # §5.2: permissive on both alphabets
_VALUE[ord("/")] = 63


class Base65tError(Exception):
    """One of the twelve codes of §10.4."""

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
    if profile == "B":
        return True
    raise ValueError("profile is U, T or B")


# --- decoder, §10 ---------------------------------------------------------


class Decoded:
    def __init__(self, data, alphabet_seen, padding_seen, framing_seen):
        self.bytes = data
        self.alphabet_seen = alphabet_seen      # "none" | "url" | "classic"
        self.padding_seen = padding_seen
        self.framing_seen = framing_seen        # "plain" | "framed"

    def __repr__(self):
        return f"Decoded({self.bytes!r}, {self.alphabet_seen}, {self.padding_seen}, {self.framing_seen})"


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

    def framed(self, stream: bytes) -> None:
        pos, n = 0, len(stream)
        while pos < n:
            if n - pos < 2 or stream[pos:pos + 2] != b"~A":
                raise Base65tError("E_FRAME_SYNC")
            if pos + 5 > n:
                raise Base65tError("E_TRUNCATED")
            a = self.read(stream[pos + 2])
            b = self.read(stream[pos + 3])
            c = self.read(stream[pos + 4])
            length = a << 12 | b << 6 | c
            if pos + 5 + length > n:
                raise Base65tError("E_TRUNCATED")
            body = stream[pos + 5:pos + 5 + length]
            if b"~A" in body:                       # F', §8.2 -- before decoding
                raise Base65tError("E_FRAME_RULE")
            self.plain(body, padding_allowed=False)  # §5.3: not the stream
            pos += 5 + length


def framing_of(stream: bytes) -> str:
    """Rule F, §5.6."""
    return "framed" if stream[:2] == b"~A" else "plain"


def _run(stream, profile, mode, strict_url=False) -> Decoded:
    d = _Decoder(profile, strict_url)
    (d.framed if mode == "framed" else (lambda s: d.plain(s, True)))(stream)
    return Decoded(bytes(d.out), d.alphabet, d.padding, mode)


def decode(stream: bytes, profile: str = "U") -> Decoded:
    return _run(stream, profile, framing_of(stream))


def decode_plain(stream: bytes, profile: str = "U") -> Decoded:
    return _run(stream, profile, "plain")


def decode_framed(stream: bytes, profile: str = "U") -> Decoded:
    return _run(stream, profile, "framed")


def decode_url_strict(stream: bytes, profile: str = "U") -> Decoded:
    return _run(stream, profile, framing_of(stream), strict_url=True)


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


def _segments(data: bytes, profile: str, lmin, framed: bool, passthrough: bool):
    """Length-optimal segmentation under §9.0, quadratic and obvious.

    Returns a list of ("B" | "L", start, end). `lmin` of None never takes a
    literal (`opaque`). Cost is (characters, -passthrough) when the preset asks
    for readability at equal length, which is the lexicographic minimum §9.3
    describes for `legible`.
    """
    n = len(data)
    INF = (float("inf"), 0)

    def literal_ok(i, j):
        if lmin is None or j - i > MAX_LITERAL or j - i < lmin:
            return False
        if any(not allows(profile, b) for b in data[i:j]):
            return False
        if framed:
            if data[j - 1] == TILDE:                       # F2
                return False
            if b"~A" in data[i:j]:                         # F1
                return False
        return True

    def lit_cost(m):
        return (m + _header(m), -m if passthrough else 0)

    def add(x, y):
        return (x[0] + y[0], x[1] + y[1])

    # r_l[j]: a base64 segment may open at j; r_b[j]: it may not (§4).
    r_l = [INF] * (n + 1)
    r_b = [INF] * (n + 1)
    for j in range(n, -1, -1):
        if j == n:
            r_l[j] = r_b[j] = (0, 0)
            continue
        best = INF
        for t in range(j + 1, min(n, j + MAX_LITERAL) + 1):
            if literal_ok(j, t) and r_l[t] != INF:
                best = min(best, add(lit_cost(t - j), r_l[t]))
        r_b[j] = best
        # A base64 segment covering [j, t), then a literal must follow.
        for t in range(j + 1, n + 1):
            if r_b[t] != INF or t == n:
                cand = add((-(-4 * (t - j) // 3), 0), r_b[t])
                best = min(best, cand)
        r_l[j] = best

    segs, pos, may_open = [], 0, True
    while pos < n:
        took = False
        if may_open:
            # `B` is the smallest symbol, so the run is as long as optimality
            # allows: pick the longest t reaching the optimum.
            best_t = None
            for t in range(pos + 1, n + 1):
                if r_b[t] == INF and t != n:
                    continue
                if add((-(-4 * (t - pos) // 3), 0), r_b[t]) == r_l[pos]:
                    best_t = t
            if best_t is not None:
                segs.append(("B", pos, best_t))
                pos, may_open, took = best_t, False, True
        if took:
            continue
        target = r_b[pos]
        ends = [
            t
            for t in range(pos + 1, min(n, pos + MAX_LITERAL) + 1)
            if literal_ok(pos, t) and r_l[t] != INF and add(lit_cost(t - pos), r_l[t]) == target
        ]
        assert ends, "the cost table promised a literal"
        # `B` beats `L` beats `S`: the first end where base64 can optimally
        # open wins, otherwise carry on to the longest optimal end.
        chosen = ends[-1]
        for t in ends:
            if t < n:
                opens = any(
                    add((-(-4 * (u - t) // 3), 0), r_b[u]) == r_l[t]
                    for u in range(t + 1, n + 1)
                    if r_b[u] != INF or u == n
                )
                if opens and add(lit_cost(t - pos), r_l[t]) == target and r_l[t] != r_b[t]:
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


def _encode_one(data, profile, lmin, framed=False, passthrough=False) -> bytes:
    return _emit(data, _segments(data, profile, lmin, framed, passthrough))


def encode_dense(data: bytes, profile: str = "U") -> bytes:
    """§9.2.1: independent blocks, so memory is constant."""
    if len(data) <= BLOCK_BYTES:
        return _encode_one(data, profile, 11)
    return b"".join(
        _encode_one(data[i:i + BLOCK_BYTES], profile, 11)
        for i in range(0, len(data), BLOCK_BYTES)
    )


def encode_legible(data: bytes, profile: str = "U") -> bytes:
    return _encode_one(data, profile, 1, passthrough=True)


def encode_canonical(data: bytes, profile: str = "U") -> bytes:
    return _encode_one(data, profile, 1)


def encode_opaque(data: bytes, profile: str = "U") -> bytes:
    return _encode_one(data, "U", None)


def encode_framed(data: bytes, profile: str = "U") -> bytes:
    out = bytearray()
    for i in range(0, len(data), FRAME_BYTES):
        body = _encode_one(data[i:i + FRAME_BYTES], profile, 11, framed=True)
        assert len(body) <= MAX_FRAME_BODY
        out += bytes((TILDE, ord("A"),
                      ALPHABET[(len(body) >> 12) & 63],
                      ALPHABET[(len(body) >> 6) & 63],
                      ALPHABET[len(body) & 63]))
        out += body
    return bytes(out)


def encode(data: bytes) -> bytes:
    """§9.3: no preset means `dense` and profile U."""
    return encode_dense(data, "U")


PRESETS = {
    "dense": encode_dense,
    "legible": encode_legible,
    "canonical": encode_canonical,
    "opaque": encode_opaque,
    "framed": encode_framed,
}
