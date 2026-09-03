// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The encoder (§9): fixed blocks, two forms, no state.
//!
//! The input is cut into blocks of [`BLOCK_BYTES`]. A block whose every byte
//! the profile admits is written raw, after `~~`; any other block is written
//! as base64. That is the whole encoder, and it is one question per block.
//!
//! Before it starts, it asks that question of the first [`SAMPLE_BLOCKS`]
//! blocks (§9.6). If none of them can be raw, the whole stream is base64url
//! and the question is not asked again -- which is what makes the encoding
//! cost exactly base64's time, byte for byte, on input where it would gain
//! nothing.
//!
//! A third form was considered and dropped: a mask block keeping the admitted
//! bytes of a mixed block in the clear, described in full in
//! `docs/history/spec-v0.4-maske.de.md`. It costs three times base64's time on
//! the blocks it applies to, for readability of text the format is not really
//! for, and the format's whole case is that it costs nothing to choose. `~`
//! followed by an alphabet character is reserved so a later version can bring
//! it back without an old decoder reading it wrongly.

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

/// Blocks the encoder looks at before deciding whether to look at any (§9.6).
///
/// Sixty-four blocks are 3072 bytes, and that number is chosen twice over.
/// It is the knee of the measurement (`binary2textbench`, `--example
/// sample`): at thirty-two blocks `xml` under profile T is mis-sampled and
/// gives up 9.8 points on five megabytes, at sixty-four it is not, and above
/// it almost nothing moves while fewer streams get the cheap path. And it is
/// longer than every value §0.1 names -- a URL query, a cookie, a header, a
/// cache key -- so for those the sample is not a sample at all but the whole
/// input, and it can give up nothing.
pub const SAMPLE_BLOCKS: usize = 64;

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

/// Which form a block of `m` bytes takes, given whether the profile admits
/// all of them, and how many characters it costs (§9.0).
///
/// Raw where every byte is admitted and raw is no longer than base64, which
/// is every length from four up; base64 otherwise. On the tie at four, five
/// and six bytes raw wins, because a tie costs nothing and text in the clear
/// is what the format is for.
pub fn choose(m: usize, admits_all: bool) -> (Form, usize) {
    debug_assert!(m <= BLOCK_BYTES);
    if admits_all && m + 2 <= base64_len(m) {
        (Form::Raw, m + 2)
    } else {
        (Form::Base64, base64_len(m))
    }
}

/// §9.6: can any of the first [`SAMPLE_BLOCKS`] blocks stand raw?
///
/// A sample of the encoder's own decision, not of something that correlates
/// with it. Where it says no, the stream is written as base64url and no block
/// is asked about again; where it says yes, the stream already holds a block
/// that is shorter than base64, so the asking is paid for.
///
/// It answers with the input and nothing else, so two encoders agree (§9.0).
pub fn any_block_can_be_raw(data: &[u8], profile: Profile) -> bool {
    data.chunks(BLOCK_BYTES)
        .take(SAMPLE_BLOCKS)
        .any(|b| choose(b.len(), profile.admits_all(b)).0 == Form::Raw)
}

/// Encode `data` in `profile`, appending to `out`.
pub fn encode_into(data: &[u8], profile: Profile, out: &mut Vec<u8>) {
    out.reserve(base64_len(data.len()));
    // An input that fits inside the sample needs no sample: asking the first
    // sixty-four blocks and then asking every block would ask the same blocks
    // twice. Where the sample would say no, every block is base64 and the
    // loop below writes exactly base64url anyway, so the output is the same
    // either way -- which is what `the_sample_is_free_on_a_short_value`
    // checks. This is the case §0.1 is about, and paying for the sample
    // there would be paying for a decision that is already made.
    if data.len() > SAMPLE_BLOCKS * BLOCK_BYTES && !any_block_can_be_raw(data, profile) {
        emit_base64(data, out);
        return;
    }
    // Consecutive base64 blocks are written as one run. They tile (§4), so
    // the bytes are the same either way; what changes is that binary input
    // goes through the base64 writer's inner loop once rather than once per
    // forty-eight bytes, which is the difference between base64's speed and
    // three quarters of it.
    let mut pending = 0..0;
    for (k, block) in data.chunks(BLOCK_BYTES).enumerate() {
        let start = k * BLOCK_BYTES;
        let form = choose(block.len(), profile.admits_all(block)).0;
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

    /// §9.6: the sample decides for the whole stream, and where it decides
    /// "no" the output is base64url byte for byte -- which is the whole point
    /// of it, because that is also base64url's time.
    #[test]
    fn a_stream_the_sample_turns_down_is_base64url() {
        // Longer than the sample, and no block of it can be raw.
        let n = (SAMPLE_BLOCKS + 40) * BLOCK_BYTES;
        let prose: Vec<u8> = (0..n)
            .map(|i| if i % 6 == 5 { b' ' } else { b'a' })
            .collect();
        let mut out = Vec::new();
        encode_into(&prose, Profile::U, &mut out);
        let mut b64 = Vec::new();
        emit_base64(&prose, &mut b64);
        assert_eq!(out, b64);
    }

    /// And where it decides "yes", every block is asked about again -- so a
    /// raw block past the sample is still written raw.
    #[test]
    fn the_sample_only_turns_the_asking_off() {
        let mut data = vec![b'a'; BLOCK_BYTES];
        data.extend(vec![b' '; (SAMPLE_BLOCKS + 10) * BLOCK_BYTES]);
        data.extend(vec![b'b'; BLOCK_BYTES]);
        let mut out = Vec::new();
        encode_into(&data, Profile::U, &mut out);
        assert!(out.starts_with(b"~~aaa"));
        assert!(out.ends_with(&[b"~~".as_slice(), &[b'b'; BLOCK_BYTES]].concat()));
    }

    /// The sample is skipped where the input fits inside it, and that may not
    /// move a byte: where it would have said no, every block is base64 and
    /// the block loop writes base64url by itself.
    #[test]
    fn the_sample_is_free_on_a_short_value() {
        let limit = SAMPLE_BLOCKS * BLOCK_BYTES;
        let mut r: u32 = 0x51ce_1234;
        let mut next = move || {
            r ^= r << 13;
            r ^= r >> 17;
            r ^= r << 5;
            r as usize
        };
        for _ in 0..2000 {
            let n = 1 + next() % (limit + 200);
            let data: Vec<u8> = (0..n)
                .map(|_| b"aabbcc  ..--\x00\xff"[next() % 14])
                .collect();
            for p in [Profile::U, Profile::T] {
                let mut out = Vec::new();
                encode_into(&data, p, &mut out);
                // What the encoder would write if it always sampled first.
                let want = if any_block_can_be_raw(&data, p) {
                    let mut v = Vec::new();
                    for b in data.chunks(BLOCK_BYTES) {
                        match choose(b.len(), p.admits_all(b)).0 {
                            Form::Raw => {
                                v.extend_from_slice(b"~~");
                                v.extend_from_slice(b);
                            }
                            Form::Base64 => emit_base64(b, &mut v),
                        }
                    }
                    v
                } else {
                    let mut v = Vec::new();
                    emit_base64(&data, &mut v);
                    v
                };
                assert_eq!(out, want, "{p:?}, {n} bytes");
            }
        }
    }

    /// Every input the format is actually for is shorter than the sample, so
    /// for it the sample is the whole input and gives up nothing.
    #[test]
    fn a_short_value_is_never_sampled_wrongly() {
        for n in 1..=SAMPLE_BLOCKS * BLOCK_BYTES {
            if n % 97 != 0 && n != SAMPLE_BLOCKS * BLOCK_BYTES {
                continue;
            }
            for tail in *b"a " {
                let mut data = vec![b'a'; n];
                *data.last_mut().unwrap() = tail;
                let mut out = Vec::new();
                encode_into(&data, Profile::U, &mut out);
                // What the encoder writes when it always asks.
                let want: usize = data
                    .chunks(BLOCK_BYTES)
                    .map(|b| choose(b.len(), Profile::U.admits_all(b)).1)
                    .sum();
                assert_eq!(out.len(), want, "n = {n}, tail {tail:?}");
            }
        }
    }

    /// §9.1: a full block is raw at 50 when every byte is admitted, and base64
    /// at 64 the moment one is not -- wherever that one byte sits, which is
    /// what `admits_all`'s early exit could get wrong.
    #[test]
    fn a_full_block_is_all_or_nothing() {
        let clean = [b'a'; 48];
        assert!(Profile::U.admits_all(&clean));
        assert_eq!(choose(48, true), (Form::Raw, 50));
        for missing in 0..48 {
            let mut block = clean;
            block[missing] = b' ';
            assert!(!Profile::U.admits_all(&block), "byte {missing}");
            block[missing] = 0x80;
            assert!(!Profile::U.admits_all(&block), "byte {missing}, high bit");
        }
        assert_eq!(choose(48, false), (Form::Base64, 64));
    }

    /// Short tails: raw needs four bytes to pay for its two marker characters,
    /// and at four, five and six it ties with base64 and takes the tie.
    #[test]
    fn short_tails() {
        for m in 1..=3 {
            assert_eq!(choose(m, true), (Form::Base64, base64_len(m)), "{m}");
        }
        for m in 4..=48 {
            assert_eq!(choose(m, true), (Form::Raw, m + 2), "{m}");
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
                    .map(|b| choose(b.len(), Profile::U.admits_all(b)).1)
                    .sum();
                assert_eq!(out.len(), want, "n = {n}, bad at {bad_at:?}");
                assert!(out.len() <= base64_len(n));
            }
        }
    }
}
