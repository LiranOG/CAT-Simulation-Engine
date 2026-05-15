# Rust Integration Tests

This directory contains integration tests for the Rust simulation crate. These
tests use only the public library API and therefore verify behavior as a
downstream user would observe it.

## Contents

| File | Purpose |
| --- | --- |
| `physics_tests.rs` | Cross-module tests for mathematical invariants, numerical stability, QuadTree stress behavior, corrupt-state handling, and deterministic simulation outcomes. |

## Current Test Classes

- Zero energy growth remains constant.
- Tribalism decay clamps at zero and never becomes negative.
- Tribalism never exceeds its initial value.
- Maximum-growth long runs remain finite under exponent caps.
- NaN state is neutralized rather than propagated.
- Dense QuadTree clusters do not exceed maximum depth or break range queries.
- Fixed seed and fixed configuration produce deterministic collapse counts.

## Running Tests

```powershell
cd engine_rust
cargo test --all-targets
```

To run only integration tests:

```powershell
cargo test --test physics_tests
```

## Maintenance Rules

- Add tests for every model equation change.
- Prefer analytical expected values over snapshot-only assertions.
- Include boundary cases such as zero growth, extreme decay, high density, and
  corrupt numeric input.
- Keep tests deterministic and independent of filesystem output unless the test
  explicitly targets exporting.
