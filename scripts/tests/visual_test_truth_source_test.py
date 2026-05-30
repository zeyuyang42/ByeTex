#!/usr/bin/env python3
"""Unit test for visual_test.resolve_truth_source — the truth-PDF source
selection logic (arXiv download vs local tectonic render vs auto).

Run: uv run --with requests --with Pillow python scripts/tests/visual_test_truth_source_test.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import visual_test as vt  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


# Explicit choices are honored regardless of context.
check(
    vt.resolve_truth_source("arxiv", "2605.1", no_download=False, tectonic_ok=True) == "arxiv",
    "explicit arxiv -> arxiv",
)
check(
    vt.resolve_truth_source("tectonic", "2605.1", no_download=False, tectonic_ok=True) == "tectonic",
    "explicit tectonic (available) -> tectonic",
)

# Explicit tectonic with no tectonic installed is an error — never silently
# fall back to a different truth source the user didn't ask for.
try:
    vt.resolve_truth_source("tectonic", "2605.1", no_download=False, tectonic_ok=False)
    check(False, "explicit tectonic (unavailable) should raise ValueError")
except ValueError:
    check(True, "explicit tectonic (unavailable) raises ValueError")

# auto: prefer the arXiv canonical PDF when downloads are allowed.
check(
    vt.resolve_truth_source("auto", "2605.1", no_download=False, tectonic_ok=True) == "arxiv",
    "auto + downloads allowed -> arxiv",
)
# auto: when downloads are disabled, render locally with tectonic if we can.
check(
    vt.resolve_truth_source("auto", "2605.1", no_download=True, tectonic_ok=True) == "tectonic",
    "auto + no-download + tectonic -> tectonic",
)
# auto: no downloads and no tectonic -> fall back to arxiv (cached PDF path).
check(
    vt.resolve_truth_source("auto", "2605.1", no_download=True, tectonic_ok=False) == "arxiv",
    "auto + no-download + no-tectonic -> arxiv (cached fallback)",
)

if fails:
    print(f"\nTEST FAILED ({len(fails)} assertion(s))")
    sys.exit(1)
print("\nTEST PASSED")
