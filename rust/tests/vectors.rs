// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §15 of v0.2, one test per vector.
//!
//! Three of these were wrong in v0.1 and are now assertions rather than
//! narrated discrepancies: TV2 named one of three equally dense streams
//! without a rule that picked it, TV5a's `legible` body was longer than the
//! base64 the same encoder writes, and TV11's last line admitted two error
//! codes where §10.3's order of checks allows one. FINDINGS.md records how
//! they were found, `docs/errata-v0.1.de.md` what was decided.

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

/// Three segmentations are 26 characters here: absorbing one or two text bytes
/// into the base64 segment costs nothing, since `ceil(4k/3) + (22-k) + 2` is 26
/// for k = 4, 5 and 6 alike. Which one comes out is not a matter of taste but
/// of which rule ran.
///
/// `dense` scans and takes the run whole (§9.2.1), so it stops the base64
/// segment at the first admissible byte: k = 4. `canonical` minimises and then
/// breaks the tie by the order of §11.1, where `B < S` at index 4 picks k = 6.
/// Both are 26 characters and both decode to the input.
///
/// The `dense` stream is the one v0.1 printed here. The vector was written for
/// a scanning encoder all along, which is worth knowing: the exact programme
/// arrived later and took the vector with it.
#[test]
fn tv2_binary_prefix_then_text() {
    let input = [b"\xde\xad\xbe\xef".as_slice(), b"session-eu-central"].concat();
    let ours = dense(&input);
    assert_eq!(ours, b"3q2-7w~Ssession-eu-central");
    assert_eq!(ours.len(), 26);
    assert_eq!(base64_len(input.len()), 30);

    let canonical = encode_canonical(&input, Profile::U);
    assert_eq!(canonical, b"3q2-73Nl~Qssion-eu-central");
    assert_eq!(
        canonical.len(),
        ours.len(),
        "the same length, a different rule"
    );
    for s in [&ours, &canonical] {
        assert_eq!(decode(s, Profile::U).unwrap().bytes, input);
    }
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
/// v0.1 gave `legible` the 16-character body `~Fhellofg~FAlice` here. No
/// conforming encoder writes it: `legible` minimises the length first (§9.3)
/// and 15 is shorter than 16 — the passthrough share only decides between
/// segmentations of *equal* length. It is a legal stream and decodes
/// correctly; it is not an encoder's output.
#[test]
fn tv5a_dense_declines_the_forced_mode_switch() {
    let input = b"hello~Alice";
    let framed = encode_framed(input, Profile::U);
    let body = &framed[5..];
    assert_eq!(body, b"aGVsbG9-QWxpY2U");
    assert_eq!(body.len(), 15);

    // Plain mode has no F1/F2 to enforce, so there the literal wins outright.
    assert_eq!(encode_legible(input, Profile::U), b"~Lhello~Alice");

    let v01_legible = b"~Fhellofg~FAlice";
    assert_eq!(v01_legible.len(), 16);
    assert!(v01_legible.len() > body.len());
    assert_eq!(
        decode_plain(v01_legible, Profile::U).unwrap().bytes,
        input,
        "a valid stream, just not one an encoder writes"
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
/// v0.1 wrote the framed answer as "`E_TRUNCATED` / `E_FRAME_SYNC`". Only the
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

// --- TV13-TV15: what v0.2 decides that v0.1 left open ---------------------

/// The tie-break of §9.0 and §11.1, on the smallest input where it decides
/// anything: nine bytes the profile admits, then one it does not.
///
/// Both segmentations are thirteen characters. At index 7 the choice is `B`
/// against `L`, and `B < L` takes the shorter literal — ending it early aligns
/// the base64 run so that the remaining three bytes cost two characters
/// instead of four. The second stream is what v0.1's *Berechnung* produced.
#[test]
fn tv13_the_tie_break_decides() {
    let input = b"aaaaaaaaa ";
    let ours = encode_canonical(input, Profile::U);
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

/// `legible` against `dense` on one input: same length, different bytes.
///
/// `dense` needs eleven bytes before it takes a literal (§9.1) and this run is
/// seven, so it writes base64 throughout. `legible` has no threshold and takes
/// the literal at equal length. That is the whole difference between the two
/// presets — readability at no cost, not readability against size.
#[test]
fn tv14_legible_against_dense() {
    let input = [b"\xde\xad\xbe\xef".as_slice(), b"abcdefg"].concat();
    let d = encode_dense(&input, Profile::U);
    let l = encode_legible(&input, Profile::U);
    assert_eq!(d, b"3q2-72FiY2RlZmc");
    assert_eq!(l, b"3q2-7w~Habcdefg");
    assert_eq!(d.len(), l.len());
    assert_eq!(d.len(), base64_len(input.len()));
    for s in [&d, &l] {
        assert_eq!(decode(s, Profile::U).unwrap().bytes, input);
    }
}

/// Padding does not reach into a frame body (§5.3). The same base64 text, once
/// as a stream and once as a body, gets two different answers — and that is
/// the point: inside a frame `=` is a character without a value at an alphabet
/// position, because no producer of ordinary base64 emits frames.
#[test]
fn tv15_padding_stops_at_the_stream() {
    let d = decode(b"YWxpY2U=", Profile::U).expect("plain, at the end of the stream");
    assert_eq!(d.bytes, b"alice");
    assert!(d.padding_seen);

    assert_eq!(decode(b"~AAAIYWxpY2U=", Profile::U), Err(Error::Charset));
    assert_eq!(decode(b"~AAAHYWxpY2U", Profile::U).unwrap().bytes, b"alice");
}
