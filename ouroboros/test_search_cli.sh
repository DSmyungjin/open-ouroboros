#!/bin/bash
# Comprehensive Search CLI Test Suite for Ouroboros

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BINARY="./target/debug/ouroboros"
TEST_LOG="search_test_results.log"

# Initialize log
echo "=== Ouroboros Search CLI Test Report ===" > $TEST_LOG
echo "Test Date: $(date)" >> $TEST_LOG
echo "" >> $TEST_LOG

test_count=0
pass_count=0
fail_count=0

# Helper function to run test
run_test() {
    local test_name="$1"
    local command="$2"
    local expected_behavior="$3"

    test_count=$((test_count + 1))
    echo -e "${BLUE}[TEST $test_count]${NC} $test_name"
    echo "----------------------------------------" | tee -a $TEST_LOG
    echo "TEST $test_count: $test_name" >> $TEST_LOG
    echo "Command: $command" >> $TEST_LOG
    echo "Expected: $expected_behavior" >> $TEST_LOG
    echo "" >> $TEST_LOG

    # Run command and capture output
    start_time=$(date +%s%3N)
    output=$(eval "$command" 2>&1)
    end_time=$(date +%s%3N)
    elapsed=$((end_time - start_time))

    # Log output
    echo "Output:" >> $TEST_LOG
    echo "$output" >> $TEST_LOG
    echo "Response Time: ${elapsed}ms" >> $TEST_LOG
    echo "" >> $TEST_LOG

    # Display key info
    echo "Response time: ${elapsed}ms"
    echo "$output" | head -20

    # Check if test passed (basic check - no error messages)
    if echo "$output" | grep -qE "(Error|error|panic|failed)"; then
        echo -e "${RED}❌ FAILED${NC}"
        echo "Status: FAILED" >> $TEST_LOG
        fail_count=$((fail_count + 1))
    else
        echo -e "${GREEN}✅ PASSED${NC}"
        echo "Status: PASSED" >> $TEST_LOG
        pass_count=$((pass_count + 1))
    fi
    echo "" | tee -a $TEST_LOG
    echo ""
}

echo -e "${YELLOW}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${YELLOW}║   OUROBOROS SEARCH CLI COMPREHENSIVE TEST SUITE           ║${NC}"
echo -e "${YELLOW}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  SECTION 1: Basic Search Functionality                   ${NC}"
echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo "" | tee -a $TEST_LOG

run_test "Single keyword search (English)" \
    "$BINARY search 'API'" \
    "Should find documents containing 'API'"

run_test "Single keyword search (Korean)" \
    "$BINARY search '데이터베이스'" \
    "Should find documents with Korean text"

run_test "Multiple keywords (AND logic)" \
    "$BINARY search 'API design'" \
    "Should find documents with both 'API' and 'design'"

run_test "Multiple keywords (Korean)" \
    "$BINARY search '사용자 관리'" \
    "Should find documents with Korean terms"

run_test "Phrase search attempt" \
    "$BINARY search '\"REST API\"'" \
    "Should search for exact phrase"

run_test "Case insensitive search" \
    "$BINARY search 'authentication'" \
    "Should find 'authentication', 'Authentication', etc."

echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  SECTION 2: Multilingual Search Tests                    ${NC}"
echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo "" | tee -a $TEST_LOG

run_test "Pure Korean query" \
    "$BINARY search '검색'" \
    "Should find Korean documents about search"

run_test "Pure English query" \
    "$BINARY search 'search'" \
    "Should find English documents about search"

run_test "Mixed language query" \
    "$BINARY search 'REST 디자인'" \
    "Should find documents with both English and Korean"

run_test "Korean morphological analysis" \
    "$BINARY search '설계'" \
    "Should leverage Korean morphological tokenization"

run_test "English technical terms in Korean context" \
    "$BINARY search 'Docker'" \
    "Should find technical terms in Korean documents"

echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  SECTION 3: Filtering Options                            ${NC}"
echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo "" | tee -a $TEST_LOG

run_test "Filter by document type: task" \
    "$BINARY search 'API' --doc-type task" \
    "Should only return task documents"

run_test "Filter by document type: task_result" \
    "$BINARY search 'API' --doc-type task_result" \
    "Should only return task result documents"

run_test "Filter by document type: context" \
    "$BINARY search 'architecture' --doc-type context" \
    "Should only return context documents"

run_test "Filter by document type: knowledge" \
    "$BINARY search 'API' --doc-type knowledge" \
    "Should only return knowledge documents"

run_test "Filter by session ID" \
    "$BINARY search 'task' --session '0db373-search-cli-evaluation'" \
    "Should only return documents from specified session"

run_test "Filter by different session ID" \
    "$BINARY search 'task' --session 'other-session-id'" \
    "Should return documents from other session"

run_test "Combined filters (type + session)" \
    "$BINARY search 'API' --doc-type task --session '0db373-search-cli-evaluation'" \
    "Should apply both filters"

run_test "Limit results to 3" \
    "$BINARY search 'API' --limit 3" \
    "Should return maximum 3 results"

run_test "Limit results to 1" \
    "$BINARY search 'test' --limit 1" \
    "Should return only 1 result"

echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  SECTION 4: Error Handling and Edge Cases                ${NC}"
echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo "" | tee -a $TEST_LOG

run_test "Empty query" \
    "$BINARY search ''" \
    "Should handle empty query gracefully"

run_test "Non-existent term" \
    "$BINARY search 'xyznonexistentterm123'" \
    "Should return no results message"

run_test "Special characters in query" \
    "$BINARY search '@#\$%'" \
    "Should handle special characters"

run_test "Emoji search" \
    "$BINARY search '🚀'" \
    "Should handle emoji characters"

run_test "Very long query (stress test)" \
    "$BINARY search 'API design authentication authorization user management REST microservices architecture database schema table users roles permissions JWT tokens bcrypt hashing password security validation testing deployment Docker Kubernetes monitoring logging CI/CD pipeline GitHub Actions Prometheus Grafana Elasticsearch Logstash Kibana ELK stack'" \
    "Should handle long queries"

run_test "Invalid document type filter" \
    "$BINARY search 'API' --doc-type invalid_type" \
    "Should show error for invalid document type"

run_test "Query with newline characters" \
    "$BINARY search 'API\ndesign'" \
    "Should handle whitespace properly"

run_test "Query with tabs" \
    "$BINARY search 'API\tdesign'" \
    "Should handle tabs in query"

run_test "Unicode characters" \
    "$BINARY search '프로젝트'" \
    "Should handle Unicode properly"

run_test "Mixed case with Korean" \
    "$BINARY search '검색 Engine'" \
    "Should handle mixed case and language"

echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  SECTION 5: Performance and Usability Tests              ${NC}"
echo -e "${GREEN}══════════════════════════════════════════════════════════${NC}"
echo "" | tee -a $TEST_LOG

run_test "Complex query (multiple terms)" \
    "$BINARY search 'microservices architecture Kubernetes Docker monitoring' --limit 5" \
    "Should handle complex multi-term queries"

run_test "Relevance ranking check" \
    "$BINARY search 'API' --limit 10" \
    "Results should be ranked by relevance (scores shown)"

run_test "Check help command" \
    "$BINARY search --help" \
    "Should display helpful usage information"

run_test "Search with all available options" \
    "$BINARY search 'API' --doc-type task --session '0db373-search-cli-evaluation' --limit 5" \
    "Should work with all options combined"

echo -e "${YELLOW}══════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}                     TEST SUMMARY                         ${NC}"
echo -e "${YELLOW}══════════════════════════════════════════════════════════${NC}"
echo "" | tee -a $TEST_LOG

echo "Total Tests:  $test_count" | tee -a $TEST_LOG
echo -e "${GREEN}Passed:       $pass_count${NC}" | tee -a $TEST_LOG
echo -e "${RED}Failed:       $fail_count${NC}" | tee -a $TEST_LOG
echo "" | tee -a $TEST_LOG

if [ $fail_count -eq 0 ]; then
    echo -e "${GREEN}✅ ALL TESTS PASSED!${NC}" | tee -a $TEST_LOG
else
    echo -e "${YELLOW}⚠️  Some tests failed. Review $TEST_LOG for details.${NC}" | tee -a $TEST_LOG
fi

echo "" | tee -a $TEST_LOG
echo "Full test log saved to: $TEST_LOG"
echo ""
