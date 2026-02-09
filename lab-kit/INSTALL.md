# Lab Agent Kit

## Install

```bash
cp -r lab-kit/.claude /path/to/your/project/
cp -r lab-kit/docs /path/to/your/project/
cat lab-kit/CLAUDE_APPEND.md >> /path/to/your/project/CLAUDE.md
```

## What's inside

```
.claude/agents/lab/
  README.md       — Workflow diagram
  professor.md    — Direction + Accept/Reject + Ouroboros keeper
  phd.md          — Methodology critic + results evaluator
  masters.md      — Experiment executor

docs/
  decisions.md    — Append-only decision log (Ouroboros)
  open-questions.md — Unresolved items (Ouroboros)
  session-log/    — Per-session logs (Ouroboros)
  TEMPLATE.md     — Session log template

CLAUDE_APPEND.md  — Paste this into your CLAUDE.md
```
