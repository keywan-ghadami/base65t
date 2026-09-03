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
    assert_eq!(out, "~~alice.jones");
    assert_eq!(out.len(), 13);
    assert_eq!(base64_len(11), 15);
    assert_eq!(
        decode_detailed(out.as_bytes()).unwrap().bytes,
        b"alice.jones"
    );
}

/// A base64 block: four bytes of the twenty-two are not text, so the block is
/// base64 and nothing else. The segment format wrote this input in 26
/// characters; the block format gives that up for an encoder that is one
/// comparison.
#[test]
fn tv2_a_base64_block() {
    let input = [b"\xde\xad\xbe\xef".as_slice(), b"session-eu-central"].concat();
    let out = encode(&input);
    assert_eq!(out, "3q2-73Nlc3Npb24tZXUtY2VudHJhbA");
    assert_eq!(out.len(), 30);
    assert_eq!(out, encode_base64url(&input));
    assert_eq!(decode_detailed(out.as_bytes()).unwrap().bytes, input);
}

/// `~` in a raw payload needs nothing: the block's length is fixed.
#[test]
fn tv3_tilde_needs_no_escaping() {
    let out = encode(b"sub~alice~jones");
    assert_eq!(out, "~~sub~alice~jones");
    assert_eq!(
        decode_detailed(out.as_bytes()).unwrap().bytes,
        b"sub~alice~jones"
    );
    // And the input that v0.1 built a whole conflict rule around.
    assert_eq!(encode(b"hello~Alice"), "~~hello~Alice");
}

/// Blocks are cut at absolute offsets of 48, and the last one is shorter.
#[test]
fn tv4_block_boundaries() {
    let input = vec![b'a'; 100];
    let out = encode(&input);
    let block = [b"~~".as_slice(), &[b'a'; 48]].concat();
    assert_eq!(out.as_bytes()[..50], block[..]);
    assert_eq!(out.as_bytes()[50..100], block[..]);
    assert_eq!(&out[100..], "~~aaaa");
    assert_eq!(out.len(), 106);
    assert_eq!(base64_len(100), 134);
    assert_eq!(decode_detailed(out.as_bytes()).unwrap().bytes, input);
}

// --- TV5: one byte decides the block -------------------------------------

/// One byte, and the block goes the other way. The same 48 bytes are raw at
/// 50 characters and base64 at 64, and the only difference between the two
/// inputs is that one of them holds a space -- which the alphabet does not
/// admit (§7), wherever in the block it sits.
#[test]
fn tv5_one_byte_decides_the_block() {
    let block = b"the-quick-brown-fox-jumps-over-the-lazy-dog.abcd".to_vec();
    assert_eq!(block.len(), 48);
    let out = encode(&block);
    assert_eq!(out.as_bytes(), [b"~~".as_slice(), &block].concat());
    assert_eq!(out.len(), 50);
    assert_eq!(decode_detailed(out.as_bytes()).unwrap().bytes, block);

    // Fifty bytes of English with spaces: no block of it can be raw, so the
    // stream is byte for byte base64url.
    let prose = b"the quick brown fox jumps over the lazy dog. again";
    assert_eq!(encode(prose), encode_base64url(prose));
    assert_eq!(encode(prose).len(), 67);
    assert_eq!(decode(encode(prose)).unwrap(), prose);

    // And it is the byte, not its position: every single position rejects.
    for i in 0..block.len() {
        let mut v = block.clone();
        v[i] = b' ';
        assert_eq!(encode(&v), encode_base64url(&v), "space at {i}");
    }
}

/// The reserved form: `~` and an alphabet character is the mask block of
/// `docs/history/`, and until a version defines it again it is an error --
/// loudly, so that a stream from such a version is not read as something
/// else.
#[test]
fn tv5b_a_tilde_and_an_alphabet_character_is_reserved() {
    assert_eq!(decode(b"~AAAAAAAA"), Err(Error::Reserved));
    assert_eq!(decode(b"~7abc"), Err(Error::Reserved));
    assert_eq!(decode(b"~_"), Err(Error::Reserved));
    // Anything else after `~` is not reserved, it is wrong.
    assert_eq!(decode(b"~="), Err(Error::Charset));
    assert_eq!(decode(b"~ a"), Err(Error::Charset));
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
        let d = decode_detailed(stream).expect("valid");
        assert_eq!(d.bytes, expect, "{:?}", String::from_utf8_lossy(stream));
        assert_eq!(d.alphabet_seen, alphabet);
        assert_eq!(d.padding_seen, padding);
    }
    // A long base64 stream is read in blocks of 64 characters, which is
    // invisible: base64 blocks tile.
    let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
    assert_eq!(
        decode_detailed(encode_base64url(&data).as_bytes())
            .unwrap()
            .bytes,
        data
    );
}

#[test]
fn tv7_rule_a_holds_at_alphabet_positions() {
    assert_eq!(decode(b"PDw_Pz8+Pg"), Err(Error::MixedAlphabet));
    assert_eq!(decode(b"PDw/Pz8-Pg"), Err(Error::MixedAlphabet));
    assert_eq!(decode_url_strict(b"PDw/Pz8+Pg"), Err(Error::NonUrlAlphabet));
    assert!(decode_url_strict(b"PDw_Pz8-Pg").is_ok());
    // Across blocks: a raw block between two base64 blocks in different
    // alphabets is still one stream.
    let url = encode_base64url([0xfbu8; 48]).into_bytes(); // `-` and `_` in here
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
    assert_eq!(decode(&s), Err(Error::MixedAlphabet));
}

/// The negative half of Rule A, and the one a whole-stream scanner fails: the
/// bytes of a raw block are data.
#[test]
fn tv7_raw_bytes_do_not_count() {
    // `-` and `_` are the URL variant's own two characters and are also
    // admitted raw, so this is exactly the stream a whole-stream scanner
    // misreads: as data they say nothing about the alphabet.
    let stream = b"~~a-b_c-d_e";
    let d = decode_detailed(stream).expect("a raw block of admitted bytes");
    assert_eq!(d.alphabet_seen, AlphabetSeen::None);
    assert_eq!(d.bytes, b"a-b_c-d_e");
    // The same characters as base64 output do set it.
    assert_eq!(
        decode_detailed(b"PDw_Pz8-Pg").unwrap().alphabet_seen,
        AlphabetSeen::Url
    );
}

#[test]
fn tv8_what_may_follow_a_tilde() {
    assert_eq!(decode(b"~"), Err(Error::TrailingTilde));
    assert_eq!(decode_detailed(b"~~").unwrap().bytes, b"");
    assert_eq!(decode(b"~A"), Err(Error::Reserved));
    assert_eq!(decode(b"~="), Err(Error::Charset));
    // A `~` where a base64 block should continue.
    assert_eq!(decode(b"YW~x"), Err(Error::Charset));
}

// --- TV9-TV10: padding, and why it may not be stripped in advance --------

#[test]
fn tv9_padding() {
    assert_eq!(decode_detailed(b"YWxpY2U=").unwrap().bytes, b"alice");
    assert!(decode_detailed(b"YWxpY2U=").unwrap().padding_seen);
    assert_eq!(decode_detailed(b"YWxpY2Uu").unwrap().bytes, b"alice.");
    assert_eq!(decode(b"YWxp=="), Err(Error::Padding));
    assert_eq!(decode(b"YWxpY2U=="), Err(Error::Padding));
    // Padding inside a base64 block that is not the last: never.
    let mut s = encode_base64url([7u8; 96]).into_bytes();
    s[63] = b'=';
    assert_eq!(decode(&s), Err(Error::Charset));
}

/// Padding may not be stripped from the end of the stream in advance, even
/// though `=` is not an admitted byte. A raw tail runs to the end of the
/// stream, so a `=` there belongs to the raw block and makes it invalid;
/// strip it first and the same stream decodes cleanly instead. An error and
/// an acceptance are not the same answer.
#[test]
fn tv10_padding_may_not_be_stripped_in_advance() {
    assert_eq!(decode_detailed(b"~~abcd="), Err(Error::Profile));
    assert_eq!(decode_detailed(b"~~abcd").unwrap().bytes, b"abcd");
    // Where Rule P does look: a base64 tail at the end of the stream.
    let d = decode_detailed(b"YWxpY2U=").expect("Rule P");
    assert_eq!(d.bytes, b"alice");
    assert!(d.padding_seen);
}

// --- TV11: the error table -----------------------------------------------

#[test]
fn tv11_error_cases() {
    assert_eq!(decode(b"abcde"), Err(Error::Align));
    assert_eq!(decode(b"~"), Err(Error::TrailingTilde));
    assert_eq!(decode(b"~Aabc"), Err(Error::Reserved));
    assert_eq!(decode(b"~~a b"), Err(Error::Profile));
    assert_eq!(decode(b"YWxp=="), Err(Error::Padding));
    assert_eq!(decode(b"YWxpY2V"), Err(Error::NonzeroTail));
    assert_eq!(decode(b"YW~x"), Err(Error::Charset));
    assert_eq!(decode(b"PDw_Pz8+Pg"), Err(Error::MixedAlphabet));
    assert_eq!(decode_url_strict(b"PDw/Pz8+Pg"), Err(Error::NonUrlAlphabet));
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
        assert_eq!(
            &out.as_bytes()[..50],
            [b"~~".as_slice(), &full].concat(),
            "{tail:?}"
        );
        assert_eq!(&out.as_bytes()[50..], want, "{tail:?}");
        assert_eq!(decode_detailed(out.as_bytes()).unwrap().bytes, data);
    }
}
