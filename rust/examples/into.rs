// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! What the caller's own buffer is worth, by size of value.
//!
//!     cargo run --release --example into
//!
//! The allocation an owning call makes is a fixed cost, so it matters exactly
//! where the values are small — which is where §0.1 says they are.

use base65t::*;
use std::time::Instant;

fn bench(n: usize, mut f: impl FnMut()) -> f64 {
    let mut best = 0.0f64;
    for _ in 0..5 {
        let reps = 4_000_000 / n.max(1) + 5000;
        f();
        let t = Instant::now();
        for _ in 0..reps {
            f();
        }
        let r = (n * reps) as f64 / t.elapsed().as_secs_f64() / (1 << 20) as f64;
        if r > best {
            best = r;
        }
    }
    best
}

fn main() {
    let mut s: u32 = 0x51ce_0042;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    println!("| bytes | encode | encode_into | gain | decode | decode_into | gain |");
    println!("|---|--:|--:|--:|--:|--:|--:|");
    for n in [8usize, 16, 32, 64, 155, 512, 4096, 65_536] {
        let data: Vec<u8> = (0..n).map(|_| (next() & 0xff) as u8).collect();
        let stream = encode(&data);
        let mut buf = Vec::with_capacity(n * 2);

        let e0 = bench(n, || {
            std::hint::black_box(encode(&data));
        });
        let e1 = bench(n, || {
            buf.clear();
            encode_into(&data, &mut buf);
        });
        let d0 = bench(n, || {
            std::hint::black_box(decode(&stream).unwrap());
        });
        let d1 = bench(n, || {
            buf.clear();
            decode_into(&stream, &mut buf).unwrap();
        });
        println!(
            "| {n} | {e0:.0} | {e1:.0} | **{:.2}x** | {d0:.0} | {d1:.0} | **{:.2}x** |",
            e1 / e0,
            d1 / d0
        );
    }
}
