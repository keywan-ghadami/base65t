// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Writes `docs/vectors.json`: the vector set §16.8 asks to grow to 200, in a
//! form a second implementation can check itself against without reading any
//! of this code.
//!
//! §16.3 wants two independent implementations to agree byte for byte. One
//! implementation cannot produce that agreement, but it can produce the half
//! that is transferable: the bytes, written down, so that the second
//! implementation costs an afternoon instead of a project.
//!
//!     cargo run --release --example vectors > ../docs/vectors.json
//!
//! Both the input and the stream are hex, because an input is arbitrary bytes
//! and a JSON string is not. Where a stream is printable ASCII it also carries
//! `stream_ascii`, and `tests/vectors_json.rs` checks the two against each
//! other — a redundancy that is verified is worth more than a field that is
//! merely convenient.

use base66::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn ascii(bytes: &[u8]) -> Option<String> {
    bytes
        .iter()
        .all(|&b| (0x20..=0x7e).contains(&b))
        .then(|| String::from_utf8_lossy(bytes).into_owned())
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Inputs chosen so that a disagreement is likely to show: the profile
/// boundary, the tilde, the block boundary at 48, the tie at 27 admitted
/// bytes, the tails where raw and base64 tie, the padding characters, and
/// lengths on both sides of every one of those.
fn inputs() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    let named: [(&str, &[u8]); 9] = [
        ("empty", b""),
        ("tv1", b"alice.jones"),
        ("tv2", b"\xde\xad\xbe\xefsession-eu-central"),
        ("tv3", b"sub~alice~jones"),
        ("tv3b", b"hello~Alice"),
        ("tv5", b"the quick brown fox jumps over the lazy dog. again"),
        ("tv6", b"<<???>>"),
        ("tildes", b"~~~~~~"),
        ("equals", b"a=b="),
    ];
    for (name, data) in named {
        v.push((name.to_string(), data.to_vec()));
    }
    for n in [1usize, 3, 4, 6, 7, 47, 48, 49, 95, 96, 97] {
        v.push((format!("text-{n}"), vec![b'a'; n]));
        v.push((
            format!("count-{n}"),
            (0..n).map(|i| (i % 251) as u8).collect(),
        ));
    }
    // The tie at 27 admitted bytes of 48, and one either side of it, at the
    // start of the block and at its end.
    for admitted in [26usize, 27, 28] {
        let mut front = vec![b'a'; admitted];
        front.extend(vec![b' '; 48 - admitted]);
        v.push((format!("tie-front-{admitted}"), front));
        let mut back = vec![b' '; 48 - admitted];
        back.extend(vec![b'a'; admitted]);
        v.push((format!("tie-back-{admitted}"), back));
    }
    let mut r = Rng(0x5eed_0000_0000_1234);
    let pools: [(&str, &[u8]); 4] = [
        ("unreserved", b"abcXY9-._~"),
        ("mixed", b"abc ~A=,/\x00\xff"),
        ("textish", b"the quick brown fox.~jumps_over"),
        ("binary", b"\x00\x01\x7f\x80\xfe\xff"),
    ];
    for (label, pool) in pools {
        for k in 0..10 {
            let n = 1 + (r.next() % 150) as usize;
            let data: Vec<u8> = (0..n)
                .map(|_| pool[r.next() as usize % pool.len()])
                .collect();
            v.push((format!("{label}-{k}"), data));
        }
    }
    v
}

fn main() {
    // Two entry points, no profiles and no presets: the encoding, and the
    // base64url way out §14 is about.
    type Enc = fn(&[u8]) -> String;
    let kinds: [(&str, Enc); 2] = [
        ("encode", (|d: &[u8]| encode(d)) as Enc),
        ("base64url", |d: &[u8]| encode_base64url(d)),
    ];

    println!("{{");
    println!("  \"spec\": \"base66 v0.4, docs/spec-v0.4.md\",");
    println!(
        "  \"note\": \"Every entry is: the named entry point over input is exactly stream, and \
         decode(stream) is exactly input. Bytes are hex. A second implementation that reproduces \
         these byte for byte discharges \\u00a716.3.\","
    );
    println!("  \"vectors\": [");

    let mut first = true;
    let mut count = 0usize;
    for (name, data) in inputs() {
        for (sname, enc) in kinds {
            let stream = enc(&data);
            let back = decode(&stream).expect("its own output");
            assert_eq!(back, data);
            if !first {
                println!(",");
            }
            first = false;
            count += 1;
            print!(
                "    {{\"name\": {}, \"kind\": \"{sname}\", \"input\": \"{}\", \"stream\": \"{}\"",
                json_string(&format!("{name}/{sname}")),
                hex(&data),
                hex(stream.as_bytes())
            );
            if let Some(a) = ascii(stream.as_bytes()) {
                print!(", \"stream_ascii\": {}", json_string(&a));
            }
            print!("}}");
        }
    }
    println!("\n  ],");
    println!("  \"count\": {count}");
    println!("}}");
}
