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

---
```

4. Create `docs/open-questions.md` with this content:

```
# Open Questions

Items no session has resolved yet. If you can answer one, move it to decisions.md.

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
