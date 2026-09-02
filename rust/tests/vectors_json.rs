// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `docs/vectors.json` against the implementation it was written from.
//!
//! The file is the transferable half of conformance point 3 (§16.3): a second
//! implementation checks itself against those bytes without reading any of
//! this code. That is only worth something if the file cannot age, so this
//! test fails the moment the encoder and the published bytes part company —
//! and `examples/vectors` regenerates it.
//!
//! Parsed by hand rather than with serde: the crate has no dependencies and
//! this file is not the place to acquire one.

use base65t::*;

fn field<'a>(entry: &'a str, key: &str) -> Option<&'a str> {
    let at = entry.find(&format!("\"{key}\": "))? + key.len() + 4;
    let rest = &entry[at..];
    if let Some(inner) = rest.strip_prefix('"') {
        Some(&inner[..inner.find('"')?])
    } else if let Some(inner) = rest.strip_prefix('[') {
        Some(&inner[..inner.find(']')?])
    } else {
        None
    }
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

#[test]
fn the_published_vectors_are_what_this_encoder_writes() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/vectors.json");
    let text = std::fs::read_to_string(path).expect("docs/vectors.json is part of the repository");

    let mut checked = 0usize;
    for line in text.lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with("{\"name\"") {
            continue;
        }
        let name = field(line, "name").expect("name");
        let preset = match field(line, "preset").expect("preset") {
            "dense" => Preset::Dense,
            "dense-fast" => Preset::DenseFast,
            "legible" => Preset::Legible,
            "canonical" => Preset::Canonical,
            "opaque" => Preset::Opaque,
            "framed" => Preset::Framed,
            other => panic!("{name}: unknown preset {other}"),
        };
        let input = unhex(field(line, "input").expect("input"));
        let stream = unhex(field(line, "stream").expect("stream"));

        // The ASCII field is a convenience, so it is checked rather than
        // trusted: a redundancy nobody verifies is a place for two truths.
        if let Some(a) = field(line, "stream_ascii") {
            let unescaped = a.replace("\\\"", "\"").replace("\\\\", "\\");
            assert_eq!(unescaped.as_bytes(), stream, "{name}: ascii and hex differ");
        }

        for p in field(line, "profiles").expect("profiles").split(',') {
            let profile = match p.trim().trim_matches('"') {
                "U" => Profile::U,
                "T" => Profile::T,
                "B" => Profile::B,
                other => panic!("{name}: unknown profile {other}"),
            };
            assert_eq!(
                encode_with(&input, preset, profile),
                stream,
                "{name}: encoder and published vector disagree"
            );
            assert_eq!(
                decode(&stream, profile)
                    .expect("published vectors decode")
                    .bytes,
                input,
                "{name}: the vector does not decode to its own input"
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 200,
        "§16.8 asks for at least 200 vectors; checked {checked}"
    );
}
