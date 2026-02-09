---
name: init
description: Initialize the Ouroboros knowledge base in the current project
disable-model-invocation: true
---

Create the following directory structure in the current project if it doesn't exist:

```
docs/
  TEMPLATE.md
  decisions.md
  open-questions.md
  session-log/
```

1. Create `docs/session-log/` directory
2. Create `docs/TEMPLATE.md` with the session output template from the knowledge-protocol skill's TEMPLATE.md
3. Create `docs/decisions.md` with this content:

```
# Decisions

Append-only log. Every entry must have rationale.
When this file exceeds 50 entries, archive older decisions to docs/decisions-archive/YYYY-QN.md and keep only the latest 50 here.

---
```

4. Create `docs/open-questions.md` with this content:

```
# Open Questions

Unresolved items only. When you resolve one:
1. Record the resolution as a decision in decisions.md
2. Delete the question from this file

This file should only contain questions that are still open.

---
```

5. Add this to the project's `CLAUDE.md` (create if it doesn't exist):

```
# Knowledge Protocol

This project uses Ouroboros for persistent knowledge across sessions.
Before starting work, read docs/decisions.md and docs/open-questions.md.
After finishing, log your session to docs/session-log/.
```

Report what was created.
