// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §15 of v0.4, one test per vector.
//!
//! The set shrank twice and each time for the same reason. v0.2 corrected
//! three vectors v0.1 had got wrong (TV2 named one of three equally dense
//! streams without a rule that picked it; TV5a's `legible` body was longer
//! than the base64 the same encoder writes; TV11 admitted two error codes
//! where §10.3's order of checks allows one). v0.4 withdrew the vectors that
//! only existed to describe framing and the second encoder, because neither
//! is in the format any more. `docs/history/` records both moves.

use base65t::*;

fn base64_len(n: usize) -> usize {
    (4 * n).div_ceil(3)
}

// --- TV1-TV4: the basic cases, profile U ---------------------------------

#[test]
fn tv1_literal_beats_base64() {
    let out = encode(b"alice.jones");
    assert_eq!(out, b"~Lalice.jones");
    assert_eq!(out.len(), 13);
    assert_eq!(base64_len(11), 15);
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"alice.jones");
}

/// Three segmentations are 26 characters here: absorbing one or two text bytes
/// into the base64 segment costs nothing, since `ceil(4k/3) + (22-k) + 2` is 26
/// for k = 4, 5 and 6 alike. Which one comes out is not a matter of taste but
/// of which rule ran, and since v0.4 there is one rule: the exact programme,
/// tie-broken by the order of §11.1, where `B < S` at index 4 picks k = 6.
///
/// v0.1 printed `3q2-7w~Ssession-eu-central` here — the k = 4 stream a
/// scanning encoder writes. It is the same length and it decodes to the same
/// bytes; it is simply not what a length-minimal encoder with a tie-break
/// produces, and the vector moved rather than the rule.
#[test]
fn tv2_binary_prefix_then_text() {
    let input = [b"\xde\xad\xbe\xef".as_slice(), b"session-eu-central"].concat();
    let ours = encode(&input);
    assert_eq!(ours, b"3q2-73Nl~Qssion-eu-central");
    assert_eq!(ours.len(), 26);
    assert_eq!(base64_len(input.len()), 30);

    let v01 = b"3q2-7w~Ssession-eu-central";
    assert_eq!(v01.len(), ours.len(), "the same length, a different rule");
    for s in [ours.as_slice(), v01.as_slice()] {
        assert_eq!(decode(s, Profile::U).unwrap().bytes, input);
    }
}

#[test]
fn tv3_tilde_needs_no_escaping() {
    let out = encode(b"sub~alice~jones");
    assert_eq!(out, b"~Psub~alice~jones");
    assert_eq!(out.len(), 17);
    assert_eq!(base64_len(15), 20);
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"sub~alice~jones");
}

/// The four-character header form: `L1 = 63`, then twelve bits of `V`.
#[test]
fn tv4_extended_length_header() {
    let input = vec![b'a'; 100];
    let out = encode(&input);
    assert_eq!(&out[..4], b"~_Al");
    assert_eq!(out.len(), 104);
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, input);

    // 100 - 63 = 37 = 000000 100101, and 37 is 'l'.
    assert_eq!(out[2], b'A');
    assert_eq!(out[3], b'l');

    // The classic alphabet writes the same header as `~/Al`, and a decoder
    // takes it (§5.2).
    let classic = [b"~/Al".as_slice(), &input].concat();
    assert_eq!(decode(&classic, Profile::U).unwrap().bytes, input);
}

// --- TV5: `~A` inside a literal ------------------------------------------

/// `hello~Alice` was the input v0.1's TV5 built its framing conflict on: a
/// frame body could not carry `~A`, so the encoder had to break the literal
/// and write base64 that was longer than the literal it replaced.
///
/// v0.4 has no frames, so there is no conflict and no rule to state: the
/// literal wins outright, `~A` is payload like any other pair of bytes, and
/// the vector is kept because the input is exactly the one that used to be
/// hard.
#[test]
fn tv5_the_literal_wins_outright() {
    let input = b"hello~Alice";
    let out = encode(input);
    assert_eq!(out, b"~Lhello~Alice");
    assert_eq!(out.len(), 13);

    // What v0.1 made the encoder write instead, for comparison: two characters
    // more, for a rule that is gone.
    assert_eq!(encode_base64url(input), b"aGVsbG9-QWxpY2U");
    assert_eq!(encode_base64url(input).len(), 15);

    assert_eq!(decode(&out, Profile::U).unwrap().bytes, input);
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
}

#[test]
fn tv7_rule_a_holds_at_alphabet_positions() {
    assert_eq!(
        decode(b"PDw_Pz8-Pg", Profile::U).unwrap().alphabet_seen,
        AlphabetSeen::Url
    );
    assert_eq!(
        decode(b"PDw/Pz8+Pg", Profile::U).unwrap().alphabet_seen,
        AlphabetSeen::Classic
    );
    assert_eq!(decode(b"PDw_Pz8+Pg", Profile::U), Err(Error::MixedAlphabet));
    assert_eq!(decode(b"PDw/Pz8-Pg", Profile::U), Err(Error::MixedAlphabet));
    assert_eq!(
        decode_url_strict(b"PDw/Pz8+Pg", Profile::U),
        Err(Error::NonUrlAlphabet)
    );
    assert!(decode_url_strict(b"PDw_Pz8-Pg", Profile::U).is_ok());
}

/// The negative half of Rule A, and the one a whole-stream scanner fails: the
/// characters in a literal payload are data.
#[test]
fn tv7_payload_characters_do_not_count() {
    let stream = b"~Ka+b/c-d_e~fg";
    let d = decode(stream, Profile::T).expect("valid under profile T");
    assert_eq!(d.alphabet_seen, AlphabetSeen::None);
    assert_eq!(d.bytes, b"a+b/c-d_e~\x7e");
}

#[test]
fn tv8_header_positions_are_checked_before_they_are_read() {
    assert_eq!(decode(b"~~abc", Profile::U), Err(Error::Charset));
    assert_eq!(decode(b"~=ab", Profile::U), Err(Error::Charset));
    assert_eq!(decode(b"~_A~", Profile::U), Err(Error::Charset));
    assert_eq!(decode(b"~", Profile::U), Err(Error::TrailingTilde));
    assert_eq!(decode(b"~A", Profile::U), Err(Error::ReservedLen));
}

// --- TV9-TV10: padding, and why it may not be stripped in advance --------

#[test]
fn tv9_padding() {
    assert_eq!(decode(b"YWxpY2U=", Profile::U).unwrap().bytes, b"alice");
    assert!(decode(b"YWxpY2U=", Profile::U).unwrap().padding_seen);
    assert_eq!(decode(b"YWxpY2Uu", Profile::U).unwrap().bytes, b"alice.");
    assert!(!decode(b"YWxpY2Uu", Profile::U).unwrap().padding_seen);
    assert_eq!(decode(b"YWxp==", Profile::U), Err(Error::Padding));
    assert_eq!(decode(b"YWxpY2U==", Profile::U), Err(Error::Padding));
    assert_eq!(decode(b"YWxpY2U=~Lfoo", Profile::U), Err(Error::Charset));
}

/// Both streams end in `=`; only the literal length decides whether the
/// scanner ever looks at it. Stripping padding up front gets one of them
/// wrong, and profile T is where it shows.
#[test]
fn tv10_equals_as_a_literal_byte_in_profile_t() {
    assert_eq!(decode(b"~Da=b=", Profile::T), Err(Error::Padding));
    let d = decode(b"~Ea=b=", Profile::T).expect("four payload bytes reach the end");
    assert_eq!(d.bytes, b"a=b=");
    assert!(!d.padding_seen);
}

// --- TV11: the error table -----------------------------------------------

#[test]
fn tv11_error_cases() {
    assert_eq!(decode(b"abcde", Profile::U), Err(Error::Align));
    assert_eq!(decode(b"~A", Profile::U), Err(Error::ReservedLen));
    assert_eq!(decode(b"~Labc", Profile::U), Err(Error::Truncated));
    assert_eq!(decode(b"~Cab~", Profile::U), Err(Error::TrailingTilde));
    assert_eq!(decode(b"YWxp==", Profile::U), Err(Error::Padding));
    assert_eq!(decode(b"~Ca b", Profile::U), Err(Error::Profile));
    // `YWxpY2V` is `alice` plus a set bit in the last quantum: canonical
    // base64 would have written `YWxpY2U`.
    assert_eq!(decode(b"YWxpY2V", Profile::U), Err(Error::NonzeroTail));
}

// --- TV12-TV13: what v0.2 and v0.4 decide that v0.1 left open -------------

/// The tie-break of §9.0 and §11.1, on the smallest input where it decides
/// anything: nine bytes the profile admits, then one it does not.
///
/// Both segmentations are thirteen characters. At index 7 the choice is `B`
/// against `L`, and `B < L` takes the shorter literal — ending it early aligns
/// the base64 run so that the remaining three bytes cost two characters
/// instead of four. The second stream is what v0.1's *Berechnung* produced.
#[test]
fn tv12_the_tie_break_decides() {
    let input = b"aaaaaaaaa ";
    let ours = encode(input);
    assert_eq!(ours, b"~HaaaaaaaYWEg");
    assert_eq!(ours.len(), 13);

    let v01 = b"~JaaaaaaaaaIA";
    assert_eq!(
        v01.len(),
        13,
        "the same length, which is why a rule is needed"
    );
    assert_ne!(ours, v01.to_vec());
    for s in [ours.as_slice(), v01.as_slice()] {
        assert_eq!(decode(s, Profile::U).unwrap().bytes, input);
    }
}

/// §9.6, the decision v0.4 adds: the encoder looks at the head of the input
/// once and either runs the programme or writes base64url.
///
/// The vector is the decision, not a stream, because that is where a second
/// implementation goes wrong: it is a function of the first
/// [`SAMPLE_BYTES`] bytes and of nothing else — not of the length, not of a
/// clock, not of a thread count — so two encoders that disagree here produce
/// different bytes for the same input.
#[test]
fn tv13_the_head_decides_once() {
    // A magic number settles it before entropy is measured.
    assert_eq!(classify(b"\x1f\x8b\x08\x00\x00\x00\x00\x00"), Mode::Base64);
    assert_eq!(classify(b"\x28\xb5\x2f\xfd\x00\x00\x00\x00"), Mode::Base64);
    // Otherwise the integer entropy of the sample does, at 7,400 millibits.
    assert_eq!(ENTROPY_LIMIT_MILLIBITS, 7400);
    assert_eq!(classify(b"alice.jones"), Mode::Exact);
    assert_eq!(classify(&vec![b'a'; 100_000]), Mode::Exact);

    // And the sample is a prefix, so what follows cannot move the answer.
    let head: Vec<u8> = (0..SAMPLE_BYTES).map(|i| b"abcdefghij"[i % 10]).collect();
    let mut long = head.clone();
    long.extend((0..100_000u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8));
    assert_eq!(classify(&long), Mode::Exact);
    assert_eq!(classify(&long), classify(&head));
}
