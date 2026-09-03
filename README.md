# base65t

Base64URL, plus one character.

`~` is not in the base64 alphabet, so it can mean something else. The input is
cut into blocks of 48 bytes. A block whose every byte the profile admits is
written after `~~`, as it stands; every other block is base64. Text that a URL
would carry unescaped anyway is carried unescaped — `alice.jones` encodes as
`~~alice.jones`, thirteen characters where base64 needs fifteen — and there is
no escaping anywhere.

```
~~alice.jones                   11 bytes of text, 13 characters
~~session-eu-central-1.frankfurt~alice.jones-20  and 48 bytes, 50 characters
YWxpY2U=                        ordinary base64, and it decodes to "alice"
```

The last line is the point of the design. A base65t decoder reads any canonical
base64 or base64url stream, padded or not, and returns the same bytes — so a
decoder can be deployed before anything starts producing the new format. It
does not work the other way: `~` is not in base64's alphabet, and a base64
decoder will reject it.

**The wire format is not stable.** v0.4 replaced the segment format of the
earlier versions with these blocks, and nothing promises v0.5 keeps them. What
is stable is the contract: bytes in, printable ASCII out, never longer and
never meaningfully slower than base64, and any base64 stream reads back.
`docs/history/` has the earlier versions and what a day of measuring taught.

> **Reading the percentages.** Two ratios appear below and they point in
> opposite directions, so every number says which it is. **Size** is
> `len(base65t) / len(base64)` — less is better, 100 % means the same length,
> and more than 100 % is impossible by construction. **Time** is
> `t(base65t) / t(base64)` — less is better, 100 % means the same speed. Time
> is always against this crate's own `encode_base64url` and its decoder on a
> pure base64 stream: the same loop shape, allocator and compiler, so the
> ratio is the format rather than a handicap.

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
bytes of text stay text, everything else is base64.**

It never searches and never remembers. Each block asks one question — does the
profile admit every one of these bytes — and the answer is a mask the profile
computes 64 bytes at a time without a branch. Blocks are independent, so a
stream can be cut at any block boundary and put back together, and two
implementations cannot disagree about a byte.

Two parameters remain, and neither is a choice about the encoding:

* **The profile** is a statement about the container, not about the stream.
  `U` is RFC 3986 *unreserved*, which goes into a URL query and a cookie value
  as it stands; `T` is printable ASCII without `"` and `\`, which a JSON string
  carries unescaped.
* **`encode_base64url`** is not a mode of the format but the way out of it: for
  a caller carrying a secret who wants no part of it left in the clear. Its
  output is ordinary unpadded base64url.

**The output is never longer than base64** — per input, not on average, with no
exception. A raw block is 50 characters where base64 is 64, and every other
block *is* base64.

**The decoder never parses a length a sender chose.** Every block's length
follows from its first character or two, and there is no length in the stream
at all — so there is no truncation error, and none of the class of bugs that
comes with one. That was the one place the earlier format stood behind base64.

## When it is worth it, and when it is not

The saving is all in one population, and the honest way to show that is not
to average it away.

**What wins** — values that are entirely text, which is what the format is
for:

| sample | bytes | size |
|---|--:|--:|
| a git commit id | 40 | **77.8 %** |
| a session id, 40 alphanumerics | 40 | **77.8 %** |
| a SHA-512 digest in hex | 128 | **78.4 %** |
| two UUIDs | 73 | **78.6 %** |
| a JWT, three segments | 155 | **78.7 %** |

**What does not** — anything with a byte the profile rejects in every 48-byte
block, which is every large document and everything binary:

| sample | profile | size |
|---|---|--:|
| English prose (Silesia `dickens`) | U | 100.0 % |
| `xml` | U | 100.0 % |
| `countries.json` | U or T | 100.0 % |
| a JPEG, a PNG, random bytes | — | 100.0 % |
| English prose | T | 94.8 % |
| `xml` | T | 90.2 % |

Summed over 69 corpus samples it is 99.98 % in profile U and 99.27 % in T.
That number is byte-weighted and therefore decided by megabyte files, where
this format saves nothing; 43 % of the samples are better than 95 %, and
they are all short values.

The decisive pair is in the corpus twice over: `session_ids_32.bin`, raw
binary session ids, comes out at **100.0 %**; the same thing written as text,
`28-alnum-session-id-40`, at **77.8 %**. The saving is not about binary data.
It is about **carrying something that is already text through a channel that
has to accept bytes**.

base65t is not a density format — against the other encodings it is the
second worst, ahead of base64 alone. What it removes is a decision. A system
that wants both usually writes "if the value is printable, pass it through,
otherwise base64 it, and set a flag": three code paths, a flag to get wrong,
and no bound on what the wrong branch costs. base65t does that per block,
self-describing, with one decoder, and with a one-sentence proof that the
answer is never longer than base64.

**Mixed text with punctuation in it does not become readable here**, and that
is a decision rather than an omission. A version of this format kept the
admitted bytes of a mixed block in the clear behind a bitmask; it made prose
76 % readable and cost three times base64's time on every block it applied
to. The whole case for this format is that choosing it costs nothing. That
door is held open: `~` followed by an alphabet character is reserved.

## Speed

Against the benchmark's own base64, built by the same compiler in the same
process, single-threaded, best of five. Size is against `ceil(4n/3)`.

**On short values base65t is faster than base64 in both directions**, and the
reason is the work: base64 reads a byte, looks up four characters and writes
four, per three bytes; a raw block reads 48 bytes, checks them against a table
and copies them.

| sample | bytes | form | size | encode time | decode time |
|---|--:|---|--:|--:|--:|
| UUID v4 | 36 | raw | 79 % | **52 %** | **82 %** |
| session id, 40 alnum | 40 | raw | 78 % | **54 %** | **68 %** |
| SHA-512 digest, hex | 128 | raw | 78 % | **55 %** | **68 %** |
| JWT, three segments | 155 | raw | 79 % | **59 %** | **65 %** |
| an IPv6 address | 28 | base64 | 100 % | **96 %** | **97 %** |
| a log line | 93 | base64 | 100 % | 109 % | **90 %** |
| 64 random bytes | 64 | base64 | 100 % | 104 % | **88 %** |
| **all 55, summed as time** | | | | **77 %** | **84 %** |

Large files, against the crate's own `encode_base64url` — the same loop shape
and the same allocator, so the ratio is the format and not a handicap. Median
of paired ratios over fifteen rounds, because a shared runner drifts more than
the effect being measured:

| file | profile | size | encode time | decode time |
|---|---|--:|--:|--:|
| `dickens` (prose) | U | 100.0 % | 118 % | **101 %** |
| `xml` | U | 100.0 % | 121 % | **99 %** |
| `x-ray` (binary) | U | 100.0 % | 119 % | **100 %** |
| `dickens` (prose) | T | 95.1 % | 112 % | **86 %** |
| `xml` | T | 88.4 % | **90 %** | **66 %** |

Encoding costs between 90 and 121 %, and the whole of that is one thing: the
check that asks whether a block is all text. It exits at the first byte that
settles the question, so on binary it costs a sixth of base64's time and on a
block whose only rejecting byte is the last it costs half:

| block content | check alone, time | encode time |
|---|--:|--:|
| all admitted (raw) | 32 % | **50 %** |
| binary | 16 % | 116 % |
| text, rejecting byte last | 33 % | 138 % |

**The check already vectorises**, on stable Rust with no `unsafe`: the
per-byte test is arithmetic rather than a table lookup, because a gather does
not vectorise and six compares do, and 112 of the 171 instructions the
compiler emits for it work on vector registers. That is 16 bytes per
operation on the baseline `x86-64` target, which assumes only SSE2.

**Build with `-C target-cpu=native` and the overhead roughly halves** — no
code change, not a byte of output different:

| block content | check alone, time | encode time |
|---|--:|--:|
| all admitted (raw) | 19 % | **37 %** |
| binary | 8 % | 113 % |
| text, rejecting byte last | 19 % | 124 % |

What is left after that is a floor: the encoder has to read every byte to
answer the question, and it reads it once. Getting the same width *without*
a build flag would mean runtime dispatch, and both routes are shut today —
`#[target_feature]` needs `unsafe`, which the crate forbids, and `std::simd`
is not stable (checked on rustc 1.94.1, rust-lang/rust#86656). Decoding
never pays any of this, because the form is in the first character.

For comparison on `dickens`: the segment format this replaced encoded at
1137 %, the mask version at 169 %, this at 104 %.

## Readability, and what the profile does to it

A raw block stands in the output as it stood in the input. How much of a file
that covers is decided by the profile, and one rejected byte costs its whole
block:

| file | size U | size T | clear U | clear T |
|---|--:|--:|--:|--:|
| `dickens` (Silesia) | 100.0 % | **94.8 %** | 0 % | **24 %** |
| `xml` (Silesia) | 100.0 % | **90.2 %** | 0 % | **45 %** |
| `lodash.js` | 100.0 % | **96.7 %** | 0 % | **15 %** |
| `bootstrap.css` | 100.0 % | **97.8 %** | 0 % | **10 %** |
| `countries.json` | 100.0 % | 100.0 % | 0 % | 0 % |

The space character is not in profile U, so a document with spaces has no
fully admitted block at all under U. What stays readable is text that runs in
stretches of 48 bytes: identifiers, ids, hex, and under profile T longer
passages without a quotation mark.

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
  rather than from the Rust, testing each byte one at a time rather than
  through a packed mask, no shared code and no shared tables. It agrees with
  the Rust on all 308 vector/profile pairs and on a quarter-megabyte stream.
  The gap that stays open is that both have the same author.
* **`docs/vectors.json`** — 173 vectors over both entry points and both
  profiles, as input and expected stream in hex, so a second implementation can
  discharge §16.3 without reading any of this code.
* **`docs/history/`** — v0.1 to v0.3, the segment-format v0.4 and the mask
  form, both of which lasted a day, the errata, the findings and the
  pre-registered measurement, with a note on what was cut between the versions
  and why. Nothing there is normative; it is the record of how the decisions
  were reached.

## Building and testing

```sh
cd rust
cargo test --release
cargo clippy --all-targets --release -- -D warnings
cargo run --release --example timing -- <file>...

# what the check costs on a wider vector unit -- no code change, same bytes
RUSTFLAGS="-C target-cpu=native" cargo run --release --example timing -- <file>...
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
