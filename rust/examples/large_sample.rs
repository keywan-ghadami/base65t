// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Writes one input larger than a block and its `dense` encoding, so that the
//! other implementation can check the seam.
//!
//!     cargo run --release --example large_sample -- /tmp/in.bin /tmp/in.b65
//!
//! `docs/vectors.json` cannot reach here: every vector in it is under a
//! kilobyte, and a hex dump of a quarter-megabyte input would be half a
//! megabyte of repository. But this is exactly where the interesting mistake
//! lives -- a block whose last base64 run leaves a partial quantum is
//! continued by the next block's run, and the seam decodes to what neither
//! block meant. So the check that crosses implementations here is decoding,
//! which is cheap in any language, rather than re-deriving the segmentation.
//!
//! The input is mixed on purpose. Homogeneous input cannot show the bug:
//! noise makes one base64 run per block of exactly `BLOCK_BYTES` bytes, and
//! profile-legal text makes no base64 run at all.

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
    let mut data: Vec<u8> = Vec::with_capacity(4 * BLOCK_BYTES);
    while data.len() < 4 * BLOCK_BYTES + 777 {
        let run = 1 + (next() % 60) as usize;
        if next() % 3 == 0 {
            data.extend((0..run).map(|_| (next() & 0xff) as u8));
        } else {
            data.extend((0..run).map(|i| b"abcdefghij"[i % 10]));
        }
    }

    let stream = encode_dense(&data, Profile::U);
    assert_eq!(
        decode(&stream, Profile::U).expect("its own output").bytes,
        data
    );
    std::fs::write(input_path, &data).expect("write input");
    std::fs::write(stream_path, &stream).expect("write stream");
    println!(
        "{} bytes over {} blocks -> {} chars ({:.4} of base64)",
        data.len(),
        data.len().div_ceil(BLOCK_BYTES),
        stream.len(),
        stream.len() as f64 / (4 * data.len()).div_ceil(3) as f64
    );
}
