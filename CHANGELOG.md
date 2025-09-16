# Changelog

All notable changes to cim-domain will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.5] - 2025-09-14
## [0.7.6] - 2025-09-15

### Added
- Dialog DAG Tools (tools/dialog_dag) to maintain `dialog-dag.json` outside the core library. Includes:
  - `log_dialog_event`: append dialog events with proper CIDv1 derived from content (Blake3 → Multihash 0x1e → CIDv1 0x55).
  - `merge_dialog_dag`: merge continuation files, de-duping by `cid` and preserving chronological order.
  - `reindex_dialog_cids`: recompute CIDs and fix parent links for an existing `dialog-dag.json`.

### Changed
- CQRS Display output standardized:
  - `CorrelationId` now displays as `correlation:<value>` (Single and Transaction).
  - `CausationId` now displays as `causation:<uuid>`.
  - Note: If downstream code parses previous string formats, adjust parsers accordingly.

### Fixed
- Implemented `Clone` for `TxOutput` to satisfy `TransitionOutput: Clone`. Clone semantics intentionally drop events (outputs are consumed), avoiding cloning trait objects.

### Documentation
- README: added “Dialog DAG Tools” section with quickstart usage, clarifying that tools live outside the pure library boundary.

### Changed
- Re-scoped crate as a pure domain library; no persistence, routing, or external I/O.
- Updated AGENTS.md to import .claude rules and clarify purity and testing expectations.

### Removed
- Deleted NATS/JetStream/IPLD/subject-based examples and demos.
- Removed all NATS- and persistence-dependent integration tests and related Cargo entries.

### Documentation
- Added repository contributor guide (AGENTS.md) tailored for pure-library usage.

## [0.7.0] - 2025-01-14

### Added
- **Entity as MONAD**: Implemented full monadic pattern for entities with proper monad laws
- **UUID v7**: Switched from UUID v4 to v7 for time-ordered identifiers
- **Mealy State Machines**: Aggregates now formally modeled as Mealy machines
- **FP-First Architecture**: Complete functional programming patterns with pragmatic breaks
- **Component System**: ECS pattern with Entity as the bridging monad
- **Phantom Types**: Type-safe EntityId<T> with zero runtime cost
- **Kleisli Composition**: Support for composing monadic transformations

### Changed
- **BREAKING**: EntityId::new() now generates UUID v7 instead of v4
- **BREAKING**: Aggregate trait redesigned around Mealy state machine pattern
- **BREAKING**: Component storage now uses FP-aligned immutable patterns
- All domain patterns now follow formal DDD structure (ValueObject, Entity, Aggregate, Policy, Saga)
- Event system enhanced with proper causation and correlation tracking

### Fixed
- Removed circular dependencies between domain concepts
- Improved type safety with phantom types
- Better separation of concerns between pure and effectful code

### Documentation
- Added comprehensive FP pattern documentation
- Documented all pragmatic breaks from pure FP with reasons
- Added monad law verification examples
- Included Mealy machine formal specification

## [0.5.0] - 2024-12-15

### Added
- Initial domain-driven design components
- Basic entity and aggregate support
- Event sourcing foundations
- CQRS pattern implementation

## [0.4.0] - 2024-11-01

### Added
- Component storage system
- Domain metadata support
- Subject-based routing

## [0.3.0] - 2024-10-01

### Added
- CID-based content addressing
- JetStream event store integration
- Persistence layer abstractions

## [0.2.0] - 2024-09-01

### Added
- Core domain traits
- Value object support
- Basic aggregate patterns

## [0.1.0] - 2024-08-01

### Added
- Initial release
- Foundation DDD building blocks
