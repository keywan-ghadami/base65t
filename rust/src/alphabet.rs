// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The alphabet, the marker `~`, and what may stand raw (§3, §5.2, §7).

/// Base64URL, RFC 4648 §5. The encoder writes this and only this (§5.1).
pub const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// The marker. Not in the alphabet, no value (§3). Doubled it opens a
/// raw block; followed by an alphabet character it is reserved (§17).
pub const TILDE: u8 = b'~';

/// Which alphabet variant a character belongs to, as bits, so that Rule A
/// (§5.4) costs one `or` per character and one test per run instead of two
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
/// which is what makes the check marked (1) in §10.1 possible at all.
///
/// Bits 0–5 hold the value, [`WORD_CLASS`] the alphabet class, and
/// [`WORD_BAD`] marks everything that is not an alphabet character. The bits
/// are laid out so that a run's characters can be `or`-ed together and
/// both questions answered once for the whole run: no legal character
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

/// The set of bytes that may stand raw in a block: RFC 3986 *unreserved*, 66
/// characters (§7). Written out one byte at a time, which is the definition
/// the specification gives, and the thing [`allows`] is tested against.
#[cfg(test)]
static MEMBERSHIP: [bool; 256] = {
    let mut t = [false; 256];
    let mut b = 0usize;
    while b < 256 {
        let c = b as u8;
        t[b] = c.is_ascii_alphanumeric() || c == b'-' || c == b'.' || c == b'_' || c == TILDE;
        b += 1;
    }
    t
};

/// Whether **every** byte of `data` may stand raw (§9.0).
///
/// The only question the encoder asks of a block, and asking exactly it —
/// rather than building a bit per byte and comparing to all-ones — is what
/// took encoding on large files from 122 % of base64's time to parity.
///
/// The membership test inside is arithmetic and not a table lookup, and that
/// is the whole trick: a gather does not vectorise, six shifts and compares
/// do. The loop below is a branchless `or` of rejections over thirty-two
/// bytes, and only then is there a branch — one per group, taken as soon as
/// any byte in it settles the block.
///
/// **It does vectorise, and that is checked rather than hoped for.** In the
/// code the compiler emits for this function, most instructions work on
/// vector registers: sixteen bytes per operation on the baseline `x86-64`
/// target, which assumes only SSE2. Build with `-C target-cpu=native` and it
/// is thirty-two or sixty-four, which roughly halves what the check costs
/// (§13.1) without changing a byte of the output. Anyone doubting it can
/// look: `cargo rustc --release -- --emit=asm`.
///
/// The same width *without* a build flag would need runtime dispatch, and
/// both routes are shut: `#[target_feature]` needs `unsafe`, which this crate
/// forbids, and `std::simd` is not stable (rustc 1.98.1,
/// rust-lang/rust#86656).
#[inline]
pub fn admits_all(data: &[u8]) -> bool {
    let (groups, tail) = data.as_chunks::<32>();
    // The cheap necessary condition, on the first group only: every byte that
    // may stand raw is at most 0x7E (§7), so one `or` and one test reject the
    // block. On compressed or binary input -- the case where this whole
    // function is pure overhead -- the first thirty-two bytes hold a high bit
    // with probability 1 - 2^-32, so the block is settled by two vector
    // operations instead of the six a byte that the full test costs. On text
    // it passes and costs those two operations once, not once per group.
    if let Some(g) = groups.first() {
        let mut hi = 0u8;
        for &b in g {
            hi |= b;
        }
        if hi & 0x80 != 0 {
            return false;
        }
    }
    for g in groups {
        let mut bad = 0u8;
        for &b in g {
            bad |= !allows(b) as u8;
        }
        if bad != 0 {
            return false;
        }
    }
    let mut bad = 0u8;
    for &b in tail {
        bad |= !allows(b) as u8;
    }
    bad == 0
}

/// Whether this byte may stand raw in a block (§7).
///
/// Arithmetic rather than a lookup in [`MEMBERSHIP`], which is the same answer
/// by a different road: a table is one load and this is six cheap operations,
/// so on a single byte the table wins, and over a block the arithmetic wins by
/// a wide margin because it vectorises and a gather does not.
/// `only_the_arithmetic_and_the_table_agree` checks the two against each other
/// over all 256 bytes, which is what keeps the duplication honest.
#[inline]
pub fn allows(b: u8) -> bool {
    // `b | 0x20` folds the upper case onto the lower, so one range test
    // covers both.
    let alpha = (b | 0x20).wrapping_sub(b'a') < 26;
    let digit = b.wrapping_sub(b'0') < 10;
    alpha || digit || b == b'-' || b == b'.' || b == b'_' || b == TILDE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §7.1, as an executable form of the ABNF argument rather than of the
    /// table beside it: every byte that may stand raw is a `cookie-octet`
    /// from RFC 6265 §4.1.1, and there are 66 of them.
    #[test]
    fn every_raw_byte_is_a_cookie_octet() {
        let cookie_octet = |b: u8| {
            b == 0x21
                || (0x23..=0x2B).contains(&b)
                || (0x2D..=0x3A).contains(&b)
                || (0x3C..=0x5B).contains(&b)
                || (0x5D..=0x7E).contains(&b)
        };
        let admitted: Vec<u8> = (0..=255u8).filter(|&b| allows(b)).collect();
        assert_eq!(admitted.len(), 66, "unreserved is 66 characters");
        for b in admitted {
            assert!(cookie_octet(b), "{:?} is not a cookie-octet", b as char);
        }
    }

    /// The base64 alphabet is a subset of what may stand raw, which is why a
    /// raw block may abut a base64 block without either needing a delimiter,
    /// and why the whole output is one 66-character set (§3).
    #[test]
    fn alphabet_is_unreserved() {
        for &c in ALPHABET.iter() {
            assert!(allows(c), "{:?}", c as char);
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

    /// The two ways of asking §7's question, against each other over every
    /// byte. One is the definition written out, one is what runs.
    #[test]
    fn only_the_arithmetic_and_the_table_agree() {
        for b in 0..=255u8 {
            assert_eq!(allows(b), MEMBERSHIP[b as usize], "{b:#04x}");
        }
    }

    /// `admits_all` is `allows` over every byte, and its early exit must not
    /// change that -- whichever byte of a block is the one that rejects.
    #[test]
    fn admits_all_agrees_with_allows_at_every_position() {
        for n in 0..=80usize {
            let clean = vec![b'a'; n];
            assert!(admits_all(&clean), "{n}");
            for i in 0..n {
                for bad in [b' ', 0x00, 0x80, 0xff, b'"', b'\\', b'/'] {
                    let mut v = clean.clone();
                    v[i] = bad;
                    assert_eq!(
                        admits_all(&v),
                        v.iter().all(|&b| allows(b)),
                        "n={n} i={i} {bad:#04x}"
                    );
                }
            }
        }
    }

    /// The space is the character this format most conspicuously does not
    /// admit, and the one an earlier revision did. Pinned so that widening
    /// the set is a deliberate act with a failing test in front of it.
    #[test]
    fn the_space_and_the_punctuation_of_prose_stand_outside() {
        for b in *b" \t\n\r,;:!?'\"()[]{}/+=@#$%^&*<>|\\" {
            assert!(!allows(b), "{:?} must not stand raw", b as char);
        }
    }
}
