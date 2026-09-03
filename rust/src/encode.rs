// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The encoder (§9): fixed blocks, two forms, no state.
//!
//! The input is cut into blocks of [`BLOCK_BYTES`]. A block whose every byte
//! the profile admits is written raw, after `~~`; any other block is written
//! as base64. That is the whole encoder, and it is one comparison per block
//! over a mask the profile computes sixty-four bytes at a time.
//!
//! There was a third form for a day -- a mask block that kept the admitted
//! bytes of a mixed block in the clear -- and it is in `docs/history/`. It
//! cost three times base64's time on the blocks it applied to, for
//! readability of text the format is not really for, and the format's whole
//! case is that it costs nothing to choose. `~` followed by an alphabet
//! character is reserved so a later version can bring it back without an
//! old decoder reading it wrongly.

use crate::alphabet::{Profile, ALPHABET, TILDE};

/// Input bytes per block (§4).
///
/// Forty-eight and not some other number: a multiple of three, so a base64
/// block is a whole number of quanta and blocks tile without a seam; and
/// large enough that the two characters a raw block spends on its marker are
/// four per cent of it rather than a third -- the same block at six bytes
/// would gain nothing at all (§9.1). A multiple of six as well, which the
/// reserved mask form of §17 would need.
pub const BLOCK_BYTES: usize = 48;

/// Characters a full base64 block occupies.
pub const BASE64_BLOCK_CHARS: usize = BLOCK_BYTES / 3 * 4;

/// Base64 length of `n` bytes, unpadded.
#[inline]
pub(crate) fn base64_len(n: usize) -> usize {
    (4 * n).div_ceil(3)
}

/// The two forms a block can take (§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Base64,
    Raw,
}

/// Which form a block of `m` bytes takes, given which of them the profile
/// admits, and how many characters it costs (§9.0).
///
/// Raw where every byte is admitted and raw is no longer than base64, which
/// is every length from four up; base64 otherwise. On the tie at four, five
/// and six bytes raw wins, because a tie costs nothing and text in the clear
/// is what the format is for.
pub fn choose(m: usize, mask: u64) -> (Form, usize) {
    debug_assert!(m <= BLOCK_BYTES);
    let all = if m == 64 { u64::MAX } else { (1u64 << m) - 1 };
    if mask == all && m + 2 <= base64_len(m) {
        (Form::Raw, m + 2)
    } else {
        (Form::Base64, base64_len(m))
    }
}

/// Encode `data` in `profile`, appending to `out`.
pub fn encode_into(data: &[u8], profile: Profile, out: &mut Vec<u8>) {
    out.reserve(base64_len(data.len()));
    // Consecutive base64 blocks are written as one run. They tile (§4), so
    // the bytes are the same either way; what changes is that binary input
    // goes through the base64 writer's inner loop once rather than once per
    // forty-eight bytes, which is the difference between base64's speed and
    // three quarters of it.
    let mut pending = 0..0;
    for (k, block) in data.chunks(BLOCK_BYTES).enumerate() {
        let start = k * BLOCK_BYTES;
        let mask = profile.mask_short(block);
        let form = choose(block.len(), mask).0;
        if form == Form::Base64 {
            if pending.end == start {
                pending.end = start + block.len();
            } else {
                pending = start..start + block.len();
            }
            continue;
        }
        if !pending.is_empty() {
            emit_base64(&data[pending.clone()], out);
            pending = 0..0;
        }
        out.push(TILDE);
        out.push(TILDE);
        out.extend_from_slice(block);
    }
    if !pending.is_empty() {
        emit_base64(&data[pending], out);
    }
}

/// Base64URL of `bytes`, unpadded (§5.1).
pub(crate) fn emit_base64(bytes: &[u8], out: &mut Vec<u8>) {
    let (groups, remainder) = bytes.as_chunks::<3>();
    for c in groups {
        let n = (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
        out.extend_from_slice(&[
            ALPHABET[(n >> 18) as usize & 63],
            ALPHABET[(n >> 12) as usize & 63],
            ALPHABET[(n >> 6) as usize & 63],
            ALPHABET[n as usize & 63],
        ]);
    }
    match remainder {
        [a] => {
            let n = (*a as u32) << 16;
            out.push(ALPHABET[(n >> 18) as usize & 63]);
            out.push(ALPHABET[(n >> 12) as usize & 63]);
        }
        [a, b] => {
            let n = (*a as u32) << 16 | (*b as u32) << 8;
            out.push(ALPHABET[(n >> 18) as usize & 63]);
            out.push(ALPHABET[(n >> 12) as usize & 63]);
            out.push(ALPHABET[(n >> 6) as usize & 63]);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §9.1: a full block is raw at 50 when every byte is admitted, and base64
    /// at 64 the moment one is not.
    #[test]
    fn a_full_block_is_all_or_nothing() {
        assert_eq!(choose(48, (1u64 << 48) - 1), (Form::Raw, 50));
        for missing in 0..48 {
            let mask = ((1u64 << 48) - 1) & !(1 << missing);
            assert_eq!(choose(48, mask), (Form::Base64, 64), "byte {missing}");
        }
        assert_eq!(choose(48, 0), (Form::Base64, 64));
    }

    /// Short tails: raw needs four bytes to pay for its two marker characters,
    /// and at four, five and six it ties with base64 and takes the tie.
    #[test]
    fn short_tails() {
        for m in 1..=3 {
            assert_eq!(
                choose(m, (1 << m) - 1),
                (Form::Base64, base64_len(m)),
                "{m}"
            );
        }
        for m in 4..=48 {
            assert_eq!(choose(m, (1 << m) - 1), (Form::Raw, m + 2), "{m}");
        }
    }

    /// What the encoder writes is what `choose` names, block by block.
    #[test]
    fn the_encoder_follows_choose() {
        for n in 1..=100usize {
            for bad_at in [None, Some(0), Some(n / 2), Some(n - 1)] {
                let mut data = vec![b'a'; n];
                if let Some(i) = bad_at {
                    data[i] = b' ';
                }
                let mut out = Vec::new();
                encode_into(&data, Profile::U, &mut out);
                let want: usize = data
                    .chunks(BLOCK_BYTES)
                    .map(|b| choose(b.len(), Profile::U.mask_short(b)).1)
                    .sum();
                assert_eq!(out.len(), want, "n = {n}, bad at {bad_at:?}");
                assert!(out.len() <= base64_len(n));
            }
        }
    }
}
