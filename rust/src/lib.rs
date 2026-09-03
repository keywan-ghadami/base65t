// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Base65t — Base64URL plus a 65th character.
//!
//! The reference implementation of `docs/spec-v0.4.de.md`. Section numbers in
//! the comments are that document's; `docs/history/` holds the earlier
//! versions and the record of how each decision was reached.
//!
//! **The wire format is not stable.** v0.4 replaced the segment format of the
//! earlier versions with fixed blocks, and nothing promises that v0.5 keeps
//! them. What is stable is the contract: bytes in, printable ASCII out, never
//! longer than base64, and any base64 stream reads back.
//!
//! ```
//! use base65t::{decode, encode, Profile};
//!
//! let out = encode(b"alice.jones");
//! assert_eq!(out, b"~~alice.jones");
//! assert_eq!(decode(&out, Profile::U).unwrap().bytes, b"alice.jones");
//! ```
//!
//! # One encoder
//!
//! [`encode`] takes bytes and returns bytes. There is no mode to pick, no
//! threshold to tune and no preset to understand, and that is the design
//! rather than an omission: a caller who has to choose has to know what the
//! choices mean before encoding a byte, and a caller who is unsure reaches
//! for base64. The encoder is one comparison per block of forty-eight bytes
//! (§4): all text, or base64. It neither searches nor remembers.
//!
//! The two parameters that remain are not choices about the encoding. The
//! profile (§7) is a statement about the container the stream has to survive,
//! and [`encode_base64url`] is the way out for a caller who wants no part of
//! the input left in the clear (§14).
//!
//! # One decoder
//!
//! [`decode`] takes a stream and a profile and needs nothing else (§0.3): the
//! alphabet variant and the padding come out of the stream and are reported
//! back in [`Decoded`]. [`decode_url_strict`] fixes the alphabet instead.

// §14 makes memory safety the payment for parsing untrusted input. Paying it
// and then reaching for `unsafe` for a lookup table would be the worst of
// both.
#![forbid(unsafe_code)]

mod alphabet;
mod decode;
mod encode;

pub use alphabet::{AlphabetSeen, Profile};
pub use decode::{decode, decode_url_strict};
pub use encode::{choose, Form, BASE64_BLOCK_CHARS, BLOCK_BYTES};

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
    pub alphabet_seen: AlphabetSeen,
    pub padding_seen: bool,
}

/// The nine error codes of §10.4, under their names there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The stream ends in a lone `~`.
    TrailingTilde,
    /// `~` followed by an alphabet character: a block form this version does
    /// not define and a later one may (§17).
    Reserved,
    /// A clear byte the profile does not admit.
    Profile,
    /// A base64 run of length `1 mod 4`, which no number of bytes produces.
    Align,
    /// Unused bits of the last quantum are not zero — a stream some permissive
    /// base64 libraries accept, and this one deliberately does not (§1.1).
    NonzeroTail,
    /// A character with no value where the grammar requires one: `~` inside a
    /// base64 run, `=` anywhere but the very end, `~` followed by something
    /// that is neither `~` nor an alphabet character.
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
            Error::Reserved => "E_RESERVED",
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
pub fn encode(data: &[u8]) -> Vec<u8> {
    encode_with(data, Profile::U)
}

/// The encoding, in the profile a container asks for (§7).
pub fn encode_with(data: &[u8], profile: Profile) -> Vec<u8> {
    let mut out = Vec::new();
    encode::encode_into(data, profile, &mut out);
    out
}

/// The encoding, appending to a buffer the caller owns.
///
/// The same bytes [`encode_with`] returns; what changes is who owns the
/// memory. A caller encoding many small values in a loop wants to say where
/// the output goes, and on those values the allocation this saves is a real
/// share of the work.
pub fn encode_into(data: &[u8], profile: Profile, out: &mut Vec<u8>) {
    encode::encode_into(data, profile, out);
}

/// Base64URL and nothing else, whatever the input looks like.
///
/// Two callers want this. One is carrying a secret and does not want any part
/// of it left in the clear, which §14 is about; the other is talking to
/// something that only speaks base64url and wants this library to be the one
/// dependency. It is not a mode of the format -- the output is ordinary
/// unpadded base64url, and any base64 decoder reads it. So does this one.
pub fn encode_base64url(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(encode::base64_len(data.len()));
    encode::emit_base64(data, &mut out);
    out
}

/// Decode into a buffer the caller owns, appending to what is there.
///
/// The counterpart of [`encode_into`]. On an error the buffer is left as it
/// was found.
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
