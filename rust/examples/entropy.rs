// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Prints the §9.6 figure for the input on stdin, so the second implementation
//! can be held against it (conformance point 4 of §16).
//!
//!     cargo run --release --example entropy < file
//!
//! Two implementations that disagree here write different bytes for the same
//! input, and no test vector under 4096 bytes would notice.

use base65t::*;
use std::io::Read;

fn main() {
    let mut data = Vec::new();
    std::io::stdin().read_to_end(&mut data).expect("stdin");
    println!("{:?} {}", classify(&data), data.len());
}
