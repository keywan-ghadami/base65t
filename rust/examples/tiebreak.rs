// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! How much the two readings of §11.1 differ, on data somebody else chose.
//!
//! The *direction* is settled without data: taking the longest optimal literal
//! never yields fewer passthrough bytes and never more segments, which
//! `the_longest_rule_weakly_dominates_on_both_metrics` checks exhaustively.
//! So this measures the magnitude and nothing else, and it reports it per file
//! rather than as one average — an effect that is present on every file is a
//! property of the rules, and one that appears only in the mean is a property
//! of the mixture.
//!
//!     cargo run --release --example tiebreak -- --profile=U path...
//!
//! Both rules are length-optimal by construction, so the encoded sizes must
//! come out identical; the run asserts it rather than trusting it.

use std::path::Path;

use base65t::internals::{c_vector, costs, segment_with, LiteralEnd, Rules};
use base65t::{encode_canonical, Profile};

fn metrics(c: &str) -> (usize, usize) {
    let passthrough = c.chars().filter(|&x| x != 'B').count();
    let mut segments = 0;
    let mut prev = ' ';
    for x in c.chars() {
        if x == 'S' || (x == 'B' && prev != 'B') {
            segments += 1;
        }
        prev = x;
    }
    (passthrough, segments)
}

fn main() {
    let mut profile = Profile::U;
    let mut min_literal = 1usize;
    let mut paths: Vec<String> = Vec::new();
    for arg in std::env::args().skip(1) {
        if let Some(p) = arg.strip_prefix("--profile=") {
            profile = match p {
                "U" => Profile::U,
                "T" => Profile::T,
                "B" => Profile::B,
                _ => panic!("profile is U, T or B"),
            };
        } else if let Some(l) = arg.strip_prefix("--lmin=") {
            min_literal = l.parse().expect("a number");
        } else {
            paths.push(arg);
        }
    }
    if paths.is_empty() {
        eprintln!("usage: tiebreak [--profile=U|T|B] [--lmin=N] <file>...");
        std::process::exit(2);
    }

    let rules = Rules::preset(profile, Some(min_literal), false);

    println!("profile {profile:?}, L_min {min_literal}\n");
    println!(
        "{:<34} {:>9} {:>10} {:>10} {:>9} {:>9}",
        "file", "bytes", "pass Key", "pass Long", "seg Key", "seg Long"
    );

    let (mut tot_bytes, mut tot_pk, mut tot_pl, mut tot_sk, mut tot_sl) = (0usize, 0, 0, 0, 0);
    let (mut differing, mut files) = (0usize, 0usize);
    for path in &paths {
        let data = match std::fs::read(path) {
            Ok(d) if !d.is_empty() => d,
            _ => continue,
        };
        let c = costs(&data, rules);
        let key = c_vector(&segment_with(&data, rules, &c, LiteralEnd::KeyOrder));
        let longest = c_vector(&segment_with(&data, rules, &c, LiteralEnd::Longest));
        let (pk, sk) = metrics(&key);
        let (pl, sl) = metrics(&longest);

        // Both are length-optimal: whatever else differs, the size does not.
        if min_literal == 1 && profile == Profile::U {
            let canonical = encode_canonical(&data, profile);
            assert_eq!(canonical.len(), data.len() + segment_cost(&key, &data));
        }
        assert_eq!(
            data.len() + segment_cost(&key, &data),
            data.len() + segment_cost(&longest, &data),
            "{path}: the two rules are not the same length"
        );

        files += 1;
        if key != longest {
            differing += 1;
        }
        tot_bytes += data.len();
        tot_pk += pk;
        tot_pl += pl;
        tot_sk += sk;
        tot_sl += sl;

        let name = Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let kb = data.len() as f64 / 1000.0;
        if data.len() >= 4096 {
            println!(
                "{:<34} {:>9} {:>9.2}% {:>9.2}% {:>9.1} {:>9.1}",
                name,
                data.len(),
                100.0 * pk as f64 / data.len() as f64,
                100.0 * pl as f64 / data.len() as f64,
                sk as f64 / kb,
                sl as f64 / kb,
            );
        }
    }

    let pct = |x: usize| 100.0 * x as f64 / tot_bytes as f64;
    let per_kb = |x: usize| x as f64 / (tot_bytes as f64 / 1000.0);
    println!(
        "\n{files} files, {differing} where the rules differ, {tot_bytes} bytes\n\
         passthrough  Key {:.2} %  Longest {:.2} %   (+{:.2} points)\n\
         segments/kB  Key {:.1}    Longest {:.1}     ({:+.1} %)",
        pct(tot_pk),
        pct(tot_pl),
        pct(tot_pl) - pct(tot_pk),
        per_kb(tot_sk),
        per_kb(tot_sl),
        100.0 * (tot_sl as f64 - tot_sk as f64) / tot_sk as f64,
    );
}

/// Characters the segmentation costs beyond the payload bytes: headers for
/// literals, the base64 expansion for the rest.
fn segment_cost(c: &str, _data: &[u8]) -> usize {
    let bytes = c.as_bytes();
    let mut cost = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'B' {
            let mut j = i;
            while j < bytes.len() && bytes[j] == b'B' {
                j += 1;
            }
            let k = j - i;
            cost += (4 * k).div_ceil(3) - k;
            i = j;
        } else {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == b'L' {
                j += 1;
            }
            cost += if j - i <= 62 { 2 } else { 4 };
            i = j;
        }
    }
    cost
}
