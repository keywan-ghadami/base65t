// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §16.8 — every one of the ten error codes in §10.4, raised on purpose.
//!
//! A table of error codes nothing produces is a table of error codes nobody
//! has checked. The last test is the other half of the same concern: the
//! decoder reads attacker-controlled input (§14), so it is run over arbitrary
//! bytes with the only requirement that it return rather than panic, allocate
//! wildly or read past the end.

use base65t::*;

#[test]
fn all_ten_codes() {
    let cases: [(Error, &str, Result<Decoded, Error>); 10] = [
        (
            Error::TrailingTilde,
            "E_TRAILING_TILDE",
            decode(
                &[encode_base64url(&[9u8; 48]), b"~".to_vec()].concat(),
                Profile::U,
            ),
        ),
        (Error::Truncated, "E_TRUNCATED", decode(b"~AAA", Profile::U)),
        (Error::Profile, "E_PROFILE", decode(b"~~a b", Profile::U)),
        (Error::Align, "E_ALIGN", decode(b"abcde", Profile::U)),
        (
            Error::NonzeroTail,
            "E_NONZERO_TAIL",
            decode(b"YWxpY2V", Profile::U),
        ),
        (Error::Charset, "E_CHARSET", decode(b"YW*j", Profile::U)),
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
        (Error::Mask, "E_MASK", decode(b"~AAAAAAABa", Profile::U)),
    ];
    for (expected, code, got) in cases {
        assert_eq!(got, Err(expected), "{code}");
        assert_eq!(expected.code(), code);
        assert_eq!(expected.to_string(), code);
    }
}

/// Where a mask block can be cut off, and what each cut is called.
#[test]
fn truncation_inside_a_mask_block() {
    assert_eq!(decode(b"~", Profile::U), Err(Error::TrailingTilde));
    for n in 1..8 {
        let s = [b"~".as_slice(), &b"AAAAAAAA"[..n]].concat();
        assert_eq!(
            decode(&s, Profile::U),
            Err(Error::Truncated),
            "{n} mask chars"
        );
    }
    // A full mask, three clear bytes promised, two present.
    assert_eq!(decode(b"~4AAAAAAAab", Profile::U), Err(Error::Truncated));
    // Three present: a valid tail of three admitted bytes and nothing else.
    assert_eq!(decode(b"~4AAAAAAAabc", Profile::U).unwrap().bytes, b"abc");
}

/// A mask promises at most forty-eight bytes and can address nothing beyond
/// its block. There is no length in this format a sender chooses, which is
/// the property §14 wanted and the earlier format did not have.
#[test]
fn a_mask_cannot_promise_more_than_a_block() {
    // All 48 bits set, no clear bytes: truncated, not a large allocation.
    assert_eq!(decode(b"~________", Profile::U), Err(Error::Truncated));
    // All 48 bits set and 48 bytes: a valid (if wasteful) block.
    let s = [b"~________".as_slice(), &[b'a'; 48]].concat();
    assert_eq!(decode(&s, Profile::U).unwrap().bytes, vec![b'a'; 48]);
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
        for profile in [Profile::U, Profile::T] {
            for f in [
                decode as fn(&[u8], Profile) -> Result<Decoded, Error>,
                decode_url_strict,
            ] {
                if let Ok(d) = f(&data, profile) {
                    // Whatever came out has to be no larger than the stream
                    // could describe: a raw byte is one per character and
                    // base64 is three per four.
                    assert!(d.bytes.len() <= data.len(), "{data:?}");
                }
            }
        }
    }
}
