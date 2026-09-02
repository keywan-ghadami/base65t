// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Throughput of `dense` against plain base64, on one corpus file.
//!
//!     cargo run --release --example timing -- <path> [<path> ...]
//!
//! Not a conformance tool: it exists so that a change meant to be faster can
//! be shown to be faster, on the same data the benchmark uses.

use base65t::*;
use std::time::Instant;

fn bench<T>(label: &str, bytes: usize, mut f: impl FnMut() -> T) {
    // One untimed pass to warm the allocator and the caches, then enough
    // repetitions that the clock's resolution is not the measurement.
    let _ = f();
    let reps = (64 << 20) / bytes.max(1) + 1;
    let t = Instant::now();
    for _ in 0..reps {
        std::hint::black_box(f());
    }
    let secs = t.elapsed().as_secs_f64();
    let mbs = (bytes * reps) as f64 / secs / (1 << 20) as f64;
    println!("  {label:<18} {mbs:8.1} MiB/s");
}

fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).expect("read");
        let dense = encode(&data);
        let opaque = encode_opaque(&data);
        println!(
            "{path}  {} bytes, dense {:.1} % of base64",
            data.len(),
            100.0 * dense.len() as f64 / opaque.len() as f64
        );
        bench("encode dense", data.len(), || encode(&data));
        bench("encode opaque", data.len(), || encode_opaque(&data));
        bench("decode dense", data.len(), || {
            decode(&dense, Profile::U).unwrap()
        });
        bench("decode opaque", data.len(), || {
            decode(&opaque, Profile::U).unwrap()
        });
    }
}
