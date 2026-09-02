// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Base65t — Base64URL plus a 65th character.
//!
//! The reference implementation of `docs/spec-v0.4.de.md`. Section numbers in
//! the comments are that document's; `docs/history/` holds v0.1 and v0.2 and
//! the record of how each decision was reached.
//!
//! ```
//! use base65t::{decode, encode, Profile};
//!
//! let out = encode(b"alice.jones");
//! assert_eq!(out, b"~Lalice.jones");
//! assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"alice.jones");
//! ```
//!
//! # One encoder
//!
//! [`encode`] takes bytes and returns bytes. There is no mode to pick, no
//! threshold to tune and no preset to understand, and that is the design
//! rather than an omission: a caller who has to choose between a dense
//! encoder and a fast one has to know what those words mean before encoding a
//! byte, and a caller who is unsure reaches for base64. §9.6 makes the choice
//! instead, from the head of the input.
//!
//! The two parameters that remain are not choices about the encoding. The
//! profile (§7) is a statement about the container the stream has to survive,
//! and [`encode_base64url`] is the way out for a caller who wants no part of
//! the input left in the clear (§14).
//!
//! # Octets, not text
//!
//! Encoding produces an octet stream (§3). Under both profiles every octet of
//! it is printable ASCII, but the interface is `[u8]` in both directions and
//! the caller converts where a container guarantees more.
//!
//! # One decoder
//!
//! [`decode`] takes a stream and a profile and needs nothing else (§0.3):
//! the alphabet variant and the padding come out of the stream and are
//! reported back in [`Decoded`]. What the stream chose is worth checking —
//! an attacker who controls the stream controls both (§14) — so
//! [`decode_url_strict`] fixes the choice instead.

// §14 makes memory safety the payment for parsing attacker-controlled lengths.
// Paying it and then reaching for `unsafe` for a lookup table would be the
// worst of both.
#![forbid(unsafe_code)]

mod alphabet;
mod canonical;
mod decode;
mod encode;

pub use alphabet::{AlphabetSeen, Profile, MAX_LITERAL, MIN_LITERAL};

/// Not the format's API, and not stable: the pieces §11.1's two readings have
/// to be compared through.
///
/// v0.1 defined the canonical form twice and the two definitions disagreed
/// (`docs/history/FINDINGS.md`, item 1). v0.4 keeps the order and drops the
/// construction, but the comparison that established which one to keep needs
/// the segmentation itself rather than the bytes, and needs both rules
/// reachable from outside the crate. `examples/tiebreak.rs` and
/// `tests/never_worse.rs` are what this exists for. Nothing else should use
/// it.
#[doc(hidden)]
pub mod internals {
    pub use crate::encode::{c_vector, costs, emit, segment_with, LiteralEnd, Rules, Seg};
}
pub use decode::{decode, decode_url_strict};
pub use encode::{classify, Mode, ENTROPY_LIMIT_MILLIBITS, SAMPLE_BYTES, WINDOW_BYTES};

use alphabet::Profile as P;
use encode::Rules;

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
}

/// What a decode found out about the stream, without the bytes: what
/// [`decode_into`] returns, because there the bytes are the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meta {
    /// `None` when no character of value 62 or 63 occurred — the stream then
    /// reads identically under both variants.
    pub alphabet_seen: AlphabetSeen,
    pub padding_seen: bool,
}

/// The twelve error codes of §10.4, under their names there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The stream ends in `~`, or in a header that is cut short.
    TrailingTilde,
    /// Length character 0, which §6.1 reserves.
    ReservedLen,
    /// A payload reaches past the end of the stream.
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
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for Error {}

/// The encoding, profile U — the parameterless call §9.3 requires.
///
/// There is one, and that is the design. A caller who has to pick between a
/// dense one and a canonical one and a fast one has to know what those words
/// mean before they can encode a byte, and a caller who is unsure reaches for
/// base64. So the encoder decides: §9.6 classifies the head of the input and
/// either writes base64url, where nothing is to be found, or runs the exact
/// programme of §9.2, where something is.
///
/// The answer is a function of the input. The classification is a magic-number
/// test and an integer entropy figure over a fixed prefix, the windows are cut
/// at absolute offsets, and the programme's tie-break is the order of §11.1 --
/// nothing here reads a clock, a thread count or a machine. Two encoders agree
/// byte for byte, which is what makes the output safe to compare, cache under
/// and sign.
pub fn encode(data: &[u8]) -> Vec<u8> {
    encode_with(data, P::U)
}

/// The encoding, in the profile a container asks for (§7).
pub fn encode_with(data: &[u8], profile: Profile) -> Vec<u8> {
    encode::encode_rules(data, Rules::new(profile, Some(1)))
}

/// The encoding, appending to a buffer the caller owns.
///
/// The same bytes [`encode_with`] returns; what changes is who owns the
/// memory. A caller encoding many small values in a loop, or one that has
/// registered a buffer with the kernel so io_uring can write from it without
/// pinning it per operation, wants to say where the output goes. On the values
/// §0.1 names, the allocation this saves is a real share of the work: about a
/// fifth of encoding an eight-byte value, a seventh at sixty-four, nothing
/// above half a kilobyte.
pub fn encode_into(data: &[u8], profile: Profile, out: &mut Vec<u8>) {
    encode::encode_rules_into(data, Rules::new(profile, Some(1)), out);
}

/// Base64URL and nothing else, whatever the input looks like.
///
/// Two callers want this. One is carrying a secret and does not want any part
/// of it left in the clear, which §14 is about; the other is talking to
/// something that only speaks base64url and wants this library to be the one
/// dependency. It is not a mode of the format -- the output is ordinary
/// unpadded base64url, and any base64 decoder reads it.
pub fn encode_base64url(data: &[u8]) -> Vec<u8> {
    encode::encode_rules(data, Rules::new(P::U, None))
}

/// Decode into a buffer the caller owns, appending to what is there.
///
/// The counterpart of [`encode_into`], and the same reasoning. On an error the
/// buffer is left as it was found.
pub fn decode_into(stream: &[u8], profile: Profile, out: &mut Vec<u8>) -> Result<Meta, Error> {
    let at = out.len();
    match decode::run_into(stream, profile, false, out) {
        Ok(meta) => Ok(meta),
        Err(e) => {
            out.truncate(at);
            Err(e)
        }
    }
}
