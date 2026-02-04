#!/bin/bash
# Health Check Testing Script
#
# This script helps test the Neo4j health check functionality
# with various scenarios.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
NEO4J_URI="${NEO4J_URI:-bolt://localhost:7687}"
NEO4J_USER="${NEO4J_USER:-neo4j}"
NEO4J_PASSWORD="${NEO4J_PASSWORD:-password}"
NEO4J_DATABASE="${NEO4J_DATABASE:-neo4j}"

print_header() {
    echo -e "${BLUE}======================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}======================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_info() {
    echo -e "$1"
}

# Check if Neo4j is running
check_neo4j() {
    print_header "Checking Neo4j Connection"

    if command -v cypher-shell &> /dev/null; then
        if cypher-shell -u "$NEO4J_USER" -p "$NEO4J_PASSWORD" -a "$NEO4J_URI" "RETURN 1" &> /dev/null; then
            print_success "Neo4j is running and accessible"
            return 0
        else
            print_error "Neo4j is not accessible at $NEO4J_URI"
            return 1
        fi
    else
        print_warning "cypher-shell not found, skipping connectivity check"
        print_info "Attempting to proceed anyway..."
        return 0
    fi
}

# Run unit tests
run_unit_tests() {
    print_header "Running Unit Tests"

    print_info "Running tests in src/connection.rs..."
    cargo test --lib connection::tests -- --nocapture

    print_info "Running tests in src/error.rs..."
    cargo test --lib error::tests -- --nocapture

    print_success "Unit tests completed"
}

# Run integration tests (without Neo4j)
run_integration_tests_no_neo4j() {
    print_header "Running Integration Tests (No Neo4j Required)"

    print_info "Running connection failure tests..."
    cargo test test_connection_failure_ --test health_check_failures -- --nocapture

    print_info "Running serialization tests..."
    cargo test test_health_check_result_serialization --test health_check_failures -- --nocapture

    print_info "Running error type tests..."
    cargo test test_error_types --test health_check_failures -- --nocapture

    print_success "Integration tests (no Neo4j) completed"
}

# Run integration tests (with Neo4j)
run_integration_tests_with_neo4j() {
    print_header "Running Integration Tests (Neo4j Required)"

    export NEO4J_URI NEO4J_USER NEO4J_PASSWORD NEO4J_DATABASE

    print_info "Running all ignored tests that require Neo4j..."
    cargo test --test health_check_test -- --ignored --nocapture

    print_info "Running failure scenario tests..."
    cargo test --test health_check_failures -- --ignored --nocapture

    print_success "Integration tests (with Neo4j) completed"
}

# Run manual testing example
run_manual_test() {
    print_header "Running Manual Health Check Example"

    export NEO4J_URI NEO4J_USER NEO4J_PASSWORD NEO4J_DATABASE

    print_info "Running manual health check example..."
    cargo run --example manual_health_check

    print_success "Manual test completed"
}

# Run benchmarks
run_benchmarks() {
    print_header "Running Performance Benchmarks"

    if [ -f "benches/health_check_bench.rs" ]; then
        print_info "Running Criterion benchmarks..."
        cargo bench --bench health_check_bench
        print_success "Benchmarks completed"
    else
        print_warning "No benchmark file found, skipping"
    fi
}

# Main menu
show_menu() {
    echo ""
    print_header "Neo4j Health Check Test Suite"
    echo ""
    echo "Configuration:"
    echo "  NEO4J_URI=$NEO4J_URI"
    echo "  NEO4J_USER=$NEO4J_USER"
    echo "  NEO4J_DATABASE=$NEO4J_DATABASE"
    echo ""
    echo "Test Options:"
    echo "  1) Check Neo4j connection"
    echo "  2) Run unit tests"
    echo "  3) Run integration tests (no Neo4j required)"
    echo "  4) Run integration tests (Neo4j required)"
    echo "  5) Run manual test example"
    echo "  6) Run all tests"
    echo "  7) Run benchmarks"
    echo "  q) Quit"
    echo ""
}

# Run all tests
run_all_tests() {
    print_header "Running Complete Test Suite"

    if check_neo4j; then
        run_unit_tests
        echo ""
        run_integration_tests_no_neo4j
        echo ""
        run_integration_tests_with_neo4j
        echo ""
        run_manual_test
        echo ""
        print_success "All tests completed successfully!"
    else
        print_error "Neo4j is not accessible. Running limited tests..."
        run_unit_tests
        echo ""
        run_integration_tests_no_neo4j
        echo ""
        print_warning "Skipped Neo4j-dependent tests"
    fi
}

# Parse command line arguments
if [ $# -gt 0 ]; then
    case "$1" in
        --all|-a)
            run_all_tests
            exit 0
            ;;
        --unit|-u)
            run_unit_tests
            exit 0
            ;;
        --integration-no-neo4j|-i)
            run_integration_tests_no_neo4j
            exit 0
            ;;
        --integration-with-neo4j|-I)
            check_neo4j && run_integration_tests_with_neo4j
            exit 0
            ;;
        --manual|-m)
            check_neo4j && run_manual_test
            exit 0
            ;;
        --bench|-b)
            run_benchmarks
            exit 0
            ;;
        --help|-h)
            echo "Usage: $0 [option]"
            echo ""
            echo "Options:"
            echo "  -a, --all                      Run all tests"
            echo "  -u, --unit                     Run unit tests only"
            echo "  -i, --integration-no-neo4j     Run integration tests (no Neo4j)"
            echo "  -I, --integration-with-neo4j   Run integration tests (with Neo4j)"
            echo "  -m, --manual                   Run manual test example"
            echo "  -b, --bench                    Run benchmarks"
            echo "  -h, --help                     Show this help"
            echo ""
            echo "Environment variables:"
            echo "  NEO4J_URI      - Neo4j connection URI (default: bolt://localhost:7687)"
            echo "  NEO4J_USER     - Neo4j username (default: neo4j)"
            echo "  NEO4J_PASSWORD - Neo4j password (default: password)"
            echo "  NEO4J_DATABASE - Neo4j database name (default: neo4j)"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
fi

# Interactive mode
while true; do
    show_menu
    read -p "Select option: " choice
    echo ""

    case "$choice" in
        1)
            check_neo4j
            ;;
        2)
            run_unit_tests
            ;;
        3)
            run_integration_tests_no_neo4j
            ;;
        4)
            if check_neo4j; then
                run_integration_tests_with_neo4j
            fi
            ;;
        5)
            if check_neo4j; then
                run_manual_test
            fi
            ;;
        6)
            run_all_tests
            ;;
        7)
            run_benchmarks
            ;;
        q|Q)
            print_info "Exiting..."
            exit 0
            ;;
        *)
            print_error "Invalid option: $choice"
            ;;
    esac

    echo ""
    read -p "Press Enter to continue..."
done
