// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Time against this crate's own base64url, as §13.3 reports it.
//!
//!     cargo run --release --example timing -- <path> [<path> ...]
//!
//! Not a conformance tool: it exists so that a change meant to be faster can
//! be shown to be faster, on the same data the benchmark uses.
//!
//! **Median of paired ratios**, not a ratio of bests. The two are timed
//! alternately within one round and the round yields one ratio, so a runner
//! that drifts between rounds moves both numbers together and cancels; over
//! 21 rounds the median then ignores the rounds where something else ran.
//! Comparing two separately-taken bests would report the drift.

use base66::*;
use std::time::Instant;

/// One round: both sides timed back to back, returning their ratio.
fn round<A, B>(bytes: usize, mut ours: impl FnMut() -> A, mut theirs: impl FnMut() -> B) -> f64 {
    let reps = (16 << 20) / bytes.max(1) + 1;
    let t = Instant::now();
    for _ in 0..reps {
        std::hint::black_box(ours());
    }
    let a = t.elapsed().as_secs_f64();
    let t = Instant::now();
    for _ in 0..reps {
        std::hint::black_box(theirs());
    }
    let b = t.elapsed().as_secs_f64();
    a / b
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

const ROUNDS: usize = 21;

fn main() {
    println!("| file | bytes | size | encode, time | decode, time |");
    println!("|---|--:|--:|--:|--:|");
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).expect("read");
        let ours = encode(&data);
        let base64 = encode_base64url(&data);

        // Warm the allocator and the caches before any round is timed.
        for _ in 0..3 {
            std::hint::black_box(encode(&data));
            std::hint::black_box(encode_base64url(&data));
            std::hint::black_box(decode(&ours).unwrap());
            std::hint::black_box(decode(&base64).unwrap());
        }
        let enc: Vec<f64> = (0..ROUNDS)
            .map(|_| round(data.len(), || encode(&data), || encode_base64url(&data)))
            .collect();
        let dec: Vec<f64> = (0..ROUNDS)
            .map(|_| {
                round(
                    data.len(),
                    || decode(&ours).unwrap(),
                    || decode(&base64).unwrap(),
                )
            })
            .collect();
        println!(
            "| `{}` | {} | {:.1} % | {:.0} % | {:.0} % |",
            path.rsplit('/').next().unwrap(),
            data.len(),
            100.0 * ours.len() as f64 / base64.len() as f64,
            100.0 * median(enc),
            100.0 * median(dec)
        );
    }
    println!();
    println!("Size is len(base66)/len(base64url); time is t(base66)/t(base64url) of");
    println!("this same crate. Less is better in both. Median of {ROUNDS} paired rounds.");
}
