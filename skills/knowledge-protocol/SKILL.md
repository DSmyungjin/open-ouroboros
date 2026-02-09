---
name: knowledge-protocol
description: Persistent knowledge accumulation across sessions. Activates when starting work, making decisions, finishing tasks, or when past context would help. Reads decisions.md and open-questions.md automatically.
user-invocable: false
---

You have access to a persistent knowledge base that survives across sessions. Use it.

## On Session Start

Before starting any work, read these files if they exist in the project:

1. `docs/decisions.md` — all past decisions with rationale. Do not re-decide what's already decided unless explicitly asked.
2. `docs/open-questions.md` — unresolved items. If you can answer any during your work, do so.

## While Working

- Reference past decisions when they're relevant: `[D#] from decisions.md`
- If you resolve an open question, note it for the session log
- Cross-reference past sessions using `[session-YYYY-MM-DD-topic/F1]` format

## On Session End

When your work is complete, create a session log at `docs/session-log/YYYY-MM-DD-topic.md` using the template in [TEMPLATE.md](TEMPLATE.md). Follow these rules:

- Every finding gets `[F#]` ID
- Every decision gets `[D#]` ID with `Caused by:` linking to evidence
- Every unresolved question gets `[Q#]` ID
- Cross-session references use `[session-YYYY-MM-DD-topic/ID]` format
- Causal chains use `→` notation
- **Namespace prefix**: If `docs/decisions.md` defines namespace prefixes (e.g., `E-` for Entrepreneur, `L-` for Lab), use them: `[E-D1]`, `[L-F2]`, `[E-Q3]`. If no namespaces are defined, use plain `[D#]`.

Then update the shared knowledge files:

1. **Append** new decisions to `docs/decisions.md`
2. **Append** new questions to `docs/open-questions.md`
3. **Remove** any questions from `docs/open-questions.md` that you resolved

## Important

- `docs/decisions.md` is append-only. Never delete or rewrite past entries.
- `docs/open-questions.md` is a live list. Resolved questions must be deleted from it (the resolution lives in `decisions.md` and the session log).
- When `docs/decisions.md` exceeds 50 entries, move older entries to `docs/decisions-archive/YYYY-QN.md` and keep only the latest 50.
- Keep session logs factual and concise. No filler.
- If `docs/` doesn't exist, suggest running `/ouroboros:init` first.
