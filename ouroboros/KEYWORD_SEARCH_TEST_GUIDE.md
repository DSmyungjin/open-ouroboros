# Keyword Search 테스트 실행 가이드

## 🚀 빠른 시작

### 전체 테스트 실행
```bash
cargo test --test keyword_search_test -- --nocapture
```

**예상 결과**: 12개 테스트 모두 통과, 실행 시간 약 16초

---

## 📋 개별 테스트 실행

### 1. BM25 검색 테스트

#### 단일 키워드 검색
```bash
cargo test --test keyword_search_test test_bm25_single_keyword_english -- --nocapture
```
**검증 내용**: BM25 알고리즘 기본 동작, 스코어 기반 정렬

#### 복합 키워드 검색
```bash
cargo test --test keyword_search_test test_bm25_multi_keyword -- --nocapture
```
**검증 내용**: 여러 키워드 조합 검색, 관련도 계산

#### 랭킹 정확도
```bash
cargo test --test keyword_search_test test_bm25_ranking_accuracy -- --nocapture
```
**검증 내용**: 키워드 빈도에 따른 정확한 순위 매기기

---

### 2. 한국어 형태소 분석 테스트

#### 기본 한국어 토큰화
```bash
cargo test --test keyword_search_test test_korean_tokenizer_basic -- --nocapture
```
**검증 내용**: Lindera + KoDic 기본 동작, 한국어 문서 검색

#### 형태소 분석
```bash
cargo test --test keyword_search_test test_korean_morphological_analysis -- --nocapture
```
**검증 내용**: 조사/어미 분리, 어근 추출, 복합명사 처리

#### 복잡한 한국어 문장
```bash
cargo test --test keyword_search_test test_korean_complex_sentences -- --nocapture
```
**검증 내용**: 전문 용어, 긴 문장, 기술 문서 처리

---

### 3. 다국어 검색 테스트

#### 한영 혼합 검색
```bash
cargo test --test keyword_search_test test_mixed_korean_english_search -- --nocapture
```
**검증 내용**: 한국어+영어 동시 처리, 언어 경계 인식

---

### 4. 필터링 테스트

#### 검색 필터
```bash
cargo test --test keyword_search_test test_search_with_filters -- --nocapture
```
**검증 내용**: 문서 타입, 세션 ID, 복합 필터

#### 최소 스코어 임계값
```bash
cargo test --test keyword_search_test test_search_min_score_threshold -- --nocapture
```
**검증 내용**: 스코어 기반 필터링, Precision 제어

---

### 5. 성능 테스트

#### 성능 벤치마크
```bash
cargo test --test keyword_search_test test_search_performance -- --nocapture
```
**검증 내용**: 100개 문서 색인, 검색 응답 속도

**예상 성능**:
- 색인: ~87ms/문서
- 검색: 13-28ms
- 결과당 처리: 1.3-2.8ms

---

### 6. 엣지 케이스 테스트

#### 예외 처리
```bash
cargo test --test keyword_search_test test_edge_cases -- --nocapture
```
**검증 내용**: 빈 쿼리, 긴 쿼리, 특수문자, Unicode

---

### 7. 통합 테스트

#### 전체 워크플로우
```bash
cargo test --test keyword_search_test test_complete_search_workflow -- --nocapture
```
**검증 내용**: 엔드투엔드 시나리오, 실제 사용 패턴

---

## 🔍 테스트 출력 이해하기

### 성공적인 테스트 출력 예시
```
=== Test: BM25 Single Keyword (English) ===
✓ Indexed 3 documents
Query: 'search'
Results found: 2
  1. Search UI Component (score: 0.7221)
  2. Search Implementation (score: 0.7176)
✓ BM25 ranking verified

test test_bm25_single_keyword_english ... ok
```

### 스코어 해석
- **0.9 이상**: 매우 높은 관련도 (정확히 일치)
- **0.7 - 0.9**: 높은 관련도 (강한 연관성)
- **0.5 - 0.7**: 중간 관련도 (부분 일치)
- **0.3 - 0.5**: 낮은 관련도 (약한 연관성)
- **0.3 미만**: 매우 낮은 관련도 (거의 무관)

---

## 🐛 문제 해결

### 테스트 실패 시

#### 1. 컴파일 오류
```bash
# 의존성 설치
cargo build

# 클린 빌드
cargo clean
cargo build
```

#### 2. 인덱스 잠금 오류
```
Error: Cannot acquire write lock
```
**해결**: 다른 프로세스가 인덱스를 사용 중. 프로세스 종료 후 재시도

#### 3. 메모리 부족
```
Error: Out of memory
```
**해결**: `test_search_performance`의 문서 수 줄이기 (100 → 50)

---

## 📊 테스트 리포트 생성

### 상세 리포트 확인
```bash
# 테스트 실행 후 리포트 확인
cat KEYWORD_SEARCH_TEST_REPORT.md
```

### JSON 형식 출력
```bash
cargo test --test keyword_search_test -- --format json > test_results.json
```

---

## 🔧 커스텀 테스트

### 자신의 쿼리로 테스트하기

테스트 파일 수정 예시:
```rust
#[tokio::test]
async fn test_my_custom_query() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut search = SearchEngine::keyword_only(temp_dir.path())?;

    // 문서 색인
    search.index_task(
        "my-task",
        "제목",
        "내용",
        Some("session-id"),
    ).await?;

    // 검색
    let options = SearchOptions::new().with_limit(10);
    let results = search.search("검색어", &options).await?;

    // 결과 출력
    for result in results {
        println!("{}: {:.4}", result.title, result.score);
    }

    Ok(())
}
```

---

## 📈 성능 모니터링

### 성능 메트릭 수집
```bash
# 성능 테스트만 실행하고 시간 측정
time cargo test --test keyword_search_test test_search_performance -- --nocapture
```

### 프로파일링
```bash
# 프로파일러와 함께 실행
cargo test --test keyword_search_test --release -- --nocapture
```

---

## 🎯 CI/CD 통합

### GitHub Actions 예시
```yaml
name: Keyword Search Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --test keyword_search_test
```

### 로컬 pre-commit 훅
```bash
#!/bin/bash
# .git/hooks/pre-commit

cargo test --test keyword_search_test --quiet
if [ $? -ne 0 ]; then
    echo "Keyword search tests failed!"
    exit 1
fi
```

---

## 📚 참고 자료

### 관련 파일
- 테스트 코드: `./tests/keyword_search_test.rs`
- 구현 코드: `./src/search/keyword.rs`
- 타입 정의: `./src/search/types.rs`
- 상세 리포트: `./KEYWORD_SEARCH_TEST_REPORT.md`

### 외부 문서
- [Tantivy 문서](https://docs.rs/tantivy/)
- [Lindera 문서](https://github.com/lindera-morphology/lindera)
- [BM25 알고리즘](https://en.wikipedia.org/wiki/Okapi_BM25)

---

## 💬 자주 묻는 질문

### Q: 테스트가 너무 오래 걸려요
**A**: 개별 테스트만 실행하거나 `--release` 플래그 사용
```bash
cargo test --test keyword_search_test test_bm25_single_keyword_english --release
```

### Q: 한국어 검색이 작동하지 않아요
**A**: Lindera KoDic이 제대로 로드되었는지 확인
```bash
cargo build --features lindera/embed-ko-dic
```

### Q: 새로운 테스트를 추가하고 싶어요
**A**: `./tests/keyword_search_test.rs`에 새로운 `#[tokio::test]` 함수 추가

### Q: 성능을 더 향상시키고 싶어요
**A**:
1. 배치 색인 사용
2. 커밋 빈도 줄이기
3. 검색 결과 캐싱
4. 인덱스 최적화

---

**가이드 작성일**: 2025-02-05
**버전**: v0.1.0
