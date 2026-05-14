// ============================================================================
// main.rs — CLI Entry Point for the CAT Simulation Engine
// ============================================================================
// Parses arguments, creates the timestamped run directory, then hands
// control to the engine. Each invocation produces a unique output path;
// previous runs are never touched.
// ============================================================================

mod agent;
mod exporter;
mod grid;
mod simulation;

use agent::CollapseThresholds;
use clap::Parser;
use exporter::Exporter;
use grid::GridConfig;
use simulation::{Simulation, SimulationConfig};

/// CAT Simulation Engine: Agent-Based Fermi Paradox Model.
///
/// Technology scales exponentially (E = e^(r·t)).
/// Wisdom scales logarithmically (T ∝ ln(t)).
/// The exponential always wins.
/// Hive-minds survive. Everyone else is silence.
#[derive(Parser, Debug)]
#[command(name = "cat-engine")]
#[command(version = "1.0.0")]
#[command(about = "Cosmobiological Asynchrony Theory — Fermi Paradox ABM")]
struct Cli {
    /// Total simulation ticks to execute.
    #[arg(short = 't', long, default_value_t = 10_000)]
    ticks: u64,

    /// Number of civilizations to seed at t=0.
    #[arg(short = 'n', long, default_value_t = 1_000)]
    agents: usize,

    /// Per-tick probability of spontaneous civilization emergence.
    #[arg(short = 's', long, default_value_t = 0.5)]
    spawn_rate: f64,

    /// RNG seed. Change for ensemble studies. Reproducibility demands it.
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Critical energy threshold (Kardashev level triggering collapse risk).
    #[arg(long, default_value_t = 2.5)]
    critical_energy: f64,

    /// Tribalism threshold above which collapse is possible.
    #[arg(long, default_value_t = 0.6)]
    survival_tribalism: f64,

    /// Collectivism threshold above which hive-mind anomaly activates.
    #[arg(long, default_value_t = 0.85)]
    hive_collectivism: f64,

    /// Grid width (simulation space units).
    #[arg(long, default_value_t = 1000.0)]
    grid_width: f64,

    /// Grid height (simulation space units).
    #[arg(long, default_value_t = 1000.0)]
    grid_height: f64,

    /// Ticks between snapshot exports.
    #[arg(long, default_value_t = 100)]
    snapshot_interval: u64,

    /// Base data directory. Each run creates a unique subdirectory here.
    /// Format: {base}/runs/YYYY-MM-DD_HHMMSS_seed{seed}_n{agents}/
    #[arg(short = 'o', long, default_value = "../data")]
    base_data_dir: String,

    /// Number of Rayon threads (0 = auto-detect).
    #[arg(long, default_value_t = 0)]
    threads: usize,

    /// Load full configuration from a JSON file (overrides all CLI args).
    #[arg(short = 'c', long)]
    config_file: Option<String>,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let cli = Cli::parse();

    log::info!("╔══════════════════════════════════════════════════════════╗");
    log::info!("║  CAT Simulation Engine v1.0.0                           ║");
    log::info!("║  Cosmobiological Asynchrony Theory — Fermi Paradox ABM  ║");
    log::info!("╚══════════════════════════════════════════════════════════╝");

    // ── Load or build configuration ─────────────────────────────────────────
    let (mut config, base_dir) = if let Some(ref config_path) = cli.config_file {
        match std::fs::read_to_string(config_path) {
            Ok(json) => match serde_json::from_str::<SimulationConfig>(&json) {
                Ok(cfg) => {
                    log::info!("Configuration loaded from '{}'", config_path);
                    let base = cfg.base_data_dir.clone();
                    (cfg, base)
                }
                Err(e) => {
                    log::error!("Failed to parse config '{}': {}", config_path, e);
                    std::process::exit(1);
                }
            },
            Err(e) => {
                log::error!("Failed to read config '{}': {}", config_path, e);
                std::process::exit(1);
            }
        }
    } else {
        let base = cli.base_data_dir.clone();
        let cfg = SimulationConfig {
            max_ticks: cli.ticks,
            initial_agents: cli.agents,
            spawn_rate: cli.spawn_rate,
            seed: cli.seed,
            thresholds: CollapseThresholds {
                critical_energy: cli.critical_energy,
                survival_tribalism: cli.survival_tribalism,
                hive_collectivism: cli.hive_collectivism,
                ..CollapseThresholds::default()
            },
            grid_config: GridConfig {
                width: cli.grid_width,
                height: cli.grid_height,
                ..GridConfig::default()
            },
            snapshot_interval: cli.snapshot_interval,
            base_data_dir: cli.base_data_dir,
            output_dir: String::new(), // resolved below
            num_threads: cli.threads,
        };
        (cfg, base)
    };

    // ── Resolve base_data_dir to an absolute path ────────────────────────────
    // Relative paths like "../data" resolve differently depending on the
    // working directory from which `cargo run` or the binary is invoked.
    // Canonicalizing here eliminates all ambiguity.
    let abs_base_dir: String = {
        let raw = std::path::Path::new(&base_dir);
        if raw.is_absolute() {
            base_dir.clone()
        } else {
            // Join cwd + relative path, then normalize.
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(raw).to_string_lossy().to_string(),
                Err(e) => {
                    log::warn!("Cannot determine cwd ({}); using raw path '{}'", e, base_dir);
                    base_dir.clone()
                }
            }
        }
    };
    // Update config so it persists the resolved path in simulation_config.json.
    config.base_data_dir = abs_base_dir.clone();
    log::info!("Data root (absolute): {}", abs_base_dir);

    // ── Create timestamped run directory ─────────────────────────────────────
    let run_dir = match Exporter::create_run_directory(
        &abs_base_dir,
        config.seed,
        config.initial_agents,
    ) {
        Ok(dir) => dir,
        Err(e) => {
            log::error!("Failed to create run directory under '{}': {}", abs_base_dir, e);
            std::process::exit(1);
        }
    };

    // output_dir is the fully-resolved timestamped subdirectory.
    // This is what the Exporter writes all files to.
    config.output_dir = run_dir.clone();

    log::info!(
        "Run: {} agents | {} ticks | seed={} | E_crit={:.2} | T_surv={:.2} | C_hive={:.2}",
        config.initial_agents,
        config.max_ticks,
        config.seed,
        config.thresholds.critical_energy,
        config.thresholds.survival_tribalism,
        config.thresholds.hive_collectivism,
    );
    log::info!("Run output (absolute): {}", run_dir);

    let mut sim = Simulation::new(config);
    sim.run();

    log::info!("Engine shutdown complete. Data in: {}", run_dir);
}
