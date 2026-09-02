# What the reference implementation found

> **Historie.** Der aktuelle Stand ist `docs/spec-v0.4.de.md`. Dieses Dokument
> ist nicht normativ; es liegt hier, weil es trägt, wie es dazu kam.
> `docs/history/README.md` sagt, was zwischen den Fassungen gestrichen wurde
> und warum.

The specification in `docs/spec-v0.1.de.md` is v0.1 final and unchanged. This
file is what came out of implementing it and running the conformance work of
§16 against it: nine places where the text says something the code cannot do,
or does not say enough for two implementations to agree. Each one names the
test that holds it in place, so that whichever way it is decided, the decision
is visible.

**All nine are decided and folded into `docs/spec-v0.2.de.md`.**
`docs/errata-v0.1.de.md` says what holds instead, entry by entry; this file
stays as the record of how each one was found — including the one decision that
was later reversed, at the end. Six of
the nine needed no measurement at all — the specification's own ranking of its
goals settled them. The two that did, and the run that decided them, are in
`PREREGISTRATION.md` and the appendix of the errata.

Nothing here is a bug in the format. Seven of the nine are cases where two
readings are both defensible; the eighth is a proof obligation that is only
half discharged; the first is an outright contradiction inside one section.

---

## 1. §11.1 defines `canonical` twice, and the two disagree

**Severity: this one matters.** `canonical` exists so that two implementations
produce the same bytes. Two implementations following §11.1 will not.

§11.1 gives an **Ordnung**:

> `encode_canonical(x, profile)` ist das Minimum von `Key(S) = ( |output(S)|, c(S) )`
> … lexikographisch von Index 0 aufsteigend mit `B < L < S`.

and then a **Berechnung** for computing it:

> … indem er an jeder Position unter den längenoptimalen Fortsetzungen zuerst
> `B` wählt, sonst das **längste** zulässige Literal. O(n), und das Ergebnis
> ist per Konstruktion das Minimum von `Key`.

It is not the minimum of `Key`. Ending a literal *early* and letting base64
cover the last few bytes is often exactly as long, and then `B < L` decides for
the shorter literal at the first position where the two differ — while the
Berechnung takes the longer one.

The shortest input that tells them apart is ten bytes: nine the profile admits
and one it does not.

```
input       "aaaaaaaaa "          (profile U)

Ordnung     SLLLLLLBBB   ~HaaaaaaaYWEg    13 characters
Berechnung  SLLLLLLLLB   ~JaaaaaaaaaIA    13 characters
                   ^ index 7: B against L, and B < L
```

Both decode to the input. Both are 13 characters. They are different streams,
so a cache key computed by one implementation misses in the other's cache.

**Why §11.1's own verification did not see it.** The section reports exhaustive
enumeration for `n <= 9` over 400 random inputs with 33 genuine length ties and
zero deviations. That is correct, and the bound is one byte short: below ten
bytes a literal never saves enough for the tie to arise. `ten_bytes_is_the_
shortest_disagreement` in `src/canonical.rs` checks every arrangement of an
admitted and a non-admitted byte up to `n = 9` and finds no disagreement, which
is the same result the specification reports — from the other side.

**How often.** 220 of 4000 seeded inputs up to sixteen bytes over a mixed
alphabet — 5.5 %. `how_often_the_two_rules_disagree` in `src/canonical.rs`
recomputes it and prints the count.

**What this implementation does.** It follows the **Ordnung**, because that is
the definition and the Berechnung only claims to compute it. The Berechnung's
rule is implemented too, as `LiteralEnd::Longest`, and is not exported: it
exists so that `divergence_from_the_berechnung_paragraph` can hold the two
apart. Change either and that test fails, which is the point of it.

### What a measurement can and cannot settle here

The obvious next move is to measure the two rules against each other and let
the numbers decide. They do not, and it is worth writing down why, because the
reason is not "the effect was too small".

**What is fixed without data.** Both rules are always the *same length* — both
minimise it and only the tie-break differs (`both_rules_are_always_the_same_
length`). So size cannot decide this, and any measurement has to be of
something else. The two candidates are the share of input bytes that stay
readable in the output, and the number of segments, which is what §9.5 ties
throughput to.

**Neither metric hands over a winner.** Measured with
`cargo run --release --example tiebreak` over the 88-sample binary2textbench
corpus:

| profile, `L_min` | passthrough, Ordnung | passthrough, Berechnung | segments/kB, Ordnung | segments/kB, Berechnung |
|---|---|---|---|---|
| U, 1 (`canonical`) | **3.75 %** | 3.34 % | 7.6 | **6.2** |
| U, 11 (`dense`) | 1.82 % | **1.84 %** | 2.5 | 2.5 |
| T, 1 | **11.68 %** | 11.36 % | 11.5 | **10.5** |
| T, 11 | 10.55 % | **10.56 %** | 8.5 | 8.5 |

At `canonical`'s threshold the two metrics point in **opposite directions**:
the Ordnung keeps more bytes readable, the Berechnung uses fewer segments. So
a measurement selects a rule only after somebody has already chosen which
metric matters — which is the decision, not evidence for it.

**The passthrough result is the counter-intuitive one and it is real.** The
longest literal ought to keep the most bytes in the clear. It does not, because
ending a literal early can realign the base64 run behind it so that a *later*
literal becomes length-optimal as well, and two literals of seven beat one of
eight. Smallest input where it happens, seventeen bytes:

```
"aaaaaaaa  aaaaaaa"    Ordnung     SLLLLLLBBBSLLLLLL   14 bytes readable
                       Berechnung  SLLLLLLLBBBBBBBBB    8 bytes readable
```

Over random inputs at `L_min = 1`, the Berechnung loses passthrough in 59 % of
the cases where the two differ (13096 of 22207). At `L_min = 11` it loses in
0.3 % — the threshold is what makes the intuition nearly true, and `canonical`
is the one preset that does not have it.

**On segments the direction did hold** across everything looked at: exhaustive
to 24 bytes and 240 000 random inputs over both profiles and three thresholds,
without a single reversal. That is an observation, not a theorem, and the
paragraph below is why it is phrased that way.

**A note on where a search space ends.** The first version of
`neither_rule_dominates_on_passthrough` asserted the opposite — that the
Berechnung never loses passthrough — and passed, exhaustively, up to sixteen
bytes. The smallest counterexample is seventeen. That is the same failure mode
as §11.1's own verification stopping at `n <= 9` when the first divergence is
at `n = 10`, and it happened here while writing the document that points it
out. A bound on a search is a claim about where the answer lives, and it
belongs next to the result rather than in the method section.

Which bound, though, is worth knowing for §16.8's fuzzing corpus. A second,
independent implementation of both rules found nothing over an exhaustive
sweep of a four-symbol alphabet to eleven bytes, and a reversal in one run in
four over random inputs of 50 to 400 bytes. Enumerating a richer alphabet
bought nothing here; length was the whole axis. A corpus built by widening the
alphabet and keeping the samples short would have missed this, and it is the
shape a generated corpus tends to take.

**What the fix would be** — for whoever decides, not decided here:

* *Keep the Ordnung, drop "längste" from the Berechnung.* The correct rule is
  "at each position take the smallest symbol that still admits a length-optimal
  completion": `B` where a base64 segment can optimally open, otherwise carry
  the literal on to its longest optimal end, because `L < S`. That is what
  `LiteralEnd::KeyOrder` does. It is the smaller edit, and it leaves every
  sentence about `B < L < S` standing.
* *Keep maximal literals, change the Ordnung.* Then `Key` needs a different
  second component — segment count before the vector, say — and the two
  bullet points under the order have to be rewritten, because `B < L` is
  exactly what makes maximal literals wrong.

Tests: `src/canonical.rs::divergence_from_the_berechnung_paragraph`,
`::ten_bytes_is_the_shortest_disagreement`, and `tests/canonical.rs`, which
checks `encode_canonical` against exhaustive enumeration of every valid
segmentation, ordered by `Key` as written, for all three profiles.

## 2. §11.1's reconstruction is not O(n)

Same section, separate point. The backward cost pass **is** O(n) as §9.2
derives — the two bands, the two monotone deques, profile and F1 as window
bounds and F2 as a position bound all work, and `src/encode.rs` implements them
that way rather than quadratically.

The forward reconstruction is a different question: the deques give the
*minimum*, and reconstruction needs the *arg-minimum under a tie-break*. This
implementation enumerates the candidate ends of each literal, which is
O(window) per literal — fine in practice, since a literal is at least `L_min`
bytes and the window is bounded by 4158, but not the O(n) §11.1 asserts. An
O(n) reconstruction may well exist; the specification does not give one, and
"per Konstruktion" is doing the work of a proof.

## 3. TV2 names one of three equally dense streams

TV2 gives

```
DE AD BE EF "session-eu-central"  ->  3q2-7w~Ssession-eu-central   (26 vs. 30)
```

26 characters is right, and this encoder writes 26. It writes different ones:

```
ours   3q2-73Nl~Qssion-eu-central
TV2    3q2-7w~Ssession-eu-central
```

Absorbing one or two text bytes into the base64 segment is free. With `k` bytes
in the base64 segment the stream is `ceil(4k/3) + (22 - k) + 2` characters
long, which is 26 for `k = 4`, `k = 5` and `k = 6` alike. §9.3 gives `dense` a
threshold and a mode and no tie-break, so all three are `dense` outputs and
none is more correct than the others. `canonical` does choose, and chooses
`k = 6`: at index 4 the alternatives are `B` and `S`, and `B < S`.

So TV2 is a test of the length, not of the bytes. Reading it as a byte-exact
vector makes an implementation fail that is doing nothing wrong — and the
vector set is meant to be grown to 200 (§16.8), which multiplies the problem.
Either the vectors say which tie-break they assume, or they state lengths where
the length is what is determined.

Test: `tests/vectors.rs::tv2_binary_prefix_then_text`, which asserts both
lengths, both decodings, and the tie.

## 4. TV5a's `legible` body is longer than `legible` would write

TV5a gives, for `hello~Alice` as a frame body:

```
Body (legible)   : ~Fhellofg~FAlice  (16 chars)
Body (dense)     : aGVsbG9-QWxpY2U   (15 chars) -> reines Base64
```

The dense line holds exactly, and it is a good test: the encoder declines the
mode switch §8.2 forces, because base64 is shorter. The legible line does not.
`legible` is defined in §9.3 as a threshold — `L >= 4` — and §9.0 keeps the
objective the same for every preset:

> Der Encoder optimiert über die Menge der im jeweiligen Modus gültigen
> Segmentierungen

A threshold widens the candidate set; it does not change what is being
minimised. Pure base64 is a candidate here at 15 characters, so no conforming
encoder writes the 16-character literal form. It is a perfectly valid stream —
the test decodes it to check that — but it is not what `legible` produces.

**The general shape of it: `legible` has no objective of its own.** It is
"`dense` with a lower threshold", which makes it choose literals a little more
often and never at a cost. A preset whose stated purpose is readability
(§0.1: *bevorzugt Lesbarkeit*) needs an objective that can pay for it — a
budget over base64, a preference among equal-length candidates, a maximisation
of literal coverage subject to a length cap. Any of the three would make TV5a's
legible body reachable; the threshold alone does not.

A visible consequence: §9.4 exempts `legible` from the never-worse guarantee,
and the exemption is not needed. Under the length objective `legible` satisfies
`len <= ceil(4n/3)` on everything, which `tests/never_worse.rs::legible_does_
not_need_its_exemption` checks over the corpus. If the exemption is ever needed,
the objective changed.

## 5. TV11 admits two error codes where §10.3 allows one

TV11's last line:

```
"~Aabc"  -> decode()       : framed, dann E_TRUNCATED / E_FRAME_SYNC
```

Only `E_TRUNCATED` is reachable. §10.3 checks the marker, then `pos + 5 <= len`,
then that the three length characters are alphabet characters, then the length
against the stream. `~Aabc` passes the first three: `abc` is a well-formed
18-bit length of 108252, which the five-character stream cannot satisfy. The
slash reads as "either is acceptable", and it is not — the check order in §10.3
is normative and it determines this.

Test: `tests/vectors.rs::tv11_two_entry_points_two_errors`.

## 6. What a frame body is a stream of

§8.1 defines `FrameBody := <Plain-Mode-Stream>`, and §10.3 hands the body to
`decode_plain`, for which the body *is* the stream. But Rule P (§5.3) is
written about a segment that ends "am Stromende", and Rule A (§5.4) about "ein
Strom". Inside a framed stream those are two different objects, and the
specification never says which one is meant.

Both readings are live and this implementation happens to take a different one
for each rule, following the text of each:

| question | this implementation | because |
|---|---|---|
| Is `=` at the end of a frame *body* padding? | **yes**, accepted | §10.3 decodes the body as its own plain stream |
| May two *frames* use different alphabet variants? | **no**, `E_MIXED_ALPHABET` | §5.4 says "ein Strom", and a framed stream is one |

Neither is forced by anything. It costs nothing today, because the encoder
never writes padding and never switches alphabets — but a decoder is a
validator, and §14 makes the case itself: three auto-detections are three
parser-differential surfaces, and this is a fourth where two decoders can
disagree about the same stream while both following the specification.

Tests: `tests/framed.rs::padding_is_recognised_at_the_end_of_a_frame_body`,
`::rule_a_is_read_across_frames_and_not_within_one`.

## 7. §10.3's marker check runs before its bounds check

```
prüfe: stream[pos..pos+2] == "~A"     sonst E_FRAME_SYNC
prüfe: pos + 5 <= len                 sonst E_TRUNCATED
```

Read literally, the first line indexes two octets before anything has
established that two octets are there — at `pos = len - 1` that is a read past
the end. §10.1 flags three implementation traps of exactly this kind and this
one is not among them; it belongs there. The order itself is fine and worth
keeping (a stream that is not framed should say so rather than say it is
truncated); the comparison just has to be length-safe, which it is here.

## 8. §12's binary row is a statement about profiles U and T

> Rein binär | 1,333 | **1,333** *(exakt)*

True under profiles U and T. Under profile B every byte is admissible in a
literal, so the encoder writes one literal segment and the ratio is 1,00096 for
binary input as much as for text — profile B is a length-prefixed passthrough
with a base64 escape hatch it never needs. §7 already says profile B leaves
ASCII behind and must not go into any text container, so this is a precision
point rather than a surprise, but the table's rows are per profile and do not
say so.

Measured, 1 MiB per input, `dense` (`cargo run --release --example density`):

| input | base64 | profile U | profile T | profile B |
|---|---|---|---|---|
| pure binary | 1.333 | 1.333 | 1.333 | 1.001 |
| pure profile-legal text | 1.333 | 1.001 | 1.001 | 1.001 |
| 70 % text / 30 % binary | 1.333 | 1.113 | 1.112 | 1.001 |
| 30 % text / 70 % binary | 1.333 | 1.244 | 1.243 | 1.001 |

The two estimated rows of §12 — ≈ 1,10 and ≈ 1,23 — come out at 1.113 and
1.244 on generated input of that shape. That is not a corpus and the numbers
move with how the mixing is done, which is what §16.5 wants binary2textbench
for.

## 9. §16.3 is half discharged, and the half that is missing is the point

> **`encode_canonical(x)` byte-identisch über zwei unabhängige
> Implementierungen** … Ohne diesen Test ist §11.1 eine Behauptung.

There is one implementation here. `tests/canonical.rs` is the nearest thing
that can be done inside it: an exhaustive enumerator that writes out every
valid segmentation, emits each with its own encoder, and takes the minimum of
`Key` as §11.1 defines it — checked against `encode_canonical` over three
profiles and every input up to twelve bytes over adversarial alphabets. It
shares no code with the encoder and it disagreed with it while finding item 1
above, which is the useful property.

**Since then there is a second implementation.** `conformance/reference.py` is
written from v0.2 rather than from the Rust, with a quadratic dynamic programme instead
of the sliding windows and no shared code; the two agree over all 553 vectors
and all three profiles, 870 pairs, plus fifteen error cases. That discharges
most of point 3. What it does not discharge is the part the section is really
after: both were written by the same person, from the same reading. A third
implementation by somebody else checks itself against `docs/vectors.json`
without reading either.

---

## What was checked and holds

Not findings, listed so that the absence of a finding is not mistaken for the
absence of a check.

* **Every base64 comparison value in §15.** `YWxpY2Uuam9uZXM`,
  `3q2-73Nlc3Npb24tZXUtY2VudHJhbA`, `c3VifmFsaWNlfmpvbmVz`, `PDw_Pz8-Pg`,
  `PDw/Pz8+Pg`, `aGVsbG9-QWxpY2U`. The header arithmetic of TV4 (`~_Al`, and
  `~/Al` in the classic alphabet), the length characters of TV1–TV3, and the
  frame lengths of TV5b, TV9a and TV9b.
* **Rule F (§5.6) is a decision procedure, not a heuristic.** Every encoder
  output decodes through `decode()` into the mode it was written in, over the
  whole round-trip corpus, and the empty stream is plain by convention.
* **Rule A ignores literal payloads (TV7).** A decoder that scans the whole
  stream rejects `~Ka+b/c-d_e~fg`; this one does not.
* **Rule P is not pre-stripping (TV10).** `~Da=b=` and `~Ea=b=` differ only in
  the literal length and get different answers, both correct.
* **F′ and not F2 (TV9).** TV9b is accepted, TV9a is rejected, and `~A` occurs
  exactly at frame starts over every arrangement of `~`, `A` and one other byte
  up to nine long, in all three profiles.
* **§7.1's cookie-octet claim.** All 66 characters of profile U, against the
  ABNF of RFC 6265 §4.1.1, as an executable check rather than a table.
* **§9.4's never-worse guarantee.** `dense` over the whole corpus, all three
  profiles, including §9.5's worst case for the switch rate.
* **§16.2's backwards compatibility.** Against `base64(1)` and Python's
  `base64` module, both alphabets, padded and unpadded, with `E_NONZERO_TAIL`
  in the corpus as the documented disagreement §1.1 calls for.
* **§12's two exact figures.** 4162/4158 = 1,00096 for a full literal segment,
  and exactly `ceil(4n/3)` for high-entropy input.
* **The decoder returns rather than panics** on 20 000 generated streams over
  an adversarial character pool, through all four entry points and all three
  profiles.

## What integrating it into the benchmark found

Wiring the codec into binary2textbench took about a minute to produce a round
trip failure on a 640 KiB tar file: `E_ALIGN`, decoding this implementation's
own output.

The block rule of §9.2.1 was wrong as first written. A block whose last segment
is a base64 run of `k` bytes with `k mod 3 != 0` leaves a partial quantum open;
the next block's run continues it, because two adjacent base64 segments are one
segment to a decoder (§4), and the seam decodes to what neither block meant.
The block size being a multiple of three does not save it — that only covers a
run starting at a block boundary, and a run that starts after a literal has
whatever length the literal left it.

Sixty-eight tests did not catch it, and the reason is worth keeping: the block
tests used homogeneous input. Pure noise makes exactly one base64 run per block
of exactly `BLOCK_BYTES` bytes, which is aligned by construction; pure
profile-legal text makes no base64 run at all. It takes a mixture — which is
what every real file is, and what a corpus assembled by somebody else for
another purpose supplies without being asked.

It was first fixed by requiring a base64 run that ends a block to close on a
quantum boundary — one boundary condition in the dynamic programme. Then the
blocks went away entirely, and with them the seam. What follows is why.

## What the benchmark's second number found

The seam was the round trip. The next thing the benchmark said was the price:

| | encode | decode | size |
|---|---|---|---|
| `dense`, exact programme over blocks | 1478 % of base64 | 678 % | 131,9 % |
| `dense`, linear rule (§9.2.1) | **124 %** | **158 %** | 132,0 % |

A twelvefold difference in encoding, for a tenth of a point of density.

This is the one decision in the repository that was made and then reversed. It
is worth setting out plainly, because the reversal is not "the measurement came
out differently" — the measurement was never taken before the decision.

**What was decided, and on what.** v0.1 permitted a greedy encoder alongside
the exact programme without saying which one a conforming encoder runs. That is
a real defect: two implementations can then write different streams for the
same input and both be conforming, and no byte-exact test vector is possible.
The fix chosen was to strike greedy and make the exact programme the only
conforming encoder — which fixes the defect, and costs the encoder O(n) memory,
which is why blocks followed, and the seam bug followed from the blocks.

**What was wrong with it.** The defect was never *greedy*. It was
*unspecified*. A greedy rule written out as a rule — this is §9.2.1 now — is a
function like any other: deterministic, byte-exact, testable. Striking it threw
away the property that mattered (a scanning encoder streams in constant memory
and runs at base64's speed) to fix a property it never had to lose.

The second mistake was reading §2's "no throughput record" as "throughput is
not a goal". It is a goal; what §2 declines is the record. Reading it the other
way made a twelvefold cost look like it cost nothing, because nothing in the
document was counting.

**What holds instead.** §9.2.1 states the linear rule normatively, and §9.1 —
which was in v0.1 all along — proves it cannot lose against base64: a literal
of eleven bytes or more wins even after the worst rounding on both sides, so an
encoder that takes only those cannot produce a longer stream than base64, and
never needed to optimise to promise it. §9.4 now derives the guarantee that way
for `dense` and from the candidate set for the other presets. Blocks are gone.
`conformance/test_large.py` no longer checks a seam; it checks that two
independent encoders write the same quarter-megabyte stream character for
character, which the quadratic Python reference could not have done while
`dense` was defined by the dynamic programme.

Three things carried the reversal, and all three were already in the
repository: a specification section that proved more than it was being asked to
(§9.1), a benchmark on somebody else's corpus, and the fact that the earlier
decision had been written down in enough detail to be checked against.

## Where the throughput actually went

After the linear rule the encoder was at 124 % of base64's time and the decoder
at 158 %, and the obvious reading was that the extra is what the format costs.
It was not. The benchmark's base64 is the same scalar shape as ours -- same
table, same loop, same compiler -- so the two can be compared directly, and
`opaque` is base65t with the format switched off: one base64 segment, no
literals, no scanning. Measured against each other on a 660 KB wasm blob:

| | ours | the benchmark's base64 |
|---|---|---|
| encode | 1594 MiB/s | 971 MiB/s |
| decode | **629 MiB/s** | 1388 MiB/s |

Our encoder was already 1.6 times the baseline. Our *decoder*, doing strictly
less work than the format requires, was less than half of it. None of that gap
was the format; all of it was two loops.

**The destination.** The inner loop pushed three bytes onto a `Vec` per
quantum, which re-checks the capacity and updates a length that lives behind
`&mut self` and therefore cannot stay in a register. Sizing the output once and
writing it as a slice -- `zip` over two chunk iterators, so the compiler sees
two arrays of known length and emits no bounds check -- took it from 629 to
1013 MiB/s.

**The scan for `~`.** `iter().position()` reads one byte per iteration. On a
stream with no literals in it -- every high-entropy stream, which is most of
what a protocol encodes -- that scan is as long as the decode, and it was:
removing it by reading eight bytes at a time took 932 to 1327 MiB/s. Fifteen
lines, no dependency, and the zero-byte kernel is old enough to be folklore.

Together: decode 158 % → 118 %, and 99 % with a compressor in front. Encoding
gained less, from fusing the scan and the writing into one pass so that no
segment list is built: 124 % → 119 %.

### Two things that did not work

Both were the same instinct -- replace an unpredictable branch with arithmetic
-- and both were measured and reverted.

* **The profile scan, eight bytes to a bitmask.** The theory was that a
  mispredict per run boundary dominates on mixed data. It made every file
  slower: JSON 136 % → 210 %, prose 209 % → 241 %, a tar 193 % → 228 %. A
  mispredict costs once per run; short runs are cheaper read one byte at a
  time, and long runs amortise the one branch away. There was nothing to win.
* **Writing the decoded bytes through a raw pointer** instead of into a sized
  slice, to skip the zero fill. Within the noise, and §14 names memory safety
  as what pays for parsing attacker-controlled lengths. Not a trade worth
  making for nothing.
* **The profile test as arithmetic instead of a table**, so that the run scan
  would vectorise -- profile U is four ranges and four constants, which
  compiles to comparisons rather than an indexed load. It made everything
  slower again (prose 205 % -> 261 %, JSON 136 % -> 185 %), and this time the
  reason names itself: the runs on the files that are slow are shorter than a
  vector register, so the chunked path never runs, and what is left is eight
  comparisons per byte where there was one load.

Three attempts, three losses -- and then the fourth worked, which is the part
worth keeping. Branchless was the right instinct all along and it kept losing
because it was being applied to *one run at a time*, where the run is five
bytes and there is nothing to amortise over. The mask does not scan a run: it
answers for sixty-four bytes at once and lets the run boundaries fall out of
the word afterwards. Same instinct, and the third time the structure was wrong,
not the idea.

### What the encoder's remaining cost is, and the probe that halved it

Not the segments it writes -- the runs it reads and throws away.

Not the segments it writes -- the runs it reads and throws away.
`commonmark-spec.txt` and `countries.json` switch segments at nearly the same
rate and differ by half again in encoding time. English prose in profile U has
a space every five characters, so it is a chain of five-byte profile-legal runs,
none of which reaches §9.1's threshold of eleven. The encoder read all of them
and wrote pure base64 (99.5 % of base64's size). §13 has the per-file table.

The probe was already the fix for the other half of this -- one lookup per
eleven bytes wherever a byte outside the profile can be found -- but it was
being used for only half of what it knows. It says a byte *is* legal as often
as it says one is not, and that also locates something: the run the byte
belongs to. Every literal that could start in the window contains that byte and
therefore lies in that run, so scanning outwards from the probe reads the run
and nothing else, where scanning forwards from the window's start also reads
every rejected run before it. Prose 209 % -> 193 %, JavaScript 250 % -> 196 %,
CSS 213 % -> 189 %, and over the corpus 119 % -> 113 %. The rule is unchanged
and `linear_rule.rs` is what says so: it still transcribes §9.2.1 one byte at a
time, with no probe and no window, and requires the encoder to agree.

### Can it beat base64? On what it is for, yes — and one line was hiding it

The corpus figures say 114 % and 107 %, and they are weighted by bytes, so
megabyte files decide them. §0.1 does not name a megabyte file anywhere: it
names URL queries, cookie values, headers and cache keys. At eight megabytes
both codecs are bound by memory bandwidth rather than by what they compute, so
that ratio measures the scan. At sixty-four bytes it measures the format.

On the benchmark's 55 `short/` samples, profile U, against the same base64:

| sample | bytes | size | encode | decode |
|---|--:|--:|--:|--:|
| SHA-256 digest, hex | 64 | 77 % | 73 % | 75 % |
| SHA-512 digest, hex | 128 | 77 % | 76 % | 68 % |
| JWT, three segments | 155 | 76 % | 74 % | 65 % |
| session id, 40 alnum | 40 | 75 % | 77 % | 80 % |
| UUID v4 | 36 | 79 % | 78 % | 87 % |
| 64 random bytes | 64 | 98 % | 96 % | 100 % |
| an IPv6 address | 28 | 95 % | 97 % | 120 % |
| a log line | 93 | 95 % | 113 % | 132 % |
| **all 55, summed as time** | | | **88 %** | **98 %** |

The throughput advantage is the density advantage, close to one for one, and
the work says why rather than the measurement: base64 reads a byte, looks up
four characters and writes four, per three bytes; a literal reads a byte, tests
it against a set and writes one, and the writing is a `memcpy`. The converse is
in the same rows -- where the output is the same size, base65t is slower by
what the looking costs, and a literal that does not come off is work with
nothing to show.

**One line was hiding half of this.** The decoder allocated
`stream.len() / 4 * 3 + 3` for its output: four characters carry three bytes,
so that is the bound -- for base64. A literal's characters carry one byte each,
so a short value that is one literal decodes to almost the stream's own length,
and the "tighter" bound made it reallocate on every decode. It was reasoned
from base64's ratio and checked against base64-shaped data, where it is right.
On the shape the format exists for it turned a decode that beats base64 into
one that loses to it: a UUID went from 132 % to 87 %, a JWT from 79 % to 65 %,
a hex digest from 124 % to 80 %. `stream.len()` is the bound that holds for
every stream, and it was what stood there before.

### Splitting `dense` across threads changes nothing in the output

Two properties of the format, neither of them about the implementation:

* A profile-illegal byte lies in no literal. So it is a position at which two
  runs of the rule -- one from the beginning of the input, one starting there --
  are in the same state, because the rule's state is the position and nothing
  else.
* A base64 run never crosses a literal. So a cut at a literal's first byte
  leaves the run before it whole, on one side.

Together: cut at the first literal at or after a profile-illegal byte, encode
the pieces independently, concatenate. The bytes are the ones a single pass
would have written, at any thread count -- which they have to be, since §11.1
hangs cache keys on them and a stream that depended on the machine's core count
would be a different format on every machine. `tests/parallel.rs` asserts it
over eight shapes, three profiles and seven thread counts, and separately
asserts that the cuts are really taken, because an encoder that quietly fell
back to one thread would pass the first assertion and prove nothing.

Measured on four cores, on prose, which is the worst case in §13's table:
305 -> 534 MiB/s. It does not move any number in the benchmark: that measures
every codec on one thread, base85n's parallel encoder included.

Decoding a plain stream has no equivalent and cannot get one. Whether a `~`
opens a segment or is payload is known only to the parser that reached it, so
there is nothing local to synchronise on. `framed` has it in both directions,
because frames are self-delimiting (§8.1) -- the same property §8.1 sells as
random access.

## Can it beat base64 on large inputs? Not a vectorised one, and here is why

The machine is not the limit: it copies at 7 GB/s and this encoder runs at 0.8.
Everything below is compute, with a factor of eight to spare.

**The scan was the branch predictor, and a bitmask fixed it.** Read one byte at
a time, "does this run go on" is a branch nothing can guess on mixed input, and
it costs one mispredict per run. Sixty-four bytes to sixty-four bits and the
run positions computed out of the word costs no branch at all:

| the scan alone | prose | binary |
|---|--:|--:|
| byte at a time | 473 MiB/s | 1594 MiB/s |
| masked | 1352 | 1352 |

Data-independent, which is the whole point. Encoding eight megabytes of prose
went from 205 % of base64's time to 113 %, and the spread over every data shape
from 103-205 % to 108-174 %. §9.2.1.2 writes the technique down.

**`base64-simd` is worth what it is worth on our run lengths, not on its own.**
SIMD base64 is advertised on megabyte calls. base65t hands its kernel one
segment at a time, and those average 63 bytes on a tar and 1852 on a wasm blob:

| run length | 16 B | 40 B | 63 B | 128 B | 366 B | 16 KiB |
|---|--:|--:|--:|--:|--:|--:|
| vectorised / scalar | 1.1x | 1.6x | 2.0x | 2.5x | 3.5x | 3.7x |

So two to three and a half, not ten. It is in as the `simd` feature, off by
default, and it cannot change a byte -- `tests/simd.rs` checks the writer
against RFC 4648 §5 read plainly, in both builds.

**And then the number that answers the question.** Eight megabytes, encoding:

| | dickens | mozilla | countries.json |
|---|--:|--:|--:|
| scalar, against a scalar base64 | 113 % | 111 % | 112 % |
| with `simd`, against a scalar base64 | 80 % | 76 % | 76 % |
| with `simd`, against a **vectorised** base64 | 388 % | 354 % | 355 % |

The third row is the honest one, and the loss is structural. **base64 does not
look, it only writes.** base65t has to read the input to find out whether a
literal is in it, and on input where none is -- which is what a compressor
hands you -- that reading is work with nothing to show for it. Even with a
vectorised profile test it would be two passes against one.

Where literals do come off, it wins for exactly the reason it is smaller, and
the short-value table in §13 is that: 57-83 % of base64's time at 75-79 % of
its size. The rule holds in both directions -- as much faster as it is shorter,
and where it is not shorter, slower by the looking.

**GPU and DMA were considered and not pursued.** The data would have to cross
PCIe and come back; for the values §0.1 names, the latency alone exceeds the
encode, and for bulk data the thread split of §9.2.1.1 is nearer and exact. DMA
moves memory, and this is not a move: the machine copies at 7 GB/s and the
bottleneck is at 0.8.

## Deciding not to look at all

The question was whether base65t can beat base64 on large inputs, and the last
answer was: not a vectorised one, because base64 does not look, it only writes.
That reading was right about the arithmetic and wrong about the options, and
the sister format had the missing one written down. base91z §11.5 is called
"Deciding not to scan": sample a window, and where the sample says nothing is
there to find, put the whole window through the cheap path. Its argument for
why that is safe to guess at is one sentence -- **a wrong decision costs size,
never correctness** -- and base65t has the same property to lean on, because a
window written as plain base64 is exactly base64 and §9.4 holds by definition.

So §9.6, `dense-fast`: windows of 65536 bytes cut at absolute offsets, the
first kilobyte of each as the sample, and a window whose sample puts less than
a tenth of its bytes into literals is written without being scanned. Absolute
offsets and a fixed prefix keep it a function of the input, so §9.0 still
holds, `docs/vectors.json` carries byte-exact vectors for it, and the thread
split of §9.2.1.1 still applies.

| file | size `dense` | size `dense-fast` | encoding |
|---|--:|--:|--:|
| `random.bin` | 100.0 % | 100.0 % | 1.82x |
| `dickens` | 99.5 % | 100.0 % | 1.82x |
| `countries.json` | 99.6 % | 100.0 % | 1.77x |
| `mozilla` | 98.5 % | 99.2 % | 1.60x |
| `requests-2.32.3.tar` | 96.9 % | 98.2 % | 1.26x |
| `bootstrap.css` | 93.2 % | 93.2 % | 0.94x |

The last row is the one that says the rule is right rather than lucky. A
stylesheet has real density to lose, the sample keeps every window of it,
nothing is skipped, and the only cost is the sample itself. The decision
selects itself.

**And it closes the gap the previous entry called structural.** With a
vectorised writer as well, against a vectorised base64:

| | `dense` | `dense-fast` |
|---|--:|--:|
| `random.bin` | 565 % | **105 %** |
| `countries.json` | 325 % | **114 %** |
| `dickens` | 455 % | **125 %** |

On input with nothing to find, base65t now writes base64's bytes in base64's
time. The structural argument stands -- looking costs -- and the answer was not
to look faster but to stop looking where there is nothing to see.

The threshold is measured and says so: at a twentieth the gain falls to 1.4x,
at a fifth `bootstrap.css` starts losing density. A tenth is the knee on this
corpus, on this machine, and it is a number read off a graph rather than
derived from one.

## Three more levers, sized

### Non-temporal stores (`_mm256_stream_si256`) — the one with headroom left

They pay against memory bandwidth and nothing else, so the first question is
whether we are near it. On eight megabytes:

| | traffic |
|---|--:|
| `memcpy` | 14.6 GB/s |
| the vectorised encoder | 11.3 GB/s — **78 % of it** |

So on the `simd` path with large inputs, yes: that is close enough for the
store traffic to matter. A normal store to a cache line reads the line first
(read-for-ownership), so writing four thirds of a byte per input byte costs
eight thirds of traffic, not four thirds. A non-temporal store skips the read
and takes the total from 3.67 units per input byte to 2.33 — **a third less**.

Two reasons it is named here rather than built. `base64-simd` does the storing,
so taking this would mean writing and keeping our own AVX2 kernel. And it is a
bet on what the caller does next: a non-temporal store pushes the output out of
cache, which is right if the next thing is a socket and wrong if it is a hash.
A library cannot know, and §13 says the reference stays readable.

The scalar path is nowhere near this — it runs at an eighth of the machine's
bandwidth — so it would gain nothing.

### `IORING_REGISTER_BUFFERS` — right question, wrong layer

Registered buffers are about getting bytes into and out of a process without
pinning them per operation. They say nothing about a transform that happens
entirely in user memory. What they need *from* a codec is the thing this one
did not have: somewhere to write that the caller chose.

So `encode_into` and `decode_into`, appending to a `Vec` the caller owns. The
allocation they save is a fixed cost, which means it matters exactly where the
values are small — which is where §0.1 says they are:

| bytes | encode | decode |
|---|--:|--:|
| 8 | **1.69x** | **1.66x** |
| 16 | 1.44x | 1.55x |
| 32 | 1.30x | 1.35x |
| 64 | 1.16x | 1.27x |
| 155 | 1.03x | 1.15x |
| 4096 | 0.99x | 1.05x |

A session id is forty bytes and a UUID thirty-six. Above half a kilobyte there
is nothing left to save, which is the same shape the allocation measurement
predicted: 22 % of encoding a sixteen-byte value, 8 % at a hundred and fifty,
nothing at four thousand.

### Decoding in place — sound, and here is why

"Read 64 characters, write the 48 decoded bytes back to the same start
address." It works for base64 because the write pointer trails the read
pointer, and it works for base65t for the same reason, which is worth writing
down because it is not obvious that a format with literals keeps the property:

* a base64 run writes three bytes per four characters read;
* a literal writes `m` bytes per `m + h` characters read, `h` being two or four;
* a frame header reads five and writes none, and a padding character reads one
  and writes none.

Every case reads at least as much as it writes, so after any prefix the bytes
written are at most the characters read, and an in-place write never touches a
character that has not been consumed. Decoding in place is therefore available
to this format, not only to base64.

It is not built. `decode_into` already removes the allocation, which is the
part that shows on the values this format is for; what in-place would add is
the second buffer's cache footprint, and that only shows on inputs large enough
not to fit — which is the case §0.1 does not name. The reasoning is recorded so
that whoever wants it does not have to re-derive whether it is allowed.

## What one preset's tie-break cost every other one

The exact programme of §9.2 ran at 21 MiB/s, twenty-one to sixty-three times
slower than the linear rule, and the first three attempts to speed it up moved
nothing worth reporting: caching the deque keys, unrolling a modulo, dropping
an infinity check from the addition. Together, a fifth.

The fourth was the whole of it. `Cost` was a pair -- characters, and a
passthrough term -- so every comparison of two costs was lexicographic, and a
lexicographic comparison branches. There are five of them per input position in
the innermost loop. Making it one number:

| | before | after |
|---|--:|--:|
| 64 bytes | 32 MiB/s | **51** |
| 64 KiB | 26 | **39** |
| 1 MiB | 10 | **29** |

The second component existed for one preset. `legible` broke ties towards
readability, and nothing else in the format ever looked at that number -- but
every preset paid for its presence, in a comparison that ran whether or not
anyone had asked for it. Sixty to a hundred and ninety per cent, on `dense`,
`canonical`, `opaque` and `framed` alike.

So `legible` is gone, and this is the reason. Not its cost in size, which was
zero as `PREREGISTRATION.md` measured; its cost in the shape of a number that
four other presets carried for it. What it offered -- five points more
plaintext at equal length -- profile T offers far more of, on the cheap rule:
the same XML comes out 93 % legible under T against 12 % under U, and 80 % of
base64's size against 98.6 %.

The other half of the speedup was cheaper to find and is worth writing down
because it is a general shape. The programme admitted candidates into its
sliding windows and let the window bounds reject them on the next position.
On text that is nearly all of them: a profile-illegal byte every few characters
means every band-2 candidate, sixty-three bytes ahead, is born ineligible.
Testing eligibility before admitting rather than after took the backward pass
from 31 to 41 MiB/s and cost four lines.

Measured end to end, the programme went from 21 MiB/s to 50 on a short value
and from 8 to 27 on a megabyte, and the factor against the linear rule from
21-63x down to 9-18x. And this is what settles the question the size figures
could not: the programme is worth 0.54 % on the short corpus and 0.31 % over
all of it, so it never had to be a *choice*, only affordable enough to be the
default where it pays.

## What was not done

* **The `L_min`/`B_min` surface of §9.5.** The throughput measurement itself is
  done — base65t is the seventh codec in binary2textbench and the numbers are in
  §13 — but the two-parameter sweep is not, and §9.5 has closed it as a format
  question: a result there can add a preset, never change one.
* **§16.6** — done for Python's parsers (`conformance/test_containers.py`), which is
  where the profile-T whitespace caveat in §7 came from. Browsers, proxies and
  frameworks are still unchecked. (`python/` is the shipped binding over the
  same Rust and is not part of this: a binding cannot disagree with what it
  wraps.)
* **§16.8** — `docs/vectors.json` carries 553 vectors, past what the section
  asks for; §15 itself still names fifteen, and those are the ones written out
  in a form a reader can check by hand.
* **The SIMD decoder of §13.1.** This implementation is scalar and is meant to
  be read.
