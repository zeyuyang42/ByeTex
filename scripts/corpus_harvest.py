#!/usr/bin/env python3
"""
ByeTex corpus harvester — downloads arXiv source tarballs for testing the
ByeTex LaTeX→Typst converter.

The manifest at corpus/manifest.json is the source of truth for known papers.
Payloads (tarballs + extracted source/) are gitignored and fetched on demand.

Usage:
    # Fetch missing payloads for all papers in the manifest:
    python scripts/corpus_harvest.py

    # Fetch only the 5 pinned regression papers (for CI / template_budgets):
    python scripts/corpus_harvest.py --pinned

    # Search arXiv for new papers and add them to the manifest:
    python scripts/corpus_harvest.py --search cs.LG --limit 5

    # Dry-run: show what would be fetched:
    python scripts/corpus_harvest.py --dry-run
    python scripts/corpus_harvest.py --pinned --dry-run
"""

import argparse
import gzip
import hashlib
import json
import re
import sys
import tarfile
import time
import random
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlencode
import xml.etree.ElementTree as ET

import requests

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).parent.parent.resolve()
CORPUS_DIR = REPO_ROOT / "corpus"
MANIFEST_PATH = CORPUS_DIR / "manifest.json"

DEFAULT_UA = (
    "ByeTex-Harvester/0.1 (+https://github.com/zeyuyang42/ByeTex; "
    "research/testing use only)"
)
ARXIV_MIN_DELAY = 3.0  # arXiv ToU: >= 3 s between requests

ARXIV_API = "https://export.arxiv.org/api/query"
ARXIV_EPRINT = "https://arxiv.org/e-print"
ARXIV_ABS = "https://arxiv.org/abs"
ARXIV_NS = {
    "atom": "http://www.w3.org/2005/Atom",
    "arxiv": "http://arxiv.org/schemas/atom",
}


# ─────────────────────────────────────────────────────────────────────────────
# HTTP helpers
# ─────────────────────────────────────────────────────────────────────────────

def make_session(ua: str) -> requests.Session:
    s = requests.Session()
    s.headers["User-Agent"] = ua
    return s


def fetch(
    session: requests.Session,
    url: str,
    stream: bool = False,
    **kwargs,
) -> requests.Response:
    last_err: Exception | None = None
    for attempt in range(3):
        try:
            r = session.get(url, stream=stream, timeout=30, **kwargs)
            if r.status_code < 500:
                return r
            wait = 2 ** attempt * 2
            print(f"  HTTP {r.status_code} for {url!r}, retry in {wait}s", file=sys.stderr)
            time.sleep(wait)
        except (requests.exceptions.Timeout, requests.exceptions.ConnectionError) as exc:
            last_err = exc
            wait = 2 ** attempt * 2
            print(f"  {exc!r}, retry in {wait}s", file=sys.stderr)
            time.sleep(wait)
    raise RuntimeError(f"Exhausted retries for {url}: {last_err}")


def sleep_politely(base: float) -> None:
    time.sleep(base + random.uniform(0, 0.5))


# ─────────────────────────────────────────────────────────────────────────────
# Manifest helpers
# ─────────────────────────────────────────────────────────────────────────────

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


def is_present(arxiv_id: str) -> bool:
    return (CORPUS_DIR / arxiv_id / "source").is_dir()


# ─────────────────────────────────────────────────────────────────────────────
# arXiv search
# ─────────────────────────────────────────────────────────────────────────────

def arxiv_query(
    session: requests.Session, category: str, n: int, delay: float
) -> list[dict]:
    url = ARXIV_API + "?" + urlencode({
        "search_query": f"cat:{category}",
        "sortBy": "submittedDate",
        "sortOrder": "descending",
        "max_results": n,
    })
    r = fetch(session, url)
    r.raise_for_status()
    sleep_politely(delay)

    root = ET.fromstring(r.text)
    results: list[dict] = []
    for entry in root.findall("atom:entry", ARXIV_NS):
        raw_id = entry.findtext("atom:id", "", ARXIV_NS) or ""
        arxiv_id = re.sub(r"v\d+$", "", raw_id.rsplit("/", 1)[-1])
        title = (entry.findtext("atom:title", "", ARXIV_NS) or "").strip().replace("\n", " ")
        published = entry.findtext("atom:published", "", ARXIV_NS) or ""
        pc_el = entry.find("arxiv:primary_category", ARXIV_NS)
        primary_cat = pc_el.get("term", category) if pc_el is not None else category
        lic_el = entry.find("arxiv:license", ARXIV_NS)
        license_url = lic_el.text.strip() if lic_el is not None and lic_el.text else ""
        authors = [
            (a.findtext("atom:name", "", ARXIV_NS) or "")
            for a in entry.findall("atom:author", ARXIV_NS)
        ]
        results.append({
            "arxiv_id": arxiv_id,
            "title": title,
            "published": published,
            "primary_category": primary_cat,
            "license_url": license_url,
            "authors": authors,
        })
    return results


# ─────────────────────────────────────────────────────────────────────────────
# Download a single arXiv paper
# ─────────────────────────────────────────────────────────────────────────────

def download_paper(
    session: requests.Session, arxiv_id: str, delay: float, dry_run: bool
) -> None:
    dest = CORPUS_DIR / arxiv_id
    source_dir = dest / "source"

    if is_present(arxiv_id):
        print(f"  [skip] {arxiv_id} (already on disk)", flush=True)
        return

    url = f"{ARXIV_EPRINT}/{arxiv_id}"

    if dry_run:
        print(f"  [dry-run] would fetch {arxiv_id} from {url}")
        return

    print(f"  fetching {arxiv_id} ...", flush=True)
    dest.mkdir(parents=True, exist_ok=True)
    archive = dest / "source.tar.gz"

    r = fetch(session, url, stream=True, allow_redirects=True)
    r.raise_for_status()
    sleep_politely(delay)

    hasher = hashlib.sha256()
    nbytes = 0
    with open(archive, "wb") as f:
        for chunk in r.iter_content(65536):
            f.write(chunk)
            hasher.update(chunk)
            nbytes += len(chunk)

    with open(archive, "rb") as f:
        magic = f.read(2)

    if magic != b"\x1f\x8b":
        print(f"  [warn] {arxiv_id}: not a gzip file, skipping extraction", file=sys.stderr)
        return

    try:
        with tarfile.open(archive, "r:gz") as tf:
            base = str(source_dir.resolve())
            for member in tf.getmembers():
                target = str((source_dir / member.name).resolve())
                if not target.startswith(base):
                    raise ValueError(f"Tar-slip detected: {member.name!r}")
            tf.extractall(source_dir)
    except tarfile.TarError:
        tex_out = source_dir / f"{arxiv_id.replace('/', '_')}.tex"
        tex_out.parent.mkdir(parents=True, exist_ok=True)
        with gzip.open(archive, "rb") as gz:
            tex_out.write_bytes(gz.read())

    print(f"  saved: corpus/{arxiv_id}/", flush=True)


# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    mode = p.add_mutually_exclusive_group()
    mode.add_argument(
        "--pinned",
        action="store_true",
        help="fetch only papers marked pinned:true in the manifest (used by CI)",
    )
    mode.add_argument(
        "--search",
        metavar="CATEGORY",
        help="query arXiv for new papers in CATEGORY and add to manifest",
    )
    p.add_argument(
        "--limit",
        type=int,
        default=5,
        metavar="N",
        help="max papers to add when using --search (default: 5)",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="show what would be fetched; no writes",
    )
    p.add_argument(
        "--delay",
        type=float,
        default=2.0,
        metavar="SEC",
        help="base delay between arXiv requests (default: 2.0; arXiv enforces >=3.0)",
    )
    p.add_argument("--user-agent", default=DEFAULT_UA, metavar="UA")
    args = p.parse_args()

    delay = max(args.delay, ARXIV_MIN_DELAY)
    session = make_session(args.user_agent)
    manifest = load_manifest()

    if args.search:
        # ── search mode: query arXiv, add new papers to manifest, then fetch ──
        existing_ids = {p["id"] for p in manifest["papers"]}
        print(f"Querying arXiv {args.search!r} for up to {args.limit} new papers ...")
        try:
            results = arxiv_query(session, args.search, args.limit * 3, delay)
        except Exception as exc:
            print(f"[error] arXiv query failed: {exc}", file=sys.stderr)
            sys.exit(1)

        added = 0
        for meta in results:
            if added >= args.limit:
                break
            aid = meta["arxiv_id"]
            if aid in existing_ids:
                continue
            paper = {
                "id": aid,
                "pinned": False,
                "source": "arxiv",
                "arxiv_primary_category": meta["primary_category"],
                "title": meta["title"],
                "arxiv_published": meta["published"],
                "arxiv_authors": meta["authors"][:5],
                "detail_url": f"{ARXIV_ABS}/{aid}",
                "download_url": f"{ARXIV_EPRINT}/{aid}",
                "archive_filename": "source.tar.gz",
                "archive_sha256": "",
                "archive_bytes": 0,
                "license_url": meta["license_url"],
                "fetched_at": "",
            }
            if not args.dry_run:
                manifest["papers"].append(paper)
            print(f"  + {aid}: {meta['title'][:70]}")
            added += 1

        if not args.dry_run:
            flush_manifest(manifest)
            print(f"Manifest updated: {MANIFEST_PATH}")

        papers_to_fetch = [p for p in manifest["papers"] if p["id"] in {
            m["arxiv_id"] for m in results[:added]
        }] if not args.dry_run else []

    elif args.pinned:
        # ── pinned mode: only fetch the pinned regression set ──
        papers_to_fetch = [p for p in manifest["papers"] if p.get("pinned")]
        print(f"Fetching {len(papers_to_fetch)} pinned paper(s) ...")

    else:
        # ── default: fetch all missing payloads ──
        papers_to_fetch = manifest["papers"]
        missing = [p for p in papers_to_fetch if not is_present(p["id"])]
        print(f"{len(papers_to_fetch)} papers in manifest; {len(missing)} missing payload(s).")
        papers_to_fetch = papers_to_fetch  # download_paper skips present ones

    count = 0
    for paper in papers_to_fetch:
        if paper.get("source", "arxiv") != "arxiv":
            # non-arXiv entries (added by corpus_add_local.py) aren't on arXiv;
            # they're fetched/refreshed by that script, not here.
            continue
        try:
            download_paper(session, paper["id"], delay, args.dry_run)
            count += 1
        except Exception as exc:
            print(f"  [error] {paper['id']}: {exc}", file=sys.stderr)

    verb = "Would fetch" if args.dry_run else "Fetched"
    print(f"\n{verb} {count} paper(s).")


if __name__ == "__main__":
    main()
