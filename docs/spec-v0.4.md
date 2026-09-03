# Base65t — Specification v0.4

**Status:** current. **Wire format: not stable** — v0.4 replaces the segment
format of v0.1 through v0.3, and nothing promises that v0.5 keeps its blocks.
What is stable is the contract, not the bytes: bytes in, printable ASCII out,
never longer than base64, and every base64 stream reads back. Store the version
number alongside any stream you keep.

## What it is

Base64url extended by a 65th character, `~`. The input is cut into blocks of 48
bytes; a block whose bytes are all in the output alphabet below stands raw
after `~~`, every other block is base64. There is no state, no search and no
threshold — that is the entire encoder.

```
~~alice.jones                    11 bytes of text, 13 characters
3q2-73Nlc3Npb24tZXUtY2VudHJhbA   4 binary bytes and 18 of text: base64
YWxpY2U=                         ordinary base64, and it decodes to "alice"
```

### The output alphabet is fixed

Whatever goes in, `encode` writes only these 66 characters:

```
A–Z   a–z   0–9   -   .   _   ~
```

That is exactly RFC 3986's *unreserved* set — every one of those characters is
reachable and nothing else ever appears, including `=`, because the encoder
never produces padding (§5.1). It is one alphabet, not a choice, and it is the
whole reason the format is easy to place:

* **URL query, path segment** — *unreserved* is what "needs no percent-encoding"
  means, by definition of RFC 3986.
* **Cookie value** — all 66 characters are `cookie-octet`, proved from RFC
  6265's ABNF (§7.1).
* **HTTP header value** — no separator, no whitespace.
* **JSON string** — nothing to escape.
* **Filename, log field, `key=value`** — no space and no delimiter.

Checked against Python's own parsers in `conformance/test_containers.py`
(§16.5), and against the encoder in `the_output_alphabet_is_exactly_unreserved`.

`encode_base64url` (§9.3) — the way out of the format for a caller who wants
none of it — writes a subset of the same 66 characters.

There is a second profile, T, which admits 93 raw characters instead of 66 and
buys readability on text with punctuation at the price of the URL. It is not
the default and nothing above depends on it; §7 has it.

## What it guarantees, so that the decision is easy

This format is for the caller who is unsure. They know base64, base64 is never
wrong, and anything they would have to weigh first is a reason not to bother.
The aim is therefore that there is nothing to weigh: in each of the three
dimensions a caller could worry about, base65t is better than base64 or the
same, never worse.

| Dimension | Against base64 | Standing |
|---|---|---|
| **Size** | never larger, `len(encode(x)) ≤ ceil(4·len(x)/3)` | **guaranteed**, per input, both profiles, no exception. The proof is one sentence: a raw block costs 50 characters where base64 costs 64, and every other block *is* base64 (§9.4) |
| **Readability** | a raw block stands in the clear; every other block *is* base64 | **guaranteed** by construction — the format never makes readable input less readable (§13.5) |
| **Time** | at base64's time where the size is the same, usually faster where it is smaller | **measured**, not guaranteed: 99–101 % in both directions, with one named exception (§13.3) |

The exception is named rather than smoothed over: `dickens` under profile T
costs 18 % more encoding time and buys 4.9 % of size for it. It is the one
input in the corpus that trades a dimension against another.

Two further properties follow from the encoder being a mapping rather than a
search:

**Byte equality: guaranteed.** `encode(x, profile)` determines its output from
48 bytes per block. There is nothing two conforming encoders could hold
different opinions about (§11).

**The wire format: not guaranteed.** See the status above.

> **Requirements language.** The key words "MUST", "MUST NOT", "REQUIRED",
> "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT
> RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted
> as described in BCP 14 (RFC 2119, RFC 8174) when, and only when, they appear
> in all capitals, as shown here.
>
> Numbers not marked *exact* are measurements on the corpus named in §16.4.

> **How to read the percentages.** This document names two ratios, and they
> point in **opposite directions**. Every number therefore says which one it
> is, and none appears without that label:
>
> * **Size** = `len(base65t) / len(base64)`. Less is better; 100 % means the
>   same size, and more than 100 % is impossible by §9.4.
> * **Time** = `t(base65t) / t(base64)`. Less is better; 100 % means the same
>   speed, more than 100 % means slower.
>
> Where time is measured, the comparison is `encode_base64url` or `decode` of
> the *same implementation* on a pure base64 stream: the same loop shape, the
> same allocator, the same compiler. Comparing against a foreign base64 would
> measure that library's hand-tuning and not this format.

## Changes from the segment format

The earlier revisions are in `docs/history/`, together with an index of what
was cut between them and why. Section numbers are kept here where the subject
is the same, so that references from that folder still land. Where a section
now describes something else, it says so at its start.

| § | Change |
|---|---|
| 4 | **Blocks instead of segments.** Two fixed-length block forms; no lengths in the stream |
| 6 | Was the literal segment with its length header; is now the reserved form |
| 9 | The encoder is a mapping per block, without search and without state. §9.2 (the program), §9.2.1 (windowing) and §9.2.4 (closed form) are gone; §9.6 remains, but samples its own decision instead of the entropy |
| 10 | The decoder knows a block's length before it reads it |
| 10.4 | `E_RESERVED_LEN` and `E_TRUNCATED` are gone, `E_RESERVED` is added |
| 11 | Canonicity follows from the mapping; the ordering `B < L < S` is gone |
| 13 | Measured afresh. Encoding and decoding sit at base64 in both profiles |
| 14 | The decoder no longer parses an attacker-chosen length |
| 15 | Twelve vectors, new |

## 0. Positioning (non-normative)

### 0.1 One format, one encoder

The caller this format is for is the one who is unsure, and the head of this
document says why that leaves nothing to weigh. What follows from it is the
shape of the API: there is exactly one parameterless `encode` function that takes
bytes and returns bytes. There are no presets, no modes, no thresholds. The
encoder is explained in one sentence: **48 bytes of text stay text, everything
else is base64.**

A third block form was considered and dropped: a mask with one bit per byte,
carrying the admitted bytes of a mixed block in the clear. On every block it
applies to it costs, measured, three times base64's time.
`docs/history/spec-v0.4-maske.de.md` describes it in full; §17 keeps the code
space open for it.

Two parameters remain, and neither is a choice about the encoding:

* **The profile** (§7) is a statement about the *container*, not about the
  stream, and cannot be derived from the stream (§7.2). The default is U.
* **`encode_base64url`** (§9.3) is not a mode of the format but the way out of
  it: for a caller carrying a secret who wants none of it standing in the
  clear (§14), and for one who is only allowed to speak base64url.

Which profile a use calls for, where the table at the head of this document
says what each one carries unchanged:

| Use | Profile | What matters |
|---------|--------|-------------------|
| URL query | U | URL safety without percent-encoding |
| Cookie value | U | `cookie-octet` conformance (§7.1) |
| HTTP header | U or T | ASCII, no separators |
| Cache or dedup key | as the container | byte equality (§11) |
| Log field, JSON string | T | that is where the text stays readable (§13.4) |
| Token containing a secret | — | `encode_base64url`, no cleartext leaks (§14) |

### 0.2 What base65t *is*, in one sentence

> Base64url in blocks of 48 bytes, where a block MAY carry its bytes raw and a
> 65th character says which blocks those are.

### 0.3 What "one decoder for everything" means exactly

> A conforming `decode()` takes an octet stream and a profile and needs **no
> further parameter**. Alphabet variant (§5.2) and padding (§5.3) are
> determined from the stream itself and reported in the result.

### 0.4 Why not "Base85N with a URL alphabet"

RFC 3986's *unreserved* set has 66 characters. A radix-85 encoding fits inside
it, but a passthrough of the Base85N kind additionally needs donor characters
— and there is no room left for those once 64 characters are bound to the
binary core. Base65t goes the opposite way: a core that **is exactly
base64url**, plus a discriminator. Only from that does the superset property
of §5.2 follow.

### 0.5 Where base65t sits in the family (non-normative)

Base65t is the **opener**, not the peak. Base85N carries the same
passthrough idea further and is denser; Base91z compresses. Both ask the
caller to learn a scheme they do not know yet. Base65t does not: the core *is*
base64url, the output is never larger and never appreciably slower, and the
only question left is the profile.

The gain is correspondingly small — 21 % on short values — and so is the cost.
That is the intent: whoever takes the first step should be able to take it
without weighing anything. Whoever then wants mixed text readable, or needs
density, goes one door further.

## 1. Goals

1. **Never worse than base64** (§9.4).
2. Pass text through where it holds together in runs of 48 bytes (§13.4).
3. Stay readable.
4. **No escaping** — not even for `~`.
5. **Read backwards-compatibly** — every canonical base64 or base64url stream,
   padded or not, decodes to the same bytes (§5.2, §5.3). Normative.
6. **Self-describing within the stream** — alphabet and padding are detected,
   not configured (§0.3).
7. Reproducible byte for byte (§11).
8. **Stateless.** No block depends on another (§4).
9. **Not appreciably slower than base64**, in both directions (§13).

### 1.1 The compatibility is asymmetric

| Direction | Holds? |
|----------|-------|
| A base65t decoder reads base64url, unpadded | **yes**, normative |
| A base65t decoder reads base64url, padded | **yes**, normative, §5.3 |
| A base65t decoder reads classic base64 (`+`/`/`), padded or not | **yes**, normative, §5.2/§5.3 |
| **A base64 decoder reads base65t** | **no** — `~` is not in the alphabet |
| **Base65t v0.4 reads v0.1 through v0.3** | **no** — different wire format |

Base65t is a *superset on the reading side* of base64. Migration path: roll
out decoders first, switch encoders later.

**Canonicity of the input.** The statement holds for *canonical* streams. A
base64 stream with remainder bits set (`YWxpY2V`) is rejected with
`E_NONZERO_TAIL` — even where a permissive base64 library would have accepted
it. That is deliberate and belongs in the differential fuzzing corpus (§16.2)
as an *expected divergence*.

## 2. Non-goals

* **Not a compression format.** From roughly 1 kB of text on, `gzip` + base64
  wins clearly.
* **Not a density record.** Z85, basE91 and Base85N are denser on binary.
* **Not a throughput record.** Base64 is the yardstick, and density is never
  traded against it.
* **Not a security mechanism.**

## 3. Notation

* `byte` = a byte of the payload. `char` = a character of the output stream.
* **Base65t produces an octet stream.** In both profiles every octet is
  printable ASCII.
* Core alphabet per base64url (RFC 4648 §5): 0–25 `A`–`Z`, 26–51 `a`–`z`,
  52–61 `0`–`9`, 62 `-`, 63 `_`.
* The 65th character is `~` (U+007E), not part of the alphabet, without value.
* Bit order MSB-first.

**Alphabet character.** An octet is an *alphabet character* if the decoder
interprets it as a value 0–63: the characters of base64 runs — **not** the raw
bytes of a block. This carries §5.4.

## 4. Stream structure

```
Stream      := Block*
Block       := Base64Block | RawBlock
Base64Block := <64 alphabet characters>                    # 48 bytes
RawBlock    := "~~" <48 raw bytes>
```

**The input is cut at absolute offsets `k · 48`.** Every block but the last
covers exactly 48 input bytes; the last covers the remaining `m ≤ 48` and is
correspondingly shorter: a base64 tail has `ceil(4m/3)` characters, a raw tail
`2 + m`, and it runs to the end of the stream.

**Blocks are independent (normative).** The encoding of a block depends only
on its own bytes. Everything that separates this format from its predecessors
follows from that: the encoder has no state, the decoder knows a block's
length before it reads it, and a stream can be split and rejoined at any block
boundary.

**Base64 blocks tile.** 48 bytes are 16 quanta, so a base64 block ends on a
quantum boundary, and two consecutive base64 blocks are one base64 run. That
is why a pure base64 stream is a valid stream, and why a decoder MAY read
consecutive base64 blocks as a single run (§10.1).

**`~` followed by an alphabet character is reserved (normative).** No encoder
writes it, and a decoder MUST reject it with `E_RESERVED`. This lets a later
revision introduce a third block form without a decoder of this revision
silently misreading it (§17).

**Why 48.** Three conditions: divisible by 3, so that base64 blocks tile;
divisible by 6, which the reserved form of §17 would need; and large enough
that the two marker characters of a raw block are four percent of it and not a
third — at six bytes per block the raw form saves exactly nothing (§9.1).
Larger blocks save little more and tip wholly to base64 more often, because a
single byte outside the profile suffices.

An empty stream is valid and decodes to zero bytes.

## 5. Base64 runs

Base64url. Let `n` be the character count without padding:

| `n mod 4` | Bytes in the last quantum | Valid |
|-----------|--------------------------|--------|
| 0 | 3 | yes |
| 2 | 1 | yes |
| 3 | 2 | yes |
| 1 | — | **no** (`E_ALIGN`) |

**Canonicity:** unused bits in the last character MUST be 0
(`E_NONZERO_TAIL`).

### 5.1 Encoder alphabet

An encoder MUST use exactly one alphabet per call and MUST NOT switch within a
stream.

| Alphabet | 62 / 63 | Purpose |
|----------|---------|-------|
| **URL** (default) | `-` / `_` | everything new |
| **Classic** (opt-in) | `+` / `/` | interop only |

Classic is not URL-safe and MUST NOT be offered as the default. The encoder
**never** produces padding (§5.3).

### 5.2 Permissive decoding (normative)

Every conforming decoder MUST accept both alphabets: `-`/`+` → 62, `_`/`/` →
63.

### 5.3 Padding (normative)

> **Rule P.** A base64 run that ends **at the end of the stream** MAY be
> terminated with 1 or 2 `=`. The decoder MUST accept those. "Stream" means
> the whole octet stream.

| `k` (number of `=`) | required |
|------------------|--------------|
| 0 | `n mod 4 ∈ {0, 2, 3}` |
| 1 | `n mod 4 == 3` |
| 2 | `n mod 4 == 2` |

Any other combination → `E_PADDING`. An `=` at any other position →
`E_CHARSET`.

**Implementation trap.** Padding MUST NOT be stripped from the end of the
stream up front. In profile T, `=` is a legal raw byte, and a raw tail runs to
the end of the stream: `~~a=b=` is four bytes of payload (TV10).

### 5.4 Alphabet consistency (Rule A, normative)

> **Rule A.** A stream MUST NOT mix the two alphabet variants. If the set of
> **alphabet characters** contains both a character from {`+`,`/`} and one
> from {`-`,`_`} → `E_MIXED_ALPHABET`. The rule holds over the whole stream,
> and so across the raw blocks that sit between two base64 blocks.

Rule A concerns alphabet characters only. Raw bytes do not count — in profile
U nearly every raw block contains `-` or `_`. Scanning the whole stream
rejects valid streams (TV7).

### 5.5 Reporting and the strict variant (normative)

A `decode()` result MUST contain:

```
alphabet_seen : { none, url, classic }
padding_seen  : bool
```

In addition, `decode_url_strict` MUST be offered (it rejects `classic` with
`E_NON_URL_ALPHABET`).

## 6. The reserved form

```
"~" <alphabet character> ...       # reserved, E_RESERVED
```

No encoder of this revision writes `~` followed by an alphabet character, and
a decoder MUST reject it. The two characters cost nothing today and keep the
code space open. The dropped third block form (§17) starts here: `~`, eight
mask characters carrying one bit per byte, the admitted bytes in the clear,
then base64 of the rest. It makes a mixed block two-thirds readable and costs
three times base64's time for it
(`docs/history/spec-v0.4-maske.de.md`).

The reason for dropping it is §0.1: the format lives on the decision to use it
costing nothing. "Three times slower on my JSON blobs" is a sentence that
tips exactly that decision, and readable mixed text is not what the format
advertises.

`~` followed by something that is neither `~` nor an alphabet character is not
a reserved stream but a broken one: `E_CHARSET`.

## 7. Profiles

| Profile | Admitted raw bytes | Direct in a URL query? |
|--------|---------------------|-------------------|
| **U** (default) | RFC 3986 *unreserved* (66 characters) | **yes** |
| **T** | ASCII 0x20–0x7E without `"` and `\` (93 characters) | no |

A byte outside the profile costs its whole block: the block becomes base64.
That is the coarseness this format traded for its speed, and §13.4 quantifies
it.

**Profile T** is JSON-string-safe, **not** CSV-structure-safe and **not**
URL-safe. **And it contains the space** (0x20): a whitespace-separated log
line has to quote a T value, a `key=value` format does not. Found by the
container test of §16.5.

### 7.1 Cookie conformance of profile U (proved, not measured)

RFC 6265 §4.1.1 defines:

```
cookie-octet = %x21 / %x23-2B / %x2D-3A / %x3C-5B / %x5D-7E
```

Profile U's alphabet — 62 alphanumerics plus `-` (0x2D), `.` (0x2E), `_`
(0x5F), `~` (0x7E) — lies entirely within those ranges. All 66 characters
checked, no exception. The statement follows from the ABNF and is therefore
**provable, not empirical**. Whether real cookie parsers hold to the ABNF is
the weaker, empirical question; Python's `http.cookies` does (§16.5).

### 7.2 Why the profile stays a parameter

The profile cannot be derived from the stream: a stream whose raw bytes happen
to be *unreserved* only is equally valid under U and T. It describes the
expectation of the **container**, not a property of the stream.

## 8. Framed mode — **withdrawn**

See `docs/history/`. A block format with fixed block boundaries needs no
second mode for random access; whoever wants it indexes block starts, and that
is a candidate for extension (§17), not a question about the format.

## 9. Encoder

### 9.0 Principle (normative)

> For every block the encoder checks whether the profile admits **every** one
> of its bytes. If it does and the block has at least four bytes, it writes
> `~~` and the bytes; otherwise it writes the block as base64.

That is the whole rule. It is a mapping from 48 bytes and a profile to an
output, without search, without state, and without a tie an ordering would
have to break. A test vector therefore checks bytes rather than lengths, and
`docs/vectors.json` does so over 173 vectors.

The four bytes are not a threshold but the point at which the raw form stops
being more expensive: see §9.1.

### 9.1 What the forms cost

For a block of `m` bytes:

```
Base64:  ceil(4m/3)
Raw:     m + 2                          only if every byte is admitted
```

| `m` | Base64 | Raw | |
|--:|--:|--:|---|
| 1 | 2 | 3 | base64 |
| 3 | 4 | 5 | base64 |
| 4 | 6 | 6 | tie → raw |
| 6 | 8 | 8 | tie → raw |
| 7 | 10 | 9 | raw |
| 48 | 64 | 50 | raw, 78 % size |

From four bytes on, the raw form is never longer, and at four, five and six it
is exactly the same length; the encoder takes it there anyway, because a tie
costs nothing and text in the clear is what the format is for. The gain grows
with the block size and runs against `(m+2)/(4m/3)`, so against 78 % size at
48 bytes and 75 % in the limit.

**All or nothing.** A single byte outside the profile costs its whole block.
That is coarse, and it is the trade this revision makes: a finer encoding — a
mask with one bit per byte — was dropped over three times base64's time (§6,
§17, `docs/history/spec-v0.4-maske.de.md`). What the coarseness means on real
data is in §13.4: short values consisting entirely of text reach 78 % size;
large documents gain **nothing** in profile U and five to ten percent in
profile T.

### 9.2 Optimal segmentation — **gone**

There is nothing to segment.

### 9.3 Entry points

| Function | What it does | Profile |
|---|---|---|
| `encode(x)` | the encoding | U |
| `encode(x, profile)` | the same, in the profile the container demands | U or T |
| `encode_base64url(x)` | base64url and nothing else (§14) | — |

A call without parameters MUST yield `encode` + profile U. Libraries SHOULD
export exactly one parameterless `encode` function.

### 9.4 Never-worse guarantee (normative)

```
len(encode(x, profile)) <= ceil(4 * len(x) / 3)
```

**Per input, not on average, without exception.** Proof: a raw block costs
`m + 2 ≤ ceil(4m/3)` (§9.1), every other block *is* base64, and base64 blocks
tile, so the sum of the base64 forms is exactly `ceil(4n/3)`. ∎

**Sharper:** where no block goes raw, the encoder writes not merely as many
characters as base64url but **the same bytes**.

**Scope.** The length of the encoded stream in octets, not transport or
container overhead.

### 9.5 Segment switch rate — **gone**

There are no segment switches.

### 9.6 The sample (normative)

Asking §9.0's question per block costs time, and on an input where the answer
is always "no" it is the only time this format spends beyond base64. Such
inputs are not only binary: English prose in profile U has a space in every
block, so no block goes raw, and for this format it is as binary as a JPEG.

> **Rule.** Before encoding, the encoder applies §9.0 to the first **64
> blocks**. If none of them yields the raw form, the whole stream MUST be
> written as base64url. Otherwise §9.0 applies to every block.

**It is the same check, once up front.** No magic numbers, no entropy, no
logarithm two implementations would have to agree on — the sample measures the
decision itself and not something correlated with it. Earlier revisions
decided by entropy; `docs/history/spec-v0.4-segmente.de.md` describes them.

**The output stays a function of the input.** The sample is a fixed prefix,
the number of blocks is a constant, and the check is §9.0's. §9.0 applies
unchanged.

**A wrong decision costs size, never correctness.** A skipped stream is
exactly base64url, so §9.4 holds in every case.

**Shorter than the sample means: no sample.** Where the input is at most 3072
bytes long, an encoder MAY skip the rule and encode directly per §9.0. That is
not an exception but an observation: the sample then sees every block §9.0
sees anyway. If it says "yes", §9.0 applies unchanged; if it says "no", every
block is base64 and §9.0 writes base64url by itself. The output is the same
either way, and the reference implementation checks this over 2000 random
inputs in both profiles. Implementing the rule literally is equally conforming
— the Python reference does exactly that, and the two implementations agree.

**Why 64 blocks.** Two reasons, both of which count. Measured, it is the knee:
at 32 blocks `xml` under profile T is misjudged and gives up 9.8 points of
size on five megabytes, at 64 it is not, and above that almost nothing moves
while fewer streams take the cheap path. And 64 blocks are **3072 bytes** —
longer than every value §0.1 names. For a URL query, a cookie value, a header
or a cache key the sample is therefore not a sample at all but the whole
input, and it can give up nothing there.

**What it costs over the corpus**, against "always check"
(`binary2textbench`, `--example sample`, 101 samples), as size:

| | Profile U | Profile T | written as pure base64 |
|---|--:|--:|--:|
| always check | 99.95 % | 97.40 % | — |
| sample, 64 blocks | 99.99 % | 97.50 % | 67 and 37 of 101 |

Four hundredths of a point in U and a tenth in T. In exchange, two thirds of
all files in profile U are written byte for byte as base64, in base64's time.

## 10. Decoder

### 10.1 Procedure

```
pos := 0 ; alphabet_seen := none ; padding_seen := false
while pos < len:
    if stream[pos] != '~':
        # Base64 run: every block starting with an alphabet character is
        # 64 characters long; the last one is whatever remains. Blocks
        # tile, so the run can be decoded as a whole (§4).
        end := pos
        while end < len and stream[end] != '~': end := min(end + 64, len)
        emit base64_decode(stream[pos..end], padding_allowed = (end == len))
        pos := end
    elif pos + 1 == len:                                  -> E_TRAILING_TILDE
    elif stream[pos+1] == '~':
        # Raw block: 48 bytes, or whatever remains.
        end := min(pos + 2 + 48, len)
        check: every byte stream[pos+2..end] legal in profile  else E_PROFILE
        emit stream[pos+2..end] ; pos := end
    elif stream[pos+1] is an alphabet character:          -> E_RESERVED    # §6
    else:                                                 -> E_CHARSET

base64_decode(seg, padding_allowed):                       # §5, §5.3
    k := padding_allowed ? number of trailing '=' (max 2) : 0
    n := len(seg) − k
    check: k == 0 ∨ (k == 1 ∧ n mod 4 == 3) ∨ (k == 2 ∧ n mod 4 == 2)
                                                             else E_PADDING
    if k > 0: padding_seen := true
    check: n mod 4 != 1                                      else E_ALIGN
    check: all n characters are alphabet characters          else E_CHARSET
    note_alphabet for every character with value 62/63
    check: remainder bits of the last quantum == 0           else E_NONZERO_TAIL
    return bytes

note_alphabet(c):
    if c in {'+','/'}:  if alphabet_seen == url     -> E_MIXED_ALPHABET
                        else alphabet_seen := classic
    if c in {'-','_'}:  if alphabet_seen == classic -> E_MIXED_ALPHABET
                        else alphabet_seen := url
```

**There is no search and no length.** The decoder never reads "up to the next
`~`". Every block length is fixed before it touches a payload byte, and none
of them is in the stream. That is more than a convenience: a `~` byte inside a
raw block is payload, and a decoder that searches for it reads it wrongly
(TV3).

**Why the tail is unambiguous.** A raw tail runs to the end of the stream, and
so does a base64 tail. "Fewer characters remain than a full block needs" is
the whole of tail detection, and because no block announces a length, nothing
can be truncated either: a shortened stream decodes to a prefix of the input
or fails Rule P, not a length field.

### 10.2 Entry point

```
decode(stream, profile)
decode_url_strict(stream, profile)  # rejects '+' and '/' with E_NON_URL_ALPHABET
```

### 10.3 Framed mode — **withdrawn**

See §8.

### 10.4 Error cases

| Code | Condition |
|------|-----------|
| `E_TRAILING_TILDE` | the stream ends with a single `~` |
| `E_RESERVED` | `~` followed by an alphabet character (§6) |
| `E_PROFILE` | a raw byte outside the profile's alphabet |
| `E_ALIGN` | base64 run length `mod 4 == 1` |
| `E_NONZERO_TAIL` | remainder bits in the last quantum ≠ 0 |
| `E_CHARSET` | not an alphabet character at an alphabet position (including `~` inside a base64 run, `=` away from the end of the stream, and `~` followed by a character without value) |
| `E_PADDING` | Rule P violated |
| `E_MIXED_ALPHABET` | Rule A violated |
| `E_NON_URL_ALPHABET` | `decode_url_strict` only |

Nine codes. `E_TRUNCATED` no longer exists, because there is nothing that
could be truncated.

**Allocation limits.** There is no length in the stream that a sender chooses.
A raw block holds at most 48 bytes, and a base64 run yields three bytes per
four characters. It follows that the specification needs **no protocol-side
limit for individual blocks**, and that the class of single-allocation bugs
which §14 of the segment format named as its one weakness against base64 does
not exist. The number of blocks is unbounded; implementations SHOULD offer
limits on total size and running time.

## 11. Canonicity and signatures

**The encoder is a mapping** (§9.0): per block, 48 bytes and the profile
determine the output, and the blocks are independent. Two conforming encoders
write the same bytes for the same input and the same profile. That is enough
for cache keys, dedup keys and content addresses, where the same side produces
and compares.

The *format* is nevertheless not canonical, for two reasons. First, the
**profile is a choice**: the same input yields different streams under U and
T. Second, the **decoder accepts forms no encoder writes**: the classic
alphabet (§5.2), padding (§5.3), and a base64 block where a raw block would be
shorter. A third party can rewrite the same stream without changing the
decoded bytes.

> **Rule:** never sign, hash or compare the output of `encode`. Sign the
> **decoded bytes**. `decode(encode(x)) == x` always holds.

**The ordering `B < L < S`** of the segment format is gone. It was needed
there because several segmentations could be the same length and one of them
had to be chosen. Here there are two forms per block and one condition that
decides.

## 12. Density

**Exact**, from §9.1:

Characters per input byte; less is better.

| Input | Base64 | **Base65t** |
|---------|--------|-------------|
| A block with one byte outside the profile | 1.333 | **1.333** — the same bytes |
| Purely profile-legal text | 1.333 | **1.0417** — `50/48`, every block raw |

There is nothing in between: a block is one or the other. What a file achieves
therefore depends only on how many of its 48-byte blocks consist entirely of
admitted bytes.

**Measured** over the binary2textbench corpus (69 samples, `--example gain`),
size against unpadded base64:

| | Profile U | Profile T |
|---|--:|--:|
| Sum over all samples | 99.99 % | 99.51 % |
| Samples better than 95 % | 43 % | |
| Indistinguishable from base64 (≥ 99.9 %) | 55 % | |

**The sum line is honest and misleading at once**, because it is weighted by
bytes and the corpus is dominated by megabyte files. On those this revision
gains almost nothing: a document with a space every five characters has, in
profile U, not one wholly admitted block. The distribution has two halves, and
the interesting one is the small one:

| Sample | Bytes | Profile U, size |
|---|--:|--:|
| Git commit ID | 40 | **77.8 %** |
| Session ID, 40 alnum | 40 | **77.8 %** |
| SHA-512 digest, hex | 128 | **78.4 %** |
| Two UUIDs | 73 | **78.6 %** |
| JWT, three segments | 155 | **78.7 %** |
| Prose, XML, JSON, every megabyte file | | 100.0 % |

**Against the earlier revisions**, same samples, as size: the segment format
reached 98.57 % in U, the mask format 98.65 %. This revision is worse on large
files and equally good on short values — and §13 says what it gets for that.
Of the difference, 0.01 points in U and 0.24 in T are the sample of §9.6; the
rest is the coarseness of the block itself.

## 13. Performance

Measured against the base64 implementation of the bench, which lives in the
same process and was built by the same compiler. Everything single-threaded,
best of five runs, base64 = 100 %.

### 13.1 What the check costs, and where it is not incurred

A raw block is a `memcpy` in both directions, a base64 block is base64. The
only work this format has beyond base64 is the question per block: does the
profile admit **every** byte?

It is built as cheaply as it can be without vector intrinsics — it breaks off
at the first byte that settles it, tests a necessary condition up front with a
single operation per 32 bytes, and its per-byte test is arithmetic rather than
a table lookup, because a gather does not vectorise and six comparisons do.
Measured against `encode_base64url` of the same implementation, median of
paired ratios, as time:

| Input, per 48-byte block | the check alone | encoding overall |
|---|--:|--:|
| wholly admitted (raw) | 33 % | **48 %** |
| binary | 6 % | 109 % |
| text, rejecting byte at the end of the block | 34 % | 145 % |

**And then it is mostly not incurred at all.** The second and third rows are
exactly the cases where the sample of §9.6 says "no": the stream is written as
base64url, no block is checked, and the row drops out of the reckoning. What
remains is the first — the one where the format gains something and is faster
than base64.

When **decoding** this work never arises: the form is in the first character.

### 13.2 The throughput criterion

> **Throughput is a goal, size is a guarantee.** A change MUST NOT touch the
> guarantee of §9.4 or the byte equality of §11. Within that bound it SHOULD
> improve throughput.

An encoder or decoder is not non-conforming because it is slower than base64.
It is non-conforming if it writes the wrong bytes.

### 13.3 Large files

Against `encode_base64url` of the same implementation — the same loop shape,
the same allocator — median of paired ratios over 21 rounds:

| File | Profile | Size | Encode, time | Decode, time |
|---|---|--:|--:|--:|
| generated, wholly admitted | U | 78.1 % | **48 %** | **40 %** |
| prose, space every 6 bytes | T | 78.1 % | **40 %** | **35 %** |
| `xml` | T | 88.4 % | **91 %** | **68 %** |
| `dickens` | T | 95.1 % | 118 % | **90 %** |
| `dickens` | U | 100.0 % | **100 %** | **100 %** |
| `xml` | U | 100.0 % | **99 %** | **100 %** |
| `countries.json` | U | 100.0 % | **100 %** | 101 % |
| `x-ray` (binary) | U | 100.0 % | 101 % | 101 % |
| random bytes | U | 100.0 % | **100 %** | 101 % |

**The table is sorted by size, and that is the whole statement.** Where the
output is not shorter than base64, it is base64 and costs base64's time: 99 to
101 %, in both directions. Where it is shorter, it is usually faster too,
because a `memcpy` is less work than a base64 loop.

One row falls out: `dickens` in profile T is 4.9 % smaller and costs 18 % more
encoding time. There the sample rightly says "check" — there are raw blocks —
but most blocks do contain a line break and become base64, and for those the
check is overhead. That is the one case where the format trades size against
time, and it is named rather than smoothed over.

For comparison, the same files, as time: the segment format cost 1137 % to
encode `dickens`, the mask format 169 %, this one 100 %.

### 13.4 Short values

The 55 short samples, profile U, size against `ceil(4n/3)`, time against the
bench's base64 (`--example short`):

| Sample | Bytes | Form | Size | Encode, time | Decode, time |
|---|--:|---|--:|--:|--:|
| UUID v4 | 36 | raw | 79 % | **52 %** | **82 %** |
| Session ID, 40 alnum | 40 | raw | 78 % | **54 %** | **68 %** |
| SHA-512 digest, hex | 128 | raw | 78 % | **55 %** | **68 %** |
| JWT, three segments | 155 | raw | 79 % | **59 %** | **65 %** |
| Credit card number | 16 | raw | 82 % | **68 %** | **88 %** |
| First and last name | 12 | base64 | 100 % | **93 %** | 119 % |
| IPv6 address | 28 | base64 | 100 % | **96 %** | **97 %** |
| SQL statement | 118 | base64 | 100 % | 104 % | **83 %** |
| Log line | 93 | base64 | 100 % | 109 % | **90 %** |
| 64 random bytes | 64 | base64 | 100 % | 104 % | **88 %** |
| **all 55 samples, as time** | | | | **77 %** | **84 %** |

**On short values base65t is faster than base64, in both directions.** The
reason is the work balance: base64 reads a byte, looks up four characters and
writes four — per three bytes. A raw block reads 48 bytes, checks them with
six comparisons per byte and copies them. Whoever writes less writes faster.
The rows at 100 % size cost at most nine percent more time, and they decode
faster.

For comparison, each as time: the segment format sat at 355 % to encode here,
the mask format at 98 % to encode and 123 % to decode.

### 13.5 What stays readable

Share of bytes that stand in the stream as they do in the input
(`--example clear`):

| File | Segment format, U | Mask format, U | **v0.4 U** | **v0.4 T** |
|---|--:|--:|--:|--:|
| Prose (dickens) | 17 % | 76 % | **0 %** | **24 %** |
| XML | 21 % | 66 % | **0 %** | **45 %** |
| CSS | 54 % | 72 % | **0 %** | **10 %** |
| JSON | 9 % | 15 % | **0 %** | **0 %** |

**That is this revision's price, and it is high.** A block goes raw only if
all 48 bytes are admitted, and in a document with punctuation that does not
happen in profile U. What stays readable is what holds together in runs of 48
bytes: identifiers, IDs, hexadecimal values, and in profile T longer stretches
of text without quotation marks.

Whoever wants readable mixed text will not find it here. That is a decision,
not a gap: the mask format delivered it and cost three times base64's time for
it, and this format lives on the decision to use it costing nothing (§0.1,
§6).

## 14. Security

* **The decoder parses no length whatsoever.** The segment format stood behind
  base64 here: its decoder read lengths up to 4158 out of the stream, which an
  attacker could choose. Here there is no length in the stream; every one
  follows from the block form and the block size. What remains is the same as
  for base64: the total length of the input.
* **Raw bytes leak structure** — which blocks consist entirely of text is
  visible in the stream, and their content stands in the clear. That is what
  `encode_base64url` is for (§9.3); its output is ordinary base64url.
* **Two auto-detections are two parser-differential surfaces:** alphabet
  (§5.2) and padding (§5.3). Countermeasures: Rule A, Rule P,
  `alphabet_seen` / `padding_seen` and `decode_url_strict` (§5.5). Differential
  fuzzing is mandatory, not optional.
* **No padding oracle** — padding is only validated, never produced.
* **Malleability** is excluded at block level, reduced at alphabet and padding
  level, and **not** excluded at profile level nor against a third party
  rewriting a raw block as a base64 block (§11).
* Decoded output is **untrusted binary**, not text.

## 15. Test vectors

Twelve vectors, each a test in `rust/tests/vectors.rs`. The machine-checkable
set — 173 entries over both entry points and both profiles — is in
`docs/vectors.json`. The vectors of the earlier revisions are in
`docs/history/`; their *inputs* were carried over, their streams were not.

### TV1–TV4 — the two forms (profile U)

| # | Input | Stream | Length | Base64 would be |
|---|---------|-------|-------|-------------|
| TV1 | `alice.jones` | `~~alice.jones` | 13 | 15 |
| TV2 | `DE AD BE EF` + `session-eu-central` | `3q2-73Nlc3Npb24tZXUtY2VudHJhbA` | 30 | 30 |
| TV3 | `sub~alice~jones` | `~~sub~alice~jones` | 17 | 20 |
| TV4 | 100 × `a` | `~~` + 48 `a`, `~~` + 48 `a`, `~~aaaa` | 106 | 134 |

**On TV2.** Four of the 22 bytes are not admitted, so the block is base64, and
the stream is byte for byte `encode_base64url`. The segment format wrote this
input in 26 characters; that is the price for an encoder that is a comparison.

**On TV3.** A `~` in a raw block needs nothing, because the block length is
fixed. `hello~Alice` becomes `~~hello~Alice`.

### TV5 — one byte decides the block

```
Input:     the quick brown fox jumps over the lazy dog. again      (50 bytes)
Profile U: dGhlIHF1aWNrIGJyb3duIGZveCBqdW1wcyBvdmVyIHRoZSBsYXp5IGRvZy4gYWdhaW4
           67 characters, byte for byte base64url — the space is not admitted in U
Profile T: ~~the quick brown fox jumps over the lazy dog. aga  aW4
           53 characters: the first block raw, the two remaining bytes as base64
```

### TV5b — the reserved form

`~AAAAAAAA`, `~7abc`, `~_` → `E_RESERVED`. `~=`, `~ a` → `E_CHARSET`. The
difference is normative: the first is a stream of a revision this decoder does
not know, the second is broken.

### TV6 — backwards compatibility

| Stream | Bytes | `alphabet_seen` | `padding_seen` |
|-------|-------|-----------------|----------------|
| `PDw_Pz8-Pg` | `<<???>>` | url | false |
| `PDw/Pz8+Pg` | `<<???>>` | classic | false |
| `YWxpY2Uuam9uZXM` | `alice.jones` | none | false |
| `YWxpY2U=` | `alice` | none | true |

A base64 stream of any length reads in blocks of 64 characters, and that is
invisible, because base64 blocks tile.

### TV7 — alphabet consistency

`PDw_Pz8+Pg` and `PDw/Pz8-Pg` → `E_MIXED_ALPHABET`. The rule holds across raw
blocks: a URL base64 block, a raw block, a classic block →
`E_MIXED_ALPHABET`. Raw bytes do not count: `~~a+b/c-d_e` in profile T has
`alphabet_seen = none`.

### TV8 — what may follow a `~`

`~` → `E_TRAILING_TILDE`. `~~` → an empty raw block, valid, zero bytes.
`~A` → `E_RESERVED`. `~=` → `E_CHARSET`. `YW~x` → `E_CHARSET`, because a `~`
stands in the middle of a base64 block.

### TV9–TV10 — padding

```
YWxpY2U=     -> "alice",  padding_seen
YWxpY2Uu     -> "alice.", no padding
YWxp==       -> E_PADDING
YWxpY2U==    -> E_PADDING
```

An `=` at the end of the 64th character of a base64 block followed by another
block is `E_CHARSET`. TV10, profile T: `~~a=b=` is a raw tail carrying four
bytes, two of them `=`; in profile U it is `E_PROFILE`.

### TV11 — error cases

| Stream | Code |
|-------|------|
| `abcde` | `E_ALIGN` |
| `~` | `E_TRAILING_TILDE` |
| `~Aabc` | `E_RESERVED` |
| `~~a b` | `E_PROFILE` (profile U) |
| `YWxp==` | `E_PADDING` |
| `YWxpY2V` | `E_NONZERO_TAIL` |
| `YW~x` | `E_CHARSET` |
| `PDw_Pz8+Pg` | `E_MIXED_ALPHABET` |

### TV12 — the tail

A last block follows §9.1: raw from four bytes on, base64 below that, ties to
the raw form. After a full raw block:

| Tail | Stream of the tail |
|---|---|
| — | — |
| `a` | `YQ` |
| `abc` | `YWJj` |
| `abcd` | `~~abcd` |
| `a b` | `YSBi` |
| `a bcd` | `YSBiY2Q` |

## 16. Conformance

§16.1 to §16.3 are what an implementation MUST evidence to count as
conforming. §16.4 to §16.7 are supplementary work on this implementation and
are not normative.

### 16.1 Round trip

**`decode(encode(x)) == x`** for both profiles, over a fuzzing corpus.

### 16.2 Reading base64

**`decode(base64(x)) == x`** and **`decode(base64url(x)) == x`** for all
canonical inputs, padded and unpadded — by differential fuzzing against the
standard base64 library of the language in question. Expected divergences
(`E_NONZERO_TAIL`, §1.1) belong in the corpus as such.

### 16.3 Two implementations

**`encode(x, profile)` byte-identical across two independent
implementations**, over the whole vector set.

**Discharged, with a named gap.** `rust/` and `conformance/reference.py`, the
second written from this document and without a line of shared code. They
agree on all 308 vector pairs, on fifteen error cases, and on a 262923-byte
input character for character (`conformance/test_large.py`). The gap: the same
author.

### 16.4 Measurement

Corpus density and throughput over binary2textbench (§12, §13) — **done**, the
numbers are there. Every measured number in this document comes from that
corpus: 69 corpus samples and 55 short values for density and time, 101
samples for the sample of §9.6.

### 16.5 Container test with real parsers

**Done for Python's parsers**, `conformance/test_containers.py`. Profile U
passes through URL, cookie, JSON, filename and log line unchanged; profile T
needs percent-encoding in a URL and contains the space. This is the weaker,
empirical counterpart to §7.1, which proves the cookie case from the ABNF.

### 16.6 API shape

Per target language: `encode` / `decode` analogous to that language's
`base64`; additionally `decode_url_strict` and `encode_base64url`, and nothing
else. Rust is included; `python/` is a PyO3 binding over it. A binding is
explicitly **not** a second implementation in the sense of §16.3.

### 16.7 Vector set

`docs/vectors.json` carries 173 vectors. They cover the block boundary at 48,
the tails from 1 to 6 bytes where the raw form takes over, and blocks one byte
short of the raw form.

## 17. Candidates for extension (not part of v0.4)

1. **A third block form** carrying a mixed block partly in the clear. `~`
   followed by an alphabet character is reserved for it (§6), and
   `docs/history/spec-v0.4-maske.de.md` describes the dropped design: a mask
   with one bit per byte. It makes mixed text readable and costs three times
   base64's time. Whoever reintroduces it has to show that it goes without
   that price — a vectorised compress operation would be the way — and needs a
   new version number.
2. **Random access.** Block boundaries lie at fixed input offsets but at
   variable output offsets. An index of block starts, outside the stream,
   gives O(1) access.
3. **Profile negotiation.** In principle not derivable from the stream (§7.2).
4. **A different block size.** 48 is justified (§4), not proved. A larger
   block size pushes the raw form towards 75 % size while also tipping a whole
   block to base64 more often; where the optimum lies is a question for
   measurement. Whoever changes the number changes the version number.
5. **Choosing the vector width at runtime.** Not a format question and not a
   gap in the code: the check of §13.1 already vectorises, on the baseline
   target `x86-64` at 16 bytes per operation. Building with `-C
   target-cpu=native` gives 32 or 64 and halves the encoding surcharge —
   today, without `unsafe` and without a code change. What is missing is the
   same **without a build flag**, that is, runtime detection with several
   variants of the same function. There are two ways to do that, and both are
   closed today: `#[target_feature]` requires `unsafe`, which §14 rules out,
   and `std::simd` is not stable (checked on rustc 1.94.1, tracking issue
   rust-lang/rust#86656). Once `std::simd` is stable it is a few lines that
   move not one byte of the output.
