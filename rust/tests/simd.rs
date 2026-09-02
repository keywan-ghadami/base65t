// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The `simd` feature is a switch on how fast, never on what.
//!
//! It replaces the writer for a base64 run, so the thing to check is that the
//! stream is the same one either way. The comparison cannot be against the
//! other build -- only one is compiled -- so it is against base64 written out
//! here, plainly, from RFC 4648 §5. That makes this test worth running in the
//! default build too: it is then the scalar writer being checked against a
//! second reading of the same paragraph.

use base65t::*;

/// Base64URL without padding, written to be read rather than to be quick.
fn base64url(data: &[u8]) -> Vec<u8> {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = Vec::new();
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..c.len() + 1 {
            out.push(A[(n >> (18 - 6 * k)) as usize & 63]);
        }
    }
    out
}

fn samples() -> Vec<(String, Vec<u8>)> {
    let mut s: u32 = 0x05ee_d51d;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    // Every length across the threshold the writer switches at, and across the
    // vector width it switches to.
    for n in 0..200usize {
        v.push((
            format!("noise {n}"),
            (0..n).map(|_| (next() & 0xff) as u8).collect(),
        ));
    }
    for n in [1000usize, 4095, 4096, 4097, 65_536, 200_000] {
        v.push((
            format!("noise {n}"),
            (0..n).map(|_| (next() & 0xff) as u8).collect(),
        ));
        // Mixed, so that `dense` writes many base64 runs of many lengths
        // rather than one long one.
        v.push((
            format!("mixed {n}"),
            (0..n)
                .map(|_| {
                    if next() % 3 == 0 {
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

#[test]
fn the_base64_written_is_the_base64_of_rfc_4648() {
    for (name, data) in samples() {
        assert_eq!(encode_opaque(&data), base64url(&data), "{name}");
    }
}

#[test]
fn and_every_preset_still_round_trips_and_holds_section_9_4() {
    for (name, data) in samples() {
        for profile in [Profile::U, Profile::T, Profile::B] {
            let dense = encode_dense(&data, profile);
            assert_eq!(decode(&dense, profile).unwrap().bytes, data, "{name}");
            assert!(
                dense.len() <= (4 * data.len()).div_ceil(3),
                "{name}, {profile:?}"
            );
            let framed = encode_framed(&data, profile);
            assert_eq!(decode(&framed, profile).unwrap().bytes, data, "{name}");
        }
    }
}
