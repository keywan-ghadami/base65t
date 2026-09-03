// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `encode_into` and `decode_into` write where the caller says, and nowhere
//! else write anything different.
//!
//! The point of them is an allocation the caller can avoid — worth a fifth of
//! encoding a sixteen-byte value — and a buffer the caller may have registered
//! with the kernel. Neither is a reason to accept different bytes, so what is
//! checked is that there are none: the same output as the owning call, into a
//! buffer that already has something in it, and nothing left behind on error.

use base65t::*;

fn samples() -> Vec<Vec<u8>> {
    let mut s: u32 = 0x1a7e_b0f5;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    let mut v = vec![Vec::new()];
    for n in [1usize, 11, 16, 64, 155, 4096, 70_000] {
        v.push((0..n).map(|_| (next() & 0xff) as u8).collect());
        v.push((0..n).map(|_| b"abcdef-._~ \t\"\\"[next() % 14]).collect());
    }
    v
}

#[test]
fn encode_into_appends_exactly_what_encode_returns() {
    for data in samples() {
        {
            let want = encode(&data).into_bytes();
            // Into an empty buffer, and into one that already holds
            // something: appending is the contract, not overwriting.
            let mut fresh = Vec::new();
            encode_into(&data, &mut fresh);
            assert_eq!(fresh, want, "{} bytes", data.len());

            let mut used = b"already here".to_vec();
            encode_into(&data, &mut used);
            assert_eq!(&used[..12], b"already here");
            assert_eq!(&used[12..], &want[..]);
        }
    }
}

#[test]
fn decode_into_appends_exactly_what_decode_returns() {
    for data in samples() {
        {
            for stream in [
                encode(&data).into_bytes(),
                encode_base64url(&data).into_bytes(),
            ] {
                let want = decode_detailed(&stream).unwrap();

                let mut used = b"already here".to_vec();
                let meta = decode_into(&stream, &mut used).unwrap();
                assert_eq!(&used[..12], b"already here");
                assert_eq!(&used[12..], &want.bytes[..]);
                assert_eq!(meta.alphabet_seen, want.alphabet_seen);
                assert_eq!(meta.padding_seen, want.padding_seen);
            }
        }
    }
}

/// A rejected stream must leave the caller's buffer as it found it. The
/// decoder writes as it goes, so this is a real risk and not a formality.
#[test]
fn a_rejected_stream_leaves_the_buffer_alone() {
    let bad: Vec<Vec<u8>> = vec![
        b"~".to_vec(),                             // E_TRAILING_TILDE
        b"~A".to_vec(),                            // E_RESERVED_LEN
        [b"A".repeat(65), b"*".to_vec()].concat(), // E_CHARSET, past the fast path
        b"A".repeat(65),                           // E_ALIGN, past the fast path
        b"YWxpY2V".to_vec(),                       // E_NONZERO_TAIL
        b"~Labcdefghij".to_vec(),                  // E_TRUNCATED
    ];
    for stream in bad {
        {
            let mut buf = b"untouched".to_vec();
            let r = decode_into(&stream, &mut buf);
            assert!(r.is_err(), "{stream:?} decoded");
            assert_eq!(buf, b"untouched", "{stream:?} left something behind");
        }
    }
}
