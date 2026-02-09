---
name: decide
description: Record a decision with rationale to the persistent knowledge base
disable-model-invocation: true
argument-hint: [decision description]
---

Append a decision to `docs/decisions.md`.

1. Read `docs/decisions.md` to find the next decision number
2. Append this entry:

```
- [D#] $ARGUMENTS | Caused by: [evidence] | Date: YYYY-MM-DD
```

3. If $ARGUMENTS doesn't include rationale, ask for the `Caused by:` reason before appending
4. Show the appended entry
