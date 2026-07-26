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

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
