# API Reference

## Rust Engine CLI

### Binary: `cat-engine`

```
USAGE:
    cat-engine [OPTIONS]

OPTIONS:
    -t, --ticks <TICKS>                   Total simulation ticks [default: 10000]
    -n, --agents <AGENTS>                 Initial civilization count [default: 1000]
    -s, --spawn-rate <RATE>               Per-tick spawn probability [default: 0.5]
        --seed <SEED>                     RNG seed for reproducibility [default: 42]
        --critical-energy <E>             E_crit threshold [default: 2.5]
        --survival-tribalism <T>          T_surv threshold [default: 0.6]
        --hive-collectivism <C>           C_hive threshold [default: 0.85]
        --grid-width <W>                  Simulation space width [default: 1000.0]
        --grid-height <H>                 Simulation space height [default: 1000.0]
        --snapshot-interval <N>           Ticks between snapshots [default: 100]
    -o, --output-dir <DIR>                Data output directory [default: ../data]
        --threads <N>                     Thread count (0=auto) [default: 0]
    -c, --config-file <PATH>              Load config from JSON file
    -h, --help                            Print help
    -V, --version                         Print version
```

### Examples

```bash
# Quick test run
cargo run --release -- -t 1000 -n 100 -o ./data

# Full-scale simulation with custom thresholds
cargo run --release -- \
  -t 50000 -n 10000 \
  --critical-energy 3.0 \
  --survival-tribalism 0.7 \
  --hive-collectivism 0.9 \
  --seed 12345 \
  -o ./data/run_01

# Load from config file
cargo run --release -- -c config.json
```

### Configuration File Schema

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
    "exogenous_extinction_rate": 1e-6,
    "resource_ceiling": 5.0
  },
  "grid_config": {
    "width": 1000.0,
    "height": 1000.0,
    "max_agents_per_node": 8,
    "max_depth": 12
  },
  "snapshot_interval": 100,
  "output_dir": "../data",
  "num_threads": 0
}
```

---

## Rust Library API

### `agent.rs`

#### `struct Agent`

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique civilization identifier |
| `state` | `AgentState` | Lifecycle state (Nascent/Evolving/Transcended/Collapsed) |
| `position` | `(f64, f64)` | Grid coordinates |
| `energy` | `f64` | Kardashev level (E state vector) |
| `tribalism` | `f64` | Aggression coefficient (T state vector) |
| `collectivism` | `f64` | Hive-mind index (C state vector), range [0,1] |
| `energy_growth_rate` | `f64` | Exponential growth rate `r` |
| `tribalism_decay_alpha` | `f64` | Logarithmic decay coefficient `α` |
| `collectivism_drift` | `f64` | Linear drift rate `δ` |
| `ticks_since_ignition` | `u64` | Internal clock |
| `birth_tick` | `u64` | Spawn tick |
| `influence_radius` | `f64` | Spatial reach = `√E · 0.1` |

#### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(position, E, T, C, r, α, δ, tick) -> Agent` | Construct new agent |
| `tick` | `(&mut self, &CollapseThresholds)` | Advance state vectors by one step |
| `evaluate_collapse` | `(&self, tick, &CollapseThresholds) -> Option<CollapseEvent>` | Check collapse conditions |
| `evaluate_transcendence` | `(&self, &CollapseThresholds) -> bool` | Check hive-mind transcendence |
| `destroy` | `(&mut self)` | Execute collapse |
| `transcend` | `(&mut self)` | Mark as transcended |
| `is_active` | `(&self) -> bool` | Check if still participating |
| `asynchronous_gap` | `(&self) -> f64` | Compute E/(1-T+ε) metric |

#### `struct CollapseThresholds`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `critical_energy` | `f64` | `2.5` | E threshold for collapse eligibility |
| `survival_tribalism` | `f64` | `0.6` | T threshold for collapse eligibility |
| `hive_collectivism` | `f64` | `0.85` | C threshold for transcendence |
| `exogenous_extinction_rate` | `f64` | `1e-6` | Per-tick exogenous death rate |
| `resource_ceiling` | `f64` | `5.0` | Max E without interstellar expansion |

### `grid.rs`

#### `struct QuadTree`

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(&GridConfig) -> QuadTree` | Create empty tree |
| `rebuild` | `(&mut self, &[Agent])` | Reconstruct from agent positions |
| `query_range` | `(&self, &BoundingBox) -> Vec<Uuid>` | Find agents in AABB |
| `query_radius` | `(&self, cx, cy, r) -> Vec<Uuid>` | Find agents in circle |
| `total_agents` | `(&self) -> usize` | Active agent count |
| `depth_stats` | `(&self) -> (u32, u32, usize)` | (min_depth, max_depth, leaf_count) |

### `simulation.rs`

#### `struct Simulation`

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(SimulationConfig) -> Simulation` | Initialize with config |
| `run` | `(&mut self)` | Execute full simulation |

### `exporter.rs`

#### `struct Exporter`

| Method | Description |
|--------|-------------|
| `export_config` | Write `simulation_config.json` |
| `export_tick_snapshot` | Write per-tick agent snapshot |
| `export_collapse_log` | Write collapse events (JSON + CSV) |
| `export_tick_history` | Write aggregate statistics (JSON + CSV) |
| `export_final_agents` | Write final agent states (CSV) |

---

## Python Analytics API

### `dashboard.py`

Launch: `streamlit run dashboard.py`

**Pages:**
1. **Overview** — Key metrics, population dynamics chart
2. **State Vectors** — Mean E, T, C evolution over time
3. **Collapse Analysis** — Type distribution, E-T scatter, timing histogram
4. **Spatial Distribution** — Final positions, collapse density heatmap
5. **Asynchronous Gap** — Theoretical curves, simulated gap metric
6. **Configuration** — Raw config JSON, threshold display

### `logic_plotter.py`

```bash
python logic_plotter.py --data-dir ../data --output-dir ../data/figures
```

**Generated Figures:**
- `asynchronous_gap_theory.png` — Theoretical E vs ln curves
- `population_dynamics.png` — Active/collapsed/transcended over time
- `state_vector_evolution.png` — 3-panel E, T, C evolution
- `collapse_scatter.png` — E-T state at collapse, colored by type
- `collapse_histogram.png` — Temporal distribution of collapses
- `spatial_distribution.png` — Final spatial positions by state

---

## Output File Formats

### CSV: `collapse_log.csv`

```
agent_id,tick,energy,tribalism,collectivism,pos_x,pos_y,collapse_type
550e8400-...,4237,3.14,0.72,0.27,423.7,618.2,AsynchronousGap
```

### CSV: `tick_history.csv`

```
tick,active_agents,nascent_count,evolving_count,transcended_count,collapsed_count,...
0,1000,1000,0,0,0,...
100,987,423,564,3,10,...
```

### CSV: `final_agents.csv`

```
id,state,pos_x,pos_y,energy,tribalism,collectivism,...,asynchronous_gap
550e8400-...,Collapsed,423.7,618.2,3.14,0.72,0.27,...,11.21
```
