# Operational Guides

This directory contains procedural documentation for installing, running, and
validating the repository.

## Contents

| File | Purpose |
| --- | --- |
| `INSTALL.md` | Operating-system-specific setup commands for Rust, Cargo, Python, Streamlit, Plotly, Pandas, troubleshooting, and validation. |

## Scope

Guides are task-oriented. They should help a user reproduce a working
environment rather than explain the theory in depth.

Current guide coverage:

- Windows PowerShell setup.
- macOS setup.
- Linux setup.
- Rust build and test commands.
- Python virtual environment setup.
- Dashboard launch.
- Streamlit port conflicts.
- Path issues caused by directories with spaces.
- Borrow checker troubleshooting guidance.

## Maintenance Rules

- Keep commands copy-pasteable.
- Prefer explicit working directories.
- Document known benign warnings when they appear in normal validation output.
- Keep installation guidance synchronized with `analytics_python/requirements.txt`
  and `engine_rust/Cargo.toml`.
