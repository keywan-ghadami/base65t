# Base65t — Specification v0.4

**Status:** current. **Wire format: not stable** — nothing promises that v0.5
keeps these blocks. What is stable is the contract, not the bytes: bytes in,
printable ASCII out, never longer than base64, and every base64 stream reads
back. Store the version number alongside any stream you keep.

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

Whatever goes in, `encode` writes these 66 characters and no others:

```
A–Z   a–z   0–9   -   .   _   ~
```

That is exactly RFC 3986's *unreserved* set. Every one of the 66 is reachable
and nothing outside it ever appears — `=` included, because the encoder
produces no padding (§5.1). It does not depend on the data, and there is no
parameter, mode or profile that changes it. That single sentence is the whole
reason the format is easy to place:

* **URL query, path segment** — *unreserved* is what "needs no percent-encoding"
  means, by definition of RFC 3986.
* **Cookie value** — all 66 characters are `cookie-octet`, proved from RFC
  6265's ABNF (§7.1).
* **HTTP header value** — no separator, no whitespace.
* **JSON string** — nothing to escape.
* **Filename, log field, `key=value`** — no space and no delimiter.
* **Pasted unquoted into a shell** — no metacharacter, no glob, and no
  expansion. `~` is the one to check, since every raw block opens with two of
  them: a *double* tilde is never a valid tilde-prefix, and the encoder never
  writes a single one (§6), so nothing expands. Measured in bash, dash and sh,
  over every stream shape, in four placements (§16.5).

Checked against Python's own parsers, and against classic base64 as a control,
in `conformance/test_containers.py` (§16.5); and against the encoder itself in
`the_output_alphabet_is_exactly_unreserved`, which builds the set from what it
emits and compares it both ways.

`encode_base64url` (§9.3) — the way out of the format for a caller who wants
none of it — writes a subset of the same 66 characters.

**There is no wider alphabet to opt into.** An earlier revision offered one,
and §7 says why it is gone: a guarantee that holds "except when" is not one.

## What it guarantees, so that the decision is easy

This format is for the caller who is unsure. They know base64, base64 is never
wrong, and anything they would have to weigh first is a reason not to bother.
The aim is that there is as little to weigh as possible. Two of the three
dimensions a caller could worry about are guaranteed never worse than base64.
The third, time, is not — and the case where base64 wins is named below rather
than left to be discovered.

| Dimension | Against base64 | Standing |
|---|---|---|
| **Size** | never larger, `len(encode(x)) ≤ ceil(4·len(x)/3)` | **guaranteed**, per input, no exception. The proof is one sentence: a raw block costs 50 characters where base64 costs 64, and every other block *is* base64 (§9.4) |
| **Readability** | a raw block stands in the clear; every other block *is* base64 | **guaranteed** by construction — the format never makes readable input less readable (§13.5) |
| **Time** | faster where the output is smaller; at base64's time on input with no raw blocks at all; **slower on one shape**, named in §13.3 | **not guaranteed, and not always equal or better.** Measured 47 % to 127 % encoding. This is the one dimension where base64 can win (§13.3) |
| **Alphabet** | the output is always the same 66 characters | **guaranteed**, and it is what the other three are stated against (§7) |

**Why time is not guaranteed.** Size and readability follow from the block
rule alone, which is why they can be proved. Time depends on how many blocks
the check rejects after being turned on, and that is a property of the input
the format cannot see in advance. §9.6 removes the check where it would never
pay; it cannot remove it where it pays a little.

Two further properties follow from the encoder being a mapping rather than a
search:

**Byte equality: guaranteed.** `encode(x)` determines its output from 48 bytes
per block. There is nothing two conforming encoders could hold
different opinions about (§11).

**The wire format: not guaranteed.** See the status above.

> **Requirements language.** The key words "MUST", "MUST NOT", "REQUIRED",
> "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT
> RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted
> as described in BCP 14 (RFC 2119, RFC 8174) when, and only when, they appear
> in all capitals, as shown here.
>
> Numbers not marked *exact* are measurements on the corpus named in §16.4.
>
> Section numbers are stable across revisions of this specification, so §8,
> §9.2, §9.5 and §10.3 carry no content here.

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

## 0. Positioning (non-normative)

### 0.1 One format, one encoder

The caller this format is for is the one who is unsure, and the head of this
document says why that leaves nothing to weigh. What follows from it is the
shape of the API: exactly one parameterless `encode` function that takes bytes
and returns bytes. No presets, no modes, no thresholds, no profiles. The
encoder is explained in one sentence: **48 bytes of text stay text, everything
else is base64.**

One thing exists beside it, and it is not a mode of the encoding:
**`encode_base64url`** (§9.3) is the way *out* of the format, for a caller
carrying a secret who wants none of it standing in the clear (§14), and for
one who is only allowed to speak base64url.

| Use | What matters |
|---------|-------------------|
| URL query | URL safety without percent-encoding (§7) |
| Cookie value | `cookie-octet` conformance (§7.1) |
| HTTP header | ASCII, no separators, no whitespace |
| JSON string, log field | nothing to escape, nothing to quote |
| Cache or dedup key | byte equality (§11) |
| Token containing a secret | `encode_base64url`, no cleartext leaks (§14) |

### 0.2 What base65t *is*, in one sentence

> Base64url in blocks of 48 bytes, where a block MAY carry its bytes raw and a
> 65th character says which blocks those are.

### 0.3 What "one decoder for everything" means exactly

> A conforming `decode()` takes an octet stream and needs **no parameter at
> all**. Alphabet variant (§5.2) and padding (§5.3) are
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
base64url, the output is a fixed 66-character alphabet that every common
container already accepts, and it is never larger. On time it is usually
faster and, on one shape, slower — §13.3 names it, and it is the only thing
left to weigh.

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
9. **Not appreciably slower than base64**, in both directions (§13). This is
   the one goal not met in every case: §13.3 names the shape that costs and
   §17.5 the rule that would fix it.

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
* **Base65t produces an octet stream.** Every octet is printable ASCII, and
  one of the 66 characters of §7.
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
single byte outside the alphabet suffices.

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
stream up front. A raw tail runs to the end of the stream, so a `=` there is
part of the raw block and makes it invalid — `~~abcd=` is `E_PROFILE`. Strip
the `=` first and the same stream decodes cleanly instead (TV10). An error and
an acceptance are not the same answer.

### 5.4 Alphabet consistency (Rule A, normative)

> **Rule A.** A stream MUST NOT mix the two alphabet variants. If the set of
> **alphabet characters** contains both a character from {`+`,`/`} and one
> from {`-`,`_`} → `E_MIXED_ALPHABET`. The rule holds over the whole stream,
> and so across the raw blocks that sit between two base64 blocks.

Rule A concerns alphabet characters only. Raw bytes do not count — `-` and
`_` are both admitted raw and are the URL variant's own two characters, so
nearly every raw block contains one. Scanning the whole stream rejects valid
streams (TV7).

### 5.5 Reporting and the strict variant (normative)

An implementation MUST make both of these reachable for any stream it
decodes:

```
alphabet_seen : { none, url, classic }
padding_seen  : bool
```

**On a named entry point, not necessarily on `decode`'s return.** The reason
for the requirement is that permissiveness which cannot be inspected cannot be
validated (§14) — that is satisfied by an entry point a caller can reach, and
it is not satisfied by discarding the information. The reference
implementation returns the bytes from `decode` and the pair from
`decode_detailed`, so that `decode` has the shape a caller replacing a base64
decoder already has (§9.3).

In addition, `decode_url_strict` MUST be offered (it rejects `classic` with
`E_NON_URL_ALPHABET`).

## 6. The reserved form

```
"~" <alphabet character> ...       # reserved, E_RESERVED
```

No encoder of this revision writes `~` followed by an alphabet character, and
a decoder MUST reject it. The two characters cost nothing today and reserve
the code space for a third block form, which §17.1 describes: `~`, eight mask
characters carrying one bit per byte, the admitted bytes in the clear, then
base64 of the rest.

That form is **not** part of v0.4, and the reason is §0.1. It makes a mixed
block two-thirds readable and costs three times base64's time for it, and this
format lives on the decision to use it costing nothing. "Three times slower on
my JSON blobs" is a sentence that tips exactly that decision, and readable
mixed text is not what the format advertises. The reservation exists so that a
revision which finds a cheaper way can add it without a decoder of today
misreading its streams.

`~` followed by something that is neither `~` nor an alphabet character is not
a reserved stream but a broken one: `E_CHARSET`.

## 7. The alphabet

> **Rule.** A byte may stand raw if and only if it is in RFC 3986's
> *unreserved* set: `A–Z`, `a–z`, `0–9`, `-`, `.`, `_`, `~`. 66 characters.

**Two numbers, and they are not the same number.** The format's *radix* — the
symbols that carry encoded data — is base64url's 64 plus `~`, which is where
the name comes from and why §0.2 calls `~` the 65th character. The set above
is the *byte values a stream can contain*, which is 66, because a raw block
passes text through and `.` is text. `.` and `~` never appear in a base64
block; every other character of the 66 does, in both roles. Container safety
is a statement about the 66 (a parser sees bytes, not roles), and the name is
a statement about the 65.

There is no second set and no parameter that selects one. That is the format's
central property, not a simplification of it: the base64 alphabet is a subset
of these 66 (§3), so **every character of every stream is one of them** — the
base64 blocks, the `~~` markers and the raw bytes alike. One sentence covers
the whole output, which is what §0.3 and the head of this document rest on.

A byte outside the set costs its whole block: the block becomes base64. That
is the coarseness this format traded for its speed, and §13.4 quantifies it.

**What is deliberately not admitted.** The space and the punctuation of
ordinary prose — `,` `;` `:` `!` `?` `'` `"` `(` `)` `/` `+` `=` `&` `%` `@`
and the rest. Each of them is the character that breaks some container: `=`
and `&` a query string, `;` a cookie, the space a whitespace-separated log
line, `"` and `\` a JSON string, `/` a path. Admitting any of them would buy
readability on text and spend the one sentence above, and the sentence is
worth more than the readability. §13.5 says what that costs: on a document
with punctuation, nothing stands in the clear.

An earlier revision offered a second, wider profile that admitted printable
ASCII, and it is withdrawn for exactly this reason — the guarantee cannot hold
"except when". `docs/history/README.md` has what it scored and what it cost.

### 7.1 Cookie conformance (proved, not measured)

RFC 6265 §4.1.1 defines:

```
cookie-octet = %x21 / %x23-2B / %x2D-3A / %x3C-5B / %x5D-7E
```

The 66 characters — 62 alphanumerics plus `-` (0x2D), `.` (0x2E), `_` (0x5F),
`~` (0x7E) — lie entirely within those ranges. All 66 checked, no exception.
The statement follows from the ABNF and is therefore **provable, not
empirical**. Whether real cookie parsers hold to the ABNF is the weaker,
empirical question; Python's `http.cookies` does (§16.5).

### 7.2 Unused

It held the argument for why the profile was a parameter. There is no
parameter.

## 8. Unused

It held the framed mode. Random access is §17.2.

## 9. Encoder

### 9.0 Principle (normative)

> For every block the encoder checks whether the alphabet of §7 admits
> **every** one of its bytes. If it does and the block has at least four bytes, it writes
> `~~` and the bytes; otherwise it writes the block as base64.

That is the whole rule. It is a mapping from 48 bytes to an output, without
search, without state, without a parameter, and without a tie an ordering
would have to break. A test vector therefore checks bytes rather than lengths, and
`docs/vectors.json` does so over 154 vectors.

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

**All or nothing.** A single byte outside the alphabet costs its whole block.
That is coarse, and it is the trade this format makes. A finer encoding is
possible — §17.1 describes one, and it costs three times base64's time, which
is why the code space for it is reserved (§6) rather than used. What the
coarseness means on real data is in §13.4: short values consisting entirely of
text reach 78 % size; large documents with punctuation gain **nothing**.

### 9.2 Unused

### 9.3 Entry points

| Function | What it does |
|---|---|
| `encode(x)` | the encoding |
| `encode_base64url(x)` | base64url and nothing else (§14) |
| `decode(s)` | the decoding, returning the bytes (§10.2) |
| `decode_url_strict(s)` | the same, with the alphabet fixed (§5.5) |
| `decode_detailed(s)` | the decoding plus what the stream chose (§5.5) |

`encode` MUST take the data and nothing else. A library MUST NOT offer a
parameter that changes the alphabet, the block size or the block rule: those
are the format, and a caller who can change them has to understand them
first (§0.1).

**These SHOULD have the shape of the host language's base64 library**, down
to argument and return types, so that a call site changes its import and
nothing else. In Rust that means `encode` returns a `String` and `decode` a
`Vec<u8>`; in Python, `bytes` from both. This is not decoration: §1.1 makes
the decoder side of a migration free, and a caller who has to rewrite call
sites to take it will not.

**The two sides are not equally safe to swap, and an implementation SHOULD say
so where a caller will read it.** A base65t decoder reads every canonical
base64 and base64url stream (§5.2, §5.3), so replacing a *decoder* changes
nothing observable. Replacing an *encoder* starts emitting `~`, which a base64
decoder rejects. Decoders first, encoders once every reader is one. Saying it
once is the requirement; making the call awkward is not.

### 9.4 Never-worse guarantee (normative)

```
len(encode(x)) <= ceil(4 * len(x) / 3)
```

**Per input, not on average, without exception.** Proof: a raw block costs
`m + 2 ≤ ceil(4m/3)` (§9.1), every other block *is* base64, and base64 blocks
tile, so the sum of the base64 forms is exactly `ceil(4n/3)`. ∎

**Sharper:** where no block goes raw, the encoder writes not merely as many
characters as base64url but **the same bytes**.

**Scope.** The length of the encoded stream in octets, not transport or
container overhead.

### 9.5 Unused

### 9.6 The sample (normative)

Asking §9.0's question per block costs time, and on an input where the answer
is always "no" it is the only time this format spends beyond base64. Such
inputs are not only binary: English prose has a space in every block, so no
block goes raw, and for this format it is as binary as a JPEG.

> **Rule.** Before encoding, the encoder applies §9.0 to the first **64
> blocks**. If none of them yields the raw form, the whole stream MUST be
> written as base64url. Otherwise §9.0 applies to every block.

**It is the same check, once up front.** No magic numbers, no entropy, no
logarithm two implementations would have to agree on — the sample measures the
decision itself and not something correlated with it.

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
inputs. Implementing the rule literally is equally conforming
— the Python reference does exactly that, and the two implementations agree.

**Why 64 blocks.** Two reasons, both of which count. Measured, there is no
knee to find: from 32 blocks on the sample costs nothing at all, so any size
from there up is free and 64 is the first power of two past it with room to
spare. And 64 blocks are **3072
bytes** — longer than every value §0.1 names. For a URL query, a cookie value,
a header or a cache key the sample is therefore not a sample at all but the
whole input, and it can give up nothing there. Both reasons point the same
way, which is why the number is not a threshold anyone has to tune.

**What it costs over the corpus**, against "always check"
(`binary2textbench`, `--example sample`, 101 samples), as size:

| sample size | size | files written as pure base64 | files that gave anything up |
|---|--:|--:|--:|
| always check | 99.99 % | — | — |
| k = 8 | 100.00 % | 69 of 101 | 2 |
| k = 16 | 100.00 % | 69 of 101 | 2 |
| **k = 64** | **99.99 %** | **67 of 101** | **0** |
| k = 128 | 99.99 % | 67 of 101 | 0 |

At the chosen size the sample costs nothing: not one file in the corpus is
encoded larger because of it. In exchange, two thirds of all files are written
byte for byte as base64, in base64's time.

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
        check: every byte stream[pos+2..end] admitted by §7  else E_PROFILE
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
decode(stream)                    -> bytes
decode_url_strict(stream)         -> bytes, '+' and '/' are E_NON_URL_ALPHABET
decode_detailed(stream)           -> bytes + alphabet_seen + padding_seen (§5.5)
```

### 10.3 Unused

### 10.4 Error cases

| Code | Condition |
|------|-----------|
| `E_TRAILING_TILDE` | the stream ends with a single `~` |
| `E_RESERVED` | `~` followed by an alphabet character (§6) |
| `E_PROFILE` | a raw byte outside the alphabet of §7 |
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
limit for individual blocks**: there is no single allocation an attacker can
size. The number of blocks is unbounded; implementations SHOULD offer limits
on total size and running time.

## 11. Canonicity and signatures

**The encoder is a mapping** (§9.0): per block, 48 bytes determine the output,
and the blocks are independent. Two conforming encoders write the same bytes
for the same input. That is enough
for cache keys, dedup keys and content addresses, where the same side produces
and compares.

The *format* is nevertheless not canonical, because the **decoder accepts
forms no encoder writes**: the classic alphabet (§5.2), padding (§5.3), and a
base64 block where a raw block would be shorter. A third party can rewrite the same stream without changing the
decoded bytes.

> **Rule:** never sign, hash or compare the output of `encode`. Sign the
> **decoded bytes**. `decode(encode(x)) == x` always holds.

## 12. Density

**Exact**, from §9.1:

Characters per input byte; less is better.

| Input | Base64 | **Base65t** |
|---------|--------|-------------|
| A block with one byte outside the alphabet | 1.333 | **1.333** — the same bytes |
| A block wholly inside it | 1.333 | **1.0417** — `50/48`, the block stands raw |

There is nothing in between: a block is one or the other. What a file achieves
therefore depends only on how many of its 48-byte blocks consist entirely of
admitted bytes.

**Measured** over the binary2textbench corpus (69 samples, `--example gain`),
size against unpadded base64:

| | |
|---|--:|
| Sum over all samples | 99.99 % |
| Samples better than 95 % | 43 % |
| Indistinguishable from base64 (≥ 99.9 %) | 55 % |

**The sum line is honest and misleading at once**, because it is weighted by
bytes and the corpus is dominated by megabyte files, which gain nothing. The
distribution is not a gradient but two populations, and the interesting one is
the small one — the values §0.1 is about:

| Sample | Bytes | Size |
|---|--:|--:|
| Git commit ID | 40 | **77.8 %** |
| Session ID, 40 alnum | 40 | **77.8 %** |
| SHA-512 digest, hex | 128 | **78.4 %** |
| Two UUIDs | 73 | **78.6 %** |
| JWT, three segments | 155 | **78.7 %** |
| Prose, XML, JSON, every megabyte file | | 100.0 % |

Nothing of the sum line is the sample of §9.6: at 64 blocks it costs no file
anything (§9.6). The gap to 78 % on the large files is entirely the
all-or-nothing block of §9.1.

## 13. Performance

Everything single-threaded. Where a figure compares against base64, the
comparison is this crate's own `encode_base64url` and its decoder on a pure
base64 stream (see the head of this document), and the statistic is the
**median of paired ratios over 21 rounds**: the two sides are timed alternately
within a round, so a runner that drifts moves both together and cancels.

### 13.1 What the check costs, and where it is not incurred

A raw block is a `memcpy` in both directions, a base64 block is base64. The
only work this format has beyond base64 is the question per block: does the
alphabet of §7 admit **every** byte?

It is built as cheaply as it can be without vector intrinsics — it breaks off
at the first byte that settles it, tests a necessary condition up front with a
single operation per 32 bytes, and its per-byte test is arithmetic rather than
a table lookup, because a gather does not vectorise and six comparisons do.
As time, on four megabytes:

| Input, per 48-byte block | the check alone | encoding overall |
|---|--:|--:|
| wholly admitted (raw) | 36 % | **46 %** |
| binary | 7 % | 100 % |
| text, rejecting byte at the end of the block | 36 % | 100 % |

**The second column is where §9.6 shows.** Rows two and three are exactly the
inputs the sample turns down: no block is checked, the stream is written as
base64url, and the encoding costs base64's time to the point of measurement.
The check's own cost is real — the first column — but it is only paid on input
that pays it back, which is row one.

When **decoding** this work never arises: the form is in the first character.

### 13.2 The throughput criterion

> **Throughput is a goal, size is a guarantee.** A change MUST NOT touch the
> guarantee of §9.4 or the byte equality of §11. Within that bound it SHOULD
> improve throughput.

An encoder or decoder is not non-conforming because it is slower than base64.
It is non-conforming if it writes the wrong bytes.

### 13.3 Large files

| File | Bytes | Size | Encode, time | Decode, time |
|---|--:|--:|--:|--:|
| generated, wholly admitted | 4 000 000 | **78.1 %** | **49 %** | **43 %** |
| `manifest.json` | 21 397 | 99.0 % | **122 %** | 101 % |
| `osdb` | 10 085 684 | 99.9 % | **127 %** | 99 % |
| `dickens` (prose) | 10 192 446 | 100.0 % | 102 % | 99 % |
| `xml` | 5 345 280 | 100.0 % | 102 % | 99 % |
| `mozilla` (binary) | 51 220 480 | 100.0 % | 100 % | 100 % |
| random bytes | 262 144 | 100.0 % | 100 % | 100 % |

Three shapes, and the middle one is the one to know about.

**Every block raw** — the output is 78 % of base64's size and takes half its
time, because a `memcpy` is less work than a base64 loop.

**No block raw** — the sample of §9.6 turns the check off, the output *is*
base64url byte for byte (§9.4), and the time is base64's: 100 to 102 %, which
is the runner's spread rather than the format's. Prose, XML and binary are all
here.

**A few blocks raw** — and this is where base64 wins. The sample sees a raw
block, so the check runs on every block; most of them turn out to be base64
and paid for the check for nothing. `manifest.json` spends **22 % more
encoding time for 1 % of size**, `osdb` **27 % for 0.1 %**. Constructed, the
worst case is one raw block at the head of a long prose file: 122 % of the
time for 100.0 % of the size, nothing gained at all.

**So the honest range is 47 % to 127 % encoding**, and the shape that costs is
a stream whose head is unlike its body. Decoding never has this problem — the
form is in the first character — and stays at 99 to 101 % throughout.

A rule that turned the check off again after enough consecutive base64 blocks
would fix it, and would need a constant that §0.1 does not want. §17.5 leaves
it open.

### 13.4 Short values

The 55 short samples, size against `ceil(4n/3)`, and — the one exception to
the preamble above — time against **the bench's** base64 rather than this
crate's, because that is the denominator every other codec in that report uses
(`--example short`). The bench's base64 pads and validates UTF-8, so these
figures flatter base65t by a little; §13.3's do not.

| Sample | Bytes | Form | Size | Encode, time | Decode, time |
|---|--:|---|--:|--:|--:|
| SHA-512 digest, hex | 128 | raw | 78 % | **38 %** | **33 %** |
| Git commit ID | 40 | raw | 78 % | **43 %** | **60 %** |
| Two UUIDs | 73 | raw | 79 % | **42 %** | **54 %** |
| UUID v4 | 36 | raw | 79 % | **50 %** | **77 %** |
| Credit card number | 16 | raw | 82 % | **69 %** | **98 %** |
| SQL statement | 118 | base64 | 100 % | **79 %** | **92 %** |
| Log line | 93 | base64 | 100 % | **74 %** | **91 %** |
| IPv6 address | 28 | base64 | 100 % | **89 %** | **99 %** |
| 64 random bytes | 64 | base64 | 100 % | **72 %** | **96 %** |
| 8 random bytes | 8 | base64 | 100 % | **84 %** | 131 % |
| **all 55 samples, as time** | | | | **65 %** | **84 %** |

**On short values base65t is faster than base64, in both directions**, and on
the rows where it saves nothing in size as well. The reason is the work
balance: base64 reads a byte, looks up four characters and writes four — per
three bytes. A raw block reads 48 bytes, checks them with six comparisons per
byte and copies them. Whoever writes less writes faster. What is left above
100 % is decoding the very shortest values, where the measurement is the
allocation and not the codec.

### 13.5 What stays readable

Share of bytes that stand in the stream as they do in the input
(`--example clear`, 102 samples):

| | Files |
|---|--:|
| 100 % readable | 32 |
| nothing readable | 67 |
| in between | 3 (1 %, 1 % and 5 %) |

**Readability is not a gradient, it is a property of the value.** A block goes
raw only if all 48 of its bytes are admitted, so a value made of identifiers,
IDs, digests or hexadecimal comes through entirely, and a document with
punctuation comes through not at all: prose, XML, CSS and JSON are all 0 %.
The three in between are large files with a stretch of identifier-shaped data
somewhere in them.

**That is the price of one alphabet, and it is named rather than softened.** A
wider alphabet would make text with punctuation readable; §7 says why there
isn't one, and `docs/history/README.md` has what a wider one scored. Whoever
needs readable mixed text needs a different format, and §0.5 says which.

## 14. Security

* **The decoder parses no length whatsoever.** There is no length in the
  stream; every one follows from the block form and the block size, so no
  number a sender chose ever reaches an allocator or a loop bound. What
  remains is the same as for base64: the total length of the input.
* **Raw bytes leak structure** — which blocks consist entirely of text is
  visible in the stream, and their content stands in the clear. That is what
  `encode_base64url` is for (§9.3); its output is ordinary base64url.
* **Two auto-detections are two parser-differential surfaces:** alphabet
  (§5.2) and padding (§5.3). Countermeasures: Rule A, Rule P,
  `alphabet_seen` / `padding_seen` and `decode_url_strict` (§5.5). Differential
  fuzzing is mandatory, not optional.
* **No padding oracle** — padding is only validated, never produced.
* **Malleability** is excluded at block level, reduced at alphabet and padding
  level, and **not** excluded against a third party rewriting a raw block as
  a base64 block (§11).
* Decoded output is **untrusted binary**, not text.

## 15. Test vectors

Twelve vectors, each a test in `rust/tests/vectors.rs`. The machine-checkable
set — 154 entries over both entry points — is in `docs/vectors.json`.

### TV1–TV4 — the two forms

| # | Input | Stream | Length | Base64 would be |
|---|---------|-------|-------|-------------|
| TV1 | `alice.jones` | `~~alice.jones` | 13 | 15 |
| TV2 | `DE AD BE EF` + `session-eu-central` | `3q2-73Nlc3Npb24tZXUtY2VudHJhbA` | 30 | 30 |
| TV3 | `sub~alice~jones` | `~~sub~alice~jones` | 17 | 20 |
| TV4 | 100 × `a` | `~~` + 48 `a`, `~~` + 48 `a`, `~~aaaa` | 106 | 134 |

**On TV2.** Four of the 22 bytes are not admitted, so the block is base64, and
the stream is byte for byte `encode_base64url`. This is the all-or-nothing
rule of §9.1 at its most expensive: four bytes cost the other eighteen their
raw form.

**On TV3.** A `~` in a raw block needs nothing, because the block length is
fixed. `hello~Alice` becomes `~~hello~Alice`.

### TV5 — one byte decides the block

```
48 bytes, all admitted:
  the-quick-brown-fox-jumps-over-the-lazy-dog.abcd
  ~~the-quick-brown-fox-jumps-over-the-lazy-dog.abcd    50 characters, raw

The same text with spaces, 49 bytes:
  the quick brown fox jumps over the lazy dog. again
  dGhlIHF1aWNrIGJyb3duIGZveCBqdW1wcyBvdmVyIHRoZSBsYXp5IGRvZy4gYWdhaW4
  67 characters, byte for byte base64url -- the space is not admitted (§7)
```

And it is the byte, not its position: put a space at any of the 48 positions
of the first input and the whole block becomes base64.

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
`E_MIXED_ALPHABET`. Raw bytes do not count: `-` and `_` are the URL variant's
own two characters and are also admitted raw, so `~~a-b_c-d_e` has
`alphabet_seen = none` while `PDw_Pz8-Pg` has `url`. This is the stream a
whole-stream scanner misreads.

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
block is `E_CHARSET`. TV10: `~~abcd=` is `E_PROFILE`, because a raw tail runs
to the end of the stream and `=` is not admitted — while `~~abcd` decodes
cleanly. Stripping the padding first turns the error into an acceptance, which
is why §5.3 forbids it.

### TV11 — error cases

| Stream | Code |
|-------|------|
| `abcde` | `E_ALIGN` |
| `~` | `E_TRAILING_TILDE` |
| `~Aabc` | `E_RESERVED` |
| `~~a b` | `E_PROFILE` |
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

**`decode(encode(x)) == x`**, over a fuzzing corpus.

### 16.2 Reading base64

**`decode(base64(x)) == x`** and **`decode(base64url(x)) == x`** for all
canonical inputs, padded and unpadded — by differential fuzzing against the
standard base64 library of the language in question. Expected divergences
(`E_NONZERO_TAIL`, §1.1) belong in the corpus as such.

### 16.3 Two implementations

**`encode(x)` byte-identical across two independent implementations**, over
the whole vector set.

**Discharged, with a named gap.** `rust/` and `conformance/reference.py`, the
second written from this document and without a line of shared code. They
agree on all 154 vectors, on sixteen error cases, and on a 262923-byte
input character for character (`conformance/test_large.py`). The gap: the same
author.

### 16.4 Measurement

Corpus density and throughput over binary2textbench (§12, §13) — **done**, the
numbers are there. Every measured number in this document comes from that
corpus: 69 corpus samples and 55 short values for density and time, 101 for
the sample of §9.6, and 102 for the readability of §13.5.

### 16.5 Container test with real parsers

**Done for Python's parsers and the shells on the machine**,
`conformance/test_containers.py`. The output passes through URL query, cookie,
JSON string, filename, log line and `key=value` unchanged, and survives being
pasted unquoted into bash, dash and sh in four placements — as a bare word, in
a URL, after `=` in an assignment, and in a `Cookie` header. **Classic base64
is the control** and fails four of those checks on the same data, which is
what shows the alphabet doing the work rather than the output merely being
ASCII. This is the weaker, empirical counterpart to §7.1, which proves the
cookie case from the ABNF.

**One hazard is left, and it is named rather than fixed.** A stream may begin
with `-`, which a *program* may read as an option — `cmd -abc…`. That is argv
parsing and not shell expansion, and base64url has exactly the same property,
so it is not something this format introduced. A caller passing a value as a
positional argument should pass `--` first, as with any base64.

### 16.6 API shape

Per target language, the five entry points of §9.3 and nothing else: `encode`,
`encode_base64url`, `decode`, `decode_url_strict`, `decode_detailed`. §9.3
requires that their argument and return types be the host language's base64
shapes, so a call site changes its import and nothing else.

**Done for Rust**, and checked rather than claimed: `rust/tests/dropin.rs` is
written in the `base64` crate's own call shapes — free functions and the
`Engine` method form — and stops compiling if a signature drifts. `python/` is
a PyO3 binding over the same crate; a binding is explicitly **not** a second
implementation in the sense of §16.3.

### 16.7 Vector set

`docs/vectors.json` carries 154 vectors. They cover the block boundary at 48,
the tails from 1 to 6 bytes where the raw form takes over, and blocks one byte
short of the raw form.

## 17. Candidates for extension (not part of v0.4)

None of the following is implemented, and each would need a new version
number.

### 17.1 A third block form

Carrying a mixed block partly in the clear: `~`, eight mask characters with
one bit per byte, the admitted bytes, then base64 of the rest. `~` followed by
an alphabet character is reserved for it (§6).

A complete design exists and was measured: it takes English prose from 0 %
readable to 76 %, and it costs **three times** base64's time in both
directions, because it does three times as much work per block. It was
optimised to the limit of what is possible without vector intrinsics and
stayed there. `docs/history/spec-v0.4-maske.de.md` has it in full, which is
where to start rather than from scratch.

Whoever reintroduces it has to show that it goes without that price — a
vectorised compress operation would be the way.

### 17.2 Random access

Block boundaries lie at fixed input offsets but at variable output offsets. An
index of block starts, kept outside the stream, gives O(1) access. This is why
the format needs no second mode for it.

### 17.3 A wider alphabet

An alphabet that admitted the space and ordinary punctuation would make text
with punctuation readable — §13.5 says it is 0 % today. A revision that wants
it has to answer the question §7 answers with "no": what the container
statements at the head of this document become when the alphabet is no longer
one set. `docs/history/README.md` has what a wider alphabet scored when it was
tried, and why it was withdrawn.

### 17.4 A different block size

48 is justified (§4), not proved. A larger block size pushes the raw form
towards 75 % size while also tipping a whole block to base64 more often; where
the optimum lies is a question for measurement.

### 17.5 Turning the check off again

§13.3 names the one shape where base64 wins: a stream whose head holds a raw
block and whose body does not, so §9.6 turns the check on and almost every
block then pays it for nothing. Measured, that is 122 to 127 % of base64's
encoding time for one percent of size or less.

A rule that stopped asking again — after some number of consecutive base64
blocks, say — would remove it. It needs a constant, and §0.1 is the reason
none has been added: a number in the specification is a number two
implementations must agree on and a caller may ask about. Whether one can be
justified the way 48 and 64 are (§4, §9.6) is open. Any such rule MUST keep
§9.4, which it does automatically: skipping the check only ever writes base64.

### 17.6 Choosing the vector width at runtime

Not a format question and not a gap in the code: the check of §13.1 already
vectorises, on the baseline target `x86-64` at 16 bytes per operation.
Building with `-C target-cpu=native` gives 32 or 64 and halves the encoding
surcharge — today, without `unsafe` and without a code change.

What is missing is the same **without a build flag**, that is, runtime
detection with several variants of the same function. There are two ways to do
that, and both are closed today: `#[target_feature]` requires `unsafe`, which
§14 rules out, and `std::simd` is not stable (checked on rustc 1.98.1,
tracking issue rust-lang/rust#86656). Once `std::simd` is stable it is a few
lines that move not one byte of the output.
