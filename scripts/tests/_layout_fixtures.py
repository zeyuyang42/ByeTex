"""Shared builder for the synthetic layout fixtures.

Not a test (the leading underscore keeps run_all.sh's `*_test.py` glob off it).
Imported by layout_synthetic_test.py and layout_ordering_test.py so the variant
set is defined once and both tests measure exactly the same PDFs.
"""
import json
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import layout_metrics as lm  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
FIXTURE_DIR = REPO_ROOT / "tests" / "fixtures" / "layout"
WORK_DIR = REPO_ROOT / "target" / "layout-fixtures"  # gitignored


def missing_tools(extra: tuple[str, ...] = ()) -> list[str]:
    """Names of unavailable prerequisites, so a test can SKIP cleanly."""
    missing = [t for t in ("typst", "pdftotext", *extra) if shutil.which(t) is None]
    try:
        import fitz  # noqa: F401,PLC0415
    except ImportError:
        missing.append("pymupdf")
    return missing


def load_manifest() -> dict:
    return json.loads((FIXTURE_DIR / "variants.json").read_text())


def expand_expect(expect: dict) -> tuple[set[str], set[str]]:
    """Resolve the `ALL_LAYOUT` shorthand into concrete class names."""
    def resolve(v):
        return set(lm.LAYOUT_CLASSES) if v == "ALL_LAYOUT" else set(v)
    must_fire = resolve(expect.get("must_fire", []))
    must_not = resolve(expect.get("must_not_fire", [])) - must_fire
    return must_fire, must_not


def build_all(manifest: dict, work: Path = WORK_DIR) -> dict[str, Path]:
    """Compile `base` plus every variant. Returns {name: pdf_path}.

    `base2` is a second, independent compile of the identical source — the
    instrument's own repeat noise (control 3). If it is not exactly 1.0 against
    `base`, nothing measured downstream means anything.
    """
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True, exist_ok=True)
    body = (FIXTURE_DIR / "body.typ").read_text()
    preamble = "\n".join(manifest["base_preamble"])

    out: dict[str, Path] = {}
    jobs = [("base", []), ("base2", [])]
    jobs += [(v["name"], v.get("overrides", [])) for v in manifest["variants"]]
    for name, overrides in jobs:
        src = work / f"{name}.typ"
        src.write_text(preamble + "\n" + "\n".join(overrides) + "\n" + body)
        pdf = work / f"{name}.pdf"
        r = subprocess.run(["typst", "compile", "--no-pdf-tags", str(src), str(pdf)],
                           capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(f"typst failed on {name}:\n{r.stdout}\n{r.stderr}")
        out[name] = pdf
    return out


def pdf_text(pdf: Path) -> str:
    """Text via the pdftotext CLI, in READING-ORDER mode (no `-layout`).

    Same binary as visual_test.extract_pdf_text, so glyph and ligature handling
    is identical and a gain can never be an extractor artifact. But NOT the same
    flag, and the difference is not cosmetic:

        base vs twocol, ordered_recall
          pdftotext -layout   0.073      <- columns interleaved line by line
          pdftotext (default) 0.997

    `-layout` reproduces the physical page, so on a 2-column page it emits
    "left-column line 1, right-column line 1, left-column line 2, ..." and the
    reading order is destroyed. `word_recall` is a set() and cannot notice; any
    ORDER-aware metric built on that output would be garbage on every
    2-column paper, which is a large share of the corpus.
    """
    r = subprocess.run(["pdftotext", str(pdf), "-"], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else ""


def metrics_for(truth_pdf: Path, out_pdf: Path) -> dict:
    """Full T1 + T2 comparison, as the wired harness will assemble it."""
    m = dict(lm.layout_compare(truth_pdf, out_pdf))
    m.update(lm.ordered_stream_compare(pdf_text(truth_pdf), pdf_text(out_pdf)))
    return m
