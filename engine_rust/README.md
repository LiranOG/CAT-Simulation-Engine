# Rust Simulation Engine

This directory contains the high-performance simulation core for CAT. The Rust
crate owns model execution, deterministic sampling, spatial indexing, and
archive export.

## Contents

| Path | Purpose |
| --- | --- |
| `Cargo.toml` | Package metadata, dependency declarations, profile settings, library target, and binary target. |
| `Cargo.lock` | Locked dependency graph for reproducible builds. |
| `src/` | Production Rust modules for CLI, model state, simulation orchestration, spatial indexing, and exporting. |
| `tests/` | Integration tests for physics invariants, numerical stability, QuadTree stress cases, and determinism. |

## Crate Structure

The crate exposes a library named `cat_simulation_engine` and a binary named
`cat-engine`. The binary is intentionally thin: it parses configuration and
delegates model behavior to the library.

Core dependencies:

- Rayon for parallel active-agent state advancement.
- Serde and Serde JSON for configuration and structured output.
- CSV for tabular output.
- Clap for the command-line interface.
- Rand and ChaCha for deterministic random sampling.
- Chrono for timestamped run archive names.
- UUID for agent identity.
- Env Logger and Log for diagnostics.

## Primary Commands

```powershell
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo run --release -- -t 10000 -n 1000 --seed 42
```

## Behavioral Contract

- Fixed seed and fixed configuration should produce deterministic outcomes.
- Agent state updates use closed-form equations, not numerical integration.
- The random number generator is isolated to main-thread phases.
- Final archive files are written under `data/runs/`.
- `RUN_MANIFEST.json` is written last and signals archive completion.

## Maintenance Rules

- Keep compiler warnings at zero.
- Run `cargo fmt --all` before committing Rust changes.
- Run `cargo check --all-targets` and `cargo test --all-targets` after any
  behavior change.
- Update `docs/architecture/` and `docs/api/` when public behavior changes.
