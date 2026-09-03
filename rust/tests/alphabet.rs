// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The head of the specification claims the encoder writes exactly the 66
//! characters of RFC 3986 *unreserved*, whatever the input, and every
//! container statement it makes rests on that one sentence. So it is checked
//! here rather than reasoned about: the set is built from what the encoder
//! actually emits and compared both ways.

use base65t::*;
use std::collections::BTreeSet;

fn unreserved() -> BTreeSet<u8> {
    (b'A'..=b'Z')
        .chain(b'a'..=b'z')
        .chain(b'0'..=b'9')
        .chain(*b"-._~")
        .collect()
}

/// Both directions, because each catches a different failure: a character
/// outside the set breaks every container claim in the head of the document,
/// and a character of the set the encoder can never write would mean the
/// document names a larger alphabet than the format has.
#[test]
fn the_output_alphabet_is_exactly_unreserved() {
    let mut seen = BTreeSet::new();
    let mut r: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        r ^= r << 13;
        r ^= r >> 7;
        r ^= r << 17;
        (r >> 24) as u8
    };
    for n in 0..3000usize {
        // Binary input, so every block is base64 and the base64 writer's own
        // alphabet is covered, tails included.
        let data: Vec<u8> = (0..n).map(|_| next()).collect();
        seen.extend(encode(&data));
        seen.extend(encode_base64url(&data));
        // Input the profile admits throughout, so raw blocks appear and the
        // marker and the admitted bytes are covered.
        let text: Vec<u8> = (0..n).map(|i| b"aZ0-._~"[i % 7]).collect();
        seen.extend(encode(&text));
    }
    let want = unreserved();
    assert_eq!(
        seen.difference(&want).copied().collect::<Vec<_>>(),
        Vec::<u8>::new(),
        "the encoder wrote a character outside RFC 3986 unreserved"
    );
    assert_eq!(
        want.difference(&seen).copied().collect::<Vec<_>>(),
        Vec::<u8>::new(),
        "the document names a character the encoder never writes"
    );
    assert_eq!(seen.len(), 66);
}

/// §5.1: the encoder never produces padding, which is what keeps `=` out of
/// the set above. Stated separately because it is the one character a reader
/// would expect to find in a base64-derived alphabet -- and because with one
/// alphabet, `=` never being written is what makes it unambiguous wherever it
/// does appear in a stream (§5.3).
#[test]
fn no_output_ever_carries_padding() {
    let mut r: u64 = 0x9E37_79B9_7F4A_7C15;
    for n in 0..3000usize {
        let data: Vec<u8> = (0..n)
            .map(|_| {
                r ^= r << 13;
                r ^= r >> 7;
                r ^= r << 17;
                (r >> 24) as u8
            })
            .collect();
        assert!(!encode(&data).contains(&b'='), "{n} bytes");
        assert!(!encode_base64url(&data).contains(&b'='));
    }
}
