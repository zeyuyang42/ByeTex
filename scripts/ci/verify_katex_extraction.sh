#!/usr/bin/env bash
set -e
# Re-run extraction and verify the JSON matches what's checked in.
# Usage: run from repo root.
TMPFILE=$(mktemp)
python3 scripts/extract_katex.py --output "$TMPFILE"
if ! diff -q "$TMPFILE" crates/byetex-core/tests/data/katex_extracted.json > /dev/null; then
    echo "ERROR: katex_extracted.json is stale. Re-run scripts/extract_katex.py and commit the updated JSON."
    diff "$TMPFILE" crates/byetex-core/tests/data/katex_extracted.json
    exit 1
fi
echo "OK: katex_extracted.json is up to date."
