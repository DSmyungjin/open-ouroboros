# Ouroboros

Persistent knowledge accumulation across Claude Code sessions and agent teams.

No database. No search engine. No binary. Just structured markdown and conventions.

## What it does

Every Claude Code session starts from zero. Ouroboros fixes that.

When installed, every session — including every teammate in agent teams — automatically:
1. **Reads** past decisions and open questions before starting work
2. **Logs** findings, decisions, and questions when finishing work
3. **Links** across sessions using grep-able IDs (`[D1]`, `[F2]`, `[session-2025-02-09-auth/Q1]`)

Knowledge survives across sessions. Decisions don't get re-made. Questions don't get forgotten.

## Install

### Option A: Clone and add as plugin directory

```bash
git clone https://github.com/DSmyungjin/open-ouroboros.git ~/.claude/plugins/ouroboros
```

Then add to your project's `.claude/settings.json`:

```json
{
  "plugins": ["~/.claude/plugins/ouroboros"]
}
```

### Option B: Copy into your project

```bash
# From your project root
git clone https://github.com/DSmyungjin/open-ouroboros.git /tmp/ouroboros

# Copy plugin files
cp -r /tmp/ouroboros/.claude-plugin .claude-plugin
cp -r /tmp/ouroboros/skills .claude/skills
cp -r /tmp/ouroboros/commands .claude/commands

rm -rf /tmp/ouroboros
```

### Option C: Install from marketplace (if published)

```
/plugin marketplace add https://github.com/DSmyungjin/open-ouroboros.git
/plugin install ouroboros
```

## Setup

After installing, run in your project:

```
/ouroboros:init
```

This creates:

```
docs/
  TEMPLATE.md          # Session output format
  decisions.md         # Append-only decision log
  open-questions.md    # Unresolved items
  session-log/         # One file per session
```

And adds knowledge protocol instructions to your `CLAUDE.md`.

## Usage

### Automatic (just work normally)

The skill auto-activates. Claude reads `docs/decisions.md` and `docs/open-questions.md` at session start, and logs findings when done. No commands needed.

### Commands

| Command | What it does |
|---------|-------------|
| `/ouroboros:init` | Set up `docs/` in your project |
| `/ouroboros:log-session [topic]` | Log current session's findings |
| `/ouroboros:decide [decision]` | Record a decision with rationale |
| `/ouroboros:question [question]` | Add or resolve an open question |

### Examples

```
/ouroboros:decide Use JWT for auth — simpler than session cookies for our API-first architecture

/ouroboros:question How should we handle token refresh for mobile clients?

/ouroboros:question resolve Q1

/ouroboros:log-session auth-design
```

## How it works with Agent Teams

Every teammate loads `CLAUDE.md` automatically. The knowledge protocol skill teaches each teammate to:

- Read shared decisions before starting
- Resolve open questions when possible
- Log findings with structured IDs
- Cross-reference other sessions

No extra coordination needed. The filesystem is the shared state.

## Session log format

```markdown
# Session: Auth Design

**Date:** 2025-02-09
**Team Role:** architect

---

## Key Findings
- [F1] JWT tokens with 15min expiry balance security and UX
- [F2] Refresh tokens need server-side storage for revocation

## Decisions Made
- [D1] Use JWT with short-lived access tokens | Caused by: [F1]
- [D2] Store refresh tokens in Redis | Caused by: [F2], [session-2025-02-08-infra/D3]

## Open Questions
- [Q1] Should refresh tokens rotate on each use?

## Causal Chain
[F1] → [D1] → [F2] → [D2]
```

## Philosophy

This plugin exists because of [Elon's 5-step process](https://www.youtube.com/watch?v=Jqo3V1wp0dE):

1. **Make requirements less dumb** — "We need a Knowledge Graph" was the wrong requirement. We needed persistent memory.
2. **Delete** — Deleted Neo4j, Tantivy search, Rust orchestrator, API server (31K lines).
3. **Simplify** — Replaced with 12 markdown files. The agents *are* the search engine.
4. **Speed up** — If grep gets slow at 1000+ sessions, add indexing then. Not before.
5. **Automate** — The skill auto-activates. No manual steps needed.

## File structure

```
ouroboros/
├── .claude-plugin/
│   ├── plugin.json              # Plugin metadata
│   └── marketplace.json         # Marketplace registry
├── skills/
│   └── knowledge-protocol/
│       ├── SKILL.md             # Auto-loaded knowledge protocol
│       └── TEMPLATE.md          # Session output format
├── commands/
│   ├── init.md                  # /ouroboros:init
│   ├── log-session.md           # /ouroboros:log-session
│   ├── decide.md                # /ouroboros:decide
│   └── question.md              # /ouroboros:question
└── docs/                        # Example knowledge base
    ├── TEMPLATE.md
    ├── decisions.md
    ├── open-questions.md
    └── session-log/
```
