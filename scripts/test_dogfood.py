"""Unit tests for scripts/dogfood.py select ranking.

Run via: uv run --with pytest python -m pytest scripts/test_dogfood.py
"""
import importlib.util
import os
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


# ─── binary resolution ───────────────────────────────────────────────────────
# Regression: `_byetex()` used to be `os.environ.get("BYETEX_BIN", "byetex")`, so
# with BYETEX_BIN unset it silently fell through to whatever `byetex` was on PATH
# — in practice the install.sh copy in ~/.local/bin, which was v0.3.0, ~40
# releases behind. A whole dogfood round was seeded by that stale binary and
# produced `blocker`-severity findings against skills that were telling the truth
# (2026-08-14). The harness must prefer the REPO's own build and must never
# resolve a binary silently.


def test_byetex_bin_env_wins(monkeypatch):
    monkeypatch.setenv("BYETEX_BIN", "/custom/path/byetex")
    assert dogfood.resolve_byetex() == "/custom/path/byetex"


def _decoy_on_path(monkeypatch, tmp_path, name="byetex"):
    """Put an executable `byetex` on PATH — the stale ~/.local/bin/byetex stand-in."""
    bin_dir = tmp_path / "decoy-bin"
    bin_dir.mkdir()
    decoy = bin_dir / name
    decoy.write_text("#!/bin/sh\necho 'byetex 0.3.0'\n")
    decoy.chmod(0o755)
    monkeypatch.setenv("PATH", str(bin_dir))
    return decoy


def test_repo_build_preferred_over_path(monkeypatch, tmp_path):
    # No BYETEX_BIN, and a `byetex` IS on PATH: the repo build must still win.
    # Without the decoy this test would pass against the old buggy
    # `os.environ.get("BYETEX_BIN", "byetex")` too, so the decoy is what pins it.
    monkeypatch.delenv("BYETEX_BIN", raising=False)
    decoy = _decoy_on_path(monkeypatch, tmp_path)
    repo_bin = tmp_path / "target" / "release" / "byetex"
    repo_bin.parent.mkdir(parents=True)
    repo_bin.write_text("#!/bin/sh\n")
    repo_bin.chmod(0o755)
    monkeypatch.setattr(dogfood, "REPO_ROOT", tmp_path)
    got = dogfood.resolve_byetex()
    assert got == str(repo_bin), got
    assert got != str(decoy) and got != "byetex", got


def test_missing_repo_build_is_a_hard_error(monkeypatch, tmp_path):
    # Falling back to PATH is exactly how the v0.3.0 seeding happened. Even with a
    # perfectly usable `byetex` on PATH, no repo build ⇒ refuse rather than guess.
    monkeypatch.delenv("BYETEX_BIN", raising=False)
    _decoy_on_path(monkeypatch, tmp_path)
    monkeypatch.setattr(dogfood, "REPO_ROOT", tmp_path)
    try:
        dogfood.resolve_byetex()
    except SystemExit as e:
        assert "cargo build --release" in str(e), str(e)
    else:
        raise AssertionError("expected SystemExit, got a silent PATH fallback")


def test_version_token_is_compared_exactly(monkeypatch, tmp_path):
    # Substring matching would call 0.7.3 == 0.7.31 and stay silent.
    assert dogfood._version_token("byetex 0.7.3") == "0.7.3"
    assert dogfood._version_token("byetex 0.7.31") == "0.7.31"
    assert dogfood._version_token("no digits here") is None


def test_workspace_version_reads_workspace_package(tmp_path, monkeypatch):
    (tmp_path / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["a"]\n\n'
        '[workspace.dependencies]\nversion = "9.9.9"\n\n'
        '[workspace.package]\nversion = "0.7.4"\nedition = "2021"\n'
    )
    monkeypatch.setattr(dogfood, "REPO_ROOT", tmp_path)
    # Must pick [workspace.package], not the decoy under [workspace.dependencies].
    assert dogfood.workspace_version() == "0.7.4"


def test_unversionable_binary_is_a_hard_error(monkeypatch, tmp_path):
    # A binary that runs but cannot report itself must stop the run, not degrade
    # into an "unknown" version warning while the round proceeds.
    failing = tmp_path / "byetex"
    failing.write_text("#!/bin/sh\nexit 3\n")
    failing.chmod(0o755)
    monkeypatch.setenv("BYETEX_BIN", str(failing))
    try:
        dogfood.report_byetex_identity()
    except SystemExit as e:
        assert "--version` failed" in str(e), str(e)
    else:
        raise AssertionError("expected SystemExit for an unversionable binary")


def test_binary_older_than_sources_is_detected(monkeypatch, tmp_path):
    # The COMMON staleness: edited .rs, forgot to rebuild — version still matches.
    repo_bin = tmp_path / "target" / "release" / "byetex"
    repo_bin.parent.mkdir(parents=True)
    repo_bin.write_text("#!/bin/sh\n")
    repo_bin.chmod(0o755)
    src = tmp_path / "crates" / "byetex-core" / "src"
    src.mkdir(parents=True)
    rs = src / "emit.rs"
    rs.write_text("// edited after the build\n")
    os.utime(repo_bin, (1_000_000, 1_000_000))
    os.utime(rs, (2_000_000, 2_000_000))
    monkeypatch.delenv("BYETEX_BIN", raising=False)
    monkeypatch.setattr(dogfood, "REPO_ROOT", tmp_path)
    reason = dogfood.binary_is_older_than_sources()
    assert reason is not None and "emit.rs" in reason, reason
    # And the reverse: a fresh build reports no staleness.
    os.utime(repo_bin, (3_000_000, 3_000_000))
    assert dogfood.binary_is_older_than_sources() is None
