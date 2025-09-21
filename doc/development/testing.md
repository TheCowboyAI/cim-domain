<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Testing Guide

The goal of this guide is to describe how we prove the behaviour that ships in this crate today. Everything below is backed by the commands we run in CI and locally.

## Test Pyramid

| Layer | Location | What it Covers |
| --- | --- | --- |
| Unit | `src/**/*` (`#[cfg(test)]`) | Invariants, state machines, algebra laws |
| Integration | `tests/*.rs` | ACT diagram coverage, UL naming, CQRS flows, content addressing |
| BDD | `examples/domain_examples/tests/*.rs` | End‑to‑end transaction lifecycle with explicit event streams |
| Tooling | `tools/*` tests | Dialog DAG + domain graph CLI tools |

Run everything with:

```
cargo test --all-targets --all-features
```

The command executes 281 tests in <10 seconds on a developer laptop.

## Critical Gates

1. **ACT Strict Mode**  
   `cargo test --features act_strict -- tests::act`  
   Ensures every morphism in `domain-graph.json` has a verified diagram, commutation proofs exist (see `tests/act_diagram_commutation_tests.rs`), and UL naming policies hold.

2. **BDD Regression**  
   `cargo test -p cim-domain-examples --test bdd_transaction_state`  
   Proves every transaction scenario exercises and asserts the full event stream. The scenarios cover start → commit, cancellation, validation failure, cancel after apply, and an illegal transition rejection.

3. **Feature Mapping Guard**  
   `tests/act_feature_mapping_strict.rs` fails the build if a feature lacks TDD assets or if a new integration test is not mapped in `doc/qa/features/index.yaml`.

4. **Test Inventory Report**  
   `doc/testing/test_report.md` is regenerated from `cargo test --workspace --all-targets --all-features -- --list`. If you add or remove tests, rerun the command and commit the updated report.

## Adding New Tests

1. **Write the test** next to the code (`src/...`) or under `tests/`. Keep imports local—no global test helpers.
2. **Map it** in `doc/qa/features/index.yaml` so the QA gate knows which feature it proves.
3. **Update diagrams** if the behaviour introduces a new morphism or law. Add a commutation test if a new diagram is created.
4. **Regenerate the test report** with `scripts/update_test_report.sh` (wrapper around the `cargo test -- --list` command).

## Coverage Notes

We do not rely on opaque coverage percentages; instead we make sure every diagram and BDD scenario is exercised. When you need numerical coverage for a downstream report, run tarpaulin on a host that permits `ptrace`:

```
cargo tarpaulin --workspace --all-features --out Json
```

(Our sandbox blocks `ptrace`, so these runs must happen on a developer machine or CI runner configured for it.)

## Troubleshooting

- **Missing feature mapping** → update `doc/qa/features/index.yaml`.  
- **ACT naming failure** → rename the morphism or document an exception in the QA file.  
- **Diagram validation failure** → run `cargo run --manifest-path tools/domain_graph/Cargo.toml --quiet --bin validate_domain_graph` and inspect the output; every morphism must point to a diagram with `verified: true`.

Keep the tests fast, deterministic, and hermetic. If you find yourself needing IO, move that behaviour to downstream infrastructure crates.
