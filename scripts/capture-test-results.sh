#!/usr/bin/env bash
# Copyright (c) 2025 - Cowboy AI, LLC.

# Script to capture test results for CI/CD dashboard
# This script runs tests and generates a JSON report with metrics

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Running tests and capturing results...${NC}"

# Create output directory
mkdir -p test-results

# Run date
TEST_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Run tests with JSON output
echo "Running tests..."
cargo test -- -Z unstable-options --format json --report-time 2>/dev/null > test-results/raw-output.json || true

# Also run with normal output to get summary
cargo test 2>&1 | tee test-results/test-output.txt

# Extract test counts from the output
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
IGNORED_TESTS=0

# Parse each test result line
while IFS= read -r line; do
    if [[ $line =~ "test result: ok."[[:space:]]+([0-9]+)[[:space:]]+"passed"[[:space:]]*";"[[:space:]]*([0-9]+)[[:space:]]+"failed"[[:space:]]*";"[[:space:]]*([0-9]+)[[:space:]]+"ignored" ]]; then
        PASSED="${BASH_REMATCH[1]}"
        FAILED="${BASH_REMATCH[2]}"
        IGNORED="${BASH_REMATCH[3]}"
        
        TOTAL_TESTS=$((TOTAL_TESTS + PASSED + FAILED))
        PASSED_TESTS=$((PASSED_TESTS + PASSED))
        FAILED_TESTS=$((FAILED_TESTS + FAILED))
        IGNORED_TESTS=$((IGNORED_TESTS + IGNORED))
    fi
done < test-results/test-output.txt

# Calculate pass rate
if [ $TOTAL_TESTS -gt 0 ]; then
    PASS_RATE=$(( (PASSED_TESTS * 100) / TOTAL_TESTS ))
else
    PASS_RATE=0
fi

# Determine overall status
if [ $FAILED_TESTS -eq 0 ]; then
    STATUS="success"
    STATUS_EMOJI="✅"
else
    STATUS="failure"
    STATUS_EMOJI="❌"
fi

# Create summary JSON
cat > test-results/summary.json <<EOF
{
  "timestamp": "$TEST_DATE",
  "status": "$STATUS",
  "total_tests": $TOTAL_TESTS,
  "passed": $PASSED_TESTS,
  "failed": $FAILED_TESTS,
  "ignored": $IGNORED_TESTS,
  "pass_rate": $PASS_RATE,
  "test_suites": {
    "library": {
      "name": "Library Unit Tests",
      "count": 396
    },
    "infrastructure": {
      "name": "Infrastructure Integration Tests",
      "count": 19
    },
    "jetstream": {
      "name": "JetStream Event Store Tests",
      "count": 6
    },
    "persistence": {
      "name": "Persistence Integration Tests",
      "count": 7
    }
  },
  "environment": {
    "rust_version": "$(rustc --version | cut -d' ' -f2)",
    "nats_required": true,
    "nats_endpoint": "localhost:4222"
  }
}
EOF

# Generate Markdown report
cat > test-results/report.md <<EOF
# Test Results Report

**Date**: $TEST_DATE  
**Status**: $STATUS_EMOJI $STATUS

## Summary

- **Total Tests**: $TOTAL_TESTS
- **Passed**: $PASSED_TESTS
- **Failed**: $FAILED_TESTS
- **Ignored**: $IGNORED_TESTS
- **Pass Rate**: $PASS_RATE%

## Test Suites

| Suite | Test Count | Description |
|-------|------------|-------------|
| Library | 396 | Core unit tests for all modules |
| Infrastructure | 19 | Integration tests for infrastructure components |
| JetStream | 6 | NATS JetStream event store tests |
| Persistence | 7 | Persistence layer integration tests |

## Requirements

- NATS server running on \`localhost:4222\` with JetStream enabled
- Rust $(rustc --version | cut -d' ' -f2)

## Test Execution

\`\`\`bash
# Start NATS
docker run -d -p 4222:4222 nats:latest -js

# Run all tests
cargo test
\`\`\`
EOF

# Display summary
echo -e "\n${GREEN}Test Results Summary:${NC}"
echo -e "Total Tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"
echo -e "Ignored: ${YELLOW}$IGNORED_TESTS${NC}"
echo -e "Pass Rate: $PASS_RATE%"
echo -e "\nResults saved to:"
echo -e "  - test-results/summary.json"
echo -e "  - test-results/report.md"
echo -e "  - test-results/test-output.txt"

# Exit with appropriate code
if [ $FAILED_TESTS -gt 0 ]; then
    exit 1
else
    exit 0
fi