// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The encoder, §9 — two rules, and which preset gets which.
//!
//! `dense` and `framed` use the linear rule of §9.2.1: one forward scan that
//! takes every profile-legal run of eleven bytes or more. It is not
//! length-optimal, and §9.1 shows it does not have to be — a literal that long
//! cannot lose against base64 even after the worst rounding on both sides, so
//! the never-worse guarantee of §9.4 holds without any optimisation at all.
//! What it buys is the encoder running in constant memory at roughly base64's
//! speed rather than at a twelfth of it.
//!
//! `canonical`, `legible` and `opaque` use the exact programme of §9.2, which
//! derives an O(n) optimum. The derivation is the interesting part, so it is
//! implemented as derived rather than as a quadratic scan that would agree
//! with it on small inputs: literals are edges, `h` has two bands, and each
//! band is a sliding-window minimum over `D[i] − i` carried by one monotone
//! deque. Admissibility never appears per edge — profile and F1 move the
//! window's far end, F2 removes a position's literal transition outright —
//! which is what keeps the linear bound (§9.2, *Zulässigkeit der Kanten*).
//!
//! The recurrence here runs backwards, so that `encode_canonical` gets the
//! `Restkosten[j]` §11.1 asks for from the same pass the presets use.
//!
//! **Base64 runs are maximal (§4).** Two adjacent base64 segments are one
//! segment to a decoder, and closing a segment mid-quantum and opening
//! another decodes to different bytes than the encoder meant. So the state
//! after a base64 segment is distinct from the state after a literal, and only
//! the latter may open a base64 segment. Adjacent *literals* stay legal —
//! §11.1 turns on them.

use std::collections::VecDeque;

use crate::alphabet::{Profile, ALPHABET, MAX_FRAME_BODY, MAX_LITERAL, TILDE};

/// What an encoding costs, as the pair the objective compares.
///
/// The first component is characters **in thirds**, so that a readability
/// bonus of a third of a character — what base64 wastes on a byte it could
/// have passed through — stays integral. The second is negated passthrough
/// bytes, and is held at zero unless the rules ask for readability; a
/// lexicographic minimum over the pair is then "shortest, and among those the
/// most readable".
pub type Cost = (i64, i64);

const INF: Cost = (i64::MAX / 4, 0);

#[inline]
fn is_inf(c: Cost) -> bool {
    c.0 >= INF.0
}

#[inline]
fn add(a: Cost, b: Cost) -> Cost {
    if is_inf(a) || is_inf(b) {
        INF
    } else {
        (a.0 + b.0, a.1 + b.1)
    }
}

/// Bytes per frame body in `encode_framed`, before encoding (§8.1): a fixed
/// decoded size is what makes offset-to-frame O(1) without a trailer.
pub const FRAME_BYTES: usize = 65536;

/// Header cost of a literal of `m` bytes (§6.1): two bands, and no third one,
/// because a run longer than `MAX_LITERAL` is several edges.
#[inline]
fn h(m: usize) -> usize {
    if m <= 62 {
        2
    } else {
        4
    }
}

/// Characters a base64 run of `k` bytes costs.
#[cfg(test)]
#[inline]
fn b64_chars(k: usize) -> usize {
    (4 * k).div_ceil(3)
}

/// Thirds of a character the byte at quantum offset `p` costs: two characters
/// for the first of a quantum, one for the others — four per three bytes
/// (§9.2), which is twelve thirds per three bytes.
#[inline]
fn inc(p: usize) -> Cost {
    (if p == 0 { 6 } else { 3 }, 0)
}

/// What the encoder is allowed to do, which is all a preset is (§9.0, §9.3).
#[derive(Debug, Clone, Copy)]
pub struct Rules {
    pub profile: Profile,
    /// `L_min`: the shortest literal this preset will emit. `None` never emits
    /// one, which is `opaque`.
    pub min_literal: Option<usize>,
    /// F1 and F2 in force, for a stream that will be carried in frames (§8.2).
    pub framed: bool,
    /// λ: thirds of a character a passthrough byte is worth, subtracted from
    /// what a literal costs. Zero is the pure length optimum, which is what
    /// every preset in §9.3 uses; the specification has no other value yet,
    /// and PREREGISTRATION.md is the measurement that picks one for `legible`.
    pub bonus: i64,
    /// Whether ties in length are broken towards readability rather than left
    /// to the reconstruction rule. This is the second component of `Cost`.
    pub prefer_passthrough: bool,
}

impl Rules {
    /// A preset as §9.3 defines one: length only, no bonus.
    pub fn preset(profile: Profile, min_literal: Option<usize>, framed: bool) -> Self {
        Rules {
            profile,
            min_literal,
            framed,
            bonus: 0,
            prefer_passthrough: false,
        }
    }

    /// Thirds of a character a literal of `m` bytes costs under these rules,
    /// and the passthrough it buys.
    #[inline]
    fn literal_edge(&self, m: usize) -> Cost {
        let m = m as i64;
        (
            (3 - self.bonus) * m + 3 * h(m as usize) as i64,
            if self.prefer_passthrough { -m } else { 0 },
        )
    }
}

/// One segment of a segmentation, as `[start, end)` over the input bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seg {
    Base64(usize, usize),
    Literal(usize, usize),
}

/// The three-symbol vector of §11.1. Not needed to encode; needed to say what
/// `canonical` means and to test it.
pub fn c_vector(segs: &[Seg]) -> String {
    let mut v = String::new();
    for seg in segs {
        match *seg {
            Seg::Base64(i, j) => v.extend(std::iter::repeat_n('B', j - i)),
            Seg::Literal(i, j) => {
                v.push('S');
                v.extend(std::iter::repeat_n('L', j - i - 1));
            }
        }
    }
    v
}

/// Suffix costs, the table both the presets and §11.1 reconstruct from.
///
/// * `r_l[j]` — cheapest encoding of `data[j..]` when a base64 segment *may*
///   start at `j`: the previous segment was a literal, or `j` is the start.
/// * `r_b[j]` — cheapest encoding of `data[j..]` when one may *not*: the
///   previous segment was base64, so only a literal may follow (§4).
/// * `g[j][p]` — cheapest finish from inside a base64 segment with `p` bytes
///   already in the open quantum.
pub struct Costs {
    pub r_l: Vec<Cost>,
    pub r_b: Vec<Cost>,
    g: Vec<[Cost; 3]>,
}

impl Costs {
    /// Cost of opening a base64 segment at `j`, or `INF` at the end of input.
    #[inline]
    fn open_b64(&self, j: usize) -> Cost {
        match self.g.get(j + 1) {
            Some(g) => add(inc(0), g[1]),
            // At the end of the input there is no byte for a segment to open
            // on.
            None => INF,
        }
    }
}

/// The backward pass of §9.2. O(n) time, O(1) extra state beyond the tables.
pub fn costs(data: &[u8], rules: Rules) -> Costs {
    let n = data.len();
    let mut r_l = vec![INF; n + 1];
    let mut r_b = vec![INF; n + 1];
    let mut g = vec![[INF; 3]; n + 1];
    r_l[n] = (0, 0);
    r_b[n] = (0, 0);
    g[n] = [(0, 0); 3];

    // Deques over candidate end positions `t`. A literal edge [j, t) costs
    // `(3 − λ)(t − j) + 3h + r_l[t]` in thirds, and buys `t − j` passthrough
    // bytes, so both components split into a part that depends only on `t` and
    // a part that depends only on `j` — and the `j` part falls out of the
    // minimisation. Band 1 is `h = 2` and `m <= 62`, band 2 is `h = 4` and
    // `63 <= m <= MAX_LITERAL`.
    //
    // Going backwards, candidates enter at the near end and expire at the far
    // end, which is the mirror image of the usual sliding-window minimum: pop
    // the back on insertion, pop the front against the window's far end.
    let mut band1: VecDeque<usize> = VecDeque::new();
    let mut band2: VecDeque<usize> = VecDeque::new();
    let pw: i64 = if rules.prefer_passthrough { 1 } else { 0 };
    let key = |t: usize, r_l: &[Cost]| -> Cost {
        if is_inf(r_l[t]) {
            INF
        } else {
            (
                (3 - rules.bonus) * t as i64 + r_l[t].0,
                r_l[t].1 - pw * t as i64,
            )
        }
    };

    // `first_bad[j]` and `first_tilde_a[j]` as running values: both are
    // monotone as `j` decreases, which is exactly why they can be window
    // bounds instead of per-edge checks.
    let mut first_bad = n;
    let mut first_tilde_a = usize::MAX;

    let lmin = rules.min_literal.unwrap_or(usize::MAX);
    for j in (0..n).rev() {
        if !rules.profile.allows(data[j]) {
            first_bad = j;
        }
        if rules.framed && data[j] == TILDE && j + 1 < n && data[j + 1] == b'A' {
            first_tilde_a = j;
        }

        if rules.min_literal.is_some() {
            // A literal ending at `t` is barred outright when its last byte is
            // a tilde (F2) — a property of `t`, so such a `t` never enters a
            // deque at all.
            let admit = |t: usize, dq: &mut VecDeque<usize>, r_l: &[Cost]| {
                if t > n {
                    return;
                }
                if rules.framed && data[t - 1] == TILDE {
                    return;
                }
                let k = key(t, r_l);
                while let Some(&b) = dq.back() {
                    if key(b, r_l) >= k {
                        dq.pop_back();
                    } else {
                        break;
                    }
                }
                dq.push_back(t);
            };
            if lmin <= 62 {
                admit(j + lmin, &mut band1, &r_l);
            }
            let entry2 = lmin.max(63);
            if entry2 <= MAX_LITERAL {
                admit(j + entry2, &mut band2, &r_l);
            }

            // The far end of each window: the byte run must stay inside the
            // profile, and inside F1 when framed.
            let mut far = first_bad;
            if rules.framed && first_tilde_a != usize::MAX {
                far = far.min(first_tilde_a + 1);
            }
            let hi1 = far.min(j + 62).min(n);
            let hi2 = far.min(j + MAX_LITERAL).min(n);
            while let Some(&f) = band1.front() {
                if f > hi1 {
                    band1.pop_front();
                } else {
                    break;
                }
            }
            while let Some(&f) = band2.front() {
                if f > hi2 {
                    band2.pop_front();
                } else {
                    break;
                }
            }

            // Put the `j` parts back on: the same constant for every candidate
            // in a band, so it cannot change which one won.
            let mut best = INF;
            for (dq, header) in [(&band1, 2i64), (&band2, 4i64)] {
                if let Some(&t) = dq.front() {
                    let k = key(t, &r_l);
                    if !is_inf(k) {
                        let c = (
                            k.0 + 3 * header - (3 - rules.bonus) * j as i64,
                            k.1 + pw * j as i64,
                        );
                        best = best.min(c);
                    }
                }
            }
            r_b[j] = best;
        }

        for p in 0..3 {
            g[j][p] = r_b[j].min(add(inc(p), g[j + 1][(p + 1) % 3]));
        }
        r_l[j] = r_b[j].min(add(inc(0), g[j + 1][1]));
    }

    Costs { r_l, r_b, g }
}

/// The forward pass: among the length-optimal continuations, take the one the
/// order of §11.1 names — `B` before `L` before `S`, decided at the earliest
/// position where the candidates differ.
///
/// For `dense`, `legible` and `framed` any optimal segmentation would do and
/// this one is simply the deterministic choice. For `canonical` it is the
/// definition, and `canonical::longest_literal` is the same walk under the
/// other rule, kept only so a test can hold the two apart.
pub fn segment(data: &[u8], rules: Rules, c: &Costs) -> Vec<Seg> {
    segment_with(data, rules, c, LiteralEnd::KeyOrder)
}

/// Which end position to take when several are length-optimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralEnd {
    /// What `Key`'s `B < L < S` asks for: end the literal at the first
    /// position where a base64 segment can optimally open, because `B` beats
    /// `L` there — and otherwise run it to its longest optimal end, because
    /// `L` beats `S`.
    KeyOrder,
    /// The longest optimal literal, which is what §11.1's *Berechnung*
    /// paragraph asks for. The two differ; see FINDINGS.md.
    Longest,
}

pub fn segment_with(data: &[u8], rules: Rules, c: &Costs, end: LiteralEnd) -> Vec<Seg> {
    let n = data.len();
    let mut segs = Vec::new();
    let mut pos = 0;
    let mut may_open_b64 = true;
    while pos < n {
        if may_open_b64 && c.r_l[pos] == c.open_b64(pos) {
            // Inside the run every byte is another `B`, and `B` is the
            // smallest symbol, so the run extends as far as optimality allows.
            let mut t = pos + 1;
            let mut p = 1usize;
            loop {
                let extend = t < n && c.g[t][p] == add(inc(p), c.g[t + 1][(p + 1) % 3]);
                if !extend {
                    break;
                }
                p = (p + 1) % 3;
                t += 1;
            }
            segs.push(Seg::Base64(pos, t));
            pos = t;
            may_open_b64 = false;
        } else {
            let t = literal_end(data, rules, c, pos, end);
            segs.push(Seg::Literal(pos, t));
            pos = t;
            may_open_b64 = true;
        }
    }
    segs
}

/// Where the literal that starts at `i` ends.
///
/// Every candidate end is enumerated rather than queried through the deques:
/// the deques give the minimum, and this needs the arg-minimum under a
/// tie-break, which is a different question. It costs O(window) for a literal
/// that is at least `L_min` bytes long — see the note on complexity in
/// FINDINGS.md.
fn literal_end(data: &[u8], rules: Rules, c: &Costs, i: usize, end: LiteralEnd) -> usize {
    let n = data.len();
    let lmin = rules.min_literal.expect("a literal was chosen");
    let best = c.r_b[i];
    debug_assert!(!is_inf(best));

    // Candidate ends, in increasing order. The walk starts at the first byte
    // rather than at `i + L_min`, because the run has to be admissible over
    // its whole length and not only past the threshold.
    let mut cands: Vec<usize> = Vec::new();
    let mut t = i + 1;
    while t <= n && t - i <= MAX_LITERAL {
        if !rules.profile.allows(data[t - 1]) {
            break;
        }
        if rules.framed && t - i >= 2 && data[t - 2] == TILDE && data[t - 1] == b'A' {
            break;
        }
        let barred_f2 = rules.framed && data[t - 1] == TILDE;
        if t - i >= lmin && !barred_f2 && add(rules.literal_edge(t - i), c.r_l[t]) == best {
            cands.push(t);
        }
        t += 1;
    }
    assert!(
        !cands.is_empty(),
        "the cost table promised a literal at {i}"
    );

    match end {
        LiteralEnd::Longest => *cands.last().expect("non-empty"),
        LiteralEnd::KeyOrder => {
            // Ending at `t` writes `B` at `t` when a base64 segment opens
            // there and `S` when another literal does; carrying on writes `L`.
            // `B < L < S`, so: the first end that opens a base64 segment wins
            // outright, and when none does, carrying on beats starting a new
            // literal — which is the *longest* optimal end, not the earliest.
            for &t in &cands {
                if t < n && add(rules.literal_edge(t - i), c.open_b64(t)) == best {
                    return t;
                }
            }
            *cands.last().expect("non-empty")
        }
    }
}

/// Writes a segmentation out (§5.1, §6.1): URL alphabet, never padded, and
/// always the shortest header form.
pub fn emit(data: &[u8], segs: &[Seg]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 3 + 8);
    for seg in segs {
        match *seg {
            Seg::Base64(i, j) => emit_base64(&data[i..j], &mut out),
            Seg::Literal(i, j) => emit_literal(&data[i..j], &mut out),
        }
    }
    out
}

/// One literal segment: the tilde, the length header in its shortest form
/// (§6.1), and the bytes.
fn emit_literal(bytes: &[u8], out: &mut Vec<u8>) {
    let m = bytes.len();
    debug_assert!((1..=MAX_LITERAL).contains(&m));
    out.push(TILDE);
    if m <= 62 {
        out.push(ALPHABET[m]);
    } else {
        let v = m - 63;
        out.push(ALPHABET[63]);
        out.push(ALPHABET[(v >> 6) & 63]);
        out.push(ALPHABET[v & 63]);
    }
    out.extend_from_slice(bytes);
}

fn emit_base64(bytes: &[u8], out: &mut Vec<u8>) {
    // `as_chunks` rather than `chunks_exact`: the group size is a constant, so
    // it belongs in the type where the compiler can see it rather than in a
    // runtime length the indexing below has to be trusted against.
    let (groups, remainder) = bytes.as_chunks::<3>();
    out.reserve(groups.len() * 4 + 4);
    for c in groups {
        let n = (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
        // Four characters written at once: the bounds check and the length
        // update happen once per quantum instead of four times.
        out.extend_from_slice(&[
            ALPHABET[(n >> 18) as usize & 63],
            ALPHABET[(n >> 12) as usize & 63],
            ALPHABET[(n >> 6) as usize & 63],
            ALPHABET[n as usize & 63],
        ]);
    }
    match remainder {
        [a] => {
            let n = (*a as u32) << 16;
            out.push(ALPHABET[(n >> 18) as usize & 63]);
            out.push(ALPHABET[(n >> 12) as usize & 63]);
        }
        [a, b] => {
            let n = (*a as u32) << 16 | (*b as u32) << 8;
            out.push(ALPHABET[(n >> 18) as usize & 63]);
            out.push(ALPHABET[(n >> 12) as usize & 63]);
            out.push(ALPHABET[(n >> 6) as usize & 63]);
        }
        _ => {}
    }
}

/// The linear-time segmentation of §9.2.1: scan, and take every run the
/// threshold admits.
///
/// This is what `dense` and `framed` use, and it is normative rather than
/// "some greedy encoder": the rule is exact, so the output is a function of
/// the input like every other preset's, and the published vectors stay
/// byte-exact. What it is not is length-optimal — it never absorbs a byte into
/// a base64 run to align a quantum, where the programme in §9.2.2 sometimes
/// would.
///
/// It cannot lose against base64, and that is §9.1's derivation rather than a
/// hope: a literal of `L >= 11` bytes saves `(L - 10)/3` characters *after*
/// the worst-case rounding its two base64 neighbours can suffer. Every literal
/// here is at least that long, and the saving composes over the stream.
///
/// One pass, no tables, no backpointers, constant memory. That is the whole
/// point: the exact programme needs O(n) backpointers and about 80 bytes of
/// state per input byte, which is what made encoding fifteen times the cost of
/// base64.
pub fn segment_greedy(data: &[u8], rules: Rules) -> Vec<Seg> {
    let n = data.len();
    let Some(lmin) = rules.min_literal else {
        // `opaque`: never a literal.
        return if n == 0 {
            Vec::new()
        } else {
            vec![Seg::Base64(0, n)]
        };
    };

    let mut segs = Vec::new();
    let mut pending = 0; // start of the base64 run being accumulated
    let mut i = 0;
    while i < n {
        // A literal of `lmin` bytes starting anywhere in `[i, i + lmin)` has
        // to cover `i + lmin - 1`, so one disallowed byte there rules out
        // every start in that window at once. The scan below would reach the
        // same place one byte at a time; on data whose bytes are mostly
        // outside the profile -- compressed, encrypted, most binary formats --
        // that is the whole of the encoder's work, and this does it in a
        // lookup per `lmin` bytes instead of one per byte. The literals found
        // are the same ones: the test is necessary, never sufficient, and the
        // framed constraints below only ever shorten a run.
        if i + lmin <= n && !rules.profile.allows(data[i + lmin - 1]) {
            i += lmin;
            continue;
        }
        // The longest literal a run starting here could carry.
        let mut j = i;
        while j < n && j - i < MAX_LITERAL && rules.profile.allows(data[j]) {
            // F1: the payload may not contain `~A` (§8.2).
            if rules.framed && data[j] == TILDE && j + 1 < n && data[j + 1] == b'A' {
                break;
            }
            j += 1;
        }
        // F2: the payload may not end on a tilde.
        if rules.framed {
            while j > i && data[j - 1] == TILDE {
                j -= 1;
            }
        }

        if j - i >= lmin {
            if pending < i {
                segs.push(Seg::Base64(pending, i));
            }
            segs.push(Seg::Literal(i, j));
            i = j;
            pending = i;
        } else {
            // Too short to pay for its header: these bytes go to base64. No
            // run starting inside a rejected one can be longer than it was,
            // so the scan resumes at its end rather than at the next byte --
            // which is what keeps this linear.
            i = j.max(i + 1);
        }
    }
    if pending < n {
        segs.push(Seg::Base64(pending, n));
    }
    segs
}

/// One plain-mode stream under the given rules, over the whole input.
pub fn encode_rules(data: &[u8], rules: Rules) -> Vec<u8> {
    let c = costs(data, rules);
    let segs = segment(data, rules, &c);
    emit(data, &segs)
}

/// One plain-mode stream under the linear rule of §9.2.1, over any input.
///
/// The scan and the writing are one pass. `segment_greedy` exists beside this
/// for the tests, which need the segmentation itself rather than the stream,
/// but going through it would mean a heap allocation proportional to the
/// number of segments and a second walk over the input to write what the first
/// one already knew. The rule is the same rule; only the list in between is
/// gone.
pub fn encode_greedy(data: &[u8], rules: Rules) -> Vec<u8> {
    let n = data.len();
    let mut out = Vec::with_capacity(n + n / 3 + 8);
    let Some(lmin) = rules.min_literal else {
        emit_base64(data, &mut out); // `opaque`: never a literal
        return out;
    };

    let mut pending = 0;
    let mut i = 0;
    while i < n {
        if i + lmin <= n && !rules.profile.allows(data[i + lmin - 1]) {
            i += lmin;
            continue;
        }
        let mut j = i;
        while j < n && j - i < MAX_LITERAL && rules.profile.allows(data[j]) {
            if rules.framed && data[j] == TILDE && j + 1 < n && data[j + 1] == b'A' {
                break;
            }
            j += 1;
        }
        if rules.framed {
            while j > i && data[j - 1] == TILDE {
                j -= 1;
            }
        }

        if j - i >= lmin {
            if pending < i {
                emit_base64(&data[pending..i], &mut out);
            }
            emit_literal(&data[i..j], &mut out);
            i = j;
            pending = i;
        } else {
            i = j.max(i + 1);
        }
    }
    if pending < n {
        emit_base64(&data[pending..n], &mut out);
    }
    out
}

/// §8: fixed-size frames, so that a byte offset names a frame without a
/// trailer. F1 and F2 hold inside each body, so `~A` occurs only where a frame
/// starts (§8.2).
pub fn encode_framed(data: &[u8], profile: Profile, min_literal: usize) -> Vec<u8> {
    let rules = Rules::preset(profile, Some(min_literal), true);
    let mut out = Vec::new();
    for chunk in data.chunks(FRAME_BYTES) {
        // Frames are for large streams, so the linear rule here too.
        let body = encode_greedy(chunk, rules);
        assert!(
            body.len() <= MAX_FRAME_BODY,
            "a frame body of {} chars does not fit 18 bits",
            body.len()
        );
        out.push(TILDE);
        out.push(b'A');
        out.push(ALPHABET[(body.len() >> 12) & 63]);
        out.push(ALPHABET[(body.len() >> 6) & 63]);
        out.push(ALPHABET[body.len() & 63]);
        out.extend_from_slice(&body);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(min_literal: Option<usize>) -> Rules {
        Rules::preset(Profile::U, min_literal, false)
    }

    /// The cost table has to agree with the string the emitter actually
    /// writes, or every claim downstream of it is about a different format.
    #[test]
    fn cost_table_matches_emitted_length() {
        let cases: [&[u8]; 6] = [
            b"",
            b"a",
            b"alice.jones",
            b"alice.jones and a space",
            b"\xde\xad\xbe\xefsession-eu-central",
            b"aaaaaaaaa ",
        ];
        for data in cases {
            for lmin in [Some(1), Some(4), Some(11), None] {
                let r = rules(lmin);
                let c = costs(data, r);
                let segs = segment(data, r, &c);
                let out = emit(data, &segs);
                assert_eq!(
                    3 * out.len() as i64,
                    c.r_l[0].0,
                    "{data:?} at L_min {lmin:?}"
                );
            }
        }
    }

    /// §9.2's base64 edge cost, spelled out: four characters per three bytes,
    /// two for the first byte of a quantum and one for each of the others.
    #[test]
    fn base64_run_costs_what_it_writes() {
        for k in 1..64usize {
            let data = vec![0u8; k];
            let mut out = Vec::new();
            emit_base64(&data, &mut out);
            assert_eq!(out.len(), b64_chars(k));
        }
    }
}
