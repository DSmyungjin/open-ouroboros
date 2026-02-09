---
name: question
description: Add or resolve an open question in the knowledge base
disable-model-invocation: true
argument-hint: [question or "resolve Q#"]
---

Manage open questions in `docs/open-questions.md`.

If $ARGUMENTS starts with "resolve":
1. Read `docs/open-questions.md`
2. Find the referenced question (e.g., "resolve Q1")
3. Remove it from `docs/open-questions.md`
4. Append the resolution to `docs/decisions.md` with rationale

Otherwise:
1. Read `docs/open-questions.md` to find the next question number
2. Append: `- [Q#] $ARGUMENTS`
3. Show the appended entry
