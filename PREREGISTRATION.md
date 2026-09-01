# Pre-registration: what `legible` optimises, and how `dense` breaks a tie

Written and committed **before** the measurement it describes. Nothing below is
adjusted afterwards; the run either satisfies a rule here or falls to the
default here, and both outcomes are decisions.

## The question

`legible` has no objective — §9.3 gives it a threshold, and a length-minimising
encoder never buys a literal that costs something, so the threshold alone
cannot make output more readable (FINDINGS.md item 4). `dense` has no tie-break,
so TV2 names one of three equally dense streams (item 3). Both are the same
question: **how much readability is worth how much else.**

## What the specification already decides

The three axes are not equal, and the document says so in three places:

| axis | where | rank |
|---|---|---|
| size | §1 goal 1 and §9.4, **normative**: `len ≤ ceil(4n/3)` | hard constraint |
| readability | §1 goal 3 "Lesbar bleiben", §0.1 `legible` | **the objective** |
| throughput | §2 non-goal: "Kein Durchsatz-Rekord"; §13 prices in being slower | constraint with a ceiling |

So the answer has this shape before anything is measured:

> Maximise the passthrough share, subject to never being longer than base64 and
> to a bounded segment rate.

What remains to be measured is how much readability that boundary yields, and
whether there is a knee before it where a small size concession buys a
disproportionate amount. **Only if that knee exists is §9.4's exemption for
`legible` justified**; otherwise the exemption is struck rather than a
parameter introduced.

## The knob

A literal of `m` bytes costs `m + h(m)` characters. The encoder is given a
bonus λ per passthrough byte, in thirds of a character so that everything stays
integral:

```
cost(literal of m) = 3(m + h(m)) − λ·m
cost(base64 of k)  = 4k
λ ∈ {0, 1, 2, 3, 4}
```

λ = 1 is the one value with a derivation rather than a measurement behind it: a
passthrough byte is worth exactly the third of a character base64 wastes on it.
It is the candidate every other value has to justify itself against — the same
move §9.1 makes when it derives `L ≥ 11` instead of measuring it.

Three reconstruction rules are swept beside λ, because they decide the `dense`
tie-break at the same time: `KeyOrder` (the order of §11.1, already decided),
`Longest`, `MaxPassthrough` (lexicographic: least length, then most
passthrough).

## Metrics

Exact ones only. `P(λ)` passthrough share, `S(λ)` encoded length as a
percentage of `ceil(4n/3)`, `G(λ)` segments per kB. All three are
deterministic: no runner, no repetitions, no noise, and identical on any
machine.

Throughput is **not** measured. §9.5 ties it to the segment switch rate, which
is exact, and a noisy throughput figure would make the same statement worse.
It stays with the benchmark (§13.2).

## Data, kept apart

| role | data |
|---|---|
| decision | binary2textbench `core` + `short` + `synthetic`, 88 samples, 8.5 MB |
| axis sweep | generated here: share of profile-illegal bytes 0–100 % × run lengths 4/16/64/256 — the b2t corpus varies the wrong axis for profile U, where the problematic byte is the space |
| **hold-out** | `silesia`, 202 MiB, assembled by other people in 2003 for another purpose. Touched **once**, after the choice is made |

## The decision rule

```
1.  λ is admissible only if  S(λ) ≤ 100 % on EVERY file        (§9.4, goal 1)
    and  G(λ) ≤ 1.5 · G(0)                                      (§2, throughput)
2.  among admissible λ: the smallest with  P(λ) ≥ P(λ_max) − 1 point
3.  if λ = 1 is within one point of that value, λ = 1 wins
4.  if no λ > 0 is admissible, the answer is λ = 0
5.  hold-out: the choice must sit in the same plateau on silesia AND on the
    axis sweep (P within one point of the optimum there). If it does not,
    the answer is λ = 0 and the failure is recorded rather than repaired.
```

For the `dense` tie-break, at λ = 0: a rule wins only if it is better on P by
≥ 1 point or on G by ≥ 5 % **and** has that sign on a majority of files
individually. Otherwise `dense` inherits the order from §11.1, so that the
format has exactly one tie-break rule and every test vector is byte-exact.

Rules 4 and 5 guarantee a decision even if the data says nothing.

## Guards, and what is already known

* **Plateau, not peak.** The start of a flat region is chosen, never the
  maximum of a curve.
* **Derivation beats measurement.** Rule 3 exists for this and is why λ = 1 is
  privileged.
* **Per-file signs, not the mean.** 39 % of the corpus carries no literal at
  all and dilutes every average; a rule that wins only in the aggregate does
  not win.
* **Search bounds belong in the result.** Every claim states what it was
  checked over. The lesson that produced this line: an exhaustive check to
  sixteen bytes found nothing, and the counterexample is at seventeen.
* **Already known before this run**, and disclosed rather than pretended away:
  the three reconstruction rules have been measured at λ = 0 and the numbers
  are in FINDINGS.md — at `L_min = 1` the order keeps 3.75 % of bytes readable
  against the longest rule's 3.34 %, and uses 7.6 segments per kB against 6.2.
  The λ dimension is new; the λ = 0 column is not, and rule 3's threshold was
  set knowing it.
