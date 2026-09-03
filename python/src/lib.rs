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

use base65t::Error;

create_exception!(
    base65t,
    Base65tDecodeError,
    PyValueError,
    "Raised by decode() on malformed input.\n\n\
     `code` is one of the nine conditions of specification section 10.4, as\n\
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
/// stream is printable ASCII; nothing here *returns* a
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
/// There is no profile either: the output alphabet is the 66 characters of
/// RFC 3986 *unreserved*, whatever the input (§7).
///
/// `bytes` and not `str` on the way out, although the output is printable
/// ASCII: the return type says what the format guarantees, and §3 guarantees
/// octets.
#[pyfunction]
#[pyo3(text_signature = "(data, /)")]
fn encode<'py>(py: Python<'py>, data: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyBytes>> {
    let data = byte_argument(data, "encode() expects bytes, bytearray or str")?;
    // The encoder touches no Python object, so other threads may run while it
    // works. That matters: this is the call a caller makes on a whole file.
    let out = py.detach(|| base65t::encode(&data));
    Ok(PyBytes::new(py, &out))
}

/// Base64URL and nothing else, whatever the input looks like (§9.3, §14).
///
/// Not a mode of the format but the way out of it: for a caller carrying a
/// secret who wants no part of it left in the clear, and for one talking to
/// something that only speaks base64url. The output is ordinary unpadded
/// base64url and any base64 decoder reads it.
#[pyfunction]
#[pyo3(text_signature = "(data, /)")]
fn encode_base64url<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyBytes>> {
    let data = byte_argument(data, "encode_base64url() expects bytes, bytearray or str")?;
    let out = py.detach(|| base65t::encode_base64url(&data));
    Ok(PyBytes::new(py, &out))
}

macro_rules! decoder {
    ($name:ident, $inner:path, $doc:expr) => {
        #[doc = $doc]
        #[pyfunction]
        #[pyo3(text_signature = "(stream, /)")]
        fn $name(py: Python<'_>, stream: &Bound<'_, PyAny>) -> PyResult<Decoded> {
            let stream = byte_argument(stream, concat!(stringify!($name), "() expects bytes, bytearray or str"))?;
            match py.detach(|| $inner(&stream)) {
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
     It takes the stream and nothing else: the alphabet variant and the \
     padding come out of the stream and are reported back on the result \
     (§0.3). An \
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
    // The output alphabet, as the specification's head states it: 66
    // characters, and there is no setting that changes them (§7).
    m.add(
        "ALPHABET",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~",
    )?;

    m.add(
        "__all__",
        vec![
            "encode",
            "encode_base64url",
            "decode",
            "decode_url_strict",
            "Decoded",
            "Base65tDecodeError",
            "ALPHABET",
            "BLOCK_BYTES",
            "SPEC_VERSION",
            "__version__",
        ],
    )?;
    Ok(())
}
