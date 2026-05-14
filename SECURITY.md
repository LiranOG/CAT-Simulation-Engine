# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | ✅ Active security updates |
| < 1.0   | ❌ Not supported |

## Threat Model

The CAT Simulation Engine is a scientific computing tool, not a network service. Its threat surface differs from web applications but is not negligible. The following threat categories are actively considered:

### 1. Malicious Input (Configuration / Data Injection)

**Threat**: Crafted configuration JSON files or malformed data inputs designed to cause crashes, memory exhaustion, or logic errors.

**Attack Vectors**:
- Extremely large `initial_agents` or `max_ticks` values designed to exhaust system memory.
- Negative or NaN values in threshold parameters causing undefined mathematical behavior.
- Malformed JSON in `--config-file` input triggering parser vulnerabilities.
- Path traversal in `--output-dir` directing writes to sensitive filesystem locations.

**Mitigations**:
- All numeric inputs are validated at parse time via `clap` type constraints.
- `serde` deserialization rejects structurally invalid JSON.
- File I/O operations use `std::path::Path` with no shell expansion or interpolation.
- Energy, tribalism, and collectivism values are clamped to mathematically valid ranges within the simulation loop.
- Resource ceiling and growth rate parameters are validated at configuration load time.

### 2. Denial of Service (Computational Exhaustion)

**Threat**: Configuration parameters designed to consume excessive CPU, memory, or disk I/O.

**Attack Vectors**:
- `max_ticks = u64::MAX` with high agent counts causing indefinite computation.
- `spawn_rate = 1.0` with no collapse thresholds causing unbounded agent population growth.
- `snapshot_interval = 1` with millions of agents causing disk I/O saturation.
- QuadTree depth exceeding stack limits on coincident agents.

**Mitigations**:
- QuadTree maximum depth is capped at 12 (configurable but bounded).
- Snapshot export is logged with timing information for monitoring.
- Memory allocation for agent vectors uses `Vec::with_capacity` with explicit initial sizing.
- Users running large-scale simulations are expected to provision appropriate hardware (see README performance table).
- CI runs include bounded test configurations to prevent resource exhaustion in automated environments.

### 3. Supply Chain (Dependency Vulnerabilities)

**Threat**: Compromised or vulnerable third-party crates/packages introducing security flaws.

**Attack Vectors**:
- Dependency confusion attacks via Cargo registry.
- Known vulnerabilities in transitive dependencies.
- Malicious updates to pinned dependency versions.

**Mitigations**:
- **`cargo audit`** is executed in CI on every push and PR (see `.github/workflows/rust_ci.yml`).
- Dependencies are pinned to specific semver-compatible ranges in `Cargo.toml`.
- `Cargo.lock` is committed to version control for reproducible builds.
- Python dependencies are version-pinned in `requirements.txt`.
- Regular manual review of dependency updates is performed before merging version bumps.

**Recommended periodic audit commands**:
```bash
# Rust dependency audit
cd engine_rust
cargo audit

# Check for outdated dependencies
cargo outdated

# Python dependency audit
cd analytics_python
pip-audit -r requirements.txt
```

### 4. Malicious Pull Requests

**Threat**: PRs introducing backdoors, logic bombs, or subtle mathematical errors that corrupt simulation results.

**Attack Vectors**:
- Modified collapse thresholds that silently alter simulation outcomes.
- Introduced race conditions in Rayon parallel sections.
- Backdoor data exfiltration via the exporter module.
- Subtle mathematical errors (e.g., changing `exp` to `exp2`) that pass cursory review.

**Mitigations**:
- All PRs require at least one maintainer review before merge.
- CI enforces `cargo clippy -- -D warnings` to catch common logic errors.
- Unit tests cover all collapse conditions, transcendence predicates, and mathematical functions.
- The PR template includes a "Mathematical Validity" checklist for model-changing PRs.
- Squash-and-merge policy ensures each PR corresponds to a single, auditable commit.
- Binary artifacts are never committed; all code is human-readable source.

### 5. Data Integrity (Simulation Output)

**Threat**: Corruption, tampering, or misrepresentation of simulation output data.

**Attack Vectors**:
- Filesystem-level corruption of JSON/CSV output during large writes.
- Concurrent write access to the output directory from multiple simulation instances.
- Silent numerical overflow producing incorrect but plausible results.

**Mitigations**:
- All numeric state vectors use `f64` (IEEE 754 double precision) with explicit overflow checks.
- The `asynchronous_gap` metric uses an epsilon guard (`1e-10`) to prevent division by zero.
- JSON output uses `serde_json::to_string_pretty` for human-readable verification.
- Configuration is exported alongside results for full reproducibility.
- Users are advised to use unique output directories (`-o ./data/run_N`) for concurrent runs.

## Vulnerability Reporting

### How to Report

If you discover a security vulnerability in this project, please report it responsibly:

1. **DO NOT** open a public GitHub issue for security vulnerabilities.
2. **Email**: Send a detailed report to **cat-security@proton.me**.
3. **Include**:
   - Description of the vulnerability.
   - Steps to reproduce.
   - Potential impact assessment.
   - Suggested remediation (if known).
   - Your name/handle for credit (optional).

### Response Timeline

| Stage | Timeline |
|-------|----------|
| Acknowledgment of receipt | Within 48 hours |
| Initial assessment | Within 7 days |
| Patch development | Within 30 days (critical), 90 days (moderate) |
| Public disclosure | After patch release, coordinated with reporter |

### Severity Classification

| Severity | Criteria | Response |
|----------|----------|----------|
| **Critical** | Remote code execution, arbitrary file write, data exfiltration | Immediate patch, advisory within 48 hours |
| **High** | Denial of service, memory corruption, silent result corruption | Patch within 7 days |
| **Medium** | Information disclosure, logic errors with limited impact | Patch within 30 days |
| **Low** | Minor issues, hardening improvements | Addressed in next release cycle |

## Security Best Practices for Users

1. **Pin your dependencies**: Use `Cargo.lock` and pinned `requirements.txt` versions.
2. **Run `cargo audit` regularly**: Especially before publishing results from simulation runs.
3. **Validate output**: Cross-check simulation results against known analytic limits (e.g., expected collapse rates for given thresholds).
4. **Isolate large runs**: Run resource-intensive simulations in containerized or sandboxed environments.
5. **Review configuration**: Before running third-party configuration files, inspect them for anomalous parameter values.

## Acknowledgments

We gratefully acknowledge security researchers who responsibly disclose vulnerabilities. Contributors who report valid security issues will be credited in the project's security acknowledgments (with their consent).
