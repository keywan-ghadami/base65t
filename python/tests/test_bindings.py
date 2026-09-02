# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""What the binding layer is responsible for, and nothing else.

The encoder and decoder are the Rust crate's and are tested there. What can
only go wrong here is the layer: argument types, the preset and profile names,
the fields of the result, and the error code that comes back. So that is what
this checks.
"""

import pytest

import base65t


def test_the_parameterless_default_is_dense_and_profile_u():
    assert base65t.encode(b"alice.jones") == b"~Lalice.jones"
    assert base65t.encode(b"alice.jones", "dense", "U") == b"~Lalice.jones"


@pytest.mark.parametrize("arg", [b"abc", bytearray(b"abc"), "abc"])
def test_bytes_bytearray_and_str_are_all_accepted(arg):
    assert base65t.decode(base65t.encode(arg)).bytes == b"abc"


@pytest.mark.parametrize("arg", [123, ["a"], None, 3.5])
def test_a_sequence_of_integers_is_a_type_error_not_an_input(arg):
    with pytest.raises(TypeError):
        base65t.encode(arg)


def test_every_preset_and_profile_name_is_wired():
    data = bytes.fromhex("deadbeef") + b"session-eu-central"
    for preset in base65t.PRESETS:
        for profile in base65t.PROFILES:
            out = base65t.encode(data, preset, profile)
            assert base65t.decode(out, profile).bytes == data, (preset, profile)


@pytest.mark.parametrize(
    "name,bad", [("preset", "densest"), ("profile", "V")]
)
def test_an_unknown_name_is_a_value_error(name, bad):
    kwargs = {name: bad}
    with pytest.raises(ValueError):
        base65t.encode(b"x", **kwargs)


def test_the_result_carries_what_the_stream_chose():
    d = base65t.decode(b"YWxpY2U=")
    assert d.bytes == b"alice"
    assert d.padding_seen is True
    assert d.framing_seen == "plain"
    assert d.alphabet_seen == "none"

    d = base65t.decode(b"PDw_Pz8-Pg")
    assert d.alphabet_seen == "url"
    assert base65t.decode(b"PDw/Pz8+Pg").alphabet_seen == "classic"

    framed = base65t.encode(b"alice.jones and a longer tail", "framed")
    assert base65t.decode(framed).framing_seen == "framed"


def test_the_entry_points_are_separate():
    framed = base65t.encode(b"alice.jones and a longer tail", "framed")
    with pytest.raises(base65t.Base65tDecodeError) as e:
        base65t.decode_plain(framed)
    assert e.value.code == "E_RESERVED_LEN"

    with pytest.raises(base65t.Base65tDecodeError) as e:
        base65t.decode_framed(b"YWxpY2U")
    assert e.value.code == "E_FRAME_SYNC"

    with pytest.raises(base65t.Base65tDecodeError) as e:
        base65t.decode_url_strict(b"PDw/Pz8+Pg")
    assert e.value.code == "E_NON_URL_ALPHABET"


@pytest.mark.parametrize(
    "stream,code",
    [
        (b"abcde", "E_ALIGN"),
        (b"abc~", "E_TRAILING_TILDE"),
        (b"~~ab", "E_CHARSET"),
        (b"YWxp==", "E_PADDING"),
        (b"YWxpY2V", "E_NONZERO_TAIL"),
        (b"~Ca b", "E_PROFILE"),
    ],
)
def test_the_error_carries_the_code_the_vectors_use(stream, code):
    with pytest.raises(base65t.Base65tDecodeError) as e:
        base65t.decode(stream)
    assert e.value.code == code
    assert isinstance(e.value, ValueError)


def test_the_constants_come_from_the_crate():
    assert base65t.MAX_LITERAL == 4158
    assert base65t.MAX_FRAME_BODY == 262143
    assert base65t.MIN_LITERAL == 11
    assert base65t.FAST_WINDOW == 65536 and base65t.FAST_SAMPLE == 1024
    assert "dense-fast" in base65t.PRESETS
    assert base65t.FRAME_BYTES == 65536
    assert base65t.SPEC_VERSION == "0.2"


@pytest.mark.parametrize("threads", [0, 1, 2, 4, 8])
def test_threads_never_reach_the_output(threads):
    # Big enough that the parallel encoder actually splits, and mixed enough
    # that it has literals to cut at. The assertion is the whole contract:
    # the thread count is a performance knob and nothing else.
    data = (b"a-string-that-is-transportable." * 40 + bytes(range(256))) * 900
    assert len(data) > (1 << 20)
    assert base65t.encode(data, threads=threads) == base65t.encode(data)
    assert base65t.decode(base65t.encode(data, threads=threads)).bytes == data
