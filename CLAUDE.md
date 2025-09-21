<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 🚨 CRITICAL: PRAGMATIC FUNCTIONAL PROGRAMMING 🚨

**THIS CODEBASE PREFERS FP WITH DOMAIN-DRIVEN DESIGN**

### FORMAL DOMAIN STRUCTURE (REQUIRED):
**Every domain concept has a marker trait**
- **ValueObject**: Immutable, compared by value
- **DomainEntity**: Has identity beyond attributes
- **Aggregate**: Mealy state machine with transitions
- **Policy**: Pure business rules
- **Saga**: Composed aggregate (aggregate of aggregates)

### ECS PATTERN (FUNDAMENTAL):
**Entity-Component-System with Entity as MONAD**
- **Entity** = The MONAD that bridges DDD and ECS
- **Components** = Pure data with invariants (no methods)
- **Systems** = Kleisli arrows (A → Entity<B>)
- **Events** = Monadic transformations
- **Aggregates** = Mealy machines (output depends on state + input)
- **State Transitions** = Form directed graph AND Markov chain

### PREFER FP (Default Approach):
- ✅ Pure functions returning new values
- ✅ Algebraic Data Types with pattern matching  
- ✅ Immutable data structures
- ✅ Effects as data (Event Sourcing)
- ✅ Function composition
- ✅ Type-safe EntityId<T> with phantom types

### ACCEPTABLE BREAKS FROM FP (Must Document WHY):
- ✅ `&mut self` in performance-critical paths
- ✅ Mutable state at I/O boundaries (NATS, storage)
- ✅ Resource management (RAII, Drop traits)
- ✅ Caching/memoization for expensive operations
- ✅ Repository pattern at storage boundaries

### DOCUMENTATION REQUIRED:
When breaking FP, add comment:
```rust
// BREAKING FP: [what] 
// REASON: [why necessary]
```

See `.claude/instructions/pragmatic-fp-domain.md` for patterns.

## Repository Overview

CIM-IPLD is a content-addressed storage system for the Composable Information Machine using IPLD (InterPlanetary Linked Data). It provides robust support for various content types, IPLD codecs, chain validation, and distributed storage through NATS JetStream.

## 🚨 SOURCE OF TRUTH: context-graph.json 🚨

**`context-graph.json`** is the authoritative source for:
- Current SDLC phase and approval status
- Bounded contexts (Categories in mathematical model)
- Domain aggregates and their relationships
- Integration events between contexts
- Hamiltonian paths/cycles defining complete aggregate traversal
- Three-force model parameters (gravity, repulsion, attention)

**ALWAYS** check `context-graph.json` before making architectural decisions or determining next steps.


