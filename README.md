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

## One encoder, no options

```rust
use base65t::{decode, encode, Profile};

let stream = encode(b"alice.jones");
assert_eq!(stream, b"~Lalice.jones");
assert_eq!(decode(&stream, Profile::U)?.bytes, b"alice.jones");
```

`encode` takes bytes and returns bytes. There is no mode to pick, no threshold
to tune and no preset to understand, and that is the design rather than an
omission: this format exists for the caller who is *unsure* — who has something
that is already text and has to put it through a channel that must accept bytes
— and every word such a caller has to learn first is a reason to write base64
instead. v0.1 had five presets and v0.2 had six; v0.4 has none.

Two parameters remain, and neither is a choice about the encoding:

* **The profile** is a statement about the container, not about the stream, and
  cannot be derived from it. `U` is RFC 3986 *unreserved*, which goes into a
  URL query and a cookie value as it stands; `T` is printable ASCII without `"`
  and `\`, which a JSON string carries unescaped.
* **`encode_base64url`** is not a mode of the format but the way out of it: for
  a caller carrying a secret who wants no part of it left in the clear, and for
  one talking to something that only speaks base64url. Its output is ordinary
  unpadded base64url and any base64 decoder reads it.

What the encoder does, it decides itself: §9.6 looks at the head of the input
once — a magic number, or the integer entropy of the first four kilobytes — and
either writes base64url without looking further, or runs the exact programme of
§9.2 over the whole input. The answer is a function of the input, so two
implementations write the same bytes.

**The output is never longer than base64** — per input, not on average, with no
exception. Where nothing is to be found it is not merely the same length as
base64url but the *same bytes*.

**Into a buffer you own.** `encode_into` and `decode_into` append to a `Vec`
the caller supplies, which is what a loop over many small values wants and what
a buffer registered with the kernel requires.

**Two decoder entry points.** `decode` reads the alphabet variant and the
padding out of the stream and reports both back; `decode_url_strict` rejects
`+` and `/` instead of accepting them.

## When it is worth it, and when it is not

Measured over 69 samples — the benchmark's short values, its files and its
synthetic set — profile U against base64:

| | share of the samples |
|---|--:|
| better than 95 % of base64's size | 55 % |
| better than 99 % | 75 % |
| indistinguishable from base64 (≥ 99.9 %) | 19 % |

Summed it is 98.6 % in profile U and 93.6 % in profile T. Those numbers are not
worth much, because the samples are not one population but two, and the average
describes neither.

**What wins**, and it is one shape:

| sample | bytes | vs base64 |
|---|--:|--:|
| a JWT, three segments | 155 | 76.8 % |
| two ULIDs | 52 | 77.1 % |
| a SHA-512 digest in hex | 128 | 77.2 % |
| a git commit id | 40 | 77.8 % |
| a session id, 40 alphanumerics | 40 | 77.8 % |
| a UUID | 36 | 79.2 % |
| `bootstrap.css` | 281 046 | 92.5 % |

**What does not:**

| sample | bytes | vs base64 |
|---|--:|--:|
| `dickens` (Silesia, English prose, profile U) | 10 192 446 | 98.8 % |
| `countries.json` | 1 408 911 | 99.3 % |
| `x-ray` (Silesia) | 8 474 240 | 100.0 % |
| a JPEG, a PNG, random bytes, anything compressed | — | 100.0 % |

The decisive pair is in the corpus twice over: `session_ids_32.bin`, which is
raw binary session ids, comes out at **100.0 %**; the same thing written as
text, `28-alnum-session-id-40`, comes out at **77.8 %**. The saving is not
about binary data at all. It is about **carrying something that is already text
through a channel that has to accept bytes**.

Which is the point, and the honest way to state the value: base65t is not a
density format — against the other encodings it is the second worst, ahead of
base64 alone. What it removes is a decision. A system that wants both usually
writes "if the value is printable, pass it through, otherwise base64 it, and
set a flag": three code paths, a flag to get wrong, and no bound on what the
wrong branch costs. base65t does that per segment, self-describing, with one
decoder, and with a proof (§9.1 → §9.4) that the answer is never longer than
base64 would have been.

**So it earns its place where a field must accept arbitrary bytes but usually
carries text.** URL query and cookie values (profile U goes into both
unescaped), log fields and debug output (profile T, where the readable part
stays readable), cache keys over mixed payloads, and any migration that has to
decode base64, base64url, padded, unpadded and this, with one decoder.

**It does not earn its place** in front of a compressor, nor on high-entropy
data, where it is byte-identical to base64 by construction, nor anywhere
density is the actual goal: base91z and base85n are both far denser. And §14
names the one place it is strictly behind base64: its decoder parses
attacker-controlled lengths, and base64's does not.

## Speed

On the values the format is for — the 55 samples the benchmark keeps as
`short/`, profile U. Size is against `ceil(4n/3)`, which is what §9.4 promises
and what a URL query carries; the timings are against the benchmark's own
base64, which pads:

| sample | bytes | size | encode | decode |
|---|--:|--:|--:|--:|
| SHA-512 digest, hex | 128 | 77 % | **38 %** | **73 %** |
| JWT, three segments | 155 | 77 % | **45 %** | **69 %** |
| SHA-256 digest, hex | 64 | 78 % | **53 %** | **92 %** |
| session id, 40 alnum | 40 | 78 % | **58 %** | **77 %** |
| UUID v4 | 36 | 79 % | **64 %** | **77 %** |
| a credit card number | 16 | 82 % | **77 %** | **86 %** |
| an IPv6 address | 28 | 100 % | 693 % | 120 % |
| a log line | 93 | 95 % | 816 % | 143 % |
| 64 random bytes | 64 | 100 % | 722 % | 114 % |

The split is not a gradient, and one property explains both halves: where
*every* byte of the input is one the profile admits, the segmentation the
programme would compute can be written down instead (§9.2.4) and the encoder
never runs a dynamic programme at all. Where one space or one `=` interrupts,
it does, and that costs six to eight times base64's encode time — on exactly
the rows where the size is 95 to 100 % and there was nothing to win anyway.
Summed by time over all 55 the figure is 355 %, which is the honest number and
describes neither population.

The same rows in **profile T** are all in the first half: a log line, a SQL
statement and an IPv6 address are entirely printable ASCII.

**On large files the exact programme is expensive, and this is the open point
of v0.4:** 45 to 69 MiB/s against base64's 393 to 601, for nought to 1.5 % of
size. v0.2 had a linear rule for the default that cost 105 to 125 % and gave up
0.22 %; it is gone, because a rule that is not length-optimal cannot satisfy
§11.1, and §11.1 is the byte-equality that cache keys hang on. §13.3 of the
specification states the number rather than hiding it, and §17 names a
branch-free backward pass as the way out.

**`--features simd`** hands the base64 writing *and reading* to a vectorised
kernel. It cannot change a byte — base64 is base64, and `tests/simd.rs` checks
that either way — so it is a speed switch, and it is off by default so the
reference build stays dependency-free and readable.

Decoding was the side that looked closed: a base64 library commits to one
alphabet per call and returns one opaque error, where §5.2 needs both variants
read, §5.4 needs to know which was seen, and §10.4 names ten conditions. What
opens it is that Rule A only asks *does this run hold a `+`, `/`, `-` or `_`* —
a search, not a decode, at a seventh of the cost — and its answer picks the
alphabet for the call. A failed call falls through to the scalar loop, which
names the condition; that is the path taken only by streams already being
rejected.

## Readability, and what the profile does to it

A literal stands in the output as it stood in the input. How much stays
readable is decided by the profile, and by orders of magnitude more than any
encoder rule ever decided it:

| file | size U | size T | clear U | clear T |
|---|--:|--:|--:|--:|
| `xml` (Silesia) | 98.0 % | **79.8 %** | 21 % | **92 %** |
| `dickens` (Silesia) | 98.8 % | **79.9 %** | 17 % | **91 %** |
| `lodash.js` | 97.9 % | **81.9 %** | 23 % | **88 %** |
| `bootstrap.css` | 92.5 % | **82.6 %** | 54 % | **88 %** |

The reason is the space character. Profile U does not admit it, so English
prose falls into five-byte runs and none is worth a literal; profile T does, and
the same text becomes one literal. v0.2 had a preset for this question
(`legible`) that bought five points and cost every other preset 60 to 190 % of
its time. The profile buys seventy points and costs nothing.

## What is here

* **`docs/spec-v0.4.de.md`** — the specification, v0.4 final, in German. The
  normative document; everything else is downstream of it.
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
  rather than from the Rust, a plain quadratic dynamic programme instead of the
  sliding windows of §9.2, no shared code and no shared tables. The gap that
  stays open is that both have the same author.
* **`docs/vectors.json`** — 137 vectors over both entry points and both
  profiles, as input and expected stream in hex, so a second implementation can
  discharge §16.3 without reading any of this code.
* **`docs/history/`** — v0.1, v0.2, the errata, the findings and the
  pre-registered measurement, with a note on what was cut between the versions
  and why. Nothing there is normative; it is the record of how the decisions
  were reached, which is the half a specification does not carry.

## Building and testing

```sh
cd rust
cargo test --release
cargo clippy --all-targets --release -- -D warnings
cargo run --release --example density
cargo run --release --example timing -- <file>...
cargo run --release --example tiebreak -- --profile=U --lmin=1 <file>...
```

`timing` is the throughput instrument: the encoding and base64url, encode and
decode, on files you name, so that a change meant to be faster can be shown to
be. `tiebreak` runs both readings of §11.1 over the same files and reports what
separates them — both are the same length by construction, and it asserts that
rather than assuming it.

The suite is organised by what it proves, not by what it covers:
`tests/vectors.rs` is §15 vector by vector, `tests/roundtrip.rs` and
`tests/against_the_system.rs` are conformance points 1 and 2 of §16,
`tests/canonical.rs` is point 3 as far as one implementation can take it,
`tests/never_worse.rs` is §9.4 and the windowing, and `tests/errors.rs` raises
each of the ten error codes of §10.4 on purpose. `tests/against_the_system.rs`
needs `base64(1)` and Python and skips itself where they are missing.

```sh
python3 conformance/test_vectors.py     # the two implementations against each other
python3 conformance/test_containers.py  # §16.6, against Python's own parsers

cd python && maturin build --release --out dist   # the wheel
python -m pip install dist/*.whl && python -m pytest tests -q
```

## Licence

MPL-2.0.
