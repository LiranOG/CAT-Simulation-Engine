// ============================================================================
// exporter.rs — I/O Operations: JSON & CSV Data Export
// ============================================================================
// Translates the silent deaths of simulated civilizations into structured data.
// Each run gets its own timestamped subdirectory. Data is never overwritten.
// Science demands reproducibility; reproducibility demands provenance.
// ============================================================================

use crate::agent::{Agent, CollapseEvent};
use crate::simulation::{SimulationConfig, TickStats};
use chrono::Local;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct Exporter {
    output_dir: String,
}

#[derive(Debug, Serialize)]
struct AgentCsvRecord {
    id: String,
    state: String,
    pos_x: f64,
    pos_y: f64,
    energy: f64,
    tribalism: f64,
    collectivism: f64,
    initial_energy: f64,
    initial_tribalism: f64,
    energy_growth_rate: f64,
    tribalism_decay_alpha: f64,
    collectivism_drift: f64,
    ticks_since_ignition: u64,
    birth_tick: u64,
    influence_radius: f64,
    asynchronous_gap: f64,
}

#[derive(Debug, Serialize)]
struct CollapseCsvRecord {
    agent_id: String,
    tick: u64,
    energy: f64,
    tribalism: f64,
    collectivism: f64,
    pos_x: f64,
    pos_y: f64,
    collapse_type: String,
}

impl Exporter {
    pub fn new(output_dir: &str) -> Self {
        Self { output_dir: output_dir.to_string() }
    }

    /// Create a timestamped run directory under `{base_data_dir}/runs/`.
    ///
    /// Format: `{base}/runs/YYYY-MM-DD_HHMMSS_seed{seed}_n{agents}/`
    ///
    /// Contract: the returned path ALWAYS contains a `runs/` component.
    /// No existing data is modified. Each call produces a unique leaf directory.
    /// Returns the absolute path string to the newly created run directory.
    ///
    /// # Panics (debug only)
    /// Asserts the returned path contains "runs" as a path component, catching
    /// any future refactor that accidentally strips the subdirectory level.
    pub fn create_run_directory(
        base_data_dir: &str,
        seed: u64,
        num_agents: usize,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = Local::now().format("%Y-%m-%d_%H%M%S").to_string();
        let run_name = format!("{}_seed{}_n{}", timestamp, seed, num_agents);

        // ALL run output goes into {base}/runs/{name}/ — never into {base}/ directly.
        let runs_root = Path::new(base_data_dir).join("runs");
        let run_path  = runs_root.join(&run_name);

        fs::create_dir_all(&run_path)?;
        let path_str = run_path.to_string_lossy().to_string();

        // Defensive: assert the archive hierarchy is intact.
        debug_assert!(
            run_path.components().any(|c| c.as_os_str() == "runs"),
            "BUG: run_path does not contain a 'runs' component: {}",
            path_str
        );

        log::info!("══ Run archive created ═════════════════════════════════════");
        log::info!("   Base dir : {}", base_data_dir);
        log::info!("   Runs root: {}", runs_root.display());
        log::info!("   Run dir  : {}", path_str);
        log::info!("   All output will be written EXCLUSIVELY to this subdirectory.");
        log::info!("═══════════════════════════════════════════════════════════");

        Ok(path_str)
    }

    pub fn ensure_output_dir(&self) {
        if let Err(e) = fs::create_dir_all(&self.output_dir) {
            log::error!("Failed to create output dir '{}': {}", self.output_dir, e);
        }
    }

    pub fn export_config(
        &self,
        config: &SimulationConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(&self.output_dir).join("simulation_config.json");
        let json = serde_json::to_string_pretty(config)?;
        fs::write(&path, json)?;
        log::info!("Config exported → {:?}", path);
        Ok(())
    }

    pub fn export_tick_snapshot(
        &self,
        tick: u64,
        agents: &[Agent],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(&self.output_dir)
            .join(format!("snapshot_tick_{:06}.json", tick));
        let active: Vec<&Agent> = agents.iter().filter(|a| a.is_active()).collect();
        let json = serde_json::to_string_pretty(&active)?;
        let mut file = fs::File::create(&path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn export_collapse_log(
        &self,
        events: &[CollapseEvent],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_path = Path::new(&self.output_dir).join("collapse_log.json");
        fs::write(&json_path, serde_json::to_string_pretty(events)?)?;

        let csv_path = Path::new(&self.output_dir).join("collapse_log.csv");
        let mut wtr = csv::Writer::from_path(&csv_path)?;
        for event in events {
            wtr.serialize(CollapseCsvRecord {
                agent_id: event.agent_id.to_string(),
                tick: event.tick,
                energy: event.energy_at_collapse,
                tribalism: event.tribalism_at_collapse,
                collectivism: event.collectivism_at_collapse,
                pos_x: event.position.0,
                pos_y: event.position.1,
                collapse_type: format!("{:?}", event.collapse_type),
            })?;
        }
        wtr.flush()?;
        log::info!("Collapse log: {} events → {:?}", events.len(), csv_path);
        Ok(())
    }

    pub fn export_tick_history(
        &self,
        history: &[TickStats],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_path = Path::new(&self.output_dir).join("tick_history.json");
        fs::write(&json_path, serde_json::to_string_pretty(history)?)?;

        let csv_path = Path::new(&self.output_dir).join("tick_history.csv");
        let mut wtr = csv::Writer::from_path(&csv_path)?;
        for stat in history {
            wtr.serialize(stat)?;
        }
        wtr.flush()?;
        log::info!("Tick history: {} records exported.", history.len());
        Ok(())
    }

    pub fn export_final_agents(
        &self,
        agents: &[Agent],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let csv_path = Path::new(&self.output_dir).join("final_agents.csv");
        let mut wtr = csv::Writer::from_path(&csv_path)?;
        for agent in agents {
            wtr.serialize(AgentCsvRecord {
                id: agent.id.to_string(),
                state: format!("{:?}", agent.state),
                pos_x: agent.position.0,
                pos_y: agent.position.1,
                energy: agent.energy,
                tribalism: agent.tribalism,
                collectivism: agent.collectivism,
                initial_energy: agent.initial_energy,
                initial_tribalism: agent.initial_tribalism,
                energy_growth_rate: agent.energy_growth_rate,
                tribalism_decay_alpha: agent.tribalism_decay_alpha,
                collectivism_drift: agent.collectivism_drift,
                ticks_since_ignition: agent.ticks_since_ignition,
                birth_tick: agent.birth_tick,
                influence_radius: agent.influence_radius,
                asynchronous_gap: agent.asynchronous_gap(),
            })?;
        }
        wtr.flush()?;
        log::info!("Final agents: {} records → {:?}", agents.len(), csv_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exporter_creation() {
        let exporter = Exporter::new("./test_output");
        assert_eq!(exporter.output_dir, "./test_output");
    }
}
