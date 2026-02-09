# Entrepreneur Agent Kit

## Install

```bash
cp -r entrepreneur-kit/.claude /path/to/your/project/
cp -r entrepreneur-kit/docs /path/to/your/project/
cp entrepreneur-kit/elon_5_principles.md /path/to/your/project/
cat entrepreneur-kit/CLAUDE_APPEND.md >> /path/to/your/project/CLAUDE.md
```

Or just copy the whole folder contents into your project root.

## What's inside

```
.claude/agents/entrepreneur/
  README.md       — Workflow diagram + Ouroboros integration
  founder.md      — Philosophy (Elon P1-P3) + Kill/Pivot/Scale + knowledge keeper
  scout.md        — Opportunity discovery + reads past decisions
  hacker.md       — MVP execution + 1hr time-box

docs/
  decisions.md    — Append-only decision log (Ouroboros)
  open-questions.md — Unresolved items (Ouroboros)
  session-log/    — Per-session logs (Ouroboros)
  TEMPLATE.md     — Session log template

elon_5_principles.md — Founder's operating system
CLAUDE_APPEND.md     — Paste this into your CLAUDE.md
```

## Usage

```
Task: "Read .claude/agents/entrepreneur/founder.md, then run discovery cycle on [topic]"
```
