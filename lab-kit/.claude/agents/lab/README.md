# Lab Agent Workflow

3 roles in a rigorous validation loop. Professor sets direction, PhD critiques methodology, Master's executes experiments.

**Memory**: Ouroboros (docs/decisions.md + docs/session-log/)

---

## Workflow

```
SESSION START
  | Ouroboros: Read docs/decisions.md + docs/open-questions.md
  v

  PROFESSOR (opus)
  Direction + Accept/Reject authority
  |
  | research question + constraints
  v

  PhD (sonnet)
  Critique + Experiment design
  |
  | approved design
  v

  MASTER'S (sonnet)
  Execute + Report numbers
  |
  | raw results
  v

  PhD (sonnet)   <--- results flow UP
  Evaluate results + recommend
  |
  | recommendation
  v

  PROFESSOR (opus)   <--- final judgment
  ACCEPT / REJECT / REVISE (max 2x)

  |
  v
SESSION END
  | Ouroboros: Log to docs/session-log/YYYY-MM-DD-topic.md
  v
```

---

## Roles

| Role | Model | Input | Output | Timescale |
|------|-------|-------|--------|-----------|
| **Professor** | opus | hypothesis / PhD eval | Accept/Reject/Revise | per-project |
| **PhD** | sonnet | research Q / results | critique + recommendation | per-experiment |
| **Master's** | sonnet | experiment design | numbers + results | per-task |

---

## vs Entrepreneur Model

```
Entrepreneur: "Is there signal?"       -> 1hr, kill fast, portfolio of bets
Lab:          "Is this real? How much?" -> hours/days, thorough, single deep dive

Entrepreneur kills 75%. Lab rejects 20%.
Typical flow: Entrepreneur SCALE -> Lab validates.
```

---

## Quality Metrics

```
Accept rate:  60-80% (ideas should be pre-filtered)
Reject rate:  10-20%
Revise rate:  10-20%

Red flags:
  Accept > 90% -> PhD critique too weak
  Reject > 40% -> ideas aren't pre-filtered (use Entrepreneur first)
```

---

## Spawn Examples

```bash
Task: "Read .claude/agents/lab/professor.md, then validate [hypothesis]"
Task: "Read .claude/agents/lab/phd.md, then critique this approach: [description]"
Task: "Read .claude/agents/lab/masters.md, then execute experiment: [design]"
```

---

| File | Role | Responsibility |
|------|------|----------------|
| [professor.md](./professor.md) | Professor | Direction + decisions + Ouroboros |
| [phd.md](./phd.md) | PhD | Methodology critique + evaluation |
| [masters.md](./masters.md) | Master's | Experiment execution |

*Version: Lab Model v1.0 (Ouroboros-integrated)*
