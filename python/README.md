# base65t (Python)

Python bindings for [Base65t](https://github.com/keywan-ghadami/base65t): a
compiled extension over the Rust reference implementation, so what Python runs
is byte for byte what a Rust caller gets.

```python
import base65t

stream = base65t.encode(b"alice.jones")        # profile U
assert stream == b"~Lalice.jones"
assert base65t.decode(stream).bytes == b"alice.jones"
```

`encode` takes bytes and returns bytes. There is no mode to pick and no preset
to name: a caller who has to choose between a dense encoder and a fast one has
to know what those words mean before encoding a byte, and a caller who is
unsure writes base64. The encoder decides for itself (specification section
9.6). `profile` is not such a choice — it is a statement about the container
the stream has to survive, and `"U"` is the default.

The return is `bytes` and not `str` although both profiles produce printable
ASCII: section 3 calls the output an octet stream, and the return type says
what the format guarantees. `.decode("ascii")` is free where it is wanted.

`encode_base64url` is the way out of the format rather than a mode of it, for a
caller carrying a secret who wants no part of it left in the clear. Its output
is ordinary unpadded base64url.

`decode` reports what the stream chose — alphabet variant and padding — because
those come out of the stream and not out of a parameter. Where the alphabet is
fixed, `decode_url_strict` fixes it.
