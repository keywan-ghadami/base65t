// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §16.1 — `decode(encode(x)) == x`, over both entry points.
//!
//! The corpus is generated from a counter stream rather than collected, so a
//! failure names an input that can be regenerated exactly, and it is built to
//! hit what the format branches on: the alphabet boundary, the tilde, the
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
    // Text with a rising share of bytes no raw block may carry, which
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
    // Tildes, and doubled tildes in particular: `~` is the one character a
    // raw block may carry that also opens a block, so it is where a decoder
    // that guesses instead of counting comes apart.
    v.push(("tilde A repeated".into(), b"~A".repeat(200)));
    v.push(("tildes".into(), b"~".repeat(200)));
    v.push((
        "text around tilde A".into(),
        b"abcdefghijkl~Amnopqrstuvwx".repeat(30),
    ));
    v.push((
        "many blocks".into(),
        (0..30 * BLOCK_BYTES + 17)
            .map(|i| (i % 251) as u8)
            .collect(),
    ));
    v.push((
        "many raw blocks".into(),
        (0..30 * BLOCK_BYTES + 17)
            .map(|i| b"abcdefghij.-_~"[i % 14])
            .collect(),
    ));
    v
}

/// The two entry points a caller has, so every claim below is checked of both.
type Enc = fn(&[u8]) -> Vec<u8>;

fn kinds() -> [(&'static str, Enc); 2] {
    [("encode", encode as Enc), ("base64url", encode_base64url)]
}

#[test]
fn decode_of_encode_is_the_identity() {
    for (name, data) in corpus() {
        for (kind, enc) in kinds() {
            let out = enc(&data);
            let d = decode(&out).unwrap_or_else(|e| panic!("{name}, {kind}: {e}"));
            assert_eq!(d.bytes, data, "{name}, {kind}");

            // An encoder never writes padding and never writes the classic
            // alphabet (§5.1, §5.3), so a decoder never sees either.
            assert!(!d.padding_seen, "{name}, {kind}");
            assert_ne!(d.alphabet_seen, AlphabetSeen::Classic, "{name}, {kind}");

            // And the strict entry point agrees with the permissive one.
            assert_eq!(
                decode_url_strict(&out).map(|d| d.bytes),
                Ok(data.clone()),
                "{name}, {kind}"
            );
        }
    }
}

/// `encode_base64url` is Base64URL and nothing else — that is its whole
/// promise (§14), and no raw block ever appears in it.
#[test]
fn the_base64url_entry_point_leaks_nothing() {
    for (name, data) in corpus() {
        let out = encode_base64url(&data);
        assert!(!out.contains(&b'~'), "{name}");
        assert_eq!(out.len(), (4 * data.len()).div_ceil(3), "{name}");
    }
}

/// There is one alphabet, so a stream the encoder wrote is read by the
/// decoder with nothing to configure between them (§0.3).
#[test]
fn a_stream_reads_back_with_no_parameter_at_all() {
    for (name, data) in corpus() {
        let out = encode(&data);
        assert_eq!(decode(&out).unwrap().bytes, data, "{name}");
    }
}
