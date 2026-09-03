# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Base65t v0.4, written from ``docs/spec-v0.4.md``.

This is the second implementation §16.3 asks for, and it is deliberately not a
translation of the Rust one: it was written from the specification, it tests
each byte of a block against a written-out character set rather than through
arithmetic over thirty-two bytes at a time, and it
shares no code, no tables and no structure with it. Where the two disagree, one of them has
misread the document -- which is the entire point of asking for two.

What it is not: written by somebody else. That gap stays open and is named in
``docs/history/FINDINGS.md``.

Reference, not production. Readability before speed everywhere.
"""

from __future__ import annotations

ALPHABET = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
TILDE = 0x7E
BLOCK_BYTES = 48          # §4
SAMPLE_BLOCKS = 64        # §9.6

_VALUE = {}
for _i, _c in enumerate(ALPHABET):
    _VALUE[_c] = _i
_VALUE[ord("+")] = 62     # §5.2: permissive on both alphabets
_VALUE[ord("/")] = 63


class Base65tError(Exception):
    """One of the nine codes of §10.4."""

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


def allows(b: int) -> bool:
    """§7: what a raw byte may be -- RFC 3986 unreserved, 66 characters."""
    return _unreserved(b)


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
    def __init__(self, strict_url: bool):
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
            if not allows(b):
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
            elif stream[pos + 1] in _VALUE:
                raise Base65tError("E_RESERVED")       # §17
            else:
                raise Base65tError("E_CHARSET")

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


def _run(stream, strict_url=False) -> Decoded:
    d = _Decoder(strict_url)
    d.blocks(stream)
    return Decoded(bytes(d.out), d.alphabet, d.padding)


def decode(stream: bytes) -> Decoded:
    """§10.2. A stream and nothing else (§0.3)."""
    return _run(stream)


def decode_url_strict(stream: bytes) -> Decoded:
    return _run(stream, strict_url=True)


# --- encoder, §9 ----------------------------------------------------------


def _b64(chunk: bytes) -> bytes:
    out = bytearray()
    for i in range(0, len(chunk), 3):
        g = chunk[i:i + 3]
        n = g[0] << 16 | (g[1] << 8 if len(g) > 1 else 0) | (g[2] if len(g) > 2 else 0)
        for k in range(len(g) + 1):
            out.append(ALPHABET[(n >> (18 - 6 * k)) & 63])
    return bytes(out)


def _encode_block(block: bytes) -> bytes:
    """§9.0: raw when every byte is admitted and raw is no longer than
    base64, which is four bytes and up; base64 otherwise."""
    m = len(block)
    if all(allows(b) for b in block) and m + 2 <= _b64_len(m):
        return b"~~" + block
    return _b64(block)


def encode_base64url(data: bytes) -> bytes:
    """§9.3, §14: base64url and nothing else, whatever the input looks like."""
    return _b64(data)


def _any_block_can_be_raw(data: bytes) -> bool:
    """§9.6: does any of the first SAMPLE_BLOCKS blocks stand raw?

    The encoder's own decision, sampled -- not a proxy for it. Written out
    here rather than reusing `_encode_block`, so that the two implementations
    can disagree if the document allows two readings.
    """
    for i in range(0, min(len(data), SAMPLE_BLOCKS * BLOCK_BYTES), BLOCK_BYTES):
        block = data[i:i + BLOCK_BYTES]
        if len(block) + 2 <= _b64_len(len(block)) and all(allows(b) for b in block):
            return True
    return False


def encode(data: bytes) -> bytes:
    """§9: block by block, and nothing carries over between blocks.

    Except the one thing that does: §9.6 asks the same question of the first
    sixty-four blocks, and where none of them can stand raw the whole stream
    is base64url and no block is asked about again.

    §9.3: there is no parameter. One alphabet, one call.
    """
    if not _any_block_can_be_raw(data):
        return _b64(data)
    out = bytearray()
    for i in range(0, len(data), BLOCK_BYTES):
        out += _encode_block(data[i:i + BLOCK_BYTES])
    return bytes(out)


KINDS = {
    "encode": encode,
    "base64url": encode_base64url,
}
