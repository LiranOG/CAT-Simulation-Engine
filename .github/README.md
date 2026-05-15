# GitHub Project Operations

This directory contains repository governance, issue intake, pull request
review structure, and continuous integration configuration. It is intentionally
separate from the root so the root remains a clean project entry point while
GitHub-specific workflows remain discoverable to contributors.

## Contents

| Path | Purpose |
| --- | --- |
| `CONTRIBUTING.md` | Contributor workflow, branch naming, commit conventions, formatting rules, testing standards, and documentation requirements. |
| `CODE_OF_CONDUCT.md` | Behavioral standards and scientific integrity expectations for contributors. |
| `SECURITY.md` | Security model, supported versions, threat categories, and responsible disclosure process. |
| `PULL_REQUEST_TEMPLATE.md` | Required structure for pull requests, including mathematical validity checks. |
| `ISSUE_TEMPLATE/` | Templates for bug reports and feature requests. |
| `workflows/` | GitHub Actions CI definitions for Rust and Python surfaces. |

## Operational Role

The files in this directory define how changes enter the project. CAT is a
scientific simulation repository, so process quality matters: a minor change to
state-vector dynamics can change all downstream research conclusions. The
governance files therefore require explicit documentation, reproducibility
checks, and mathematical validation whenever behavior changes.

## Maintenance Rules

- Keep process documents in English and ASCII-only text.
- Keep CI names aligned with README badges.
- Update `PULL_REQUEST_TEMPLATE.md` whenever model-change review requirements
  change.
- Update workflow files when the Rust or Python validation commands change.
- Do not place general research documentation here; use `docs/` instead.
