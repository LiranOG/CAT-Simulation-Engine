# API Documentation

This directory documents the external and internal interfaces of the CAT
Simulation Engine.

## Contents

| File | Purpose |
| --- | --- |
| `API_Reference.md` | Command-line options, JSON configuration shape, Rust library modules, Python analytics entry points, and output file contracts. |

## Scope

The API documentation is responsible for interface stability. It should describe
what users and downstream tools can rely on:

- CLI flags and defaults.
- Configuration JSON fields.
- Public Rust modules and methods.
- Python analytics entry points.
- CSV and JSON output schemas.
- Archive completion semantics.

## Maintenance Rules

- Update `API_Reference.md` whenever CLI flags, config fields, public Rust
  methods, Python commands, or output schemas change.
- Keep examples executable from the repository root unless a section explicitly
  states another working directory.
- Keep behavior descriptions synchronized with source code, not assumptions.
