#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Conformance point 6 of §16: the containers, against real parsers.

§7.1 proves from the ABNF that every character of profile U is a
`cookie-octet`. This checks the weaker, empirical thing the section separates
out: whether parsers actually behave that way. They are Python's -- one set of
parsers, not all of them, and the file says which.

The negative controls matter as much as the positive ones. Profile T is *not*
URL-safe and §7 says so; a test that only showed U passing would not have
established that the profile distinction is real.

    python3 conformance/test_containers.py
"""

import http.cookies
import json
import pathlib
import sys
import urllib.parse

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import reference as base65t  # noqa: E402

# Two samples, because one would make the negative control vacuous: text that
# is entirely unreserved encodes identically under U and T, and then "T is not
# URL-safe" is never exercised. The second sample uses characters T admits and
# U does not -- a space, a slash, a comma, an equals sign.
SAMPLE_U = b"\xde\xad\xbe\xefsession-eu-central-1.frankfurt~alice"
SAMPLE_T = b"\xde\xad\xbe\xefGET /api/v2?id=42, status=ok; done"

failures = []


def check(name: str, ok: bool, detail: str = "") -> None:
    if ok:
        print(f"  ok   {name}")
    else:
        print(f"  FAIL {name} {detail}")
        failures.append(name)


def main() -> int:
    u = base65t.encode_dense(SAMPLE_U, "U").decode("ascii")
    t = base65t.encode_dense(SAMPLE_T, "T").decode("ascii")
    print(f"profile U: {u}\nprofile T: {t}\n")
    assert any(c in t for c in " /?=,;"), "the T sample must exercise T"

    print("URL query (urllib.parse)")
    check("U survives quote() unchanged", urllib.parse.quote(u, safe="") == u)
    check(
        "U round-trips through parse_qs",
        urllib.parse.parse_qs(f"v={u}")["v"] == [u],
    )
    check(
        "T does need escaping, as §7 says",
        urllib.parse.quote(t, safe="") != t,
        urllib.parse.quote(t, safe=""),
    )

    print("\nCookie value (http.cookies)")
    jar = http.cookies.SimpleCookie()
    jar["sid"] = u
    header = jar.output(header="Set-Cookie:").strip()
    check("U is not quoted by the serialiser", f"sid={u}" in header, header)
    back = http.cookies.SimpleCookie()
    back.load(f"sid={u}")
    check("U round-trips through a cookie parser", back["sid"].value == u)

    print("\nJSON string (json)")
    for label, s in (("U", u), ("T", t)):
        check(
            f"{label} needs no escaping in JSON",
            len(json.dumps(s)) == len(s) + 2,
            json.dumps(s),
        )
        check(f"{label} round-trips through JSON", json.loads(json.dumps(s)) == s)

    print("\nFile name and log line")
    check("U has no path separator", "/" not in u and "\\" not in u)
    check("T may well have one, which is why it is not a file name", "/" in t)
    check("U has no whitespace, so a log line can be split on it",
          not any(c.isspace() for c in u))
    # Profile T admits 0x20 (§7). §0.1 recommends `legible` with T for a log
    # field, and this is the caveat that recommendation needs: a
    # whitespace-delimited log format has to quote it, a key=value one does
    # not. Asserted rather than fixed, because it is a property of the profile.
    check("T may contain a space, and a log format must expect it", " " in t)

    print("\nAnd every stream still decodes to the input")
    for label, s, want in (("U", u, SAMPLE_U), ("T", t, SAMPLE_T)):
        check(f"{label} decodes", base65t.decode(s.encode(), label).bytes == want)

    print()
    if failures:
        print(f"{len(failures)} container checks failed: {', '.join(failures)}")
        return 1
    print("all container checks passed (Python's parsers, not every parser)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
