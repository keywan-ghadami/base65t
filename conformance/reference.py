# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Base65t v0.4, written from ``docs/spec-v0.4.de.md``.

This is the second implementation §16.3 asks for, and it is deliberately not a
translation of the Rust one: it was written from the specification, it builds
the mask by hand rather than through a packed lookup, and it shares no code,
no tables and no structure with it. Where the two disagree, one of them has
misread the document -- which is the entire point of asking for two.

What it is not: written by somebody else. That gap stays open and is named in
``docs/history/FINDINGS.md``.

Reference, not production. Readability before speed everywhere.
"""

from __future__ import annotations

ALPHABET = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
TILDE = 0x7E
BLOCK_BYTES = 48          # §4
MASK_CHARS = BLOCK_BYTES // 6

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
    """§7: what a raw byte may be."""
    if profile == "U":
        return _unreserved(b)
    if profile == "T":
        return 0x20 <= b <= 0x7E and b not in (0x22, 0x5C)
    raise ValueError("profile is U or T")


def _b64_len(n: int) -> int:
    return -(-4 * n // 3)


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

    def raw(self, payload: bytes) -> None:
        for b in payload:
            if not allows(self.profile, b):
                raise Base65tError("E_PROFILE")
        self.out += payload          # no Rule A here -- §5.4, TV7

    def blocks(self, stream: bytes) -> None:
        pos, n = 0, len(stream)
        while pos < n:
            if stream[pos] != TILDE:
                # A base64 block: 64 characters, or what is left.
                end = min(pos + 4 * BLOCK_BYTES // 3, n)
                self.out += self.base64(stream[pos:end], end == n)
                pos = end
            elif pos + 1 == n:
                raise Base65tError("E_TRAILING_TILDE")
            elif stream[pos + 1] == TILDE:
                # A raw block: 48 bytes, or what is left.
                end = min(pos + 2 + BLOCK_BYTES, n)
                self.raw(stream[pos + 2:end])
                pos = end
            else:
                pos = self.mask_block(stream, pos + 1)

    def mask_block(self, stream: bytes, pos: int) -> int:
        """§6: the mask form, from its first mask character."""
        n = len(stream)
        if pos + MASK_CHARS > n:
            raise Base65tError("E_TRUNCATED")
        bits = []
        for j in range(MASK_CHARS):
            v = self.read(stream[pos + j])
            # First byte in the top bit, so the mask reads like the bytes.
            bits.extend((v >> (5 - t)) & 1 for t in range(6))
        pos += MASK_CHARS
        admitted = sum(bits)
        if pos + admitted > n:
            raise Base65tError("E_TRUNCATED")
        clear = stream[pos:pos + admitted]
        for b in clear:
            if not allows(self.profile, b):
                raise Base65tError("E_PROFILE")
        pos += admitted
        full = _b64_len(BLOCK_BYTES - admitted)
        at_end = n - pos <= full
        end = n if at_end else pos + full
        rest = self.base64(stream[pos:end], at_end)
        m = admitted + len(rest)
        if m > BLOCK_BYTES or any(bits[m:]):
            raise Base65tError("E_MASK")
        clear_it, rest_it = iter(clear), iter(rest)
        for i in range(m):
            self.out.append(next(clear_it) if bits[i] else next(rest_it))
        return end

    def base64(self, seg: bytes, at_stream_end: bool) -> bytes:
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
        out = bytearray()
        acc = bits = 0
        for c in seg[:m]:
            acc = acc << 6 | self.read(c)
            bits += 6
            if bits == 24:
                out += bytes(((acc >> 16) & 255, (acc >> 8) & 255, acc & 255))
                acc = bits = 0
        if bits == 12:
            if acc & 0x0F:
                raise Base65tError("E_NONZERO_TAIL")
            out.append((acc >> 4) & 255)
        elif bits == 18:
            if acc & 0x03:
                raise Base65tError("E_NONZERO_TAIL")
            out += bytes(((acc >> 10) & 255, (acc >> 2) & 255))
        return bytes(out)


def _run(stream, profile, strict_url=False) -> Decoded:
    d = _Decoder(profile, strict_url)
    d.blocks(stream)
    return Decoded(bytes(d.out), d.alphabet, d.padding)


def decode(stream: bytes, profile: str = "U") -> Decoded:
    """§10.2. The profile is the only parameter."""
    return _run(stream, profile)


def decode_url_strict(stream: bytes, profile: str = "U") -> Decoded:
    return _run(stream, profile, strict_url=True)


# --- encoder, §9 ----------------------------------------------------------


def _b64(chunk: bytes) -> bytes:
    out = bytearray()
    for i in range(0, len(chunk), 3):
        g = chunk[i:i + 3]
        n = g[0] << 16 | (g[1] << 8 if len(g) > 1 else 0) | (g[2] if len(g) > 2 else 0)
        for k in range(len(g) + 1):
            out.append(ALPHABET[(n >> (18 - 6 * k)) & 63])
    return bytes(out)


def _forms(block: bytes, profile: str):
    """§9.0: the three forms a block may take, each with its length.

    Listed in the order the tie-break prefers them *least*: base64 first, so
    that a later form of equal length replaces it.
    """
    bits = [allows(profile, b) for b in block]
    m, admitted = len(block), sum(bits)
    forms = [("base64", _b64_len(m))]
    forms.append(("mask", 1 + MASK_CHARS + admitted + _b64_len(m - admitted)))
    if admitted == m:
        forms.append(("raw", m + 2))
    return bits, forms


def _encode_block(block: bytes, profile: str) -> bytes:
    bits, forms = _forms(block, profile)
    best = None
    for form, length in forms:
        if best is None or length <= best[1]:
            best = (form, length)
    form = best[0]
    if form == "base64":
        return _b64(block)
    if form == "raw":
        return b"~~" + block
    out = bytearray(b"~")
    for j in range(MASK_CHARS):
        v = 0
        for t in range(6):
            i = 6 * j + t
            v = v << 1 | (1 if i < len(block) and bits[i] else 0)
        out.append(ALPHABET[v])
    out += bytes(b for b, ok in zip(block, bits) if ok)
    out += _b64(bytes(b for b, ok in zip(block, bits) if not ok))
    return bytes(out)


def encode_base64url(data: bytes, profile: str = "U") -> bytes:
    """§9.3, §14: base64url and nothing else, whatever the input looks like."""
    return _b64(data)


def encode_with(data: bytes, profile: str = "U") -> bytes:
    """§9: block by block, and nothing carries over between blocks."""
    out = bytearray()
    for i in range(0, len(data), BLOCK_BYTES):
        out += _encode_block(data[i:i + BLOCK_BYTES], profile)
    return bytes(out)


def encode(data: bytes) -> bytes:
    """§9.3: no parameter means profile U."""
    return encode_with(data, "U")


KINDS = {
    "encode": encode_with,
    "base64url": encode_base64url,
}
