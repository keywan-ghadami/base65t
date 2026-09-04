// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §16.2 — every canonical base64 stream decodes to the bytes it came from.
//!
//! The claim is about somebody else's base64, so it is checked against
//! somebody else's base64: `base64(1)` from coreutils and Python's `base64`
//! module, both padded and unpadded, in both alphabets. A base64 written by
//! this crate and read by this crate would only show that it is
//! self-consistent.
//!
//! The one expected deviation is in here too, as §1.1 asks: a stream with a
//! set bit in the unused tail is accepted by permissive libraries and rejected
//! with `E_NONZERO_TAIL` here. That is a difference in what the two consider a
//! stream, not a difference in what they decode.
//!
//! Missing tools skip rather than fail, so the suite still runs on a machine
//! without coreutils or Python.

use std::io::Write;
use std::process::{Command, Stdio};

use base66::*;

fn have(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn pipe(cmd: &str, args: &[&str], input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input)
        .expect("write");
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "{cmd} failed");
    out.stdout
        .into_iter()
        .filter(|&b| b != b'\n' && b != b'\r')
        .collect()
}

/// Boundary lengths, every byte value, and a long seeded run.
fn cases() -> Vec<Vec<u8>> {
    let mut s: u32 = 0x5eed_1234;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s & 0xff) as u8
    };
    let long: Vec<u8> = (0..3000).map(|_| next()).collect();
    let mut v: Vec<Vec<u8>> = (0..=16).map(|n| long[..n].to_vec()).collect();
    v.push((0..=255u8).collect());
    v.push(b"alice.jones".to_vec());
    v.push(long);
    v
}

#[test]
fn reads_what_base64_1_writes() {
    if !have("base64") {
        eprintln!("skipping: no base64(1)");
        return;
    }
    for data in cases() {
        let padded = pipe("base64", &["-w0"], &data);
        let d = decode_detailed(&padded)
            .unwrap_or_else(|e| panic!("{e} on {:?}", String::from_utf8_lossy(&padded)));
        assert_eq!(d.bytes, data);
        assert_eq!(d.padding_seen, data.len() % 3 != 0);

        // The same stream with the padding taken off is the same bytes: this
        // is the migration path in §1.1, where a producer stops padding and
        // nothing downstream has to be told.
        let stripped: Vec<u8> = padded.iter().copied().filter(|&c| c != b'=').collect();
        assert_eq!(decode_detailed(&stripped).unwrap().bytes, data);
    }
}

#[test]
fn reads_what_python_writes() {
    if !have("python3") {
        eprintln!("skipping: no python3");
        return;
    }
    // Four spellings of the same bytes: two alphabets, padded and not. All
    // four are canonical base64 and all four must decode to the input.
    for data in cases() {
        for variant in ["b64encode", "urlsafe_b64encode"] {
            for strip in [false, true] {
                let one = format!(
                    "import base64,sys;s=base64.{variant}(sys.stdin.buffer.read()).decode();\
                     sys.stdout.write(s.rstrip('=') if {strip} else s)",
                    strip = if strip { "True" } else { "False" }
                );
                let stream = pipe("python3", &["-c", &one], &data);
                let d = decode_detailed(&stream)
                    .unwrap_or_else(|e| panic!("{e} on {variant}, stripped={strip}"));
                assert_eq!(d.bytes, data, "{variant}, stripped={strip}");
                assert_eq!(d.padding_seen, !strip && data.len() % 3 != 0);
                let expected = if variant == "b64encode" {
                    // `+` and `/` only appear in some payloads; when neither
                    // occurs the two alphabets are indistinguishable.
                    if stream.iter().any(|&c| c == b'+' || c == b'/') {
                        AlphabetSeen::Classic
                    } else {
                        AlphabetSeen::None
                    }
                } else if stream.iter().any(|&c| c == b'-' || c == b'_') {
                    AlphabetSeen::Url
                } else {
                    AlphabetSeen::None
                };
                assert_eq!(d.alphabet_seen, expected, "{variant}");
            }
        }
    }
}

/// The expected deviation, spelled out (§1.1): Python accepts a stream whose
/// unused tail bits are set and this decoder does not. Both are deliberate.
#[test]
fn nonzero_tail_bits_are_the_documented_disagreement() {
    if !have("python3") {
        eprintln!("skipping: no python3");
        return;
    }
    // `YWxpY2U` is "alice"; `YWxpY2V` differs only in the bits the last
    // character does not use.
    let permissive = pipe(
        "python3",
        &[
            "-c",
            "import base64,sys;sys.stdout.buffer.write(base64.b64decode('YWxpY2V='))",
        ],
        b"",
    );
    assert_eq!(permissive, b"alice", "Python takes it");
    assert_eq!(
        decode(b"YWxpY2V"),
        Err(Error::NonzeroTail),
        "and this decoder does not"
    );
    assert_eq!(decode(b"YWxpY2U").unwrap(), b"alice");
}

/// The other direction of §16.2: what `encode_base64url` writes is
/// what base64(1) writes, minus the padding it is not allowed to write.
#[test]
fn base64url_is_base64_1_without_the_padding() {
    if !have("base64") {
        eprintln!("skipping: no base64(1)");
        return;
    }
    for data in cases() {
        let theirs = pipe("base64", &["-w0"], &data);
        let theirs_url: Vec<u8> = theirs
            .iter()
            .filter(|&&c| c != b'=')
            .map(|&c| match c {
                b'+' => b'-',
                b'/' => b'_',
                c => c,
            })
            .collect();
        assert_eq!(encode_base64url(&data).as_bytes(), theirs_url);
    }
}
