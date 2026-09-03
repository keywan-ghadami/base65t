#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Every §-reference in the specification, checked two ways.

A checker that only asks "does §17.6 exist" passes a reference that should
have said §17.5 -- which is what happened, and is why this file exists.
Inserting §17.5 pushed every later section down one, and a reference written
against the old numbering silently began pointing at a different section. It
still resolved. It was still wrong.

So there are two checks, and the second is the one that matters:

1. **Resolution.** Every §N.M names a heading that exists.
2. **Retargeting.** For every section number, compare the heading it names now
   with the heading it named in the last commit. Where a number changed
   meaning, report every reference to it -- because each of those was written
   about the old section and now points at the new one.

The second check needs no heuristic and has no false positives that are not
worth a look: if §17.6 used to be "Turning the check off again" and is now
"Choosing the vector width at runtime", every `§17.6` in the document is a
sentence written about a different subject.

    python3 conformance/check_references.py [git-ref]
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SPEC = pathlib.Path("docs/spec-v0.4.md")


def sections(text: str) -> dict[str, str]:
    """number -> heading title."""
    return {
        m.group(2): m.group(3).strip()
        for m in re.finditer(r"^(#{2,3}) (\d+(?:\.\d+)?)\.? *(.*)$", text, re.M)
    }


def references(text: str) -> list[tuple[int, str]]:
    """(line, number) for every §-reference that is this document's own."""
    out = []
    for m in re.finditer(r"§(\d+(?:\.\d+)?)", text):
        # "RFC 6265 §4.1.1" points at another document.
        if re.search(r"RFC \d+ *$", text[max(0, m.start() - 40) : m.start()]):
            continue
        out.append((text[: m.start()].count("\n") + 1, m.group(1)))
    return out


def at_ref(ref: str) -> str | None:
    r = subprocess.run(
        ["git", "-C", str(ROOT), "show", f"{ref}:{SPEC}"],
        capture_output=True,
        text=True,
    )
    return r.stdout if r.returncode == 0 else None


def main(argv: list[str]) -> int:
    ref = argv[0] if argv else "HEAD"
    text = (ROOT / SPEC).read_text()
    now = sections(text)
    refs = references(text)

    bad = 0
    for line, num in refs:
        if num not in now:
            print(f"  FAIL line {line}: §{num} names no section")
            bad += 1
    print(f"{len(refs)} references, {len(now)} sections, {bad} unresolved")

    before_text = at_ref(ref)
    if before_text is None:
        print(f"(no {ref}:{SPEC} to compare against; retargeting not checked)")
        return 1 if bad else 0

    before = sections(before_text)
    moved = {
        n: (before[n], now[n])
        for n in now
        if n in before and before[n] != now[n]
    }
    if not moved:
        print(f"no section changed meaning since {ref}")
        return 1 if bad else 0

    print(f"\n{len(moved)} section number(s) changed meaning since {ref}:")
    for n, (was, is_) in sorted(moved.items()):
        print(f"  §{n}: {was!r} -> {is_!r}")
        cites = [ln for ln, x in refs if x == n]
        if cites:
            print(f"    referenced at line(s) {', '.join(map(str, cites))}"
                  f" -- each was written about {was!r}")
            bad += len(cites)
        else:
            print("    not referenced anywhere, nothing to retarget")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
