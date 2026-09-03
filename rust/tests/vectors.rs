// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §15 of v0.4, one test per vector.
//!
//! v0.4 is a different wire format from v0.1 to v0.3, so none of the earlier
//! vectors carries over as bytes; the *inputs* do, because they were chosen
//! to sit on the format's boundaries, and the boundaries moved with it.
//! `docs/history/` holds the old streams, and the mask form that lasted a
//! day.

use base65t::*;

fn base64_len(n: usize) -> usize {
    (4 * n).div_ceil(3)
}

// --- TV1-TV4: the two forms, profile U ------------------------------------

/// A raw block: two characters of marker, the bytes as they are.
#[test]
fn tv1_a_raw_block() {
    let out = encode(b"alice.jones");
    assert_eq!(out, b"~~alice.jones");
    assert_eq!(out.len(), 13);
    assert_eq!(base64_len(11), 15);
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"alice.jones");
}

/// A base64 block: four bytes of the twenty-two are not text, so the block is
/// base64 and nothing else. The segment format wrote this input in 26
/// characters; the block format gives that up for an encoder that is one
/// comparison.
#[test]
fn tv2_a_base64_block() {
    let input = [b"\xde\xad\xbe\xef".as_slice(), b"session-eu-central"].concat();
    let out = encode(&input);
    assert_eq!(out, b"3q2-73Nlc3Npb24tZXUtY2VudHJhbA");
    assert_eq!(out.len(), 30);
    assert_eq!(out, encode_base64url(&input));
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, input);
}

/// `~` in a raw payload needs nothing: the block's length is fixed.
#[test]
fn tv3_tilde_needs_no_escaping() {
    let out = encode(b"sub~alice~jones");
    assert_eq!(out, b"~~sub~alice~jones");
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"sub~alice~jones");
    // And the input that v0.1 built a whole conflict rule around.
    assert_eq!(encode(b"hello~Alice"), b"~~hello~Alice");
}

/// Blocks are cut at absolute offsets of 48, and the last one is shorter.
#[test]
fn tv4_block_boundaries() {
    let input = vec![b'a'; 100];
    let out = encode(&input);
    let block = [b"~~".as_slice(), &[b'a'; 48]].concat();
    assert_eq!(out[..50], block[..]);
    assert_eq!(out[50..100], block[..]);
    assert_eq!(&out[100..], b"~~aaaa");
    assert_eq!(out.len(), 106);
    assert_eq!(base64_len(100), 134);
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, input);
}

// --- TV5: one byte decides the block -------------------------------------

/// Fifty bytes of English. In profile U the space is not admitted, so the
/// first block is base64 as a whole and only the tail `in` is raw -- no,
/// even the tail is too short to be raw (§9.1), so the stream is base64url
/// exactly. In profile T the space is admitted, the first block is raw, and
/// the two-byte tail is base64.
#[test]
fn tv5_one_byte_decides_the_block() {
    let input = b"the quick brown fox jumps over the lazy dog. again";
    let u = encode_with(input, Profile::U);
    assert_eq!(u, encode_base64url(input));
    assert_eq!(u.len(), 67);
    assert_eq!(decode(&u, Profile::U).unwrap().bytes, input);

    let t = encode_with(input, Profile::T);
    assert_eq!(&t[..50], [b"~~".as_slice(), &input[..48]].concat());
    assert_eq!(&t[50..], b"aW4");
    assert_eq!(t.len(), 53);
    assert_eq!(decode(&t, Profile::T).unwrap().bytes, input);
}

/// The reserved form: `~` and an alphabet character is the mask block of
/// `docs/history/`, and until a version defines it again it is an error --
/// loudly, so that a stream from such a version is not read as something
/// else.
#[test]
fn tv5b_a_tilde_and_an_alphabet_character_is_reserved() {
    assert_eq!(decode(b"~AAAAAAAA", Profile::U), Err(Error::Reserved));
    assert_eq!(decode(b"~7abc", Profile::U), Err(Error::Reserved));
    assert_eq!(decode(b"~_", Profile::U), Err(Error::Reserved));
    // Anything else after `~` is not reserved, it is wrong.
    assert_eq!(decode(b"~=", Profile::U), Err(Error::Charset));
    assert_eq!(decode(b"~ a", Profile::U), Err(Error::Charset));
}

// --- TV6-TV8: reading base64, and the two rules that keep it unambiguous --

#[test]
fn tv6_reads_base64_and_base64url() {
    let bytes = b"<<???>>".to_vec();
    let cases: [(&[u8], &[u8], AlphabetSeen, bool); 4] = [
        (b"PDw_Pz8-Pg", &bytes, AlphabetSeen::Url, false),
        (b"PDw/Pz8+Pg", &bytes, AlphabetSeen::Classic, false),
        (
            b"YWxpY2Uuam9uZXM",
            b"alice.jones",
            AlphabetSeen::None,
            false,
        ),
        (b"YWxpY2U=", b"alice", AlphabetSeen::None, true),
    ];
    for (stream, expect, alphabet, padding) in cases {
        let d = decode(stream, Profile::U).expect("valid");
        assert_eq!(d.bytes, expect, "{:?}", String::from_utf8_lossy(stream));
        assert_eq!(d.alphabet_seen, alphabet);
        assert_eq!(d.padding_seen, padding);
    }
    // A long base64 stream is read in blocks of 64 characters, which is
    // invisible: base64 blocks tile.
    let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
    assert_eq!(
        decode(&encode_base64url(&data), Profile::U).unwrap().bytes,
        data
    );
}

#[test]
fn tv7_rule_a_holds_at_alphabet_positions() {
    assert_eq!(decode(b"PDw_Pz8+Pg", Profile::U), Err(Error::MixedAlphabet));
    assert_eq!(decode(b"PDw/Pz8-Pg", Profile::U), Err(Error::MixedAlphabet));
    assert_eq!(
        decode_url_strict(b"PDw/Pz8+Pg", Profile::U),
        Err(Error::NonUrlAlphabet)
    );
    assert!(decode_url_strict(b"PDw_Pz8-Pg", Profile::U).is_ok());
    // Across blocks: a raw block between two base64 blocks in different
    // alphabets is still one stream.
    let url = encode_base64url(&[0xfbu8; 48]); // `-` and `_` in here
    let classic: Vec<u8> = url
        .iter()
        .map(|&c| match c {
            b'-' => b'+',
            b'_' => b'/',
            c => c,
        })
        .collect();
    let mut s = url.clone();
    s.extend(b"~~");
    s.extend([b'a'; 48]);
    s.extend(&classic);
    assert_eq!(decode(&s, Profile::U), Err(Error::MixedAlphabet));
}

/// The negative half of Rule A, and the one a whole-stream scanner fails: the
/// bytes of a raw block are data.
#[test]
fn tv7_raw_bytes_do_not_count() {
    let stream = b"~~a+b/c-d_e";
    let d = decode(stream, Profile::T).expect("valid under profile T");
    assert_eq!(d.alphabet_seen, AlphabetSeen::None);
    assert_eq!(d.bytes, b"a+b/c-d_e");
}

#[test]
fn tv8_what_may_follow_a_tilde() {
    assert_eq!(decode(b"~", Profile::U), Err(Error::TrailingTilde));
    assert_eq!(decode(b"~~", Profile::U).unwrap().bytes, b"");
    assert_eq!(decode(b"~A", Profile::U), Err(Error::Reserved));
    assert_eq!(decode(b"~=", Profile::U), Err(Error::Charset));
    // A `~` where a base64 block should continue.
    assert_eq!(decode(b"YW~x", Profile::U), Err(Error::Charset));
}

// --- TV9-TV10: padding, and why it may not be stripped in advance --------

#[test]
fn tv9_padding() {
    assert_eq!(decode(b"YWxpY2U=", Profile::U).unwrap().bytes, b"alice");
    assert!(decode(b"YWxpY2U=", Profile::U).unwrap().padding_seen);
    assert_eq!(decode(b"YWxpY2Uu", Profile::U).unwrap().bytes, b"alice.");
    assert_eq!(decode(b"YWxp==", Profile::U), Err(Error::Padding));
    assert_eq!(decode(b"YWxpY2U==", Profile::U), Err(Error::Padding));
    // Padding inside a base64 block that is not the last: never.
    let mut s = encode_base64url(&[7u8; 96]);
    s[63] = b'=';
    assert_eq!(decode(&s, Profile::U), Err(Error::Charset));
}

/// Both streams end in `=`; in profile T it is a legal raw byte. A raw tail
/// runs to the end of the stream, so the `=` is data; a base64 tail is where
/// Rule P looks.
#[test]
fn tv10_equals_as_a_raw_byte_in_profile_t() {
    let d = decode(b"~~a=b=", Profile::T).expect("a raw tail");
    assert_eq!(d.bytes, b"a=b=");
    assert!(!d.padding_seen);
    assert_eq!(decode(b"~~a=b=", Profile::U), Err(Error::Profile));
}

// --- TV11: the error table -----------------------------------------------

#[test]
fn tv11_error_cases() {
    assert_eq!(decode(b"abcde", Profile::U), Err(Error::Align));
    assert_eq!(decode(b"~", Profile::U), Err(Error::TrailingTilde));
    assert_eq!(decode(b"~Aabc", Profile::U), Err(Error::Reserved));
    assert_eq!(decode(b"~~a b", Profile::U), Err(Error::Profile));
    assert_eq!(decode(b"YWxp==", Profile::U), Err(Error::Padding));
    assert_eq!(decode(b"YWxpY2V", Profile::U), Err(Error::NonzeroTail));
    assert_eq!(decode(b"YW~x", Profile::U), Err(Error::Charset));
    assert_eq!(decode(b"PDw_Pz8+Pg", Profile::U), Err(Error::MixedAlphabet));
    assert_eq!(
        decode_url_strict(b"PDw/Pz8+Pg", Profile::U),
        Err(Error::NonUrlAlphabet)
    );
}

// --- TV12: the tail ------------------------------------------------------

/// A last block is shorter than 48 bytes, and its form follows §9.1: raw
/// from four bytes up, base64 below, ties to raw.
#[test]
fn tv12_the_tail() {
    let full = [b'a'; 48];
    for (tail, want) in [
        (&b""[..], &b""[..]),
        (b"a", b"YQ"),
        (b"ab", b"YWI"),
        (b"abc", b"YWJj"),
        (b"abcd", b"~~abcd"),
        (b"abcdef", b"~~abcdef"),
        (b"a b", b"YSBi"),
        (b"a bcd", b"YSBiY2Q"),
    ] {
        let data = [&full[..], tail].concat();
        let out = encode(&data);
        assert_eq!(&out[..50], [b"~~".as_slice(), &full].concat(), "{tail:?}");
        assert_eq!(&out[50..], want, "{tail:?}");
        assert_eq!(decode(&out, Profile::U).unwrap().bytes, data);
    }
}
