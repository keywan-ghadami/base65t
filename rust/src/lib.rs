// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Base65t — Base64URL plus a 65th character.
//!
//! The reference implementation of `docs/spec-v0.2.de.md`. Section numbers in
//! the comments are that document's. v0.2 changed no bit of the wire format;
//! it settled what an encoder must choose where v0.1 left the choice open, and
//! `docs/errata-v0.1.de.md` records each decision with its reasoning.
//!
//! ```
//! use base65t::{decode, encode, Profile};
//!
//! let out = encode(b"alice.jones");
//! assert_eq!(out, b"~Lalice.jones");
//! assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"alice.jones");
//! ```
//!
//! # Octets, not text
//!
//! Encoding produces an octet stream (§3). Under profiles U and T every octet
//! of it is printable ASCII, but under profile B it is not, so the interface
//! is `[u8]` in both directions and the caller converts where a container
//! guarantees more.
//!
//! # One decoder
//!
//! `decode` takes a stream and a profile and needs nothing else (§0.3):
//! alphabet variant, padding and framing come out of the stream and are
//! reported back in [`Decoded`]. What the stream chose is worth checking —
//! an attacker who controls the stream controls all three (§14) — so
//! [`decode_plain`], [`decode_framed`] and [`decode_url_strict`] fix the
//! choice instead.

// §14 makes memory safety the payment for parsing attacker-controlled lengths.
// Paying it and then reaching for `unsafe` for a lookup table would be the
// worst of both.
#![forbid(unsafe_code)]

mod alphabet;
mod canonical;
mod decode;
mod encode;

pub use alphabet::{AlphabetSeen, Profile, MAX_FRAME_BODY, MAX_LITERAL, MIN_LITERAL};

/// Not the format's API, and not stable: the pieces §11.1's two readings have
/// to be compared through.
///
/// `canonical` is defined twice in §11.1 and the two definitions disagree
/// (FINDINGS.md, item 1). Deciding which one the format keeps is a judgement
/// about streams neither the encoder nor a caller ever needs to see, so the
/// comparison needs the segmentation itself rather than the bytes — and it
/// needs both rules reachable from outside the crate. `examples/tiebreak.rs`
/// is what this exists for. Nothing else should use it.
#[doc(hidden)]
pub mod internals {
    pub use crate::encode::{
        c_vector, costs, emit, segment_greedy, segment_with, LiteralEnd, Rules, Seg,
    };

    /// Where [`crate::encode_parallel`] splits. Exposed so that a test can
    /// assert the split happens at all: an encoder that quietly fell back to
    /// one thread would pass every assertion about the output.
    pub fn cut_points(data: &[u8], profile: crate::Profile, threads: usize) -> Vec<usize> {
        crate::encode::cut_points(
            data,
            Rules::preset(profile, Some(crate::MIN_LITERAL), false),
            threads,
        )
    }
}
pub use canonical::encode_canonical;
pub use decode::{decode, decode_framed, decode_plain, decode_url_strict, framing_of};
pub use encode::{FAST_SAMPLE, FAST_WINDOW, FRAME_BYTES};

use alphabet::Profile as P;
use encode::Rules;

/// How a stream carries its segments (§5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    /// Segments back to back; decoding is sequential from the start.
    Plain,
    /// Length-prefixed frames, so a byte offset can be reached without
    /// decoding what precedes it (§8).
    Framed,
}

/// What `decode` found while decoding, which §5.5 makes part of the result
/// rather than an option: permissiveness that cannot be inspected is
/// permissiveness that cannot be validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub bytes: Vec<u8>,
    /// `None` when no character of value 62 or 63 occurred at an alphabet
    /// position — the stream then reads identically under both variants.
    pub alphabet_seen: AlphabetSeen,
    pub padding_seen: bool,
    pub framing_seen: Framing,
}

/// What a decode found out about the stream, without the bytes: what
/// [`decode_into`] returns, because there the bytes are the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meta {
    /// `None` when no character of value 62 or 63 occurred — the stream then
    /// reads identically under both variants.
    pub alphabet_seen: AlphabetSeen,
    pub padding_seen: bool,
    pub framing_seen: Framing,
}

/// The twelve error codes of §10.4, under their names there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The stream ends in `~`, or in a header that is cut short.
    TrailingTilde,
    /// Length character 0 in plain mode. In framed mode this is a frame
    /// header, which is what Rule F rests on (§5.6).
    ReservedLen,
    /// A payload or a frame reaches past the end of the stream.
    Truncated,
    /// A literal byte the profile does not admit.
    Profile,
    /// A base64 segment of length `1 mod 4`, which no number of bytes
    /// produces.
    Align,
    /// Unused bits of the last quantum are not zero — a stream some permissive
    /// base64 libraries accept, and this one deliberately does not (§1.1).
    NonzeroTail,
    /// A character with no value where the grammar requires one: `~`, `=`
    /// anywhere but the very end, a header position that is not an alphabet
    /// character.
    Charset,
    /// Rule P: padding that `n mod 4` does not call for (§5.3).
    Padding,
    /// Rule A: both alphabet variants at alphabet positions (§5.4).
    MixedAlphabet,
    /// `+` or `/` under [`decode_url_strict`] (§5.5).
    NonUrlAlphabet,
    /// Invariant F′: `~A` inside a frame body (§8.2).
    FrameRule,
    /// A frame header was expected and is not there.
    FrameSync,
}

impl Error {
    /// The code as §10.4 writes it.
    pub fn code(self) -> &'static str {
        match self {
            Error::TrailingTilde => "E_TRAILING_TILDE",
            Error::ReservedLen => "E_RESERVED_LEN",
            Error::Truncated => "E_TRUNCATED",
            Error::Profile => "E_PROFILE",
            Error::Align => "E_ALIGN",
            Error::NonzeroTail => "E_NONZERO_TAIL",
            Error::Charset => "E_CHARSET",
            Error::Padding => "E_PADDING",
            Error::MixedAlphabet => "E_MIXED_ALPHABET",
            Error::NonUrlAlphabet => "E_NON_URL_ALPHABET",
            Error::FrameRule => "E_FRAME_RULE",
            Error::FrameSync => "E_FRAME_SYNC",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for Error {}

/// The five presets of §9.3. They differ in what the encoder is allowed to do,
/// never in what a decoder has to understand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Smallest output, and never larger than base64 (§9.4). The default.
    Dense,
    /// Literals from four bytes up: readable in a log at some cost in size.
    Legible,
    /// The deterministic one (§11.1). For cache keys, not for signatures.
    Canonical,
    /// Never a literal, so nothing of the input shows through. Byte-identical
    /// to unpadded Base64URL.
    Opaque,
    /// Fixed-size frames for random access (§8). The guarantee in §9.4 does
    /// not cover it.
    Framed,
    /// `dense`, except that a window whose sample shows too little to gain is
    /// written as base64 without being scanned (§9.6).
    DenseFast,
}

/// `dense`, profile U — the parameterless call §9.3 requires.
pub fn encode(data: &[u8]) -> Vec<u8> {
    encode_dense(data, P::U)
}

/// Encode into a buffer the caller owns, appending to what is there.
///
/// The same bytes [`encode_with`] returns; what changes is who owns the
/// memory. A caller that encodes many small values in a loop, or one that has
/// registered a buffer with the kernel so that io_uring can write from it
/// without pinning it per operation, wants to say where the output goes. On
/// the values §0.1 names, the allocation this saves is a real share of the
/// work: 22 % of encoding a sixteen-byte value, 16 % of a sixty-four-byte one,
/// 8 % at a hundred and fifty. Above a few kilobytes it is nothing.
pub fn encode_into(data: &[u8], preset: Preset, profile: Profile, out: &mut Vec<u8>) {
    // Not `out.append(&mut encode_with(...))`, which would allocate the thing
    // this exists to avoid.
    match preset {
        Preset::Dense => {
            encode::encode_greedy_into(data, Rules::preset(profile, Some(MIN_LITERAL), false), out)
        }
        Preset::DenseFast => {
            let mut rules = Rules::preset(profile, Some(MIN_LITERAL), false);
            rules.fast = true;
            encode::encode_greedy_into(data, rules, out)
        }
        Preset::Opaque => encode::encode_greedy_into(data, Rules::preset(P::U, None, false), out),
        // The presets that optimise over the whole input build a segment list
        // either way, so there is nothing to save beyond the final copy.
        other => out.extend_from_slice(&encode_with(data, other, profile)),
    }
}

/// Decode into a buffer the caller owns, appending to what is there.
///
/// The counterpart of [`encode_into`], and the same reasoning. The bytes are
/// what [`decode`] would have put in its own `Vec`; the framing is detected as
/// it is there. On an error the buffer is left as it was found.
pub fn decode_into(stream: &[u8], profile: Profile, out: &mut Vec<u8>) -> Result<Meta, Error> {
    let at = out.len();
    let mode = decode::framing_of(stream);
    match decode::run_into(stream, profile, false, mode, out) {
        Ok(meta) => Ok(meta),
        Err(e) => {
            out.truncate(at);
            Err(e)
        }
    }
}

/// Any preset, any profile.
pub fn encode_with(data: &[u8], preset: Preset, profile: Profile) -> Vec<u8> {
    match preset {
        Preset::Dense => encode_dense(data, profile),
        Preset::Legible => encode_legible(data, profile),
        Preset::Canonical => encode_canonical(data, profile),
        Preset::Opaque => encode_opaque(data),
        Preset::Framed => encode_framed(data, profile),
        Preset::DenseFast => encode_dense_fast(data, profile),
    }
}

/// Literals from eleven bytes up, where §9.1 shows they can never cost.
///
/// One forward scan, constant memory, no backpointers: the linear rule of
/// §9.2.1 rather than the exact programme of §9.2.2. It is not length-optimal
/// — it never absorbs a byte into a base64 run to align a quantum — but it is
/// exactly specified, so its output is a function like every other preset's,
/// and §9.1's derivation makes it impossible for it to lose against base64.
///
/// This is what a caller gets from [`encode`], so it is the one that has to be
/// fast.
pub fn encode_dense(data: &[u8], profile: Profile) -> Vec<u8> {
    encode::encode_greedy(data, Rules::preset(profile, Some(MIN_LITERAL), false))
}

/// Readability at no cost in size: the shortest encoding, and among the
/// shortest the one that leaves the most bytes readable.
///
/// v0.1 defined `legible` by a threshold — literals from four bytes up — and a
/// threshold cannot make output more readable: the objective is still the
/// length (§9.0), so a literal that costs anything is never chosen and
/// `legible` collapses into `dense`. §9.3 now gives it an objective instead.
///
/// The measurement in PREREGISTRATION.md is why this and not a budget over
/// `dense`: every bonus large enough to buy a literal that costs something
/// broke §9.4 on 35 of 87 corpus files, while breaking ties towards
/// readability costs nothing at all and is worth about five points of
/// passthrough. So §9.4 covers `legible` too (TV14).
pub fn encode_legible(data: &[u8], profile: Profile) -> Vec<u8> {
    let mut rules = Rules::preset(profile, Some(1), false);
    rules.prefer_passthrough = true;
    encode::encode_rules(data, rules)
}

/// `dense` on several threads, byte for byte what [`encode_dense`] writes.
///
/// `threads` is a performance knob and nothing else: every value produces the
/// same stream, because §9.2.1 is a rule about local bytes and the workers cut
/// where no segment spans the cut (`encode::encode_parallel` carries the
/// argument). `0` asks for one worker per available core. Inputs below a
/// megabyte, and inputs the rule finds no literal in, run on the calling
/// thread -- the second because a stream with no literals is base64, which
/// this already writes faster than the comparison does.
pub fn encode_parallel(data: &[u8], profile: Profile, threads: usize) -> Vec<u8> {
    let threads = if threads == 0 {
        std::thread::available_parallelism().map_or(1, |n| n.get())
    } else {
        threads
    };
    encode::encode_parallel(
        data,
        Rules::preset(profile, Some(MIN_LITERAL), false),
        threads,
    )
}

/// `dense`, minus the looking where the looking does not pay (§9.6).
///
/// The rule of §9.2.1 has to read the input to find out whether a literal is
/// in it, and where none is -- which is what a compressor hands you -- that
/// reading is work with nothing to show. `dense-fast` takes a sample of each
/// 65536-byte window and, where the sample puts less than a tenth of its bytes
/// into literals, writes the window as base64 without scanning it.
///
/// Windows are cut at absolute offsets and the sample is a fixed prefix, so
/// the output is a function of the input like every other preset's (§9.0), and
/// [`encode_parallel`] still splits it. A decision that turns out wrong costs
/// size and never correctness: an unscanned window is exactly base64, so §9.4
/// holds whatever the sample says.
///
/// Measured against `dense` over the corpus: 1.3x to 1.9x the encoding speed,
/// for nought to 1.3 points of density. Where there is real density to lose --
/// a stylesheet, a tar of source -- the sample says so and nothing is skipped.
pub fn encode_dense_fast(data: &[u8], profile: Profile) -> Vec<u8> {
    let mut rules = Rules::preset(profile, Some(MIN_LITERAL), false);
    rules.fast = true;
    encode::encode_greedy(data, rules)
}

/// Base64URL and nothing else. The profile does not enter into it: there are
/// no literals for it to constrain.
///
/// It used to reach the same answer through the programme of §9.2.2, which
/// then had one candidate to choose between and O(n) memory to do it in. The
/// answer was never in doubt: with no threshold there is no literal, so the
/// segmentation is the single base64 run, and writing it is the whole of the
/// work.
pub fn encode_opaque(data: &[u8]) -> Vec<u8> {
    encode::encode_greedy(data, Rules::preset(P::U, None, false))
}

/// Frames of [`FRAME_BYTES`] decoded bytes each, `dense` inside (§8.1).
pub fn encode_framed(data: &[u8], profile: Profile) -> Vec<u8> {
    encode::encode_framed(data, profile, MIN_LITERAL)
}
