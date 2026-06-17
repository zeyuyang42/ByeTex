#!/usr/bin/env bash
# Sync the canonical subagent defs (committed, in agents/) into .claude/agents/,
# where the Agent tool resolves `subagent_type`. .claude/ is gitignored, so every
# checkout that drives the autonomous-dev loop runs this once (see docs/autonomous-dev.md).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$ROOT/.claude/agents"
cp "$ROOT"/agents/*.md "$ROOT/.claude/agents/"
n=$(find "$ROOT"/agents -maxdepth 1 -name '*.md' | wc -l | tr -d ' ')
echo "synced $n agent def(s) → .claude/agents/"
