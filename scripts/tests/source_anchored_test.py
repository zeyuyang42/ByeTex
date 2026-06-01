#!/usr/bin/env python3
"""Unit tests for D4: source-anchored truth extraction in visual_test.py.

The truth heading/float lists were derived from `pdftotext` of the rendered
PDF, which on math-heavy papers pulls equation fragments in as bogus
"headings" (high false-negative heading_recall — see Phase-2b triage). Since
we HAVE the LaTeX source, derive the truth headings + float counts directly
from it: clean, ordered, and noise-free.

Run: uv run python scripts/tests/source_anchored_test.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import visual_test as vt  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


# ── source_headings: ordered, cleaned section titles ────────────────────────────
tex = r"""
\section{Introduction}
Some text.
\subsection{Background and Notation}
\section{The Main Result}\label{sec:main}
\subsection*{Acknowledgments}
"""
h = vt.source_headings(tex)
check(h == ["introduction", "background and notation", "the main result", "acknowledgments"],
      f"headings extracted in order, lowercased, label stripped; got {h}")

# Residue cleanup: \texorpdfstring{pdf}{tex}, math, trailing commands.
tex2 = r"""
\section{Theorem A: \texorpdfstring{$\Sigma$}{Sigma} necessity}
\subsection{The $L^2$ bound \label{sub:l2}}
"""
h2 = vt.source_headings(tex2)
check(len(h2) == 2, f"two headings; got {h2}")
check("texorpdfstring" not in " ".join(h2) and "$" not in " ".join(h2),
      f"LaTeX residue (texorpdfstring/math) cleaned; got {h2}")
check(h2[0].startswith("theorem a"), f"first heading keeps its prose; got {h2}")

# Commented-out sections are ignored.
tex3 = "\\section{Real}\n%\\section{Commented Out}\n\\section{Also Real}\n"
h3 = vt.source_headings(tex3)
check(h3 == ["real", "also real"], f"commented \\section ignored; got {h3}")

# No headings -> empty list.
check(vt.source_headings("just prose, no sections") == [], "no headings -> []")


# ── source_float_counts: figure/table environment counts ────────────────────────
floats = r"""
\begin{figure}\includegraphics{a}\caption{A}\end{figure}
\begin{table}\begin{tabular}{cc}x&y\end{tabular}\caption{T}\end{table}
\begin{figure*}\includegraphics{b}\caption{B}\end{figure*}
%\begin{figure}\caption{commented, ignore}\end{figure}
"""
fc = vt.source_float_counts(floats)
check(fc["figures"] == 2, f"2 figure envs (figure + figure*, comment ignored); got {fc}")
check(fc["tables"] == 1, f"1 table env; got {fc}")

check(vt.source_float_counts("no floats here") == {"figures": 0, "tables": 0},
      "no floats -> zeros")


# ── typ_headings: byetex's OWN .typ output headings (clean, marker-based) ────────
# The typst side must be anchored too, else clean-truth vs noisy-pdftotext-typst
# creates false misses. byetex emits `= H`, `== H <label>` markers.
typ = """#set page(paper: "us-letter")
= Introduction <sec:intro>
Body text = not a heading (no leading marker).
== Related Work
=== Phase 0: warm start
text == still not a heading
= Conclusions <sec:conc>
"""
th = vt.typ_headings(typ)
check(th == ["introduction", "related work", "phase 0: warm start", "conclusions"],
      f"typ headings from = / == markers, label stripped, lowercased; got {th}")
# A leading-marker line is required; inline '=' must not match.
check("not a heading" not in " ".join(th) and "still not a heading" not in " ".join(th),
      f"inline '=' must not be taken as a heading; got {th}")
check(vt.typ_headings("no headings here\njust text") == [], "no markers -> []")


if fails:
    print(f"\nTEST FAILED ({len(fails)} assertion(s))")
    sys.exit(1)
print("\nTEST PASSED")
