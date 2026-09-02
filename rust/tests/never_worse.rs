// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §9.4 — the encoding is never longer than base64, and §12's two exact figures.
//!
//! The guarantee is what makes the format a drop-in rather than a trade, so it
//! is checked over everything the round-trip corpus holds and not only over
//! the cases it was designed for. There is no exemption left: v0.2 exempted
//! `framed`, and v0.4 removed framing rather than keep a mode the guarantee
//! did not cover.

use base65t::internals::{costs, emit, segment_with, LiteralEnd, Rules};
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

/// §9.4, the sentence the whole case for switching rests on: the encoding is
/// never longer than base64, per input rather than on average.
///
/// One encoder now, so this is a statement about the format and not about a
/// preset. It holds in both modes §9.6 can pick — the exact programme keeps
/// the all-base64 candidate in its search space, and the base64url mode *is*
/// that candidate — so it is checked over everything the corpus holds rather
/// than only over the cases it was designed for.
#[test]
fn the_encoding_is_never_longer_than_base64() {
    for (name, data) in corpus() {
        for profile in [Profile::U, Profile::T] {
            for (kind, out) in [
                ("encode", encode_with(&data, profile)),
                ("base64url", encode_base64url(&data)),
            ] {
                assert!(
                    out.len() <= base64_len(data.len()),
                    "{name}, {kind}, {profile:?}: {} > {}",
                    out.len(),
                    base64_len(data.len())
                );
            }
        }
    }
}

/// §12, second row: a literal segment carries 4158 bytes for 4162 characters,
/// and that ratio is a bound rather than a limit — long input approaches it
/// from below and never reaches 1.
#[test]
fn the_density_bound_for_pure_literal_text_is_exact() {
    let n = MAX_LITERAL;
    let data = vec![b'a'; n];
    let out = encode_with(&data, Profile::U);
    assert_eq!(out.len(), 4162);
    assert_eq!(out.len() as f64 / n as f64, 4162.0 / 4158.0);

    // Four full segments back to back, which is also four window boundaries
    // crossed without one of them costing a character.
    for k in 1..=4 {
        let data = vec![b'a'; k * n];
        let out = encode_with(&data, Profile::U);
        assert_eq!(out.len(), k * 4162, "{k} full literal segments");
    }
}

/// §12, first row: high-entropy input is base64 exactly, because nothing else
/// is shorter — and §9.6 reaches the same stream by not looking at all.
#[test]
fn high_entropy_input_encodes_at_the_base64_ratio() {
    let data = noise(10_000);
    let out = encode_with(&data, Profile::U);
    assert_eq!(out.len(), base64_len(data.len()));
    assert_eq!(out, encode_base64url(&data));
    assert_eq!(classify(&data), Mode::Base64);
}

/// §9.6 decides once, at the head, and the decision is a function of the
/// input: the same bytes classify the same way whatever precedes the call.
#[test]
fn classification_is_a_function_of_the_head() {
    // Compressed containers are named by their magic number, before entropy
    // is even looked at -- a short gzip header is too little to measure.
    let gzip = [b"\x1f\x8b\x08\x00".to_vec(), vec![b'a'; 40]].concat();
    assert_eq!(classify(&gzip), Mode::Base64);
    // Text is text at any length past the sample, and below it too.
    assert_eq!(classify(b"alice.jones"), Mode::Exact);
    assert_eq!(classify(&vec![b'a'; 100_000]), Mode::Exact);
    // And the sample is a prefix: what follows it cannot change the answer.
    let head = vec![b'a'; SAMPLE_BYTES];
    let mut long = head.clone();
    long.extend(noise(100_000));
    assert_eq!(classify(&long), classify(&head));
}

/// The case the benchmark found within a minute of being pointed at a real
/// file, kept because the window boundary reopens exactly this door.
///
/// The exact programme runs over windows of [`WINDOW_BYTES`], and a window
/// whose last segment is a base64 run of `k` bytes with `k mod 3 != 0` leaves
/// a partial quantum that the next window's run continues -- two adjacent
/// base64 segments are one segment to a decoder (§4), so a seam that is not
/// merged decodes to what neither window meant. `segment_windowed` merges
/// them; this holds the door shut, and the length assertion is the second
/// half: an unmerged seam also costs a character, which §9.4 does not allow.
///
/// Homogeneous input cannot show that class of bug: noise makes one base64
/// run, and profile-legal text makes none. It needs a mixture, which is what
/// every real file is.
#[test]
fn large_mixed_input_survives_every_seam() {
    let data = mixed(300_000);
    for profile in [Profile::U, Profile::T] {
        let out = encode_with(&data, profile);
        let back = decode(&out, profile).unwrap_or_else(|e| panic!("{profile:?}: {e}"));
        assert_eq!(back.bytes, data, "{profile:?}");
        assert!(out.len() <= base64_len(data.len()), "{profile:?}");
    }
    // High entropy leaves no literal worth taking, so the stream is base64url
    // exactly -- at any length.
    let noise = noise(200_000);
    assert_eq!(encode_with(&noise, Profile::U), encode_base64url(&noise));

    // Every offset around a window boundary, since the seam is where the two
    // halves of the merge meet.
    for n in [
        WINDOW_BYTES - 2,
        WINDOW_BYTES - 1,
        WINDOW_BYTES,
        WINDOW_BYTES + 1,
        2 * WINDOW_BYTES - 1,
        2 * WINDOW_BYTES,
    ] {
        let d = &data[..n];
        assert_eq!(
            decode(&encode_with(d, Profile::U), Profile::U)
                .unwrap()
                .bytes,
            d,
            "n = {n}"
        );
        assert!(encode_with(d, Profile::U).len() <= base64_len(n), "n = {n}");
    }
}

/// What windowing costs against the same programme run over the whole input.
///
/// The windows are what make the encoder O(1) in memory on an input of any
/// size, and the question that buys is how much density it spends. A literal
/// cannot span a boundary, so the bound is one header per boundary -- four
/// characters per 64 KiB, under 0,01 %. This measures the real figure, which
/// is smaller still, and asserts the bound.
#[test]
fn windowing_costs_almost_nothing() {
    for (name, data) in [
        ("mixed", mixed(300_000)),
        (
            "text",
            (0..300_000)
                .map(|i| b"abcdefghij"[i % 10])
                .collect::<Vec<u8>>(),
        ),
    ] {
        for profile in [Profile::U, Profile::T] {
            let windowed = encode_with(&data, profile).len();
            // The same programme over the whole input, through the pieces the
            // windowed encoder is built from: this compares the rule against
            // itself, not against a second implementation of it.
            let r = Rules::new(profile, Some(1));
            let c = costs(&data, r);
            let whole = emit(&data, &segment_with(&data, r, &c, LiteralEnd::KeyOrder)).len();

            assert!(
                windowed >= whole,
                "{name}: the whole-input rule cannot lose"
            );
            let boundaries = data.len() / WINDOW_BYTES;
            assert!(
                windowed - whole <= 4 * boundaries,
                "{name}, {profile:?}: {} characters over {boundaries} boundaries",
                windowed - whole
            );
            println!(
                "{name} {profile:?}: {windowed} vs {whole}, {:.4} %",
                100.0 * (windowed as f64 / whole as f64 - 1.0)
            );
        }
    }
}

/// Deterministic inputs the tests above share.
fn mixed(n: usize) -> Vec<u8> {
    let mut s: u32 = 0x51ce_1234;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    };
    let mut data: Vec<u8> = Vec::with_capacity(n);
    while data.len() < n {
        let run = 1 + (next() % 60) as usize;
        if next() % 3 == 0 {
            data.extend((0..run).map(|_| (next() & 0xff) as u8));
        } else {
            data.extend((0..run).map(|i| b"abcdefghij"[i % 10]));
        }
    }
    data.truncate(n);
    data
}

fn noise(n: usize) -> Vec<u8> {
    let mut s: u32 = 0x9e37_79b9;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s & 0xff) as u8
        })
        .collect()
}
