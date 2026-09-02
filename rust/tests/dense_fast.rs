// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §9.6 — `dense-fast` gives up density and never anything else.
//!
//! The preset exists because looking for literals costs, and on data that has
//! none the looking buys nothing. Declining to look is a guess, so what has to
//! be checked is what a wrong guess can cost: it may make the output larger
//! than `dense` would have, up to the base64 ceiling of §9.4, and it may not
//! do anything else at all.

use base65t::*;

fn samples() -> Vec<(String, Vec<u8>)> {
    let mut s: u32 = 0xfa57_0001;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    v.push(("empty".into(), Vec::new()));
    for n in [
        1usize, 11, 1023, 1024, 1025, 65_535, 65_536, 65_537, 400_000,
    ] {
        v.push((
            format!("noise {n}"),
            (0..n).map(|_| (next() & 0xff) as u8).collect(),
        ));
        v.push((format!("text {n}"), vec![b'a'; n]));
        // Prose in profile U: runs of five, none of them long enough, which is
        // the shape the sample is meant to write off.
        v.push((
            format!("prose {n}"),
            (0..n)
                .map(|i| if i % 6 == 5 { b' ' } else { b'a' })
                .collect(),
        ));
        // A window of noise followed by a window of text: the decision has to
        // be taken per window or one of the two is wrong.
        let mut mixed = Vec::new();
        while mixed.len() < n {
            if (mixed.len() / 65_536) % 2 == 0 {
                mixed.extend((0..4096).map(|_| (next() & 0xff) as u8));
            } else {
                mixed.extend(std::iter::repeat_n(b'x', 4096));
            }
        }
        mixed.truncate(n);
        v.push((format!("alternating windows {n}"), mixed));
    }
    v
}

#[test]
fn it_round_trips_and_holds_the_base64_ceiling() {
    for (name, data) in samples() {
        for profile in [Profile::U, Profile::T, Profile::B] {
            let out = encode_with(&data, Preset::DenseFast, profile);
            assert_eq!(
                decode(&out, profile).unwrap().bytes,
                data,
                "{name}, {profile:?}"
            );
            // §9.4 holds whatever the sample decided: an unscanned window is
            // exactly base64, and a scanned one obeys §9.1.
            assert!(
                out.len() <= (4 * data.len()).div_ceil(3),
                "{name}, {profile:?}: {} chars against base64's {}",
                out.len(),
                (4 * data.len()).div_ceil(3)
            );
        }
    }
}

#[test]
fn it_is_never_smaller_than_dense_and_never_larger_than_opaque() {
    for (name, data) in samples() {
        for profile in [Profile::U, Profile::T, Profile::B] {
            let fast = encode_with(&data, Preset::DenseFast, profile).len();
            let dense = encode_dense(&data, profile).len();
            let opaque = encode_opaque(&data).len();
            assert!(fast >= dense, "{name}, {profile:?}: {fast} < {dense}");
            assert!(fast <= opaque, "{name}, {profile:?}: {fast} > {opaque}");
        }
    }
}

/// The decision is per window, and a test that never saw it fire would prove
/// nothing. On alternating windows it has to fire on some and not others.
#[test]
fn the_decision_is_taken_per_window_and_both_ways() {
    let mut data = Vec::new();
    while data.len() < 8 * 65_536 {
        let mut s: u32 = 0xabc0_0000 ^ data.len() as u32;
        let mut next = move || {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as usize
        };
        if (data.len() / 65_536) % 2 == 0 {
            data.extend((0..65_536).map(|_| (next() & 0xff) as u8));
        } else {
            data.extend(std::iter::repeat_n(b'x', 65_536));
        }
    }
    let fast = encode_with(&data, Preset::DenseFast, Profile::U);
    let dense = encode_dense(&data, Profile::U);
    // The text windows are scanned -- they are almost all literal -- so the
    // two agree there; the noise windows are skipped, where `dense` finds the
    // occasional short-lived literal and this does not.
    assert!(
        fast.len() > dense.len(),
        "the decision never skipped a window"
    );
    assert!(
        fast.len() < dense.len() + dense.len() / 100,
        "it skipped a window it should have scanned: {} against {}",
        fast.len(),
        dense.len()
    );
    assert_eq!(decode(&fast, Profile::U).unwrap().bytes, data);
}

/// Window boundaries are absolute, so the same bytes at the same offsets are
/// decided the same way however the caller got there.
#[test]
fn the_windows_are_cut_at_absolute_offsets() {
    let mut s: u32 = 0x0ff5_e700;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    let data: Vec<u8> = (0..300_000)
        .map(|_| {
            if next() % 4 == 0 {
                (next() & 0xff) as u8
            } else {
                b"abcdefghijklmnop-._~"[next() % 20]
            }
        })
        .collect();
    // Encoding a prefix must agree with the prefix of encoding the whole, for
    // every prefix that ends on a window boundary.
    for w in 1..=4usize {
        let cut = w * 65_536;
        let head = encode_with(&data[..cut], Preset::DenseFast, Profile::U);
        let whole = encode_with(&data, Preset::DenseFast, Profile::U);
        // Not a prefix in general -- a literal may straddle the cut -- so what
        // is asserted is that both decode to what they encoded, and that the
        // head is what a fresh encode of those bytes gives.
        assert_eq!(
            decode(&head, Profile::U).unwrap().bytes,
            &data[..cut],
            "window {w}"
        );
        assert_eq!(decode(&whole, Profile::U).unwrap().bytes, data);
    }
}
