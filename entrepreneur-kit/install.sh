#!/bin/bash
# Install Entrepreneur Agent Kit into current project
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TARGET="${1:-.}"

echo "Installing Entrepreneur Kit into: $TARGET"

# Agent files
mkdir -p "$TARGET/.claude/agents/entrepreneur"
cp "$SCRIPT_DIR/.claude/agents/entrepreneur/README.md" "$TARGET/.claude/agents/entrepreneur/"
cp "$SCRIPT_DIR/.claude/agents/entrepreneur/founder.md" "$TARGET/.claude/agents/entrepreneur/"
cp "$SCRIPT_DIR/.claude/agents/entrepreneur/scout.md" "$TARGET/.claude/agents/entrepreneur/"
cp "$SCRIPT_DIR/.claude/agents/entrepreneur/hacker.md" "$TARGET/.claude/agents/entrepreneur/"

# Philosophy
cp "$SCRIPT_DIR/elon_5_principles.md" "$TARGET/"

# Ouroboros docs
mkdir -p "$TARGET/docs/session-log"
for f in decisions.md open-questions.md TEMPLATE.md; do
  if [ ! -f "$TARGET/docs/$f" ]; then
    cp "$SCRIPT_DIR/docs/$f" "$TARGET/docs/"
  else
    echo "  skip docs/$f (already exists)"
  fi
done

# CLAUDE.md append (only if not already present)
if [ -f "$TARGET/CLAUDE.md" ]; then
  if ! grep -q "Entrepreneur Agent Model" "$TARGET/CLAUDE.md"; then
    cat "$SCRIPT_DIR/CLAUDE_APPEND.md" >> "$TARGET/CLAUDE.md"
    echo "  appended to CLAUDE.md"
  else
    echo "  skip CLAUDE.md (already has Entrepreneur section)"
  fi
else
  cp "$SCRIPT_DIR/CLAUDE_APPEND.md" "$TARGET/CLAUDE.md"
  echo "  created CLAUDE.md"
fi

echo ""
echo "Done. To start:"
echo '  Task: "Read .claude/agents/entrepreneur/founder.md, then run discovery cycle on [topic]"'
