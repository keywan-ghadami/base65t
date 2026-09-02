// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §16.1 — `decode(encode(x)) == x`, over every profile and every preset.
//!
//! The corpus is generated from a counter stream rather than collected, so a
//! failure names an input that can be regenerated exactly, and it is built to
//! hit what the format branches on: the profile boundary, the tilde, the
//! header bands at 62 and 63 bytes, the literal cap at 4158, and text that is
//! legal apart from the occasional byte that is not.

use base65t::*;

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn byte(&mut self) -> u8 {
        (self.next() & 0xff) as u8
    }
    fn below(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }
}

/// Inputs, with what each one is for.
fn corpus() -> Vec<(String, Vec<u8>)> {
    let mut r = Rng(0x1234_5678);
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();

    v.push(("empty".into(), Vec::new()));
    for n in 1..=20 {
        v.push((
            format!("all bytes, {n} long"),
            (0..n).map(|i| i as u8).collect(),
        ));
    }
    // The header bands meet at 62/63 and the segment cap is 4158.
    for n in [61, 62, 63, 64, 124, 125, 4157, 4158, 4159, 4160, 8317] {
        v.push((
            format!("text of {n}"),
            (0..n).map(|i| b"abcdefghij"[i % 10]).collect(),
        ));
    }
    for n in [0usize, 1, 7, 10, 11, 100, 3000] {
        v.push((
            format!("high entropy, {n}"),
            (0..n).map(|_| r.byte()).collect(),
        ));
    }
    // Text with a rising share of bytes no profile-U literal may carry, which
    // is where segmentation has to decide something.
    for percent in [0, 1, 5, 20, 50] {
        let data: Vec<u8> = (0..2000)
            .map(|_| {
                if r.below(100) < percent {
                    b" ,;"[r.below(3)]
                } else {
                    b"abcdefghijklmnop.-_~"[r.below(20)]
                }
            })
            .collect();
        v.push((format!("{percent}% untransportable"), data));
    }
    // Tildes, and tilde-A in particular: the framed encoder has to break these
    // out of literals (§8.2).
    v.push(("tilde A repeated".into(), b"~A".repeat(200)));
    v.push(("tildes".into(), b"~".repeat(200)));
    v.push((
        "text around tilde A".into(),
        b"abcdefghijkl~Amnopqrstuvwx".repeat(30),
    ));
    v.push((
        "frame boundary".into(),
        (0..3 * FRAME_BYTES + 17).map(|i| (i % 251) as u8).collect(),
    ));
    v
}

const PROFILES: [Profile; 3] = [Profile::U, Profile::T, Profile::B];
const PRESETS: [Preset; 4] = [
    Preset::Dense,
    Preset::Canonical,
    Preset::Opaque,
    Preset::Framed,
];

#[test]
fn decode_of_encode_is_the_identity() {
    for (name, data) in corpus() {
        for profile in PROFILES {
            for preset in PRESETS {
                let out = encode_with(&data, preset, profile);
                let d = decode(&out, profile)
                    .unwrap_or_else(|e| panic!("{name}, {preset:?}, {profile:?}: {e}"));
                assert_eq!(d.bytes, data, "{name}, {preset:?}, {profile:?}");

                // An encoder never writes padding and never writes the classic
                // alphabet (§5.1, §5.3), so a decoder never sees either.
                assert!(!d.padding_seen, "{name}, {preset:?}");
                assert_ne!(d.alphabet_seen, AlphabetSeen::Classic, "{name}, {preset:?}");

                // Rule F reads the encoder's own output correctly: a plain
                // stream can never begin `~A`, because 0 is not a length.
                let expect = if preset == Preset::Framed && !data.is_empty() {
                    Framing::Framed
                } else {
                    Framing::Plain
                };
                assert_eq!(d.framing_seen, expect, "{name}, {preset:?}");

                // And the strict entry points agree with the permissive one.
                let strict = match expect {
                    Framing::Plain => decode_plain(&out, profile),
                    Framing::Framed => decode_framed(&out, profile),
                };
                assert_eq!(
                    strict.map(|d| d.bytes),
                    Ok(data.clone()),
                    "{name}, {preset:?}"
                );
                assert_eq!(
                    decode_url_strict(&out, profile).map(|d| d.bytes),
                    Ok(data.clone()),
                    "{name}, {preset:?}"
                );
            }
        }
    }
}

/// `opaque` is Base64URL and nothing else — that is its whole promise (§9.3),
/// and the profile has no say in it because there are no literals to constrain.
#[test]
fn opaque_leaks_nothing() {
    for (name, data) in corpus() {
        let out = encode_opaque(&data);
        assert!(!out.contains(&b'~'), "{name}");
        assert_eq!(out.len(), (4 * data.len()).div_ceil(3), "{name}");
        for profile in PROFILES {
            assert_eq!(encode_with(&data, Preset::Opaque, profile), out, "{name}");
        }
    }
}

/// A stream that came out of the encoder is decoded by the profile it was
/// encoded under. A wider profile still reads it; a narrower one may not, and
/// that is the profile doing its job rather than a bug.
#[test]
fn a_wider_profile_reads_a_narrower_ones_stream() {
    for (name, data) in corpus() {
        let out = encode_dense(&data, Profile::U);
        for profile in [Profile::T, Profile::B] {
            assert_eq!(decode(&out, profile).unwrap().bytes, data, "{name}");
        }
    }
}
