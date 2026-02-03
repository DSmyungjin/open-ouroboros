# Ouroboros

LLM Agent Orchestration System using Claude Code CLI.

## Overview

Ouroboros breaks down goals into tasks, executes them via Claude Code CLI, and manages the workflow with a DAG (Directed Acyclic Graph).

```
Goal → Plan (DAG) → Execute → Validate → Results
```

## Requirements

- Rust 1.75+
- [Claude Code CLI](https://claude.ai/claude-code) installed and authenticated
- (Optional) Neo4j for Knowledge Graph (Phase 2)

## Installation

```bash
cd ouroboros
cargo build --release
```

Or run directly:

```bash
cargo run -- <command>
```

## Usage

### Initialize Project

```bash
ouroboros init
```

Creates `data/` directory structure:
```
data/
├── tasks/      # Task definitions
├── results/    # Execution results
└── contexts/   # Assembled contexts
```

### Plan Tasks from Goal

```bash
ouroboros plan "Design a REST API for user management"
```

Claude analyzes the goal and generates a task DAG:
```
Created 4 tasks:
  - task-001
  - task-002
  - task-003
  - task-004
```

### List Tasks

```bash
ouroboros tasks
```

Output:
```
Tasks:
  [ ] task-001 - Analyze requirements
  [ ] task-002 - Design data models
  [ ] task-003 - Define API endpoints
  [ ] task-004 - Write OpenAPI spec
```

### Execute Tasks

Run all tasks in dependency order:
```bash
ouroboros run --all
```

Run a specific task:
```bash
ouroboros run task-001
```

### Validate Results

Use Opus to validate task output:
```bash
ouroboros validate task-001
```

### Check Statistics

```bash
ouroboros stats
```

Output:
```
Task Statistics:
  Total:       4
  Completed:   2
  Failed:      0
  Pending:     2
  In Progress: 0
```

## Options

```
-w, --workdir <WORKDIR>    Working directory [default: .]
-d, --data-dir <DATA_DIR>  Data directory [default: ./data]
```

## Architecture

```
┌─────────────────────────────────────────┐
│           Rust Orchestrator             │
│  ├── DAG Manager (petgraph)             │
│  ├── Document Store (Markdown)          │
│  └── CLI Runner (subprocess)            │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│            claude CLI                   │
│  ├── --model (opus/sonnet/haiku)        │
│  ├── --print (non-interactive)          │
│  └── --resume (session continuity)      │
└─────────────────────────────────────────┘
```

## Document Format

Tasks and results are stored as Markdown with YAML frontmatter:

```markdown
---
id: task-001-result
type: task_result
task_id: task-001
created_at: 2025-02-03T10:00:00Z
tags: [api, design]
---

# Result: API Design

## Summary
...
```

## Roadmap

- [x] Phase 1: Core MVP
  - [x] Claude CLI wrapper
  - [x] DAG task management
  - [x] Document store
  - [x] Sequential execution

- [ ] Phase 2: Knowledge Graph
  - [ ] Neo4j integration
  - [ ] Hybrid search (vector + BM25)
  - [ ] Context assembly from KG

- [ ] Phase 3: Advanced
  - [ ] Parallel execution
  - [ ] Fork/branch workflows
  - [ ] Validation pipeline

## License

MIT
