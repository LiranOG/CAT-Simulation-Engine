// ============================================================================
// simulation.rs — Main Tick Loop with Rayon Parallel Processing
// ============================================================================
// The simulation runs millions of civilizations through the Asynchronous Gap,
// collecting the corpses and the occasional transcendent survivor.
// Parallelized via Rayon because even modeling extinction should be efficient.
// ============================================================================

use crate::agent::{Agent, AgentState, CollapseEvent, CollapseThresholds, CollapseType};
use crate::exporter::Exporter;
use crate::grid::{GridConfig, QuadTree};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
// (Arc/Mutex not needed: Rayon's work-stealing owns agent mutation per-phase)

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
    /// Collapse thresholds — the cosmic constants of failure.
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
            agents.len(), config.max_ticks, config.seed
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
    /// - Energy: Uniform(0.01, 0.3) — pre-technological baseline
    /// - Tribalism: Uniform(0.6, 0.95) — biology starts tribal
    /// - Collectivism: Uniform(0.05, 0.4) — individualism is the default
    /// - Energy growth rate: Uniform(0.005, 0.05) — variable tech trajectories
    /// - Tribalism decay α: Uniform(0.001, 0.015) — glacial psychological evolution
    /// - Collectivism drift: Uniform(-0.001, 0.003) — slight upward bias
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
            (x, y), energy, tribalism, collectivism,
            growth_rate, decay_alpha, c_drift, tick,
        )
    }

    /// Execute the full simulation run.
    pub fn run(&mut self) {
        log::info!("=== Simulation commencing: {} ticks ===", self.config.max_ticks);

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
                log::info!(
                    "Tick {}: active={}, collapsed={}, transcended={}, mean_E={:.4}",
                    tick, stats.active_agents, stats.collapsed_count,
                    stats.transcended_count, stats.mean_energy
                );

                if let Err(e) = exporter.export_tick_snapshot(tick, &self.agents) {
                    log::error!("Snapshot export failed at tick {}: {}", tick, e);
                }
            }
        }

        // Final exports
        self.export_results(&exporter);
        log::info!("=== Simulation complete: {} collapses logged ===", self.collapse_log.len());
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

        // Phase 2: Parallel state vector advancement via Rayon
        let thresholds = self.config.thresholds.clone();
        self.agents.par_iter_mut().for_each(|agent| {
            agent.tick(&thresholds);
        });

        // Phase 3: Collapse & transcendence evaluation (sequential for mutation)
        let mut tick_collapses = 0usize;
        let mut tick_transcensions = 0usize;

        for agent in self.agents.iter_mut() {
            if !agent.is_active() {
                continue;
            }
            // Check transcendence first — the rare, quiet victory
            if agent.evaluate_transcendence(&self.config.thresholds) {
                agent.transcend();
                tick_transcensions += 1;
                continue;
            }
            // Check collapse — the common, loud defeat
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

        // Phase 5: Exogenous extinction — the universe's dice roll
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

        // Phase 6: Aggregate statistics
        let stats = self.compute_stats(tick, tick_collapses, tick_transcensions);
        self.tick_history.push(stats);
    }

    /// Compute per-tick aggregate statistics across all agents.
    fn compute_stats(&self, tick: u64, collapses: usize, transcensions: usize) -> TickStats {
        let active: Vec<&Agent> = self.agents.iter().filter(|a| a.is_active()).collect();
        let active_count = active.len();

        let (sum_e, sum_t, sum_c, sum_gap, max_e) = active.iter().fold(
            (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64),
            |(se, st, sc, sg, me), a| {
                (se + a.energy, st + a.tribalism, sc + a.collectivism,
                 sg + a.asynchronous_gap(), me.max(a.energy))
            },
        );

        let n = active_count.max(1) as f64;
        let count_by_state = |s: AgentState| self.agents.iter().filter(|a| a.state == s).count();

        TickStats {
            tick,
            active_agents: active_count,
            nascent_count: count_by_state(AgentState::Nascent),
            evolving_count: count_by_state(AgentState::Evolving),
            transcended_count: count_by_state(AgentState::Transcended),
            collapsed_count: count_by_state(AgentState::Collapsed),
            collapses_this_tick: collapses,
            transcensions_this_tick: transcensions,
            mean_energy: sum_e / n,
            mean_tribalism: sum_t / n,
            mean_collectivism: sum_c / n,
            max_energy: max_e,
            mean_async_gap: sum_gap / n,
        }
    }

    /// Export all results at simulation end.
    fn export_results(&self, exporter: &Exporter) {
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
            self.config.output_dir, self.collapse_log.len(), self.tick_history.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_initialization() {
        let config = SimulationConfig {
            initial_agents: 10, max_ticks: 5, ..Default::default()
        };
        let sim = Simulation::new(config);
        assert_eq!(sim.agents.len(), 10);
        assert_eq!(sim.current_tick, 0);
    }

    #[test]
    fn test_single_step() {
        let config = SimulationConfig {
            initial_agents: 50, max_ticks: 1, ..Default::default()
        };
        let mut sim = Simulation::new(config);
        sim.step(0);
        assert_eq!(sim.tick_history.len(), 1);
    }
}
