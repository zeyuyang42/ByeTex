"""Tectonic reference ("truth") renderer — stdlib-only so any script can import it
without pulling in the metric deps (numpy/Pillow) that the rest of visual_test needs.

Renders a paper's *original* LaTeX to a PDF locally with tectonic, using the deps
provisioned by scripts/setup_truth_deps.sh (a version-matched biber on PATH + fonts).
Mirrors the `byetex doctor` shell-out: skip cleanly when tectonic is absent.
BYETEX_TECTONIC_BIN overrides the binary (tests / custom installs).
"""
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# The reason the most recent render_reference_tectonic() call failed (stderr tail), or None.
# Read it via `truth_render.LAST_TRUTH_RENDER_ERROR` right after the call (module attribute,
# so it reflects the latest run — a `from ... import` would freeze it at None).
LAST_TRUTH_RENDER_ERROR: "str | None" = None


def tectonic_bin() -> str:
    return os.environ.get("BYETEX_TECTONIC_BIN", "tectonic")


def tectonic_available() -> bool:
    try:
        return subprocess.run(
            [tectonic_bin(), "--version"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        ).returncode == 0
    except FileNotFoundError:
        return False


def _deps_bin_dirs() -> list:
    """Every `.truth-deps/bin` worth putting on PATH, most specific first.

    `.truth-deps/` is gitignored and lives in whichever checkout ran
    `setup_truth_deps.sh` — so a WORKTREE does not have one, the prepend below
    silently no-ops there, and tectonic falls back to whatever biber is on the
    system. That is not hypothetical: a Homebrew biber 2.21 expects control-file
    3.11 while tectonic's bundled biblatex 3.17 emits 3.8, so both biblatex
    theses in the corpus failed their truth render with a version mismatch and
    the fidelity gate went blind to them. With the pinned 2.17 they render fine.
    So fall back to the MAIN worktree's copy, which is where setup normally runs.
    """
    dirs = [REPO_ROOT / ".truth-deps" / "bin"]
    try:
        common = subprocess.run(
            ["git", "rev-parse", "--git-common-dir"],
            cwd=REPO_ROOT, capture_output=True, text=True, check=True,
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return [d for d in dirs if d.is_dir()]
    if common:
        main_root = (REPO_ROOT / common).resolve().parent
        dirs.append(main_root / ".truth-deps" / "bin")
    seen, out = set(), []
    for d in dirs:
        if d.is_dir() and str(d) not in seen:
            seen.add(str(d))
            out.append(d)
    return out


def _truth_render_env() -> dict:
    """Subprocess env for tectonic: prepend the provisioned `.truth-deps/bin` so the
    version-matched biber (and any other provisioned tools) is found. Run
    `scripts/setup_truth_deps.sh` to populate it."""
    env = os.environ.copy()
    found = _deps_bin_dirs()
    if found:
        prefix = os.pathsep.join(str(d) for d in found)
        env["PATH"] = f"{prefix}{os.pathsep}{env.get('PATH', '')}"
    return env


# Tectonic `error:` lines that carry no cause of their own. Two kinds: the
# blanket "halted" line it prints on nearly every failure, and the wrappers it
# emits around a sub-tool's output — those END with a colon and defer to the
# lines that follow, which is where the real message lives (a biber/biblatex
# version mismatch, say).
_TECTONIC_WRAPPERS = (
    "halted on potentially-recoverable error",
    # Matches both "the external tool exited with ..." and the form that names
    # the tool ("the external tool \"biber\" exited with ..."), which a literal
    # phrase would miss — and that one matches the `biber` hint keyword, so it
    # would be reported as the cause while explaining nothing.
    "the external tool",
    "its stdout was:",
    "its stderr was:",
)

# Markers for a line that states a cause. Sub-tools do not follow tectonic's
# `error:` convention — biber says `ERROR - Error: ...` — so matching only the
# prefix would skip straight past the one line that explains the failure.
_ERROR_MARKERS = ("error:", "error -", "! latex error")

# Fallback keywords, used only when tectonic emitted no `error:` line at all.
_HINT_KEYWORDS = ("font", "biber", "cannot be found", "not found")

# Leading segment when tectonic gave no attributable cause. Explicit, so the
# reader knows the tail is unexplained output rather than a diagnosis.
NO_CAUSE_FOUND = "(no error line in tectonic stderr)"


def summarize_tectonic_failure(stderr: str) -> str:
    """Explain WHY a tectonic render failed: a leading one-line cause, then the
    raw stderr tail.

    The cause is drawn from tectonic's `error:` lines, never its `warning:`
    lines. That distinction is the whole point of this function: scanning all of
    stderr for a font/package keyword matched warnings just as readily as
    errors, and on 3 of the corpus's 6 failing papers it reported a benign
    warning — a Carlito font *path* notice, an `algorithm.sty` UTF-8 notice —
    while the real cause (a missing "Latin Modern Math", an undefined control
    sequence) went unmentioned. A paper whose failure is misattributed is one
    nobody can fix, and the fidelity driver stays blind on it.
    """
    err = (stderr or "").strip()
    # Always attributable, even with nothing to go on: this is only called on a
    # failure, and returning "" would leave the caller printing
    # "render failed: " and storing an empty reason — the unattributed failure
    # this function exists to prevent. Tectonic can exit 0 without producing the
    # expected PDF, and that path reaches here with empty stderr.
    if not err:
        return NO_CAUSE_FOUND

    # Walk the output as blocks. A `warning:`/`note:`/`error:` line opens one and
    # any following unprefixed line continues it, inheriting its severity. Both
    # halves matter: a warning whose text spills onto the next line must not
    # supply the cause, while biber's `ERROR - Error: …` — an unprefixed
    # continuation of tectonic's `error:` wrapper — must stay eligible.
    lines = [ln.strip() for ln in err.splitlines() if ln.strip()]
    errors = []
    in_error_block = True  # nothing has established a warning context yet
    for ln in lines:
        low = ln.lower()
        if low.startswith(("warning:", "note:")):
            in_error_block = False
            continue
        if low.startswith("error:"):
            in_error_block = True
        elif not in_error_block:
            continue  # continuation of a warning/note block
        if any(w in low for w in _TECTONIC_WRAPPERS):
            continue
        if any(m in low for m in _ERROR_MARKERS):
            errors.append(ln)
    if errors:
        hint = errors[0]
    else:
        # No usable `error:` line — fall back to the keyword scan. It skips the
        # same warnings AND the same wrappers as the scan above; a wrapper that
        # names its sub-tool ("the external tool \"biber\" exited with ...")
        # would otherwise match on `biber` and be reported as the cause.
        hint = next(
            (ln for ln in lines
             if not ln.lower().startswith(("warning:", "note:"))
             and not any(w in ln.lower() for w in _TECTONIC_WRAPPERS)
             and any(k in ln.lower() for k in _HINT_KEYWORDS)),
            "",
        )
    # Say so explicitly rather than emitting a bare tail: a caller that prints
    # only the leading segment would otherwise show whatever line happened to
    # come first, which is the misattribution this function exists to prevent.
    return (hint or NO_CAUSE_FOUND) + " | " + err[-400:]


def render_reference_tectonic(toplevel: Path, out_pdf: Path) -> bool:
    """Render a LaTeX source to PDF with tectonic; return True on success.

    The scratch outputs land in a tempdir anchored inside the source's own
    directory (kept out of the system temp), and the produced PDF is copied
    to `out_pdf`. On failure, `LAST_TRUTH_RENDER_ERROR` holds the reason
    (missing font / biber backend / unsupported package) for the caller to record.
    """
    global LAST_TRUTH_RENDER_ERROR
    LAST_TRUTH_RENDER_ERROR = None
    # Resolve to absolute so --outdir is independent of the subprocess cwd
    # (we run with cwd=src_dir so \input/\include resolve like the source).
    src_dir = toplevel.parent.resolve()
    with tempfile.TemporaryDirectory(dir=src_dir, prefix=".tectonic-out-") as tmp:
        result = subprocess.run(
            [tectonic_bin(), "--outdir", str(Path(tmp)), "--keep-logs", toplevel.name],
            cwd=src_dir, capture_output=True, text=True, env=_truth_render_env(),
        )
        produced = Path(tmp) / (toplevel.stem + ".pdf")
        if result.returncode != 0 or not produced.exists():
            LAST_TRUTH_RENDER_ERROR = summarize_tectonic_failure(result.stderr or "")
            return False
        out_pdf.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(produced, out_pdf)
    return out_pdf.exists() and out_pdf.stat().st_size > 0
