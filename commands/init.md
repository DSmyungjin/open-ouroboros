---
name: init
description: Initialize the Ouroboros knowledge base in the current project
disable-model-invocation: true
---

Initialize the Ouroboros knowledge base. **NEVER overwrite existing files.**

For each file below, SKIP if it already exists. Only create missing ones.

1. Create `docs/session-log/` directory (if not exists)

2. **SKIP if exists**, otherwise create `docs/TEMPLATE.md` with the session output template from the knowledge-protocol skill's TEMPLATE.md

3. **SKIP if exists**, otherwise create `docs/decisions.md` with:

```
# Decisions

Append-only log. Every entry must have rationale.
When this file exceeds 50 entries, archive older decisions to docs/decisions-archive/YYYY-QN.md and keep only the latest 50 here.

---
```

4. **SKIP if exists**, otherwise create `docs/open-questions.md` with:

```
# Open Questions

Unresolved items only. When you resolve one:
1. Record the resolution as a decision in decisions.md
2. Delete the question from this file

This file should only contain questions that are still open.

---
```

5. **Append** to the project's `CLAUDE.md` (create if it doesn't exist) — but only if it doesn't already contain "Knowledge Protocol":

```
# Knowledge Protocol

This project uses Ouroboros for persistent knowledge across sessions.
Before starting work, read docs/decisions.md and docs/open-questions.md.
After finishing, log your session to docs/session-log/.
```

Report what was created and what was skipped (already existed).
