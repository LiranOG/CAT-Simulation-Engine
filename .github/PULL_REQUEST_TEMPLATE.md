## Pull Request

### Type

- [ ] **Feature**: New simulation capability, model extension, or analytical tool
- [ ] **Bugfix**: Correction to existing logic, data pipeline, or visualization
- [ ] **Refactor**: Internal restructuring without behavioral change
- [ ] **Documentation**: Updates to docs, comments, or README
- [ ] **CI/CD**: Pipeline configuration, dependency updates, or build system changes
- [ ] **Performance**: Optimization targeting throughput, memory, or latency

### Description

Provide a concise summary of the changes. Reference the related issue(s) using `Closes #<number>`.

**What**: Describe what was changed.
**Why**: Explain the motivation — why is this change necessary?
**How**: Briefly describe the implementation approach.

### Mathematical Validity (if applicable)

If this PR modifies agent dynamics, collapse functions, or state vector equations:

- [ ] The modified equations are documented in code comments
- [ ] Unit tests verify the mathematical correctness against known analytic results
- [ ] Edge cases (E → ∞, T → 0, C → 1) have been tested
- [ ] The Asynchronous Gap collapse condition `(E > E_crit) ∧ (T > T_surv) ∧ (C < C_hive)` is preserved

### Checklist

#### Code Quality
- [ ] Code compiles without warnings (`cargo build --release` / `python -c "import dashboard"`)
- [ ] All existing tests pass (`cargo test` / `pytest`)
- [ ] New tests added for new functionality
- [ ] No `TODO`, `FIXME`, or placeholder comments remain
- [ ] Comments are precise, cynical, and academically rigorous

#### Formatting
- [ ] Rust: `cargo fmt --all` applied
- [ ] Python: `black .` and `isort .` applied
- [ ] Linting: `cargo clippy -- -D warnings` and `flake8 --max-line-length 100` pass

#### Documentation
- [ ] Public API changes reflected in `docs/API_Reference.md`
- [ ] Architecture changes reflected in `docs/CAT_Architecture.md`
- [ ] README updated if user-facing behavior changed

#### Performance (if applicable)
- [ ] Benchmarked against baseline (N agents, M ticks)
- [ ] No regression in simulation throughput
- [ ] Memory profile reviewed for large-scale runs

### Screenshots / Visualizations

If the PR affects the dashboard or analytical output, include before/after screenshots or figures.

### Breaking Changes

List any breaking changes to CLI arguments, configuration JSON schema, or output file formats.
