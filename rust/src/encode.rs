// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The encoder (§9): fixed blocks, three forms, no state.
//!
//! The input is cut into blocks of [`BLOCK_BYTES`]. Each block is written in
//! whichever of three forms is shortest, and nothing about one block depends
//! on any other:
//!
//! * **base64** -- exactly `4k/3` alphabet characters. Blocks of this form tile
//!   seamlessly, which is why a plain base64 stream is also a valid stream.
//! * **raw** -- `~~` and the `k` bytes as they are. Only when the profile
//!   admits every byte.
//! * **mask** -- `~`, then [`MASK_CHARS`] characters holding one bit per byte,
//!   then the admitted bytes in order, then base64 of the rest in order.
//!
//! There is no search in here. Where earlier versions of this format found
//! runs and priced them against each other, this asks one question of every
//! block -- how many of its bytes may stand as they are -- and the answer is
//! a popcount over a mask the profile already computes sixty-four bytes at a
//! time. That is the whole of the encoder, and it runs at base64's speed
//! because it does base64's amount of work.

use crate::alphabet::{Profile, ALPHABET, TILDE};

/// Input bytes per block (§4).
///
/// Forty-eight and not some other number, for three reasons that have to
/// hold at once. It is a multiple of three, so a base64 block is a whole
/// number of quanta and blocks tile without a seam. It is a multiple of six,
/// so the mask is a whole number of characters. And it is large enough that
/// the two characters a raw block spends on its marker are four per cent of
/// it rather than a third: the same block at six bytes would gain nothing at
/// all (§9.1).
pub const BLOCK_BYTES: usize = 48;

/// Characters a mask occupies: one bit per byte, six bits per character.
pub const MASK_CHARS: usize = BLOCK_BYTES / 6;

/// Characters a full base64 block occupies.
pub const BASE64_BLOCK_CHARS: usize = BLOCK_BYTES / 3 * 4;

/// Base64 length of `n` bytes, unpadded.
#[inline]
pub(crate) fn base64_len(n: usize) -> usize {
    (4 * n).div_ceil(3)
}

/// The three forms a block can take (§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Base64,
    Raw,
    Mask,
}

/// Which form a block of `m` bytes takes, given which of them the profile
/// admits, and how many characters it costs (§9.0).
///
/// The shortest wins. On a tie the form with more bytes in the clear wins,
/// which is the only preference the format has beyond length: a tie costs
/// nothing, and readability is what the format is for.
///
/// Both the length and the tie-break are functions of `m` and the mask
/// alone, so two encoders agree.
pub fn choose(m: usize, mask: u64) -> (Form, usize) {
    debug_assert!(m <= BLOCK_BYTES);
    let admitted = mask.count_ones() as usize;
    let mut best = (Form::Base64, base64_len(m));
    let masked = 1 + MASK_CHARS + admitted + base64_len(m - admitted);
    if masked <= best.1 {
        best = (Form::Mask, masked);
    }
    if admitted == m && m + 2 <= best.1 {
        best = (Form::Raw, m + 2);
    }
    best
}

/// Encode `data` in `profile`, appending to `out`.
pub fn encode_into(data: &[u8], profile: Profile, out: &mut Vec<u8>) {
    out.reserve(base64_len(data.len()));
    for block in data.chunks(BLOCK_BYTES) {
        let mask = profile.mask_short(block);
        match choose(block.len(), mask).0 {
            Form::Base64 => emit_base64(block, out),
            Form::Raw => {
                out.push(TILDE);
                out.push(TILDE);
                out.extend_from_slice(block);
            }
            Form::Mask => emit_mask_block(block, mask, out),
        }
    }
}

/// The mask form: marker, mask, admitted bytes, base64 of the rest (§6).
fn emit_mask_block(block: &[u8], mask: u64, out: &mut Vec<u8>) {
    out.push(TILDE);
    emit_mask(mask, out);
    for (i, &b) in block.iter().enumerate() {
        if mask >> i & 1 == 1 {
            out.push(b);
        }
    }
    // The bytes the profile rejects, in order, through a small buffer so the
    // base64 writer sees them as one run.
    let mut rest = [0u8; BLOCK_BYTES];
    let mut n = 0;
    for (i, &b) in block.iter().enumerate() {
        if mask >> i & 1 == 0 {
            rest[n] = b;
            n += 1;
        }
    }
    emit_base64(&rest[..n], out);
}

/// The mask as [`MASK_CHARS`] characters (§6.1).
///
/// Character `j` carries bytes `6j` to `6j + 5`, first byte in the top bit,
/// so that the mask reads left to right like the bytes it describes.
pub(crate) fn emit_mask(mask: u64, out: &mut Vec<u8>) {
    for j in 0..MASK_CHARS {
        let mut v = 0usize;
        for t in 0..6 {
            v = v << 1 | (mask >> (6 * j + t) & 1) as usize;
        }
        out.push(ALPHABET[v]);
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

    /// §9.1's table: what each form costs on a full block, by how many bytes
    /// are admitted. The mask form pays for itself from 27 bytes up, the raw
    /// form always.
    #[test]
    fn the_cost_table_of_section_9_1() {
        assert_eq!(choose(48, 0), (Form::Base64, 64));
        assert_eq!(choose(48, u64::MAX >> 16), (Form::Raw, 50));
        for admitted in 1..48u32 {
            let mask = (1u64 << admitted) - 1;
            let (form, len) = choose(48, mask);
            let masked = 9 + admitted as usize + base64_len(48 - admitted as usize);
            assert!(len <= 64, "{admitted}: {len}");
            if admitted >= 27 {
                assert_eq!((form, len), (Form::Mask, masked), "{admitted}");
            } else {
                assert_eq!(form, Form::Base64, "{admitted}");
            }
        }
        // 27 is a tie, and the tie goes to the clear text.
        assert_eq!(choose(48, (1 << 27) - 1), (Form::Mask, 64));
    }

    /// Short tails: raw needs four bytes to pay for its two marker
    /// characters, and at exactly four, five and six it ties with base64 and
    /// takes the tie.
    #[test]
    fn short_tails() {
        for m in 1..=3 {
            assert_eq!(choose(m, (1 << m) - 1).0, Form::Base64, "{m}");
        }
        for m in 4..=48 {
            assert_eq!(choose(m, (1 << m) - 1), (Form::Raw, m + 2), "{m}");
        }
    }

    /// The mask is written first byte in the top bit, and reads back the
    /// same way.
    #[test]
    fn mask_characters_read_left_to_right() {
        let mut out = Vec::new();
        emit_mask(1, &mut out); // byte 0 admitted
        assert_eq!(&out, b"gAAAAAAA"); // 100000 = 32 = 'g'
        out.clear();
        emit_mask(1 << 5, &mut out); // byte 5 admitted
        assert_eq!(&out, b"BAAAAAAA");
        out.clear();
        emit_mask(1 << 47, &mut out);
        assert_eq!(&out, b"AAAAAAAB");
    }

    /// Every full block is at most 64 characters and every form the encoder
    /// writes is the one `choose` names -- over every mask, which is not
    /// feasible, so over every popcount at every position pattern a shift
    /// gives, which is the part `choose` actually reads.
    #[test]
    fn never_longer_than_base64_by_block() {
        for admitted in 0..=48 {
            for shift in 0..=(48 - admitted) {
                let mask = if admitted == 0 {
                    0
                } else {
                    ((1u64 << admitted) - 1) << shift
                };
                let (_, len) = choose(48, mask);
                assert!(len <= 64);
                let block: Vec<u8> = (0..48)
                    .map(|i| if mask >> i & 1 == 1 { b'a' } else { b' ' })
                    .collect();
                let mut out = Vec::new();
                encode_into(&block, Profile::U, &mut out);
                assert_eq!(out.len(), len, "admitted {admitted} at {shift}");
            }
        }
    }
}
