// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §15, one test per vector.
//!
//! Three of them do not hold as written, and each says so where it stands
//! rather than being quietly weakened: TV2 names a stream that is one of three
//! equally dense ones, TV5a's `legible` body is longer than the base64 the
//! same encoder would pick, and TV11's last line admits two error codes where
//! the order of checks in §10.3 allows only one. FINDINGS.md has the
//! reasoning; the assertions below hold to what the specification actually
//! determines and pin the rest.

use base65t::*;

fn dense(data: &[u8]) -> Vec<u8> {
    encode_dense(data, Profile::U)
}

fn base64_len(n: usize) -> usize {
    (4 * n).div_ceil(3)
}

// --- TV1-TV4: the basic cases, profile U ---------------------------------

#[test]
fn tv1_literal_beats_base64() {
    let out = dense(b"alice.jones");
    assert_eq!(out, b"~Lalice.jones");
    assert_eq!(out.len(), 13);
    assert_eq!(base64_len(11), 15);
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"alice.jones");
}

/// TV2 gives `3q2-7w~Ssession-eu-central`: four binary bytes as base64, then
/// the whole text as one literal. It is 26 characters and so is what this
/// encoder writes — but they are not the same 26 characters, and neither is
/// wrong. Absorbing one or two text bytes into the base64 segment costs
/// nothing: `ceil(4k/3) + (22-k) + 2` is 26 for k = 4, 5 and 6 alike. `dense`
/// has no tie-break in §9.3, so all three are `dense` outputs; only
/// `canonical` picks one, and it picks k = 6, because `B < S` at index 4.
#[test]
fn tv2_binary_prefix_then_text() {
    let input = [b"\xde\xad\xbe\xef".as_slice(), b"session-eu-central"].concat();
    let spec = b"3q2-7w~Ssession-eu-central".to_vec();
    let ours = dense(&input);

    assert_eq!(spec.len(), 26);
    assert_eq!(ours.len(), 26);
    assert_eq!(base64_len(input.len()), 30);
    assert_eq!(ours, b"3q2-73Nl~Qssion-eu-central");

    // Both are streams of the same format and decode to the same bytes.
    for s in [&spec, &ours] {
        assert_eq!(decode(s, Profile::U).unwrap().bytes, input);
    }
    // And the tie is exact, not approximate.
    assert_eq!(encode_canonical(&input, Profile::U), ours);
}

#[test]
fn tv3_tilde_needs_no_escaping() {
    let out = dense(b"sub~alice~jones");
    assert_eq!(out, b"~Psub~alice~jones");
    assert_eq!(out.len(), 17);
    assert_eq!(base64_len(15), 20);
    assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"sub~alice~jones");
}

/// The four-character header form: `L1 = 63`, then twelve bits of `V`.
#[test]
fn tv4_extended_length_header() {
    let input = vec![b'a'; 100];
    let out = dense(&input);
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

// --- TV5: the F1/F2 conflict ---------------------------------------------

/// `hello~Alice` cannot be one literal inside a frame: the payload would carry
/// `~A` (F1). §8.2 says the encoder writes the tilde as base64 instead, and
/// TV5a says the dense encoder does not bother — plain base64 is shorter than
/// the forced mode switch. That part holds exactly.
///
/// TV5a's `legible` body, `~Fhellofg~FAlice`, does not: it is 16 characters
/// where the same encoder can write 15. A preset is a threshold and a mode
/// (§9.3), and the objective is still the length in §9.0, so no conforming
/// encoder writes the longer one. It is a legal stream and decodes correctly;
/// it is just not what `legible` produces.
#[test]
fn tv5a_dense_declines_the_forced_mode_switch() {
    let input = b"hello~Alice";
    let framed = encode_framed(input, Profile::U);
    let body = &framed[5..];
    assert_eq!(body, b"aGVsbG9-QWxpY2U");
    assert_eq!(body.len(), 15);

    let legible_body = b"~Fhellofg~FAlice";
    assert_eq!(legible_body.len(), 16);
    assert!(
        legible_body.len() > body.len(),
        "the spec's legible body is longer"
    );
    assert_eq!(
        decode_plain(legible_body, Profile::U).unwrap().bytes,
        input,
        "and is nevertheless a valid stream"
    );
}

#[test]
fn tv5b_the_same_bodies_as_whole_framed_streams() {
    let input = b"hello~Alice";
    let dense_stream = encode_framed(input, Profile::U);
    assert_eq!(dense_stream, b"~AAAPaGVsbG9-QWxpY2U");
    assert_eq!(dense_stream.len(), 20);

    let legible_stream = b"~AAAQ~Fhellofg~FAlice";
    assert_eq!(legible_stream.len(), 21);

    for s in [dense_stream.as_slice(), legible_stream.as_slice()] {
        let d = decode(s, Profile::U).unwrap();
        assert_eq!(d.bytes, input);
        assert_eq!(d.framing_seen, Framing::Framed);
        // `~A` occurs at index 0-1 and nowhere else — F′.
        assert_eq!(
            s.windows(2).filter(|w| *w == b"~A").count(),
            1,
            "{:?}",
            String::from_utf8_lossy(s)
        );
    }
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
        assert_eq!(d.framing_seen, Framing::Plain);
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
}

// --- TV9: F′ is the decoder's rule, F2 is the encoder's ------------------

#[test]
fn tv9a_a_spurious_frame_marker_is_rejected() {
    let stream = b"~AAAG~Cx~AA";
    assert_eq!(stream.len(), 11);
    assert_eq!(&stream[8..10], b"~A");
    assert_eq!(decode(stream, Profile::U), Err(Error::FrameRule));
}

/// The regression test §8.2 asks for: a literal ending in `~` breaks F2 and
/// nothing else, and a decoder that checks F2 rather than F′ rejects a valid
/// stream.
#[test]
fn tv9b_breaking_f2_without_breaking_f_prime_is_valid() {
    let stream = b"~AAAI~Cx~~Cyz";
    assert_eq!(stream.len(), 13);
    let d = decode(stream, Profile::U).expect("F′ is what the decoder checks");
    assert_eq!(d.bytes, b"x~yz");
    assert_eq!(d.framing_seen, Framing::Framed);
}

// --- TV10: padding, and why it may not be stripped in advance ------------

#[test]
fn tv10_padding() {
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

// --- TV11-TV12: framing detection and the error table --------------------

#[test]
fn tv11_framing_detection() {
    let cases: [(&[u8], Framing); 4] = [
        (b"", Framing::Plain),
        (b"YWxpY2U", Framing::Plain),
        (b"~Lalice.jones", Framing::Plain),
        (b"~AAAI~Cx~~Cyz", Framing::Framed),
    ];
    for (stream, framing) in cases {
        assert_eq!(framing_of(stream), framing);
        assert_eq!(
            decode(stream, Profile::U).unwrap().framing_seen,
            framing,
            "{:?}",
            String::from_utf8_lossy(stream)
        );
    }
    assert!(decode(b"", Profile::U).unwrap().bytes.is_empty());
    assert!(
        decode_framed(b"", Profile::U).is_ok(),
        "empty is valid in both modes"
    );
}

/// The same stream, two entry points, two errors — the point of Rule F.
///
/// TV11 writes the framed answer as "`E_TRUNCATED` / `E_FRAME_SYNC`". Only the
/// first is reachable: §10.3 checks the marker, then the length, and `abc` is
/// a well-formed 18-bit length of 108252 that the stream cannot satisfy.
#[test]
fn tv11_two_entry_points_two_errors() {
    assert_eq!(decode(b"~Aabc", Profile::U), Err(Error::Truncated));
    assert_eq!(decode_plain(b"~Aabc", Profile::U), Err(Error::ReservedLen));
    assert_eq!(decode_framed(b"YWxpY2U", Profile::U), Err(Error::FrameSync));
}

#[test]
fn tv12_error_cases() {
    assert_eq!(decode(b"abcde", Profile::U), Err(Error::Align));
    assert_eq!(decode_plain(b"~Aabc", Profile::U), Err(Error::ReservedLen));
    assert_eq!(decode(b"~Labc", Profile::U), Err(Error::Truncated));
    assert_eq!(decode(b"~Cab~", Profile::U), Err(Error::TrailingTilde));
    assert_eq!(decode(b"YWxp==", Profile::U), Err(Error::Padding));
    assert_eq!(decode(b"~Ca b", Profile::U), Err(Error::Profile));
    // `YWxpY2V` is `alice` plus a set bit in the last quantum: canonical
    // base64 would have written `YWxpY2U`.
    assert_eq!(decode(b"YWxpY2V", Profile::U), Err(Error::NonzeroTail));
}
