# Health Check Testing Guide

This guide provides step-by-step instructions for testing the Neo4j health check functionality.

## Quick Start

### 1. Run Tests Without Neo4j (Fastest)

```bash
# Run all unit tests
cargo test --lib

# Run connection failure tests
cargo test --test health_check_failures

# Results: 13/13 tests passing
```

These tests cover:
- Health status types and conversions
- Health check result creation
- Error handling and formatting
- Connection failure detection
- JSON serialization

### 2. Run Tests With Neo4j (Complete Validation)

#### Option A: Using Docker (Recommended)

```bash
# 1. Start Neo4j
docker run -d \
  --name neo4j-test \
  -p 7474:7474 -p 7687:7687 \
  -e NEO4J_AUTH=neo4j/password \
  neo4j:latest

# 2. Wait for Neo4j to start (about 30 seconds)
echo "Waiting for Neo4j to start..."
sleep 30

# 3. Verify Neo4j is running
curl -I http://localhost:7474 || echo "Neo4j not ready, wait longer"

# 4. Run all tests
export NEO4J_URI="bolt://localhost:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASSWORD="password"
cargo test -- --ignored --nocapture

# 5. Run manual validation
cargo run --example manual_health_check

# 6. Cleanup
docker stop neo4j-test
docker rm neo4j-test
```

#### Option B: Using Automated Script

```bash
# Make script executable (if not already)
chmod +x test_health_check.sh

# Run all tests with Docker Neo4j
docker run -d --name neo4j-test -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=neo4j/password neo4j:latest
sleep 30

# Run test suite
./test_health_check.sh --all

# Cleanup
docker stop neo4j-test && docker rm neo4j-test
```

#### Option C: Using Local Neo4j

```bash
# Ensure Neo4j is running on localhost:7687

# Set environment variables (adjust as needed)
export NEO4J_URI="bolt://localhost:7687"
export NEO4J_USER="neo4j"
export NEO4J_PASSWORD="your-password"
export NEO4J_DATABASE="neo4j"

# Run all tests
cargo test -- --ignored --nocapture

# Or use the test script
./test_health_check.sh --all
```

## Test Categories

### Unit Tests (No Dependencies)

```bash
cargo test --lib
```

**Coverage:**
- Health status types (Healthy, Degraded, Unhealthy)
- HTTP status code mapping (200, 503)
- Health check result creation
- Configuration defaults
- Error type creation and formatting

**Expected Output:**
```
test result: ok. 8 passed; 0 failed; 0 ignored
```

### Integration Tests - Connection Failures

```bash
cargo test --test health_check_failures
```

**Coverage:**
- Invalid hostname handling
- Invalid port handling
- Invalid URI scheme detection
- Error serialization
- Connection error detection

**Expected Output:**
```
test result: ok. 5 passed; 0 failed; 6 ignored
```

### Integration Tests - With Neo4j

```bash
cargo test --test health_check_test -- --ignored --nocapture
cargo test --test health_check_failures -- --ignored --nocapture
```

**Coverage:**
- Simple health check (RETURN 1)
- Standard health check (CALL db.ping())
- Detailed health check (CALL db.info())
- Health check with retry logic
- Custom configuration
- Fallback mechanism
- Degraded state detection
- Concurrent health checks
- Authentication failures
- Invalid database handling

### Manual Testing

```bash
cargo run --example manual_health_check
```

**What it tests:**
1. Connection establishment
2. All three health check methods (simple, standard, detailed)
3. Retry logic
4. Custom configurations
5. Fallback mechanisms
6. Concurrent execution
7. Degraded state detection
8. JSON serialization

**Expected Output:**
```
=== Neo4j Health Check Manual Testing ===

Test 1: Connecting to Neo4j...
✓ Successfully connected to Neo4j

Test 2: Simple health check (RETURN 1)...
✓ Simple health check passed

Test 3: Standard health check (CALL db.ping())...
✓ Standard health check passed: Healthy

Test 4: Detailed health check (CALL db.info())...
✓ Detailed health check completed:
  Status: Healthy
  Response time: 15ms
  Database name: neo4j
  ...

[Additional test results...]

=== Summary ===
All manual health check tests completed!
✓ Connection: Working
✓ Simple health check: Working
✓ Detailed health check: Working
✓ Retry mechanism: Working
✓ Concurrent checks: Working
✓ JSON serialization: Working
```

## Environment Variables

All tests support these environment variables:

```bash
# Neo4j connection URI
export NEO4J_URI="bolt://localhost:7687"

# Authentication credentials
export NEO4J_USER="neo4j"
export NEO4J_PASSWORD="password"

# Database name
export NEO4J_DATABASE="neo4j"
```

If not set, tests use these defaults:
- URI: `bolt://localhost:7687`
- User: `neo4j`
- Password: `password`
- Database: `neo4j`

## Test Script Usage

The `test_health_check.sh` script provides an interactive menu:

```bash
./test_health_check.sh
```

**Options:**
1. Check Neo4j connection
2. Run unit tests
3. Run integration tests (no Neo4j required)
4. Run integration tests (Neo4j required)
5. Run manual test example
6. Run all tests
7. Run benchmarks
q. Quit

**Command-line flags:**
```bash
./test_health_check.sh --help                     # Show help
./test_health_check.sh --all                      # Run all tests
./test_health_check.sh --unit                     # Unit tests only
./test_health_check.sh --integration-no-neo4j     # Integration tests (no Neo4j)
./test_health_check.sh --integration-with-neo4j   # Integration tests (with Neo4j)
./test_health_check.sh --manual                   # Manual test example
./test_health_check.sh --bench                    # Run benchmarks
```

## Testing Different Neo4j Versions

### Neo4j 5.x (Latest)

```bash
docker run -d --name neo4j-5 -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=neo4j/password neo4j:5-community
sleep 30
cargo test -- --ignored --nocapture
docker stop neo4j-5 && docker rm neo4j-5
```

**Expected:** All tests pass, `db.ping()` available

### Neo4j 4.4

```bash
docker run -d --name neo4j-44 -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=neo4j/password neo4j:4.4-community
sleep 30
cargo test -- --ignored --nocapture
docker stop neo4j-44 && docker rm neo4j-44
```

**Expected:** All tests pass, `db.ping()` available

### Neo4j 4.0 (Testing Fallback)

```bash
docker run -d --name neo4j-40 -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=neo4j/password neo4j:4.0-community
sleep 30
cargo test test_fallback_from_ping_to_simple -- --ignored --nocapture
docker stop neo4j-40 && docker rm neo4j-40
```

**Expected:** Fallback to `RETURN 1` when `db.ping()` not available

## Troubleshooting

### Problem: "Connection refused"

**Solution:**
```bash
# Check if Neo4j is running
docker ps | grep neo4j

# Check logs
docker logs neo4j-test

# Wait longer for Neo4j to start
sleep 60
```

### Problem: "Authentication failed"

**Solution:**
```bash
# Check Neo4j authentication
docker exec neo4j-test cypher-shell -u neo4j -p password "RETURN 1"

# Or reset password
docker exec neo4j-test cypher-shell -u neo4j -p neo4j "ALTER CURRENT USER SET PASSWORD FROM 'neo4j' TO 'password'"
```

### Problem: "Port already in use"

**Solution:**
```bash
# Find and stop existing Neo4j
docker ps -a | grep neo4j
docker stop $(docker ps -aq --filter "ancestor=neo4j")

# Or use different ports
docker run -d --name neo4j-test -p 17474:7474 -p 17687:7687 -e NEO4J_AUTH=neo4j/password neo4j:latest
export NEO4J_URI="bolt://localhost:17687"
```

### Problem: Tests hang or timeout

**Solution:**
```bash
# Increase timeout by setting RUST_TEST_TIMEOUT
export RUST_TEST_TIMEOUT=120

# Or run tests individually
cargo test test_health_check_simple -- --ignored --nocapture
```

## Performance Testing

To benchmark health check performance:

```bash
# Run benchmarks (if implemented)
cargo bench --bench health_check_bench

# Or use manual timing
time cargo run --example manual_health_check
```

**Expected performance:**
- Simple check: 1-5ms
- Standard check: 5-10ms
- Detailed check: 10-20ms

## Continuous Integration

Example GitHub Actions workflow:

```yaml
name: Test Health Checks

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      neo4j:
        image: neo4j:latest
        ports:
          - 7687:7687
        env:
          NEO4J_AUTH: neo4j/password
        options: >-
          --health-cmd "cypher-shell -u neo4j -p password 'RETURN 1'"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run unit tests
        run: cargo test --lib

      - name: Run integration tests
        run: cargo test -- --ignored --nocapture
        env:
          NEO4J_URI: bolt://localhost:7687
          NEO4J_USER: neo4j
          NEO4J_PASSWORD: password

      - name: Run manual test
        run: cargo run --example manual_health_check
```

## Summary

**Quick validation (no Neo4j):**
```bash
cargo test --lib && cargo test --test health_check_failures
```

**Full validation (with Docker Neo4j):**
```bash
docker run -d --name neo4j-test -p 7474:7474 -p 7687:7687 -e NEO4J_AUTH=neo4j/password neo4j:latest && \
sleep 30 && \
cargo test -- --ignored --nocapture && \
cargo run --example manual_health_check && \
docker stop neo4j-test && docker rm neo4j-test
```

**Interactive testing:**
```bash
./test_health_check.sh
```

For questions or issues, refer to:
- `TEST_RESULTS.md` - Detailed test results and coverage
- `tests/health_check_test.rs` - Integration test source
- `tests/health_check_failures.rs` - Error scenario tests
- `examples/manual_health_check.rs` - Manual validation example
