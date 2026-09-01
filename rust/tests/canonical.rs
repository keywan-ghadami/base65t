// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §16.3 — `encode_canonical` against an implementation that shares nothing
//! with it.
//!
//! The specification asks for two independent implementations. This is the
//! honest half of that: the encoder under test computes a minimum with a
//! dynamic programme and a tie-break rule, and this file finds the same
//! minimum by writing out *every* valid segmentation, emitting each one with
//! its own encoder, and sorting by `Key` as §11.1 defines it. Nothing but the
//! definition is shared, and where the definition is ambiguous the two would
//! disagree rather than agree by construction.
//!
//! What it is not: a second implementation in another language, written by
//! somebody else. FINDINGS.md says so where it counts §16.3.

use base65t::{decode_plain, encode_canonical, Profile};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// A segmentation, as the specification's three-symbol vector (§11.1).
type CVec = Vec<u8>;

/// Every segmentation of `data` the profile admits, as `(c_vector, output)`.
///
/// Base64 runs are maximal (§4) — the vector cannot express a split one, and
/// this builds the output from the vector, so it cannot produce one either.
/// Adjacent literals are expressible and are enumerated.
fn all_segmentations(data: &[u8], profile: Profile) -> Vec<(CVec, Vec<u8>)> {
    fn walk(data: &[u8], profile: Profile, i: usize, c: &mut CVec, out: &mut Vec<(CVec, Vec<u8>)>) {
        if i == data.len() {
            out.push((c.clone(), emit(data, c)));
            return;
        }
        c.push(b'B');
        walk(data, profile, i + 1, c, out);
        c.pop();

        let mut m = 1;
        while i + m <= data.len() && m <= 4158 {
            if !profile.allows(data[i + m - 1]) {
                break;
            }
            c.push(b'S');
            for _ in 1..m {
                c.push(b'L');
            }
            walk(data, profile, i + m, c, out);
            c.truncate(c.len() - m);
            m += 1;
        }
    }
    let mut out = Vec::new();
    walk(data, profile, 0, &mut Vec::new(), &mut out);
    out
}

/// The stream a three-symbol vector names (§11.1: `output(S)` is a function of
/// `c`). Written from the format description, not from the crate.
fn emit(data: &[u8], c: &CVec) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == b'B' {
            let mut j = i;
            while j < c.len() && c[j] == b'B' {
                j += 1;
            }
            out.extend_from_slice(&base64url(&data[i..j]));
            i = j;
        } else {
            assert_eq!(c[i], b'S');
            let mut j = i + 1;
            while j < c.len() && c[j] == b'L' {
                j += 1;
            }
            let m = j - i;
            out.push(b'~');
            if m <= 62 {
                out.push(ALPHABET[m]);
            } else {
                out.push(ALPHABET[63]);
                out.push(ALPHABET[(m - 63) >> 6 & 63]);
                out.push(ALPHABET[(m - 63) & 63]);
            }
            out.extend_from_slice(&data[i..j]);
            i = j;
        }
    }
    out
}

fn base64url(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in bytes.chunks(3) {
        let mut n: u32 = 0;
        for (k, &b) in chunk.iter().enumerate() {
            n |= (b as u32) << (16 - 8 * k);
        }
        let chars = chunk.len() + 1;
        for k in 0..chars {
            out.push(ALPHABET[(n >> (18 - 6 * k)) as usize & 63]);
        }
    }
    out
}

/// `Key(S) = (|output(S)|, c(S))`. `B < L < S` is also the ASCII order of the
/// three letters — 0x42, 0x4C, 0x53 — so the vectors compare as they stand
/// and nothing has to be mapped first.
fn key(entry: &(CVec, Vec<u8>)) -> (usize, CVec) {
    (entry.1.len(), entry.0.clone())
}

fn brute_force(data: &[u8], profile: Profile) -> (CVec, Vec<u8>) {
    all_segmentations(data, profile)
        .into_iter()
        .min_by_key(key)
        .expect("at least the all-base64 segmentation")
}

/// A counter stream, so a failure names an input that can be typed back in.
fn corpus(profile: Profile, alphabet: &[u8], max_len: usize) -> Vec<Vec<u8>> {
    let mut s: u32 = 0x5eed_9a31 ^ (profile as u32);
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    };
    let mut cases = vec![Vec::new()];
    for n in 1..=max_len {
        for _ in 0..40 {
            cases.push(
                (0..n)
                    .map(|_| alphabet[next() as usize % alphabet.len()])
                    .collect(),
            );
        }
    }
    cases
}

#[test]
fn canonical_is_the_minimum_of_key_profile_u() {
    let alphabet = b"ab.~ -_9";
    let mut ties = 0usize;
    for data in corpus(Profile::U, alphabet, 12) {
        let (_, expected) = brute_force(&data, Profile::U);
        let got = encode_canonical(&data, Profile::U);
        assert_eq!(
            String::from_utf8_lossy(&got),
            String::from_utf8_lossy(&expected),
            "input {data:?}"
        );
        let all = all_segmentations(&data, Profile::U);
        let best = all.iter().map(|e| e.1.len()).min().unwrap_or(0);
        if all.iter().filter(|e| e.1.len() == best).count() > 1 {
            ties += 1;
        }
    }
    // The interesting inputs are the ones where the length does not decide;
    // a run that found none would be proving nothing.
    assert!(ties > 50, "only {ties} inputs had a length tie");
}

#[test]
fn canonical_is_the_minimum_of_key_profiles_t_and_b() {
    for (profile, alphabet) in [
        (Profile::T, b"a=~\" x".as_slice()),
        (Profile::B, b"a~\x00\xff".as_slice()),
    ] {
        for data in corpus(profile, alphabet, 11) {
            let (_, expected) = brute_force(&data, profile);
            assert_eq!(
                encode_canonical(&data, profile),
                expected,
                "profile {profile:?}, input {data:?}"
            );
        }
    }
}

/// Whatever else canonical is, it has to decode back.
#[test]
fn canonical_round_trips() {
    for data in corpus(Profile::B, b"a~ =\x00\xfe".as_slice(), 14) {
        let out = encode_canonical(&data, Profile::B);
        let d = decode_plain(&out, Profile::B).expect("decodes");
        assert_eq!(d.bytes, data);
        assert!(!d.padding_seen);
    }
}

/// Long inputs, where the two header bands meet and the sliding windows have
/// something to slide over. Brute force cannot reach here; the check is that
/// the result decodes and is no longer than base64.
#[test]
fn canonical_on_long_inputs() {
    for n in [62, 63, 64, 124, 125, 4157, 4158, 4159, 4300, 9000] {
        let data: Vec<u8> = (0..n).map(|i| b"abcdefghij"[i % 10]).collect();
        let out = encode_canonical(&data, Profile::U);
        assert_eq!(decode_plain(&out, Profile::U).expect("decodes").bytes, data);
        assert!(out.len() <= (4 * n).div_ceil(3), "n = {n}");
        // A pure literal run costs its own length plus headers, and §6.1 caps
        // a segment at 4158 bytes.
        let headers = out.len() - n;
        assert!(headers <= 4 * n.div_ceil(4158) + 4, "n = {n}: {headers}");
    }
}
