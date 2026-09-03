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

use base65t::{Error, Profile};

create_exception!(
    base65t,
    Base65tDecodeError,
    PyValueError,
    "Raised by decode() on malformed input.\n\n\
     `code` is one of the ten conditions of specification section 10.4, as\n\
     the string the shared test vectors use."
);

fn decode_error(py: Python<'_>, err: Error) -> PyErr {
    let e = Base65tDecodeError::new_err(err.code());
    if let Ok(obj) = e.value(py).setattr("code", err.code()) {
        let _ = obj;
    }
    e
}

/// `bytes` or `bytearray` in, a copy out. A `str` is accepted because the
/// stream is printable ASCII under both profiles; nothing here *returns* a
/// `str`, because §3 calls the output an octet stream and a decoded payload is
/// arbitrary bytes.
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
        other => Err(PyValueError::new_err(format!(
            "profile is 'U' or 'T', not {other:?}"
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
}

#[pymethods]
impl Decoded {
    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "Decoded(bytes={} bytes, alphabet_seen={:?}, padding_seen={})",
            self.bytes.bind(py).as_bytes().len(),
            self.alphabet_seen,
            self.padding_seen
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
    }
}

/// Encode bytes. Returns the octet stream of section 3 as `bytes`.
///
/// Accepts `bytes`, `bytearray` or `str`, and always succeeds: every byte
/// sequence has an encoding, including the empty one.
///
/// There is no mode to pick and no preset to name, which is the design and not
/// an omission (§0.1): a caller who has to choose has to know what the choices
/// mean before encoding a byte, and a caller who is unsure writes base64. The
/// encoder is a fixed mapping over blocks of forty-eight bytes (§4).
///
/// `profile` is not such a choice: it is a statement about the container the
/// stream has to survive, and it cannot be derived from the stream (§7.2).
///
/// `bytes` and not `str` on the way out, although both profiles produce
/// printable ASCII: the return type says what the format guarantees, and §3
/// guarantees octets.
#[pyfunction]
#[pyo3(signature = (data, /, profile = "U"))]
#[pyo3(text_signature = "(data, /, profile='U')")]
fn encode<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
    profile: &str,
) -> PyResult<Bound<'py, PyBytes>> {
    let data = byte_argument(data, "encode() expects bytes, bytearray or str")?;
    let profile = profile_of(profile)?;
    // The encoder touches no Python object, so other threads may run while it
    // works. That matters: this is the call a caller makes on a whole file.
    let out = py.detach(|| base65t::encode_with(&data, profile));
    Ok(PyBytes::new(py, &out))
}

/// Base64URL and nothing else, whatever the input looks like (§9.3, §14).
///
/// Not a mode of the format but the way out of it: for a caller carrying a
/// secret who wants no part of it left in the clear, and for one talking to
/// something that only speaks base64url. The output is ordinary unpadded
/// base64url and any base64 decoder reads it. `profile` is accepted and
/// ignored, because there are no literals for it to constrain.
#[pyfunction]
#[pyo3(signature = (data, /, profile = "U"))]
#[pyo3(text_signature = "(data, /, profile='U')")]
fn encode_base64url<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
    profile: &str,
) -> PyResult<Bound<'py, PyBytes>> {
    let data = byte_argument(data, "encode_base64url() expects bytes, bytearray or str")?;
    profile_of(profile)?;
    let out = py.detach(|| base65t::encode_base64url(&data));
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
    "Decode a stream.\n\n\
     The profile is the only parameter: the alphabet variant and the padding \
     come out of the stream and are reported back on the result (§0.3). An \
     attacker who controls the stream controls both (§14), so where the \
     alphabet is fixed, use `decode_url_strict`."
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
    m.add_function(wrap_pyfunction!(encode_base64url, m)?)?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    m.add_function(wrap_pyfunction!(decode_url_strict, m)?)?;
    m.add_class::<Decoded>()?;
    m.add("Base65tDecodeError", m.py().get_type::<Base65tDecodeError>())?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("SPEC_VERSION", "0.4")?;

    // The constants the specification fixes, so that tooling has one source
    // for them rather than a transcribed copy.
    m.add("BLOCK_BYTES", base65t::BLOCK_BYTES)?;
    m.add("MASK_CHARS", base65t::MASK_CHARS)?;
    m.add("PROFILES", vec!["U", "T"])?;

    m.add(
        "__all__",
        vec![
            "encode",
            "encode_base64url",
            "decode",
            "decode_url_strict",
            "Decoded",
            "Base65tDecodeError",
            "PROFILES",
            "BLOCK_BYTES",
            "MASK_CHARS",
            "SPEC_VERSION",
            "__version__",
        ],
    )?;
    Ok(())
}
