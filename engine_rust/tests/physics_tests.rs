// ============================================================================
// tests/physics_tests.rs - CAT physics edge-case integration tests.
// ============================================================================
// These tests verify mathematical correctness under extreme or degenerate
// parameter regimes. They operate through the public API because the invariants
// cross module boundaries between agents, the spatial index, and the simulation
// engine.
// ============================================================================

use cat_simulation_engine::agent::{Agent, AgentState, CollapseThresholds};
use cat_simulation_engine::grid::{BoundingBox, GridConfig, QuadTree};
use cat_simulation_engine::simulation::{Simulation, SimulationConfig};

/// A civilization with energy_growth_rate = 0 must maintain constant energy
/// forever, regardless of elapsed ticks.
///
/// Mathematical invariant: E(t) = E0 * exp(0 * t) = E0.
#[test]
fn test_zero_energy_growth_is_constant() {
    let initial_energy = 1.234_f64;
    let mut agent = Agent::new(
        (50.0, 50.0),
        initial_energy,
        0.3,   // Tribalism below survival_tribalism, so no resource ceiling.
        0.5,   // Collectivism.
        0.0,   // Zero energy growth.
        0.005, // Tribalism decay alpha.
        0.0,   // Collectivism drift.
        0,
    );
    let thresholds = CollapseThresholds::default();

    for _ in 0..10_000 {
        agent.tick(&thresholds);
    }

    assert!(
        (agent.energy - initial_energy).abs() < f64::EPSILON * 10.0,
        "Zero-growth energy drifted: expected {}, got {} (delta={})",
        initial_energy,
        agent.energy,
        (agent.energy - initial_energy).abs()
    );
    assert!(agent.energy.is_finite(), "Energy must remain finite");
}

/// A civilization with very large tribalism_decay_alpha reaches T = 0 early
/// and must remain exactly at 0.0 after the clamp engages.
///
/// Mathematical invariant: T(t) = T0 * max(0, 1 - alpha * ln(1 + t)).
#[test]
fn test_maximum_tribalism_decay_stabilises_at_zero() {
    let mut agent = Agent::new(
        (0.0, 0.0),
        0.5,
        0.9,
        0.3,
        0.01,
        1.0, // Extreme alpha; T reaches zero within about two ticks.
        0.0,
        0,
    );
    let thresholds = CollapseThresholds::default();

    for tick in 0..5_000 {
        agent.tick(&thresholds);
        assert!(
            agent.tribalism >= 0.0,
            "Tribalism went negative at tick {}: {}",
            tick,
            agent.tribalism
        );
        assert!(
            agent.tribalism.is_finite(),
            "Tribalism became non-finite at tick {}: {}",
            tick,
            agent.tribalism
        );
        if tick >= 3 {
            assert_eq!(
                agent.tribalism, 0.0,
                "T should be locked at 0 after decay completes (tick {}): got {}",
                tick, agent.tribalism
            );
        }
    }
}

/// Verify T stays within [0, initial_tribalism] for several valid decay rates.
#[test]
fn test_tribalism_never_exceeds_initial() {
    let cases = [(0.9_f64, 0.001_f64), (0.9, 0.01), (0.5, 0.1), (0.1, 0.001)];
    let thresholds = CollapseThresholds::default();

    for (initial_t, alpha) in cases {
        let mut agent = Agent::new((0.0, 0.0), 0.1, initial_t, 0.3, 0.01, alpha, 0.0, 0);
        for _ in 0..50_000 {
            agent.tick(&thresholds);
            assert!(
                agent.tribalism <= initial_t + f64::EPSILON,
                "T({}) exceeded initial_tribalism({}) with alpha={}",
                agent.tribalism,
                initial_t,
                alpha
            );
            assert!(agent.tribalism >= 0.0);
        }
    }
}

/// After 1,000,000 ticks with maximum modeled growth rate, energy must remain
/// finite and bounded by the absolute cap.
#[test]
fn test_million_tick_overflow_resistance() {
    let mut agent = Agent::new(
        (0.0, 0.0),
        0.3,
        0.3, // Below threshold; resource ceiling does not mask the cap test.
        0.5,
        0.05,
        0.001,
        0.0,
        0,
    );
    let thresholds = CollapseThresholds::default();

    for _ in 0..1_000_000 {
        agent.tick(&thresholds);
    }

    assert!(
        agent.energy.is_finite(),
        "Energy overflowed to {} after 1M ticks",
        agent.energy
    );
    assert!(
        agent.energy <= 1_000.0,
        "Energy exceeded ENERGY_ABS_MAX: {}",
        agent.energy
    );
    assert!(
        agent.tribalism.is_finite(),
        "Tribalism is non-finite after 1M ticks: {}",
        agent.tribalism
    );
    assert!(agent.tribalism >= 0.0);
}

/// If an agent receives corrupt numeric state, tick() must neutralize it rather
/// than propagate NaN into aggregate statistics or exported data.
#[test]
fn test_nan_guard_neutralises_corrupt_agent() {
    let mut agent = Agent::new((0.0, 0.0), 0.5, 0.5, 0.5, 0.01, 0.001, 0.0, 0);
    agent.energy = f64::NAN;

    let thresholds = CollapseThresholds::default();
    agent.tick(&thresholds);

    assert_eq!(
        agent.state,
        AgentState::Collapsed,
        "Agent with NaN energy must be neutralised (Collapsed), got {:?}",
        agent.state
    );
}

/// Build a high-density QuadTree with clustered agents. The tree must build
/// without stack overflow, preserve the active-agent count, stay within
/// max_depth, and return the clustered agents from a tight range query.
#[test]
fn test_high_density_quadtree_stress() {
    let config = GridConfig {
        width: 1000.0,
        height: 1000.0,
        max_agents_per_node: 8,
        max_depth: 12,
    };
    let mut tree = QuadTree::new(&config);

    let n_total = 10_000usize;
    let n_clustered = 1_000usize;
    let cluster_x = 500.0_f64;
    let cluster_y = 500.0_f64;

    let mut agents: Vec<Agent> = Vec::with_capacity(n_total);

    for i in 0..n_clustered {
        let offset = (i as f64) * 0.001;
        agents.push(Agent::new(
            (cluster_x + offset, cluster_y + offset),
            0.1,
            0.5,
            0.3,
            0.01,
            0.001,
            0.0,
            0,
        ));
    }

    for i in 0..(n_total - n_clustered) {
        let x = (i as f64 / (n_total - n_clustered) as f64) * 999.0 + 0.5;
        agents.push(Agent::new(
            (x, x % 1000.0),
            0.1,
            0.5,
            0.3,
            0.01,
            0.001,
            0.0,
            0,
        ));
    }

    tree.rebuild(&agents);

    let active_count = agents.iter().filter(|a| a.is_active()).count();
    assert_eq!(
        tree.total_agents(),
        active_count,
        "QuadTree agent count mismatch: tree={}, active={}",
        tree.total_agents(),
        active_count
    );

    let (_, max_d, _) = tree.depth_stats();
    assert!(
        max_d <= config.max_depth,
        "Tree exceeded max_depth {}: got depth {}",
        config.max_depth,
        max_d
    );

    let cluster_box = BoundingBox::new(
        cluster_x - 0.5,
        cluster_y - 0.5,
        cluster_x + n_clustered as f64 * 0.001 + 0.5,
        cluster_y + n_clustered as f64 * 0.001 + 0.5,
    );
    let found = tree.query_range(&cluster_box);
    assert!(
        found.len() >= n_clustered,
        "Range query returned {} agents; expected at least {} clustered agents",
        found.len(),
        n_clustered
    );
}

/// Two runs with identical seed and parameters must produce the same collapse
/// count. This verifies the deterministic phase ordering and RNG isolation.
#[test]
fn test_simulation_is_deterministic() {
    let make_config = || SimulationConfig {
        max_ticks: 100,
        initial_agents: 200,
        spawn_rate: 0.5,
        seed: 12345,
        snapshot_interval: 10_000,
        output_dir: String::new(),
        base_data_dir: String::new(),
        num_threads: 1,
        ..SimulationConfig::default()
    };

    let collapse_count_a = {
        let mut sim = Simulation::new(make_config());
        for tick in 0..100 {
            sim.advance_tick(tick);
        }
        sim.collapse_count()
    };

    let collapse_count_b = {
        let mut sim = Simulation::new(make_config());
        for tick in 0..100 {
            sim.advance_tick(tick);
        }
        sim.collapse_count()
    };

    assert_eq!(
        collapse_count_a, collapse_count_b,
        "Simulation is non-deterministic: run A had {} collapses, run B had {}",
        collapse_count_a, collapse_count_b
    );
}
