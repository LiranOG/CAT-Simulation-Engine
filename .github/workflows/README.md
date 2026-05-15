# Continuous Integration Workflows

This directory contains GitHub Actions workflows for validating the repository.
The workflows are the remote enforcement layer for the same checks developers
are expected to run locally.

## Workflows

| File | Scope | Primary Checks |
| --- | --- | --- |
| `rust_ci.yml` | `engine_rust/**` | `cargo fmt`, `cargo clippy`, release build, tests, and dependency audit. |
| `python_ci.yml` | `analytics_python/**` | Black formatting check, isort import-order check, Flake8 linting, and pytest. |

## Rust Workflow

The Rust workflow is split into compile/lint, test, and audit jobs. This keeps
mathematical correctness and supply-chain checks visible as separate failure
classes. The workflow sets `RUSTFLAGS="-D warnings"` so warnings are treated as
build failures in CI.

## Python Workflow

The Python workflow validates formatting and static linting for the analytics
layer. The analytics code is mostly visualization-oriented, so the test job
allows repositories with no Python tests to pass while still surfacing pytest
output when tests are added.

## Maintenance Rules

- Keep workflow commands synchronized with `README.md`, `docs/guides/INSTALL.md`,
  and `.github/CONTRIBUTING.md`.
- Keep path filters narrow so unrelated documentation-only changes do not run
  unnecessary language jobs.
- Update README badges whenever workflow filenames change.
