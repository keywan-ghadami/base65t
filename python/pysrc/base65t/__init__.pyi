from typing import Final, Union

_Bytes = Union[bytes, bytearray, str]

class Decoded:
    bytes: bytes
    alphabet_seen: str
    padding_seen: bool

class Base65tDecodeError(ValueError):
    code: str

def encode(data: _Bytes, /, profile: str = ...) -> bytes: ...
def encode_base64url(data: _Bytes, /, profile: str = ...) -> bytes: ...
def decode(stream: _Bytes, /, profile: str = ...) -> Decoded: ...
def decode_url_strict(stream: _Bytes, /, profile: str = ...) -> Decoded: ...

PROFILES: Final[list[str]]
BLOCK_BYTES: Final[int]
MASK_CHARS: Final[int]
SPEC_VERSION: Final[str]
__version__: Final[str]
