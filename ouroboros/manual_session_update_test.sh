#!/bin/bash
# Manual Session Update Testing Script
# This script demonstrates the session update functionality in action

set -e

echo "=========================================="
echo "Session Update Manual Testing"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test 1: Run all integration tests
echo -e "${BLUE}Test 1: Running All Integration Tests${NC}"
echo "----------------------------------------"
cargo test --test session_update_test --quiet
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ All integration tests passed${NC}"
else
    echo -e "${RED}✗ Integration tests failed${NC}"
    exit 1
fi
echo ""

# Test 2: Run unit tests
echo -e "${BLUE}Test 2: Running Unit Tests${NC}"
echo "----------------------------------------"
cargo test work_session::tests --quiet
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ All unit tests passed${NC}"
else
    echo -e "${RED}✗ Unit tests failed${NC}"
    exit 1
fi
echo ""

# Test 3: Run with verbose output to see details
echo -e "${BLUE}Test 3: Verbose Test Execution${NC}"
echo "----------------------------------------"
echo -e "${YELLOW}Running test_update_session_in_index_basic with output:${NC}"
cargo test --test session_update_test test_update_session_in_index_basic -- --nocapture --show-output
echo ""

# Test 4: Run specific edge case tests
echo -e "${BLUE}Test 4: Edge Case Tests${NC}"
echo "----------------------------------------"
echo -e "${YELLOW}Testing orphan session handling:${NC}"
cargo test --test session_update_test test_update_session_nonexistent --quiet
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Orphan session test passed${NC}"
fi

echo -e "${YELLOW}Testing multiple concurrent sessions:${NC}"
cargo test --test session_update_test test_update_session_in_index_multiple_sessions --quiet
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Multiple sessions test passed${NC}"
fi
echo ""

# Test 5: Run performance test (measure execution time)
echo -e "${BLUE}Test 5: Performance Check${NC}"
echo "----------------------------------------"
echo -e "${YELLOW}Measuring test execution time:${NC}"
start_time=$(date +%s%N)
cargo test --test session_update_test --quiet
end_time=$(date +%s%N)
elapsed=$(( (end_time - start_time) / 1000000 ))
echo -e "${GREEN}✓ All tests completed in ${elapsed}ms${NC}"
echo ""

# Test 6: Check test coverage
echo -e "${BLUE}Test 6: Test Coverage Summary${NC}"
echo "----------------------------------------"
total_tests=9  # We have 9 integration tests
unit_tests=5   # We have 5 unit tests
echo "Integration tests: ${total_tests}"
echo "Unit tests: ${unit_tests}"
echo "Total tests: $((total_tests + unit_tests))"
echo ""

# Final summary
echo -e "${GREEN}=========================================="
echo "All Session Update Tests Passed! ✓"
echo "==========================================${NC}"
echo ""
echo "Test Report Details:"
echo "  - Integration Tests: ${total_tests} tests"
echo "  - Unit Tests: ${unit_tests} tests"
echo "  - Total Execution Time: ${elapsed}ms"
echo "  - Status: All tests passed"
echo ""
echo "For detailed test report, see: SESSION_UPDATE_TEST_REPORT.md"
echo ""
