# Data Directory

This directory is the default storage root for CAT simulation output. It is not
source code and should not contain hand-authored research prose other than this
README and directory markers.

## Structure

```text
data/
|-- README.md
|-- .gitkeep
`-- runs/
    `-- README.md
```

Runtime archives are written under `data/runs/`. Generated run contents are
ignored by Git because they can be large, machine-specific, and reproducible
from configuration plus seed.

## Archive Contract

The Rust exporter creates run directories with this form:

```text
runs/YYYY-MM-DD_HHMMSS_seed<seed>_n<agents>/
```

Each completed archive should contain:

```text
simulation_config.json
tick_history.json
tick_history.csv
collapse_log.json
collapse_log.csv
final_agents.csv
RUN_MANIFEST.json
snapshot_tick_XXXXXX.json
```

`RUN_MANIFEST.json` is written last and is the authoritative completion signal.
Python analytics tools should treat directories without a manifest as
incomplete.

## Maintenance Rules

- Do not commit generated simulation outputs.
- Keep `data/runs/README.md` committed so the archive location is documented.
- Move large experimental outputs outside the repository when sharing or
  publishing datasets.
- Preserve configuration files inside each run archive for reproducibility.
