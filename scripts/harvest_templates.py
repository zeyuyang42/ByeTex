#!/usr/bin/env python3
"""
ByeTex corpus harvester — downloads LaTeX templates from latextemplates.com
and arXiv source tarballs for testing the ByeTex LaTeX→Typst converter.

Usage:
    python scripts/harvest_templates.py --dry-run
    python scripts/harvest_templates.py --limit 5
    python scripts/harvest_templates.py --source arxiv --limit 20 --arxiv-category math.NA
    python scripts/harvest_templates.py --source all --no-limit   # large batch (confirm first)

Output goes into ./templates/ (under latextemplates/, arxiv/, and a top-level
manifest.json — all gitignored). Curated hand-written templates live alongside
in templates/{IEEE,ACM,NeurIPS,thesis}/ and ARE committed. Use --resume to
skip items already fetched on a previous run.
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
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlencode
import xml.etree.ElementTree as ET

import requests
from bs4 import BeautifulSoup

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

DEFAULT_UA = (
    "ByeTex-Harvester/0.1 (+https://github.com/zeyuyang42/ByeTex; "
    "research/testing use only)"
)
ARXIV_MIN_DELAY = 3.0  # arXiv ToU: >= 3 s between requests

LT_BASE = "https://www.latextemplates.com"
# Category slugs on latextemplates.com, ordered essay-first for small batches
LT_ESSAY_CATS = ["essays"]
LT_ACADEMIC_CATS = ["articles", "academic-journals"]

ARXIV_API = "https://export.arxiv.org/api/query"
ARXIV_EPRINT = "https://arxiv.org/e-print"
ARXIV_ABS = "https://arxiv.org/abs"
ARXIV_NS = {
    "atom": "http://www.w3.org/2005/Atom",
    "arxiv": "http://arxiv.org/schemas/atom",
}
DEFAULT_ARXIV_CATS = ["cs.LG", "math.NA"]


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


def load_manifest(path: Path) -> dict:
    if path.exists():
        return json.loads(path.read_text())
    return {"version": 1, "generated_at": _now(), "entries": []}


def flush_manifest(manifest: dict, path: Path) -> None:
    manifest["generated_at"] = _now()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(manifest, indent=2) + "\n")


def is_fetched(manifest: dict, item_id: str, archive: Path) -> bool:
    return any(e["id"] == item_id for e in manifest["entries"]) and archive.exists()


# ─────────────────────────────────────────────────────────────────────────────
# latextemplates.com
# ─────────────────────────────────────────────────────────────────────────────

def lt_list_slugs(session: requests.Session, cat_slug: str, delay: float) -> list[str]:
    url = f"{LT_BASE}/cat/{cat_slug}"
    r = fetch(session, url)
    r.raise_for_status()
    sleep_politely(delay)
    soup = BeautifulSoup(r.text, "html.parser")
    seen: set[str] = set()
    slugs: list[str] = []
    for a in soup.find_all("a", href=True):
        href = a["href"]
        if not href.startswith("/template/"):
            continue
        slug = href.split("/template/", 1)[1].rstrip("/")
        if slug and slug not in seen:
            seen.add(slug)
            slugs.append(slug)
    return slugs


def lt_detail(session: requests.Session, slug: str, delay: float) -> dict:
    url = f"{LT_BASE}/template/{slug}"
    r = fetch(session, url)
    r.raise_for_status()
    sleep_politely(delay)
    soup = BeautifulSoup(r.text, "html.parser")

    h1 = soup.find("h1")
    title = h1.get_text(strip=True) if h1 else slug

    version = ""
    m = re.search(r"Version\s+([\d.]+)", r.text, re.I)
    if m:
        version = f"v{m.group(1)}"

    license_text = ""
    license_url = ""
    for h3 in soup.find_all("h3"):
        if "license" in h3.get_text(strip=True).lower():
            block = h3.find_next_sibling()
            if block:
                a = block.find("a")
                if a:
                    license_url = a.get("href", "")
                license_text = block.get_text(" ", strip=True)
            break

    return {
        "title": title,
        "version": version,
        "license": license_text,
        "license_url": license_url,
        "detail_url": url,
        "download_url": (
            f"{LT_BASE}/actions/action_download_template"
            f"?template={slug}&type=Download+Template+Code"
        ),
    }


def lt_download_zip(
    session: requests.Session, slug: str, dl_url: str, dest: Path
) -> dict:
    r = fetch(session, dl_url, stream=True)
    r.raise_for_status()
    ct = r.headers.get("Content-Type", "")
    if not ("zip" in ct or "octet-stream" in ct):
        raise ValueError(f"Unexpected Content-Type for {slug!r}: {ct!r}")

    cd = r.headers.get("Content-Disposition", "")
    m = re.search(r'filename="?([^";\s]+)"?', cd)
    fname = m.group(1) if m else "source.zip"

    dest.mkdir(parents=True, exist_ok=True)
    archive = dest / "source.zip"
    hasher = hashlib.sha256()
    nbytes = 0
    with open(archive, "wb") as f:
        for chunk in r.iter_content(65536):
            f.write(chunk)
            hasher.update(chunk)
            nbytes += len(chunk)

    source_dir = dest / "source"
    with zipfile.ZipFile(archive) as zf:
        for name in zf.namelist():
            if name.startswith("/") or ".." in name:
                raise ValueError(f"Unsafe path in ZIP: {name!r}")
        zf.extractall(source_dir)

    return {
        "archive_filename": fname,
        "archive_sha256": hasher.hexdigest(),
        "archive_bytes": nbytes,
    }


def harvest_latextemplates(
    session: requests.Session,
    out: Path,
    manifest: dict,
    manifest_path: Path,
    limit: int,
    delay: float,
    dry_run: bool,
    resume: bool,
) -> int:
    # Collect slugs: essays first, then academic-paper categories
    essay_entries: list[tuple[str, str]] = []
    for cat in LT_ESSAY_CATS:
        print(f"  listing {LT_BASE}/cat/{cat} ...", flush=True)
        slugs = lt_list_slugs(session, cat, delay)
        print(f"    {len(slugs)} template(s)", flush=True)
        essay_entries.extend((s, "essay") for s in slugs)

    academic_entries: list[tuple[str, str]] = []
    for cat in LT_ACADEMIC_CATS:
        print(f"  listing {LT_BASE}/cat/{cat} ...", flush=True)
        slugs = lt_list_slugs(session, cat, delay)
        print(f"    {len(slugs)} template(s)", flush=True)
        academic_entries.extend((s, "academic-paper") for s in slugs)

    ordered: list[tuple[str, str]] = []
    seen: set[str] = set()
    for slug, bytetex_cat in essay_entries + academic_entries:
        if slug not in seen:
            seen.add(slug)
            ordered.append((slug, bytetex_cat))

    count = 0
    for slug, bytetex_cat in ordered:
        if count >= limit:
            break
        item_id = f"latextemplates:{slug}"
        dest = out / "latextemplates" / bytetex_cat / slug
        archive = dest / "source.zip"

        if resume and is_fetched(manifest, item_id, archive):
            print(f"  [skip] {item_id}", flush=True)
            continue

        print(f"  detail: {slug} ({bytetex_cat})", flush=True)
        try:
            detail = lt_detail(session, slug, delay)
        except Exception as exc:
            print(f"  [error] detail for {slug}: {exc}", file=sys.stderr)
            continue

        if dry_run:
            print(f"  [dry-run] {item_id}")
            print(f"    title:   {detail['title']}")
            print(f"    license: {detail['license'] or '(not found)'}")
            print(f"    zip:     {detail['download_url']}")
            count += 1
            continue

        print(f"  downloading: {slug}", flush=True)
        try:
            dl = lt_download_zip(session, slug, detail["download_url"], dest)
        except Exception as exc:
            print(f"  [error] download {slug}: {exc}", file=sys.stderr)
            continue
        sleep_politely(delay)

        entry: dict = {
            "id": item_id,
            "source": "latextemplates",
            "category": bytetex_cat,
            "slug": slug,
            "title": detail["title"],
            "detail_url": detail["detail_url"],
            "download_url": detail["download_url"],
            "archive_filename": dl["archive_filename"],
            "archive_sha256": dl["archive_sha256"],
            "archive_bytes": dl["archive_bytes"],
            "license": detail["license"],
            "license_url": detail["license_url"],
            "version": detail["version"],
            "fetched_at": _now(),
        }
        (dest / "meta.json").write_text(json.dumps(entry, indent=2) + "\n")
        manifest["entries"].append(entry)
        flush_manifest(manifest, manifest_path)
        print(f"  saved: {dest.relative_to(out.parent)}", flush=True)
        count += 1

    return count


# ─────────────────────────────────────────────────────────────────────────────
# arXiv
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


def arxiv_download_source(
    session: requests.Session, arxiv_id: str, dest: Path, delay: float
) -> dict:
    url = f"{ARXIV_EPRINT}/{arxiv_id}"
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
        return {
            "archive_filename": "source.tar.gz",
            "archive_sha256": hasher.hexdigest(),
            "archive_bytes": nbytes,
        }

    source_dir = dest / "source"
    try:
        with tarfile.open(archive, "r:gz") as tf:
            base = str(source_dir.resolve())
            for member in tf.getmembers():
                target = str((source_dir / member.name).resolve())
                if not target.startswith(base):
                    raise ValueError(f"Tar-slip detected: {member.name!r}")
            tf.extractall(source_dir)
    except tarfile.TarError:
        # Might be a single .tex file gzipped, not a tarball
        tex_out = source_dir / f"{arxiv_id.replace('/', '_')}.tex"
        tex_out.parent.mkdir(parents=True, exist_ok=True)
        with gzip.open(archive, "rb") as gz:
            tex_out.write_bytes(gz.read())

    return {
        "archive_filename": "source.tar.gz",
        "archive_sha256": hasher.hexdigest(),
        "archive_bytes": nbytes,
    }


def harvest_arxiv(
    session: requests.Session,
    out: Path,
    manifest: dict,
    manifest_path: Path,
    limit: int,
    delay: float,
    dry_run: bool,
    resume: bool,
    categories: list[str],
) -> int:
    actual_delay = max(delay, ARXIV_MIN_DELAY)
    # Distribute limit evenly across categories (ceiling per category)
    per_cat = max(1, -((-limit) // len(categories)))
    count = 0

    for cat in categories:
        if count >= limit:
            break
        n = min(per_cat, limit - count)
        print(f"  querying arXiv {cat!r} (n={n}) ...", flush=True)
        try:
            entries = arxiv_query(session, cat, n, actual_delay)
        except Exception as exc:
            print(f"  [error] arXiv query for {cat}: {exc}", file=sys.stderr)
            continue
        print(f"  got {len(entries)} result(s)", flush=True)

        for meta in entries:
            if count >= limit:
                break
            arxiv_id = meta["arxiv_id"]
            item_id = f"arxiv:{arxiv_id}"
            safe_id = arxiv_id.replace("/", "_")
            dest = out / "arxiv" / cat.replace(".", "_") / safe_id
            archive = dest / "source.tar.gz"

            if resume and is_fetched(manifest, item_id, archive):
                print(f"  [skip] {item_id}", flush=True)
                continue

            if dry_run:
                print(f"  [dry-run] {item_id}")
                print(f"    title:   {meta['title'][:72]}")
                print(f"    license: {meta['license_url'] or '(unknown)'}")
                print(f"    source:  {ARXIV_EPRINT}/{arxiv_id}")
                count += 1
                continue

            print(f"  fetching {item_id} — {meta['title'][:60]}", flush=True)
            try:
                dl = arxiv_download_source(session, arxiv_id, dest, actual_delay)
            except Exception as exc:
                print(f"  [error] {arxiv_id}: {exc}", file=sys.stderr)
                continue

            entry: dict = {
                "id": item_id,
                "source": "arxiv",
                "category": "academic-paper",
                "arxiv_primary_category": meta["primary_category"],
                "arxiv_id": arxiv_id,
                "title": meta["title"],
                "arxiv_published": meta["published"],
                "arxiv_authors": meta["authors"][:5],
                "detail_url": f"{ARXIV_ABS}/{arxiv_id}",
                "download_url": f"{ARXIV_EPRINT}/{arxiv_id}",
                "archive_filename": dl["archive_filename"],
                "archive_sha256": dl["archive_sha256"],
                "archive_bytes": dl["archive_bytes"],
                "license_url": meta["license_url"],
                "fetched_at": _now(),
            }
            (dest / "meta.json").write_text(json.dumps(entry, indent=2) + "\n")
            manifest["entries"].append(entry)
            flush_manifest(manifest, manifest_path)
            print(f"  saved: {dest.relative_to(out.parent)}", flush=True)
            count += 1

    return count


# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument(
        "--source",
        choices=["latextemplates", "arxiv", "all"],
        default="all",
        help="which source(s) to harvest (default: all)",
    )
    p.add_argument(
        "--limit",
        type=int,
        default=5,
        metavar="N",
        help="max items per source; ignored when --no-limit is set (default: 5)",
    )
    p.add_argument(
        "--no-limit",
        action="store_true",
        help="fetch all available items (large batch — confirm with the user first)",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="resolve URLs and print what would be downloaded; no writes",
    )
    p.add_argument(
        "--out",
        type=Path,
        default=Path("templates"),
        metavar="PATH",
        help="output directory (default: ./templates)",
    )
    p.add_argument(
        "--delay",
        type=float,
        default=2.0,
        metavar="SEC",
        help="base delay between requests in seconds (default: 2.0; arXiv enforces ≥3.0)",
    )
    p.add_argument(
        "--arxiv-category",
        action="append",
        dest="arxiv_cats",
        metavar="CAT",
        help="arXiv category to harvest (repeatable; default: cs.LG math.NA)",
    )
    p.add_argument(
        "--resume",
        action="store_true",
        help="skip items that are already present in the manifest",
    )
    p.add_argument("--user-agent", default=DEFAULT_UA, metavar="UA")
    args = p.parse_args()

    if not args.arxiv_cats:
        args.arxiv_cats = list(DEFAULT_ARXIV_CATS)

    limit = 9999 if args.no_limit else args.limit

    out = args.out.resolve()
    manifest_path = out / "manifest.json"
    manifest = load_manifest(manifest_path)
    session = make_session(args.user_agent)

    if not args.dry_run:
        out.mkdir(parents=True, exist_ok=True)

    total = 0

    if args.source in ("latextemplates", "all"):
        lt_lim = limit if args.source == "latextemplates" else max(1, round(limit * 0.6))
        print(f"\n=== latextemplates.com (limit={lt_lim}) ===")
        total += harvest_latextemplates(
            session, out, manifest, manifest_path,
            lt_lim, args.delay, args.dry_run, args.resume,
        )

    if args.source in ("arxiv", "all"):
        ax_lim = limit if args.source == "arxiv" else max(1, limit - round(limit * 0.6))
        print(f"\n=== arXiv (limit={ax_lim}, categories={args.arxiv_cats}) ===")
        total += harvest_arxiv(
            session, out, manifest, manifest_path,
            ax_lim, args.delay, args.dry_run, args.resume, args.arxiv_cats,
        )

    verb = "Would fetch" if args.dry_run else "Fetched"
    print(f"\n{verb} {total} item(s) total.")
    if not args.dry_run:
        print(f"Manifest: {manifest_path}")


if __name__ == "__main__":
    main()
