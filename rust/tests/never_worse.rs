// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §9.4 — `dense` is never longer than base64, and §12's two exact figures.
//!
//! The guarantee is what makes the format a drop-in rather than a trade, so it
//! is checked over everything the round-trip corpus holds and not only over
//! the cases it was designed for. §9.4 exempts `legible` and `framed`; only
//! one of those exemptions turns out to be needed, and the test says which.

use base65t::*;

fn base64_len(n: usize) -> usize {
    (4 * n).div_ceil(3)
}

fn corpus() -> Vec<(String, Vec<u8>)> {
    let mut s: u32 = 0xa5a5_1234;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    v.push(("empty".into(), Vec::new()));
    for n in 0..40usize {
        v.push((
            format!("noise {n}"),
            (0..n).map(|_| (next() & 0xff) as u8).collect(),
        ));
        v.push((
            format!("text {n}"),
            (0..n).map(|i| b"abcdefghij"[i % 10]).collect(),
        ));
        v.push((
            format!("alternating {n}"),
            (0..n)
                .map(|i| if i % 12 == 11 { b' ' } else { b'a' })
                .collect(),
        ));
    }
    for percent in [0usize, 1, 5, 20, 50, 100] {
        v.push((
            format!("{percent}% untransportable"),
            (0..4000)
                .map(|_| {
                    if next() % 100 < percent {
                        (next() & 0xff) as u8
                    } else {
                        b"abcdefghijklmnop.-_~"[next() % 20]
                    }
                })
                .collect(),
        ));
    }
    // §9.5's worst case for the switch rate, which is also the case where a
    // naive encoder is most tempted to do something longer than base64.
    v.push((
        "eleven legal, one not, repeated".into(),
        (0..100)
            .flat_map(|_| {
                let mut c = vec![b'a'; 11];
                c.push(b' ');
                c
            })
            .collect(),
    ));
    v
}

#[test]
fn dense_is_never_longer_than_base64() {
    for (name, data) in corpus() {
        for profile in [Profile::U, Profile::T, Profile::B] {
            let out = encode_dense(&data, profile);
            assert!(
                out.len() <= base64_len(data.len()),
                "{name}, {profile:?}: {} > {}",
                out.len(),
                base64_len(data.len())
            );
        }
    }
}

/// §9.4 in v0.2 is a statement about the format, not about one preset: four of
/// the five presets are never longer than base64, per input rather than on
/// average. That is the whole of the case for switching, so it is checked for
/// each of them rather than argued from the one that was measured.
#[test]
fn the_guarantee_covers_four_of_the_five_presets() {
    for (name, data) in corpus() {
        for profile in [Profile::U, Profile::T, Profile::B] {
            for (preset, out) in [
                ("dense", encode_dense(&data, profile)),
                ("legible", encode_legible(&data, profile)),
                ("canonical", encode_canonical(&data, profile)),
                ("opaque", encode_opaque(&data)),
            ] {
                assert!(
                    out.len() <= base64_len(data.len()),
                    "{name}, {preset}, {profile:?}"
                );
            }
        }
    }
}

/// The fifth is `framed`, and its exemption is quantified rather than left as
/// "does not hold": five characters per frame, and the base64 each frame body
/// would have cost on its own.
///
/// It is an exemption, not a penalty — on data a literal can carry, `framed`
/// is far shorter than base64 despite the headers. What it cannot do is
/// *promise* to be shorter, and `framing_can_be_worse_than_base64` in
/// tests/framed.rs is the case where it is not.
#[test]
fn only_framed_is_exempt_and_by_how_much() {
    for n in [1usize, 100, 10_000, FRAME_BYTES + 1] {
        for profile in [Profile::U, Profile::B] {
            let data: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
            let frames = n.div_ceil(FRAME_BYTES);
            let bound: usize = data
                .chunks(FRAME_BYTES)
                .map(|c| base64_len(c.len()))
                .sum::<usize>()
                + 5 * frames;
            let framed = encode_framed(&data, profile);
            assert!(
                framed.len() <= bound,
                "n = {n}, {profile:?}: {} > {bound}",
                framed.len()
            );
        }
    }
}

/// `dense` encodes in blocks so that memory is constant (§9.2), and the block
/// size is a multiple of three so that this costs nothing at the boundary.
///
/// Both halves are checked, because the second is the one that is easy to get
/// wrong: at 65536 rather than 65535 each block would round up on its own, the
/// guarantee above would fail by a character per 64 KiB, and the output on
/// high-entropy input would stop being base64url byte for byte.
#[test]
fn blocking_costs_neither_the_guarantee_nor_the_base64_identity() {
    assert_eq!(BLOCK_BYTES % 3, 0);
    let mut s: u32 = 0x1234_9e37;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    };
    // Lengths either side of one, two and three boundaries, and one that ends
    // exactly on one.
    for n in [
        BLOCK_BYTES - 1,
        BLOCK_BYTES,
        BLOCK_BYTES + 1,
        2 * BLOCK_BYTES,
        2 * BLOCK_BYTES + 7,
        3 * BLOCK_BYTES + 4159,
    ] {
        let noise: Vec<u8> = (0..n).map(|_| (next() & 0xff) as u8).collect();
        let text: Vec<u8> = (0..n).map(|i| b"abcdefghij"[i % 10]).collect();
        for (what, data) in [("noise", &noise), ("text", &text)] {
            for profile in [Profile::U, Profile::B] {
                let out = encode_dense(data, profile);
                assert!(out.len() <= base64_len(n), "{what} {n} {profile:?}");
                assert_eq!(
                    decode(&out, profile).expect("decodes").bytes,
                    *data,
                    "{what} {n} {profile:?}"
                );
            }
        }
        // High entropy under profile U leaves no literal worth taking, so the
        // stream must be base64url exactly -- across block boundaries too.
        assert_eq!(
            encode_dense(&noise, Profile::U),
            encode_opaque(&noise),
            "n = {n}"
        );
    }
}

/// What blocking does cost: a literal cannot span a boundary, so at most one
/// extra header per boundary. Measured against the same encoder run over the
/// whole input in one piece.
#[test]
fn blocking_costs_less_than_a_hundredth_of_a_percent() {
    use base65t::internals::{costs, emit, segment_with, LiteralEnd, Rules};
    let n = 3 * BLOCK_BYTES + 1000;
    let data: Vec<u8> = (0..n)
        .map(|i| {
            if i % 97 == 0 {
                b' '
            } else {
                b"abcdefghij"[i % 10]
            }
        })
        .collect();
    let rules = Rules::preset(Profile::U, Some(11), false);
    let c = costs(&data, rules);
    let whole = emit(&data, &segment_with(&data, rules, &c, LiteralEnd::KeyOrder));
    let blocked = encode_dense(&data, Profile::U);
    assert!(blocked.len() >= whole.len());
    let overhead = (blocked.len() - whole.len()) as f64 / whole.len() as f64;
    assert!(overhead < 0.0001, "blocking cost {:.4} %", 100.0 * overhead);
}

/// §9.4 does not extend the guarantee to `legible`, but under §9.0's objective
/// — the shortest of the valid segmentations — `legible` cannot exceed it
/// either: pure base64 is always a candidate. The exemption is only needed by
/// an encoder that prefers literals for their own sake, and §9.3 defines
/// `legible` as a threshold rather than as such a preference. See FINDINGS.md.
#[test]
fn legible_does_not_need_its_exemption() {
    for (name, data) in corpus() {
        let out = encode_legible(&data, Profile::U);
        assert!(out.len() <= base64_len(data.len()), "{name}");
    }
}

/// `framed` does need its exemption: five characters a frame are five
/// characters base64 does not spend.
#[test]
fn framed_needs_its_exemption() {
    let data = b"alice.jones";
    assert!(encode_framed(data, Profile::U).len() > base64_len(data.len()));
}

/// §12, second row: a literal segment carries 4158 bytes for 4162 characters,
/// and that ratio is a bound rather than a limit — long input approaches it
/// from below and never reaches 1.
#[test]
fn the_density_bound_for_pure_literal_text_is_exact() {
    let n = MAX_LITERAL;
    let data = vec![b'a'; n];
    let out = encode_dense(&data, Profile::U);
    assert_eq!(out.len(), 4162);
    assert_eq!(out.len() as f64 / n as f64, 4162.0 / 4158.0);

    for k in 1..=4 {
        let data = vec![b'a'; k * n];
        let out = encode_dense(&data, Profile::U);
        assert_eq!(out.len(), k * 4162, "{k} full literal segments");
    }
}

/// §12, first row: high-entropy input is base64 exactly, because nothing else
/// is shorter.
#[test]
fn high_entropy_input_encodes_at_the_base64_ratio() {
    let mut s: u32 = 0x9e37_79b9;
    let data: Vec<u8> = (0..10_000)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s & 0xff) as u8
        })
        .collect();
    let out = encode_dense(&data, Profile::U);
    assert_eq!(out.len(), base64_len(data.len()));
    assert_eq!(out, encode_opaque(&data));
}

/// Bytes a reader can see without decoding: the payloads of the literal
/// segments. Walked out of the stream rather than taken from the encoder, so
/// that what is checked is the stream and not a number beside it.
fn passthrough(stream: &[u8]) -> usize {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let value = |c: u8| A.iter().position(|&x| x == c).expect("alphabet");
    let mut total = 0;
    let mut pos = 0;
    while pos < stream.len() {
        if stream[pos] == b'~' {
            let l1 = value(stream[pos + 1]);
            let (len, header) = if l1 == 63 {
                (
                    63 + (value(stream[pos + 2]) << 6) + value(stream[pos + 3]),
                    4,
                )
            } else {
                (l1, 2)
            };
            total += len;
            pos += header + len;
        } else {
            pos += 1;
        }
    }
    total
}

/// What `legible` is, after the errata: the shortest encoding, and among the
/// shortest the one that leaves the most bytes readable (E4). Both halves are
/// normative now, so both are checked — the second one against the rule
/// `dense` and `canonical` use, which is what it has to beat to be worth a
/// preset of its own.
#[test]
fn legible_is_readability_at_no_cost_in_size() {
    let mut ahead = 0usize;
    let mut inputs = 0usize;
    for (name, data) in corpus() {
        if data.is_empty() {
            continue;
        }
        for profile in [Profile::U, Profile::T] {
            let legible = encode_legible(&data, profile);
            let dense = encode_dense(&data, profile);
            let canonical = encode_canonical(&data, profile);

            // Never longer than base64 — §9.4 now covers `legible` too.
            assert!(legible.len() <= base64_len(data.len()), "{name}");
            // And never longer than the length-optimal encoding without a
            // threshold, because it *is* one of those.
            assert_eq!(legible.len(), canonical.len(), "{name}, {profile:?}");
            assert!(legible.len() <= dense.len(), "{name}, {profile:?}");

            inputs += 1;
            if passthrough(&legible) < passthrough(&canonical) {
                ahead += 1;
            }
        }
    }
    // The passthrough claim is a property of the objective, not of the corpus:
    // maximising it among equal-length segmentations cannot come out behind on
    // any single input.
    assert_eq!(ahead, 0, "{ahead} of {inputs} inputs were less readable");
}
