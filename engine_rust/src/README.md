# Rust Source Modules

This directory contains production Rust source for the CAT Simulation Engine.
Each file has a narrow responsibility so mathematical behavior, orchestration,
spatial indexing, and output are auditable independently.

## Module Map

| File | Responsibility |
| --- | --- |
| `lib.rs` | Public library module declarations used by the binary and integration tests. |
| `main.rs` | CLI parsing, configuration assembly, run directory creation, logging setup, and simulation launch. |
| `agent.rs` | Agent state vectors, lifecycle states, collapse events, collapse thresholds, numerical guards, and derived metrics. |
| `simulation.rs` | Six-phase tick loop, deterministic RNG ownership, Rayon dispatch, spawning, extinction, statistics, and final export sequencing. |
| `grid.rs` | QuadTree spatial index, bounding boxes, range queries, radius queries, and depth diagnostics. |
| `exporter.rs` | JSON and CSV archive writer, atomic final-output writes, timestamped run directory creation, and run manifest generation. |

## Execution Flow

```text
main.rs
  -> builds SimulationConfig
  -> creates data/runs/<run>/
  -> Simulation::new
  -> Simulation::run
  -> Exporter finalization
```

Within each tick, `simulation.rs` executes:

```text
1. Rebuild QuadTree
2. Advance active agents
3. Evaluate transcendence and collapse
4. Sample spawning
5. Sample exogenous extinction
6. Compute aggregate statistics
```

## Mathematical Responsibilities

`agent.rs` is the primary mathematical boundary. It evaluates:

```text
E(t) = E0 * exp(r * t)
T(t) = T0 * max(0, 1 - alpha * ln(1 + t))
C(t) = clamp(C0 + delta * t, 0, 1)
```

These equations are evaluated directly from initial conditions at every tick.
This is deliberate: direct evaluation avoids cumulative integration drift and
keeps test expectations analytically clear.

## Maintenance Rules

- Keep public functions documented.
- Keep model equations synchronized with `docs/architecture/CAT_Architecture.md`.
- Do not introduce shared mutable state inside Rayon agent updates.
- Do not use `unwrap()` or `expect()` in production code without documenting
  the invariant that makes failure impossible.
- Keep output field names stable unless `docs/api/API_Reference.md` is updated.
