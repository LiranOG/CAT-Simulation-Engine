# Run Archives

This directory receives timestamped simulation archives produced by the Rust
engine. The directory itself is documented, but generated run contents are
ignored by Git.

## Naming Convention

Each run directory is named:

```text
YYYY-MM-DD_HHMMSS_seed<seed>_n<agents>
```

The timestamp records archive creation time. The seed and initial agent count
make common experiment parameters visible without opening the configuration
file.

## Completion Semantics

The exporter writes `RUN_MANIFEST.json` as the final operation of a successful
run. A dashboard or script should load a directory only if this manifest exists.
The manifest includes engine version, total ticks, total collapse events, total
transcensions, completion time, and file inventory.

## Expected Files

| File | Role |
| --- | --- |
| `simulation_config.json` | Full run configuration and resolved output paths. |
| `tick_history.json` / `tick_history.csv` | Per-tick aggregate metrics. |
| `collapse_log.json` / `collapse_log.csv` | Collapse event records. |
| `final_agents.csv` | Final state of every agent. |
| `snapshot_tick_XXXXXX.json` | Periodic active-agent state snapshots. |
| `RUN_MANIFEST.json` | Completion marker and archive inventory. |

## Maintenance Rules

- Do not edit generated files manually.
- Do not commit generated run directories.
- Preserve the full directory when moving data for analysis.
- If a run is interrupted and lacks `RUN_MANIFEST.json`, treat it as partial.
