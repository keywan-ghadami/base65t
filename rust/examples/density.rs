// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! §12's table, measured rather than estimated.
//!
//! Two of its four rows are marked exact in the specification and two are
//! marked `[OFFEN: geschätzt]`. This fills the estimates in on generated
//! inputs of a stated shape — which is not the same as a corpus, and says so:
//! the number a mixed row gets depends entirely on how the mixing is done, and
//! that is exactly why §16.5 asks for binary2textbench rather than for this.
//!
//!     cargo run --release --example density

use base65t::*;

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

const N: usize = 1 << 20;

/// Text bytes that every profile admits, in roughly the proportions of a
/// lowercase identifier stream.
fn text_byte(r: &mut Rng) -> u8 {
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~"[r.next() as usize % 66]
}

fn mixed(r: &mut Rng, binary_percent: usize) -> Vec<u8> {
    // Runs rather than single bytes: a payload that alternates every byte has
    // no literal runs to find, and no real one looks like that.
    let mut v = Vec::with_capacity(N);
    while v.len() < N {
        let run = 8 + r.next() as usize % 120;
        if r.next() as usize % 100 < binary_percent {
            v.extend((0..run).map(|_| (r.next() & 0xff) as u8));
        } else {
            v.extend((0..run).map(|_| text_byte(r)));
        }
    }
    v.truncate(N);
    v
}

fn ratio(data: &[u8], out: usize) -> f64 {
    out as f64 / data.len() as f64
}

fn main() {
    let mut r = Rng(0x13579bdf);
    let binary: Vec<u8> = (0..N).map(|_| (r.next() & 0xff) as u8).collect();
    let text: Vec<u8> = (0..N).map(|_| text_byte(&mut r)).collect();
    let mix30 = mixed(&mut r, 30);
    let mix70 = mixed(&mut r, 70);

    println!(
        "Base65t density, {} KiB per input, dense preset\n",
        N / 1024
    );
    println!("| input | base64 | profile U | profile T | profile B |");
    println!("|---|---|---|---|---|");
    for (name, data) in [
        ("pure binary", &binary),
        ("pure profile-legal text", &text),
        ("70 % text / 30 % binary", &mix30),
        ("30 % text / 70 % binary", &mix70),
    ] {
        let u = encode_dense(data, Profile::U).len();
        let t = encode_dense(data, Profile::T).len();
        let b = encode_dense(data, Profile::B).len();
        println!(
            "| {name} | {:.3} | {:.3} | {:.3} | {:.3} |",
            ratio(data, (4 * data.len()).div_ceil(3)),
            ratio(data, u),
            ratio(data, t),
            ratio(data, b),
        );
    }

    println!("\nThe exact figures §12 states, checked:");
    let full = vec![b'a'; MAX_LITERAL];
    println!(
        "  one full literal segment: {}/{} = {:.5}",
        encode_dense(&full, Profile::U).len(),
        MAX_LITERAL,
        ratio(&full, encode_dense(&full, Profile::U).len())
    );
    println!(
        "  high-entropy binary:      {:.5} (base64 is {:.5})",
        ratio(&binary, encode_dense(&binary, Profile::U).len()),
        4.0 / 3.0
    );

    println!("\nPresets on the mixed input (70 % text):");
    for (name, out) in [
        ("dense", encode_dense(&mix30, Profile::U)),
        ("legible", encode_legible(&mix30, Profile::U)),
        ("canonical", encode_canonical(&mix30, Profile::U)),
        ("opaque", encode_opaque(&mix30)),
        ("framed", encode_framed(&mix30, Profile::U)),
    ] {
        println!("  {name:<10} {:.4}", ratio(&mix30, out.len()));
    }
}
