#!/usr/bin/env python3
"""
Populate corpus/inhouse/ from the committed tests/inhouse/ source-of-truth.

Run this once before using bytetex + typst against the inhouse templates,
so that generated outputs (.typ, .warnings.json, .pdf) are written inside
corpus/ and never pollute the committed tests/ tree.

Usage:
    python scripts/setup_corpus.py

Idempotent: re-running updates corpus/inhouse/ with any new files from
tests/inhouse/ without removing files already there.
"""

import shutil
from pathlib import Path

REPO = Path(__file__).parent.parent.resolve()
SRC = REPO / "tests" / "inhouse"
CORPUS = REPO / "corpus"
INHOUSE_DST = CORPUS / "inhouse"
ONLINE_ARXIV = CORPUS / "online" / "arxiv"


def main() -> None:
    print("Setting up corpus/ ...")

    # corpus/inhouse/ — copy from committed source
    if SRC.exists():
        shutil.copytree(SRC, INHOUSE_DST, dirs_exist_ok=True)
        print(f"  corpus/inhouse/   <- tests/inhouse/ ({sum(1 for _ in INHOUSE_DST.rglob('*'))} entries)")
    else:
        print(f"  [warn] {SRC} not found; skipping inhouse copy")

    # corpus/online/arxiv/ — ensure dir exists for harvester output
    ONLINE_ARXIV.mkdir(parents=True, exist_ok=True)
    print(f"  corpus/online/arxiv/  ready")

    print("Done. Generated outputs (*.typ, *.warnings.json, *.pdf) go into corpus/ and are gitignored.")


if __name__ == "__main__":
    main()
