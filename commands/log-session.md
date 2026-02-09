---
name: log-session
description: Log the current session's findings, decisions, and questions
disable-model-invocation: true
argument-hint: [topic]
---

Create a session log for the work done in this session.

1. Determine the topic from $ARGUMENTS or infer from the conversation
2. Create `docs/session-log/YYYY-MM-DD-$ARGUMENTS.md` using today's date
3. Fill it using the template from the knowledge-protocol skill:
   - Summarize key findings as `[F1]`, `[F2]`, etc.
   - List decisions as `[D1]`, `[D2]` with `Caused by:` links
   - List unresolved questions as `[Q1]`, `[Q2]`
   - Add causal chain if applicable
4. Append any new decisions to `docs/decisions.md`
5. Append any new questions to `docs/open-questions.md`
6. Remove any resolved questions from `docs/open-questions.md`

Show the created session log file.
