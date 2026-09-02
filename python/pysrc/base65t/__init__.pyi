from typing import Final, Union

_Bytes = Union[bytes, bytearray, str]

class Decoded:
    bytes: bytes
    alphabet_seen: str
    padding_seen: bool
    framing_seen: str

class Base65tDecodeError(ValueError):
    code: str

def encode(data: _Bytes, /, preset: str = ..., profile: str = ...) -> bytes: ...
def decode(stream: _Bytes, /, profile: str = ...) -> Decoded: ...
def decode_plain(stream: _Bytes, /, profile: str = ...) -> Decoded: ...
def decode_framed(stream: _Bytes, /, profile: str = ...) -> Decoded: ...
def decode_url_strict(stream: _Bytes, /, profile: str = ...) -> Decoded: ...

PRESETS: Final[list[str]]
PROFILES: Final[list[str]]
MAX_LITERAL: Final[int]
MAX_FRAME_BODY: Final[int]
BLOCK_BYTES: Final[int]
FRAME_BYTES: Final[int]
SPEC_VERSION: Final[str]
__version__: Final[str]
