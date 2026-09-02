// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! What the exact programme of §9.2 costs against the linear rule of §9.2.1.
//!
//!     cargo run --release --example dp_cost
//!
//! Both are exact rules and both are deterministic; what separates them is a
//! fraction of a percent of size against a factor in time, and neither number
//! is guessable. This prints both, by input size, because the answer changes
//! with it: the programme earns most on short values and almost nothing on
//! long ones, while its cost goes the other way.

use base65t::*;
use std::time::Instant;

fn bench(n: usize, mut f: impl FnMut()) -> f64 {
    let mut best = 0.0f64;
    for _ in 0..3 {
        let reps = (24 << 20) / n.max(1) + 3;
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
    let mut s: u32 = 0x51ce_7777;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as usize
    };
    println!("| bytes | linear | exact DP | factor | exact is smaller by |");
    println!("|---|--:|--:|--:|--:|");
    for n in [64usize, 256, 1024, 4096, 16_384, 65_536, 262_144, 1 << 20] {
        let d: Vec<u8> = (0..n)
            .map(|_| {
                if next() % 3 == 0 {
                    (next() & 0xff) as u8
                } else {
                    b"abcdefghij-._~ "[next() % 15]
                }
            })
            .collect();
        let a = bench(n, || {
            std::hint::black_box(encode_dense(&d, Profile::U));
        });
        let b = bench(n, || {
            std::hint::black_box(encode_canonical(&d, Profile::U));
        });
        let (la, lb) = (
            encode_dense(&d, Profile::U).len(),
            encode_canonical(&d, Profile::U).len(),
        );
        println!(
            "| {n} | {a:.0} MiB/s | {b:.0} MiB/s | **{:.0}x** | {:.2} % |",
            a / b,
            100.0 * (la as f64 / lb as f64 - 1.0)
        );
    }
}
