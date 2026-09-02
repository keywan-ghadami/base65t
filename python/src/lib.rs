// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Python bindings for Base65t, built with PyO3 and packaged by maturin.
//!
//! This is not an implementation of the format: it is a thin layer over the
//! `base65t` crate, so what Python runs is byte for byte the same encoder and
//! decoder a Rust caller gets. The layer converts argument types, releases the
//! GIL around the call, and turns a decode error into a Python exception
//! carrying the same code the specification and the shared vectors use.
//!
//! The independent implementation lives in `conformance/reference.py` and is
//! not this. It exists to disagree with the Rust if the document allows two
//! readings (§16.3); a binding cannot, by construction.

use pyo3::create_exception;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};

use base65t::{Error, Framing, Preset, Profile};

create_exception!(
    base65t,
    Base65tDecodeError,
    PyValueError,
    "Raised by decode() on malformed input.\n\n\
     `code` is one of the twelve conditions of specification section 10.4, as\n\
     the string the shared test vectors use."
);

fn decode_error(py: Python<'_>, err: Error) -> PyErr {
    let e = Base65tDecodeError::new_err(err.code());
    if let Ok(obj) = e.value(py).setattr("code", err.code()) {
        let _ = obj;
    }
    e
}

/// `bytes` or `bytearray` in, a copy out. A `str` is accepted where the stream
/// is text, which it is under profiles U and T; profile B is octets and §3
/// says so, which is why nothing here returns a `str`.
fn byte_argument(obj: &Bound<'_, PyAny>, what: &str) -> PyResult<Vec<u8>> {
    // Matched by type rather than by extraction, so that a sequence that
    // merely happens to hold small integers -- a list, a tuple -- is a
    // TypeError and not silently an input.
    if let Ok(b) = obj.cast::<PyBytes>() {
        return Ok(b.as_bytes().to_vec());
    }
    if let Ok(b) = obj.cast::<PyByteArray>() {
        return Ok(b.to_vec());
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(s.into_bytes());
    }
    Err(PyTypeError::new_err(what.to_string()))
}

fn profile_of(name: &str) -> PyResult<Profile> {
    match name {
        "U" => Ok(Profile::U),
        "T" => Ok(Profile::T),
        "B" => Ok(Profile::B),
        other => Err(PyValueError::new_err(format!(
            "profile is 'U', 'T' or 'B', not {other:?}"
        ))),
    }
}

fn preset_of(name: &str) -> PyResult<Preset> {
    match name {
        "dense" => Ok(Preset::Dense),
        "dense-fast" => Ok(Preset::DenseFast),
        "legible" => Ok(Preset::Legible),
        "canonical" => Ok(Preset::Canonical),
        "opaque" => Ok(Preset::Opaque),
        "framed" => Ok(Preset::Framed),
        other => Err(PyValueError::new_err(format!(
            "preset is one of dense, dense-fast, legible, canonical, opaque, framed, \
             not {other:?}"
        ))),
    }
}

/// What a decode found, which section 5.5 makes part of the result rather than
/// an option: permissiveness that cannot be inspected cannot be validated.
#[pyclass(module = "base65t", frozen, get_all)]
struct Decoded {
    /// The decoded payload.
    bytes: Py<PyBytes>,
    /// `"none"`, `"url"` or `"classic"` — which alphabet variant the stream's
    /// alphabet positions used. Literal payloads never count (Rule A, §5.4).
    alphabet_seen: String,
    /// Whether any base64 segment carried `=` (§5.3). An encoder never writes
    /// it.
    padding_seen: bool,
    /// `"plain"` or `"framed"`, decided by the stream itself (Rule F, §5.6).
    framing_seen: String,
}

#[pymethods]
impl Decoded {
    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "Decoded(bytes={} bytes, alphabet_seen={:?}, padding_seen={}, framing_seen={:?})",
            self.bytes.bind(py).as_bytes().len(),
            self.alphabet_seen,
            self.padding_seen,
            self.framing_seen
        )
    }
}

fn wrap(py: Python<'_>, d: base65t::Decoded) -> Decoded {
    Decoded {
        bytes: PyBytes::new(py, &d.bytes).unbind(),
        alphabet_seen: match d.alphabet_seen {
            base65t::AlphabetSeen::None => "none",
            base65t::AlphabetSeen::Url => "url",
            base65t::AlphabetSeen::Classic => "classic",
        }
        .to_string(),
        padding_seen: d.padding_seen,
        framing_seen: match d.framing_seen {
            Framing::Plain => "plain",
            Framing::Framed => "framed",
        }
        .to_string(),
    }
}

/// Encode bytes. Returns the octet stream of section 3 as `bytes`.
///
/// Accepts `bytes`, `bytearray` or `str`, and always succeeds: every byte
/// sequence has an encoding, including the empty one. `preset` and `profile`
/// default to what a call without arguments must give (§9.3): `dense` and
/// profile U.
///
/// `bytes` and not `str` on the way out, at every preset: under profiles U and
/// T every octet is printable ASCII and `.decode("ascii")` is free, but under
/// profile B it is not text at all, and an API that pretended otherwise would
/// be lying at exactly one of its three settings.
///
/// `threads` is a performance knob and nothing else: every value produces the
/// same stream, because §9.2.1 is a rule about local bytes and the parallel
/// encoder cuts where no segment spans the cut. `0` asks for one worker per
/// available core. It applies to `dense` only -- the other presets optimise
/// over the whole input by definition -- and inputs below a megabyte encode on
/// the calling thread whatever it says.
#[pyfunction]
#[pyo3(signature = (data, /, preset = "dense", profile = "U", threads = 1))]
#[pyo3(text_signature = "(data, /, preset='dense', profile='U', threads=1)")]
fn encode<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
    preset: &str,
    profile: &str,
    threads: usize,
) -> PyResult<Bound<'py, PyBytes>> {
    let data = byte_argument(data, "encode() expects bytes, bytearray or str")?;
    let preset_name = preset;
    let preset = preset_of(preset)?;
    let profile = profile_of(profile)?;
    // The encoder touches no Python object, so other threads may run while it
    // works -- and so the workers it starts are free of the GIL too. That
    // matters: this is the call a caller makes on a whole file.
    let out = py.detach(|| {
        if threads != 1 && preset_name == "dense" {
            base65t::encode_parallel(&data, profile, threads)
        } else {
            base65t::encode_with(&data, preset, profile)
        }
    });
    Ok(PyBytes::new(py, &out))
}

macro_rules! decoder {
    ($name:ident, $inner:path, $doc:expr) => {
        #[doc = $doc]
        #[pyfunction]
        #[pyo3(signature = (stream, /, profile = "U"))]
        #[pyo3(text_signature = "(stream, /, profile='U')")]
        fn $name(py: Python<'_>, stream: &Bound<'_, PyAny>, profile: &str) -> PyResult<Decoded> {
            let stream = byte_argument(stream, concat!(stringify!($name), "() expects bytes, bytearray or str"))?;
            let profile = profile_of(profile)?;
            match py.detach(|| $inner(&stream, profile)) {
                Ok(d) => Ok(wrap(py, d)),
                Err(e) => Err(decode_error(py, e)),
            }
        }
    };
}

decoder!(
    decode,
    base65t::decode,
    "Decode a stream, letting it say whether it is framed (Rule F, §5.6).\n\n\
     The profile is the only parameter: alphabet variant, padding and framing \
     come out of the stream and are reported back on the result. An attacker \
     who controls the stream controls all three (§14), so where the mode is \
     fixed, use `decode_plain` or `decode_framed` instead."
);
decoder!(
    decode_plain,
    base65t::decode_plain,
    "Decode as plain mode whatever the stream looks like. A framed stream \
     reaches `E_RESERVED_LEN` here, which is the right answer for this entry \
     point (§10.2)."
);
decoder!(
    decode_framed,
    base65t::decode_framed,
    "Decode as framed mode whatever the stream looks like. A plain stream \
     reaches `E_FRAME_SYNC` here."
);
decoder!(
    decode_url_strict,
    base65t::decode_url_strict,
    "Like `decode`, but a `+` or `/` at an alphabet position ends it with \
     `E_NON_URL_ALPHABET` (§5.5)."
);

#[pymodule]
#[pyo3(name = "base65t")]
fn base65t_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    m.add_function(wrap_pyfunction!(decode_plain, m)?)?;
    m.add_function(wrap_pyfunction!(decode_framed, m)?)?;
    m.add_function(wrap_pyfunction!(decode_url_strict, m)?)?;
    m.add_class::<Decoded>()?;
    m.add("Base65tDecodeError", m.py().get_type::<Base65tDecodeError>())?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("SPEC_VERSION", "0.2")?;

    // The constants the specification fixes, so that tooling has one source
    // for them rather than a transcribed copy.
    m.add("MAX_LITERAL", base65t::MAX_LITERAL)?;
    m.add("MAX_FRAME_BODY", base65t::MAX_FRAME_BODY)?;
    m.add("MIN_LITERAL", base65t::MIN_LITERAL)?;
    m.add("FAST_WINDOW", base65t::FAST_WINDOW)?;
    m.add("FAST_SAMPLE", base65t::FAST_SAMPLE)?;
    m.add("FRAME_BYTES", base65t::FRAME_BYTES)?;
    m.add(
        "PRESETS",
        vec!["dense", "dense-fast", "legible", "canonical", "opaque", "framed"],
    )?;
    m.add("PROFILES", vec!["U", "T", "B"])?;

    m.add(
        "__all__",
        vec![
            "encode",
            "decode",
            "decode_plain",
            "decode_framed",
            "decode_url_strict",
            "Decoded",
            "Base65tDecodeError",
            "PRESETS",
            "PROFILES",
            "MAX_LITERAL",
            "MAX_FRAME_BODY",
            "MIN_LITERAL",
            "FAST_WINDOW",
            "FAST_SAMPLE",
            "FRAME_BYTES",
            "SPEC_VERSION",
            "__version__",
        ],
    )?;
    Ok(())
}
