// ============================================================================
// lib.rs - CAT Simulation Engine library crate.
// ============================================================================
// The binary and integration tests consume the same public module surface. This
// keeps command-line execution, tests, and downstream library use aligned.
// ============================================================================

pub mod agent;
pub mod exporter;
pub mod grid;
pub mod simulation;
