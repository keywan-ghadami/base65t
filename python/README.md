# base66 (Python)

Python bindings for [Base66](https://github.com/keywan-ghadami/base65t): a
compiled extension over the Rust reference implementation, so what Python runs
is byte for byte what a Rust caller gets.

```python
import base66

stream = base66.encode(b"alice.jones")
assert stream == b"~~alice.jones"
assert base66.decode(stream).bytes == b"alice.jones"
```

`encode` takes bytes and returns bytes. There is no mode to pick and no preset
to name: a caller who has to choose between a dense encoder and a fast one has
to know what those words mean before encoding a byte, and a caller who is
unsure writes base64. The encoder decides for itself (specification section
9.6). There is no profile either: the output is always the 66 characters of
RFC 3986 *unreserved*, exported as `base66.ALPHABET`. The name counts 65 —
base64url's 64 symbols plus the marker `~` — but a container is checked
against all 66. The 66th is `.`, and it is what makes the passthrough worth
having: a block passes through only if every one of its bytes is admitted, so
without `.` no hostname, filename or dotted identifier would ever stand in the
clear.

The return is `bytes` and not `str` although the output is printable ASCII:
section 3 calls the output an octet stream, and the return type says what the
format guarantees. `.decode("ascii")` is free where it is wanted.

`encode_base64url` is the way out of the format rather than a mode of it, for a
caller carrying a secret who wants no part of it left in the clear. Its output
is ordinary unpadded base64url.

`decode` reports what the stream chose — alphabet variant and padding — because
those come out of the stream and not out of a parameter. Where the alphabet is
fixed, `decode_url_strict` fixes it.
