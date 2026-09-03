// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §9.4 — the encoding is never longer than base64 — and the block rules of
//! §4 and §9, over a corpus rather than over the cases they were designed for.

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
    let mut v: Vec<(String, Vec<u8>)> = vec![("empty".into(), Vec::new())];
    for n in (0..140usize).chain([47, 48, 49, 95, 96, 97, 143, 144, 145, 4096, 4097]) {
        v.push((
            format!("noise {n}"),
            (0..n).map(|_| (next() & 0xff) as u8).collect(),
        ));
        v.push((
            format!("text {n}"),
            (0..n).map(|i| b"abcdefghij"[i % 10]).collect(),
        ));
        v.push((
            format!("prose {n}"),
            (0..n)
                .map(|i| {
                    if i % 6 == 5 {
                        b' '
                    } else {
                        b'a' + (i % 26) as u8
                    }
                })
                .collect(),
        ));
    }
    for percent in [0usize, 1, 5, 20, 44, 45, 50, 56, 57, 80, 100] {
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
    v
}

/// §9.4, the sentence the whole case for switching rests on: never longer
/// than base64, per input rather than on average, in both profiles and at
/// both entry points. It holds block by block, because each block takes the
/// shortest of three forms and base64 is one of them.
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
                assert_eq!(decode(&out, profile).unwrap().bytes, data, "{name}, {kind}");
            }
        }
    }
}

/// Where nothing is to be found the stream is base64url exactly, not merely
/// as long: the same bytes.
#[test]
fn high_entropy_input_is_base64url_byte_for_byte() {
    let mut s: u32 = 0x9e37_79b9;
    let data: Vec<u8> = (0..100_000)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s & 0xff) as u8
        })
        .collect();
    assert_eq!(encode(&data), encode_base64url(&data));
}

/// Pure profile-legal text costs two characters per 48 bytes: 50/48.
#[test]
fn the_density_bound_for_pure_text_is_exact() {
    for k in [1usize, 2, 10, 100] {
        let data = vec![b'a'; k * BLOCK_BYTES];
        assert_eq!(encode(&data).len(), k * (BLOCK_BYTES + 2), "{k} blocks");
    }
}

/// Blocks are independent: the encoding of a concatenation of whole blocks is
/// the concatenation of the encodings. That is what "no state" means, and it
/// is also what makes the encoder trivially parallel and streamable.
#[test]
fn whole_blocks_encode_independently() {
    let mut s: u32 = 0x51ce_1234;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    };
    let blocks: Vec<Vec<u8>> = (0..40)
        .map(|_| {
            (0..BLOCK_BYTES)
                .map(|_| {
                    if next() % 4 == 0 {
                        (next() & 0xff) as u8
                    } else {
                        b"abcdefghij .,"[(next() % 13) as usize]
                    }
                })
                .collect()
        })
        .collect();
    let whole: Vec<u8> = blocks.concat();
    let joined: Vec<u8> = blocks.iter().flat_map(|b| encode(b)).collect();
    assert_eq!(encode(&whole), joined);
    assert_eq!(decode(&joined, Profile::U).unwrap().bytes, whole);
}

/// The three forms all occur on ordinary input, and each decodes.
#[test]
fn every_form_occurs_and_round_trips() {
    let mut seen = [false; 3];
    for (_, data) in corpus() {
        for block in data.chunks(BLOCK_BYTES) {
            let mask = (0..block.len())
                .filter(|&i| Profile::U.allows(block[i]))
                .fold(0u64, |m, i| m | 1 << i);
            let (form, _) = choose(block.len(), mask);
            seen[form as usize] = true;
        }
    }
    assert_eq!(seen, [true; 3], "base64, raw, mask");
}

/// A raw or base64 tail runs to the end of the stream, and a decoder must
/// not read the block after it as the block after it: there is none.
#[test]
fn a_short_last_block_is_the_last_block() {
    for n in 1..=48usize {
        let data: Vec<u8> = (0..48 + n).map(|i| b"abcdefghij"[i % 10]).collect();
        let out = encode(&data);
        // A tail under four bytes is shorter as base64 (§9.1).
        let tail = if n < 4 { base64_len(n) } else { n + 2 };
        assert_eq!(out.len(), 50 + tail, "n = {n}");
        assert_eq!(decode(&out, Profile::U).unwrap().bytes, data);
        // And a second stream appended is not silently read as a tail.
        let two = [out.clone(), out.clone()].concat();
        if n < 48 {
            assert_ne!(
                decode(&two, Profile::U).map(|d| d.bytes),
                Ok([data.clone(), data.clone()].concat())
            );
        }
    }
}
