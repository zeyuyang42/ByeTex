"""`cites_without_bibliography` — the missing-input guard for text recall.

A paper that uses \\cite but ships no .bib/.bbl cannot generate its own reference
list, while the truth PDF has one. Every text-recall metric then understates the
converter by that much, and the deficit looks exactly like a converter gap.
"""

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def _load():
    # visual_test imports requests/pymupdf at module scope; the function under
    # test needs neither, so read it out rather than importing the module.
    import ast

    src = (Path(__file__).resolve().parent.parent / "visual_test.py").read_text()
    tree = ast.parse(src)
    fn = next(
        n for n in tree.body
        if isinstance(n, ast.FunctionDef) and n.name == "cites_without_bibliography"
    )
    ns: dict = {}
    exec(compile(ast.Module([fn], []), "<extract>", "exec"), ns)
    return ns["cites_without_bibliography"]


def make(tmp: Path, files: dict[str, str]) -> Path:
    for name, body in files.items():
        p = tmp / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body)
    return tmp


def main() -> int:
    fn = _load()
    cases = [
        ("cites, no bib", {"m.tex": "x \\cite{k}\n"}, True),
        ("cites, has bib", {"m.tex": "x \\cite{k}\n", "r.bib": "@a{k}\n"}, False),
        ("cites, has bbl", {"m.tex": "x \\cite{k}\n", "m.bbl": "\\bibitem{k}\n"}, False),
        ("bib in a subdir", {"m.tex": "x \\cite{k}\n", "sub/r.bib": "@a{k}\n"}, False),
        ("no citations at all", {"m.tex": "plain text\n"}, False),
        # The reason the check is `\cite` and not `\bibliography`: a paper can
        # declare `\bibliography{custom}` with no such file, which is exactly
        # corpus 2605.31563.
        ("declares a missing bib file", {"m.tex": "\\cite{k}\\bibliography{custom}\n"}, True),
    ]
    failures = 0
    for label, files, want in cases:
        with tempfile.TemporaryDirectory() as td:
            got = fn(make(Path(td), files))
        if got != want:
            print(f"  FAIL {label}: got {got}, want {want}")
            failures += 1
        else:
            print(f"  ok   {label}")
    print(f"\n{len(cases) - failures}/{len(cases)} passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
