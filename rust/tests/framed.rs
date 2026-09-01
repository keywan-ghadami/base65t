// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §16.4 — no false frame markers, however adversarial the literal bytes.
//!
//! The claim under test is invariant F′ (§8.2): in a framed stream, `~A`
//! occurs only where a frame begins. The encoder keeps it by keeping F1 and
//! F2, which are stricter; the decoder checks F′ and nothing else, which is
//! why TV9b has to be accepted. Both halves are here, and the adversarial
//! input is generated rather than chosen: every arrangement of `~`, `A` and
//! one other byte up to nine long, plus long runs built out of the same three.

use base65t::*;

/// Where `~A` occurs. In a framed stream this must be exactly the frame
/// starts, and nothing in a body may add to it.
fn markers(stream: &[u8]) -> Vec<usize> {
    stream
        .windows(2)
        .enumerate()
        .filter(|(_, w)| *w == b"~A")
        .map(|(i, _)| i)
        .collect()
}

/// Frame starts, found the way §10.3 says a reader may find them: forwards
/// from anywhere, without decoding what precedes.
fn frame_starts(stream: &[u8]) -> Vec<usize> {
    let mut v = Vec::new();
    let mut pos = 0;
    while pos < stream.len() {
        assert_eq!(&stream[pos..pos + 2], b"~A");
        v.push(pos);
        let len = |i: usize| {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
                .iter()
                .position(|&c| c == stream[i])
                .expect("alphabet character")
        };
        pos += 5 + ((len(pos + 2) << 12) | (len(pos + 3) << 6) | len(pos + 4));
    }
    v
}

fn exhaustive_three_byte_inputs(max_len: usize) -> Vec<Vec<u8>> {
    let alphabet = b"~Ax";
    let mut all = vec![Vec::new()];
    let mut level = vec![Vec::new()];
    for _ in 0..max_len {
        let mut next = Vec::new();
        for base in &level {
            for &c in alphabet {
                let mut v: Vec<u8> = base.clone();
                v.push(c);
                next.push(v);
            }
        }
        all.extend(next.iter().cloned());
        level = next;
    }
    all
}

#[test]
fn f_prime_holds_over_every_short_arrangement_of_tilde_and_a() {
    for data in exhaustive_three_byte_inputs(9) {
        for profile in [Profile::U, Profile::T, Profile::B] {
            let stream = encode_framed(&data, profile);
            let d = decode(&stream, profile).expect("valid");
            assert_eq!(d.bytes, data, "{data:?}");
            assert_eq!(
                markers(&stream),
                frame_starts(&stream),
                "{:?} -> {:?}",
                String::from_utf8_lossy(&data),
                String::from_utf8_lossy(&stream)
            );
        }
    }
}

#[test]
fn f_prime_holds_over_long_adversarial_runs() {
    let mut s: u32 = 0xfeed_face;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    for n in [100usize, 1000, 20_000] {
        for weights in [
            b"~A".as_slice(),
            b"~Aa".as_slice(),
            b"~Aabcdefghij".as_slice(),
        ] {
            let data: Vec<u8> = (0..n).map(|_| weights[next() % weights.len()]).collect();
            let stream = encode_framed(&data, Profile::U);
            assert_eq!(decode(&stream, Profile::U).unwrap().bytes, data);
            assert_eq!(markers(&stream), frame_starts(&stream));
        }
    }
}

/// §8.1's recommendation, which is the whole reason for framing: every frame
/// but the last decodes to exactly `FRAME_BYTES`, so a byte offset names a
/// frame without a trailer and without decoding anything before it.
#[test]
fn frames_are_a_fixed_number_of_decoded_bytes() {
    let data: Vec<u8> = (0..3 * FRAME_BYTES + 17).map(|i| (i % 251) as u8).collect();
    let stream = encode_framed(&data, Profile::B);
    let starts = frame_starts(&stream);
    assert_eq!(starts.len(), 4);

    for (k, &start) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(stream.len());
        let frame = &stream[start..end];
        let decoded = decode_framed(frame, Profile::B).expect("a frame is a stream of one");
        let expected = if k == 3 { 17 } else { FRAME_BYTES };
        assert_eq!(decoded.bytes.len(), expected);
        assert_eq!(
            decoded.bytes,
            data[k * FRAME_BYTES..k * FRAME_BYTES + expected]
        );
    }
}

/// The decoder checks F′, not F2 (§8.2). These are TV9a and TV9b again, with
/// the neighbouring cases that show where the line falls.
#[test]
fn the_decoder_checks_f_prime_and_not_f2() {
    // A literal ending in `~` followed by a literal: legal.
    assert_eq!(decode(b"~AAAI~Cx~~Cyz", Profile::U).unwrap().bytes, b"x~yz");
    // The same literal followed by a base64 segment beginning `A`: not.
    assert_eq!(decode(b"~AAAG~Cx~AA", Profile::U), Err(Error::FrameRule));
    // And a body with `~A` anywhere else is rejected before it is decoded,
    // which is what keeps Rule F from running recursively (§10.3).
    assert_eq!(decode(b"~AAAC~A", Profile::U), Err(Error::FrameRule));
}

/// A framed stream handed to the plain decoder, and the other way round. Both
/// answers are correct for the entry point that gave them (§10.2).
#[test]
fn the_entry_points_do_not_bleed_into_each_other() {
    let stream = encode_framed(b"alice.jones and a longer tail", Profile::U);
    assert_eq!(decode_plain(&stream, Profile::U), Err(Error::ReservedLen));
    let plain = encode_dense(b"alice.jones", Profile::U);
    assert_eq!(decode_framed(&plain, Profile::U), Err(Error::FrameSync));
}

/// Framing costs five characters a frame and nothing else (§8.1), and §9.4
/// does not cover the difference. Measured against the plain encoding of the
/// same chunks — on input without a tilde, where F1 and F2 have nothing to
/// forbid and the bodies are the plain streams.
#[test]
fn framing_costs_five_characters_a_frame() {
    for n in [0usize, 1, 100, FRAME_BYTES, FRAME_BYTES + 1] {
        let data: Vec<u8> = (0..n).map(|i| b"abcdefghij"[i % 10]).collect();
        let frames = n.div_ceil(FRAME_BYTES);
        let plain: usize = data
            .chunks(FRAME_BYTES)
            .map(|c| encode_dense(c, Profile::U).len())
            .sum();
        assert_eq!(
            encode_framed(&data, Profile::U).len(),
            plain + 5 * frames,
            "n = {n}"
        );
    }
}

/// The one thing framing is not: smaller. A frame header is five characters a
/// frame that plain mode does not spend, and on a short payload that is the
/// whole difference between beating base64 and not.
#[test]
fn framing_can_be_worse_than_base64() {
    let data = b"alice.jones";
    assert!(encode_framed(data, Profile::U).len() > (4 * data.len()).div_ceil(3));
    assert!(encode_dense(data, Profile::U).len() < (4 * data.len()).div_ceil(3));
}

/// Two questions the specification does not answer, pinned to what this
/// decoder does — see FINDINGS.md, "What a frame body is a stream of".
///
/// §8.1 calls a frame body "<Plain-Mode-Stream>", and §10.3 hands it to
/// `decode_plain`, for which the body *is* the stream. But Rule P (§5.3) and
/// Rule A (§5.4) are both written about "the stream", and the two readings
/// pull apart exactly here: padding is a question about the end of a stream,
/// and mixing alphabets is a question about the whole of one.
#[test]
fn padding_is_recognised_at_the_end_of_a_frame_body() {
    // `YWxpY2U=` is eight characters, so the frame length is 'A','A','I'.
    let d = decode(b"~AAAIYWxpY2U=", Profile::U).expect("the body ends where it ends");
    assert_eq!(d.bytes, b"alice");
    assert!(d.padding_seen);
    // The encoder never writes this: padding is validated, never produced
    // (§5.1, §14).
    assert!(!encode_framed(b"alice", Profile::U).contains(&b'='));
}

#[test]
fn rule_a_is_read_across_frames_and_not_within_one() {
    let mixed = b"~AAAKPDw_Pz8-Pg~AAAKPDw/Pz8+Pg";
    assert_eq!(decode(mixed, Profile::U), Err(Error::MixedAlphabet));
    let consistent = b"~AAAKPDw_Pz8-Pg~AAAKPDw_Pz8-Pg";
    let d = decode(consistent, Profile::U).expect("one alphabet throughout");
    assert_eq!(d.alphabet_seen, AlphabetSeen::Url);
    assert_eq!(d.bytes, b"<<???>><<???>>");
}
