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

* **`docs/spec-v0.1.de.md`** — the specification, v0.1 final, in German. The
  normative document; everything else is downstream of it.
* **`rust/`** — the reference implementation. No dependencies, no unsafe, and
  written to be read against the specification rather than to be fast: the
  section numbers are in the comments and the two places where the code has to
  interpret rather than follow are marked.
* **`FINDINGS.md`** — what implementing it turned up. Nine places where the
  specification says something the code cannot do or does not say enough for
  two implementations to agree, each with the test that holds it in place. One
  of them is a contradiction inside §11.1 that makes `canonical` two different
  functions; the others are ambiguities.

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
`dense` (the default, and never longer than base64), `legible`, `canonical`
(deterministic, for cache keys — read FINDINGS.md item 1 first), `opaque`
(never a literal, byte-identical to unpadded base64url, for tokens that carry a
secret) and `framed` (fixed-size frames, for random access).

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
| 70 % text / 30 % binary | 1.333 | 1.113 | 1.112 |
| 30 % text / 70 % binary | 1.333 | 1.244 | 1.243 |

Binary data is base64 exactly — that is the guarantee in §9.4, and it is
checked over the corpus rather than argued. Text costs four characters per 4158
bytes, which is the header of one literal segment. Everything between is
between.

These are generated inputs of a stated shape, not a corpus. The corpus
measurement — throughput as well as size, against the other encodings — is
binary2textbench's job and has not been run yet.

## Building and testing

```sh
cd rust
cargo test --release
cargo clippy --all-targets --release -- -D warnings
cargo run --release --example density
```

The suite is organised by what it proves, not by what it covers:
`tests/vectors.rs` is §15 vector by vector, `tests/roundtrip.rs` and
`tests/against_the_system.rs` are conformance points 1 and 2 of §16,
`tests/canonical.rs` is point 3 as far as one implementation can take it,
`tests/framed.rs` is point 4, and `tests/errors.rs` raises each of the twelve
error codes of §10.4 on purpose. `tests/against_the_system.rs` needs
`base64(1)` and Python and skips itself where they are missing.

## Licence

MPL-2.0.
