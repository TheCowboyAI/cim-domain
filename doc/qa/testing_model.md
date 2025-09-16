# Testing Model: Unit + BDD (Gherkin) with Expected Event Streams

This repository uses a two‑level testing strategy:

- Unit tests (100% coverage of library code)
- Behavior‑Driven tests (Gherkin features + scenarios) that assert domain behaviors via Expected Event Streams

The goal is twofold: prove that undesirable states are unrepresentable and demonstrate that intended behaviors are fulfilled.

## 1) Unit Tests (Required: 100% coverage)

Principles
- Test all public API and all internal branches; prefer small, isolated tests.
- Prove that valid state transitions are achievable and invalid transitions are rejected.
- Keep tests hermetic: no network, no persistence, no transport. Pure function or in‑memory only.
- Use table‑driven tests for state machines and error paths.

Guidelines
- Put unit tests next to code under `#[cfg(test)]` or in `tests/` when exercising multiple modules.
- Assert both success and failure paths; if something can’t happen by construction, assert it won’t compile or is impossible to construct.
- Fix warnings and keep clippy clean under `-D warnings`.

## 2) BDD Tests (Gherkin)

BDD tests describe how a domain is used from the outside. We capture:
- Feature (capability), Scenarios (paths), and Steps (Given/When/Then)
- Expected Event Streams arising from commands

Structure
- Place Gherkin feature files under `doc/qa/features/*.feature`.
- BDD test code lives under `tests/bdd_*`. Use `include_str!()` to embed the feature text (no runtime I/O).
- Map `Given/When/Then` to domain operations using the library’s pure APIs.

Example: Expected Event Streams
- Given an aggregate state and an input command, the BDD test asserts the resulting event sequence and new state.
- For example aggregates modeled as Mealy machines, map `When <command>` to `output()` and `transition()` and compare against expected events.

Conventions
- “Given …” sets up state/context.
- “When …” performs an operation (command).
- “Then …” asserts final state and expected event stream.
- “And …” continues the previous clause type.

## Instructions for Codex (Authoring New Tests)

- Unit tests
  - For each public function/trait/method, add tests that cover normal, boundary, and error cases.
  - For state machines, enumerate legal transitions and assert illegal ones fail.
  - Maintain a 1:1 mapping with branches; if a `match` has 6 arms, test all 6.

- BDD tests
  - Add a Gherkin file under `doc/qa/features/` describing the Feature and Scenarios in business terms.
  - Add a corresponding `tests/bdd_<feature>.rs` file that:
    - `include_str!()` the `.feature` file
    - Implements a minimal step interpreter that maps steps to pure domain operations
    - Uses helper assertions to compare “expected event streams” and final state

- Event stream assertions
  - Use the library’s event output types; if a domain returns `Vec<Box<dyn DomainEvent>>`, assert type and order.
  - Where event payloads are value objects, assert equality by value; for time‑sensitive fields, normalize or stub inputs.

- Purity and speed
  - No network/filesystem at runtime for tests. Use inline data or `include_str!()` for features.
  - Keep tests deterministic and fast.

## Minimal BDD Helper (pattern)

- Parse only what you need for the scenario; don’t add heavy parsers.
- Prefer simple `match` on lines beginning with `Given|When|Then|And` and route to handlers.
- Keep the interpreter in the test file or behind `#[cfg(test)]`.

## Example Files
- `doc/qa/features/transaction_state.feature`: sample scenarios
- `tests/bdd_transaction_state.rs`: example BDD test interpreting the feature file and asserting final state + event stream.
- `tests/bdd_support.rs`: shared helpers to assert expected event streams.
- `doc/qa/templates/bdd_test_template.rs`: copy‑and‑edit template for new BDD tests.

## Enforcing in Flake Checks

- `nix flake check` runs:
  - `cargo fmt --check`
  - `cargo clippy -D warnings`
  - `cargo test --workspace`
  - `cargo llvm-cov --fail-under-lines 100`

Codex must keep these checks green.
