// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §9.2.1 — the linear rule is what the specification says, not what the
//! implementation found convenient.
//!
//! The encoder skips ahead in windows of `L_min`: a literal that long has to
//! cover the last byte of the window, so one byte outside the profile there
//! rules out every start in it. That is an argument, and an argument is worth
//! exactly as much as the check that follows it. This transcribes §9.2.1 the
//! way one would read it aloud -- one byte at a time, no windows, no
//! cleverness -- and requires the two to agree byte for byte.
//!
//! It is deliberately the slow version. If the fast one is ever wrong, this is
//! the test that says so, and the input it says it on is small enough to read.

use base65t::internals::{segment_greedy, Rules, Seg};
use base65t::*;

/// §9.2.1 read literally: at every position, the longest admissible run; take
/// it if it reaches the threshold, otherwise move on by one byte.
fn transcribed(data: &[u8], profile: Profile, lmin: usize, framed: bool) -> Vec<Seg> {
    const MAX_LITERAL: usize = 4158;
    const TILDE: u8 = b'~';
    let n = data.len();
    let mut segs = Vec::new();
    let mut pending = 0;
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n && j - i < MAX_LITERAL && profile.allows(data[j]) {
            if framed && data[j] == TILDE && j + 1 < n && data[j + 1] == b'A' {
                break;
            }
            j += 1;
        }
        if framed {
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
            i += 1;
        }
    }
    if pending < n {
        segs.push(Seg::Base64(pending, n));
    }
    segs
}

fn samples() -> Vec<(String, Vec<u8>)> {
    let mut s: u32 = 0x1bad_c0de;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    v.push(("empty".into(), Vec::new()));
    // Short inputs first: every length up to twice the threshold, so that a
    // window that overshoots the end of the input has nowhere to hide.
    for n in 0..64usize {
        v.push((
            format!("noise {n}"),
            (0..n).map(|_| (next() & 0xff) as u8).collect(),
        ));
        v.push((format!("text {n}"), vec![b'a'; n]));
        v.push((
            format!("tildes {n}"),
            (0..n)
                .map(|i| if i % 3 == 0 { b'~' } else { b'A' })
                .collect(),
        ));
    }
    // Runs of every length around the threshold, separated by one byte the
    // profile rejects: this is where an off-by-one in the window shows.
    for run in 1..40usize {
        let mut b = Vec::new();
        for _ in 0..12 {
            b.extend(std::iter::repeat_n(b'x', run));
            b.push(0x80);
        }
        v.push((format!("runs of {run}"), b));
    }
    for percent in [1usize, 10, 50, 90, 99] {
        v.push((
            format!("{percent}% transportable"),
            (0..9000)
                .map(|_| {
                    if next() % 100 < percent {
                        b"abcXYZ019.-_~=/ \t"[next() % 17]
                    } else {
                        (next() & 0xff) as u8
                    }
                })
                .collect(),
        ));
    }
    v
}

#[test]
fn the_window_finds_the_literals_the_specification_names() {
    for (name, data) in samples() {
        for profile in [Profile::U, Profile::T, Profile::B] {
            for lmin in [1usize, 2, 3, 10, 11, 12, 32] {
                for framed in [false, true] {
                    let rules = Rules::preset(profile, Some(lmin), framed);
                    assert_eq!(
                        segment_greedy(&data, rules),
                        transcribed(&data, profile, lmin, framed),
                        "{name}, {profile:?}, L_min = {lmin}, framed = {framed}"
                    );
                }
            }
        }
    }
}

#[test]
fn and_so_the_streams_agree_too() {
    // The segmentation is what is compared above; this is the stream a caller
    // actually gets, through the public entry points, on the same data.
    for (name, data) in samples() {
        for profile in [Profile::U, Profile::T, Profile::B] {
            let dense = encode_dense(&data, profile);
            assert_eq!(
                decode_plain(&dense, profile).unwrap().bytes,
                data,
                "{name}, {profile:?}"
            );
            let framed = encode_framed(&data, profile);
            assert_eq!(
                decode_framed(&framed, profile).unwrap().bytes,
                data,
                "{name}, {profile:?}, framed"
            );
        }
    }
}
