// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Writes `docs/vectors.json`: the vector set §16.8 asks to grow to 200, in a
//! form a second implementation can check itself against without reading any
//! of this code.
//!
//! §16.3 wants `encode_canonical` to agree byte for byte across two
//! independent implementations. One implementation cannot produce that
//! agreement, but it can produce the half that is transferable: the bytes,
//! written down, so that the second implementation costs an afternoon instead
//! of a project.
//!
//!     cargo run --release --example vectors > ../docs/vectors.json
//!
//! Everything is hex, because a profile-B stream is not text. Where a stream
//! is printable ASCII it also carries `stream_ascii`, and `tests/vectors_json.rs`
//! checks the two against each other — a redundancy that is verified is worth
//! more than a field that is merely convenient.

use base65t::*;

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
/// boundary, the tilde, `~A`, the header bands at 62 and 63, the padding
/// characters, and lengths on both sides of every threshold.
fn inputs() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    let named: [(&str, &[u8]); 8] = [
        ("empty", b""),
        ("tv1", b"alice.jones"),
        ("tv2", b"\xde\xad\xbe\xefsession-eu-central"),
        ("tv3", b"sub~alice~jones"),
        ("tv5", b"hello~Alice"),
        ("tv6", b"<<???>>"),
        ("tilde-a", b"~A~A~A"),
        ("equals", b"a=b="),
    ];
    for (name, data) in named {
        v.push((name.to_string(), data.to_vec()));
    }
    for n in [1usize, 7, 10, 11, 62, 63, 64, 124, 125] {
        v.push((format!("text-{n}"), vec![b'a'; n]));
        v.push((
            format!("count-{n}"),
            (0..n).map(|i| (i % 251) as u8).collect(),
        ));
    }
    let mut r = Rng(0x5eed_0000_0000_1234);
    let pools: [(&str, &[u8]); 4] = [
        ("unreserved", b"abcXY9-._~"),
        ("mixed", b"abc ~A=,/\x00\xff"),
        ("textish", b"the quick brown fox.~jumps_over"),
        ("binary", b"\x00\x01\x7f\x80\xfe\xff"),
    ];
    for (label, pool) in pools {
        for k in 0..8 {
            let n = 1 + (r.next() % 90) as usize;
            let data: Vec<u8> = (0..n)
                .map(|_| pool[r.next() as usize % pool.len()])
                .collect();
            v.push((format!("{label}-{k}"), data));
        }
    }
    v
}

fn main() {
    let presets: [(&str, Preset); 6] = [
        ("dense", Preset::Dense),
        ("dense-fast", Preset::DenseFast),
        ("legible", Preset::Legible),
        ("canonical", Preset::Canonical),
        ("opaque", Preset::Opaque),
        ("framed", Preset::Framed),
    ];
    let profiles: [(&str, Profile); 3] = [("U", Profile::U), ("T", Profile::T), ("B", Profile::B)];

    println!("{{");
    println!("  \"spec\": \"base65t v0.2, docs/spec-v0.2.de.md\",");
    println!(
        "  \"note\": \"Every entry is: encode(input, preset, profile) is exactly stream, and \
         decode(stream, profile) is exactly input. Bytes are hex. A second implementation that \
         reproduces the canonical entries byte for byte discharges conformance point 3 of \\u00a716.\","
    );
    println!("  \"vectors\": [");

    let mut first = true;
    let mut count = 0usize;
    for (name, data) in inputs() {
        for (sname, preset) in presets {
            // A preset's stream depends on the profile only through which
            // bytes a literal may carry, so a wider profile often produces the
            // same stream as a narrower one. Those are one entry listing every
            // profile it holds for, rather than three that differ in a field.
            let mut groups: Vec<(Vec<u8>, Vec<&str>)> = Vec::new();
            for (pname, profile) in profiles {
                let stream = encode_with(&data, preset, profile);
                let back = decode(&stream, profile).expect("its own output");
                assert_eq!(back.bytes, data);
                match groups.iter_mut().find(|(s, _)| *s == stream) {
                    Some((_, names)) => names.push(pname),
                    None => groups.push((stream, vec![pname])),
                }
            }
            for (stream, pnames) in groups {
                if !first {
                    println!(",");
                }
                first = false;
                count += 1;
                print!(
                    "    {{\"name\": {}, \"preset\": \"{sname}\", \"profiles\": [{}], \
                     \"input\": \"{}\", \"stream\": \"{}\"",
                    json_string(&format!("{name}/{sname}/{}", pnames.join(""))),
                    pnames
                        .iter()
                        .map(|p| format!("\"{p}\""))
                        .collect::<Vec<_>>()
                        .join(", "),
                    hex(&data),
                    hex(&stream)
                );
                if let Some(a) = ascii(&stream) {
                    print!(", \"stream_ascii\": {}", json_string(&a));
                }
                print!("}}");
            }
        }
    }
    println!("\n  ],");
    println!("  \"count\": {count}");
    println!("}}");
}
