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
