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

use crate::alphabet::{value, AlphabetSeen, Profile, TILDE};
use crate::{Decoded, Error, Framing};

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
    let mut d = Decoder {
        profile,
        strict_url,
        alphabet_seen: AlphabetSeen::None,
        padding_seen: false,
        out: Vec::with_capacity(stream.len()),
    };
    match mode {
        Framing::Plain => d.plain(stream)?,
        Framing::Framed => d.framed(stream)?,
    }
    Ok(Decoded {
        bytes: d.out,
        alphabet_seen: d.alphabet_seen,
        padding_seen: d.padding_seen,
        framing_seen: mode,
    })
}

/// The three fields §5.5 requires in the result, carried while they are being
/// found. Rule A and Rule P are stream-wide, so a framed stream threads one
/// decoder through all its frames rather than one per frame.
struct Decoder {
    profile: Profile,
    strict_url: bool,
    alphabet_seen: AlphabetSeen,
    padding_seen: bool,
    out: Vec<u8>,
}

impl Decoder {
    /// Rule A (§5.4), and the strict variant from §5.5 in the same place: both
    /// are questions about a character at an alphabet position, and there is
    /// no other place where such a character is read.
    #[inline]
    fn note_alphabet(&mut self, c: u8) -> Result<(), Error> {
        match c {
            b'+' | b'/' => {
                if self.strict_url {
                    return Err(Error::NonUrlAlphabet);
                }
                if self.alphabet_seen == AlphabetSeen::Url {
                    return Err(Error::MixedAlphabet);
                }
                self.alphabet_seen = AlphabetSeen::Classic;
            }
            b'-' | b'_' => {
                if self.alphabet_seen == AlphabetSeen::Classic {
                    return Err(Error::MixedAlphabet);
                }
                self.alphabet_seen = AlphabetSeen::Url;
            }
            _ => {}
        }
        Ok(())
    }

    /// One alphabet position: check (1), then Rule A, then the value.
    #[inline]
    fn read_alphabet(&mut self, c: u8) -> Result<u8, Error> {
        let v = value(c).ok_or(Error::Charset)?;
        self.note_alphabet(c)?;
        Ok(v)
    }

    /// §10.1.
    fn plain(&mut self, stream: &[u8]) -> Result<(), Error> {
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
                while pos < len && stream[pos] != TILDE {
                    pos += 1;
                }
                let seg = &stream[start..pos];
                self.base64_segment(seg, pos == len)?;
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

        let body = &seg[..n];
        let mut acc: u32 = 0;
        let mut have = 0u32;
        for &c in body {
            let v = self.read_alphabet(c)? as u32;
            acc = (acc << 6) | v;
            have += 6;
            if have == 24 {
                self.out.push((acc >> 16) as u8);
                self.out.push((acc >> 8) as u8);
                self.out.push(acc as u8);
                acc = 0;
                have = 0;
            }
        }
        match have {
            0 => {}
            12 => {
                // Two characters, one byte: four bits are unused (§5).
                if acc & 0x0F != 0 {
                    return Err(Error::NonzeroTail);
                }
                self.out.push((acc >> 4) as u8);
            }
            18 => {
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
            self.plain(body)?;
            pos += 5 + frame_len;
        }
        Ok(())
    }
}
