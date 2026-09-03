// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The decoder (§10): one block at a time, and it never searches.
//!
//! Every block announces its form in its first character or two, and every
//! form has a length the decoder can compute before it reads a byte of
//! payload: sixty-four characters of base64, `~~` and forty-eight bytes, or
//! `~`, eight mask characters, as many bytes as the mask has bits set, and
//! base64 of the remainder. The last block is the only one that can be
//! shorter, and it is shorter by construction, so "fewer characters remain
//! than a full block needs" is the whole of the tail detection.
//!
//! Nothing in the stream is a length a sender chooses. That was the one place
//! the previous format stood behind base64 (§14): its decoder parsed
//! attacker-controlled lengths. This one parses a mask, and a mask cannot
//! address anything past its own block.

use crate::alphabet::{
    AlphabetSeen, Profile, CLASSIC_BIT, TILDE, URL_BIT, WORDS, WORD_BAD, WORD_CLASS,
};
use crate::encode::{base64_len, BASE64_BLOCK_CHARS, BLOCK_BYTES, MASK_CHARS};
use crate::{Decoded, Error, Meta};

/// §10.2. The profile is the only parameter.
pub fn decode(stream: &[u8], profile: Profile) -> Result<Decoded, Error> {
    run(stream, profile, false)
}

/// §5.5: like [`decode`], but `+` and `/` at an alphabet position are an
/// error rather than the classic alphabet.
pub fn decode_url_strict(stream: &[u8], profile: Profile) -> Result<Decoded, Error> {
    run(stream, profile, true)
}

fn run(stream: &[u8], profile: Profile, strict_url: bool) -> Result<Decoded, Error> {
    // The output is never longer than the stream: a raw byte is one
    // character, and base64 is three bytes per four.
    let mut out = Vec::with_capacity(stream.len());
    let meta = run_into(stream, profile, strict_url, &mut out)?;
    Ok(Decoded {
        bytes: out,
        alphabet_seen: meta.alphabet_seen,
        padding_seen: meta.padding_seen,
    })
}

pub(crate) fn run_into(
    stream: &[u8],
    profile: Profile,
    strict_url: bool,
    out: &mut Vec<u8>,
) -> Result<Meta, Error> {
    let mut d = Decoder {
        profile,
        strict_url,
        alphabet_seen: AlphabetSeen::None,
        padding_seen: false,
        out,
    };
    d.blocks(stream)?;
    Ok(Meta {
        alphabet_seen: d.alphabet_seen,
        padding_seen: d.padding_seen,
    })
}

/// The two fields §5.5 requires in the result, carried while they are being
/// found. Rule A and Rule P are statements about the whole stream, so one
/// decoder threads through every block.
struct Decoder<'a> {
    profile: Profile,
    strict_url: bool,
    alphabet_seen: AlphabetSeen,
    padding_seen: bool,
    out: &'a mut Vec<u8>,
}

impl Decoder<'_> {
    /// Rule A (§5.4), and the strict variant from §5.5 in the same place.
    ///
    /// Takes the *classes seen* rather than a character, so that a base64 run
    /// can `or` a table entry per character and settle the rule once for the
    /// whole run.
    #[inline]
    fn note_classes(&mut self, bits: u8) -> Result<(), Error> {
        if bits & CLASSIC_BIT != 0 {
            if self.strict_url {
                return Err(Error::NonUrlAlphabet);
            }
            if self.alphabet_seen == AlphabetSeen::Url {
                return Err(Error::MixedAlphabet);
            }
            self.alphabet_seen = AlphabetSeen::Classic;
        }
        if bits & URL_BIT != 0 {
            if self.alphabet_seen == AlphabetSeen::Classic {
                return Err(Error::MixedAlphabet);
            }
            self.alphabet_seen = AlphabetSeen::Url;
        }
        Ok(())
    }

    /// One alphabet position: check first (§10.1, trap 1), then Rule A, then
    /// the value.
    #[inline]
    fn read_alphabet(&mut self, c: u8) -> Result<u8, Error> {
        let w = WORDS[c as usize];
        if w & WORD_BAD != 0 {
            return Err(Error::Charset);
        }
        self.note_classes(((w & WORD_CLASS) >> 8) as u8)?;
        Ok((w & 63) as u8)
    }

    /// §7: every byte of a raw payload must be one the profile admits.
    fn check_profile(&self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.iter().all(|&b| self.profile.allows(b)) {
            Ok(())
        } else {
            Err(Error::Profile)
        }
    }

    /// §10.1.
    fn blocks(&mut self, stream: &[u8]) -> Result<(), Error> {
        let len = stream.len();
        let mut pos = 0;
        while pos < len {
            if stream[pos] != TILDE {
                // A run of base64 blocks: every block that starts with an
                // alphabet character, sixty-four characters each, and the
                // last one whatever is left. Blocks tile (§4), so the run
                // decodes as one, and the inner loop runs once per run
                // rather than once per block.
                let mut end = pos;
                while end < len && stream[end] != TILDE {
                    end = (end + BASE64_BLOCK_CHARS).min(len);
                }
                self.base64_run(&stream[pos..end], end == len)?;
                pos = end;
            } else if pos + 1 == len {
                return Err(Error::TrailingTilde);
            } else if stream[pos + 1] == TILDE {
                // A raw block: forty-eight bytes, or whatever is left.
                pos += 2;
                let n = BLOCK_BYTES.min(len - pos);
                let bytes = &stream[pos..pos + n];
                self.check_profile(bytes)?;
                self.out.extend_from_slice(bytes);
                pos += n;
            } else {
                pos = self.mask_block(stream, pos + 1)?;
            }
        }
        Ok(())
    }

    /// The mask form (§6), from the first mask character. Returns where the
    /// block ends.
    fn mask_block(&mut self, stream: &[u8], mut pos: usize) -> Result<usize, Error> {
        let len = stream.len();
        if pos + MASK_CHARS > len {
            return Err(Error::Truncated);
        }
        let mut mask: u64 = 0;
        for j in 0..MASK_CHARS {
            let v = self.read_alphabet(stream[pos + j])? as u64;
            // Character j carries bytes 6j..6j+5, first byte in the top bit.
            for t in 0..6 {
                mask |= (v >> (5 - t) & 1) << (6 * j + t);
            }
        }
        pos += MASK_CHARS;
        let admitted = mask.count_ones() as usize;
        if pos + admitted > len {
            return Err(Error::Truncated);
        }
        let clear = &stream[pos..pos + admitted];
        self.check_profile(clear)?;
        pos += admitted;

        // The base64 part: its full length is known from the mask, and a
        // shorter remainder means this is the last block.
        let full = base64_len(BLOCK_BYTES - admitted);
        let at_end = len - pos <= full;
        let n = if at_end { len - pos } else { full };
        // Decoded in place at the end of `out`, then lifted into a buffer on
        // the stack: an allocation per block was a third of the decoder's
        // time on prose.
        let base = self.out.len();
        self.base64_run(&stream[pos..pos + n], at_end)?;
        let rest_len = self.out.len() - base;
        if rest_len > BLOCK_BYTES {
            self.out.truncate(base);
            return Err(Error::Mask);
        }
        let mut rest = [0u8; BLOCK_BYTES + 1];
        rest[..rest_len].copy_from_slice(&self.out[base..]);
        self.out.truncate(base);
        pos += n;

        // How many bytes this block holds, and whether the mask agrees. A full
        // block is forty-eight; a tail is what the two parts add up to, and
        // then the mask may not claim a byte past it.
        let m = admitted + rest_len;
        if m > BLOCK_BYTES || (mask >> m) != 0 {
            return Err(Error::Mask);
        }

        // Interleave in the order the mask gives, without a branch per byte:
        // both cursors read, one advances. The buffers carry one spare byte
        // so the read past the last real one is in bounds and unused.
        let mut clear_buf = [0u8; BLOCK_BYTES + 1];
        clear_buf[..admitted].copy_from_slice(clear);
        let mut block = [0u8; BLOCK_BYTES];
        let (mut c, mut r) = (0usize, 0usize);
        for (i, slot) in block.iter_mut().take(m).enumerate() {
            let bit = (mask >> i & 1) as usize;
            let cb = clear_buf[c];
            let rb = rest[r];
            *slot = if bit == 1 { cb } else { rb };
            c += bit;
            r += 1 - bit;
        }
        self.out.extend_from_slice(&block[..m]);
        Ok(pos)
    }

    /// One run of base64 characters (§5), with Rule P allowed only where the
    /// run is also the end of the stream (§5.3).
    fn base64_run(&mut self, seg: &[u8], at_stream_end: bool) -> Result<(), Error> {
        let k = if at_stream_end {
            seg.iter().rev().take(2).take_while(|&&c| c == b'=').count()
        } else {
            0
        };
        let n = seg.len() - k;
        let ok = match k {
            0 => true,
            1 => n % 4 == 3,
            2 => n % 4 == 2,
            _ => unreachable!("take(2)"),
        };
        if !ok {
            return Err(Error::Padding);
        }
        if k > 0 {
            self.padding_seen = true;
        }
        if n % 4 == 1 {
            return Err(Error::Align);
        }

        // The inner loop. Neither the character check nor Rule A branches:
        // both are properties of the *set* of characters in the run, so the
        // loop only accumulates bits and the questions are asked once,
        // afterwards. The destination is sized once and written as a slice.
        let body = &seg[..n];
        let (quanta, tail) = body.as_chunks::<4>();
        let mut seen: u16 = 0;
        let base = self.out.len();
        self.out.resize(base + quanta.len() * 3, 0);
        let (dst, _) = self.out[base..].as_chunks_mut::<3>();
        for (d, q) in dst.iter_mut().zip(quanta) {
            let (w0, w1, w2, w3) = (
                WORDS[q[0] as usize],
                WORDS[q[1] as usize],
                WORDS[q[2] as usize],
                WORDS[q[3] as usize],
            );
            seen |= w0 | w1 | w2 | w3;
            let v = ((w0 & 63) as u32) << 18
                | ((w1 & 63) as u32) << 12
                | ((w2 & 63) as u32) << 6
                | (w3 & 63) as u32;
            *d = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        }
        if seen & WORD_BAD != 0 {
            self.out.truncate(base);
            return Err(Error::Charset);
        }

        let mut acc: u32 = 0;
        for &c in tail {
            let w = WORDS[c as usize];
            if w & WORD_BAD != 0 {
                return Err(Error::Charset);
            }
            seen |= w;
            acc = (acc << 6) | (w & 63) as u32;
        }
        self.note_classes(((seen & WORD_CLASS) >> 8) as u8)?;
        match tail.len() {
            0 => {}
            2 => {
                if acc & 0x0F != 0 {
                    return Err(Error::NonzeroTail);
                }
                self.out.push((acc >> 4) as u8);
            }
            3 => {
                if acc & 0x03 != 0 {
                    return Err(Error::NonzeroTail);
                }
                self.out.push((acc >> 10) as u8);
                self.out.push((acc >> 2) as u8);
            }
            _ => unreachable!("n mod 4 == 1 was rejected above"),
        }
        Ok(())
    }
}
