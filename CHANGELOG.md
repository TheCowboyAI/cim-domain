<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.8] - 2025-09-21
### Added
- Production-grade architectural documentation (`doc/architecture/**`) describing real modules, domain authoring workflow, and tested patterns.
- `scripts/update_test_report.sh` helper to regenerate the consolidated test inventory and keep `doc/testing/test_report.md` current.
- ACT commutation proofs for the subject and domain-path diagrams with dedicated tests (`tests/act_diagram_commutation_tests.rs`).

### Changed
- `TransactionState` transitions now emit deterministic events so BDD scenarios assert full event streams.
- Transaction state BDD feature expanded to cover every transition, including cancellation after apply and invalid commits.
- QA feature mappings updated to include new diagram and BDD coverage; test inventory regenerated to 281 tests.
- Nix flake now builds only the library (tools moved out of the workspace) and installs the `.rlib` artifact in the Nix store.

[0.7.8]: https://github.com/thecowboyai/cim-domain/releases/tag/v0.7.8
