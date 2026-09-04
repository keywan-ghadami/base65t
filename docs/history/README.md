# How it got here

Nothing here is meant to be implemented against. The current state is in
`docs/spec-v0.4.md`; this folder answers the other question — **why does it say
that** — and it answers it because the interesting decisions all went against
the obvious option.

Anyone who wants to use the format does not need this folder. Anyone who wants
to develop it further, or implement it a second time, finds the reasoning here
and above all the measurements that forced individual sentences of the
specification.

**The format was called `base65t` until v0.4.** The old name counted the
characters the encoder is built from — base64url's 64 plus the marker `~` —
and left out `.`, which the alphabet also contains and which decides whether
the passthrough fires at all. Readers met "plus one character" and then a
second one, and the arithmetic had to be explained every time it came up. The
current name counts the output alphabet instead: 66 characters, RFC 3986
*unreserved*, which is the set a caller checks a container against and the one
thing about the format they cannot avoid knowing. Which of the 66 plays which
role inside the encoder is §7's business, not the name's. **The documents in
this folder keep the old name**, as they were written; nothing here is
normative and a record that has been edited to agree with a later decision is
no longer a record.

**The documents below are in German, as they were written.** They are records;
translating them would make them something other than what was decided at the
time. The current specification, the README and the code are English.

## The documents

| File | What is in it |
|---|---|
| `spec-v0.1.de.md` | The first revision. Five presets, a framed mode, three profiles, a greedy encoder as a permitted alternative. Wholly superseded, but every later section number comes from here |
| `errata-v0.1.de.md` | Ten decisions (E1–E10) that fell due while implementing v0.1. E1 is the finding that §11.1 contained two mutually contradictory definitions of the canonical form |
| `spec-v0.2.de.md` | v0.1 + errata, plus the linear rule and `dense-fast`. The intermediate state the performance work was measured against |
| `spec-v0.4-segmente.de.md` | The segment format with **one** encoder instead of five, decided at the head, programmed exactly, windowed. It carried the number v0.4 before being withdrawn; its §13.3 is the measurement that toppled it |
| `spec-v0.4-maske.de.md` | The block format with a **third** block form: a mask with one bit per byte, leaving the admitted bytes of a mixed block in the clear. Also withdrawn; its §13.1 is the measurement that toppled it |
| `FINDINGS.md` | What implementing found: contradictions, search spaces too narrow, numbers that were wrong. Chronological, not edited |
| `PREREGISTRATION.md` | The sweet-spot measurement, fixed **before** it ran, so that the threshold `L_min = 11` is not the result of an evaluation chosen to fit afterwards |

## What v0.4 changed, section by section

This table used to stand at the head of the specification. It is here now,
because the specification is a document for someone implementing the format
and not for someone tracking how it got here. Section numbers are stable
across revisions, so each row points at the same number in both documents.

| § | Change |
|---|---|
| 4 | **Blocks instead of segments.** Two fixed-length block forms; no lengths in the stream |
| 6 | Was the literal segment with its length header; is now the reserved form |
| 8, 9.2, 9.5, 10.3 | Framed mode, the segmentation program, its windowing and closed form, and the segment switch rate. All gone; the numbers carry no content in v0.4 |
| 9 | The encoder is a mapping per block, without search and without state. §9.6 remains, but samples its own decision instead of the entropy |
| 10 | The decoder knows a block's length before it reads it |
| 10.4 | `E_RESERVED_LEN` and `E_TRUNCATED` are gone, `E_RESERVED` is added |
| 11 | Canonicity follows from the mapping; the ordering `B < L < S` is gone, because there is no tie left to break |
| 7, 12 | One alphabet, and density as size: v0.4 reaches 99.99 % over 69 samples. The segment format reached 98.57 %, the mask format 98.65 % — so v0.4 is worse on large files and equally good on short values. The two profiles it briefly had are their own section below |
| 13 | Measured afresh, and this is what the size was traded for. As time, encoding `dickens`: the segment format 1137 %, the mask format 169 %, v0.4 100 %. Over the 55 short values: 355 % for the segment format, 98 % for the mask format, 77 % for v0.4 |
| 13.5 | Readability, share of bytes standing in the clear in profile U: prose 17 % → 76 % → **0 %**, XML 21 % → 66 % → **0 %**, CSS 54 % → 72 % → **0 %**, JSON 9 % → 15 % → **0 %**, for the segment format, the mask format and v0.4 in that order. This is the price, and it is the largest single thing v0.4 gave up |
| 14 | The decoder no longer parses an attacker-chosen length. The segment format read lengths up to 4158 out of the stream; v0.4 reads none at all |
| 15 | Twelve vectors, new. The inputs of the earlier revisions were carried over, their streams were not |

## And then profile T

v0.4 shipped for a while with two profiles: **U**, the 66 characters of RFC
3986 *unreserved*, and **T**, printable ASCII without `"` and `\` — 93
characters, the space among them. T was inherited from v0.1, where there were
three, and it was carried through every redesign without its reason being
re-examined.

The reason it is gone is not a measurement, it is the argument the measurement
was asked to settle. The head of the specification makes one claim — the
output alphabet is these characters, therefore it goes into a URL, a cookie, a
header, a JSON string, a filename and a log field — and every container
statement in the document rests on it. With two profiles that claim needs an
"except when", and a guarantee with an exception is not one. The target
alphabet is the product; it cannot be one thing sometimes.

The question that surfaced it was whether the space belonged in T at all,
since the space is the one character that costs T a container the rest of its
alphabet passes. The measurement (`binary2textbench`, `--example tspace`, 118
samples) answered something larger than it was asked:

| | size | cleartext share of all input bytes |
|---|--:|--:|
| profile T | 97.50 % | 11.5 % |
| profile T without the space | 99.96 % | 0.2 % |

Twenty-four files were affected and **every one went to exactly 100.0 %**,
without exception. Text with punctuation is text with spaces, and 48
consecutive printable non-space characters effectively do not occur in it. So
T without the space was not a narrower T, it was nothing at all — and T *with*
the space was the thing breaking the one-alphabet claim. There was no version
of T that was both safe and worth having.

What it cost to drop it, as size against base64: a log line 78 % → 100 %, a
SQL statement 78 % → 100 %, `xml` 90 % → 100 %, `dickens` 95 % → 100 %.
Readability of prose in the encoded stream, 24 % → 0 %. Summed over the
corpus, 97.50 % → 99.99 %. That is a real loss and it is not softened
anywhere: base65t now helps values that are already URL-shaped, and nothing
else.

What it bought: one sentence that holds without qualification, an `encode`
that takes no parameter at all, and the disappearance of the only row that
traded size against time (`dickens` under T cost 18 % more encoding time for
4.9 % of size). §17.3 of the specification is where a wider alphabet would
have to start if anyone wants it back.

## What happened between v0.2 and v0.4

There is no `spec-v0.3.de.md`. v0.3 was a state in the code, not a document:
the linear rule, the parallelisation, the `dense-fast` preset, the vectorised
base64 loop. What survived of it is in v0.4; what did not survive is in the
commit history and summarised here, because four dropped ideas say more about
the format than the four that stayed.

**Dropped: `legible`.** A preset choosing the more readable segmentation on a
length tie. The tie-break needed a second cost component, and comparing those
lexicographically cost the program of §9.2 between 60 and 190 % more time — in
*every* preset, including the four that never asked for it. A feature only one
caller wanted sent the bill to all of them.

**Dropped: the framed mode.** It was the one place §9.4 could not cover, and a
guarantee with an exception is a guarantee nobody quotes. The price would have
been five characters per 64 KiB; the return was random access nobody had asked
for. `~A` stays reserved so a later revision can put the question again.

**Dropped: profile B.** A profile in which a literal may carry any octet. With
it the output is no longer text, and "the output is text" is the sentence that
makes anyone look at this format at all. A profile that puts a footnote on the
core sentence costs more than it returns.

**Dropped: the presets themselves.** Five, then six, then none. The reason is
in §0.1 of v0.4: whoever has to choose between a dense and a fast encoder has
to learn what those words mean first, and whoever is unsure reaches for base64.
The choice is now made by §9.6, from the first 64 blocks.

**Dropped: the linear rule.** It was the answer to "the exact DP is too slow
for large data" and cost 0.22 points of size. The better answer was not to look
at large data *at all* where nothing is to be found — and to keep the exact DP
where something is. The factor by which the DP is slower was reported wrongly
twice along the way (first "12×", then measured 21–63×); the correction is in
`FINDINGS.md`.

## And then the segment format itself

With one encoder instead of five the segment format was coherent, and it was
measured: faster than base64 on short values that were legal throughout, byte
for byte base64 on compressed data. In between, on mixed text, the exact
program cost six to eleven times base64's time for zero to one and a half
points of size — and every attempt to fix that was one more mechanism: a head
decision, a windowing, a closed form, a sample. Five mechanisms to make one
idea affordable.

The question that ended it was not "how do we make the program faster" but "why
do we have to look at all". A block of fixed length, a fixed mapping, no
search, no state, no lengths in the stream.

## And then the mask

The first block revision had three forms. The third was a mask with one bit per
byte, leaving the admitted bytes of a mixed block standing in the clear and
appending the rest as base64. It was the most elegant idea in this project: it
took English prose from 17 % readable to 76 %, at a smaller output
than the segment format, and it cost not a single search.

It is dropped anyway, and the reason is not a measurement error but the
positioning. A mask block cost, measured, **three times** base64's time in both
directions, because it does three times as much: write or read the mask,
separate 48 bytes onto two destinations or reassemble them from two, and the
rest as base64. It was optimised to the limit of what is possible scalar —
branch-free, stack buffers, tables over groups of eight, table popcount — and
stayed at three times.

"Three times slower on my JSON blobs" is the sentence that tips the decision
for this format. It does not live on its gain, which is small, but on the
decision to use it costing **nothing**. Readable mixed text is a fine thing,
but it is not the reason anyone picks this format up, and it is available one
door further at Base85N.

`~` followed by an alphabet character stays reserved and is an error today. A
later revision can bring the mask back if it shows that it goes without that
price; a decoder of today then fails loudly instead of reading wrongly.

## What came of it

Two block forms, one question per block, one alphabet: `docs/spec-v0.4.md`.
Faster than base64 on short values in both directions (65 % and 84 % of the
time), and on large files at base64's time — 98 to 103 % where the output is
the same size, and faster where it is smaller. The price is in §13.5 of the
current revision: text with punctuation is not readable at all, and on large
documents the format gains nothing.

**Kept from the segment era:** the base64 loop that writes into a slice of
known length, and the Rule A insight that alphabet consistency is a search and
not a decoding. The per-byte membership test survived too, but inverted: it
used to build a 64-bit mask and compare it; it now answers one question and
breaks off at the first byte that settles it, which is where most of the
encoder's speed comes from (§13.1).
