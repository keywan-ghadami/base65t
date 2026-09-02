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
* **`rust/`** — the reference implementation. No dependencies and no unsafe in
  the default build, and written to be read against the specification rather
  than to be fast: the section numbers are in the comments. `--features simd`
  is the one exception and is off by default; it takes a vectorised base64
  kernel as a dependency, which has unsafe inside it.
* **`python/`** — the Python distribution: a PyO3 extension over the same
  crate, packaged with maturin, so what Python runs is byte for byte what a
  Rust caller gets. There is no Python implementation of the format in it, and
  its tests are about what a binding can get wrong on its own — argument types,
  the preset and profile names, the fields of the result, the error code.
* **`conformance/reference.py`** — the second implementation §16.3 asks for,
  which is a different thing from a binding: written from the specification
  rather than from the Rust, a plain quadratic dynamic programme instead of the
  sliding windows of §9.2, no shared code and no shared tables. The two agree
  byte for byte over all 449 vectors, all five presets and all three profiles —
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
* **`docs/vectors.json`** — 449 vectors over every preset and profile, as input
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
encoding over the corpus and about ten times faster to produce), `canonical`
(the shortest encoding, for cache keys),
`opaque` (never a literal, byte-identical to unpadded base64url, for tokens
that carry a secret), `framed` (fixed-size frames, for random access) and
`dense-fast` (§9.6: `dense`, minus the looking in windows where a sample says
the looking will not pay — 1.3× to 1.8× the encoding speed for nought to 1.3
points of density, and where there is real density to lose the sample keeps
every window and nothing is skipped).

All five are deterministic — the output of a preset is a function of input,
preset and profile (§9.0). What separates them is whether that function carries
parameters: `dense` and `framed` do (`L ≥ 11`, frame size), and §9.5
may still move them; `canonical` and `opaque` do not and are frozen.
That is why cache keys belong to `canonical` and not to `dense`.

**All but `framed` are never longer than base64** — per input, not on average
(§9.4). `framed` is the exception, at five characters per frame. And on
high-entropy input `dense` does not merely match base64's length: it writes the
same bytes.

**Three profiles** decide what a literal may carry: `U` is RFC 3986
*unreserved*, which goes into a URL query and a cookie value as it stands; `T`
is printable ASCII without `"` and `\`, which a JSON string carries unescaped;
`B` is every octet, which no text container should be given.

**Into a buffer you own.** `encode_into` and `decode_into` append to a `Vec`
the caller supplies, which is what a loop over many small values wants and what
a buffer registered with the kernel requires. The allocation they save is a
fixed cost, so it shows where the values are small: 1.69× on eight bytes, 1.44×
on sixteen, 1.16× on sixty-four, nothing above half a kilobyte.

**Four decoder entry points.** `decode` detects the framing; `decode_plain`,
`decode_framed` and `decode_url_strict` fix it instead. Auto-detection is a
convenience for a stream you trust — an attacker who controls the stream
chooses the mode, and §14 of the specification says so at more length.

## When it is worth it, and when it is not

Measured over 101 samples — the benchmark's short values, its files, its
synthetic set and the Silesia corpus — `dense` in profile U against base64:

| | share of the samples |
|---|--:|
| better than 95 % of base64's size | 39 % |
| better than 99 % | 54 % |
| indistinguishable from base64 (≥ 99.9 %) | 31 % |

Summed over all of them it is 98.8 %. That number is not worth much, because
the samples are not one population but two, and the average describes neither.

**What wins**, and it is one shape:

| sample | bytes | vs base64 |
|---|--:|--:|
| a JWT, three segments | 155 | 76.8 % |
| two ULIDs | 52 | 77.1 % |
| a SHA-512 digest in hex | 128 | 77.2 % |
| a git commit id | 40 | 77.8 % |
| a session id, 40 alphanumerics | 40 | 77.8 % |
| a UUID | 36 | 79.2 % |
| an email address | 24 | 90.6 % |
| `bootstrap.css` | 281 046 | 93.2 % |

**What does not:**

| sample | bytes | vs base64 |
|---|--:|--:|
| `dickens` (Silesia, English prose) | 10 192 446 | 99.5 % |
| `webster` (Silesia) | 41 458 703 | 99.6 % |
| `sql-wasm.wasm` | 659 730 | 99.9 % |
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
stays readable), cache keys over mixed payloads (`canonical`), and any
migration that has to decode base64, base64url, padded, unpadded and this,
with one decoder.

**It does not earn its place** in front of a compressor — measured identical to
base64 there, 40.6 % either way — nor on high-entropy data, where it is
byte-identical to base64 by construction, nor anywhere density is the actual
goal: base91z reaches 37.5 % and base85n 100.7 % where this reaches 132.0 %.
And §14 names the one place it is strictly behind base64: its decoder parses
attacker-controlled lengths, and base64's does not.

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
| no compressor | 105 % of base64's time | 108 % | 132.0 % (base64: 133.3 %) |
| zstd −5 in front | 102 % | 102 % | 56.1 % (base64: 56.6 %) |
| zstd 1 in front | 99 % | 98 % | 40.6 % (base64: 40.6 %) |

That corpus is weighted by bytes, so megabyte files decide it. On the values
the format is for — the ones §0.1 names, and the 55 samples the benchmark
keeps as `short/` — base65t is **faster than base64 as well as smaller**:

| sample | bytes | size | encode | decode |
|---|--:|--:|--:|--:|
| SHA-256 digest, hex | 64 | 77 % | **58 %** | **82 %** |
| JWT, three segments | 155 | 76 % | **63 %** | **71 %** |
| session id, 40 alnum | 40 | 75 % | **69 %** | **89 %** |
| UUID v4 | 36 | 79 % | **80 %** | **89 %** |
| 64 random bytes | 64 | 98 % | 102 % | 108 % |
| a log line | 93 | 95 % | 110 % | 137 % |
| **all 55, as time** | | | **86 %** | **~100 %** |

The throughput advantage *is* the density advantage, near enough one for one,
and the arithmetic says why: base64 reads a byte, looks up four characters and
writes four, per three bytes. A literal reads a byte, tests it against the
profile set and writes **one** — the writing is a `memcpy`. Write less, write
faster. The converse is in the same rows: where the output is the same size as
base64, base65t is slower by exactly what the looking costs.

**`--features simd`** hands the base64 writing *and reading* to a vectorised
kernel. It cannot change a byte — base64 is base64, and `tests/simd.rs` checks
that either way — so it is a speed switch like the thread count, and it is off
by default so the reference build stays dependency-free and readable. On eight
megabytes it takes encoding from 113 % of a scalar base64's time to 80 %, and
decoding from 103 % to 72 %.

Decoding was the side that looked closed: a base64 library commits to one
alphabet per call and returns one opaque error, where §5.2 needs both variants
read, §5.4 needs to know which was seen, and §10.4 names twelve conditions.
What opens it is that Rule A only asks *does this run hold a `+`, `/`, `-` or
`_`* — a search, not a decode, at a seventh of the cost — and its answer picks
the alphabet for the call. A failed call falls through to the scalar loop,
which names the condition; that is the path taken only by streams already being
rejected.

Against a *vectorised* base64 it is 3.5× slower on high-entropy input, and the
reason is structural: base64 does not look, it only writes. base65t has to read
the input to know whether a literal is in it, and on input where none is, that
reading is pure overhead.

Which is what `dense-fast` declines to do. With both — `--features simd` and
the preset — the gap closes to **105 %** of a vectorised base64 on random
bytes, 114 % on JSON, 125 % on prose, where `dense` sits at 565 %, 325 % and
455 %. §13.1.1 and §9.6 of the specification have the numbers.

The last row is what a protocol that compresses actually sees: the input is
high-entropy by then, `dense` writes the same bytes base64url would, and what
is left is the cost of looking for literals that are not there. The base64 it
is measured against is the same scalar shape with the same table, built by the
same compiler, so the ratio is the format rather than a handicap.

Per file, the cost tracks how often the stream switches segments — §13 of the
specification carries the table, from one segment per 262 144 bytes to one per
19.

`encode_parallel(data, profile, threads)` splits the input and writes **the
same bytes**, whatever the thread count: a profile-illegal byte lies in no
literal, so it is a point two runs of the rule agree on, and a cut at a
literal's first byte leaves no segment spanning it (§9.2.1.1). Decoding a plain
stream cannot be split — whether a `~` opens a segment or is payload is known
only to the parser that came before it. `framed` can, in both directions, and
that is what it is for.

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
