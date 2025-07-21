# Test Infrastructure Documentation

## Overview

The CIM Domain project has comprehensive test coverage with 437 tests across multiple categories. All tests are now enabled and passing, including those that require NATS integration.

## Test Categories

### 1. Library Unit Tests (396 tests)
- **Location**: `src/` (inline tests)
- **Command**: `cargo test --lib`
- **Coverage**: All core domain modules
- **Requirements**: None (pure unit tests)

### 2. Infrastructure Integration Tests (19 tests)
- **Location**: `tests/infrastructure_tests.rs`
- **Command**: `cargo test --test infrastructure_tests`
- **Coverage**: Event sourcing, sagas, projections
- **Requirements**: NATS with JetStream

### 3. JetStream Event Store Tests (6 tests)
- **Location**: `tests/jetstream_event_store_tests.rs`
- **Command**: `cargo test --test jetstream_event_store_tests`
- **Coverage**: Event persistence, replay, snapshots
- **Requirements**: NATS with JetStream

### 4. Persistence Integration Tests (7 tests)
- **Location**: `tests/persistence_tests.rs`
- **Command**: `cargo test --test persistence_tests`
- **Coverage**: Repository patterns, KV store, TTL
- **Requirements**: NATS with JetStream

## NATS Requirements

All integration tests require NATS with JetStream enabled:

```bash
# Start NATS with JetStream
docker run -d -p 4222:4222 nats:latest -js

# Verify NATS is running
nc -zv localhost 4222
```

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
1. Sets up NATS service container
2. Runs test capture script
3. Uploads results as artifacts
4. Can comment on PRs with results
5. Provides data for dashboard display

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

Current test metrics (as of last run):
- **Total Tests**: 437
- **Pass Rate**: 100%
- **Execution Time**: ~15 seconds (with NATS)

### Performance Characteristics

- Unit tests: <1 second
- Infrastructure tests: ~3 seconds
- JetStream tests: ~10 seconds (due to stream operations)
- Persistence tests: ~3 seconds

## Best Practices

1. **Always run with NATS** - Many tests require it
2. **Use specific test commands** - For faster iteration
3. **Check test output** - Some tests log useful debugging info
4. **Monitor test times** - Long-running tests may indicate issues

## Troubleshooting

### NATS Connection Errors

If tests fail with connection errors:
```bash
# Check NATS is running
docker ps | grep nats

# Check port is available
lsof -i :4222

# Restart NATS
docker stop nats-server
docker run -d --name nats-server -p 4222:4222 nats:latest -js
```

### Test Timeouts

Some tests have longer timeouts for NATS operations:
- TTL tests wait for expiration (3+ seconds)
- Stream creation tests may take time
- Use `--nocapture` to see progress

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