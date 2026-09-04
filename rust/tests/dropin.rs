// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Every call shape the `base64` crate offers, written here exactly as a
//! caller would write it there.
//!
//! This file is the claim "swapping the import compiles", and it is a test
//! rather than a sentence in the README: if a signature drifts, this stops
//! building. What it cannot check is the other crate -- for that,
//! `tests/against_the_system.rs` runs Python's base64 and compares bytes.

use base66::prelude::*;
use base66::{decode, decode_url_strict, encode, encode_base64url};

/// The deprecated-but-everywhere free-function form:
/// `base64::encode(x)` / `base64::decode(x)`.
#[test]
fn the_free_function_form() {
    let s: String = encode(b"alice.jones");
    assert_eq!(s, "~~alice.jones");
    let back: Vec<u8> = decode(&s).unwrap();
    assert_eq!(back, b"alice.jones");
}

/// The current form: `URL_SAFE.encode(x)` after `use base64::Engine as _`.
#[test]
fn the_engine_method_form() {
    assert_eq!(URL_SAFE.encode("alice.jones"), "~~alice.jones");
    assert_eq!(STANDARD.encode("alice.jones"), "~~alice.jones");
    assert_eq!(URL_SAFE.decode("~~alice.jones").unwrap(), b"alice.jones");
    assert_eq!(URL_SAFE_NO_PAD.encode("alice.jones"), "~~alice.jones");
    assert_eq!(STANDARD_NO_PAD.encode("alice.jones"), "~~alice.jones");
}

/// `AsRef<[u8]>` and not `&[u8]`, because that is what the other crate takes:
/// a caller passing a `String`, a `&str`, a `Vec<u8>` or an array must compile
/// without a cast.
#[test]
fn every_argument_type_a_caller_may_already_be_passing() {
    let owned = String::from("alice.jones");
    let vec = b"alice.jones".to_vec();
    for s in [
        encode("alice.jones"),
        encode(&owned),
        encode(owned.clone()),
        encode(b"alice.jones"),
        encode(b"alice.jones".as_slice()),
        encode(&vec),
        encode(vec.clone()),
    ] {
        assert_eq!(s, "~~alice.jones");
    }
    // Bound first, so that what is checked is the argument *type* rather
    // than a literal clippy would rather see written differently.
    let stream_owned = String::from("~~alice.jones");
    let stream_vec: Vec<u8> = stream_owned.clone().into_bytes();
    for d in [
        decode("~~alice.jones").unwrap(),
        decode(&stream_owned).unwrap(),
        decode(stream_owned.clone()).unwrap(),
        decode(b"~~alice.jones").unwrap(),
        decode(b"~~alice.jones".as_slice()).unwrap(),
        decode(&stream_vec).unwrap(),
        decode(stream_vec.clone()).unwrap(),
    ] {
        assert_eq!(d, b"alice.jones");
    }
}

/// The buffer-reusing forms: `encode_string` and `decode_vec`.
#[test]
fn the_buffer_forms() {
    let mut s = String::from("keep ");
    URL_SAFE.encode_string("alice.jones", &mut s);
    assert_eq!(s, "keep ~~alice.jones");

    let mut v = b"keep ".to_vec();
    URL_SAFE.decode_vec("~~alice.jones", &mut v).unwrap();
    assert_eq!(v, b"keep alice.jones");
}

/// **The decoder side is the one that is safe to swap first**, and this is
/// what that means: every canonical base64 and base64url stream a base64
/// decoder would have taken, this one takes too, to the same bytes (§1.1).
#[test]
fn the_decoder_is_a_drop_in_for_base64_itself() {
    for (stream, want) in [
        ("YWxpY2U=", b"alice".as_slice()), // padded standard
        ("YWxpY2U", b"alice"),             // unpadded
        ("PDw_Pz8-Pg", b"<<???>>"),        // url alphabet
        ("PDw/Pz8+Pg", b"<<???>>"),        // classic alphabet
        ("", b""),                         // empty
    ] {
        assert_eq!(decode(stream).unwrap(), want, "{stream}");
    }
    // And the strict entry point is the one that refuses the classic alphabet.
    assert!(decode_url_strict("PDw/Pz8+Pg").is_err());
    assert!(decode_url_strict("PDw_Pz8-Pg").is_ok());
}

/// `encode_base64url` keeps the same shape, so a caller who wants the way out
/// of the format does not change call style either.
#[test]
fn the_way_out_has_the_same_shape() {
    let s: String = encode_base64url("alice.jones");
    assert_eq!(s, "YWxpY2Uuam9uZXM");
    assert_eq!(decode(&s).unwrap(), b"alice.jones");
    assert!(!s.contains('~'));
}
