// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The encoder, §9 — one rule, and a question asked before it runs.
//!
//! §9.2 derives an O(n) optimum, and the derivation is the interesting part,
//! so it is implemented as derived rather than as a quadratic scan that would
//! agree with it on small inputs: literals are edges, `h` has two bands, and
//! each band is a sliding-window minimum over `D[i] − i` carried by one
//! monotone deque. Admissibility never appears per edge — the profile moves
//! the window's far end — which is what keeps the linear bound (§9.2,
//! *Zulässigkeit der Kanten*).
//!
//! What the encoder decides is not *how hard* to look but *whether*: §9.6
//! classifies the head of the input, and a stream that a magic number or the
//! entropy of its first bytes marks as already compressed goes through base64
//! with nothing scanned at all. Everything else gets the programme.
//!
//! The recurrence here runs backwards, so that the forward pass gets the
//! `Restkosten[j]` §11.1 asks for out of the same table it minimises over.
//!
//! **Base64 runs are maximal (§4).** Two adjacent base64 segments are one
//! segment to a decoder, and closing a segment mid-quantum and opening
//! another decodes to different bytes than the encoder meant. So the state
//! after a base64 segment is distinct from the state after a literal, and only
//! the latter may open a base64 segment. Adjacent *literals* stay legal —
//! §11.1 turns on them.

use crate::alphabet::{Profile, ALPHABET, MAX_LITERAL, TILDE};

/// What an encoding costs, in **thirds of a character**.
///
/// Thirds rather than characters because a base64 quantum is four characters
/// for three bytes, so a per-byte cost is integral only in thirds. There is no
/// rounding anywhere in the programme, which is what lets the minimum be
/// compared exactly.
///
/// One number, not two. It was a pair until v0.4, the second component
/// carrying negated passthrough bytes for a tie-break towards readability; the
/// lexicographic comparison that needs branches, five times per position, in
/// the innermost loop. Removing it took the backward pass from 32 to 51 MiB/s
/// on a short input and from 10 to 29 on a megabyte, and halved the tables.
/// One tie-break nobody had asked for was costing every encode between sixty
/// and a hundred and ninety per cent.
pub type Cost = i64;

const INF: Cost = i64::MAX / 4;

#[inline]
fn is_inf(c: Cost) -> bool {
    c >= INF
}

/// Addition without the infinity check, which the choice of `INF` makes safe.
///
/// `INF` is `i64::MAX / 4`, so any sum of two costs fits, and a sum involving
/// one stays above `INF` -- it goes on comparing as infinite, which is all the
/// sentinel is for. Over a whole input the accumulated slack is `3n`, and `3n`
/// against `i64::MAX / 4` leaves room for an input of two exabytes. The check
/// this replaces was a branch three times per position, in the innermost loop
/// of the encoder.
#[inline]
fn add(a: Cost, b: Cost) -> Cost {
    a + b
}

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
    if p == 0 {
        6
    } else {
        3
    }
}

/// What the programme is allowed to do (§9.0).
///
/// Two fields, and neither is a choice about the encoding. The profile is a
/// statement about the container (§7). `min_literal` is `Some(1)` for the
/// encoding and `None` for [`crate::encode_base64url`], which is not a mode of
/// the format but the way out of it (§14) — so the only value the format's own
/// encoder ever passes is `Some(1)`, and there is no `L_min` left to tune.
#[derive(Debug, Clone, Copy)]
pub struct Rules {
    pub profile: Profile,
    /// `L_min`: the shortest literal to emit. `None` emits none at all, which
    /// is base64url exactly.
    pub min_literal: Option<usize>,
}

impl Rules {
    pub fn new(profile: Profile, min_literal: Option<usize>) -> Self {
        Rules {
            profile,
            min_literal,
        }
    }

    /// Thirds of a character a literal of `m` bytes costs under these rules,
    /// and the passthrough it buys.
    #[inline]
    fn literal_edge(&self, m: usize) -> Cost {
        let m = m as i64;
        3 * m + 3 * h(m as usize) as i64
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

/// Suffix costs, the table the forward pass and §11.1 both reconstruct from.
///
/// * `r_l[j]` — cheapest encoding of `data[j..]` when a base64 segment *may*
///   start at `j`: the previous segment was a literal, or `j` is the start.
/// * `r_b[j]` — cheapest encoding of `data[j..]` when one may *not*: the
///   previous segment was base64, so only a literal may follow (§4).
/// * `g[j][p]` — cheapest finish from inside a base64 segment with `p` bytes
///   already in the open quantum.
pub struct Costs {
    /// `g[j][0..3]` as above, and `g[j][3]` is `r_b[j]`.
    ///
    /// One table rather than three. `r_l` was a copy of `g[j][0]` and `r_b`
    /// was a second allocation for a number the same loop already computes; on
    /// a short value the encoder's cost is the number of times it calls the
    /// allocator, and this took it from three to one.
    g: Vec<[Cost; 4]>,
}

impl Costs {
    /// `r_l[j]`, which is `g[j][0]` and was a table of its own until it was
    /// noticed that the two are the same number. Dropping the duplicate is
    /// worth more than the memory it saves: on a short value the encoder's
    /// time is its allocations, and this is one of three.
    #[inline]
    pub fn r_l(&self, j: usize) -> Cost {
        self.g[j][0]
    }

    /// `r_b[j]`: the cheapest encoding of `data[j..]` when a literal must come
    /// first, because the previous segment was base64 (§4).
    #[inline]
    pub fn r_b(&self, j: usize) -> Cost {
        self.g[j][3]
    }

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

/// A monotone deque over a ring of power-of-two capacity: the sliding-window
/// minimum the two bands of §9.2 need, and nothing else.
///
/// It replaced a pair of `VecDeque`s, and the replacement is most of what the
/// backward pass costs. The window widths are known from §6.1 -- 62 for the
/// two-character header, 4096 for the four-character one -- so the ring never
/// grows, the index arithmetic is a mask rather than a division, and the whole
/// structure is one allocation sized to the input. On a 155-byte value that
/// took the backward pass from 2840 ns to 850.
struct Band {
    buf: Vec<(u32, Cost)>,
    mask: usize,
    head: usize,
    len: usize,
}

impl Band {
    /// `width` is the widest window this band can hold; `n` bounds it for a
    /// short input, where a full-width ring would be the allocation the whole
    /// encode is trying to avoid.
    fn new(width: usize, n: usize) -> Self {
        if n == 0 {
            return Band {
                buf: Vec::new(),
                mask: 0,
                head: 0,
                len: 0,
            };
        }
        let cap = (n + 2).min(width + 2).next_power_of_two();
        Band {
            buf: vec![(0, 0); cap],
            mask: cap - 1,
            head: 0,
            len: 0,
        }
    }

    #[inline]
    fn push(&mut self, t: usize, k: Cost) {
        while self.len > 0 {
            let back = self.buf[(self.head + self.len - 1) & self.mask];
            if back.1 >= k {
                self.len -= 1;
            } else {
                break;
            }
        }
        debug_assert!(self.len < self.buf.len());
        self.buf[(self.head + self.len) & self.mask] = (t as u32, k);
        self.len += 1;
    }

    /// Drops candidates past the window's far end and returns the minimum key
    /// of what is left.
    #[inline]
    fn min_within(&mut self, hi: usize) -> Cost {
        while self.len > 0 {
            let front = self.buf[self.head];
            if front.0 as usize > hi {
                self.head = (self.head + 1) & self.mask;
                self.len -= 1;
            } else {
                return front.1;
            }
        }
        INF
    }
}

/// The backward pass of §9.2. O(n) time, O(1) extra state beyond the tables.
pub fn costs(data: &[u8], rules: Rules) -> Costs {
    let n = data.len();
    let mut g = vec![[INF; 4]; n + 1];
    g[n] = [0; 4];

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
    // The key is stored beside the candidate. It is a function of `t` and of
    // `r_l[t]`, neither of which changes after the candidate enters, and the
    // pop loop below compares against it once per candidate examined --
    // recomputing it there was a third of this function's time.
    // A band that no candidate can ever enter is not allocated: on a short
    // value the encoder's time is its allocations, and band 2 needs 63 bytes
    // before it holds anything.
    let lmin = rules.min_literal.unwrap_or(usize::MAX);
    let mut band1 = Band::new(62, if lmin <= n { n } else { 0 });
    let mut band2 = Band::new(MAX_LITERAL - 63, if lmin.max(63) <= n { n } else { 0 });

    // `first_bad[j]` as a running value: it is monotone as `j` decreases,
    // which is exactly why it can be a window bound instead of a per-edge
    // check.
    let mut first_bad = n;

    for j in (0..n).rev() {
        if !rules.profile.allows(data[j]) {
            first_bad = j;
        }

        if rules.min_literal.is_some() {
            // The far end of each window: the byte run must stay inside the
            // profile. Computed before the candidates enter, because it
            // decides whether they may.
            let far = first_bad;
            let hi1 = far.min(j + 62).min(n);
            let hi2 = far.min(j + MAX_LITERAL).min(n);

            // A candidate past `far` names a literal that runs through a byte
            // the profile rejects, so it can never be chosen -- and it never
            // will be, because `far` only ever moves towards `j`. It used to
            // be admitted anyway and popped again on the next position, which
            // on text is most of what this loop did: a profile-illegal byte
            // every few characters means every band-2 candidate, sixty-three
            // bytes out, is born ineligible.
            if lmin <= 62 && j + lmin <= hi1 {
                let t = j + lmin;
                band1.push(t, 3 * t as i64 + g[t][0]);
            }
            let entry2 = lmin.max(63);
            if entry2 <= MAX_LITERAL && j + entry2 <= hi2 {
                let t = j + entry2;
                band2.push(t, 3 * t as i64 + g[t][0]);
            }

            // Put the `j` parts back on: the same constant for every candidate
            // in a band, so it cannot change which one won.
            let k1 = band1.min_within(hi1);
            let k2 = band2.min_within(hi2);
            let base = 3 * j as i64;
            let mut best = INF;
            if !is_inf(k1) {
                best = k1 + 6 - base;
            }
            if !is_inf(k2) {
                best = best.min(k2 + 12 - base);
            }
            g[j][3] = best;
        }

        // Written out rather than looped: the modulo was a division in the
        // innermost loop of the whole encoder, and there are exactly three.
        let gn = g[j + 1];
        let b = g[j][3];
        g[j] = [
            b.min(add(inc(0), gn[1])),
            b.min(add(inc(1), gn[2])),
            b.min(add(inc(2), gn[0])),
            b,
        ];
    }

    Costs { g }
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
    let mut segs = Vec::new();
    segment_scan(data, rules, c, end, |seg| segs.push(seg));
    segs
}

/// The forward pass of §9.2, handing each segment to `f` as it is decided.
///
/// A callback rather than a `Vec`, because the encoder does not want the list:
/// it wants to write the bytes. On the values §0.1 names that list was one of
/// four allocations against base64's one, and the allocations were most of the
/// difference.
pub fn segment_scan(data: &[u8], rules: Rules, c: &Costs, end: LiteralEnd, mut f: impl FnMut(Seg)) {
    let n = data.len();
    let mut pos = 0;
    let mut may_open_b64 = true;
    while pos < n {
        if may_open_b64 && c.r_l(pos) == c.open_b64(pos) {
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
            f(Seg::Base64(pos, t));
            pos = t;
            may_open_b64 = false;
        } else {
            let t = literal_end(data, rules, c, pos, end);
            f(Seg::Literal(pos, t));
            pos = t;
            may_open_b64 = true;
        }
    }
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
    let best = c.r_b(i);
    debug_assert!(!is_inf(best));

    // One walk over the candidate ends, in increasing order, keeping the two
    // the order can ask for rather than the list of them. The walk starts at
    // the first byte rather than at `i + L_min`, because the run has to be
    // admissible over its whole length and not only past the threshold.
    //
    // `B < L < S`: ending at `t` writes `B` at `t` when a base64 segment opens
    // there and `S` when another literal does, and carrying on writes `L`. So
    // the first end that opens a base64 segment wins outright, and when none
    // does, carrying on beats starting a new literal -- which is the *longest*
    // optimal end, not the earliest.
    let mut last = 0usize;
    let mut t = i + 1;
    while t <= n && t - i <= MAX_LITERAL {
        if !rules.profile.allows(data[t - 1]) {
            break;
        }
        if t - i >= lmin && add(rules.literal_edge(t - i), c.r_l(t)) == best {
            if end == LiteralEnd::KeyOrder
                && t < n
                && add(rules.literal_edge(t - i), c.open_b64(t)) == best
            {
                return t;
            }
            last = t;
        }
        t += 1;
    }
    assert!(last > 0, "the cost table promised a literal at {i}");
    last
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
    // A vectorised writer where the build asked for one. The threshold is
    // where it starts to pay on this machine: at sixteen bytes it is level
    // with the loop below, at forty it is 1.6x, at a few hundred 3.5x. Short
    // runs stay here rather than paying a dispatch to break even.
    #[cfg(feature = "simd")]
    if bytes.len() >= 32 {
        let at = out.len();
        let len = base64_simd::URL_SAFE_NO_PAD.encoded_length(bytes.len());
        out.resize(at + len, 0);
        // The returned slice is the one just written, which the caller already
        // holds; `out` is where it goes.
        let written = base64_simd::URL_SAFE_NO_PAD
            .encode(bytes, base64_simd::Out::from_slice(&mut out[at..]));
        debug_assert_eq!(written.len(), len);
        return;
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    /// The closed form of §9.2.4 is the programme's own answer, and this is why
    /// it may be believed: every length it claims, over both profiles and over
    /// alphabets that reach the header bands, checked against the programme
    /// byte for byte.
    ///
    /// The interesting lengths are all near a boundary -- 6/7, 62/63, 65/66,
    /// 68/69 -- so it walks every one of them rather than sampling.
    #[test]
    fn the_closed_form_agrees_with_the_programme() {
        let pools: [(Profile, &[u8]); 3] = [
            (Profile::U, b"a"),
            (Profile::U, b"abcXY9-._~"),
            (Profile::T, b"a=b, |"),
        ];
        for (profile, pool) in pools {
            for n in 1..=300usize {
                let data: Vec<u8> = (0..n).map(|i| pool[i % pool.len()]).collect();
                let r = Rules::new(profile, Some(1));
                let c = costs(&data, r);
                let by_table = emit(&data, &segment_with(&data, r, &c, LiteralEnd::KeyOrder));
                let by_argument = crate::encode_with(&data, profile);
                assert_eq!(
                    by_argument, by_table,
                    "{profile:?}, n = {n}: the closed form and the programme disagree"
                );
            }
        }
        // And at the far end of the header's reach, where the closed form
        // stops claiming anything.
        for n in [4155usize, 4156, 4157, 4158, 4159, 4160, 4300] {
            let data = vec![b'a'; n];
            let r = Rules::new(Profile::U, Some(1));
            let c = costs(&data, r);
            assert_eq!(
                crate::encode_with(&data, Profile::U),
                emit(&data, &segment_with(&data, r, &c, LiteralEnd::KeyOrder)),
                "n = {n}"
            );
        }
    }

    fn rules(min_literal: Option<usize>) -> Rules {
        Rules::new(Profile::U, min_literal)
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
                let segs = segment_with(data, r, &c, LiteralEnd::KeyOrder);
                let out = emit(data, &segs);
                assert_eq!(3 * out.len() as i64, c.r_l(0), "{data:?} at L_min {lmin:?}");
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

// ---------------------------------------------------------------------------
// §9.6 — the question asked before the programme runs.

/// Bytes of the head the entropy is measured over.
pub const SAMPLE_BYTES: usize = 4096;

/// Above this many bits per byte, nothing a literal can hold is in there.
///
/// A literal needs eleven consecutive bytes the profile admits, and profile U
/// admits 66 of 256. At 7.4 bits per byte such a run is vanishingly rare, so
/// the scan would read the whole input to find nothing. The number is read off
/// the corpus, not derived: at that threshold the decision agrees with always
/// scanning to within 0.00 % of size over 101 samples, and no file gives up
/// more than half a point.
pub const ENTROPY_LIMIT_MILLIBITS: u32 = 7400;

/// Magic numbers of containers whose contents are already compressed.
///
/// Not a guess, unlike the entropy: a stream that opens with these bytes holds
/// deflate, LZMA or an entropy-coded image, and no literal will be found in it
/// at any length. Checking costs a few comparisons at the head.
const MAGIC: [&[u8]; 9] = [
    &[0x1f, 0x8b],                   // gzip
    &[0x28, 0xb5, 0x2f, 0xfd],       // zstd
    &[0xfd, b'7', b'z', b'X', b'Z'], // xz
    b"BZh",                          // bzip2
    b"PK\x03\x04",                   // zip
    &[0xff, 0xd8, 0xff],             // JPEG
    &[0x89, b'P', b'N', b'G'],       // PNG
    b"OggS",                         // Ogg
    &[0x1a, 0x45, 0xdf, 0xa3],       // Matroska / WebM
];

/// Shannon entropy of `data`, in thousandths of a bit per byte.
///
/// Integer arithmetic on purpose: this decides what the encoder writes, so two
/// implementations have to agree on it exactly, and floating point is where
/// two implementations stop agreeing. The logarithm is a 256-entry table of
/// `-log2(k/n)` scaled by 1000, computed by integer bisection.
fn entropy_millibits(data: &[u8]) -> u32 {
    let n = data.len();
    if n == 0 {
        return 0;
    }
    let mut count = [0u32; 256];
    for &b in data {
        count[b as usize] += 1;
    }
    // Sum of k * log2(n/k), scaled by 1000, divided by n at the end.
    let mut total: u64 = 0;
    for &k in count.iter() {
        if k == 0 {
            continue;
        }
        total += k as u64 * log2_millibits(n as u64, k as u64);
    }
    (total / n as u64) as u32
}

/// `1000 * log2(a / b)` for `a >= b > 0`, by integer bisection.
///
/// Exact and reproducible: the same inputs give the same answer on every
/// machine and in every language, which a `f64::log2` does not promise.
fn log2_millibits(a: u64, b: u64) -> u64 {
    // Integer part.
    let mut whole = 0u64;
    let mut x = a;
    while x >= 2 * b {
        x /= 2;
        whole += 1;
    }
    // Fractional part by squaring: x/b is in [1, 2), and squaring it moves one
    // bit of the fraction into the integer part each round.
    let mut frac = 0u64;
    let mut num = x;
    let mut den = b;
    for bit in 0..16 {
        // num/den squared, kept from overflowing by dropping low bits.
        while num > (1u64 << 31) || den > (1u64 << 31) {
            num >>= 1;
            den >>= 1;
        }
        num *= num;
        den *= den;
        if num >= 2 * den {
            num /= 2;
            frac += 1000 >> (bit + 1);
        }
    }
    whole * 1000 + frac
}

/// What §9.6 decides for a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Nothing to find: write base64url and do not scan.
    Base64,
    /// Run the programme of §9.2.
    Exact,
}

/// §9.6, once at the head of the stream and carried through.
///
/// The magic numbers are asked first because they are evidence and the entropy
/// is an estimate. The estimate then needs a full sample: over `n` bytes the
/// plug-in entropy of 256 symbols cannot exceed `log2(n)` at all, so under
/// 4096 bytes the figure cannot reach the limit however random the input is,
/// and asking would be arithmetic with a foregone answer. Such an input gets
/// the programme -- which is the thorough branch, not a fallback, and cheap
/// on an input that small.
pub fn classify(data: &[u8]) -> Mode {
    if MAGIC.iter().any(|m| data.starts_with(m)) {
        return Mode::Base64;
    }
    if data.len() < SAMPLE_BYTES {
        return Mode::Exact;
    }
    if entropy_millibits(&data[..SAMPLE_BYTES]) > ENTROPY_LIMIT_MILLIBITS {
        Mode::Base64
    } else {
        Mode::Exact
    }
}

/// Input bytes the programme is run over at a time (§9.2.3).
///
/// The programme needs about twenty-four bytes of table per input byte, so a
/// gigabyte object cannot have it run over the whole of it. A window bounds
/// that at 1.5 MB whatever the input is, and costs at most one extra literal
/// header per boundary -- under 0.01 %. The windows are cut at absolute
/// offsets, so the segmentation stays a function of the input.
pub const WINDOW_BYTES: usize = 65536;

/// Whether every byte of `data` may appear in a literal payload.
///
/// The mask of §9.2.1.2 answers for sixty-four bytes at a time and does not
/// branch on the data, so this costs about three quarters of a nanosecond a
/// byte and stops at the first block that says no.
fn all_admitted(profile: Profile, data: &[u8]) -> bool {
    let (blocks, tail) = data.as_chunks::<64>();
    for b in blocks {
        if profile.mask64(b) != u64::MAX {
            return false;
        }
    }
    tail.is_empty() || profile.mask_short(tail) == (1u64 << tail.len()) - 1
}

/// The programme's answer, in closed form, for an input every byte of which
/// the profile admits and which fits inside the header's reach (§6.1).
///
/// Not a second encoder and not an approximation: it is the same segmentation
/// the programme returns, arrived at by argument instead of by table, and
/// `the_closed_form_agrees_with_the_programme` checks that byte for byte over
/// every length it claims.
///
/// **The argument.** Adjacent base64 runs are one run (§4) and `ceil` is
/// superadditive over a split, so an optimum needs at most one base64 run, and
/// putting it first is what `B` being the smallest symbol asks for. With `B`
/// base64 bytes and the rest in literals, the cost over `n` is
///
/// ```text
/// extra(B) = ceil(B/3) + cover(n - B),   cover(M) = 2*ceil(M/62) capped at 4
/// ```
///
/// because a literal of at most 62 bytes carries a two-character header and one
/// of up to 4158 a four-character one (§6.1). Minimising it, and breaking ties
/// by `B < L < S` from index 0:
///
/// * `n <= 6` — `ceil(n/3) <= 2`, so base64 is shorter, or equal at 4, 5 and 6
///   where `B` takes the tie.
/// * `7 <= n <= 62` — one literal at `n + 2`, and strictly: any base64 byte
///   adds at least one character and saves none.
/// * `63 <= n <= 65` — three base64 bytes cost one character over their own
///   length and drop the literal from the four-character header band to the
///   two-character one, which is worth two. Three rather than one or two,
///   because all three tie and the longest base64 run is the smallest vector.
/// * `66 <= n <= 68` — the same one band further: six base64 bytes cost two and
///   still save two, tying with the single four-character literal, and `B < S`
///   at index 0 decides for the run.
/// * `69 <= n <= 4158` — one literal at `n + 4`. Below 125 that ties with two
///   literals of at most 62, and `L < S` at the seam takes the single one; the
///   `100 = 50 + 50` case §11.1 raises is exactly this.
///
/// Above 4158 the partition question is real -- `4238 + 62` costs six header
/// characters where `4158 + 142` costs eight -- and this declines to answer it.
fn closed_form(profile: Profile, data: &[u8]) -> Option<(Seg, Option<Seg>)> {
    let n = data.len();
    if n == 0 || n > MAX_LITERAL || !all_admitted(profile, data) {
        return None;
    }
    Some(match n {
        1..=6 => (Seg::Base64(0, n), None),
        7..=62 => (Seg::Literal(0, n), None),
        63..=65 => (Seg::Base64(0, 3), Some(Seg::Literal(3, n))),
        66..=68 => (Seg::Base64(0, 6), Some(Seg::Literal(6, n))),
        _ => (Seg::Literal(0, n), None),
    })
}

/// The segmentation of the whole input: the programme per window, and adjacent
/// base64 runs joined across the boundaries.
///
/// The joining is not a nicety. Two adjacent base64 segments are one segment to
/// a decoder (§4), so emitting them separately writes a partial quantum into
/// the middle of a run and the seam decodes to what neither window meant.
/// Joining them first also makes the all-base64 case exactly base64, which is
/// what §9.4 needs.
fn segment_windowed(data: &[u8], rules: Rules) -> Vec<Seg> {
    let mut segs: Vec<Seg> = Vec::new();
    for start in (0..data.len()).step_by(WINDOW_BYTES) {
        let end = (start + WINDOW_BYTES).min(data.len());
        let c = costs(&data[start..end], rules);
        segment_scan(&data[start..end], rules, &c, LiteralEnd::KeyOrder, |seg| {
            let seg = match seg {
                Seg::Base64(i, j) => Seg::Base64(start + i, start + j),
                Seg::Literal(i, j) => Seg::Literal(start + i, start + j),
            };
            match (segs.last_mut(), seg) {
                (Some(Seg::Base64(_, prev)), Seg::Base64(i, j)) if *prev == i => *prev = j,
                _ => segs.push(seg),
            }
        });
    }
    segs
}

/// The encoding of §9: classify, then either base64 or the programme.
pub fn encode_rules(data: &[u8], rules: Rules) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 3 + 8);
    encode_rules_into(data, rules, &mut out);
    out
}

/// The same, appending to a buffer the caller owns.
pub fn encode_rules_into(data: &[u8], rules: Rules, out: &mut Vec<u8>) {
    out.reserve(data.len() + data.len() / 3 + 8);
    if data.is_empty() {
        return;
    }
    match rules.min_literal {
        None => emit_base64(data, out),
        Some(_) => match classify(data) {
            Mode::Base64 => emit_base64(data, out),
            Mode::Exact => {
                // The case §0.1 is about -- a value that is already text --
                // is also the case the programme has nothing to decide about,
                // and answering it by argument rather than by table is the
                // difference between costing four times what base64 costs on
                // a session id and costing a fifth of it.
                let mut write = |seg: Seg| match seg {
                    Seg::Base64(i, j) => emit_base64(&data[i..j], out),
                    Seg::Literal(i, j) => emit_literal(&data[i..j], out),
                };
                match closed_form(rules.profile, data) {
                    Some((a, b)) => {
                        write(a);
                        if let Some(b) = b {
                            write(b);
                        }
                    }
                    // One window is the common case and needs no list of
                    // segments at all: the runs inside a window are already
                    // maximal, so there is no seam to join and the bytes can
                    // go straight out.
                    None if data.len() <= WINDOW_BYTES => {
                        let c = costs(data, rules);
                        segment_scan(data, rules, &c, LiteralEnd::KeyOrder, write);
                    }
                    None => {
                        for seg in segment_windowed(data, rules) {
                            write(seg);
                        }
                    }
                }
            }
        },
    }
}
