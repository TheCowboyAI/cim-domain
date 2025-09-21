<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Diagram Style (CIM Domain)

Goals: readable, consistent, minimal crossings, grouped by concept.

Palette (semantic):
- Aggregate (container): teal stroke `#0ea5e9`, light fill `#f0f9ff`
- DomainEntity: blue stroke `#2563eb`, light fill `#eff6ff`
- ValueObject: indigo stroke `#6366f1`, light fill `#eef2ff`
- Command/Query: amber/cyan stroke `#f59e0b` / `#06b6d4`, very light fill
- DomainEvent/Envelope/Stream: rose/violet/gray `#f43f5e` / `#7c3aed` / `#6b7280`
- Projection/ReadModel: purple/emerald `#8b5cf6` / `#10b981`
- StateMachine/Policy: slate `#475569` / lime `#84cc16`
- CID: zinc `#71717a`

Layout rules:
- Left→Right dataflow: Commands → Aggregate → Events → Envelope/Stream → Projection → ReadModel → Queries.
- Nesting shows composition: Aggregate contains Entities; Entities contain ValueObjects.
- Edges straight when possible; avoid crossings by vertical staggering.
- Labels close to edges; short verbs: handled_by, emits, wraps, appends_to, subscribes_to, consumes, updates, reads, responds, governed_by, constrained_by.

File format:
- Store final as static SVG; keep styles in diagram for portability.
- Prefer grouped `<g>` blocks with ids for major sections to ease future edits.
