"""`_deps_bin_dirs` — the provisioned biber must be found from a WORKTREE too.

`.truth-deps/` is gitignored and lives in whichever checkout ran
`setup_truth_deps.sh`, so a worktree does not have one. The PATH prepend then
silently no-ops and tectonic falls back to the system biber — which is how both
biblatex theses in the corpus ended up `truth_render_failed` (Homebrew biber 2.21
expects control-file 3.11; tectonic's bundled biblatex 3.17 emits 3.8).
"""

import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import truth_render as T  # noqa: E402


def main() -> int:
    failures = 0

    dirs = T._deps_bin_dirs()
    env = T._truth_render_env()

    # Whatever the checkout, every returned path must actually exist — the
    # function's whole job is to hand tectonic a real directory.
    for d in dirs:
        if not Path(d).is_dir():
            print(f"  FAIL returned a non-directory: {d}")
            failures += 1
    if not failures:
        print(f"  ok   every returned dir exists ({len(dirs)} found)")

    # In a worktree the main checkout's copy has to be reachable, or the prepend
    # is a no-op exactly where the gate usually runs.
    common = subprocess.run(
        ["git", "rev-parse", "--git-common-dir"],
        cwd=T.REPO_ROOT, capture_output=True, text=True,
    ).stdout.strip()
    in_worktree = common and not (T.REPO_ROOT / ".git").is_dir()
    if in_worktree:
        main_deps = (T.REPO_ROOT / common).resolve().parent / ".truth-deps" / "bin"
        if main_deps.is_dir() and main_deps not in [Path(d) for d in dirs]:
            print(f"  FAIL worktree run did not pick up {main_deps}")
            failures += 1
        else:
            print("  ok   worktree resolves the main checkout's .truth-deps")
    else:
        print("  skip not running in a worktree")

    # The prepend must WIN over whatever is already on PATH, or a system biber
    # of the wrong version still gets used.
    if dirs:
        first = env["PATH"].split(":")[0]
        if first != str(dirs[0]):
            print(f"  FAIL PATH does not start with the provisioned bin: {first}")
            failures += 1
        else:
            print("  ok   provisioned bin is first on PATH")

    print(f"\n{'FAILED' if failures else 'passed'}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
