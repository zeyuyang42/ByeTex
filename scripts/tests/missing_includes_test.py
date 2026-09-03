"""`missing_includes` — the other missing-input guard.

A paper can be ingested without its content files. gh-pelegs-maths-book is missing
its chapter `\\input`s and converts to a single cover page: correct for the input,
identical in appearance to catastrophic content loss in the converter.
"""

import ast
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _load():
    # visual_test imports requests/pymupdf at module scope; the function under
    # test needs neither.
    src = (Path(__file__).resolve().parent.parent / "visual_test.py").read_text()
    fn = next(
        n for n in ast.parse(src).body
        if isinstance(n, ast.FunctionDef) and n.name == "missing_includes"
    )
    ns: dict = {}
    exec(compile(ast.Module([fn], []), "<extract>", "exec"), ns)
    return ns["missing_includes"]


def build(tmp: Path, files: dict[str, str]) -> Path:
    for name, body in files.items():
        p = tmp / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body)
    return tmp


def main() -> int:
    fn = _load()
    cases = [
        ("all present", {"m.tex": "\\input{ch1}\n", "ch1.tex": "hi\n"}, set()),
        ("one missing", {"m.tex": "\\input{ch1}\n"}, {"ch1"}),
        ("\\include too", {"m.tex": "\\include{ch2}\n"}, {"ch2"}),
        ("explicit .tex suffix", {"m.tex": "\\input{ch1.tex}\n", "ch1.tex": "hi\n"}, set()),
        ("target in a subdir", {"m.tex": "\\input{sec/ch1}\n", "sec/ch1.tex": "hi\n"}, set()),
        # A commented-out include is not a missing file.
        ("commented out", {"m.tex": "% \\input{ch1}\n"}, set()),
        # `\input{#1}` inside a macro body is a parameter, not a path.
        ("macro parameter", {"m.tex": "\\newcommand{\\x}[1]{\\input{#1}}\n"}, set()),
    ]
    failures = 0
    for label, files, want in cases:
        with tempfile.TemporaryDirectory() as td:
            root = build(Path(td), files)
            got = fn(root, root / "m.tex")
        if got != want:
            print(f"  FAIL {label}: got {got}, want {want}")
            failures += 1
        else:
            print(f"  ok   {label}")
    print(f"\n{len(cases) - failures}/{len(cases)} passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
