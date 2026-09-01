#!/usr/bin/env python3
# requires: none
"""Unit test for truth_render.summarize_tectonic_failure — what the harness
reports as the REASON a truth render failed.

This exists because the reason was wrong on 3 of the corpus's 6
`truth_render_failed` papers. The old logic scanned stderr for the first line
containing font/biber/"not found" and reported it — which happily matched a
tectonic *warning*, so an operator was told the cause was a Carlito font path
when it was actually a missing "Latin Modern Math", or an `algorithm.sty` UTF-8
warning when it was an undefined control sequence. A truth render that fails for
an unreported reason is a paper the fidelity driver goes blind on and nobody can
fix.

Run: python3 scripts/tests/truth_render_error_test.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import truth_render  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


summarize = truth_render.summarize_tectonic_failure

# Verbatim stderr shapes from the corpus papers (trimmed).
MAUROVM = """\
warning: accessing absolute path `/Users/x/Library/Fonts/Carlito-Regular.ttf`; build may not be reproducible
note: this is a warning
error: oxengthesis.cls:146: Package fontspec Error: The font "Latin Modern Math" cannot be found.
error: halted on potentially-recoverable error as specified
"""

CALPOLY = """\
warning: algorithm.sty:11: Invalid UTF-8 byte or sequence at line 11 replaced by U+FFFD.
warning: chapters/ch1.tex:3: Invalid UTF-8 byte or sequence replaced by U+FFFD.
error: main.tex:204: Undefined control sequence
error: halted on potentially-recoverable error as specified
"""

PELEGS = """\
warning: kpfonts.sty:157:
warning: commath.sty:6: Invalid UTF-8 byte or sequence at line 6 replaced by U+FFFD.
error: figures/cover/cover:30: Package svg Error: File `tapir_svg-tex.pdf' is missing.
error: halted on potentially-recoverable error as specified
"""

KAOBOOK = """\
error: main.tex:8: ! LaTeX Error: File `kaobook.cls' not found.
error: halted on potentially-recoverable error as specified
"""

# 1. A real `error:` line beats any warning, even one matching the old keywords.
check("Latin Modern Math" in summarize(MAUROVM), "maurovm: reports the missing Latin Modern Math font")
check("Carlito" not in summarize(MAUROVM).split(" | ")[0], "maurovm: does NOT lead with the Carlito warning")
check("Undefined control sequence" in summarize(CALPOLY), "calpoly: reports the undefined control sequence")
check(
    "Invalid UTF-8" not in summarize(CALPOLY).split(" | ")[0],
    "calpoly: does NOT lead with the UTF-8 warning",
)
check("tapir_svg-tex.pdf" in summarize(PELEGS), "pelegs: reports the missing svg include")
check("kpfonts" not in summarize(PELEGS).split(" | ")[0], "pelegs: does NOT lead with the kpfonts warning")

# 2. The cases that were already right stay right.
check("kaobook.cls" in summarize(KAOBOOK), "kaobook: still reports the missing class")

# 3. Tectonic's own boilerplate is never the reported cause — it says nothing
#    about WHY, and it is present on essentially every failure.
BOILERPLATE_ONLY = """\
warning: something benign about a font
error: halted on potentially-recoverable error as specified
"""
lead = summarize(BOILERPLATE_ONLY).split(" | ")[0]
check("halted on potentially-recoverable" not in lead, "boilerplate-only: does not lead with `halted on ...`")

# 4. No error line at all → do not dress a warning up as the cause.
WARNINGS_ONLY = """\
warning: accessing absolute path `/Users/x/Library/Fonts/Carlito-Regular.ttf`; build may not be reproducible
warning: some other note
"""
check(
    not summarize(WARNINGS_ONLY).startswith("warning:"),
    "warnings-only: does not present a warning as the failure reason",
)

# 5. Empty stderr still yields an attributable answer. This is only reached on a
#    failure, and tectonic can exit 0 without producing the expected PDF — an
#    empty return would leave the caller printing "render failed: " and storing
#    "" as the reason, which is the unattributed failure this guards against.
check(summarize("") == truth_render.NO_CAUSE_FOUND, "empty stderr: says so rather than returning nothing")
check(summarize("   \n  ") == truth_render.NO_CAUSE_FOUND, "whitespace-only stderr: same")

# 6. The raw tail is still carried, so nothing is lost.
check("halted on potentially-recoverable" in summarize(MAUROVM), "maurovm: raw stderr tail is retained")

# 7. A sub-tool's error reached through tectonic's wrapper lines. The wrappers
#    are themselves `error:` lines that end in a colon and defer to what follows,
#    and biber does not use tectonic's prefix at all — so the cause is only
#    reachable by looking past both.
CALPOLY_BIBER = """\
error: the external tool exited with an error code; its stdout was:
ERROR - Error: Found biblatex control file version 3.8, expected version 3.11.
error: its stderr was:
error: the external tool exited with error code 2
"""
lead = summarize(CALPOLY_BIBER).split(" | ")[0]
check("biblatex control file version" in lead, "calpoly/biber: reports the biblatex version mismatch")
check("external tool exited" not in lead, "calpoly/biber: does NOT lead with the wrapper line")

# 8. A warning whose text spills onto following lines must not supply the cause
#    through those continuation lines — prefix-only suppression would let an
#    unprefixed `... Error: ...` continuation through, one line below where the
#    old keyword scan went wrong.
WARNING_CONTINUATION = """\
warning: somepkg.sty:12: Package somepkg Warning:
    relayed text that happens to contain Error: not the real problem
error: main.tex:9: ! LaTeX Error: File `missing.sty' not found.
"""
lead = summarize(WARNING_CONTINUATION).split(" | ")[0]
check("missing.sty" in lead, "warning continuation: reports the real error below it")
check("not the real problem" not in lead, "warning continuation: does NOT report the relayed warning text")

# 9. The keyword fallback must skip wrappers too, not just warnings. A wrapper
#    naming its sub-tool matches `biber` and would otherwise be reported as the
#    cause even though it explains nothing.
WRAPPER_NAMES_SUBTOOL = """\
error: the external tool "biber" exited with an error code
error: halted on potentially-recoverable error as specified
"""
lead = summarize(WRAPPER_NAMES_SUBTOOL).split(" | ")[0]
check(lead == truth_render.NO_CAUSE_FOUND, "wrapper naming a sub-tool is not reported as the cause")

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
