# Changelog

All notable changes to cim-domain will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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