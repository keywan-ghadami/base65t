# base65t

Base64URL, plus one character.

`~` is not in the base64 alphabet, so it can mean something else: it introduces
a length-prefixed run of raw bytes. Text that a URL would carry unescaped
anyway is carried unescaped — `alice.jones` encodes as `~Lalice.jones`, thirteen
characters where base64 needs fifteen — and everything else is base64, in the
same stream, with no escaping anywhere and no delimiter between the two.

```
~Lalice.jones                  11 bytes of text, 13 characters
3q2-73Nl~Qssion-eu-central     4 binary bytes and 18 of text, 26 characters
YWxpY2U=                       ordinary base64, and it decodes to "alice"
```

The last line is the point of the design. A base65t decoder reads any canonical
base64 or base64url stream, padded or not, and returns the same bytes — so a
decoder can be deployed before anything starts producing the new format. It
does not work the other way: `~` is not in base64's alphabet, and a base64
decoder will reject it. The compatibility is one-directional and the migration
path follows from that.

## What is here

* **`docs/spec-v0.2.de.md`** — the specification, v0.2 final, in German. The
  normative document; everything else is downstream of it. v0.2 changed no bit
  of the wire format: every v0.1 stream is a v0.2 stream and the other way
  round. What it settled is what an encoder must *choose* where v0.1 left the
  choice open, and two things a decoder must reject.
* **`rust/`** — the reference implementation. No dependencies, no unsafe, and
  written to be read against the specification rather than to be fast: the
  section numbers are in the comments.
* **`python/`** — the Python distribution: a PyO3 extension over the same
  crate, packaged with maturin, so what Python runs is byte for byte what a
  Rust caller gets. There is no Python implementation of the format in it, and
  its tests are about what a binding can get wrong on its own — argument types,
  the preset and profile names, the fields of the result, the error code.
* **`conformance/reference.py`** — the second implementation §16.3 asks for,
  which is a different thing from a binding: written from the specification
  rather than from the Rust, a plain quadratic dynamic programme instead of the
  sliding windows of §9.2, no shared code and no shared tables. The two agree
  byte for byte over all 456 vectors, all five presets and all three profiles —
  870 pairs — and over fifteen error cases, which counts as much: agreeing
  about valid streams and not about invalid ones is not agreeing about the
  format. The gap that stays open is that both have the same author.
* **`FINDINGS.md`** — what implementing it turned up. Nine places where the
  specification says something the code cannot do or does not say enough for
  two implementations to agree, each with the test that holds it in place. One
  of them is a contradiction inside §11.1 that makes `canonical` two different
  functions; the others are ambiguities.
* **`docs/spec-v0.1.de.md`** and **`docs/errata-v0.1.de.md`** — the previous
  version and the decisions taken against it, kept because they carry the
  reasoning v0.2 only states. `PREREGISTRATION.md` is the measurement rule for
  the two decisions that needed one, written before the run.
* **`docs/vectors.json`** — 456 vectors over every preset and profile, as input
  and expected stream in hex, so a second implementation can discharge §16.3
  without reading any of this code.

## Using it

```rust
use base65t::{decode, encode, Profile};

let stream = encode(b"alice.jones");          // dense, profile U
assert_eq!(stream, b"~Lalice.jones");
assert_eq!(decode(&stream, Profile::U)?.bytes, b"alice.jones");
```

`encode` takes no options and gives the default of §9.3. `decode` takes a
profile and nothing else: the alphabet variant, the padding and the framing are
read out of the stream and reported back, because they are properties of the
stream, while the profile is a statement about the container the stream is
going into and cannot be derived from it.

**Five presets**, all the same format and all read by the same decoder:
`dense` (the default: one forward scan, constant memory, 0.2 % off the shortest
encoding over the corpus and about ten times faster to produce), `legible`
(readability at no cost in size: the shortest encoding, and among the shortest
the one that leaves the most bytes readable), `canonical` (the shortest
encoding, for cache keys),
`opaque` (never a literal, byte-identical to unpadded base64url, for tokens
that carry a secret) and `framed` (fixed-size frames, for random access).

All five are deterministic — the output of a preset is a function of input,
preset and profile (§9.0). What separates them is whether that function carries
parameters: `dense` and `framed` do (`L ≥ 11`, frame size), and §9.5
may still move them; `canonical`, `legible` and `opaque` do not and are frozen.
That is why cache keys belong to `canonical` and not to `dense`.

**Four of the five are never longer than base64** — per input, not on average
(§9.4). `framed` is the exception, at five characters per frame. And on
high-entropy input `dense` does not merely match base64's length: it writes the
same bytes.

**Three profiles** decide what a literal may carry: `U` is RFC 3986
*unreserved*, which goes into a URL query and a cookie value as it stands; `T`
is printable ASCII without `"` and `\`, which a JSON string carries unescaped;
`B` is every octet, which no text container should be given.

**Four decoder entry points.** `decode` detects the framing; `decode_plain`,
`decode_framed` and `decode_url_strict` fix it instead. Auto-detection is a
convenience for a stream you trust — an attacker who controls the stream
chooses the mode, and §14 of the specification says so at more length.

## Density

`cargo run --release --example density`, 1 MiB per input:

| input | base64 | base65t/U | base65t/T |
|---|---|---|---|
| pure binary | 1.333 | 1.333 | 1.333 |
| pure profile-legal text | 1.333 | 1.001 | 1.001 |
| 70 % text / 30 % binary | 1.333 | 1.113 | 1.113 |
| 30 % text / 70 % binary | 1.333 | 1.244 | 1.243 |

Binary data is base64 exactly — that is the guarantee in §9.4, and it is
checked over the corpus rather than argued. Text costs four characters per 4158
bytes, which is the header of one literal segment. Everything between is
between.

These are generated inputs of a stated shape, not a corpus. The corpus
measurement is binary2textbench's, where base65t is the seventh codec:

| | encode | decode | size |
|---|---|---|---|
| no compressor | 119 % of base64's time | 118 % | 132.0 % (base64: 133.3 %) |
| zstd −5 in front | 106 % | 105 % | 56.1 % (base64: 56.6 %) |
| zstd 1 in front | 101 % | 99 % | 40.6 % (base64: 40.6 %) |

The last row is what a protocol that compresses actually sees: the input is
high-entropy by then, `dense` writes the same bytes base64url would, and what
is left is the cost of looking for literals that are not there. The base64 it
is measured against is the same scalar shape with the same table, built by the
same compiler, so the ratio is the format rather than a handicap.

Per file, the cost tracks how often the stream switches segments — §13 of the
specification carries the table, from one segment per 262 144 bytes (98 % / 92 %)
to one per 19 (213 % / 197 %).

## Building and testing

```sh
cd rust
cargo test --release
cargo clippy --all-targets --release -- -D warnings
cargo run --release --example density
cargo run --release --example tiebreak -- --profile=U --lmin=1 <file>...
cargo run --release --example timing -- <file>...
```

`timing` is the throughput instrument: `dense` and `opaque`, encode and decode,
on files you name, so that a change meant to be faster can be shown to be.

`tiebreak` is the instrument for the open question in FINDINGS.md item 1: it
runs both readings of §11.1 over the same files and reports what separates
them, per file rather than as one average. Both are the same length by
construction, and it asserts that rather than assuming it.

The suite is organised by what it proves, not by what it covers:
`tests/vectors.rs` is §15 vector by vector, `tests/roundtrip.rs` and
`tests/against_the_system.rs` are conformance points 1 and 2 of §16,
`tests/canonical.rs` is point 3 as far as one implementation can take it,
`tests/framed.rs` is point 4, and `tests/errors.rs` raises each of the twelve
error codes of §10.4 on purpose. `tests/against_the_system.rs` needs
`base64(1)` and Python and skips itself where they are missing.

```sh
python3 conformance/test_vectors.py     # the two implementations against each other
python3 conformance/test_containers.py  # §16.6, against Python's own parsers

cd python && maturin build --release --out dist   # the wheel
python -m pip install dist/*.whl && python -m pytest tests -q
```

## Licence

MPL-2.0.
