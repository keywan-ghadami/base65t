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

/// The vectorised path only runs on long runs, and every error test in the
/// suite is short — so without this, the branch that matters here is the one
/// nothing exercises. Each case is padded past the threshold and must come
/// back with the same verdict a short one does.
#[test]
fn a_long_run_is_judged_the_same_as_a_short_one() {
    // 64 characters of clean base64url to get past the dispatch threshold.
    let pad = "A".repeat(64);

    let cases: Vec<(String, Result<usize, Error>)> = vec![
        // Clean, and long: the case the fast path exists for.
        (format!("{pad}{pad}"), Ok(96)),
        // A character outside both alphabets, well inside the run.
        (format!("{pad}A*A{pad}"), Err(Error::Charset)),
        // `=` is not an alphabet character and this is not the stream end.
        (format!("{pad}A=A{pad}"), Err(Error::Charset)),
        // A run whose length is 1 mod 4 carries a character that decodes to
        // nothing (§5).
        (format!("{pad}{pad}A"), Err(Error::Align)),
        // Both alphabets in one stream is Rule A (§5.4), and it is caught
        // before anything is decoded.
        (format!("{pad}-{pad}+"), Err(Error::MixedAlphabet)),
    ];

    for (stream, want) in cases {
        let got = decode(stream.as_bytes(), Profile::U);
        match (&got, &want) {
            (Ok(d), Ok(n)) => assert_eq!(d.bytes.len(), *n, "{stream:.20}…"),
            (Err(e), Err(w)) => assert_eq!(e, w, "{stream:.20}…"),
            _ => panic!("{stream:.20}…: got {got:?}, wanted {want:?}"),
        }
    }
}

/// Rule A over a long run: the variant has to be reported, and the classic
/// alphabet has to decode to the same bytes as the URL one.
#[test]
fn a_long_run_reports_which_alphabet_it_was_written_in() {
    let data: Vec<u8> = (0..=255u8).chain(0..=255u8).collect();
    let url = encode_opaque(&data);
    let classic: Vec<u8> = url
        .iter()
        .map(|&c| match c {
            b'-' => b'+',
            b'_' => b'/',
            other => other,
        })
        .collect();
    assert!(url.len() > 64 && url.iter().any(|&c| c == b'-' || c == b'_'));

    let a = decode(&url, Profile::U).unwrap();
    let b = decode(&classic, Profile::U).unwrap();
    assert_eq!(a.bytes, data);
    assert_eq!(b.bytes, data);
    assert_eq!(a.alphabet_seen, AlphabetSeen::Url);
    assert_eq!(b.alphabet_seen, AlphabetSeen::Classic);
    // And §5.5's strict entry point turns the classic one down.
    assert_eq!(
        decode_url_strict(&classic, Profile::U),
        Err(Error::NonUrlAlphabet)
    );
    assert!(decode_url_strict(&url, Profile::U).is_ok());
}
