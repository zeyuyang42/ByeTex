#!/usr/bin/env python3
"""Unit tests for the acceptance-gate comparator (scripts/acceptance_check.py).

Run: python3 scripts/tests/acceptance_check_test.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import acceptance_check as ac  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


BASELINE = {"known_pass": ["p1", "p2", "p3"], "known_fail": ["f1", "f2"]}


def verdicts(text):
    return ac.parse_verdicts(text.splitlines())


# Parsing: handles the four verdict prefixes and the trailing ":" on FAIL lines.
v = verdicts("PASS p1\nBYETEX_FAIL f1: unknown variable\nINPUT_BROKEN x: y\nSKIP s\n")
check(v.get("PASS") == {"p1"}, "parses PASS ids")
check(v.get("BYETEX_FAIL") == {"f1"}, "parses BYETEX_FAIL ids (strips trailing ':')")
check(v.get("SKIP") == {"s"}, "parses SKIP ids")

# Clean run: all known_pass pass, known_fail still fail → no regression.
clean = verdicts("PASS p1\nPASS p2\nPASS p3\nBYETEX_FAIL f1:\nBYETEX_FAIL f2:\n")
reg, fixed, new = ac.evaluate(clean, BASELINE)
check(reg == [], "no regression when known_pass all pass")
check(fixed == [] and new == [], "nothing fixed/new in the steady state")

# Regression: a known_pass paper now BYETEX_FAILs.
r = verdicts("PASS p1\nBYETEX_FAIL p2:\nPASS p3\n")
reg, _, _ = ac.evaluate(r, BASELINE)
check(reg == ["p2"], "known_pass paper that now fails is a REGRESSION")

# Fixed: a known_fail paper now passes (reported, not a failure).
f = verdicts("PASS p1\nPASS p2\nPASS p3\nPASS f1\nBYETEX_FAIL f2:\n")
reg, fixed, _ = ac.evaluate(f, BASELINE)
check(reg == [] and fixed == ["f1"], "known_fail paper now passing is FIXED (not a regression)")

# New fail: a paper not in the baseline now fails (reported, not a regression).
n = verdicts("PASS p1\nPASS p2\nPASS p3\nBYETEX_FAIL newpaper:\n")
reg, _, new = ac.evaluate(n, BASELINE)
check(reg == [] and new == ["newpaper"], "unknown paper failing is NEW_FAIL (not a regression)")

# Payload-robust: a known_pass paper simply absent (no payload) is NOT a regression.
absent = verdicts("PASS p1\nPASS p2\n")  # p3 missing entirely
reg, _, _ = ac.evaluate(absent, BASELINE)
check(reg == [], "absent known_pass paper (no payload) is not a regression")

if fails:
    print(f"\nTEST FAILED ({len(fails)} assertion(s))")
    sys.exit(1)
print("\nTEST PASSED")
