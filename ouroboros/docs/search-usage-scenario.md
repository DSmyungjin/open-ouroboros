# Search 기능 활용 시나리오

## 시나리오: 과거 지식을 활용한 신규 기능 구현

### Phase 1: 지식 축적 (이전 세션들)

```bash
# Session 1: API 설계
ouroboros plan "REST API 설계 - 인증, 사용자 관리" -l "api-design"
ouroboros run --all

# Session 2: 데이터베이스 설계
ouroboros plan "PostgreSQL 스키마 설계 - users, sessions 테이블" -l "db-design"
ouroboros run --all

# Session 3: 에러 핸들링 패턴
ouroboros plan "Rust 에러 핸들링 패턴 - anyhow, thiserror 사용법" -l "error-patterns"
ouroboros run --all
```

이 세션들의 결과가 자동으로 인덱싱됨.

### Phase 2: 신규 작업 - 검색 활용

```bash
# 새 세션: 사용자 등록 API 구현
ouroboros plan "사용자 등록 API 엔드포인트 구현" -l "user-registration"
```

### Phase 3: Task 실행 시 Search 활용

Worker가 받는 프롬프트:
```
# Role: Implementation Worker

## Searching Past Knowledge
Before researching externally, search the knowledge base:

# Search all sessions
ouroboros search "API 인증" --all
ouroboros search "users 테이블" --all
ouroboros search "에러 핸들링" --all -t knowledge
```

### 예상 동작

1. Worker가 "사용자 등록" 구현 시작
2. 인증 로직 필요 → `ouroboros search "인증" --all` 실행
3. 과거 "api-design" 세션의 인증 설계 문서 발견
4. DB 스키마 필요 → `ouroboros search "users" --all` 실행
5. 과거 "db-design" 세션의 스키마 정보 활용
6. 에러 처리 필요 → `ouroboros search "에러" --all` 실행
7. 과거 "error-patterns" 세션의 패턴 참조

### 검증 방법

Task 실행 후 결과에서 확인:
- `ouroboros search` 명령어 호출 흔적
- 과거 세션 결과 참조 여부
- 일관된 설계 패턴 적용 여부

## 현재 상태

- [x] Search CLI 구현 완료
- [x] `--all` 전체 세션 검색 지원
- [x] Worker 프롬프트에 search 안내 추가
- [ ] 실제 활용 테스트 필요

## 버그 발견

```
thread 'main' panicked at src/main.rs:201:57:
byte index 37 is not a char boundary; it is inside '주' (bytes 36..39)
```

한글 문자열 슬라이싱 문제 - `work-sessions list` 명령에서 발생.
수정 필요.
