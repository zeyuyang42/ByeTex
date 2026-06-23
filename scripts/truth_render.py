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


def _truth_render_env() -> dict:
    """Subprocess env for tectonic: prepend the provisioned `.truth-deps/bin` so the
    version-matched biber (and any other provisioned tools) is found. Run
    `scripts/setup_truth_deps.sh` to populate it."""
    env = os.environ.copy()
    deps_bin = REPO_ROOT / ".truth-deps" / "bin"
    if deps_bin.is_dir():
        env["PATH"] = f"{deps_bin}{os.pathsep}{env.get('PATH', '')}"
    return env


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
            # Surface the most actionable line (font / biber / package errors) plus a tail.
            err = (result.stderr or "").strip()
            hint = next(
                (ln for ln in err.splitlines()
                 if any(k in ln.lower() for k in ("font", "biber", "cannot be found", "not found"))),
                "",
            )
            LAST_TRUTH_RENDER_ERROR = ((hint + " | ") if hint else "") + err[-400:]
            return False
        out_pdf.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(produced, out_pdf)
    return out_pdf.exists() and out_pdf.stat().st_size > 0
