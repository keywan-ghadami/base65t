// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Base65t — Base64URL plus a 65th character, `~`.
//!
//! Three numbers here and they differ (§7): **64** symbols carry data, which
//! is base64url's alphabet unchanged; **65** is those plus `~`, the marker,
//! which carries none; **66** is what a byte of a stream can be, RFC 3986
//! *unreserved*. `.` is the difference between the last two, and it is what
//! makes the passthrough useful: a block is raw only if every byte is
//! admitted, so without `.` no hostname, filename or dotted identifier would
//! ever stand raw. The name counts the mechanism, 65; a container is checked
//! against 66.
//!
//! The reference implementation of `docs/spec-v0.4.md`. Section numbers in
//! the comments are that document's; `docs/history/` holds the earlier
//! versions and the record of how each decision was reached.
//!
//! **The wire format is not stable.** Nothing promises that v0.5 keeps these
//! blocks. What is stable is the contract: bytes in, printable ASCII out,
//! never longer than base64, and any base64 stream reads back.
//!
//! ```
//! use base65t::{decode, encode};
//!
//! let out = encode(b"alice.jones");
//! assert_eq!(out, "~~alice.jones");
//! assert_eq!(decode(&out).unwrap(), b"alice.jones");
//! ```
//!
//! # A drop-in for `base64`
//!
//! The signatures are that crate's: `encode` takes anything `AsRef<[u8]>` and
//! returns a `String`, `decode` returns `Result<Vec<u8>, _>`, and the
//! method-style call site compiles too.
//!
//! ```
//! use base65t::prelude::*;
//!
//! assert_eq!(URL_SAFE.encode("alice.jones"), "~~alice.jones");
//! assert_eq!(URL_SAFE.decode("YWxpY2U=").unwrap(), b"alice");
//! ```
//!
//! **The two sides are not equally safe to swap, and that is a fact about
//! the format rather than a restriction of this API.** A base65t decoder
//! reads every canonical base64 and base64url stream (§1.1, §5.2, §5.3), so
//! replacing a decoder changes nothing anyone can observe. Replacing an
//! *encoder* starts emitting `~`, which a base64 decoder rejects. So:
//! decoders first, encoders once every reader is one. Nothing here makes
//! that awkward; it is said once, here, and then the call sites are the
//! ones you already have.
//!
//! # One alphabet
//!
//! Whatever goes in, the output is these 66 characters and nothing else:
//! `A-Z a-z 0-9 - . _ ~`, exactly RFC 3986 *unreserved*. Not `=` either,
//! because the encoder writes no padding. The alphabet does not depend on the
//! data and there is no setting that changes it, which is what lets one
//! sentence cover every container: a URL query, a cookie value, a header
//! value, a JSON string, a filename, a log field.
//!
//! # One encoder
//!
//! [`encode`] takes bytes and returns bytes. There is no mode to pick, no
//! threshold to tune and no preset to understand, and that is the design
//! rather than an omission: a caller who has to choose has to know what the
//! choices mean before encoding a byte, and a caller who is unsure reaches
//! for base64. The encoder is one question per block of forty-eight bytes
//! (§4): all text, or base64. It neither searches nor remembers, and it takes
//! no parameter at all.
//!
//! It asks that question of the first sixty-four blocks before it starts, and
//! where none of them can be raw it writes base64url and stops asking (§9.6).
//! So on input where the format would gain nothing -- anything compressed,
//! and English prose, whose spaces leave no block whole -- the output is
//! base64url byte for byte and costs base64's time.
//!
//! [`encode_base64url`] is not a second mode but the way out of the format,
//! for a caller who wants no part of the input left in the clear (§14).
//!
//! # One decoder
//!
//! [`decode`] takes a stream and needs nothing else (§0.3): the alphabet
//! variant and the padding come out of the stream and are reported back in
//! [`Decoded`]. [`decode_url_strict`] fixes the alphabet variant instead.

// §14 makes memory safety the payment for parsing untrusted input. Paying it
// and then reaching for `unsafe` for a lookup table would be the worst of
// both.
#![forbid(unsafe_code)]

mod alphabet;
mod decode;
mod encode;

pub use alphabet::{admits_all, allows, AlphabetSeen};
pub use decode::{decode_detailed, decode_url_strict_detailed};
pub use encode::{
    any_block_can_be_raw, choose, Form, BASE64_BLOCK_CHARS, BLOCK_BYTES, SAMPLE_BLOCKS,
};

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
    /// A raw byte the alphabet does not admit.
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

/// The encoding — the parameterless call §9.3 requires, and the only one.
///
/// `String` rather than `Vec<u8>`, and generic over `AsRef<[u8]>`, so that
/// this is the same call as `base64`'s: swapping the import compiles. The
/// return type is not a claim the output is text in general — §3 calls it an
/// octet stream — it is that **this** output is always printable ASCII (§7)
/// and therefore always valid UTF-8. `into_bytes()` is free where the octets
/// are wanted.
pub fn encode<T: AsRef<[u8]>>(data: T) -> String {
    let mut out = String::new();
    encode_string(data, &mut out);
    out
}

/// The encoding, appending to a buffer the caller owns.
///
/// The same characters [`encode`] returns; what changes is who owns the
/// memory. A caller encoding many small values in a loop wants to say where
/// the output goes, and on those values the allocation this saves is a real
/// share of the work. Named as `base64`'s `encode_string` is.
pub fn encode_string<T: AsRef<[u8]>>(data: T, out: &mut String) {
    // Safe by construction rather than by assertion: §7 admits only printable
    // ASCII and the base64 alphabet is a subset of it, so every byte the
    // encoder appends is ASCII. `alphabet.rs` pins that in both directions,
    // and this crate forbids `unsafe`, so the check below is a real check.
    let mut bytes = std::mem::take(out).into_bytes();
    encode::encode_into(data.as_ref(), &mut bytes);
    *out = String::from_utf8(bytes).expect("§7: the encoder writes printable ASCII");
}

/// The octet form of [`encode`], for a caller who wants the bytes and no
/// UTF-8 check at all.
pub fn encode_into<T: AsRef<[u8]>>(data: T, out: &mut Vec<u8>) {
    encode::encode_into(data.as_ref(), out);
}

/// Base64URL and nothing else, whatever the input looks like.
///
/// Two callers want this. One is carrying a secret and does not want any part
/// of it left in the clear, which §14 is about; the other is talking to
/// something that only speaks base64url and wants this library to be the one
/// dependency. It is not a mode of the format -- the output is ordinary
/// unpadded base64url, and any base64 decoder reads it. So does this one.
pub fn encode_base64url<T: AsRef<[u8]>>(data: T) -> String {
    let data = data.as_ref();
    let mut out = Vec::with_capacity(encode::base64_len(data.len()));
    encode::emit_base64(data, &mut out);
    String::from_utf8(out).expect("base64url is ASCII")
}

/// Decode a stream — the drop-in counterpart of [`encode`].
///
/// Takes the stream and nothing else (§0.3), and returns the bytes, which is
/// the same shape as `base64`'s `decode`. What the stream *chose* — the
/// alphabet variant and whether it carried padding — is reported by
/// [`decode_detailed`]; §5.5 requires that it be available, and §14 is why a
/// caller validating untrusted input should ask.
///
/// **This is the safe half of a migration.** A base65t decoder reads every
/// canonical base64 and base64url stream, padded or not (§5.2, §5.3), so it
/// can replace a base64 decoder before anything writes the new format. The
/// encoder cannot be swapped as freely, and [`encode`] says why.
pub fn decode<T: AsRef<[u8]>>(stream: T) -> Result<Vec<u8>, Error> {
    decode::decode_detailed(stream.as_ref()).map(|d| d.bytes)
}

/// [`decode`], but a `+` or `/` at an alphabet position is `E_NON_URL_ALPHABET`
/// (§5.5).
///
/// The entry point for a caller who has decided which alphabet it speaks,
/// rather than letting the stream decide (§14).
pub fn decode_url_strict<T: AsRef<[u8]>>(stream: T) -> Result<Vec<u8>, Error> {
    decode::decode_url_strict_detailed(stream.as_ref()).map(|d| d.bytes)
}

/// Decode into a buffer the caller owns, appending to what is there.
///
/// The counterpart of [`encode_into`]. On an error the buffer is left as it
/// was found.
pub fn decode_into(stream: &[u8], out: &mut Vec<u8>) -> Result<Meta, Error> {
    let at = out.len();
    match decode::run_into(stream, false, out) {
        Ok(meta) => Ok(meta),
        Err(e) => {
            out.truncate(at);
            Err(e)
        }
    }
}

/// The method-style call shape of the `base64` crate, so that a call site
/// written against it compiles when only the import changes.
///
/// ```
/// use base65t::{engine::general_purpose::URL_SAFE, Engine as _};
///
/// assert_eq!(URL_SAFE.encode(b"alice.jones"), "~~alice.jones");
/// assert_eq!(URL_SAFE.decode("~~alice.jones").unwrap(), b"alice.jones");
/// ```
///
/// There is one engine because there is one encoding (§0.1). The names below
/// exist so that `STANDARD` and `URL_SAFE` call sites both keep compiling;
/// they are the same engine, and neither selects an alphabet — §7 fixes it.
pub trait Engine {
    /// The encoding (§9.3).
    fn encode<T: AsRef<[u8]>>(&self, input: T) -> String {
        encode(input)
    }
    /// The encoding, appending to a buffer the caller owns.
    fn encode_string<T: AsRef<[u8]>>(&self, input: T, out: &mut String) {
        encode_string(input, out)
    }
    /// The decoding (§10.2).
    fn decode<T: AsRef<[u8]>>(&self, input: T) -> Result<Vec<u8>, Error> {
        decode(input)
    }
    /// The decoding, appending to a buffer the caller owns.
    fn decode_vec<T: AsRef<[u8]>>(&self, input: T, out: &mut Vec<u8>) -> Result<(), Error> {
        decode_into(input.as_ref(), out).map(|_| ())
    }
}

/// The one engine. It carries no configuration, because there is none to
/// carry: §7 fixes the alphabet and §9.3 forbids a parameter that moves it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Base65t;

impl Engine for Base65t {}

/// Named as the `base64` crate names them, so both call sites compile.
pub mod engine {
    /// Named as the `base64` crate names it.
    pub mod general_purpose {
        use super::super::Base65t;
        /// The encoding. Not a standard-alphabet variant — there is one
        /// alphabet (§7) — but the name a call site may already use.
        pub static STANDARD: Base65t = Base65t;
        /// The same engine under the other name a call site may use. Base65t
        /// is URL-safe by construction (§7.1), so this is not a second thing.
        pub static URL_SAFE: Base65t = Base65t;
        /// The same engine. The encoder never writes padding (§5.1) and the
        /// decoder always accepts it (§5.3), so "no pad" is not a choice here.
        pub static STANDARD_NO_PAD: Base65t = Base65t;
        /// The same engine, for the same reason as `STANDARD_NO_PAD`.
        pub static URL_SAFE_NO_PAD: Base65t = Base65t;
    }
}

/// A minimal `use base65t::prelude::*;`, as the `base64` crate offers.
pub mod prelude {
    pub use super::engine::general_purpose::{
        STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
    };
    pub use super::Engine as _;
}
