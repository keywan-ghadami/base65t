# What the reference implementation found

The specification in `docs/spec-v0.1.de.md` is v0.1 final and unchanged. This
file is what came out of implementing it and running the conformance work of
§16 against it: nine places where the text says something the code cannot do,
or does not say enough for two implementations to agree. Each one names the
test that holds it in place, so that whichever way it is decided, the decision
is visible.

**All nine are decided and folded into `docs/spec-v0.2.de.md`.**
`docs/errata-v0.1.de.md` says what holds instead, entry by entry; this file
stays as the record of how each one was found. Six of
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

**Since then there is a second implementation.** `python/base65t.py` is written
from v0.2 rather than from the Rust, with a quadratic dynamic programme instead
of the sliding windows and no shared code; the two agree over all 456 vectors
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

## What was not done

* **§16.5** — the throughput measurements and the `L_min`/`B_min` surface. That
  is binary2textbench's job and needs the codec wired into it. Since v0.2 it is
  no longer binding on anything: §13.2 has no acceptance gate to fail, and §9.5
  lets a result add a preset but never change one.
* **§16.6** — done for Python's parsers (`python/test_containers.py`), which is
  where the profile-T whitespace caveat in §7 came from. Browsers, proxies and
  frameworks are still unchecked.
* **§16.8** — the vector set is 17 tests over §15's twelve vectors, not the 200
  the section asks for.
* **The SIMD decoder of §13.1.** This implementation is scalar and is meant to
  be read.
