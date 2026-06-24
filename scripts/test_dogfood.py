"""Unit tests for scripts/dogfood.py select ranking.

Run via: uv run --with pytest python -m pytest scripts/test_dogfood.py
"""
import importlib.util
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "dogfood", Path(__file__).with_name("dogfood.py")
)
dogfood = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(dogfood)


def _fid(**papers):
    return papers


def test_truth_render_failed_papers_are_excluded():
    # A `truth_render_failed` paper has no truth → dogfood `score` cannot compute
    # fidelity_after, so it is an un-scoreable (degenerate) target and must NOT be
    # selected, even though its structure_ok is False.
    fid = _fid(
        **{
            "ctan-memoir": {"status": "truth_render_failed",
                            "structure_ok": False, "word_recall": None},
            "2605.00001": {"status": "ok", "structure_ok": True, "word_recall": 0.70},
        }
    )
    ranked = dogfood.rank_candidates(fid, known_fail=set(),
                                     corpus_ids=["ctan-memoir", "2605.00001"])
    ids = [r["id"] for r in ranked]
    assert "ctan-memoir" not in ids, ids
    assert ids == ["2605.00001"], ids


def test_measured_hard_papers_rank_above_high_recall():
    fid = _fid(
        **{
            "low": {"status": "structure_failed", "structure_ok": False, "word_recall": 0.50},
            "mid": {"status": "ok", "structure_ok": True, "word_recall": 0.67},
            "high": {"status": "ok", "structure_ok": True, "word_recall": 0.95},
        }
    )
    ranked = dogfood.rank_candidates(fid, known_fail=set(),
                                     corpus_ids=["high", "mid", "low"])
    assert [r["id"] for r in ranked] == ["low", "mid", "high"], ranked


def test_known_fail_kept_even_if_unmeasured():
    # A BYETEX compile failure (known_fail) is still dogfoodable — the agent can
    # repair the compile — so it is kept even with word_recall None.
    fid = _fid(
        **{
            "broken": {"word_recall": None},
            "ok": {"status": "ok", "structure_ok": True, "word_recall": 0.80},
        }
    )
    ranked = dogfood.rank_candidates(fid, known_fail={"broken"},
                                     corpus_ids=["broken", "ok"])
    ids = [r["id"] for r in ranked]
    assert ids[0] == "broken", ids
