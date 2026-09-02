// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The canonical order of §11.1, and the one other reading of it.
//!
//! There is no `encode_canonical` any more, and its absence is the point: the
//! encoder of §9 *is* the canonical rule. `Key(S) = (|output(S)|, c(S))` under
//! `B < L < S`, and the forward pass takes at every position the smallest
//! symbol that still admits a length-optimal completion — `B` where a base64
//! segment can optimally open, otherwise `L` while the literal can optimally
//! carry on, otherwise `S`. One encoder, one output, for every input.
//!
//! v0.1 described that computation as "otherwise the **longest** admissible
//! literal" and claimed it was the minimum of `Key`. It is not: ending a
//! literal early can align the base64 run behind it so that a later literal
//! becomes length-optimal too, and `B < L` then decides for the shorter one.
//! The correction is E1 of the errata and TV13 of §15. This module is what is
//! left of that argument: `LiteralEnd::Longest` is v0.1's rule, unexported,
//! kept so the difference stays testable rather than remembered.

#[cfg(test)]
use crate::alphabet::Profile;
#[cfg(test)]
use crate::encode::{costs, emit, segment_with, LiteralEnd, Rules};

/// The rules the encoder runs under, so the tests below compare against what
/// ships rather than against a copy of it.
#[cfg(test)]
fn rules(profile: Profile) -> Rules {
    Rules::new(profile, Some(1))
}

/// The same length, under the rule §11.1's *Berechnung* paragraph gives.
///
/// Not exported from the crate: it is not a second canonical form, it is the
/// evidence that there would be two if the paragraph were followed.
#[cfg(test)]
fn encode_canonical_longest_literal(data: &[u8], profile: Profile) -> Vec<u8> {
    let r = rules(profile);
    let c = costs(data, r);
    emit(data, &segment_with(data, r, &c, LiteralEnd::Longest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{c_vector, emit, segment_with as seg};

    fn cvec(data: &[u8], end: LiteralEnd) -> String {
        let r = rules(Profile::U);
        let c = costs(data, r);
        c_vector(&seg(data, r, &c, end))
    }

    /// The two readings of §11.1, on the shortest input that tells them apart.
    ///
    /// Nine profile-legal bytes and one that is not. Both segmentations are 13
    /// characters, so `Key`'s first component is a tie and the vector decides:
    /// `B < L` at index 7. This test is a pin, not a preference — if either
    /// rule is ever changed, it should be changed knowingly.
    #[test]
    fn divergence_from_the_berechnung_paragraph() {
        let data = b"aaaaaaaaa ";
        assert_eq!(cvec(data, LiteralEnd::KeyOrder), "SLLLLLLBBB");
        assert_eq!(cvec(data, LiteralEnd::Longest), "SLLLLLLLLB");

        let by_order = crate::encode_with(data, Profile::U);
        let by_construction = encode_canonical_longest_literal(data, Profile::U);
        assert_eq!(by_order.len(), by_construction.len(), "a length tie");
        assert_ne!(by_order, by_construction, "and two different streams");
        assert_eq!(by_order, b"~HaaaaaaaYWEg".to_vec());
        assert_eq!(by_construction, b"~JaaaaaaaaaIA".to_vec());

        // Both decode to the input, which is why this is a canonicity bug and
        // not a correctness one.
        for s in [&by_order, &by_construction] {
            assert_eq!(
                crate::decode(s, Profile::U).expect("decodes").bytes,
                data.to_vec()
            );
        }
    }

    /// Nothing shorter than ten bytes separates the two rules, which is why
    /// the verification quoted in §11.1 — exhaustive to `n <= 9` — reports no
    /// deviation.
    #[test]
    fn ten_bytes_is_the_shortest_disagreement() {
        // Two byte values are enough: one the profile admits, one it does not.
        for n in 1..10usize {
            for bits in 0..(1u32 << n) {
                let data: Vec<u8> = (0..n)
                    .map(|i| if bits >> i & 1 == 1 { b'a' } else { b' ' })
                    .collect();
                assert_eq!(
                    cvec(&data, LiteralEnd::KeyOrder),
                    cvec(&data, LiteralEnd::Longest),
                    "{data:?}"
                );
            }
        }
    }

    /// How often it matters, over a corpus that can be regenerated: a seeded
    /// stream of inputs up to sixteen bytes over an alphabet of admitted and
    /// non-admitted bytes. The assertion is a band rather than a number so
    /// that it says "often enough to care" without pinning a digit nobody
    /// would maintain; the exact count is printed with `--nocapture` and is
    /// quoted in FINDINGS.md.
    #[test]
    fn how_often_the_two_rules_disagree() {
        let alphabet = b"ab.~ -_9";
        let mut s: u32 = 0x5eed_9a31;
        let mut next = move || {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as usize
        };
        let (mut differ, mut total) = (0usize, 0usize);
        for _ in 0..4000 {
            let n = 1 + next() % 16;
            let data: Vec<u8> = (0..n).map(|_| alphabet[next() % alphabet.len()]).collect();
            total += 1;
            if cvec(&data, LiteralEnd::KeyOrder) != cvec(&data, LiteralEnd::Longest) {
                differ += 1;
            }
        }
        let percent = 100.0 * differ as f64 / total as f64;
        println!("{differ}/{total} inputs differ ({percent:.1} %)");
        assert!(
            (3.0..12.0).contains(&percent),
            "{percent:.1} % is outside the band this was written against"
        );
    }

    /// The one thing that is true of both rules whatever the input: they are
    /// the same length. Both minimise it; only the tie-break differs. If this
    /// ever fails, the disagreement in §11.1 is not a tie-break question at
    /// all and everything written about it is wrong.
    #[test]
    fn both_rules_are_always_the_same_length() {
        let mut s: u32 = 0x2545_f491;
        let mut next = move || {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as usize
        };
        for _ in 0..4000 {
            let n = 1 + next() % 400;
            let data: Vec<u8> = (0..n).map(|_| b"aabbc.-_ ,;\n\x00"[next() % 13]).collect();
            for profile in [Profile::U, Profile::T] {
                for lmin in [1usize, 4, 11] {
                    let r = Rules::new(profile, Some(lmin));
                    let c = costs(&data, r);
                    let by_order = emit(&data, &seg(&data, r, &c, LiteralEnd::KeyOrder));
                    let by_longest = emit(&data, &seg(&data, r, &c, LiteralEnd::Longest));
                    assert_eq!(
                        by_order.len(),
                        by_longest.len(),
                        "{data:?} {profile:?} {lmin}"
                    );
                    assert_eq!(3 * by_order.len() as i64, c.r_l(0));
                }
            }
        }
    }

    /// Neither rule dominates the other on passthrough — the share of input
    /// bytes that stay readable in the output.
    ///
    /// This is worth a test because the opposite is the obvious guess: the
    /// longest literal keeps the most bytes in the clear, surely. It does not.
    /// Ending a literal early can realign the base64 run that follows so that
    /// a *later* literal becomes length-optimal too, and two literals of seven
    /// beat one of eight.
    ///
    /// The smallest input where it happens is seventeen bytes — which is one
    /// byte past where the first version of this test looked, and the same
    /// mistake §11.1's own verification makes at `n <= 9`. A search space is
    /// a claim about where the answer lives, and it is worth stating: below,
    /// exhaustive to sixteen bytes finds nothing.
    #[test]
    fn neither_rule_dominates_on_passthrough() {
        fn passthrough(c: &str) -> usize {
            c.chars().filter(|&x| x != 'B').count()
        }
        // Two literals of seven against one of eight, at equal length.
        let data = b"aaaaaaaa  aaaaaaa";
        assert_eq!(data.len(), 17);
        let r = Rules::new(Profile::U, Some(1));
        let c = costs(data, r);
        let by_order = c_vector(&seg(data, r, &c, LiteralEnd::KeyOrder));
        let by_longest = c_vector(&seg(data, r, &c, LiteralEnd::Longest));
        assert_eq!(by_order, "SLLLLLLBBBSLLLLLL");
        assert_eq!(by_longest, "SLLLLLLLBBBBBBBBB");
        assert_eq!(passthrough(&by_order), 14);
        assert_eq!(passthrough(&by_longest), 8);

        // And nothing shorter does it: every arrangement of an admitted and a
        // non-admitted byte up to sixteen long, at every threshold.
        for n in 1..=16usize {
            for bits in 0u32..(1 << n) {
                let d: Vec<u8> = (0..n)
                    .map(|i| if bits >> i & 1 == 1 { b'a' } else { b' ' })
                    .collect();
                for lmin in [1usize, 4, 11] {
                    let r = Rules::new(Profile::U, Some(lmin));
                    let c = costs(&d, r);
                    let k = c_vector(&seg(&d, r, &c, LiteralEnd::KeyOrder));
                    let l = c_vector(&seg(&d, r, &c, LiteralEnd::Longest));
                    assert!(passthrough(&l) >= passthrough(&k), "{d:?} at n = {n}");
                }
            }
        }
    }

    /// `canonical` has no `L_min`, and §11.1 says what that buys: a seven-byte
    /// literal where the alignment is right — nine characters against ten.
    #[test]
    fn literals_reach_down_to_seven_bytes() {
        let data = b"abcdefg";
        let out = crate::encode_with(data, Profile::U);
        assert_eq!(out, b"~Habcdefg".to_vec());
        assert_eq!(out.len(), 9);
        assert_eq!((4 * data.len()).div_ceil(3), 10, "base64 would be longer");
    }
}
