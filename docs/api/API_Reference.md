# API Reference

This reference describes the command-line interface, Rust library surface,
Python analytics entry points, and output file contracts for CAT Simulation
Engine v1.0.0.

## Rust CLI

Binary name:

```text
cat-engine
```

Run from source:

```powershell
cd engine_rust
cargo run --release -- [OPTIONS]
```

Options:

| Option | Default | Description |
| --- | --- | --- |
| `-t, --ticks <TICKS>` | `10000` | Number of simulation ticks. |
| `-n, --agents <AGENTS>` | `1000` | Initial civilization count. |
| `-s, --spawn-rate <RATE>` | `0.5` | Per-tick probability of one spontaneous spawn. |
| `--seed <SEED>` | `42` | Deterministic random seed. |
| `--critical-energy <E>` | `2.5` | Energy threshold for collapse risk. |
| `--survival-tribalism <T>` | `0.6` | Tribalism threshold for collapse risk. |
| `--hive-collectivism <C>` | `0.85` | Collectivism threshold for transcendence. |
| `--grid-width <W>` | `1000.0` | Simulation-space width. |
| `--grid-height <H>` | `1000.0` | Simulation-space height. |
| `--snapshot-interval <N>` | `100` | Tick interval between active-agent snapshots. |
| `-o, --base-data-dir <DIR>` | `../data` | Base directory where `runs/` is created. |
| `--threads <N>` | `0` | Rayon thread count; `0` uses Rayon default. |
| `-c, --config-file <PATH>` | none | Load full JSON configuration. |

Examples:

```powershell
cargo run --release -- -t 1000 -n 100 --seed 7
```

```powershell
cargo run --release -- `
  -t 50000 `
  -n 10000 `
  --critical-energy 3.0 `
  --survival-tribalism 0.7 `
  --hive-collectivism 0.9 `
  --seed 12345 `
  -o ../data
```

```powershell
cargo run --release -- -c ../configs/experiment.json
```

## Configuration JSON

The CLI accepts a full serialized `SimulationConfig`. When a config file is
provided, it overrides individual CLI parameters.

```json
{
  "max_ticks": 10000,
  "initial_agents": 1000,
  "spawn_rate": 0.5,
  "seed": 42,
  "thresholds": {
    "critical_energy": 2.5,
    "survival_tribalism": 0.6,
    "hive_collectivism": 0.85,
    "exogenous_extinction_rate": 0.000001,
    "resource_ceiling": 5.0
  },
  "grid_config": {
    "width": 1000.0,
    "height": 1000.0,
    "max_agents_per_node": 8,
    "max_depth": 12
  },
  "snapshot_interval": 100,
  "base_data_dir": "../data",
  "output_dir": "",
  "num_threads": 0
}
```

`output_dir` is normally empty in input files. `main.rs` resolves it to a new
timestamped run directory before starting the simulation.

## Rust Library Surface

The library crate is named `cat_simulation_engine`.

```rust
pub mod agent;
pub mod exporter;
pub mod grid;
pub mod simulation;
```

### `agent`

Primary types:

| Type | Purpose |
| --- | --- |
| `Agent` | Civilization state vector and lifecycle. |
| `AgentState` | `Nascent`, `Evolving`, `Transcended`, or `Collapsed`. |
| `CollapseEvent` | Recorded failure event. |
| `CollapseType` | `AsynchronousGap`, `ResourceDepletion`, or `ExogenousExtinction`. |
| `CollapseThresholds` | Threshold parameters for collapse and transcendence. |

Important methods:

| Method | Contract |
| --- | --- |
| `Agent::new(...) -> Agent` | Constructs an agent from explicit initial state vectors and growth rates. |
| `state_vectors_valid(&self) -> bool` | Checks finite numeric domains and bounded state variables. |
| `tick(&mut self, &CollapseThresholds)` | Advances `E`, `T`, and `C` by closed-form equations. |
| `influence_radius(&self) -> f64` | Computes `sqrt(E) * 0.1`. |
| `evaluate_collapse(&self, tick, thresholds) -> Option<CollapseEvent>` | Evaluates collapse predicates without mutating the agent. |
| `evaluate_transcendence(&self, thresholds) -> bool` | Evaluates the high-collectivism survival predicate. |
| `destroy(&mut self)` | Marks the agent collapsed. |
| `transcend(&mut self)` | Marks the agent transcended. |
| `is_active(&self) -> bool` | Returns true for `Nascent` or `Evolving`. |
| `asynchronous_gap(&self) -> f64` | Computes `E / (1 - T + 1e-10)`. |

### `grid`

Primary types:

| Type | Purpose |
| --- | --- |
| `GridConfig` | Width, height, leaf capacity, and maximum depth. |
| `BoundingBox` | Axis-aligned rectangular query bounds. |
| `AgentRef` | Minimal `Uuid`, `x`, and `y` stored in the tree. |
| `QuadNode` | Leaf or internal tree node. |
| `QuadTree` | Adaptive active-agent spatial index. |

Important methods:

| Method | Contract |
| --- | --- |
| `QuadTree::new(&GridConfig) -> QuadTree` | Creates an empty index spanning the configured space. |
| `rebuild(&mut self, &[Agent])` | Rebuilds the tree from active agents. |
| `query_range(&self, &BoundingBox) -> Vec<Uuid>` | Returns agents inside an axis-aligned range. |
| `query_radius(&self, cx, cy, radius) -> Vec<Uuid>` | Returns agents inside an exact circular radius. |
| `total_agents(&self) -> usize` | Returns active agents represented in the tree. |
| `depth_stats(&self) -> (u32, u32, usize)` | Returns minimum leaf depth, maximum leaf depth, and leaf count. |

### `simulation`

Primary types:

| Type | Purpose |
| --- | --- |
| `SimulationConfig` | Complete runtime configuration. |
| `TickStats` | Per-tick aggregate metrics. |
| `Simulation` | Engine state and tick orchestration. |

Important methods:

| Method | Contract |
| --- | --- |
| `Simulation::new(config) -> Simulation` | Seeds the deterministic initial population. |
| `run(&mut self)` | Runs all ticks and exports final results. |
| `advance_tick(&mut self, tick)` | Runs one tick for integration tests. |
| `collapse_count(&self) -> usize` | Returns recorded collapse count. |

### `exporter`

Primary type:

| Type | Purpose |
| --- | --- |
| `Exporter` | JSON and CSV archive writer. |

Important methods:

| Method | Contract |
| --- | --- |
| `Exporter::new(output_dir) -> Exporter` | Creates an exporter for an existing or future run directory. |
| `create_run_directory(base, seed, agents)` | Creates `base/runs/<timestamp_seed_n>/`. |
| `ensure_output_dir(&self)` | Creates the output directory if needed. |
| `export_config(&self, &SimulationConfig)` | Writes `simulation_config.json`. |
| `export_tick_snapshot(&self, tick, &[Agent])` | Writes active-agent snapshot JSON. |
| `export_collapse_log(&self, &[CollapseEvent])` | Writes JSON and CSV collapse logs. |
| `export_tick_history(&self, &[TickStats])` | Writes JSON and CSV tick history. |
| `export_final_agents(&self, &[Agent])` | Writes final agent CSV. |
| `export_run_manifest(&self, ticks, collapses, transcensions)` | Writes final completion manifest. |

## Python Analytics

### Dashboard

Entry point:

```powershell
cd analytics_python
streamlit run dashboard.py
# If the above fails:
python -m streamlit run dashboard.py
```

The dashboard expects a base data directory containing `runs/`. It loads only
run directories with `RUN_MANIFEST.json`.

Main capabilities:

- Completed and incomplete run discovery.
- Orphaned pre-archive data warnings.
- Plotly population dynamics.
- Plotly state-vector evolution.
- Collapse analysis and collapse-type distributions.
- Spatial distribution views.
- Asynchronous Gap visualization.
- Configuration and manifest inspection.

### Static Plotter

Entry point:

```powershell
cd analytics_python
python logic_plotter.py --data-dir ../data/runs/<run_directory>
```

Generated files are written to `<data-dir>/figures` unless `--output-dir` is
provided:

```text
asynchronous_gap_theory.png
population_dynamics.png
state_vector_evolution.png
collapse_scatter.png
collapse_histogram.png
spatial_distribution.png
```

## Output Files

### `collapse_log.csv`

```text
agent_id,tick,energy,tribalism,collectivism,pos_x,pos_y,collapse_type
550e8400-e29b-41d4-a716-446655440000,4237,3.14,0.72,0.27,423.7,618.2,AsynchronousGap
```

### `tick_history.csv`

Representative columns:

```text
tick,active_agents,nascent_count,evolving_count,transcended_count,collapsed_count,collapses_this_tick,transcensions_this_tick,mean_energy,mean_tribalism,mean_collectivism,max_energy,mean_async_gap
```

### `final_agents.csv`

Representative columns:

```text
id,state,pos_x,pos_y,energy,tribalism,collectivism,initial_energy,initial_tribalism,energy_growth_rate,tribalism_decay_alpha,collectivism_drift,ticks_since_ignition,birth_tick,influence_radius,asynchronous_gap
```

### `RUN_MANIFEST.json`

The manifest is the completion signal for the archive. It includes completion
time, engine version, total ticks, total collapses, total transcensions, and a
file inventory with byte sizes.

## Validation Commands

```powershell
cd engine_rust
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
```

```powershell
python -m py_compile analytics_python\dashboard.py analytics_python\logic_plotter.py
```
