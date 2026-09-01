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

/// Longest frame body, in characters (§8.1): 18 bits of length.
pub const MAX_FRAME_BODY: usize = 262_143;

/// Value of an alphabet character, permissively (§5.2): a decoder MUST take
/// `-`/`+` as 62 and `_`/`/` as 63, in base64 segments and in length headers
/// alike. `None` for everything else, including `~` and `=`, which is what
/// makes the checks marked (1) in §10.1 possible at all.
#[inline]
pub fn value(c: u8) -> Option<u8> {
    let v = match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'-' | b'+' => 62,
        b'_' | b'/' => 63,
        _ => return None,
    };
    Some(v)
}

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

impl Profile {
    /// Whether this byte may appear in a literal payload.
    #[inline]
    pub fn allows(self, b: u8) -> bool {
        match self {
            Profile::U => {
                b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_' || b == TILDE
            }
            Profile::T => (0x20..=0x7E).contains(&b) && b != b'"' && b != b'\\',
            Profile::B => true,
        }
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
        for (i, &c) in ALPHABET.iter().enumerate() {
            assert_eq!(value(c), Some(i as u8));
        }
        assert_eq!(value(b'+'), Some(62));
        assert_eq!(value(b'/'), Some(63));
        assert_eq!(value(TILDE), None);
        assert_eq!(value(b'='), None);
        assert_eq!(value(b'.'), None);
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
