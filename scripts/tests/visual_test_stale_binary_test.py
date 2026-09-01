#!/usr/bin/env python3
# requires: requests Pillow
"""Unit test for visual_test.ensure_byetex — that the fidelity gate measures the
converter actually in the working tree.

`ensure_byetex` used to be `if not bin_path.exists(): cargo build`, so an
existing-but-STALE binary was measured in silence. That is how a gate run right
after a rebase reported three beamer papers as `layout_body_font_ratio`
REGRESSIONs: it ran the pre-fix converter against the post-fix truth. The tell was
a measured value identical to the committed baseline — the fix was simply not in
the binary.

The fix is to build unconditionally and let cargo decide what is stale. An mtime
heuristic here would be wrong twice: it cannot see the non-Rust inputs
(`build.rs` compiles the vendored tree-sitter `parser.c`), and it would never
clear, since `cargo build -p byetex` does not compile `crates/**/tests/*.rs` and
those are usually the newest `.rs` files after a TDD tick.

Run: uv run --with requests --with Pillow python scripts/tests/visual_test_stale_binary_test.py
"""
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import visual_test as vt  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


class FakeCargo:
    """Stands in for subprocess.run; records calls and optionally makes the binary."""

    def __init__(self, root: Path, *, produces: bool = True):
        self.root, self.produces, self.calls = root, produces, []

    def __call__(self, cmd, **kw):
        self.calls.append(cmd)
        if self.produces:
            out = self.root / "target" / "release" / "byetex"
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_text("binary")
        return None


def with_fake(root: Path, *, produces: bool = True) -> FakeCargo:
    fake = FakeCargo(root, produces=produces)
    vt.REPO_ROOT, vt.subprocess.run = root, fake
    return fake


real_root, real_run = vt.REPO_ROOT, vt.subprocess.run
try:
    # 1. THE BUG: a binary is already present. The old code returned it untouched;
    #    it must now still go through cargo, which is what decides staleness.
    with TemporaryDirectory() as d:
        root = Path(d)
        stale = root / "target" / "release" / "byetex"
        stale.parent.mkdir(parents=True)
        stale.write_text("stale binary")
        fake = with_fake(root)
        vt.ensure_byetex("release")
        check(len(fake.calls) == 1, "an existing binary is still rebuilt (cargo decides, not us)")
        check(
            fake.calls and fake.calls[0][:4] == ["cargo", "build", "-p", "byetex"],
            "invokes `cargo build -p byetex`",
        )
        check(fake.calls and "--release" in fake.calls[0], "release profile passes --release")

    # 2. A missing binary still builds — the case the old code did handle.
    with TemporaryDirectory() as d:
        root = Path(d)
        fake = with_fake(root)
        got = vt.ensure_byetex("release")
        check(len(fake.calls) == 1, "a missing binary is built")
        check(got.exists(), "returns a path that exists")

    # 3. cargo can succeed while writing the binary elsewhere (CARGO_TARGET_DIR, a
    #    custom profile). Say so once instead of failing inside all 71 papers.
    with TemporaryDirectory() as d:
        root = Path(d)
        with_fake(root, produces=False)
        try:
            vt.ensure_byetex("release")
            check(False, "a build that produces no binary must not be measured")
        except SystemExit as e:
            check("does not exist" in str(e), "a build that produces no binary raises SystemExit")

    # 4. The debug profile must not silently measure the release binary.
    with TemporaryDirectory() as d:
        root = Path(d)
        fake = with_fake(root, produces=False)
        try:
            vt.ensure_byetex("debug")
        except SystemExit:
            pass
        check(fake.calls and "--release" not in fake.calls[0], "debug profile omits --release")
finally:
    vt.REPO_ROOT, vt.subprocess.run = real_root, real_run

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
