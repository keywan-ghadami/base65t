// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §16.8 — every one of the twelve error codes in §10.4, raised on purpose.
//!
//! A table of error codes nothing produces is a table of error codes nobody
//! has checked. The last test is the other half of the same concern: the
//! decoder parses attacker-controlled lengths (§14), so it is run over
//! arbitrary bytes with the only requirement that it return rather than
//! panic, allocate wildly or read past the end.

use base65t::*;

#[test]
fn all_twelve_codes() {
    let cases: [(Error, &str, Result<Decoded, Error>); 12] = [
        (
            Error::TrailingTilde,
            "E_TRAILING_TILDE",
            decode(b"abc~", Profile::U),
        ),
        (
            Error::ReservedLen,
            "E_RESERVED_LEN",
            decode_plain(b"~AAAA", Profile::U),
        ),
        (Error::Truncated, "E_TRUNCATED", decode(b"~_A", Profile::U)),
        (Error::Profile, "E_PROFILE", decode(b"~Ca b", Profile::U)),
        (Error::Align, "E_ALIGN", decode(b"abcde", Profile::U)),
        (
            Error::NonzeroTail,
            "E_NONZERO_TAIL",
            decode(b"YWxpY2V", Profile::U),
        ),
        (Error::Charset, "E_CHARSET", decode(b"~~ab", Profile::U)),
        (Error::Padding, "E_PADDING", decode(b"YWxp==", Profile::U)),
        (
            Error::MixedAlphabet,
            "E_MIXED_ALPHABET",
            decode(b"PDw_Pz8+Pg", Profile::U),
        ),
        (
            Error::NonUrlAlphabet,
            "E_NON_URL_ALPHABET",
            decode_url_strict(b"PDw/Pz8+Pg", Profile::U),
        ),
        (
            Error::FrameRule,
            "E_FRAME_RULE",
            decode(b"~AAAC~A", Profile::U),
        ),
        (
            Error::FrameSync,
            "E_FRAME_SYNC",
            decode_framed(b"YWxpY2U", Profile::U),
        ),
    ];
    for (expected, code, got) in cases {
        assert_eq!(got, Err(expected), "{code}");
        assert_eq!(expected.code(), code);
        assert_eq!(expected.to_string(), code);
    }
}

/// The header form decides which of two truncation errors comes out, and the
/// two-character form has its own: a `~` with nothing behind it is a trailing
/// tilde, a length that overruns the stream is a truncation.
#[test]
fn truncation_at_each_header_form() {
    assert_eq!(decode(b"~", Profile::U), Err(Error::TrailingTilde));
    assert_eq!(decode(b"~L", Profile::U), Err(Error::Truncated));
    assert_eq!(decode(b"~Labc", Profile::U), Err(Error::Truncated));
    assert_eq!(decode(b"~_", Profile::U), Err(Error::Truncated));
    assert_eq!(decode(b"~_A", Profile::U), Err(Error::Truncated));
    assert_eq!(decode(b"~_AA", Profile::U), Err(Error::Truncated));
    // 63 + 0 = 63 payload bytes are promised and 63 are there.
    let ok = [b"~_AA".as_slice(), &[b'a'; 63]].concat();
    assert_eq!(decode(&ok, Profile::U).unwrap().bytes.len(), 63);
}

/// A frame promises up to 262143 characters. Promising them is not the same as
/// having them, and a decoder that allocates on the promise is the bug §10.4
/// warns about.
#[test]
fn a_frame_length_is_a_promise_not_an_allocation() {
    // 'a' 'a' 'a' is 26 << 12 | 26 << 6 | 26 = 108186 characters claimed.
    assert_eq!(decode(b"~Aaaa", Profile::U), Err(Error::Truncated));
    assert_eq!(decode(b"~A__-", Profile::U), Err(Error::Truncated));
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
    // A mix that hits the grammar often enough to be interesting: mostly
    // alphabet characters, with tildes, padding and high bytes thrown in.
    let pool: Vec<u8> = b"ABCabc012-_+/=~\x00\xff \t\"\\.".to_vec();
    for _ in 0..20_000 {
        let n = (next() % 40) as usize;
        let data: Vec<u8> = (0..n).map(|_| pool[next() as usize % pool.len()]).collect();
        for profile in [Profile::U, Profile::T, Profile::B] {
            for f in [
                decode as fn(&[u8], Profile) -> Result<Decoded, Error>,
                decode_plain,
                decode_framed,
                decode_url_strict,
            ] {
                if let Ok(d) = f(&data, profile) {
                    // Whatever came out has to be no larger than the stream
                    // could describe: a literal is one byte per byte and base64
                    // is three per four.
                    assert!(d.bytes.len() <= data.len(), "{data:?}");
                }
            }
        }
    }
}
