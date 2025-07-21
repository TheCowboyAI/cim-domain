# Test Results Report

**Date**: 2025-07-21T20:54:17Z  
**Status**: ✅ success

## Summary

- **Total Tests**: 437
- **Passed**: 437
- **Failed**: 0
- **Ignored**: 0
- **Pass Rate**: 100%

## Test Suites

| Suite | Test Count | Description |
|-------|------------|-------------|
| Library | 396 | Core unit tests for all modules |
| Infrastructure | 19 | Integration tests for infrastructure components |
| JetStream | 6 | NATS JetStream event store tests |
| Persistence | 7 | Persistence layer integration tests |

## Requirements

- NATS server running on `localhost:4222` with JetStream enabled
- Rust 1.90.0-nightly

## Test Execution

```bash
# Start NATS
docker run -d -p 4222:4222 nats:latest -js

# Run all tests
cargo test
```
