# Ouroboros - Persistent Knowledge Protocol

## What This Project Is

A file-based knowledge accumulation protocol for Claude Code agent teams.
No database. No search engine. No Rust binary. Just structured markdown and conventions.

Agents read past decisions, do work, log findings. Knowledge survives across sessions.

## Before Starting Work

1. Read `docs/decisions.md` for all past decisions and their rationale
2. Read `docs/open-questions.md` for unresolved items you might be able to answer
3. Scan `docs/session-log/` for recent sessions related to your task

## While Working

- If you resolve an open question from `docs/open-questions.md`, move it to `docs/decisions.md` with your rationale
- Cross-reference past sessions using `[session-YYYY-MM-DD-topic/F1]` format

## After Finishing Work

1. Create a session log at `docs/session-log/YYYY-MM-DD-topic.md` using the template in `docs/TEMPLATE.md`
2. Append any decisions you made to `docs/decisions.md`
3. Append any unresolved questions to `docs/open-questions.md`
4. Remove any questions from `docs/open-questions.md` that you resolved

## Session Log Rules

- Every finding gets `[F#]` ID
- Every decision gets `[D#]` ID with `Caused by:` linking to the evidence
- Every question gets `[Q#]` ID
- Cross-session references use `[session-YYYY-MM-DD-topic/ID]` format
- Causal chains use `→` notation

## File Structure

```
docs/
  TEMPLATE.md         # Session output format (mandatory)
  decisions.md        # Append-only decision log (read before every session)
  open-questions.md   # Unresolved items (check if you can answer any)
  session-log/        # One file per session
    YYYY-MM-DD-topic.md
```
