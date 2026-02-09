# Entrepreneur Agent Workflow

3 roles in a fast iteration loop. Only what's worth deep validation gets promoted.

**Philosophy**: Elon's 5 Principles (elon_5_principles.md)
**Memory**: Ouroboros (docs/decisions.md + docs/session-log/)
**Core**: Kill speed > Validation depth. Test 20 cheap, Kill 15, Pivot 3, Scale 2.

---

## Workflow

```
SESSION START
  |
  | Ouroboros: Read docs/decisions.md + docs/open-questions.md
  v

  FOUNDER (opus)
  Philosophy holder + Kill/Pivot/Scale authority
  |                         |
  | direction          | evaluation
  v                         v
  SCOUT (sonnet)  <--->  HACKER (sonnet)
  opportunity scan       MVP execution
  |                         |
  v                         v

  DECISION GATE
  Strong signal + High TAI  ->  SCALE (deep validation)
  Weak signal + High TAI    ->  PIVOT (different angle)
  No signal OR Low TAI      ->  KILL  (graveyard + next)

  |
  v
SESSION END
  | Ouroboros: Log to docs/session-log/YYYY-MM-DD-topic.md
  v
```

---

## Roles

| Role | Model | Input | Output | Elon Principle |
|------|-------|-------|--------|----------------|
| **Founder** | opus | opportunities / MVP results | Kill/Pivot/Scale | P1-P3 |
| **Scout** | sonnet | search direction | TAI-ranked opportunities | P1 |
| **Hacker** | sonnet | MVP spec | Signal / No signal | P4-P5 |

---

## Portfolio Health

```
Kill rate:    60-80% (healthy)
Scale rate:   10-20%
Avg cycle:    < 2hr

Red flags:
  Kill < 50%  -> Scout filter too loose
  Kill > 90%  -> search space exhausted
  Scale > 30% -> threshold too loose
```

---

## Spawn Examples

```bash
# Full cycle
Task: "Read .claude/agents/entrepreneur/founder.md, then run discovery cycle on [topic]"

# Individual roles
Task: "Read .claude/agents/entrepreneur/scout.md, then find opportunities in [area]"
Task: "Read .claude/agents/entrepreneur/hacker.md, then execute MVP: [spec]"
```

---

| File | Role | Responsibility |
|------|------|----------------|
| [founder.md](./founder.md) | Founder | Philosophy + decisions + Ouroboros keeper |
| [scout.md](./scout.md) | Scout | Opportunity discovery + dedup from decisions.md |
| [hacker.md](./hacker.md) | Hacker | MVP execution + time-box |

*Version: Entrepreneur Model v2.0 (Ouroboros-integrated)*
