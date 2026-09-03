#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""§16.5: the containers, against real parsers.

§7.1 proves from the ABNF that every character base65t writes is a
`cookie-octet`. This checks the weaker, empirical thing §16 separates out:
whether parsers actually behave that way. They are Python's -- one set of
parsers, not all of them, and the file says which.

**The negative control is classic base64**, and it is the point of the file.
That base65t's output survives a URL is only interesting if something
comparable does not, and classic base64 is exactly that comparison: same data,
same length, `+` and `/` and `=` instead of this alphabet. Every check below
runs both, and the pair is what shows the alphabet doing the work rather than
the output merely being ASCII.

    python3 conformance/test_containers.py
"""

import base64
import http.cookies
import json
import pathlib
import sys
import urllib.parse

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import reference as base65t  # noqa: E402

# Long enough to be a raw block and made only of admitted bytes, so its own
# characters stand in the output (§9.0). A stream that was pure base64 would
# tell us only that base64 goes through a URL, which nobody doubts.
SAMPLE = b"session-eu-central-1.frankfurt~alice.jones-2026"

# Chosen so that classic base64 of it contains `+`, `/` and `=` -- otherwise
# the control is vacuous.
CONTROL_INPUT = bytes([0xFB, 0xEF, 0xBE, 0x00, 0x3F, 0xF0, 0xFB, 0xFF, 0xEF, 0xFF])

failures = []


def check(name: str, ok: bool, detail: str = "") -> None:
    if ok:
        print(f"  ok   {name}")
    else:
        print(f"  FAIL {name} {detail}")
        failures.append(name)


def main() -> int:
    s = base65t.encode(SAMPLE).decode("ascii")
    control = base64.b64encode(CONTROL_INPUT).decode("ascii")
    print(f"base65t:        {s}")
    print(f"classic base64: {control}\n")
    assert s.startswith("~~"), "the sample must be a raw block"
    assert any(c in control for c in "+/="), "the control must exercise +, / and ="

    print("URL query (urllib.parse)")
    check("survives quote() unchanged", urllib.parse.quote(s, safe="") == s)
    check("round-trips through parse_qs", urllib.parse.parse_qs(f"v={s}")["v"] == [s])
    check(
        "classic base64 does need escaping",
        urllib.parse.quote(control, safe="") != control,
        urllib.parse.quote(control, safe=""),
    )

    print("\nCookie value (http.cookies)")
    jar = http.cookies.SimpleCookie()
    jar["sid"] = s
    header = jar.output(header="Set-Cookie:").strip()
    check("not quoted by the serialiser", f"sid={s}" in header, header)
    back = http.cookies.SimpleCookie()
    back.load(f"sid={s}")
    check("round-trips through a cookie parser", back["sid"].value == s)
    ctrl_jar = http.cookies.SimpleCookie()
    ctrl_jar["sid"] = control
    check(
        "classic base64 is quoted by the same serialiser",
        f"sid={control}" not in ctrl_jar.output(header="Set-Cookie:"),
        ctrl_jar.output(header="Set-Cookie:").strip(),
    )

    print("\nJSON string (json)")
    check("needs no escaping in JSON", len(json.dumps(s)) == len(s) + 2, json.dumps(s))
    check("round-trips through JSON", json.loads(json.dumps(s)) == s)

    print("\nFile name, log line, key=value")
    check("no path separator", "/" not in s and "\\" not in s)
    check("classic base64 may have one", "/" in control)
    check("no whitespace, so a log line splits on it", not any(c.isspace() for c in s))
    check("no `=`, so a key=value field has one", "=" not in s)
    check("classic base64 has `=`", "=" in control)

    print("\nAnd the stream still decodes to the input")
    check("decodes", base65t.decode(s.encode()).bytes == SAMPLE)

    print()
    if failures:
        print(f"{len(failures)} container checks failed: {', '.join(failures)}")
        return 1
    print("all container checks passed (Python's parsers, not every parser)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
