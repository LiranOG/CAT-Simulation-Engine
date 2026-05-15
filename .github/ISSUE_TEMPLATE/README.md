# Issue Templates

This directory contains GitHub issue templates used to collect structured input
from users and contributors. Templates reduce ambiguity and make triage faster.

## Current Templates

| File | Purpose |
| --- | --- |
| `bug_report.md` | Captures reproducible failures, expected behavior, actual behavior, environment details, and logs. |
| `feature_request.md` | Captures proposed model extensions, mathematical formulation, implementation impact, and acceptance criteria. |

## Design Principles

The issue templates are written for a research-code repository rather than a
generic application. A useful issue should identify whether the topic affects
the Rust engine, the Python analytics layer, data archive semantics, theory
documentation, or project process.

For model changes, the feature template asks for equations, thresholds, boundary
conditions, and expected impact on collapse or transcendence rates. This keeps
speculative theory work separate from implementable engineering work.

## Maintenance Rules

- Keep template prompts short but decision-oriented.
- Require enough information to reproduce bugs without requiring private data.
- Keep mathematical terminology consistent with `docs/architecture/`.
- Keep all visible text English-only and ASCII-only.
