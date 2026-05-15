# Architecture Documentation

This directory documents the mathematical and systems architecture of CAT.

## Contents

| File | Purpose |
| --- | --- |
| `CAT_Architecture.md` | Formal theory bounds, state-vector equations, collapse predicates, tick loop, QuadTree behavior, data pipeline, and complexity notes. |

## Scope

Architecture documentation explains why the system is structured the way it is.
It is not a user tutorial and not a philosophical essay. It should connect the
model equations to implementation choices:

- Closed-form state-vector evaluation.
- Six-phase simulation loop.
- Deterministic random number isolation.
- Sequential collapse mutation after parallel state advancement.
- QuadTree rebuild strategy.
- Archive manifest semantics.
- Complexity and performance expectations.

## Maintenance Rules

- Update this directory when model dynamics, phase ordering, spatial indexing,
  data export semantics, or performance assumptions change.
- Keep diagrams in Mermaid when possible so they remain version-control friendly.
- Keep claims testable against source code and existing validation commands.
