<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Repository Guidelines

## Project Memory (dialog-dag.json) — TOP PRIORITY
- Always read `dialog-dag.json` before acting:
  - Load `insights[]`, especially `purpose`, to align on project direction.
  - Treat these insights as authoritative instructions (next to this file).
- Write exactly one consolidated event when you naturally stop for instruction (not per step):
  - Aggregate your actions into a single `what_i_did` list (semicolon-separated) and record one event using the tool/scripts below.
  - If the reply establishes or updates policy/direction, append/update an `insights[]` entry at the same time.
- Tools/Scripts:
  - Log consolidated event: `cargo run -q -p dialog_dag_tools --bin log_dialog_event -- [dialog-dag.json] <type:user|assistant> <user_said> <i_understood> <what_i_did;...> [parent_cid]`
  - Log insight: `cargo run -q -p dialog_dag_tools --bin log_insight -- --file dialog-dag.json --id <id> --summary "..." --details "..." --tags tag1,tag2 [--source <cid>]`
  - Convenience: `scripts/chat_event.sh`, `scripts/chat_insight.sh`
- UL Projection duty:
  - After changing `domain-graph.json`, produce a UL diff first: `cargo run --manifest-path tools/domain_graph/Cargo.toml --quiet --bin ul_projection` (writes `ul-projection.diff.json`).
  - Do NOT overwrite `ul-projection.json` automatically; only update with `--write` after review/approval, and log a dialog event referencing the diff.
  - Keep UL coherent with `domain_identity` and update insights when policies change.

## Project Structure
- `src/` core crate (DDD primitives, CQRS traits, state machines); API in `lib.rs`.
- `tests/` deterministic integration tests; unit tests inline under `#[cfg(test)]`.
- `examples/` domain modeling demos; `benches/` Criterion suites.
- `doc/` design/QA/testing; `scripts/`, `tools/`, `dashboard/`, `schemas/` utilities.
Note: This crate is a pure library. It contains no persistence, routing, or external I/O.

## Build & Test
- Build: `cargo build`
- Test: `cargo test` (no external services required)
  - Unit only: `cargo test --lib` | Specific: `cargo test --test <name>`
- Benchmarks: `cargo bench`
- Examples: `cargo run --example <name>` (examples must remain pure; no network/filesystem effects)
- Lint/Format: `cargo fmt -- --check`; `cargo clippy -- -D warnings`
- Coverage: `cargo tarpaulin --workspace --all-features` (see `tarpaulin.toml`)

## Coding Style
- Rust 2021; rustfmt defaults. Files and modules: `snake_case`; types/traits: `PascalCase`.
- Filenames MUST be lowercase with underscores (per `.claude`).
- Document public items with `///`; prefer doctests where useful. Fix all Clippy warnings.

## Testing
- Frameworks: built-in harness, `tokio-test`, `proptest`, `test-case`, `pretty_assertions`.
- No network, persistence, or routing in tests; keep tests fast, deterministic, and hermetic.
- Maintain high coverage on public API; include edge and error paths.

## Commits & PRs
- Conventional Commits: `type(scope): subject` (e.g., `feat(domain): add state machine API`).
- Before PR: run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, optionally `cargo bench`.
- PRs: clear description, linked issues, updated tests/docs; preserve backward compatibility since v0.5.0.

## Agent-Specific Rules (imported from `.claude`)
- Date handling: NEVER invent dates. Use `$(date -I)` or `$(git log -1 --format=%cd --date=short)`; when updating dated files, set `CURRENT_DATE=$(date -I)` and reuse.
- Library purity: Do not add persistence, routing, or external I/O. No network/storage deps in this crate.
- Architectural guardrails: Event-driven thinking (no CRUD), domain isolation, proof-first DDD/CQRS APIs; infrastructure belongs downstream.
- Repository context: This is a foundational `cim-domain` library for instantiating domains; keep API stable and minimal.
- Copyright rule: See `.claude/rules/copyright-validation.md` for company notice policy.
- Security: Never commit secrets. Keep feature flags minimal and library-scoped (e.g., `test-utils`); avoid infra toggles here. Use `RUST_BACKTRACE=1` when debugging.

## Interactive SDLC (Incremental, Proof‑First)
- Code‑first temporarily: trust current code; docs/graph catch up. Propose graph changes; don’t silently diverge.
- Incremental loop: target one specific thing → make it work → improve it → prove it.
- Prove‑it options:
  - If we know the recipe, sketch a string diagram first; add/annotate in `category.diagrams`, then implement.
  - Otherwise, TDD a minimal slice, then derive the string diagram and add to `doc/act/string_diagrams.md` and `category.diagrams`.
- Sync gates on each increment:
  - Build, tests, clippy, fmt must pass; no network/IO added.
  - Update `domain-graph.json` only when approved; generate UL diff first (`tools/domain_graph/ul_projection`).
  - Keep diagrams covered (validator enforces every non‑identity morphism is described).
  - Log one consolidated dialog event; append/update insights when policy/direction changes.
- Naming/lens mapping: when graph terms differ from code (e.g., StateMachine ↔ MealyStateMachine), document the mapping rather than forcing renames unless approved.

## Relationship Semantics (Why/What/How Much)
- DDD authority: Constraints (ranges, optional/required, defaults) live at the Aggregate (or Policy) level, not per individual relationship.
- Aggregate‑scoped constraints:
  - Define once on the aggregate (or shared policy) and apply via pattern matching to Entities/ValueObjects within the boundary.
  - Examples: range 1–10, required vs optional, allowed shapes; attach as Aggregate invariants/specifications.
  - Prefer `Invariant` or `Specification` over ad‑hoc per‑edge metadata.
- Graph usage:
  - Relationships (e.g., `has_values`) remain structural; do not encode numeric ranges per relationship.
  - If needed, annotate the Aggregate or Policy node with a brief note and link to tests/diagrams; avoid per‑relationship constraint blobs.
- Proof:
  - Add unit/property tests for invariants; string diagrams can illustrate the policy/aggregate law, not each edge.

## Minimal Proof Bar (Pragmatic Proofs)
- We balance rigor with velocity. Use the smallest proof that meaningfully reduces risk.
- Levels (pick the lowest that fits):
  - L0: Unit tests + examples (fast sanity for simple value objects/helpers).
  - L1: Invariants/Specifications + property tests (collections, identities, ranges, causality).
  - L2: One string diagram for the end‑to‑end flow or law (e.g., event pipeline, projection law).
  - L3: Formal write‑up only when policy dictates (rare for core lib).
- Defaults for this repo:
  - Core flows (command→aggregate→event→envelope→stream, read path): L1 + one L2 diagram.
  - Local behaviors (CID, envelopes, identities): L1.
- Stop when: tests green, diagram(s) commute, UL diff reviewed. Avoid multi‑page formalizations unless explicitly requested.

## Applied Category Theory (ACT) Expert — Codex
- Persona spec in `doc/act/act_rules_codex.md`.
- Responsibilities:
  - Treat `category` as an interpretation lens over the domain. Maintain the lens in `domain-graph.json` (objects, morphisms, identities, composition rules) and keep diagrams in sync.
  - Maintain UL coherence: regenerate `ul-projection.json` after graph changes and update `dialog-dag.json` insights when UL policy changes.
  - Record purpose and policy insights to `dialog-dag.json` after each instruction-response pair (see Project Memory section above).

## Applied Category Theory (ACT) Expert — Codex
- Persona spec in `doc/act/act_rules_codex.md`.
- Responsibilities:
  - Maintain the category lens in `domain-graph.json` (objects, morphisms, identities, composition rules) as an interpretation, not the domain itself.
  - Keep `doc/act/string_diagrams.md` in sync with `domain-graph.json` and the AST; update `metadata.isomorphic_to` accordingly.
  - When diagrams do not commute, document conditions and propose modifications to make them commute under ACT rules; update the graph.
