// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The decoder, §10.
//!
//! §10.1 is written as pseudocode with three numbered traps in it, and this is
//! that pseudocode with the traps taken seriously:
//!
//! 1. A header position is checked for being an alphabet character *before*
//!    `value()` is called on it, so `~~abc` and `~=ab` are `E_CHARSET` rather
//!    than a lookup nobody defined.
//! 2. `note_alphabet` runs on header and segment characters and never on a
//!    literal payload. Rule A (§5.4) is about alphabet positions; a decoder
//!    that scans the whole stream rejects valid streams (TV7).
//! 3. Padding is recognised only where a segment ends at the end of the
//!    stream, and is never stripped in advance. In profile T `=` is a legal
//!    payload byte, and `~Ea=b=` decodes to `a=b=` (TV10).

use crate::alphabet::{
    AlphabetSeen, Profile, CLASSIC_BIT, TILDE, URL_BIT, WORDS, WORD_BAD, WORD_CLASS,
};
use crate::{Decoded, Error, Framing, Meta};

/// What `decode()` does with a stream before it looks at anything else
/// (Rule F, §5.6): two octets decide the mode, and nothing else can.
pub fn framing_of(stream: &[u8]) -> Framing {
    if stream.len() >= 2 && stream[0] == TILDE && stream[1] == b'A' {
        Framing::Framed
    } else {
        // Including the empty stream, which carries no information and is
        // valid in both modes; `plain` for it is a convention, not a
        // derivation (§5.6).
        Framing::Plain
    }
}

/// Rule F, then the mode it names. The profile is the only parameter (§0.3).
pub fn decode(stream: &[u8], profile: Profile) -> Result<Decoded, Error> {
    match framing_of(stream) {
        Framing::Framed => decode_framed(stream, profile),
        Framing::Plain => decode_plain(stream, profile),
    }
}

/// `decode`, but a `+` or `/` at an alphabet position ends it with
/// `E_NON_URL_ALPHABET` (§5.5).
pub fn decode_url_strict(stream: &[u8], profile: Profile) -> Result<Decoded, Error> {
    match framing_of(stream) {
        Framing::Framed => run(stream, profile, true, Framing::Framed),
        Framing::Plain => run(stream, profile, true, Framing::Plain),
    }
}

/// Plain mode, whatever the stream looks like. A framed stream reaches
/// `E_RESERVED_LEN` here, which is the correct answer for this entry point
/// (§10.2, TV11).
pub fn decode_plain(stream: &[u8], profile: Profile) -> Result<Decoded, Error> {
    run(stream, profile, false, Framing::Plain)
}

/// Framed mode, whatever the stream looks like. A plain stream reaches
/// `E_FRAME_SYNC` here.
pub fn decode_framed(stream: &[u8], profile: Profile) -> Result<Decoded, Error> {
    run(stream, profile, false, Framing::Framed)
}

fn run(stream: &[u8], profile: Profile, strict_url: bool, mode: Framing) -> Result<Decoded, Error> {
    // The only allocation a decode does, and `stream.len()` is the bound that
    // holds for every stream: four characters carry three bytes, but a
    // literal's characters carry one byte each, so a literal-heavy stream
    // decodes to almost its own length. `3/4` of it was briefly here as a
    // tighter bound. It is the bound for base64 and wrong for this format --
    // it made the shape base65t exists for, a short value that is one literal,
    // reallocate on every decode.
    let mut out = Vec::with_capacity(stream.len());
    let meta = run_into(stream, profile, strict_url, mode, &mut out)?;
    Ok(Decoded {
        bytes: out,
        alphabet_seen: meta.alphabet_seen,
        padding_seen: meta.padding_seen,
        framing_seen: meta.framing_seen,
    })
}

/// The same, appending to a buffer the caller owns.
pub(crate) fn run_into(
    stream: &[u8],
    profile: Profile,
    strict_url: bool,
    mode: Framing,
    out: &mut Vec<u8>,
) -> Result<Meta, Error> {
    let mut d = Decoder {
        profile,
        strict_url,
        alphabet_seen: AlphabetSeen::None,
        padding_seen: false,
        out,
    };
    match mode {
        Framing::Plain => d.plain(stream, Padding::Allowed)?,
        Framing::Framed => d.framed(stream)?,
    }
    Ok(Meta {
        alphabet_seen: d.alphabet_seen,
        padding_seen: d.padding_seen,
        framing_seen: mode,
    })
}

/// The offset of the next `~` in `hay`, or `None`.
///
/// Eight bytes at a time, because this is the other loop that runs once per
/// character of the stream. A byte-at-a-time scan -- `iter().position()` is
/// one, whatever it looks like -- costs about what decoding the characters
/// costs, so on a stream with no literals in it, which is every high-entropy
/// stream and most of what a protocol actually encodes, it halves the
/// decoder. Measured on a 660 KB wasm blob: 930 MiB/s against 1670.
///
/// The kernel is the standard zero-byte test. `v` is the word with every `~`
/// turned into a zero byte; `v.wrapping_sub(LO)` borrows into the high bit of
/// each byte that was zero, `!v` rules out the bytes that were merely `0x80`
/// or greater, and `HI` keeps one bit per byte. It has no false positives, so
/// the first set bit names the first `~` outright.
fn find_tilde(hay: &[u8]) -> Option<usize> {
    const LO: u64 = 0x0101_0101_0101_0101;
    const HI: u64 = 0x8080_8080_8080_8080;
    let (words, rest) = hay.as_chunks::<8>();
    for (i, w) in words.iter().enumerate() {
        let v = u64::from_le_bytes(*w) ^ (LO * TILDE as u64);
        let z = v.wrapping_sub(LO) & !v & HI;
        if z != 0 {
            return Some(i * 8 + (z.trailing_zeros() / 8) as usize);
        }
    }
    let done = hay.len() - rest.len();
    rest.iter().position(|&b| b == TILDE).map(|k| done + k)
}

/// Which alphabet variants a base64 run holds: [`CLASSIC_BIT`] for `+` or `/`,
/// [`URL_BIT`] for `-` or `_` (§5.2, §5.4).
///
/// This is the whole of what Rule A needs, and it is a *search* rather than a
/// decode -- which is what makes a vectorised decoder possible at all. A
/// library decodes into one alphabet and reports one opaque error; it cannot
/// say which of the two it saw, and base65t has to. Asked separately, at eight
/// bytes a word, the answer costs about a seventh of what decoding costs.
#[cfg(feature = "simd")]
fn variant_bits(hay: &[u8]) -> u8 {
    const LO: u64 = 0x0101_0101_0101_0101;
    const HI: u64 = 0x8080_8080_8080_8080;
    #[inline(always)]
    fn has(v: u64, needle: u8) -> u64 {
        let x = v ^ (LO * needle as u64);
        x.wrapping_sub(LO) & !x & HI
    }
    let (words, rest) = hay.as_chunks::<8>();
    let (mut classic, mut url) = (0u64, 0u64);
    for w in words {
        let v = u64::from_le_bytes(*w);
        classic |= has(v, b'+') | has(v, b'/');
        url |= has(v, b'-') | has(v, b'_');
    }
    let mut bits = ((classic != 0) as u8 * CLASSIC_BIT) | ((url != 0) as u8 * URL_BIT);
    for &b in rest {
        bits |= match b {
            b'+' | b'/' => CLASSIC_BIT,
            b'-' | b'_' => URL_BIT,
            _ => 0,
        };
    }
    bits
}

/// Below this many characters the two passes and the dispatch cost more than
/// the scalar loop they replace.
#[cfg(feature = "simd")]
const SIMD_MIN: usize = 64;

/// Whether Rule P (§5.3) reaches the end of this byte range: only the end of
/// the whole stream is the end of the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Padding {
    Allowed,
    Forbidden,
}

/// The three fields §5.5 requires in the result, carried while they are being
/// found. Rule A and Rule P are stream-wide, so a framed stream threads one
/// decoder through all its frames rather than one per frame.
struct Decoder<'a> {
    profile: Profile,
    strict_url: bool,
    alphabet_seen: AlphabetSeen,
    padding_seen: bool,
    out: &'a mut Vec<u8>,
}

impl Decoder<'_> {
    /// Rule A (§5.4), and the strict variant from §5.5 in the same place: both
    /// are questions about a character at an alphabet position, and there is
    /// no other place where such a character is read.
    ///
    /// Takes the *classes seen* rather than a character, so that a base64
    /// segment can `or` a table entry per character and settle the rule once
    /// for the whole segment. Header positions pass a single character's bits.
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

    /// One alphabet position: check (1), then Rule A, then the value.
    #[inline]
    fn read_alphabet(&mut self, c: u8) -> Result<u8, Error> {
        let w = WORDS[c as usize];
        if w & WORD_BAD != 0 {
            return Err(Error::Charset);
        }
        self.note_classes(((w & WORD_CLASS) >> 8) as u8)?;
        Ok((w & 63) as u8)
    }

    /// §10.1.
    ///
    /// `padding` is Rule P's reach. A frame body is a plain-mode stream by the
    /// grammar of §8.1, but it is not *the* stream, and §5.3 says the stream is
    /// always the whole octet stream. Padding exists so that a producer of
    /// ordinary base64 needs no changes (§1.1), and no such producer emits
    /// frames, so inside one it would be a parser-differential surface bought
    /// for nothing (TV15).
    fn plain(&mut self, stream: &[u8], padding: Padding) -> Result<(), Error> {
        let len = stream.len();
        let mut pos = 0;
        while pos < len {
            if stream[pos] == TILDE {
                if pos + 2 > len {
                    return Err(Error::TrailingTilde);
                }
                let l1 = self.read_alphabet(stream[pos + 1])?;
                if l1 == 0 {
                    // Reserved in plain mode; in framed mode this is a frame
                    // header, and that is the whole of Rule F (§5.6, §6.1).
                    return Err(Error::ReservedLen);
                }
                let l = if l1 == 63 {
                    if pos + 4 > len {
                        return Err(Error::Truncated);
                    }
                    let hi = self.read_alphabet(stream[pos + 2])? as usize;
                    let lo = self.read_alphabet(stream[pos + 3])? as usize;
                    pos += 4;
                    63 + ((hi << 6) | lo)
                } else {
                    pos += 2;
                    l1 as usize
                };
                if pos + l > len {
                    return Err(Error::Truncated);
                }
                let payload = &stream[pos..pos + l];
                if payload.iter().any(|&b| !self.profile.allows(b)) {
                    return Err(Error::Profile);
                }
                // No Rule A and no Rule P here: a payload is data (§5.4, TV7).
                self.out.extend_from_slice(payload);
                pos += l;
            } else {
                let start = pos;
                pos += find_tilde(&stream[pos..]).unwrap_or(len - pos);
                let seg = &stream[start..pos];
                let at_end = pos == len && padding == Padding::Allowed;
                self.base64_segment(seg, at_end)?;
            }
        }
        Ok(())
    }

    /// One base64 segment, and Rule P (§5.3) with it.
    ///
    /// `at_stream_end` is trap (3): only there may `=` appear, which is what
    /// keeps a padding character out of the scanning loop and out of the last
    /// byte of a profile-T literal.
    fn base64_segment(&mut self, seg: &[u8], at_stream_end: bool) -> Result<(), Error> {
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

        // The inner loop. Two things make it as fast as a plain base64
        // decoder rather than half its speed, and neither is about the
        // arithmetic:
        //
        // * The destination is sized once and written as a slice, not pushed
        //   onto a `Vec`. `extend_from_slice` per quantum re-checks the
        //   capacity and updates the length every three bytes, and the length
        //   lives behind `&mut self`, so it cannot stay in a register across
        //   the loop. Here `zip` over two chunk iterators gives the compiler
        //   two arrays of known length and no bounds check at all.
        // * Neither the character check nor Rule A branches. Both are
        //   properties of the *set* of characters in the segment, so the loop
        //   only accumulates bits and the questions are asked once, afterwards.
        let body = &seg[..n];

        // A vectorised decoder where the build asked for one, and where the
        // run is long enough to pay for two passes instead of one.
        //
        // Rule A goes first because its answer chooses the alphabet to ask
        // for: a library commits to one per call, and this stream may be in
        // either (§5.2). It also settles §5.5's strict variant, and rejects a
        // stream that mixes them -- all before a byte is decoded.
        //
        // The library returns one opaque error where §10.4 names twelve
        // conditions, so a failure falls through to the loop below rather than
        // being translated. That is the slow path by definition: it runs once,
        // on a stream that is about to be rejected.
        #[cfg(feature = "simd")]
        if body.len() >= SIMD_MIN {
            let bits = variant_bits(body);
            self.note_classes(bits)?;
            let alphabet = if bits & CLASSIC_BIT != 0 {
                base64_simd::STANDARD_NO_PAD
            } else {
                base64_simd::URL_SAFE_NO_PAD
            };
            let at = self.out.len();
            self.out.resize(at + body.len() / 4 * 3 + 3, 0);
            match alphabet.decode(body, base64_simd::Out::from_slice(&mut self.out[at..])) {
                Ok(decoded) => {
                    let len = decoded.len();
                    self.out.truncate(at + len);
                    return Ok(());
                }
                Err(_) => self.out.truncate(at),
            }
        }

        let (quanta, tail) = body.as_chunks::<4>();
        let mut seen = 0u16;

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

        // One character outside the alphabet set the bit. The bytes written
        // for its quantum go back with the error -- an error result promises
        // nothing about `out`, but leaving decoded garbage behind would make
        // the promise harder to keep for a caller that reuses the decoder.
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
                // Two characters, one byte: four bits are unused (§5).
                if acc & 0x0F != 0 {
                    return Err(Error::NonzeroTail);
                }
                self.out.push((acc >> 4) as u8);
            }
            3 => {
                // Three characters, two bytes: two bits are unused.
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

    /// §10.3. The order is normative: F′ is checked *before* the body is
    /// decoded, and the body goes to `plain`, never back through Rule F.
    fn framed(&mut self, stream: &[u8]) -> Result<(), Error> {
        let len = stream.len();
        let mut pos = 0;
        while pos < len {
            // Written as a slice comparison rather than two indexed reads: at
            // pos = len-1 the second read would be past the end, and a decoder
            // that parses attacker-controlled lengths (§14) should not have
            // that shape anywhere.
            if len - pos < 2 || &stream[pos..pos + 2] != b"~A" {
                return Err(Error::FrameSync);
            }
            if pos + 5 > len {
                return Err(Error::Truncated);
            }
            let a = self.read_alphabet(stream[pos + 2])? as usize;
            let b = self.read_alphabet(stream[pos + 3])? as usize;
            let c = self.read_alphabet(stream[pos + 4])? as usize;
            let frame_len = (a << 12) | (b << 6) | c;
            if pos + 5 + frame_len > len {
                return Err(Error::Truncated);
            }
            let body = &stream[pos + 5..pos + 5 + frame_len];
            if body.windows(2).any(|w| w == b"~A") {
                return Err(Error::FrameRule);
            }
            self.plain(body, Padding::Forbidden)?;
            pos += 5 + frame_len;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The word-at-a-time scan against the sentence it replaces, at every
    /// alignment and with the bytes that break a careless zero-byte test in
    /// the haystack: `0x00`, `0x80`, `0xFF`, and `0x7E ^ 0x80`.
    #[test]
    fn find_tilde_agrees_with_reading_it_one_byte_at_a_time() {
        let pool = [b'A', 0x00, 0x80, 0xFF, 0xFE, 0x7F, 0x7D, TILDE];
        let mut s: u32 = 0x7e7e_1234;
        let mut next = move || {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as usize
        };
        for len in 0..40usize {
            for _ in 0..200 {
                let hay: Vec<u8> = (0..len).map(|_| pool[next() % pool.len()]).collect();
                assert_eq!(
                    find_tilde(&hay),
                    hay.iter().position(|&b| b == TILDE),
                    "{hay:02x?}"
                );
            }
            // And with no tilde at all, which is the case the scan is for.
            let clean: Vec<u8> = (0..len).map(|_| pool[next() % (pool.len() - 1)]).collect();
            assert_eq!(find_tilde(&clean), None, "{clean:02x?}");
        }
    }
}
