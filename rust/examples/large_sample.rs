// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Writes one long input and its encoding, so that the other
//! implementation can check both against its own.
//!
//!     cargo run --release --example large_sample -- /tmp/in.bin /tmp/in.b65
//!
//! `docs/vectors.json` cannot reach here: every vector in it is under a
//! kilobyte, and a hex dump of a quarter-megabyte input would be half a
//! megabyte of repository. But a long stream is where a segmentation mistake
//! hides -- one did, when the encoder still cut blocks whose seams could
//! end on a partial quantum.
//!
//! The input is mixed on purpose. Homogeneous input shows nothing: noise makes
//! one base64 run, and profile-legal text makes none.

use base65t::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (input_path, stream_path) = match args.as_slice() {
        [a, b] => (a, b),
        _ => {
            eprintln!("usage: large_sample <input-path> <stream-path>");
            std::process::exit(2);
        }
    };

    let mut s: u32 = 0x51ce_1234;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    };
    const LEN: usize = 262_923;
    let mut data: Vec<u8> = Vec::with_capacity(LEN);
    while data.len() < LEN {
        let run = 1 + (next() % 60) as usize;
        if next() % 3 == 0 {
            data.extend((0..run).map(|_| (next() & 0xff) as u8));
        } else {
            data.extend((0..run).map(|i| b"abcdefghij"[i % 10]));
        }
    }

    let stream = encode_with(&data, Profile::U);
    assert_eq!(
        decode(&stream, Profile::U).expect("its own output").bytes,
        data
    );
    std::fs::write(input_path, &data).expect("write input");
    std::fs::write(stream_path, &stream).expect("write stream");
    println!(
        "{} bytes -> {} chars ({:.4} of base64)",
        data.len(),
        stream.len(),
        stream.len() as f64 / (4 * data.len()).div_ceil(3) as f64
    );
}
