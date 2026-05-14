# CAT Architecture: Mathematical & Systems Specification

## 1. Theoretical Foundation

### 1.1 Cosmobiological Asynchrony Theory (CAT)

The Fermi Paradox — the contradiction between the high probability of extraterrestrial civilizations and the absence of evidence for them — is resolved by CAT through three interlocking mechanisms:

1. **The Optical Boundary**: Spacetime delay prevents real-time observation of advanced technology. A civilization 1,000 light-years away is observed as it was 1,000 years ago, regardless of its current state. The observable universe is a museum of the past.

2. **The Asynchronous Gap (The Great Filter)**: Technological capacity scales exponentially while biological/psychological maturity scales logarithmically. The exponential inevitably outruns the logarithm. When technology crosses a critical threshold and the civilization's psychology remains tribal/competitive, systemic collapse occurs. This is not a risk — it is a mathematical certainty for civilizations that do not achieve collective coordination before the crossover point.

3. **The Hive-Mind Anomaly**: Civilizations with high Collectivism (C ≥ C_hive) bypass the filter entirely. With no internal factions, there is no one to aim the weapons at. However, high-C civilizations lack the competitive pressure that drives exponential technological growth, resulting in lower peak energy levels and reduced detectable signatures. They survive, but silently.

### 1.2 Core Equations

#### Energy / Kardashev Level (E)
```
E(t) = E₀ · exp(r · t)
```
Where:
- `E₀` = initial energy level (pre-technological baseline, typically 0.01–0.3)
- `r` = exponential growth rate (0.005–0.05, varies by civilization)
- `t` = ticks since technological ignition

The exponential growth is resource-capped when tribalism prevents interstellar expansion:
```
E_effective(t) = min(E(t), E_ceiling)  if T > T_survival
                 E(t)                   otherwise
```

#### Tribalism / Aggression (T)
```
T(t) = T₀ · max(0, 1 - α · ln(1 + t))
```
Where:
- `T₀` = initial tribalism coefficient (typically 0.6–0.95)
- `α` = logarithmic decay rate (0.001–0.015)

This formulation encodes the central tragedy: biological evolution operates on timescales of millions of years; technological evolution operates on timescales of decades. The prefrontal cortex is, cosmologically speaking, a prototype.

#### Collectivism / Hive-Mind Index (C)
```
C(t+1) = clamp(C(t) + δ, 0, 1)
```
Where:
- `δ` = collectivism drift rate (typically -0.001 to +0.003)

Linear drift with hard bounds at [0, 1]. Most civilizations drift slightly upward. It is rarely enough.

#### Influence Radius
```
R(t) = √E(t) · κ
```
Where `κ = 0.1` is the influence coefficient. Spatial reach scales with the square root of energy output. Even at Type II, the lightcone constrains communication.

#### Asynchronous Gap Metric
```
G(t) = E(t) / (1 - T(t) + ε)
```
Where `ε = 10⁻¹⁰`. This ratio quantifies the disparity between technological capacity and psychological readiness. High G indicates existential danger.

### 1.3 Collapse Conditions

A civilization collapses when ALL of the following hold simultaneously:

| Condition | Expression | Interpretation |
|-----------|-----------|----------------|
| **Technological Danger** | `E > E_critical` | Technology is existentially weaponizable |
| **Psychological Immaturity** | `T > T_survival` | Internal conflict remains dominant |
| **Insufficient Coordination** | `C < C_hive` | No species-wide cooperation mechanism |

Default thresholds:
- `E_critical = 2.5` (mid-Type-I Kardashev, ~10¹⁶ W normalized)
- `T_survival = 0.6` (60% tribal psychology retained)
- `C_hive = 0.85` (85% collective coordination required)

### 1.4 Transcendence Conditions

A civilization transcends the Asynchronous Gap when:
```
C ≥ C_hive  AND  E > 0.5 · E_critical  AND  T < 0.5 · T_survival
```

This represents the Hive-Mind Anomaly: sufficient collective consciousness to prevent internal conflict, with enough technology to sustain the civilization long-term.

---

## 2. System Architecture

### 2.1 Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLI (main.rs)                            │
│  clap argument parsing, config loading, logging initialization  │
├─────────────────────────────────────────────────────────────────┤
│                    Simulation Engine (simulation.rs)             │
│  6-phase tick loop, Rayon parallel dispatch, statistics          │
├──────────────┬──────────────┬──────────────┬────────────────────┤
│  Agent       │  QuadTree    │  Exporter    │  Config            │
│  (agent.rs)  │  (grid.rs)   │ (exporter.rs)│  (serde structs)   │
│  State vecs  │  Spatial AMR │  JSON + CSV  │  Thresholds        │
│  Collapse    │  Range query │  Snapshots   │  Grid params       │
│  Transcend   │  O(1) void   │  Tick hist.  │  Spawn rates       │
└──────────────┴──────────────┴──────────────┴────────────────────┘
                              │
                              ▼ JSON/CSV
┌─────────────────────────────────────────────────────────────────┐
│                 Analytics Layer (Python)                         │
│  dashboard.py — Streamlit interactive dashboard                 │
│  logic_plotter.py — matplotlib publication figures              │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Simulation Tick Loop (6 Phases)

Each tick executes six phases in strict order:

1. **Spatial Index Rebuild**: QuadTree reconstructed from current agent positions. O(N log N) construction; empty quadrants consume zero memory.

2. **Parallel State Advancement**: All active agents update their E, T, C state vectors simultaneously via Rayon's `par_iter_mut`. Each agent's update is independent — no inter-agent dependencies during this phase.

3. **Collapse & Transcendence Evaluation**: Sequential scan evaluates each agent against collapse and transcendence predicates. Collapsed agents are destroyed; transcended agents are marked as inactive survivors.

4. **Spontaneous Spawning**: New civilizations emerge stochastically at rate `spawn_rate` per tick, with initial conditions drawn from calibrated distributions.

5. **Exogenous Extinction**: Random cosmic catastrophes (GRBs, asteroid impacts) applied at rate `exogenous_extinction_rate` per tick per agent.

6. **Statistics Computation**: Aggregate metrics (mean E, T, C, active count, collapse count) computed and appended to tick history.

### 2.3 QuadTree Spatial Management

The simulation grid uses a QuadTree with Adaptive Mesh Refinement:

- **Leaf nodes** store up to `max_agents_per_node` (default: 8) agent references.
- **Internal nodes** subdivide into four equal quadrants when capacity is exceeded.
- **Maximum depth** of 12 prevents infinite recursion on coincident agents.
- **Empty quadrants** are represented as empty leaf nodes — O(1) memory, zero iteration cost during range queries.
- **Full rebuild per tick** is cheaper than incremental updates for the agent densities modeled (sparse civilizations in vast space, typically <10⁴ agents in a 10⁶ unit² grid).

### 2.4 Parallelism Strategy

- **Rayon** provides work-stealing parallelism for agent state advancement (Phase 2).
- Each agent's tick update is pure: it reads only its own state and the shared (immutable) threshold configuration.
- Collapse evaluation (Phase 3) is sequential to avoid concurrent mutation of the agent vector and collapse log.
- Thread count defaults to auto-detection but can be pinned via CLI `--threads N`.

---

## 3. Data Pipeline

### 3.1 Output Files

| File | Format | Contents | Frequency |
|------|--------|----------|-----------|
| `simulation_config.json` | JSON | Complete run parameters | Once (start) |
| `snapshot_tick_XXXXXX.json` | JSON | All active agents at tick | Every N ticks |
| `collapse_log.json` / `.csv` | JSON + CSV | All collapse events | Once (end) |
| `tick_history.json` / `.csv` | JSON + CSV | Per-tick aggregate stats | Once (end) |
| `final_agents.csv` | CSV | All agents' final state | Once (end) |

### 3.2 JSON Schema: Collapse Event
```json
{
  "agent_id": "uuid-v4",
  "tick": 4237,
  "energy_at_collapse": 3.1415,
  "tribalism_at_collapse": 0.7182,
  "collectivism_at_collapse": 0.2718,
  "position": [423.7, 618.2],
  "collapse_type": "AsynchronousGap"
}
```

### 3.3 JSON Schema: Tick Statistics
```json
{
  "tick": 1000,
  "active_agents": 847,
  "nascent_count": 112,
  "evolving_count": 735,
  "transcended_count": 23,
  "collapsed_count": 630,
  "collapses_this_tick": 3,
  "transcensions_this_tick": 0,
  "mean_energy": 1.2345,
  "mean_tribalism": 0.5432,
  "mean_collectivism": 0.2876,
  "max_energy": 4.9876,
  "mean_async_gap": 2.7183
}
```

---

## 4. Performance Characteristics

### 4.1 Complexity Analysis

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| QuadTree rebuild | O(N log N) | N = active agents |
| Agent tick (parallel) | O(N / P) | P = thread count |
| Collapse evaluation | O(N) | Sequential scan |
| Range query | O(log N + K) | K = results in range |
| Snapshot export | O(N) | Serialization cost |

### 4.2 Memory Profile

- **Agent struct**: ~200 bytes per agent
- **QuadTree overhead**: ~48 bytes per internal node, ~40 bytes per leaf
- **1M agents**: ~200 MB agent storage + ~50 MB tree overhead
- **Collapse log**: ~120 bytes per event (grows with simulation length)

### 4.3 Recommended Hardware

| Scale | Agents | Ticks | RAM | CPU Cores | Est. Runtime |
|-------|--------|-------|-----|-----------|-------------|
| Small | 1,000 | 10,000 | 1 GB | 2 | ~5 seconds |
| Medium | 100,000 | 50,000 | 8 GB | 8 | ~15 minutes |
| Large | 1,000,000 | 100,000 | 32 GB | 16+ | ~4 hours |
