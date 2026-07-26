# Plugin setup (Claude Code)

ByeTex ships as a Claude Code plugin that bundles the 14 repair skills. The plugin
needs the `byetex` binary on PATH — install it first.

## 1. Install the binary

```bash
curl -fsSL https://raw.githubusercontent.com/zeyuyang42/ByeTex/main/install.sh | sh
# or build from source:
cargo install --git https://github.com/zeyuyang42/ByeTex byetex
```

## 2. Install the plugin (Claude Code)

```bash
claude plugin marketplace add zeyuyang42/ByeTex
claude plugin install byetex@byetex
```

Skills then appear as `/byetex:<name>`. A `SessionStart` hook warns if the `byetex`
binary isn't found on PATH.

## Verify

```bash
byetex --version
byetex skills list        # 14 skills
```
