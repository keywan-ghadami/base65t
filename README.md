# base65t

Base64URL, plus one character.

`~` is not in the base64 alphabet, so it can mean something else. The input is
cut into blocks of 48 bytes. A block made entirely of characters the output
alphabet already contains is written after `~~`, as it stands; every other
block is base64. Text that a URL would carry unescaped anyway is carried
unescaped — `alice.jones` encodes as `~~alice.jones`, thirteen characters where
base64 needs fifteen — and there is no escaping anywhere.

**The output alphabet is fixed at 66 characters** — `A–Z a–z 0–9 - . _ ~`,
exactly RFC 3986's *unreserved* set. Nothing else is ever written, not even
`=`, because the encoder produces no padding. That is one alphabet and not a
choice, and it is why the output drops into a URL, a cookie, a header, a JSON
string, a filename or a log field without escaping any of them — **and
survives being pasted unquoted into a shell**, which the conformance test
checks in bash, dash and sh over every stream shape. The test
`the_output_alphabet_is_exactly_unreserved` pins the set in both directions.

Two numbers live here and they are different: the **radix** is 64 + `~` = 65,
which is what the name says, and the **byte values a stream can contain** is
66, because a raw block passes text through and `.` is text. `.` and `~` never
appear in a base64 block.

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
`docs/history/` has the earlier versions and the measurements that dropped them.

> **Reading the percentages.** Two ratios appear below and they point in
> opposite directions, so every number says which it is. **Size** is
> `len(base65t) / len(base64)` — less is better, 100 % means the same length,
> and more than 100 % is impossible by construction. **Time** is
> `t(base65t) / t(base64)` — less is better, 100 % means the same speed. Time
> is always against this crate's own `encode_base64url` and its decoder on a
> pure base64 stream: the same loop shape, allocator and compiler, so the
> ratio is the format rather than a handicap.

## A drop-in for `base64`

```rust
use base65t::{decode, encode};

let stream: String = encode(b"alice.jones");
assert_eq!(stream, "~~alice.jones");
assert_eq!(decode(&stream)?, b"alice.jones");
```

The signatures are the `base64` crate's, so a call site changes its import and
nothing else — the method form works too:

```rust
use base65t::prelude::*;

assert_eq!(URL_SAFE.encode("alice.jones"), "~~alice.jones");
assert_eq!(URL_SAFE.decode("YWxpY2U=")?, b"alice");   // and it reads base64
```

`tests/dropin.rs` is that claim as a test rather than a sentence: it is written
in the other crate's call shapes and stops compiling if a signature drifts.

**The two sides are not equally safe to swap.** A base65t decoder reads every
canonical base64 and base64url stream, so replacing a *decoder* changes nothing
anyone can observe. Replacing an *encoder* starts emitting `~`, which a base64
decoder rejects. Decoders first, encoders once every reader is one — said once,
here, and then the call sites are the ones you already have.

## One encoder, no options, no state

`encode` takes bytes and returns bytes. There is no mode to pick, no threshold
to tune and no preset to understand: this format exists for the caller who is
*unsure* — who has something that is already text and has to put it through a
channel that must accept bytes — and every word such a caller has to learn
first is a reason to write base64 instead. The encoder is one sentence: **48
bytes of text stay text, everything else is base64.**

It never searches and never remembers. Each block asks one question — is every
one of these bytes in the alphabet — and the check answers it in groups of 32
bytes, stopping at the first byte that settles it. Blocks are independent, so a
stream can be cut at any block boundary and put back together, and two
implementations cannot disagree about a byte.

Nothing has to be decided to use it: the alphabet above goes into every
container listed there, and a caller who reads no further is already right.

One thing exists beside it, and it is not a mode of the encoding.
**`encode_base64url`** is the way *out* of the format, for a caller carrying a
secret who wants no part of it left in the clear. Its output is ordinary
unpadded base64url — a subset of the same 66 characters.

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

**What does not** — anything with a rejected byte in every 48-byte block,
which is every large document and everything binary:

| sample | size |
|---|--:|
| English prose (Silesia `dickens`) | 100.0 % |
| `xml` | 100.0 % |
| `countries.json` | 100.0 % |
| a JPEG, a PNG, random bytes | 100.0 % |

Summed over 69 corpus samples it is 99.99 %. That number is byte-weighted and
therefore decided by megabyte files, where this format saves nothing; 43 % of
the samples are better than 95 %, and they are all short values. None of the
difference is the sample: at 64 blocks it costs no file in the corpus
anything. It is the block being all-or-nothing.

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
| **all 55, summed as time** | | | | **71 %** | **81 %** |

Large files, against the crate's own `encode_base64url` — the same loop shape
and the same allocator, so the ratio is the format and not a handicap. Median
of paired ratios over 21 rounds, sorted by size, because that is the whole
story:

| file | bytes | size | encode | decode |
|---|--:|--:|--:|--:|
| generated, all admitted | 4 000 000 | **78.1 %** | **49 %** | **43 %** |
| `manifest.json` | 21 397 | 99.0 % | **122 %** | 101 % |
| `osdb` | 10 085 684 | 99.9 % | **127 %** | 99 % |
| `dickens` (prose) | 10 192 446 | 100.0 % | 102 % | 99 % |
| `xml` | 5 345 280 | 100.0 % | 102 % | 99 % |
| `mozilla` (binary) | 51 220 480 | 100.0 % | 100 % | 100 % |
| random bytes | 262 144 | 100.0 % | 100 % | 100 % |

Three shapes. **Every block raw**: 78 % of the size in half the time, since a
`memcpy` is less work than a base64 loop. **No block raw**: the sample turns
the check off, the output *is* base64url byte for byte, and the time is
base64's. **A few blocks raw** — and this is where base64 wins: the sample sees
one, so every block is checked, and most of them turn out to be base64 and paid
for nothing. `manifest.json` spends 22 % more encoding time for 1 % of size.

So encoding is 47 % to 127 % depending on shape, and the shape that costs is a
stream whose head is unlike its body. Decoding never has the problem — the form
is in the first character — and stays at 99 to 101 %.

The check itself, when it does run, costs 7 % of base64's time on a block that
rejects early and 36 % on one whose only rejecting byte is the last:

| block content | check alone | encode |
|---|--:|--:|
| all admitted (raw) | 36 % | **46 %** |
| binary | 7 % | 100 % |
| text, rejecting byte last | 36 % | 100 % |

The 100 % in the second column of the last two rows is the sample: those
inputs are written as base64url without a block ever being checked. Decoding
never pays the check at all, because the form is in the first character.

## Readability, and what it costs

A raw block stands in the output as it stood in the input. One rejected byte
costs its whole block, so this is not a gradient but a property of the value —
over 103 corpus samples, 32 files come through entirely, 68 not at all, and
three land in between at 1 %, 1 % and 5 %.

| file | size | in the clear |
|---|--:|--:|
| a git commit id | **77.8 %** | **100 %** |
| a SHA-512 digest in hex | **78.4 %** | **100 %** |
| `dickens` (Silesia) | 100.0 % | 0 % |
| `xml` (Silesia) | 100.0 % | 0 % |
| `bootstrap.css` | 100.0 % | 0 % |
| `countries.json` | 100.0 % | 0 % |

The space is not in the alphabet, so a document with spaces has no fully
admitted block at all. What stays readable is text that runs in stretches of
48 bytes: identifiers, ids, hex, digests. **That is the price of one
alphabet.** A wider one would make prose readable and would take the container
guarantee at the top of this file with it; `docs/history/` has what a wider
one scored before it was withdrawn.

## What is here

* **`docs/spec-v0.4.md`** — the specification, v0.4. The normative document;
  everything else is downstream of it. The wire format is marked not stable.
  It is the only normative document, so there is no translation to drift out
  of step with it; the earlier revisions in `docs/history/` stay in German, as
  they were written.
* **`rust/`** — the reference implementation. No dependencies, no features to
  turn on, and `#![forbid(unsafe_code)]`, written to be read against the
  specification rather than to be fast: the section numbers are in the
  comments. That it is also fast comes from the alphabet check being arithmetic
  rather than a table lookup, which is what lets the compiler vectorise it
  without being asked.
* **`python/`** — the Python distribution: a PyO3 extension over the same
  crate, packaged with maturin, so what Python runs is byte for byte what a
  Rust caller gets. There is no Python implementation of the format in it, and
  its tests are about what a binding can get wrong on its own.
* **`conformance/reference.py`** — the second implementation §16.3 asks for,
  which is a different thing from a binding: written from the specification
  rather than from the Rust, testing each byte against a written-out character
  set rather than through arithmetic over thirty-two bytes at a time, no
  shared code and no shared tables. It agrees with
  the Rust on all 154 vectors and on a quarter-megabyte stream.
  The gap that stays open is that both have the same author.
* **`docs/vectors.json`** — 154 vectors over both entry points, as input and
  expected stream in hex, so a second implementation can
  discharge §16.3 without reading any of this code.
* **`docs/history/`** — v0.1 to v0.3, the withdrawn segment-format v0.4 and
  mask form, the errata, the findings and the
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
