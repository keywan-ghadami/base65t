// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The alphabet, the 65th character, and the three profiles (§3, §5.2, §7).

/// Base64URL, RFC 4648 §5. The encoder writes this and only this (§5.1).
pub const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// The 65th character. Not in the alphabet, no value (§3).
pub const TILDE: u8 = b'~';

/// Longest literal a single segment can carry (§6.1): `63 + 4095`.
pub const MAX_LITERAL: usize = 4158;

/// Shortest literal `dense` and `framed` will take (§9.1).
///
/// Not a tuning parameter: §9.1 derives it. A literal of `L` bytes saves
/// `(L − 10)/3` characters against base64 once the worst rounding on both
/// sides is charged to it, so eleven is the first length that cannot lose --
/// which is the whole of why §9.4 holds for a rule that never looks ahead.
pub const MIN_LITERAL: usize = 11;

/// Longest frame body, in characters (§8.1): 18 bits of length.
pub const MAX_FRAME_BODY: usize = 262_143;

/// Which alphabet variant a character belongs to, as bits, so that Rule A
/// (§5.4) costs one `or` per character and one test per segment instead of two
/// branches per character.
pub const CLASSIC_BIT: u8 = 1;
pub const URL_BIT: u8 = 2;

/// Value, alphabet class and validity of a character in one word.
///
/// The decoder asks two things of every character it reads -- what it is worth
/// and which alphabet it came from -- and asked them of two tables until this
/// one merged them. That halves the loads in the loop, though not the time:
/// the loop is bound by the dependency chain that assembles each quantum, and
/// the measured difference sits inside the run-to-run spread of the bench. It
/// is kept for the simplification, one table where the two had to be read in
/// step, and the claim is only that it does not cost anything.
///
/// A table rather than a chain of range tests, and not as a micro-optimisation:
/// five branches per character against one indexed load is most of the
/// difference between decoding at base64's speed and at several times it.
/// `~` and `=` are marked invalid like everything else outside the alphabet,
/// which is what makes the checks marked (1) in §10.1 possible at all.
///
/// Bits 0–5 hold the value, [`WORD_CLASS`] the alphabet class, and
/// [`WORD_BAD`] marks everything that is not an alphabet character. The bits
/// are laid out so that a segment's characters can be `or`-ed together and
/// both questions answered once for the whole segment: no legal character
/// carries `WORD_BAD`, so the accumulated word carries it exactly when some
/// character was illegal.
pub const WORD_CLASS: u16 = 0x0300;
pub const WORD_BAD: u16 = 0x8000;

pub static WORDS: [u16; 256] = {
    let mut t = [WORD_BAD; 256];
    let mut i = 0;
    while i < 64 {
        t[ALPHABET[i] as usize] = i as u16;
        i += 1;
    }
    t[b'+' as usize] = 62 | ((CLASSIC_BIT as u16) << 8);
    t[b'/' as usize] = 63 | ((CLASSIC_BIT as u16) << 8);
    t[b'-' as usize] = 62 | ((URL_BIT as u16) << 8);
    t[b'_' as usize] = 63 | ((URL_BIT as u16) << 8);
    t
};

/// Which alphabet variant a stream's *alphabet characters* have been written
/// in (§5.5). Literal payloads never touch this — see Rule A in §5.4 and the
/// negative test in TV7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphabetSeen {
    /// No character of value 62 or 63 occurred, so the stream reads the same
    /// under both variants.
    None,
    /// `-` or `_`.
    Url,
    /// `+` or `/`.
    Classic,
}

/// What a literal payload may contain (§7). The one parameter `decode` keeps,
/// because it is a statement about the container and not about the stream
/// (§7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// RFC 3986 *unreserved*: 66 characters, and the only profile that goes
    /// into a URL query or a cookie value as it stands (§7.1).
    U,
    /// Printable ASCII without `"` and `\`, so a JSON string carries it
    /// unescaped. Not URL-safe and not CSV-safe.
    T,
    /// Every octet. Leaves ASCII behind, and with it every text container.
    B,
}

/// Profile membership as a table: bit 0 for U, bit 1 for T, bit 2 for B.
///
/// The encoder asks this of every input byte, so it is an inner loop like the
/// decoder's value lookup, and for the same reason it is an indexed load
/// rather than a handful of range tests.
static MEMBERSHIP: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut b = 0usize;
    while b < 256 {
        let c = b as u8;
        let unreserved =
            c.is_ascii_alphanumeric() || c == b'-' || c == b'.' || c == b'_' || c == TILDE;
        let text = 0x20 <= c && c <= 0x7E && c != b'"' && c != b'\\';
        t[b] = (unreserved as u8) | ((text as u8) << 1) | 0b100;
        b += 1;
    }
    t
};

impl Profile {
    /// Whether this byte may appear in a literal payload.
    #[inline]
    pub fn allows(self, b: u8) -> bool {
        let bit = match self {
            Profile::U => 0b001,
            Profile::T => 0b010,
            Profile::B => 0b100,
        };
        MEMBERSHIP[b as usize] & bit != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §7.1, as an executable form of the ABNF argument rather than of the
    /// table beside it: every byte profile U admits is a `cookie-octet` from
    /// RFC 6265 §4.1.1, and there are 66 of them.
    #[test]
    fn profile_u_is_cookie_octet() {
        let cookie_octet = |b: u8| {
            b == 0x21
                || (0x23..=0x2B).contains(&b)
                || (0x2D..=0x3A).contains(&b)
                || (0x3C..=0x5B).contains(&b)
                || (0x5D..=0x7E).contains(&b)
        };
        let admitted: Vec<u8> = (0..=255u8).filter(|&b| Profile::U.allows(b)).collect();
        assert_eq!(admitted.len(), 66, "unreserved is 66 characters");
        for b in admitted {
            assert!(cookie_octet(b), "{:?} is not a cookie-octet", b as char);
        }
    }

    /// The alphabet is a subset of profile U, which is why a literal may abut a
    /// base64 segment without either needing a delimiter.
    #[test]
    fn alphabet_is_unreserved() {
        for &c in ALPHABET.iter() {
            assert!(Profile::U.allows(c), "{:?}", c as char);
        }
    }

    #[test]
    fn values_round_trip_and_are_permissive() {
        let word = |c: u8| WORDS[c as usize];
        let val = |c: u8| (word(c) & 63) as u8;
        let bad = |c: u8| word(c) & WORD_BAD != 0;
        for (i, &c) in ALPHABET.iter().enumerate() {
            assert!(!bad(c));
            assert_eq!(val(c), i as u8);
        }
        assert_eq!(val(b'+'), 62);
        assert_eq!(val(b'/'), 63);
        assert!(bad(TILDE) && bad(b'=') && bad(b'.'));
    }

    /// The class bits are what Rule A (§5.4) reads, and they may not move when
    /// the value bits are packed beside them.
    #[test]
    fn only_the_four_variant_characters_carry_a_class() {
        for c in 0..=255u8 {
            let class = ((WORDS[c as usize] & WORD_CLASS) >> 8) as u8;
            let expected = match c {
                b'+' | b'/' => CLASSIC_BIT,
                b'-' | b'_' => URL_BIT,
                _ => 0,
            };
            assert_eq!(class, expected, "{:?}", c as char);
        }
    }

    /// Profile T is the JSON-safe one, and that is the whole of its claim.
    #[test]
    fn profile_t_excludes_exactly_quote_and_backslash() {
        for b in 0x20..=0x7Eu8 {
            assert_eq!(Profile::T.allows(b), b != b'"' && b != b'\\');
        }
        assert!(!Profile::T.allows(0x1F));
        assert!(!Profile::T.allows(0x7F));
    }
}
