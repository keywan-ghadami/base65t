# base65t (Python)

Python bindings for [Base65t](https://github.com/keywan-ghadami/base65t): a
compiled extension over the Rust reference implementation, so what Python runs
is byte for byte what a Rust caller gets.

```python
import base65t

stream = base65t.encode(b"alice.jones")        # dense, profile U
assert stream == b"~Lalice.jones"
assert base65t.decode(stream).bytes == b"alice.jones"
```

`encode` returns `bytes` at every setting, because under profile B the stream
is octets rather than text (specification section 3). Under profiles U and T
every octet is printable ASCII, so `.decode("ascii")` is free where it is
wanted.

`decode` reports what the stream chose — alphabet variant, padding, framing —
because those come out of the stream and not out of a parameter. Where the mode
is fixed, `decode_plain` and `decode_framed` fix it.
