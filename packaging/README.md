# Packaging & distribution

ByeTex ships two ways. The `byetex` binary is the same everywhere; the Claude
Code plugin (bundled skills) is a separate artifact that needs the binary
on PATH.

> Not published to crates.io, and there is no Homebrew tap. `cargo install byetex`
> and `brew install …` do **not** work — use the install script below, or
> `cargo install --git https://github.com/zeyuyang42/ByeTex byetex` from source.

## 1. Install script (prebuilt binary)

```bash
curl -fsSL https://raw.githubusercontent.com/zeyuyang42/ByeTex/main/install.sh | sh
```

Downloads the matching `byetex-<tag>-<target>.tar.gz` from GitHub Releases
(built by `.github/workflows/release.yml`) into `~/.local/bin`. Override with
`BYETEX_VERSION` / `BYETEX_BINDIR`. Windows: download the `.zip` from Releases.

## 2. Claude Code plugin

```bash
claude plugin marketplace add zeyuyang42/ByeTex
claude plugin install byetex@byetex
```

Bundles the 14 repair/grading skills. Install the binary first
(see above) — the plugin's `SessionStart` hook reminds you if it is
missing.
