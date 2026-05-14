# Contributing to CAT Simulation Engine

Thank you for your interest in contributing to the Cosmobiological Asynchrony Theory simulation engine. This project operates at the intersection of computational astrophysics, agent-based modeling, and high-performance systems engineering. Contributions are held to the standards of rigorous scientific software.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Branch Naming Conventions](#branch-naming-conventions)
- [Commit Message Standards](#commit-message-standards)
- [Code Formatting Mandates](#code-formatting-mandates)
- [Pull Request Process](#pull-request-process)
- [Testing Requirements](#testing-requirements)
- [Documentation Standards](#documentation-standards)
- [Issue Guidelines](#issue-guidelines)

## Code of Conduct

All contributors must adhere to our [Code of Conduct](CODE_OF_CONDUCT.md). This is a scientific project; discourse must be precise, evidence-based, and respectful.

## Getting Started

1. **Fork** the repository on GitHub.
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/CAT-Simulation-Engine.git
   cd CAT-Simulation-Engine
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/cat-research/CAT-Simulation-Engine.git
   ```
4. **Create a feature branch** from `develop`:
   ```bash
   git checkout develop
   git pull upstream develop
   git checkout -b feat/your-feature-name
   ```

## Development Environment

### Rust Engine

- **Rust** ≥ 1.78 (stable channel)
- Install via [rustup](https://rustup.rs/)
- Required components: `rustfmt`, `clippy`
  ```bash
  rustup component add rustfmt clippy
  ```
- Build: `cd engine_rust && cargo build`
- Test: `cargo test --all-targets`

### Python Analytics

- **Python** ≥ 3.12
- Create a virtual environment:
  ```bash
  cd analytics_python
  python -m venv .venv
  source .venv/bin/activate  # or .venv\Scripts\activate on Windows
  pip install -r requirements.txt
  pip install flake8 black isort pytest
  ```

## Branch Naming Conventions

All branches must follow this naming scheme:

| Prefix | Purpose | Example |
|--------|---------|---------|
| `feat/` | New feature or model extension | `feat/inter-agent-communication` |
| `fix/` | Bug fix | `fix/collapse-threshold-boundary` |
| `refactor/` | Code restructuring (no behavior change) | `refactor/quadtree-memory-layout` |
| `docs/` | Documentation only | `docs/api-reference-update` |
| `perf/` | Performance optimization | `perf/rayon-chunk-sizing` |
| `ci/` | CI/CD pipeline changes | `ci/add-cargo-audit` |
| `test/` | Test additions or fixes | `test/agent-edge-cases` |

Branch names must be lowercase, use hyphens for word separation, and be descriptive of the change.

## Commit Message Standards

This project uses **Conventional Commits** (v1.0.0). Every commit message must follow this format:

```
<type>(<scope>): <subject>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `docs` | Documentation only changes |
| `test` | Adding or correcting tests |
| `perf` | Performance improvement |
| `ci` | CI/CD configuration changes |
| `chore` | Build process, dependency updates |
| `style` | Formatting, semicolons, etc. (no code change) |

### Scopes

| Scope | Component |
|-------|-----------|
| `agent` | Agent module (`agent.rs`) |
| `sim` | Simulation engine (`simulation.rs`) |
| `grid` | Spatial indexing (`grid.rs`) |
| `export` | Data export (`exporter.rs`) |
| `cli` | CLI interface (`main.rs`) |
| `dash` | Streamlit dashboard |
| `plot` | Logic plotter |
| `docs` | Documentation |
| `ci` | CI/CD workflows |

### Examples

```
feat(agent): implement resource depletion collapse pathway

Add a secondary collapse condition triggered when E reaches the resource
ceiling and T prevents interstellar expansion. This models civilizations
that exhaust their home system's energy budget.

Closes #42
```

```
fix(grid): correct quadtree boundary inclusion for edge agents

Agents at exact quadrant boundaries were being excluded from range
queries due to strict inequality. Changed to inclusive bounds.
```

```
perf(sim): batch collapse evaluation with parallel prefix scan

Reduces Phase 3 latency by 40% for >100k agents by parallelizing
the collapse predicate evaluation using a two-phase commit pattern.

Benchmark: 100k agents, 10k ticks: 14.2s → 8.5s
```

## Code Formatting Mandates

### Rust

All Rust code **must** pass `rustfmt` and `clippy` before submission:

```bash
cd engine_rust
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

Configuration is via the default `rustfmt` settings. Do not add a custom `rustfmt.toml` without team consensus.

### Python

All Python code **must** pass `black`, `isort`, and `flake8`:

```bash
cd analytics_python
black .
isort .
flake8 . --max-line-length 100
```

Settings:
- **Black**: Default configuration, line length 100
- **isort**: Compatible with Black (`profile = "black"`)
- **Flake8**: Max line length 100, no additional plugins required

## Pull Request Process

1. **Ensure all checks pass locally** before pushing:
   ```bash
   # Rust
   cd engine_rust && cargo fmt --all -- --check && cargo clippy -- -D warnings && cargo test

   # Python
   cd analytics_python && black --check . && isort --check-only . && flake8 . --max-line-length 100
   ```

2. **Push your branch** and open a PR against `develop` (not `main`).

3. **Fill out the PR template completely.** Incomplete PRs will be closed without review.

4. **Respond to review feedback** within 7 days. Stale PRs will be closed.

5. **Squash and merge** is the default merge strategy. Each PR should result in a single, clean commit on `develop`.

### PR Requirements

- [ ] All CI checks pass
- [ ] At least one approving review from a maintainer
- [ ] No unresolved conversations
- [ ] Documentation updated (if applicable)
- [ ] Commit messages follow Conventional Commits
- [ ] No `TODO`, `FIXME`, or placeholder comments

### Mathematical Validity (for model changes)

PRs that modify agent dynamics, collapse functions, or state vector equations require:

- Formal specification of the modified equations in the PR description
- Unit tests verifying correctness against known analytic results
- Edge case testing (E → ∞, T → 0, C → 1, t = 0)
- Updated documentation in `docs/CAT_Architecture.md`

## Testing Requirements

### Rust

- All public functions must have unit tests.
- Integration tests for the simulation pipeline belong in `engine_rust/tests/`.
- Test coverage should not decrease with any PR.
- Use deterministic seeds for reproducible test outcomes.

```bash
cargo test --all-targets --all-features -- --nocapture
```

### Python

- Dashboard components are tested via visual inspection (Streamlit limitation).
- Plotting utilities should have snapshot tests where feasible.
- Data loading functions must handle missing/malformed input gracefully.

```bash
pytest --tb=short -q
```

## Documentation Standards

- All public Rust functions and structs must have `///` doc comments.
- Comments should be precise and academically rigorous. Cynicism is permitted; vagueness is not.
- Markdown documentation must be free of broken links and rendering errors.
- Architecture changes must be reflected in `docs/CAT_Architecture.md`.
- API changes must be reflected in `docs/API_Reference.md`.

## Issue Guidelines

- Use the provided issue templates ([Bug Report](.github/ISSUE_TEMPLATE/bug_report.md), [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md)).
- Include reproduction steps for bugs and mathematical justification for feature requests.
- Label issues appropriately: `bug`, `enhancement`, `documentation`, `performance`, `question`.

---

Thank you for contributing to our understanding of why the universe is so quiet. Your code may not save any civilizations, but it might help explain why they don't save themselves.
