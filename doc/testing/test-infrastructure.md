<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Test Infrastructure Documentation

## Overview

The CIM Domain project has comprehensive hermetic tests with no external service dependencies. Tests are fast, deterministic, and focused on pure domain logic.

## Test Categories

### 1. Library Unit Tests (396 tests)
- **Location**: `src/` (inline tests)
- **Command**: `cargo test --lib`
- **Coverage**: All core domain modules
- **Requirements**: None (pure unit tests)

### 2. Infrastructure-Oriented Tests
- **Location**: `tests/infrastructure/`
- **Command**: `cargo test --test infrastructure_tests`
- **Coverage**: CQRS flows, projections (pure, in-memory models)

## External Services

No external services are required. Tests must not perform network or filesystem I/O.

### Test Isolation

Tests are designed to be isolated:
- Each test creates its own buckets/streams
- Resources are cleaned up after completion
- Tests can run in parallel without conflicts

## CI/CD Integration

### Test Results Capture

The `scripts/capture-test-results.sh` script:
1. Runs all tests
2. Captures results in JSON format
3. Generates summary statistics
4. Creates markdown reports

Output files:
- `test-results/summary.json` - Machine-readable results
- `test-results/report.md` - Human-readable report
- `test-results/test-output.txt` - Raw test output

### GitHub Actions Workflow

The `.github/workflows/test-dashboard.yml` workflow:
1. Runs the test capture script
2. Uploads results as artifacts
3. Optionally comments on PRs with results
4. Provides data for dashboard display

### Dashboard

The `dashboard/` directory contains:
- `index.html` - Web dashboard for viewing results
- `test-server.py` - Local development server

To view locally:
```bash
# Run tests and capture results
./scripts/capture-test-results.sh

# Start dashboard server
cd dashboard
python3 test-server.py

# Open http://localhost:8080
```

## Test Metrics

Current test metrics vary by commit; aim for fast execution and high coverage.

### Performance Characteristics

- Unit tests: <1 second
- Infrastructure tests: ~3 seconds (pure, in-memory)

## Best Practices

1. **Use specific test commands** - For faster iteration
2. **Check test output** - Some tests log useful debugging info
3. **Monitor test times** - Long-running tests may indicate issues

## Troubleshooting

### Test Timeouts

Tests should execute quickly. Use `--nocapture` to see progress when debugging.

### Debugging Failed Tests

```bash
# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --exact --nocapture

# Set Rust backtrace
RUST_BACKTRACE=1 cargo test
```

## Future Improvements

1. **Test parallelization** - Further optimize test execution
2. **Coverage reporting** - Add code coverage metrics
3. **Performance benchmarks** - Track performance over time
4. **Flaky test detection** - Identify and fix intermittent failures
