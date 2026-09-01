// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The measurement PREREGISTRATION.md describes: what a readability bonus buys
//! and what it costs.
//!
//!     cargo run --release --example sweetspot -- --profile=U <file>...
//!     cargo run --release --example sweetspot -- --profile=U --axis
//!
//! `--axis` replaces the files with generated inputs that vary the one thing
//! the binary2textbench corpus does not: the share of bytes the *profile*
//! rejects. For profile U that is the space, not the control characters the
//! corpus's density classes inject.
//!
//! Everything reported is exact — encoded length, passthrough share, segments
//! per kB — so there is nothing to average over repetitions and nothing a
//! second machine could disagree about. Throughput is deliberately absent:
//! §9.5 ties it to the segment rate, which is here, and a noisy figure would
//! say the same thing worse.

use base65t::internals::{costs, emit, segment_with, LiteralEnd, Rules, Seg};
use base65t::Profile;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Knob {
    bonus: i64,
    prefer_passthrough: bool,
    end: LiteralEnd,
}

impl Knob {
    fn label(&self) -> String {
        let rule = if self.prefer_passthrough {
            "MaxPass"
        } else {
            match self.end {
                LiteralEnd::KeyOrder => "KeyOrder",
                LiteralEnd::Longest => "Longest",
            }
        };
        format!("λ={} {rule}", self.bonus)
    }
    fn rules(&self, profile: Profile, min_literal: usize) -> Rules {
        let mut r = Rules::preset(profile, Some(min_literal), false);
        r.bonus = self.bonus;
        r.prefer_passthrough = self.prefer_passthrough;
        r
    }
}

#[derive(Default, Clone)]
struct Tally {
    bytes: usize,
    coded: usize,
    passthrough: usize,
    segments: usize,
    /// Files whose encoding is longer than base64 would be — §9.4 is a
    /// per-file guarantee, so one violation is a violation.
    over_base64: usize,
    worst_ratio: f64,
    files: usize,
}

fn measure(data: &[u8], knob: Knob, profile: Profile, min_literal: usize, t: &mut Tally) {
    let rules = knob.rules(profile, min_literal);
    let c = costs(data, rules);
    let segs = segment_with(data, rules, &c, knob.end);
    let out = emit(data, &segs);
    let pass: usize = segs
        .iter()
        .map(|s| match *s {
            Seg::Literal(i, j) => j - i,
            Seg::Base64(_, _) => 0,
        })
        .sum();
    let base64 = (4 * data.len()).div_ceil(3);
    let ratio = out.len() as f64 / base64.max(1) as f64;
    t.bytes += data.len();
    t.coded += out.len();
    t.passthrough += pass;
    t.segments += segs.len();
    t.files += 1;
    if out.len() > base64 {
        t.over_base64 += 1;
    }
    t.worst_ratio = t.worst_ratio.max(ratio);
}

/// Inputs that vary the share of bytes the profile rejects, in runs rather
/// than singly — a payload that alternates every byte has no literal runs to
/// find and no real one looks like that.
fn axis(profile: Profile) -> Vec<(String, Vec<u8>)> {
    let mut s: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let legal: Vec<u8> = (0..=255u8)
        .filter(|&b| profile.allows(b) && b != b'~')
        .collect();
    let illegal: Vec<u8> = (0..=255u8).filter(|&b| !profile.allows(b)).collect();
    let mut out = Vec::new();
    for share in [0usize, 1, 2, 5, 10, 20, 35, 50, 75, 100] {
        for run in [4usize, 16, 64, 256] {
            let mut v = Vec::with_capacity(1 << 16);
            while v.len() < (1 << 16) {
                let bad = (next() % 100) < share as u64;
                let len = 1 + next() as usize % (2 * run);
                for _ in 0..len {
                    let pool = if bad { &illegal } else { &legal };
                    v.push(pool[next() as usize % pool.len()]);
                }
            }
            v.truncate(1 << 16);
            out.push((format!("{share:>3} % illegal, runs ~{run}"), v));
        }
    }
    out
}

fn main() {
    let mut profile = Profile::U;
    let mut min_literal = 1usize;
    let mut use_axis = false;
    let mut per_file = false;
    let mut paths: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--axis" => use_axis = true,
            "--per-file" => per_file = true,
            a if a.starts_with("--profile=") => {
                profile = match &a[10..] {
                    "U" => Profile::U,
                    "T" => Profile::T,
                    "B" => Profile::B,
                    _ => panic!("profile is U, T or B"),
                }
            }
            a if a.starts_with("--lmin=") => min_literal = a[7..].parse().expect("a number"),
            a => paths.push(a.to_string()),
        }
    }

    let inputs: Vec<(String, Vec<u8>)> = if use_axis {
        axis(profile)
    } else {
        paths
            .iter()
            .filter_map(|p| {
                std::fs::read(p)
                    .ok()
                    .filter(|d| !d.is_empty())
                    .map(|d| (p.rsplit('/').next().unwrap_or(p).to_string(), d))
            })
            .collect()
    };
    assert!(!inputs.is_empty(), "no input");

    let mut knobs = vec![
        Knob {
            bonus: 0,
            prefer_passthrough: false,
            end: LiteralEnd::KeyOrder,
        },
        Knob {
            bonus: 0,
            prefer_passthrough: false,
            end: LiteralEnd::Longest,
        },
        Knob {
            bonus: 0,
            prefer_passthrough: true,
            end: LiteralEnd::KeyOrder,
        },
    ];
    for bonus in 1..=4 {
        knobs.push(Knob {
            bonus,
            prefer_passthrough: true,
            end: LiteralEnd::KeyOrder,
        });
    }

    println!(
        "profile {profile:?}, L_min {min_literal}, {} inputs, {} bytes\n",
        inputs.len(),
        inputs.iter().map(|(_, d)| d.len()).sum::<usize>()
    );
    println!(
        "| knob | P: passthrough | S: size vs base64 | worst file | over base64 | G: segments/kB |"
    );
    println!("|---|---|---|---|---|---|");

    let mut rows: Vec<(Knob, Tally)> = Vec::new();
    for knob in &knobs {
        let mut t = Tally::default();
        for (_, data) in &inputs {
            measure(data, *knob, profile, min_literal, &mut t);
        }
        let base64: usize = inputs.iter().map(|(_, d)| (4 * d.len()).div_ceil(3)).sum();
        println!(
            "| {} | {:.2} % | {:.1} % | {:.1} % | {} of {} | {:.1} |",
            knob.label(),
            100.0 * t.passthrough as f64 / t.bytes as f64,
            100.0 * t.coded as f64 / base64 as f64,
            100.0 * t.worst_ratio,
            t.over_base64,
            t.files,
            t.segments as f64 / (t.bytes as f64 / 1000.0),
        );
        rows.push((*knob, t));
    }

    if per_file {
        println!("\nPer input, passthrough share — the sign, not the mean\n");
        print!("| input |");
        for k in &knobs {
            print!(" {} |", k.label());
        }
        println!();
        print!("|---|");
        for _ in &knobs {
            print!("---|");
        }
        println!();
        for (name, data) in &inputs {
            if data.len() < 4096 {
                continue;
            }
            print!("| {name} |");
            for knob in &knobs {
                let mut t = Tally::default();
                measure(data, *knob, profile, min_literal, &mut t);
                print!(" {:.1} |", 100.0 * t.passthrough as f64 / t.bytes as f64);
            }
            println!();
        }
    }
}
