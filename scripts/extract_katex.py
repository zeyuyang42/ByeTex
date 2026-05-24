#!/usr/bin/env python3
"""
Extract KaTeX math commands from vendor/katex source files.

Outputs:
  crates/byetex-core/tests/data/katex_extracted.json  -- machine-readable for Rust tests
  scripts/output/symbol_draft.toml                     -- human-readable translation worklist

Usage:
  python3 scripts/extract_katex.py [--output PATH]

--output PATH redirects the JSON to PATH instead of the default location (for CI diff checks).
"""

import re
import json
import os
import sys
import unicodedata
from pathlib import Path
from datetime import date

# ---------------------------------------------------------------------------
# Paths (resolved relative to repo root regardless of cwd)
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parent.parent
KATEX_SRC = REPO_ROOT / "vendor" / "katex" / "src"
SYMBOLS_JS = KATEX_SRC / "symbols.js"
MACROS_JS = KATEX_SRC / "macros.js"
FUNCTIONS_DIR = KATEX_SRC / "functions"

DEFAULT_OUTPUT_JSON = REPO_ROOT / "crates" / "byetex-core" / "tests" / "data" / "katex_extracted.json"
OUTPUT_TOML = REPO_ROOT / "scripts" / "output" / "symbol_draft.toml"

KATEX_VERSION = "v0.16.11"

# ---------------------------------------------------------------------------
# Excluded function files (HTML/CSS/layout only, no math equivalents)
# ---------------------------------------------------------------------------

EXCLUDED_FILES = {
    "href.js", "html.js", "htmlmathml.js", "color.js", "raisebox.js",
    "lap.js", "hbox.js", "vcenter.js", "kern.js", "rule.js",
    "mathchoice.js", "verb.js", "tag.js", "includegraphics.js",
    "cr.js", "relax.js", "smash.js", "pmb.js", "char.js",
    "ordgroup.js", "environment.js",
}

# The task spec lists .ts extensions but the actual files are .js
# Normalize: we use .js
EXCLUDED_FILES_NORMALIZED = {
    f.replace(".ts", ".js") for f in EXCLUDED_FILES
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def decode_js_unicode(s: str) -> str:
    r"""Convert JS \uXXXX escape sequences to actual Unicode characters."""
    return re.sub(
        r'\\u([0-9a-fA-F]{4})',
        lambda m: chr(int(m.group(1), 16)),
        s,
    )


def unicode_name(char: str) -> str:
    """Return the Unicode name of a character, or '' if unknown."""
    try:
        if not char:
            return ""
        if len(char) == 1:
            return unicodedata.name(char)
        # Multi-char string: name the first char only
        return unicodedata.name(char[0])
    except (ValueError, TypeError):
        return ""


# ---------------------------------------------------------------------------
# Parse symbols.js
# ---------------------------------------------------------------------------
#
# Format:
#   defineSymbol(math, main, rel, "≡", "\\equiv", true);
#   defineSymbol(math, main, mathord, "α", "\\alpha");
#
# The arguments are positional variables (not string literals) for mode/font/group,
# followed by string literals for unicode and name.
#
# We need to handle the JS variable abbreviations:
#   const math = "math"; const text = "text";
#   const main = "main"; const ams = "ams";
#   const accent = "accent-token"; const bin = "bin"; ...


def parse_symbols(content: str) -> list[dict]:
    """Extract all defineSymbol calls from symbols.js content."""
    # Map JS variable abbreviations to their string values
    # These are defined in the file itself
    mode_map = {"math": "math", "text": "text"}
    font_map = {"main": "main", "ams": "ams"}
    group_map = {
        "accent": "accent-token",
        "bin": "bin",
        "close": "close",
        "inner": "inner",
        "mathord": "mathord",
        "op": "op-token",
        "open": "open",
        "punct": "punct",
        "rel": "rel",
        "spacing": "spacing",
        "textord": "textord",
    }

    symbols = []

    # Find all defineSymbol calls — handle possible multi-line
    # Strategy: find each 'defineSymbol(' position, then extract balanced parens
    i = 0
    while True:
        pos = content.find("defineSymbol(", i)
        if pos == -1:
            break
        # Skip the export function definition itself
        # Look back for 'function' within 10 chars
        prefix = content[max(0, pos - 20):pos]
        if "function" in prefix:
            i = pos + 1
            continue

        # Extract the balanced parenthesized content
        start = pos + len("defineSymbol(")
        depth = 1
        j = start
        while j < len(content) and depth > 0:
            c = content[j]
            if c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
            elif c == '"':
                # Skip string content
                j += 1
                while j < len(content):
                    if content[j] == '\\':
                        j += 2
                        continue
                    if content[j] == '"':
                        break
                    j += 1
            j += 1

        raw_args = content[start:j - 1]  # exclude closing paren
        i = j

        # Parse the 5 arguments:
        # arg0: mode variable name
        # arg1: font variable name
        # arg2: group variable name
        # arg3: unicode string literal (may contain \uXXXX)
        # arg4: name string literal (the LaTeX command)
        # arg5: optional boolean

        args = split_js_args(raw_args)
        if len(args) < 5:
            continue

        mode_var = args[0].strip()
        font_var = args[1].strip()
        group_var = args[2].strip()
        unicode_raw = args[3].strip()
        name_raw = args[4].strip()

        # Decode string literals
        if not (unicode_raw.startswith('"') and name_raw.startswith('"')):
            continue

        unicode_str = decode_js_unicode(unicode_raw[1:-1])
        # name_raw is a JS string literal like "\\alpha" where \\ is an escaped backslash.
        # Strip outer quotes and decode the JS escape: \\ -> \
        name_inner = name_raw[1:-1]  # remove surrounding quotes
        name_str = name_inner.replace("\\\\", "\\")

        if not name_str.startswith("\\"):
            continue

        mode = mode_map.get(mode_var, mode_var)
        font = font_map.get(font_var, font_var)
        group = group_map.get(group_var, group_var)

        symbols.append({
            "name": name_str,
            "unicode": unicode_str,
            "group": group,
            "mode": mode,
            "font": font,
        })

    return symbols


def split_js_args(raw: str) -> list[str]:
    """Split a comma-separated JS argument list, respecting string literals."""
    args = []
    current = []
    depth = 0
    in_string = False
    string_char = None
    i = 0
    while i < len(raw):
        c = raw[i]
        if in_string:
            if c == '\\':
                current.append(c)
                i += 1
                if i < len(raw):
                    current.append(raw[i])
            elif c == string_char:
                in_string = False
                current.append(c)
            else:
                current.append(c)
        elif c in ('"', "'"):
            in_string = True
            string_char = c
            current.append(c)
        elif c in ('(', '[', '{'):
            depth += 1
            current.append(c)
        elif c in (')', ']', '}'):
            depth -= 1
            current.append(c)
        elif c == ',' and depth == 0:
            args.append(''.join(current))
            current = []
        else:
            current.append(c)
        i += 1
    if current:
        args.append(''.join(current))
    return args


# ---------------------------------------------------------------------------
# Parse macros.js
# ---------------------------------------------------------------------------
#
# We only want string-replacement macros, not function-handler macros.
# Format of string replacements:
#   defineMacro("\\name", "body string");
# Format of function handlers (skip):
#   defineMacro("\\name", function(context) { ... });
#   defineMacro("\\name", (context) => { ... });


def parse_macros(content: str) -> list[dict]:
    """Extract string-replacement defineMacro calls from macros.js."""
    # Pattern: defineMacro("\\name", "body")
    # Both args must be quoted strings (not functions)
    pattern = re.compile(
        r'defineMacro\(\s*("\\\\[^"]*")\s*,\s*("[^"]*")\s*\)'
    )

    macros = []
    for m in pattern.finditer(content):
        name_raw = m.group(1)[1:-1]   # strip outer quotes
        body_raw = m.group(2)[1:-1]   # strip outer quotes

        # name_raw is like \\alpha (JS escaped) -> \alpha (Python)
        name = name_raw.replace("\\\\", "\\")

        # body_raw is the JS string body; unescape JS unicode
        body = decode_js_unicode(body_raw)

        # Count params: max #N found in body
        param_nums = [int(x) for x in re.findall(r'#(\d+)', body)]
        params = max(param_nums) if param_nums else 0

        macros.append({
            "name": name,
            "body": body,
            "params": params,
        })

    return macros


# ---------------------------------------------------------------------------
# Parse functions/*.js
# ---------------------------------------------------------------------------
#
# Each file may have one or more defineFunction({...}) calls.
# We extract the `names` array and `numArgs` from each call.


def extract_js_string_array(block: str, key: str) -> list[str]:
    """Extract the string items from a JS array like: key: ["\\foo", "\\bar", ...].

    Handles multi-line arrays, inline comments (// ...), and skips internal
    commands that use four backslashes (\\\\name = internal, not user-enterable).
    Returns decoded names (JS \\\\ -> single backslash).
    """
    # Find `key:` followed by `[`
    m = re.search(r'\b' + re.escape(key) + r'\s*:\s*\[', block)
    if not m:
        return []

    # Find the balanced closing bracket
    start = m.end() - 1  # position of '['
    depth = 0
    j = start
    while j < len(block):
        c = block[j]
        if c == '[':
            depth += 1
        elif c == ']':
            depth -= 1
            if depth == 0:
                break
        j += 1

    array_content = block[start + 1:j]

    # Strip line comments (// ...) before splitting
    # Remove from // to end of line
    array_no_comments = re.sub(r'//[^\n]*', '', array_content)

    # Extract all quoted strings from the array
    names = []
    for str_match in re.finditer(r'"([^"]*)"', array_no_comments):
        raw = str_match.group(1)
        # Skip internal commands (4 backslashes in raw file = 2 backslashes = \\name, internal)
        # e.g. "\\\\atopfrac" in file = \\atopfrac internal
        # These have 4 backslash chars in the raw file
        if raw.startswith("\\\\\\\\"):
            continue
        # Decode JS escape: \\ -> \
        decoded = raw.replace("\\\\", "\\")
        if decoded.startswith("\\") or len(decoded) == 1:
            names.append(decoded)

    return names


def parse_function_file(filepath: Path) -> list[dict]:
    """Extract defineFunction calls from a single function file."""
    content = filepath.read_text(encoding="utf-8")
    filename = filepath.name
    kind = filepath.stem  # e.g. "accent" from "accent.js"

    results = []

    # Find all defineFunction({ ... }) blocks
    i = 0
    while True:
        pos = content.find("defineFunction({", i)
        if pos == -1:
            break

        # Extract balanced brace block
        start = pos + len("defineFunction(")
        depth = 0
        j = start
        in_str = False
        str_char = None
        while j < len(content):
            c = content[j]
            if in_str:
                if c == '\\':
                    j += 2
                    continue
                if c == str_char:
                    in_str = False
            elif c in ('"', "'", '`'):
                in_str = True
                str_char = c
            elif c == '{':
                depth += 1
            elif c == '}':
                depth -= 1
                if depth == 0:
                    j += 1
                    break
            j += 1

        block = content[start:j]
        i = j

        # Extract names array — parse the JS array properly
        names = extract_js_string_array(block, "names")

        if not names:
            continue

        # Extract numArgs
        num_args = 0
        num_args_match = re.search(r'numArgs:\s*(\d+)', block)
        if num_args_match:
            num_args = int(num_args_match.group(1))

        results.append({
            "file": filename,
            "names": names,
            "num_args": num_args,
            "kind": kind,
        })

    return results


def parse_functions(functions_dir: Path, excluded: set) -> tuple[list[dict], list[str]]:
    """Parse all non-excluded function files. Returns (entries, excluded_list)."""
    all_entries = []
    excluded_files_found = []

    for filepath in sorted(functions_dir.glob("*.js")):
        fname = filepath.name
        if fname in excluded:
            excluded_files_found.append(fname)
            continue
        entries = parse_function_file(filepath)
        all_entries.extend(entries)

    return all_entries, excluded_files_found


# ---------------------------------------------------------------------------
# Symbol draft TOML
# ---------------------------------------------------------------------------

def write_symbol_draft_toml(symbols: list[dict], output_path: Path) -> None:
    """Write a TOML worklist for manual symbol translation."""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# Manual translation worklist. Fill `final` column with Typst symbol name.",
        "# Generated by scripts/extract_katex.py — re-run to refresh.",
        "",
    ]
    for sym in symbols:
        uname = unicode_name(sym["unicode"]) if sym["unicode"] else ""
        lines.append("[[symbol]]")
        lines.append(f'name = "{sym["name"]}"')
        lines.append(f'unicode = "{sym["unicode"]}"')
        lines.append(f'unicode_name = "{uname}"')
        lines.append('final = ""  # TODO: fill in (e.g. "subset.eq")')
        lines.append("")

    output_path.write_text("\n".join(lines), encoding="utf-8")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    # Parse CLI args
    output_path = DEFAULT_OUTPUT_JSON
    args = sys.argv[1:]
    if "--output" in args:
        idx = args.index("--output")
        if idx + 1 < len(args):
            output_path = Path(args[idx + 1])

    # Verify KaTeX source exists
    if not SYMBOLS_JS.exists():
        print(f"ERROR: {SYMBOLS_JS} not found. Run: git submodule update --init", file=sys.stderr)
        sys.exit(1)

    # Parse symbols
    print("Parsing symbols.js...")
    symbols_content = SYMBOLS_JS.read_text(encoding="utf-8")
    symbols = parse_symbols(symbols_content)
    print(f"  Found {len(symbols)} symbol definitions")

    # Parse macros
    print("Parsing macros.js...")
    macros_content = MACROS_JS.read_text(encoding="utf-8")
    macros = parse_macros(macros_content)
    print(f"  Found {len(macros)} string-replacement macros")

    # Parse functions
    print("Parsing functions/*.js...")
    functions, excluded_found = parse_functions(FUNCTIONS_DIR, EXCLUDED_FILES_NORMALIZED)
    print(f"  Found {len(functions)} function entries from non-excluded files")
    print(f"  Excluded {len(excluded_found)} files")

    # Build output JSON
    today = str(date.today())
    data = {
        "katex_version": KATEX_VERSION,
        "extracted_at": today,
        "symbols": symbols,
        "macros": macros,
        "functions": functions,
        "excluded_files": sorted(excluded_found),
    }

    # Write JSON
    output_path.parent.mkdir(parents=True, exist_ok=True)
    json_text = json.dumps(data, indent=2, ensure_ascii=False)
    output_path.write_text(json_text, encoding="utf-8")
    print(f"Wrote JSON: {output_path}")

    # Write symbol draft TOML
    write_symbol_draft_toml(symbols, OUTPUT_TOML)
    print(f"Wrote TOML: {OUTPUT_TOML}")

    # Summary
    print(f"\nSummary:")
    print(f"  Symbols:   {len(symbols)}")
    print(f"  Macros:    {len(macros)}")
    print(f"  Functions: {len(functions)}")


if __name__ == "__main__":
    main()
