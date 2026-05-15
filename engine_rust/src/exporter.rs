// ============================================================================
// exporter.rs - JSON and CSV archive export.
// ============================================================================
// The exporter is the boundary between simulation state and reproducible data.
// Each completed run receives a timestamped archive directory. Final outputs
// are written with a write-then-rename pattern where practical, and the manifest
// is written last so analytics tools can distinguish complete runs from
// interrupted runs.
// ============================================================================

// Atomic I/O strategy:
// Final exports (collapse_log, tick_history, final_agents) use a write-then-
// rename pattern: data is written to "{path}.part", then fs::rename() moves it
// to the final path. On the same filesystem, rename() is atomic on both Unix
// and Windows NTFS for new-file creation when the destination does not exist.
// A crash during the .part write leaves a harmless orphan; the final file
// either exists in its complete form or not at all. No partial reads possible.
//
// Tick snapshot files (snapshot_tick_NNNNNN.json) are incremental by nature
// and are not atomically renamed. They provide in-progress data and
// are acceptable as best-effort exports.
//
// RUN_MANIFEST.json:
// Written as the LAST action of a successful simulation run. The dashboard uses
// the presence of this file to distinguish completed runs from crashed/incomplete
// ones. A run directory without RUN_MANIFEST.json is treated as incomplete and
// shown as a warning, never silently loaded as valid data.
use crate::agent::{Agent, CollapseEvent};
use crate::simulation::{SimulationConfig, TickStats};
use chrono::Local;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct Exporter {
    output_dir: String,
}

/// Completion manifest written as the last action of every successful run.
/// The dashboard checks for this file before treating a run as loadable.
/// Its presence guarantees all final data files were fully written.
#[derive(Debug, Serialize)]
struct RunManifest {
    /// ISO-8601 timestamp of when the run completed.
    completed_at: String,
    /// Engine version string for forward-compatibility checks.
    engine_version: &'static str,
    /// Total simulation ticks executed.
    total_ticks: u64,
    /// Total civilisational collapse events logged.
    total_collapses: usize,
    /// Total civilisational transcendence events logged.
    total_transcensions: usize,
    /// Inventory of all final data files: filename to byte size.
    /// A mismatch between recorded and actual size signals post-run tampering
    /// or incomplete transfer of the run directory.
    file_inventory: BTreeMap<String, u64>,
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
    /// Create an exporter that writes into a resolved run output directory.
    pub fn new(output_dir: &str) -> Self {
        Self {
            output_dir: output_dir.to_string(),
        }
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

        // All run output goes into {base}/runs/{name}/, never into {base}/ directly.
        let runs_root = Path::new(base_data_dir).join("runs");
        let run_path = runs_root.join(&run_name);

        fs::create_dir_all(&run_path)?;
        let path_str = run_path.to_string_lossy().to_string();

        // Defensive: assert the archive hierarchy is intact.
        debug_assert!(
            run_path.components().any(|c| c.as_os_str() == "runs"),
            "BUG: run_path does not contain a 'runs' component: {}",
            path_str
        );

        log::info!("Run archive created.");
        log::info!("   Base dir : {}", base_data_dir);
        log::info!("   Runs root: {}", runs_root.display());
        log::info!("   Run dir  : {}", path_str);
        log::info!("   All output will be written EXCLUSIVELY to this subdirectory.");

        Ok(path_str)
    }

    /// Ensure the output directory exists before files are written.
    pub fn ensure_output_dir(&self) {
        if let Err(e) = fs::create_dir_all(&self.output_dir) {
            log::error!("Failed to create output dir '{}': {}", self.output_dir, e);
        }
    }

    /// Write `data` to `path` atomically using a write-then-rename pattern.
    ///
    /// 1. Write to `{path}.part` (crash here leaves an orphan, not a corrupt file).
    /// 2. `fs::rename("{path}.part", path)` on the same filesystem.
    ///
    /// If the destination does not yet exist (always the case for a fresh run
    /// directory), NTFS and ext4 guarantee the rename is atomic for new files.
    fn write_atomic(path: &Path, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let part_path = path.with_extension(format!(
            "{}.part",
            path.extension().unwrap_or_default().to_string_lossy()
        ));
        // Write fully to the .part file.
        let mut file = fs::File::create(&part_path)?;
        file.write_all(data)?;
        file.flush()?;
        drop(file); // Ensure file handle is closed before rename on Windows.
                    // Atomic rename: either the final file exists complete, or not at all.
        fs::rename(&part_path, path)?;
        Ok(())
    }

    /// Export the resolved simulation configuration for reproducibility.
    pub fn export_config(
        &self,
        config: &SimulationConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(&self.output_dir).join("simulation_config.json");
        let json = serde_json::to_string_pretty(config)?;
        fs::write(&path, json)?;
        log::info!("Config exported to {:?}", path);
        Ok(())
    }

    /// Export the active-agent state at a snapshot tick.
    pub fn export_tick_snapshot(
        &self,
        tick: u64,
        agents: &[Agent],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new(&self.output_dir).join(format!("snapshot_tick_{:06}.json", tick));
        let active: Vec<&Agent> = agents.iter().filter(|a| a.is_active()).collect();
        let json = serde_json::to_string_pretty(&active)?;
        let mut file = fs::File::create(&path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Export collapse events as JSON and CSV.
    pub fn export_collapse_log(
        &self,
        events: &[CollapseEvent],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // JSON: atomic write.
        let json_path = Path::new(&self.output_dir).join("collapse_log.json");
        Self::write_atomic(&json_path, serde_json::to_string_pretty(events)?.as_bytes())?;

        // CSV: build in memory, then atomic write.
        let csv_path = Path::new(&self.output_dir).join("collapse_log.csv");
        let mut buf = Vec::with_capacity(events.len() * 128);
        {
            let mut wtr = csv::Writer::from_writer(&mut buf);
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
        }
        Self::write_atomic(&csv_path, &buf)?;
        log::info!(
            "Collapse log (atomic): {} events to {:?}",
            events.len(),
            csv_path
        );
        Ok(())
    }

    /// Export aggregate per-tick statistics as JSON and CSV.
    pub fn export_tick_history(
        &self,
        history: &[TickStats],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // JSON: atomic write.
        let json_path = Path::new(&self.output_dir).join("tick_history.json");
        Self::write_atomic(
            &json_path,
            serde_json::to_string_pretty(history)?.as_bytes(),
        )?;

        // CSV: build in memory, then atomic write.
        let csv_path = Path::new(&self.output_dir).join("tick_history.csv");
        let mut buf = Vec::with_capacity(history.len() * 128);
        {
            let mut wtr = csv::Writer::from_writer(&mut buf);
            for stat in history {
                wtr.serialize(stat)?;
            }
            wtr.flush()?;
        }
        Self::write_atomic(&csv_path, &buf)?;
        log::info!("Tick history (atomic): {} records exported.", history.len());
        Ok(())
    }

    /// Export the terminal state of every agent as CSV.
    pub fn export_final_agents(&self, agents: &[Agent]) -> Result<(), Box<dyn std::error::Error>> {
        // CSV: build in memory, then atomic write.
        let csv_path = Path::new(&self.output_dir).join("final_agents.csv");
        let mut buf = Vec::with_capacity(agents.len() * 256);
        {
            let mut wtr = csv::Writer::from_writer(&mut buf);
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
                    // influence_radius is now a computed method on Agent.
                    influence_radius: agent.influence_radius(),
                    asynchronous_gap: agent.asynchronous_gap(),
                })?;
            }
            wtr.flush()?;
        }
        Self::write_atomic(&csv_path, &buf)?;
        log::info!(
            "Final agents (atomic): {} records to {:?}",
            agents.len(),
            csv_path
        );
        Ok(())
    }

    /// Write RUN_MANIFEST.json as the final action of a successful simulation.
    ///
    /// The dashboard's run-discovery logic treats the presence of this file as
    /// the canonical signal that a run completed successfully. Directories
    /// without it are incomplete/crashed runs and must not be silently loaded.
    pub fn export_run_manifest(
        &self,
        total_ticks: u64,
        total_collapses: usize,
        total_transcensions: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Inventory every file in the run directory (excluding the manifest itself
        // and in-progress .part files).
        let mut file_inventory: BTreeMap<String, u64> = BTreeMap::new();
        let run_dir = Path::new(&self.output_dir);
        for entry in fs::read_dir(run_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".part") || name == "RUN_MANIFEST.json" {
                continue;
            }
            if entry.file_type()?.is_file() {
                let size = entry.metadata()?.len();
                file_inventory.insert(name, size);
            }
        }

        let manifest = RunManifest {
            completed_at: Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string(),
            engine_version: "1.0.0",
            total_ticks,
            total_collapses,
            total_transcensions,
            file_inventory,
        };

        let manifest_path = run_dir.join("RUN_MANIFEST.json");
        Self::write_atomic(
            &manifest_path,
            serde_json::to_string_pretty(&manifest)?.as_bytes(),
        )?;
        log::info!("RUN_MANIFEST written to {:?}", manifest_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Verify the exporter stores the configured output directory.
    fn test_exporter_creation() {
        let exporter = Exporter::new("./test_output");
        assert_eq!(exporter.output_dir, "./test_output");
    }
}
