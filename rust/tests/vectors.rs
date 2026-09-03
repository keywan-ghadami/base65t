// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §15 of v0.4, one test per vector.
//!
//! v0.4 is a different wire format from v0.1 to v0.3, so none of the earlier
//! vectors carries over as bytes; the *inputs* do, because they were chosen
//! to sit on the format's boundaries, and the boundaries moved with it.
//! `docs/history/` holds the old streams.

use base65t::*;

fn base64_len(n: usize) -> usize {
    (4 * n).div_ceil(3)
}

// --- TV1-TV4: the three forms, profile U ----------------------------------

/// A raw block: two characters of marker, the bytes as they are.
#[test]
fn tv1_a_raw_block() {
    let out = encode(b"alice.jones");
    assert_eq!(out, b"~~alice.jones");
    assert_eq!(out.len(), 13);
    assert_eq!(base64_len(11), 15);
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"alice.jones");
}

/// A base64 block, and the reason for it: on a tail of 22 bytes the mask
/// form costs nine characters of overhead for eighteen clear bytes, which is
/// 33 against base64's 30. The earlier format wrote this input in 26; the
/// block format gives that up, and TV5 is what it buys instead.
#[test]
fn tv2_a_base64_block_where_the_mask_would_not_pay() {
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

// --- TV5: the mask form ---------------------------------------------------

/// Fifty bytes of English in profile U. The first block has nine spaces and
/// a full stop in it, which the earlier format would have written as ten
/// short base64 runs and the text in between as base64 too, because no run
/// reached the length where a literal pays. The mask form keeps 39 of 48
/// bytes in the clear, and the whole input costs 63 characters against
/// base64's 67.
///
/// The mask's first character is `7`: `the qu` is admitted, space, admitted,
/// admitted, which is `111011`, which is 59.
#[test]
fn tv5_a_mask_block() {
    let input = b"the quick brown fox jumps over the lazy dog. again";
    let out = encode(input);
    assert_eq!(
        out,
        b"~777vvd73thequickbrownfoxjumpsoverthelazydog.agaICAgICAgICAgaW4"
    );
    assert_eq!(out.len(), 63);
    assert_eq!(base64_len(50), 67);
    assert_eq!(out[0], b'~');
    assert_eq!(&out[1..9], b"777vvd73");
    assert_eq!(&out[9..48], b"thequickbrownfoxjumpsoverthelazydog.aga");
    assert_eq!(&out[48..60], b"ICAgICAgICAg");
    assert_eq!(&out[60..], b"aW4");
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, input);

    // The same input in profile T admits the space, so the first block is
    // raw and the whole thing is 53 characters.
    let t = encode_with(input, Profile::T);
    assert_eq!(&t[..50], [b"~~".as_slice(), &input[..48]].concat());
    assert_eq!(&t[50..], b"aW4");
    assert_eq!(decode(&t, Profile::T).unwrap().bytes, input);
}

/// The tie at 27 admitted bytes goes to the mask, because a tie costs
/// nothing and the format prefers text in the clear. At 26 base64 is shorter
/// and wins outright.
#[test]
fn tv5b_the_tie_goes_to_the_clear_text() {
    let mut d = vec![b'a'; 27];
    d.extend(vec![b' '; 21]);
    let out = encode(&d);
    assert_eq!(out.len(), 64);
    assert_eq!(out[0], b'~');
    assert_eq!(&out[1..9], b"____4AAA");
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, d);

    let mut d = vec![b'a'; 26];
    d.extend(vec![b' '; 22]);
    let out = encode(&d);
    assert_eq!(out.len(), 64);
    assert_ne!(out[0], b'~');
    assert_eq!(out, encode_base64url(&d));
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
    // Mask characters are alphabet positions too.
    let mut d = vec![b'a'; 27];
    d.extend(vec![b' '; 21]);
    let mut out = encode(&d);
    assert_eq!(&out[1..5], b"____");
    out[1..5].fill(b'/');
    assert_eq!(
        decode(&out, Profile::U).unwrap().alphabet_seen,
        AlphabetSeen::Classic
    );
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, d);
    // One of the four back to `_`: both variants in one stream.
    out[1] = b'_';
    assert_eq!(decode(&out, Profile::U), Err(Error::MixedAlphabet));
    out[1] = b'/';
    assert_eq!(
        decode_url_strict(&out, Profile::U),
        Err(Error::NonUrlAlphabet)
    );
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
fn tv8_mask_positions_are_checked_before_they_are_read() {
    assert_eq!(decode(b"~=AAAAAAA", Profile::U), Err(Error::Charset));
    assert_eq!(decode(b"~AAAAAA~A", Profile::U), Err(Error::Charset));
    assert_eq!(decode(b"~", Profile::U), Err(Error::TrailingTilde));
    assert_eq!(decode(b"~AAAA", Profile::U), Err(Error::Truncated));
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
    assert_eq!(decode(b"~AAAA", Profile::U), Err(Error::Truncated));
    assert_eq!(decode(b"~~a b", Profile::U), Err(Error::Profile));
    assert_eq!(decode(b"YWxp==", Profile::U), Err(Error::Padding));
    assert_eq!(decode(b"YWxpY2V", Profile::U), Err(Error::NonzeroTail));
    assert_eq!(decode(b"YW~x", Profile::U), Err(Error::Charset));
    // A mask that claims byte 47 in a tail of one clear byte.
    assert_eq!(decode(b"~AAAAAAABa", Profile::U), Err(Error::Mask));
}

// --- TV12: the mask in a tail --------------------------------------------

/// A tail block is shorter than 48 bytes, and the mask then has bits it may
/// not use. The decoder learns the block's length from what the two parts
/// add up to, and checks the mask against it.
#[test]
fn tv12_a_mask_tail() {
    // 30 bytes: 27 admitted, three not. Mask form 9 + 27 + 4 = 40, base64
    // 40: a tie, and the mask takes it.
    let mut d = vec![b'a'; 27];
    d.extend(b"   ");
    let out = encode(&d);
    assert_eq!(out.len(), 40);
    assert_eq!(&out[..9], b"~____4AAA");
    assert_eq!(&out[9..36], &[b'a'; 27][..]);
    assert_eq!(&out[36..], b"ICAg");
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, d);

    // The same stream with one more bit set in the mask, past the data.
    let mut bad = out.clone();
    bad[8] = b'B'; // byte 47 claimed
    assert_eq!(decode(&bad, Profile::U), Err(Error::Mask));
}
