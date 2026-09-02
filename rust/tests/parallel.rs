// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `encode_parallel` writes what `encode_dense` writes, at every thread count.
//!
//! The point of the function is that the thread count is invisible in the
//! output: §11.1 hangs cache keys on these bytes, so a stream that depended on
//! how many cores the machine had would be a different format on every
//! machine. That is one assertion, and everything here is that assertion under
//! conditions chosen to break it -- inputs whose literals sit exactly where the
//! cuts want to be, inputs with no literals at all, inputs just either side of
//! the size at which splitting turns on.

use base65t::*;

fn mixed(n: usize, text_percent: usize, run: usize) -> Vec<u8> {
    let mut s: u32 = 0xc0ff_ee01 ^ (n as u32) ^ ((text_percent as u32) << 8);
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    let mut v = Vec::with_capacity(n);
    while v.len() < n {
        let len = 1 + next() % run;
        if next() % 100 < text_percent {
            let c = b"abcdefghijklmnopqrstuvwxyz0123456789-._~"[next() % 40];
            v.extend(std::iter::repeat_n(c, len));
        } else {
            v.extend((0..len).map(|_| (next() & 0xff) as u8));
        }
    }
    v.truncate(n);
    v
}

#[test]
fn the_thread_count_never_reaches_the_output() {
    let cases: Vec<(String, Vec<u8>)> = vec![
        ("empty".into(), Vec::new()),
        ("one byte".into(), vec![b'a']),
        // Either side of the megabyte at which splitting turns on, and either
        // side of the quarter-megabyte window a cut is looked for in.
        ("just under".into(), mixed((1 << 20) - 1, 50, 40)),
        ("just over".into(), mixed((1 << 20) + 1, 50, 40)),
        ("four megabytes".into(), mixed(4 << 20, 50, 40)),
        // Literals as long as the format allows, so that a cut lands inside
        // one if the boundary rule is wrong.
        ("long literals".into(), mixed(3 << 20, 80, 9000)),
        // Prose: short runs, almost none of them taken.
        (
            "prose".into(),
            (0..(3 << 20))
                .map(|i| if i % 6 == 5 { b' ' } else { b'a' })
                .collect(),
        ),
        // No literal anywhere: the fallback path.
        ("noise".into(), mixed(3 << 20, 0, 64)),
        // Every byte legal: one literal chain, no cut point at all.
        ("all text".into(), vec![b'a'; 3 << 20]),
    ];

    for (name, data) in cases {
        for profile in [Profile::U, Profile::T, Profile::B] {
            let one = encode_dense(&data, profile);
            for threads in [1usize, 2, 3, 4, 7, 8, 16] {
                assert_eq!(
                    encode_parallel(&data, profile, threads),
                    one,
                    "{name}, {profile:?}, {threads} threads"
                );
            }
            // And `0`, which asks the machine how many cores it has.
            assert_eq!(
                encode_parallel(&data, profile, 0),
                one,
                "{name}, {profile:?}"
            );
            assert_eq!(
                decode(&one, profile).unwrap().bytes,
                data,
                "{name}, {profile:?}"
            );
        }
    }
}

/// A cut is only useful if it is actually taken; a test that silently fell
/// back to one thread everywhere would pass the assertion above and prove
/// nothing.
#[test]
fn the_cuts_are_really_taken() {
    let data = mixed(4 << 20, 50, 40);
    let one = encode_dense(&data, Profile::U);
    // Eight workers on four megabytes of mixed input: the encoder has to find
    // seven boundaries. If it found none the output would still match, so this
    // asserts on the timing-independent thing that would differ -- the split
    // happening at all -- by checking that each piece encodes on its own.
    let cuts = base65t::internals::cut_points(&data, Profile::U, 8);
    assert!(cuts.len() >= 7, "only {} cuts found", cuts.len());
    let mut joined = Vec::new();
    let mut prev = 0;
    for &c in cuts.iter().chain(std::iter::once(&data.len())) {
        joined.extend_from_slice(&encode_dense(&data[prev..c], Profile::U));
        prev = c;
    }
    assert_eq!(joined, one);
}
