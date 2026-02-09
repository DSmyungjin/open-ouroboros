# Decisions

Append-only log. Every entry must have rationale.
When this file exceeds 50 entries, archive older decisions to `docs/decisions-archive/YYYY-QN.md` and keep only the latest 50 here.

## Namespace Convention

When multiple teams share this file, prefix IDs to avoid collisions:
- `[X-D#]`, `[X-F#]`, `[X-Q#]` where `X` is a short team identifier
- `[D#]` = no prefix when only one team uses the file
- Define your project's prefixes below this line:

---

- [D1] Delete Knowledge Graph (Neo4j) requirement | Caused by: validation sessions 001-005 showed 0/5 cases where KG would have helped. Structured markdown + grep handles keyword search, causal queries, and multi-hop traces. | Date: 2025-02-09
- [D2] Delete Rust orchestrator, Tantivy search engine, API server | Caused by: [D1] + Claude Code agent teams already provides task DAG, messaging, and coordination natively. No need to rebuild. | Date: 2025-02-09
- [D3] Adopt file-based knowledge protocol instead of software | Caused by: [D2] + the product is the convention (CLAUDE.md + structured markdown), not a binary. Agents are the search engine. | Date: 2025-02-09
