# base65t

Base64URL, plus one character.

`~` is not in the base64 alphabet, so it can mean something else. The input
is cut into blocks of 48 bytes, and each block is written in whichever of
three forms is shortest: base64, or `~~` and the bytes as they are, or `~`, a
48-bit mask saying which bytes are text, the text bytes, and base64 of the
rest. Text that a URL would carry unescaped anyway is carried unescaped —
`alice.jones` encodes as `~~alice.jones`, thirteen characters where base64
needs fifteen — and there is no escaping anywhere.

```
~~alice.jones                                          11 bytes of text, 13 characters
~777vvd73thequickbrownfoxjumpsoverthelazydog.agaICAgICAgICAgaW4
                                                       50 bytes of English, 63 characters
YWxpY2U=                                               ordinary base64, and it decodes to "alice"
```

The last line is the point of the design. A base65t decoder reads any canonical
base64 or base64url stream, padded or not, and returns the same bytes — so a
decoder can be deployed before anything starts producing the new format. It
does not work the other way: `~` is not in base64's alphabet, and a base64
decoder will reject it.

**The wire format is not stable.** v0.4 replaced the segment format of the
earlier versions with these blocks, and nothing promises v0.5 keeps them. What
is stable is the contract: bytes in, printable ASCII out, never longer than
base64, and any base64 stream reads back. `docs/history/` has the earlier
versions and what a day of measuring taught.

## One encoder, no options, no state

```rust
use base65t::{decode, encode, Profile};

let stream = encode(b"alice.jones");
assert_eq!(stream, b"~~alice.jones");
assert_eq!(decode(&stream, Profile::U)?.bytes, b"alice.jones");
```

`encode` takes bytes and returns bytes. There is no mode to pick, no threshold
to tune and no preset to understand: this format exists for the caller who is
*unsure* — who has something that is already text and has to put it through a
channel that must accept bytes — and every word such a caller has to learn
first is a reason to write base64 instead. The encoder is one sentence: **48
bytes of text stay text, otherwise base64, and in between a mask says which is
which.**

It never searches and never remembers. Each block asks one question — how many
of its bytes may stand as they are — and the answer is a popcount over a mask
the profile computes 64 bytes at a time. Blocks are independent, so the
stream can be cut at any block boundary and put back together, and two
implementations cannot disagree about a byte.

Two parameters remain, and neither is a choice about the encoding:

* **The profile** is a statement about the container, not about the stream.
  `U` is RFC 3986 *unreserved*, which goes into a URL query and a cookie value
  as it stands; `T` is printable ASCII without `"` and `\`, which a JSON string
  carries unescaped. The same prose keeps 76 % of its bytes readable under U
  and 96 % under T.
* **`encode_base64url`** is not a mode of the format but the way out of it: for
  a caller carrying a secret who wants no part of it left in the clear. Its
  output is ordinary unpadded base64url.

**The output is never longer than base64** — per input, not on average, with no
exception. The proof is one sentence: each block takes the shortest of three
forms, and base64 is one of them. Where nothing is text it is not merely the
same length as base64url but the *same bytes*.

**The decoder never parses a length a sender chose.** Every block's length
follows from its first characters and its mask, and a mask cannot address
anything past its own block. That was the one place the earlier format stood
behind base64.

## When it is worth it, and when it is not

Measured over 69 samples — the benchmark's short values, its files and its
synthetic set — size against unpadded base64:

| | profile U | profile T |
|---|--:|--:|
| summed over all samples | 98.6 % | 95.0 % |
| samples better than 95 % | 46 % | |
| samples better than 99 % | 65 % | |
| indistinguishable from base64 (≥ 99.9 %) | 29 % | |

Those sums are not worth much, because the samples are two populations and
the average describes neither.

**What wins**, and it is one shape:

| sample | bytes | vs base64 |
|---|--:|--:|
| a git commit id | 40 | 77.8 % |
| a session id, 40 alphanumerics | 40 | 77.8 % |
| a SHA-512 digest in hex | 128 | 78.4 % |
| a JWT, three segments | 155 | 78.7 % |
| a UUID | 36 | 79.2 % |

**What does not:** a JPEG, a PNG, random bytes, anything compressed — 100.0 %,
byte for byte base64.

**And what the mask is for**, the population in between, where the earlier
format found nothing to save and left the text unreadable:

| file | profile | size | bytes readable in the stream |
|---|---|--:|--:|
| English prose (Silesia `dickens`) | U | 95.6 % | **76 %** (was 17 %) |
| `xml` | U | 97.1 % | **66 %** (was 21 %) |
| `bootstrap.css` | U | 96.1 % | **72 %** (was 54 %) |
| `countries.json` | T | 93.5 % | **84 %** (was 47 %) |

The decisive pair is in the corpus twice over: `session_ids_32.bin`, raw
binary session ids, comes out at **100.0 %**; the same thing written as text,
`28-alnum-session-id-40`, at **77.8 %**. The saving is not about binary data.
It is about **carrying something that is already text through a channel that
has to accept bytes**, and the mask extends that to text with punctuation in
it.

base65t is not a density format — against the other encodings it is the
second worst, ahead of base64 alone. What it removes is a decision. A system
that wants both usually writes "if the value is printable, pass it through,
otherwise base64 it, and set a flag": three code paths, a flag to get wrong,
and no bound on what the wrong branch costs. base65t does that per block,
self-describing, with one decoder, and with a one-sentence proof that the
answer is never longer than base64.

## Speed

Against the benchmark's own base64, built by the same compiler in the same
process, single-threaded, best of five. Size is against `ceil(4n/3)`.

The 55 short samples the benchmark keeps as `short/`, profile U:

| sample | bytes | form | size | encode | decode |
|---|--:|---|--:|--:|--:|
| SHA-512 digest, hex | 128 | raw | 78 % | **68 %** | **80 %** |
| JWT, three segments | 155 | raw | 79 % | **64 %** | **80 %** |
| session id, 40 alnum | 40 | raw | 78 % | **62 %** | **81 %** |
| UUID v4 | 36 | raw | 79 % | **64 %** | **87 %** |
| 64 random bytes | 64 | base64 | 100 % | 117 % | 92 % |
| a log line | 93 | mask | 94 % | 170 % | 239 % |
| a JSON record | 92 | mask | 98 % | 181 % | 236 % |
| **all 55, summed as time** | | | | **98 %** | **123 %** |

Large files:

| file | profile | size | encode | decode |
|---|---|--:|--:|--:|
| `x-ray` (binary) | U | 100.0 % | **100 %** | **88 %** |
| `mozilla` | U | 99.0 % | 115 % | 115 % |
| `countries.json` | U | 99.3 % | 142 % | 146 % |
| `dickens` (prose) | U | 95.6 % | 169 % | 322 % |
| `dickens` (prose) | T | 87.9 % | 135 % | 224 % |
| `xml` | T | 85.2 % | 121 % | 215 % |

The rows sort by one thing: how many mask blocks the input produces. A raw
block is a `memcpy` each way and runs at base64's speed. A base64 block
decodes at parity, because consecutive ones are read as one run. **The mask
block costs about three times base64**, on both sides, because it does three
times the work — the mask, a split or join of 48 bytes across two
destinations, and base64 of the remainder — and that is where scalar code
stops. A vectorised compress is the next step and is not taken here.

For comparison, the segment format this replaced encoded `dickens` at 1137 %
and `xml` at 922 %, because its exact programme had to search; it decoded them
faster (165 %) because it found almost nothing and wrote base64. The block
format pays on decoding for what it delivers: the text is in the stream.

## Readability, and what the profile does to it

A raw block and the clear part of a mask block stand in the output as they
stood in the input. How much stays readable is decided first by the profile
and then by the mask:

| file | size U | size T | clear U | clear T |
|---|--:|--:|--:|--:|
| `dickens` (Silesia) | 95.6 % | **87.9 %** | 76 % | **96 %** |
| `xml` (Silesia) | 97.1 % | **85.2 %** | 66 % | **97 %** |
| `lodash.js` | 98.4 % | **88.8 %** | 42 % | **97 %** |
| `bootstrap.css` | 96.1 % | **89.6 %** | 72 % | **96 %** |
| `countries.json` | 99.3 % | **93.5 %** | 15 % | **84 %** |

The space character is not in profile U, so English prose under U is a mask
block every 48 bytes; under T it is a raw block. The mask pays one bit per
byte whatever the punctuation looks like, which is why prose under U is 76 %
readable here and was 17 % under the segment format: there a run of five
letters between two spaces was never worth a header.

## What is here

* **`docs/spec-v0.4.de.md`** — the specification, v0.4, in German. The
  normative document; everything else is downstream of it. The wire format
  is marked not stable.
* **`rust/`** — the reference implementation. No dependencies and no unsafe in
  the default build, and written to be read against the specification rather
  than to be fast: the section numbers are in the comments. `--features simd`
  is the one exception and is off by default.
* **`python/`** — the Python distribution: a PyO3 extension over the same
  crate, packaged with maturin, so what Python runs is byte for byte what a
  Rust caller gets. There is no Python implementation of the format in it, and
  its tests are about what a binding can get wrong on its own.
* **`conformance/reference.py`** — the second implementation §16.3 asks for,
  which is a different thing from a binding: written from the specification
  rather than from the Rust, the mask built bit by bit, no shared code and no
  shared tables. It agrees with the Rust on all 308 vector/profile pairs and
  on a quarter-megabyte stream. The gap that stays open is that both have the
  same author.
* **`docs/vectors.json`** — 183 vectors over both entry points and both
  profiles, as input and expected stream in hex, so a second implementation can
  discharge §16.3 without reading any of this code.
* **`docs/history/`** — v0.1 to v0.3, the segment-format v0.4 that lasted a
  day, the errata, the findings and the pre-registered measurement, with a
  note on what was cut between the versions and why. Nothing there is
  normative; it is the record of how the decisions were reached.

## Building and testing

```sh
cd rust
cargo test --release
cargo clippy --all-targets --release -- -D warnings
cargo run --release --example timing -- <file>...
```

`timing` is the throughput instrument: the encoding and base64url, encode and
decode, on files you name, so that a change meant to be faster can be shown to
be.

The suite is organised by what it proves, not by what it covers:
`tests/vectors.rs` is §15 vector by vector, `tests/roundtrip.rs` and
`tests/against_the_system.rs` are conformance points 1 and 2 of §16,
`tests/blocks.rs` is §9.4 and the block rules of §4, and `tests/errors.rs`
raises each of the ten error codes of §10.4 on purpose.
`tests/against_the_system.rs` needs `base64(1)` and Python and skips itself
where they are missing.

```sh
python3 conformance/test_vectors.py     # the two implementations against each other
python3 conformance/test_containers.py  # §16.6, against Python's own parsers

cd python && maturin build --release --out dist   # the wheel
python -m pip install dist/*.whl && python -m pytest tests -q
```

## Licence

MPL-2.0.
