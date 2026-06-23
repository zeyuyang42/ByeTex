#!/usr/bin/env python3
"""
corpus_add_local.py — ingest a NON-arXiv LaTeX project into the ByeTex corpus.

Companion to corpus_harvest.py (which handles arXiv). Drops a project — from a
local directory, a .zip/.tar.gz archive, a git repo, or a plain URL — under
corpus/<id>/source/ in the canonical layout the sweep + visual harness already
consume: it writes a 00README.json (recording the toplevel .tex) and appends a
manifest.json entry. No converter or harness change is needed afterwards.

The common cases (local dir, local archive, git clone) need only the Python
stdlib + `git`; `--url` lazily imports `requests`.

ID naming (must NOT look like an arXiv id, i.e. not \\d{4}\\.\\d{4,6}):
    gh-<org>-<repo>     github         ctan-<name>     CTAN package/manual
    overleaf-<slug>     overleaf        local-<slug>    local-only

Usage:
    # local directory
    python scripts/corpus_add_local.py ./mybook --id local-mybook \\
        --source-kind local --doc-type book --title "My Book"

    # git repo (records the resolved commit SHA for reproducibility)
    python scripts/corpus_add_local.py --git https://github.com/org/repo \\
        [--ref BRANCH_OR_SHA] --id gh-org-repo --source-kind github \\
        --doc-type book --title "The Title"

    # a hand-downloaded archive (e.g. a login-walled Overleaf export)
    python scripts/corpus_add_local.py ~/Downloads/x.zip --id overleaf-x \\
        --source-kind overleaf --doc-type thesis --needs-manual-download

    # plain URL to an archive (CTAN, release tarball, ...)
    python scripts/corpus_add_local.py --url https://example.org/x.tar.gz \\
        --id ctan-foo --source-kind ctan --doc-type book

Options:
    --toplevel NAME.tex   override toplevel autodetection (required if 0 or >1 found)
    --force               overwrite an existing corpus/<id>/source/
    --dry-run             show what would happen; write nothing
"""

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path

# ─────────────────────────────────────────────────────────────────────────────
# Paths / constants (mirror corpus_harvest.py; kept local to avoid a hard
# `requests` import for the stdlib-only ingestion paths)
# ─────────────────────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).parent.parent.resolve()
CORPUS_DIR = REPO_ROOT / "corpus"
MANIFEST_PATH = CORPUS_DIR / "manifest.json"

ARXIV_ID_RE = re.compile(r"^[0-9]{4}\.[0-9]{4,6}$")  # corpus_clean.sh ID_RE
VALID_ID_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
DOCCLASS_RE = re.compile(r"\\documentclass\s*(?:\[[^\]]*\])?\s*\{([^}]+)\}")
BEGIN_DOC_RE = re.compile(r"\\begin\{document\}")
ARCHIVE_SUFFIXES = (".zip", ".tar.gz", ".tgz", ".tar", ".tar.bz2", ".tar.xz")


def _now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def load_manifest() -> dict:
    if MANIFEST_PATH.exists():
        return json.loads(MANIFEST_PATH.read_text())
    return {"schema_version": 2, "generated_at": _now(), "papers": []}


def flush_manifest(manifest: dict) -> None:
    manifest["generated_at"] = _now()
    MANIFEST_PATH.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST_PATH.write_text(json.dumps(manifest, indent=2) + "\n")


# ─────────────────────────────────────────────────────────────────────────────
# Safe extraction (guard tar-slip / zip-slip), with single-wrapper-dir flatten
# ─────────────────────────────────────────────────────────────────────────────

def _within(base: Path, target: Path) -> bool:
    try:
        target.resolve().relative_to(base.resolve())
        return True
    except ValueError:
        return False


def extract_archive(archive: Path, into: Path) -> None:
    into.mkdir(parents=True, exist_ok=True)
    name = archive.name.lower()
    if name.endswith(".zip"):
        with zipfile.ZipFile(archive) as zf:
            for member in zf.namelist():
                if not _within(into, into / member):
                    raise ValueError(f"Zip-slip detected: {member!r}")
            zf.extractall(into)
    else:
        mode = "r:*"  # transparent gz/bz2/xz/plain
        with tarfile.open(archive, mode) as tf:
            for member in tf.getmembers():
                if not _within(into, into / member.name):
                    raise ValueError(f"Tar-slip detected: {member.name!r}")
            tf.extractall(into)


def flatten_single_wrapper(root: Path) -> Path:
    """GitHub/CTAN archives wrap everything in one top dir (repo-main/). If the
    extracted tree is exactly one directory, treat that as the project root."""
    entries = [p for p in root.iterdir() if p.name not in (".DS_Store",)]
    if len(entries) == 1 and entries[0].is_dir():
        return entries[0]
    return root


# ─────────────────────────────────────────────────────────────────────────────
# Source acquisition → a directory tree we can copy into corpus/<id>/source/
# ─────────────────────────────────────────────────────────────────────────────

def acquire_git(url: str, ref: str | None, workdir: Path) -> tuple[Path, str]:
    """Clone (shallow when possible) and return (worktree_dir, resolved_sha)."""
    clone_dir = workdir / "clone"
    cmd = ["git", "clone", "--quiet"]
    if ref is None:
        cmd += ["--depth", "1"]
    cmd += [url, str(clone_dir)]
    subprocess.run(cmd, check=True)
    if ref is not None:
        subprocess.run(["git", "-C", str(clone_dir), "fetch", "--quiet", "--depth", "1",
                        "origin", ref], check=True)
        subprocess.run(["git", "-C", str(clone_dir), "checkout", "--quiet", ref], check=True)
    sha = subprocess.run(["git", "-C", str(clone_dir), "rev-parse", "HEAD"],
                         check=True, capture_output=True, text=True).stdout.strip()
    shutil.rmtree(clone_dir / ".git", ignore_errors=True)
    return clone_dir, sha


def acquire_url(url: str, workdir: Path) -> tuple[Path, str, str, int]:
    """Download an archive, extract, return (root, archive_name, sha256, bytes)."""
    import requests  # lazy: only this path needs it
    archive_name = url.rsplit("/", 1)[-1] or "download"
    if not archive_name.lower().endswith(ARCHIVE_SUFFIXES):
        archive_name += ".tar.gz"
    archive = workdir / archive_name
    r = requests.get(url, stream=True, timeout=60,
                     headers={"User-Agent": "ByeTex-Harvester/0.1 (research/testing)"})
    r.raise_for_status()
    hasher = hashlib.sha256()
    nbytes = 0
    with open(archive, "wb") as f:
        for chunk in r.iter_content(65536):
            f.write(chunk)
            hasher.update(chunk)
            nbytes += len(chunk)
    extract_dir = workdir / "extract"
    extract_archive(archive, extract_dir)
    return flatten_single_wrapper(extract_dir), archive_name, hasher.hexdigest(), nbytes


def hash_file(path: Path) -> tuple[str, int]:
    hasher = hashlib.sha256()
    nbytes = 0
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            hasher.update(chunk)
            nbytes += len(chunk)
    return hasher.hexdigest(), nbytes


# ─────────────────────────────────────────────────────────────────────────────
# Toplevel .tex detection
# ─────────────────────────────────────────────────────────────────────────────

def detect_toplevel(source_dir: Path) -> tuple[list[Path], dict[Path, str]]:
    """Return (candidates, {file: doc_class}). A toplevel has both \\documentclass
    and \\begin{document}; \\input-ed chapter files have neither."""
    candidates: list[Path] = []
    classes: dict[Path, str] = {}
    for tex in sorted(source_dir.rglob("*.tex")):
        if any(part.startswith(".") for part in tex.relative_to(source_dir).parts):
            continue
        try:
            text = tex.read_text(errors="replace")
        except OSError:
            continue
        m = DOCCLASS_RE.search(text)
        if m and BEGIN_DOC_RE.search(text):
            candidates.append(tex)
            classes[tex] = m.group(1).strip()
    return candidates, classes


# ─────────────────────────────────────────────────────────────────────────────
# Truth-render gate (truth-first rule — see docs/autonomous-dev.md)
# ─────────────────────────────────────────────────────────────────────────────
def decide_truth_gate(
    render_ok: bool, reason: "str | None", tectonic_present: bool, allow_no_truth: bool
) -> "tuple[bool, str, str | None]":
    """Decide whether to accept a paper based on its truth render. Returns
    (accept, truth_render_status, truth_render_error).

    - tectonic absent → can't verify; accept as "unverified" (don't block what we can't check).
    - render ok → accept, "ok".
    - render fails → "failed"+reason; REJECT unless `--allow-no-truth` (then accept but record it,
      so it's never a silent unmeasured "pass").
    """
    if not tectonic_present:
        return (True, "unverified", None)
    if render_ok:
        return (True, "ok", None)
    return (allow_no_truth, "failed", reason)


def verify_truth_render(toplevel: Path) -> "tuple[bool, str | None]":
    """Render the paper's truth with tectonic (deps via scripts/setup_truth_deps.sh).
    Returns (ok, reason). Uses the stdlib-only `truth_render` module so this stays usable
    without the metric deps (numpy/Pillow)."""
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import truth_render as tr

    with tempfile.TemporaryDirectory(prefix=".truthgate-") as tmp:
        ok = tr.render_reference_tectonic(toplevel, Path(tmp) / "truth.pdf")
    return ok, (None if ok else tr.LAST_TRUTH_RENDER_ERROR)


def tectonic_present() -> bool:
    try:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        import truth_render as tr
        return tr.tectonic_available()
    except Exception:
        return False


# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    p = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("source", nargs="?", help="local directory or archive (.zip/.tar.gz)")
    p.add_argument("--git", metavar="URL", help="clone a git repo as the source")
    p.add_argument("--ref", metavar="SHA", help="git ref/branch/commit to check out (with --git)")
    p.add_argument("--url", metavar="URL", help="download an archive from URL as the source")
    p.add_argument("--id", required=True, help="corpus id, e.g. gh-org-repo / ctan-name")
    p.add_argument("--source-kind", default="local",
                   choices=["github", "local", "ctan", "overleaf"],
                   help="manifest `source` value (default: local)")
    p.add_argument("--doc-type", default="", help="hint: book|report|thesis|article")
    p.add_argument("--title", default="", help="human title (defaults to the id)")
    p.add_argument("--homepage", default="", help="where to find/learn about the source")
    p.add_argument("--toplevel", metavar="NAME.tex",
                   help="override toplevel autodetection (required if 0 or >1 found)")
    p.add_argument("--needs-manual-download", action="store_true",
                   help="mark login-walled (not auto re-fetchable)")
    p.add_argument("--force", action="store_true", help="overwrite an existing corpus/<id>/")
    p.add_argument("--allow-no-truth", action="store_true",
                   help="accept even if the truth render fails (records truth_render_status=failed "
                        "+ reason instead of rejecting). Use only when the failure is genuinely "
                        "out of scope — see docs/truth-render-status.md.")
    p.add_argument("--dry-run", action="store_true", help="show what would happen; no writes")
    args = p.parse_args()

    # ── validate id + source selection ──
    if ARXIV_ID_RE.match(args.id):
        p.error(f"id {args.id!r} looks like an arXiv id; use a prefixed scheme (gh-/ctan-/local-).")
    if not VALID_ID_RE.match(args.id):
        p.error(f"id {args.id!r} must be lowercase [a-z0-9-], not starting with '-'.")
    chosen = [s for s in (args.source, args.git, args.url) if s]
    if len(chosen) != 1:
        p.error("provide exactly one source: a positional path, --git URL, or --url URL.")

    dest = CORPUS_DIR / args.id
    source_dir = dest / "source"
    if source_dir.exists() and not args.force:
        p.error(f"{source_dir} already exists (use --force to overwrite).")

    manifest = load_manifest()
    if any(pp["id"] == args.id for pp in manifest["papers"]) and not args.force:
        p.error(f"manifest already has id {args.id!r} (use --force).")

    repo_url = repo_ref = download_url = archive_filename = archive_sha256 = ""
    archive_bytes = 0

    with tempfile.TemporaryDirectory(prefix="byetex-add-") as tmp:
        workdir = Path(tmp)

        # ── 1. acquire the source tree ──
        if args.git:
            print(f"  cloning {args.git} ...", flush=True)
            root, repo_ref = acquire_git(args.git, args.ref, workdir)
            repo_url = args.git
        elif args.url:
            print(f"  downloading {args.url} ...", flush=True)
            root, archive_filename, archive_sha256, archive_bytes = acquire_url(args.url, workdir)
            download_url = args.url
        else:
            src = Path(args.source).expanduser()
            if src.is_dir():
                root = src
            elif src.is_file() and src.name.lower().endswith(ARCHIVE_SUFFIXES):
                print(f"  extracting {src.name} ...", flush=True)
                archive_filename = src.name
                archive_sha256, archive_bytes = hash_file(src)
                extract_dir = workdir / "extract"
                extract_archive(src, extract_dir)
                root = flatten_single_wrapper(extract_dir)
            else:
                p.error(f"{src} is neither a directory nor a supported archive {ARCHIVE_SUFFIXES}.")

        # ── 2. detect toplevel .tex ──
        candidates, classes = detect_toplevel(root)
        if args.toplevel:
            top = root / args.toplevel
            matches = [c for c in candidates if c == top] or ([top] if top.exists() else [])
            if not matches:
                p.error(f"--toplevel {args.toplevel!r} not found under the source.")
            toplevel = matches[0]
        elif len(candidates) == 1:
            toplevel = candidates[0]
        elif not candidates:
            p.error("no toplevel .tex found (a file with both \\documentclass and "
                    "\\begin{document}); pass --toplevel NAME.tex.")
        else:
            rels = "\n    ".join(str(c.relative_to(root)) for c in candidates)
            p.error(f"multiple toplevel candidates; pass --toplevel:\n    {rels}")

        toplevel_rel = toplevel.relative_to(root).as_posix()
        doc_class = classes.get(toplevel, "")
        print(f"  toplevel: {toplevel_rel}   doc_class: {doc_class or '?'}", flush=True)

        if args.dry_run:
            print(f"  [dry-run] would copy → {source_dir}")
            print(f"  [dry-run] would write {source_dir / '00README.json'} (toplevel: {toplevel_rel})")
            print(f"  [dry-run] would append manifest entry id={args.id} source={args.source_kind}")
            return

        # ── 3. materialize corpus/<id>/source/ ──
        if source_dir.exists():
            shutil.rmtree(source_dir)
        shutil.copytree(root, source_dir, ignore=shutil.ignore_patterns(
            ".git", ".github", ".DS_Store", "__pycache__"))

        # ── 4. truth-first gate: the original LaTeX must render a truth PDF (the fidelity
        #    DRIVER's reference) before we accept the paper. Never let a paper become a
        #    silent unmeasured "pass" (see docs/autonomous-dev.md). ──
        has_tectonic = tectonic_present()
        if not has_tectonic:
            print("  [warn] tectonic not found — cannot verify truth render; accepting as "
                  "'unverified'. Install tectonic + run scripts/setup_truth_deps.sh.", flush=True)
            truth_ok, truth_reason = False, None
        else:
            print("  verifying truth render (tectonic) ...", flush=True)
            truth_ok, truth_reason = verify_truth_render(source_dir / toplevel_rel)
        accept, truth_status, truth_error = decide_truth_gate(
            truth_ok, truth_reason, has_tectonic, args.allow_no_truth)
        if truth_status == "ok":
            print("  truth render: OK", flush=True)
        elif truth_status == "failed":
            print(f"  truth render: FAILED — {truth_error}", flush=True)
        if not accept:
            shutil.rmtree(dest, ignore_errors=True)  # don't leave a half-added paper (whole corpus/<id>)
            p.error(
                f"truth render failed for {toplevel_rel} — not adding {args.id!r} to the corpus.\n"
                f"    reason: {truth_error}\n"
                f"    fix the dep (scripts/setup_truth_deps.sh adds biber + fonts) and retry, or\n"
                f"    pass --allow-no-truth if the failure is genuinely out of scope "
                f"(record it in docs/truth-render-status.md).")

        # ── 5. write 00README.json (only `toplevel` is load-bearing for the harness) ──
        readme = {
            "sources": [{"usage": "toplevel", "filename": toplevel_rel}],
            "spec_version": 1,
            "texlive_version": "2025",
            "process": {"compiler": "pdflatex"},
            "truth_render_status": truth_status,
        }
        if truth_error:
            readme["truth_render_error"] = truth_error
        (source_dir / "00README.json").write_text(json.dumps(readme, indent=2) + "\n")

        # ── 6. append manifest entry ──
        entry = {
            "id": args.id,
            "pinned": False,
            "source": args.source_kind,
            "doc_class": doc_class,
            "doc_type": args.doc_type,
            "title": args.title or args.id,
            "repo_url": repo_url,
            "repo_ref": repo_ref,
            "homepage": args.homepage,
            "download_url": download_url,
            "archive_filename": archive_filename,
            "archive_sha256": archive_sha256,
            "archive_bytes": archive_bytes,
            "needs_manual_download": bool(args.needs_manual_download),
            "license_url": "",
            "fetched_at": _now(),
            "truth_render_status": truth_status,
        }
        if truth_error:
            entry["truth_render_error"] = truth_error
        manifest["papers"] = [pp for pp in manifest["papers"] if pp["id"] != args.id]
        manifest["papers"].append(entry)
        flush_manifest(manifest)

    print(f"  added corpus/{args.id}/  ({args.source_kind}, doc_class={doc_class or '?'})", flush=True)
    print(f"  next: ./scripts/corpus_sweep.sh {args.id}", flush=True)


if __name__ == "__main__":
    main()
