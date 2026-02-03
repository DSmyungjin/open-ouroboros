# Context Tree Design

## Overview

Context Tree는 워커 태스크가 참조해야 할 문서를 정의하는 구조입니다.
Claude 세션 관리가 아닌 **문서 참조 명세**를 위한 시스템입니다.

## Plan Stage vs Work Stage

```
[Plan Stage]
├── 자료조사, 분석, 계획 수립
├── 컨텍스트 많이 소모 (일회성)
└── 결과물: DAG + Context Tree 정의

[Work Stage]
├── Context Fill 태스크 → 문서 준비
└── Worker 태스크 → 준비된 문서 참조하여 작업
```

Plan Stage의 무거운 컨텍스트를 Work Stage로 그대로 전달하지 않고,
필요한 정보만 정제하여 Context Tree에 저장합니다.

## Context Tree 구조

```
ctx-root (공통 문서)
    │
    ├── ctx-branch-a (A 브랜치용 문서)
    │       └── delta_docs: [api-docs.md, schema.md]
    │
    └── ctx-branch-b (B 브랜치용 문서)
            └── delta_docs: [config-guide.md]
```

### ContextNode

```rust
pub struct ContextNode {
    pub node_id: String,           // 노드 ID
    pub parent: Option<String>,    // 부모 노드
    pub cached_prefix: Option<PathBuf>,  // 공유 문서 (캐시)
    pub delta_docs: Vec<PathBuf>,        // 브랜치별 추가 문서
    pub status: ContextStatus,
}
```

### 문서 조회

```rust
// 해당 노드의 전체 문서 목록 (root → node 경로의 모든 문서)
let docs = context_tree.get_docs("ctx-branch-a");
// Returns: [root docs] + [branch-a docs]
```

## Task Types

### ContextFill 태스크

문서를 준비하여 Context Tree 노드에 추가합니다.

```rust
Task::new_context_fill(
    "API 문서 준비",
    "REST API 엔드포인트 문서화",
    "ctx-branch-a"  // target_node
)
```

실행 완료 시:
```rust
context_tree.get_mut("ctx-branch-a").add_doc(result_path)
```

### Worker 태스크

Context Tree의 문서를 참조하여 실제 작업을 수행합니다.

```rust
Task::new("기능 구현", "설명")
    .with_context_ref("ctx-branch-a")  // 참조할 context node
```

실행 시 context 조립:
```
context = context_tree.get_docs(context_ref)  // 참조 문서
        + dependency_results                   // 의존성 결과
        + previous_attempts                    // 재시도 컨텍스트
```

## DAG 통합

Context 준비 작업도 DAG에 명시적으로 포함됩니다.

```
[ctx-fill-root]  ──────────────────────────────────────┐
       │                                                │
       ├── [ctx-fill-branch-a] ──→ [task-a1] ──→ [task-a2]
       │                                                │
       └── [ctx-fill-branch-b] ──→ [task-b1] ──→ [task-b2]
                                                        │
                                                  [merge-task]
```

## 역할별 프롬프트

### Context Fill

```
# Role: Context Preparer

You are preparing reference documents for context node '{target_node}'.
Your output will be used by downstream worker tasks.

## Guidelines
- Research and gather relevant information for this context
- Output structured, reusable documentation
- Focus on facts and references that workers will need
```

### Worker

```
# Role: Implementation Worker

Reference documents are provided above. Use them as your primary source.

## Guidelines
- Use the provided context for your work
- If context is insufficient, spawn a research sub-agent
- Do NOT do ad-hoc research directly - delegate it
```

## 동적 Context 추가 (ADD_CONTEXT)

Worker 태스크가 작업 중 발견한 정보를 Context Tree에 추가할 수 있습니다.

### 출력 형식

```markdown
작업 결과...

[ADD_CONTEXT:ctx-branch-a]
## 발견한 API 정보
Bearer token 인증 사용
만료시간: 1시간
[/ADD_CONTEXT]
```

### 동작

1. 출력에서 `[ADD_CONTEXT:node_id]...[/ADD_CONTEXT]` 파싱
2. 내용을 문서로 저장
3. 해당 context node에 추가
4. sibling/downstream 태스크가 `get_docs()`로 참조 가능

## 사용 예시

```rust
// 1. Context Tree 초기화
let mut tree = ContextTree::new();
tree.init_root_with_docs(vec![PathBuf::from("./docs/spec.md")]);

// 2. 브랜치 생성
tree.branch_with_ids(
    root_id,
    "ctx-fill-root",
    &["branch-a", "branch-b"],
    None
)?;

// 3. ContextFill 태스크 생성
let ctx_fill = Task::new_context_fill(
    "API 문서 준비",
    "REST API 문서화",
    "ctx-branch-a"
);

// 4. Worker 태스크 생성
let worker = Task::new("API 클라이언트 구현", "설명")
    .with_context_ref("ctx-branch-a");
```
