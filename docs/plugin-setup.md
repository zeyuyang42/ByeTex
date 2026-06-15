# Plugin setup (Claude Code / Cursor)

ByeTex ships as a Claude Code plugin that bundles the 12 skills and auto-registers
the `byetex serve` MCP server (11 tools). The plugin needs the `byetex` binary on
PATH — install it first.

## 1. Install the binary

```bash
curl -fsSL https://raw.githubusercontent.com/zeyuyang42/ByeTex/main/install.sh | sh
# or build from source:
cargo install --git https://github.com/zeyuyang42/ByeTex byetex --features mcp
```

> `cargo install byetex` (crates.io) and `brew install` (a Homebrew tap) are coming soon.

## 2. Install the plugin (Claude Code)

```bash
claude plugin marketplace add zeyuyang42/ByeTex
claude plugin install byetex@byetex
```

Skills then appear as `/byetex:<name>`, and the MCP tools register automatically.
A `SessionStart` hook warns if the `byetex` binary isn't found on PATH.

## MCP directly (any MCP client, e.g. Cursor)

You can register the server without the plugin:

```bash
claude mcp add byetex byetex serve
```

or point your client at the stdio command `byetex serve`. The 11 tools:
`convert`, `convert_file`, `convert_fragment`, `convert_project`, `diagnose`,
`validate`, `compile`, `render`, `explain`, `list_skills`, `read_skill`.

## Verify

```bash
byetex --version
byetex skills list        # 12 skills
```
