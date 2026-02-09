#!/bin/bash
# Install Lab Agent Kit into current project
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TARGET="${1:-.}"

echo "Installing Lab Kit into: $TARGET"

# Agent files
mkdir -p "$TARGET/.claude/agents/lab"
cp "$SCRIPT_DIR/.claude/agents/lab/README.md" "$TARGET/.claude/agents/lab/"
cp "$SCRIPT_DIR/.claude/agents/lab/professor.md" "$TARGET/.claude/agents/lab/"
cp "$SCRIPT_DIR/.claude/agents/lab/phd.md" "$TARGET/.claude/agents/lab/"
cp "$SCRIPT_DIR/.claude/agents/lab/masters.md" "$TARGET/.claude/agents/lab/"

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
  if ! grep -q "Lab Agent Model" "$TARGET/CLAUDE.md"; then
    cat "$SCRIPT_DIR/CLAUDE_APPEND.md" >> "$TARGET/CLAUDE.md"
    echo "  appended to CLAUDE.md"
  else
    echo "  skip CLAUDE.md (already has Lab section)"
  fi
else
  cp "$SCRIPT_DIR/CLAUDE_APPEND.md" "$TARGET/CLAUDE.md"
  echo "  created CLAUDE.md"
fi

echo ""
echo "Done. To start:"
echo '  Task: "Read .claude/agents/lab/professor.md, then validate [hypothesis]"'
