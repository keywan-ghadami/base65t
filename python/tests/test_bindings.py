# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""What the binding layer is responsible for, and nothing else.

The encoder and decoder are the Rust crate's and are tested there. What can
only go wrong here is the layer: argument types, the profile names, the fields
of the result, and the error code that comes back. So that is what this checks.
"""

import pytest

import base65t


def test_the_parameterless_call_is_profile_u():
    assert base65t.encode(b"alice.jones") == b"~~alice.jones"
    assert base65t.encode(b"alice.jones", "U") == b"~~alice.jones"


@pytest.mark.parametrize("arg", [b"abc", bytearray(b"abc"), "abc"])
def test_bytes_bytearray_and_str_are_all_accepted(arg):
    assert base65t.decode(base65t.encode(arg)).bytes == b"abc"


@pytest.mark.parametrize("arg", [123, ["a"], None, 3.5])
def test_a_sequence_of_integers_is_a_type_error_not_an_input(arg):
    with pytest.raises(TypeError):
        base65t.encode(arg)


def test_every_profile_name_is_wired():
    data = bytes.fromhex("deadbeef") + b"session-eu-central"
    for profile in base65t.PROFILES:
        out = base65t.encode(data, profile)
        assert base65t.decode(out, profile).bytes == data, profile


def test_an_unknown_profile_is_a_value_error():
    with pytest.raises(ValueError):
        base65t.encode(b"x", "V")
    # Profile B was a profile until v0.4 and is not one any more (§7).
    with pytest.raises(ValueError):
        base65t.encode(b"x", "B")


def test_the_base64url_entry_point_is_not_a_mode():
    """§9.3, §14: ordinary unpadded base64url, and no literal in it."""
    data = b"alice.jones"
    out = base65t.encode_base64url(data)
    assert out == b"YWxpY2Uuam9uZXM"
    assert b"~" not in out
    assert base65t.decode(out).bytes == data
    # And it is not what `encode` writes, which is the point of having both.
    assert base65t.encode(data) != out


def test_the_result_carries_what_the_stream_chose():
    d = base65t.decode(b"YWxpY2U=")
    assert d.bytes == b"alice"
    assert d.padding_seen is True
    assert d.alphabet_seen == "none"

    d = base65t.decode(b"PDw_Pz8-Pg")
    assert d.alphabet_seen == "url"
    assert base65t.decode(b"PDw/Pz8+Pg").alphabet_seen == "classic"


def test_the_strict_entry_point_is_separate():
    with pytest.raises(base65t.Base65tDecodeError) as e:
        base65t.decode_url_strict(b"PDw/Pz8+Pg")
    assert e.value.code == "E_NON_URL_ALPHABET"
    assert base65t.decode_url_strict(b"PDw_Pz8-Pg").bytes == b"<<???>>"


@pytest.mark.parametrize(
    "stream,code",
    [
        (b"abcde", "E_ALIGN"),
        (b"~", "E_TRAILING_TILDE"),
        (b"YW*j", "E_CHARSET"),
        (b"YWxp==", "E_PADDING"),
        (b"YWxpY2V", "E_NONZERO_TAIL"),
        (b"~~a b", "E_PROFILE"),
        (b"~Aabc", "E_RESERVED"),
        (b"PDw_Pz8+Pg", "E_MIXED_ALPHABET"),
    ],
)
def test_the_error_carries_the_code_the_vectors_use(stream, code):
    with pytest.raises(base65t.Base65tDecodeError) as e:
        base65t.decode(stream)
    assert e.value.code == code
    assert isinstance(e.value, ValueError)


def test_the_constants_come_from_the_crate():
    assert base65t.BLOCK_BYTES == 48
    assert base65t.PROFILES == ["U", "T"]
    assert base65t.SPEC_VERSION == "0.4"


def test_nothing_the_old_api_had_is_still_exported():
    """v0.4 removed presets, framing and profile B (§8, §9.3, §7).

    A binding that kept a name alive after the format dropped it is worse than
    one that never had it: the call keeps working and means something else.
    """
    for gone in (
        "PRESETS",
        "decode_plain",
        "decode_framed",
        "classify",
        "FRAME_BYTES",
        "MAX_LITERAL",
        "WINDOW_BYTES",
        "MASK_CHARS",
    ):
        assert not hasattr(base65t, gone), gone
