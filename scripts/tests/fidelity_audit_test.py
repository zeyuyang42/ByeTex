#!/usr/bin/env python3
"""Unit tests for scripts/fidelity_audit.py helpers.

Run: python3 scripts/tests/fidelity_audit_test.py

Focus: the corpus location written into the COMMITTED artifacts must never be
an absolute path. `docs/fidelity-nonvisual-audit.{md,json}` are regenerated on
every audit run and committed, so an absolute path embeds the developer's home
directory (and username) in a public repo, over and over.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import fidelity_audit as fa  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


ROOT = Path("/repo/root")

check(
    fa.corpus_display(Path("/repo/root/corpus"), ROOT) == "corpus",
    "a corpus inside the repo renders repo-relative",
)
check(
    fa.corpus_display(Path("/repo/root/tests/fixtures/corpus"), ROOT)
    == "tests/fixtures/corpus",
    "a nested corpus keeps its repo-relative path",
)
check(
    fa.corpus_display(Path("/Users/someone/elsewhere/corpus"), ROOT) == "corpus",
    "a corpus OUTSIDE the repo degrades to its directory name",
)
check(
    not Path(fa.corpus_display(Path("/Users/someone/elsewhere/corpus"), ROOT)).is_absolute(),
    "the rendered value is never absolute",
)

# The real default must also be relative.
rendered = fa.corpus_display(fa.CORPUS_DIR)
check(not Path(rendered).is_absolute(), f"the live CORPUS_DIR renders relative (got {rendered!r})")
check(
    "/Users/" not in rendered and "/home/" not in rendered,
    f"no home directory leaks into the artifact (got {rendered!r})",
)


# ── silent-gap probes ────────────────────────────────────────────────────────
# The "silent gaps" table is the loop's step-1 measurement for picking Loop-A
# work, and it was almost entirely false positives: the scan counted constructs
# in the SOURCE and never checked whether the converter handled them, so 12 of
# the 13 tracked gaps named constructs that already work (`\textcolor` ->
# `#text(fill:)`, `\vspace` -> `#v()`, `\cmidrule` -> `table.hline()`, ...).
# Measured 2026-08-14; the biggest entry (`\vspace/\hspace`, 39 papers / 1205x)
# was entirely spurious.
#
# Every gap must carry a probe AND a negative control. A probe whose marker the
# control also produces proves nothing, and since a passing probe DELETES the
# gap from the published work list, a vacuous probe hides real work — strictly
# worse than the noise it replaces. Two first-draft probes were exactly that.

check(
    set(fa.GAP_PROBES) == set(fa.SILENT_GAPS),
    "every silent-gap label has a probe (no gap can be reported unverified)",
)
for _label, _probe in sorted(fa.GAP_PROBES.items()):
    check(len(_probe) == 3, f"probe for {_label!r} carries a negative control")
    _src, _expect, _ctrl = _probe
    check(bool(_src.strip()) and bool(_expect.strip()) and bool(_ctrl.strip()),
          f"probe for {_label!r} has LaTeX, a marker, and a control")
    # A control identical to the probe can never discriminate.
    check(_src.strip() != _ctrl.strip(),
          f"probe for {_label!r} differs from its control")
    # The marker must not be a literal substring of the control INPUT either —
    # a cheap offline guard against asserting something the control also states.
    check(_expect not in _ctrl,
          f"marker for {_label!r} is absent from its control input")

check(
    hasattr(fa, "verify_gaps"),
    "fidelity_audit exposes verify_gaps() to run the probes",
)

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
