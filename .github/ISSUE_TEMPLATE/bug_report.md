---
name: Bug Report
about: Report a defect in the simulation engine, analytics dashboard, or data pipeline
title: "[BUG] "
labels: bug, triage
assignees: ''
---

## Environment

- **OS**: [e.g., Ubuntu 24.04, macOS 15, Windows 11]
- **Rust Version**: [output of `rustc --version`]
- **Python Version**: [output of `python --version`]
- **CAT Engine Version**: [output of `cat-engine --version`]
- **Hardware**: [CPU cores, RAM, GPU if applicable]

## Description

A clear and precise description of the bug. Include the expected behavior and the actual observed behavior.

## Steps to Reproduce

1. Configure the simulation with parameters: `...`
2. Execute: `cargo run --release -- -t 10000 -n 5000`
3. Observe: `...`

## Expected Behavior

Describe what should have happened according to the CAT mathematical model.

## Actual Behavior

Describe what actually happened. Include error messages, incorrect output values, or unexpected collapse/transcendence rates.

## Relevant Data

If applicable, attach:
- Simulation configuration JSON (`simulation_config.json`)
- Relevant log output (set `RUST_LOG=debug`)
- Screenshots of dashboard anomalies
- Collapse log excerpts showing the erroneous behavior

## Severity Assessment

- [ ] **Critical**: Simulation produces mathematically incorrect results (wrong collapse dynamics, energy function errors)
- [ ] **High**: Engine crash, data corruption, or CI pipeline failure
- [ ] **Medium**: Performance degradation, incorrect visualization, non-blocking errors
- [ ] **Low**: Cosmetic issues, documentation errors, minor UX inconveniences

## Additional Context

Any additional context, related issues, or theoretical implications of the bug.
