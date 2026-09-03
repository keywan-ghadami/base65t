// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §16.8 — every one of the nine error codes in §10.4, raised on purpose.
//!
//! A table of error codes nothing produces is a table of error codes nobody
//! has checked. The last test is the other half of the same concern: the
//! decoder reads attacker-controlled input (§14), so it is run over arbitrary
//! bytes with the only requirement that it return rather than panic, allocate
//! wildly or read past the end.

use base65t::*;

#[test]
fn all_nine_codes() {
    type Case = (Error, &'static str, Result<Vec<u8>, Error>);
    let cases: [Case; 9] = [
        (
            Error::TrailingTilde,
            "E_TRAILING_TILDE",
            decode([encode_base64url([9u8; 48]).into_bytes(), b"~".to_vec()].concat()),
        ),
        (Error::Reserved, "E_RESERVED", decode(b"~Aabc")),
        (Error::Profile, "E_PROFILE", decode(b"~~a b")),
        (Error::Align, "E_ALIGN", decode(b"abcde")),
        (Error::NonzeroTail, "E_NONZERO_TAIL", decode(b"YWxpY2V")),
        (Error::Charset, "E_CHARSET", decode(b"YW*j")),
        (Error::Padding, "E_PADDING", decode(b"YWxp==")),
        (
            Error::MixedAlphabet,
            "E_MIXED_ALPHABET",
            decode(b"PDw_Pz8+Pg"),
        ),
        (
            Error::NonUrlAlphabet,
            "E_NON_URL_ALPHABET",
            decode_url_strict(b"PDw/Pz8+Pg"),
        ),
    ];
    for (expected, code, got) in cases {
        assert_eq!(got, Err(expected), "{code}");
        assert_eq!(expected.code(), code);
        assert_eq!(expected.to_string(), code);
    }
}

/// There is no length in this format a sender chooses, and so there is no
/// truncation error: a raw tail and a base64 tail both run to the end of the
/// stream, whatever that is.
#[test]
fn nothing_can_be_truncated() {
    let data: Vec<u8> = (0..100).map(|i| b"abcdefghij"[i % 10]).collect();
    let out = encode(&data);
    for cut in 0..=out.len() {
        let r = decode(&out[..cut]);
        match r {
            Ok(d) => assert!(data.starts_with(&d), "cut {cut}"),
            Err(e) => assert!(
                matches!(e, Error::TrailingTilde | Error::Align | Error::NonzeroTail),
                "cut {cut}: {e}"
            ),
        }
    }
}

/// Arbitrary bytes in, an answer out. No panic, no overrun, no runaway.
#[test]
fn arbitrary_input_decodes_or_errors() {
    let mut s: u32 = 0x0bad_c0de;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    };
    let pool: Vec<u8> = b"ABCabc012-_+/=~\x00\xff \t\"\\.".to_vec();
    for _ in 0..20_000 {
        let n = (next() % 140) as usize;
        let data: Vec<u8> = (0..n).map(|_| pool[next() as usize % pool.len()]).collect();
        {
            for f in [
                (|s: &[u8]| decode(s)) as fn(&[u8]) -> Result<Vec<u8>, Error>,
                |s: &[u8]| decode_url_strict(s),
            ] {
                if let Ok(d) = f(&data) {
                    // Whatever came out has to be no larger than the stream
                    // could describe: a raw byte is one per character and
                    // base64 is three per four.
                    assert!(d.len() <= data.len(), "{data:?}");
                }
            }
        }
    }
}
