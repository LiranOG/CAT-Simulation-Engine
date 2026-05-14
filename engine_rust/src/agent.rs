// ============================================================================
// agent.rs — Agent State Vectors & Collapse Dynamics
// ============================================================================
// Technology (E) scales as e^x. Psychological maturity scales as ln(x).
// The exponential always wins. The universe does not reward ambition.
//
// NUMERICAL STABILITY NOTE:
//   E(t) = E₀ · e^(r·t) is evaluated DIRECTLY from initial conditions each
//   tick. The previous implementation applied the growth multiplicatively
//   against the CURRENT energy, producing E(n) = E₀·e^(r·n(n+1)/2) — an
//   exponential of a quadratic that overflows f64 by tick ~700.
//   Same correction applied to T(t): computed from initial_tribalism, not
//   the decayed-current value, which was compounding decay into subnormals.
// ============================================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Exponent argument cap: e^MAX_EXPONENT ≈ 5.2×10²¹.
/// Any civilization with energy this far beyond E_critical (2.5) has been
/// collapsed or transcended long ago. This prevents f64 overflow while
/// preserving all physically meaningful dynamics.
const MAX_EXPONENT: f64 = 50.0;

/// Hard energy ceiling regardless of other parameters.
/// Ensures f64 stays finite; values beyond this are physically irrelevant.
const ENERGY_ABS_MAX: f64 = 1_000.0;

/// Obituary of a civilization that failed the Asynchronous Gap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollapseEvent {
    pub agent_id: Uuid,
    pub tick: u64,
    pub energy_at_collapse: f64,
    pub tribalism_at_collapse: f64,
    pub collectivism_at_collapse: f64,
    pub position: (f64, f64),
    pub collapse_type: CollapseType,
}

/// Taxonomy of civilizational failure modes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CollapseType {
    /// E >> T_maturity: tribalism weaponizes exponential technology.
    AsynchronousGap,
    /// Burned through planetary resources while arguing about ownership.
    ResourceDepletion,
    /// GRB, asteroid, stellar death — cosmic bad luck.
    ExogenousExtinction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentState {
    Nascent,
    Evolving,
    Transcended,
    Collapsed,
}

/// A civilization. State vectors: E (energy), T (tribalism), C (collectivism).
///
/// # Equations (evaluated DIRECTLY from initial conditions each tick):
///
/// E(t) = min(E₀ · e^(r·t), ENERGY_ABS_MAX)
///   — exponential growth, capped against f64 overflow.
///   — additionally capped at resource_ceiling when T > T_survival.
///
/// T(t) = T₀ · max(0, 1 − α·ln(1+t))
///   — logarithmic decay from initial_tribalism, NOT from the previous T.
///   — the previous cumulative formulation compounded decay into subnormals.
///
/// C(t) = clamp(C₀ + δ·t, 0, 1)
///   — linear drift evaluated directly to avoid floating-point drift issues
///     over very long runs. Uses initial_collectivism + accumulated delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub state: AgentState,
    pub position: (f64, f64),

    // ── Current state vectors (updated each tick) ──────────────────
    pub energy: f64,
    pub tribalism: f64,
    pub collectivism: f64,

    // ── Initial conditions (immutable reference baseline) ──────────
    // Stored to allow direct-formula evaluation rather than cumulative
    // application, which compounds errors catastrophically over long runs.
    pub initial_energy: f64,
    pub initial_tribalism: f64,
    pub initial_collectivism: f64,

    // ── Growth parameters ──────────────────────────────────────────
    pub energy_growth_rate: f64,
    pub tribalism_decay_alpha: f64,
    pub collectivism_drift: f64,

    // ── Internal clock ─────────────────────────────────────────────
    pub ticks_since_ignition: u64,
    pub birth_tick: u64,
    pub influence_radius: f64,
}

/// Collapse thresholds — the cosmic speed limits no civilization outruns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollapseThresholds {
    /// Energy above which technology is existentially dangerous (~mid-Type-I).
    pub critical_energy: f64,
    /// Tribalism above which internal conflict weaponizes technology.
    pub survival_tribalism: f64,
    /// Collectivism above which hive-mind anomaly activates.
    pub hive_collectivism: f64,
    /// Per-tick probability of exogenous extinction.
    pub exogenous_extinction_rate: f64,
    /// Max energy achievable without interstellar expansion.
    pub resource_ceiling: f64,
}

impl Default for CollapseThresholds {
    fn default() -> Self {
        Self {
            critical_energy: 2.5,
            survival_tribalism: 0.6,
            hive_collectivism: 0.85,
            exogenous_extinction_rate: 1e-6,
            resource_ceiling: 5.0,
        }
    }
}

impl Agent {
    pub fn new(
        position: (f64, f64),
        energy: f64,
        tribalism: f64,
        collectivism: f64,
        energy_growth_rate: f64,
        tribalism_decay_alpha: f64,
        collectivism_drift: f64,
        birth_tick: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: AgentState::Nascent,
            position,
            energy,
            tribalism,
            collectivism,
            initial_energy: energy,
            initial_tribalism: tribalism,
            initial_collectivism: collectivism,
            energy_growth_rate,
            tribalism_decay_alpha,
            collectivism_drift,
            ticks_since_ignition: 0,
            birth_tick,
            influence_radius: 0.0,
        }
    }

    /// Advance state vectors by one tick using DIRECT formulae from initial
    /// conditions. This is the only numerically stable approach over long runs.
    ///
    /// # Energy — E(t) = E₀ · e^(r·t), capped
    ///
    ///   Exponent is clamped to MAX_EXPONENT before calling exp() to prevent
    ///   f64 overflow (which serializes to JSON `null`). At r=0.05 this cap
    ///   engages at t=1000 ticks — well after any civilization with high r
    ///   has already collapsed or transcended.
    ///
    /// # Tribalism — T(t) = T₀ · max(0, 1 − α·ln(1+t))
    ///
    ///   ln(1+t) is always in the domain [0, ∞) since t ≥ 0, so ln(0) is
    ///   never reached (the +1 guard handles t=0 giving ln(1)=0).
    ///   Evaluated from initial_tribalism to prevent cumulative subnormal drift.
    ///
    /// # Collectivism — C(t) = clamp(C₀ + δ·t, 0, 1)
    ///
    ///   Direct evaluation from initial_collectivism eliminates accumulated
    ///   floating-point error over 100k+ ticks.
    pub fn tick(&mut self, thresholds: &CollapseThresholds) {
        if self.state == AgentState::Collapsed || self.state == AgentState::Transcended {
            return;
        }
        self.ticks_since_ignition += 1;
        let t = self.ticks_since_ignition as f64;

        // ── Energy: E(t) = E₀ · e^(r·t), overflow-safe ────────────────
        let exponent = (self.energy_growth_rate * t).min(MAX_EXPONENT);
        let raw_energy = (self.initial_energy * exponent.exp()).min(ENERGY_ABS_MAX);

        // Resource ceiling when tribal psychology blocks interstellar expansion.
        self.energy = if self.tribalism > thresholds.survival_tribalism {
            raw_energy.min(thresholds.resource_ceiling)
        } else {
            raw_energy
        };

        // ── Tribalism: T(t) = T₀ · max(0, 1 − α·ln(1+t)) ─────────────
        // ln(1+t) is defined for all t ≥ 0. The max(0) prevents negative T.
        // Critical: computed from initial_tribalism, NOT self.tribalism.
        let maturity_factor = (1.0 - self.tribalism_decay_alpha * (1.0 + t).ln()).max(0.0);
        self.tribalism = self.initial_tribalism * maturity_factor;

        // ── Collectivism: C(t) = clamp(C₀ + δ·t, 0, 1) ───────────────
        // Direct formula prevents accumulated floating-point rounding error
        // from drift on long runs.
        self.collectivism =
            (self.initial_collectivism + self.collectivism_drift * t).clamp(0.0, 1.0);

        // ── Influence radius: R = √E · κ ───────────────────────────────
        // Energy is finite by the cap above, so sqrt is always defined.
        self.influence_radius = self.energy.sqrt() * 0.1;

        // ── State: Nascent → Evolving at technological ignition ─────────
        if self.state == AgentState::Nascent && self.energy > 0.5 {
            self.state = AgentState::Evolving;
            log::info!(
                "Agent {} ignited at tick {}.",
                self.id,
                self.birth_tick + self.ticks_since_ignition
            );
        }
    }

    /// Collapse predicate: E > E_crit AND T > T_surv AND C < C_hive.
    pub fn evaluate_collapse(
        &self,
        tick: u64,
        thresholds: &CollapseThresholds,
    ) -> Option<CollapseEvent> {
        if self.state == AgentState::Collapsed || self.state == AgentState::Transcended {
            return None;
        }
        if self.energy > thresholds.critical_energy
            && self.tribalism > thresholds.survival_tribalism
            && self.collectivism < thresholds.hive_collectivism
        {
            return Some(CollapseEvent {
                agent_id: self.id,
                tick,
                energy_at_collapse: self.energy,
                tribalism_at_collapse: self.tribalism,
                collectivism_at_collapse: self.collectivism,
                position: self.position,
                collapse_type: CollapseType::AsynchronousGap,
            });
        }
        if self.energy >= thresholds.resource_ceiling * 0.99
            && self.tribalism > thresholds.survival_tribalism
        {
            return Some(CollapseEvent {
                agent_id: self.id,
                tick,
                energy_at_collapse: self.energy,
                tribalism_at_collapse: self.tribalism,
                collectivism_at_collapse: self.collectivism,
                position: self.position,
                collapse_type: CollapseType::ResourceDepletion,
            });
        }
        None
    }

    /// Transcendence: high-C civilizations bypass the filter silently.
    pub fn evaluate_transcendence(&self, thresholds: &CollapseThresholds) -> bool {
        if self.state == AgentState::Collapsed || self.state == AgentState::Transcended {
            return false;
        }
        self.collectivism >= thresholds.hive_collectivism
            && self.energy > thresholds.critical_energy * 0.5
            && self.tribalism < thresholds.survival_tribalism * 0.5
    }

    pub fn destroy(&mut self) {
        self.state = AgentState::Collapsed;
        self.influence_radius = 0.0;
        log::warn!(
            "Agent {} collapsed. E={:.4} T={:.4} C={:.4}",
            self.id,
            self.energy,
            self.tribalism,
            self.collectivism
        );
    }

    pub fn transcend(&mut self) {
        self.state = AgentState::Transcended;
        log::info!(
            "Agent {} transcended. E={:.4} T={:.4} C={:.4}",
            self.id,
            self.energy,
            self.tribalism,
            self.collectivism
        );
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, AgentState::Nascent | AgentState::Evolving)
    }

    /// Gap metric: E / (1 − T + ε). High → existential danger.
    pub fn asynchronous_gap(&self) -> f64 {
        self.energy / (1.0 - self.tribalism + 1e-10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thresholds() -> CollapseThresholds {
        CollapseThresholds::default()
    }

    #[test]
    fn test_agent_creation() {
        let a = Agent::new((10.0, 20.0), 0.1, 0.9, 0.1, 0.01, 0.005, 0.001, 0);
        assert_eq!(a.state, AgentState::Nascent);
        assert!(a.is_active());
        assert_eq!(a.initial_energy, 0.1);
        assert_eq!(a.initial_tribalism, 0.9);
    }

    #[test]
    fn test_exponential_energy_growth() {
        let mut a = Agent::new((0.0, 0.0), 1.0, 0.3, 0.5, 0.1, 0.001, 0.0, 0);
        let t = thresholds();
        for _ in 0..10 {
            a.tick(&t);
        }
        // E(10) = 1.0 * e^(0.1 * 10) = e^1 ≈ 2.718
        let expected = (0.1_f64 * 10.0).exp();
        assert!((a.energy - expected).abs() < 1e-6);
    }

    #[test]
    fn test_no_overflow_long_run() {
        // r=0.05, 100_000 ticks → exponent = 5000, capped at MAX_EXPONENT
        let mut a = Agent::new((0.0, 0.0), 0.3, 0.3, 0.5, 0.05, 0.001, 0.0, 0);
        let t = thresholds();
        for _ in 0..100_000 {
            a.tick(&t);
        }
        assert!(
            a.energy.is_finite(),
            "Energy must remain finite over 100k ticks: got {}",
            a.energy
        );
        assert!(a.energy <= ENERGY_ABS_MAX);
    }

    #[test]
    fn test_tribalism_direct_formula() {
        let mut a = Agent::new((0.0, 0.0), 0.1, 0.9, 0.1, 0.001, 0.01, 0.0, 0);
        let t = thresholds();
        for _ in 0..9901 {
            a.tick(&t);
        }
        // T(9901) = 0.9 * max(0, 1 - 0.01 * ln(9902))
        let expected = 0.9 * (1.0 - 0.01 * (9902.0_f64).ln()).max(0.0);
        assert!(
            (a.tribalism - expected).abs() < 1e-10,
            "Tribalism should be {:.6}, got {:.6}. Subnormal drift detected.",
            expected,
            a.tribalism
        );
        assert!(a.tribalism.is_finite() && a.tribalism > 0.0);
    }

    #[test]
    fn test_tribalism_no_negative_domain() {
        // ln(1+t) is always in [0, ∞) for t ≥ 0. Verify no NaN/negative.
        let mut a = Agent::new((0.0, 0.0), 0.1, 0.9, 0.1, 0.001, 0.1, 0.0, 0);
        let t = thresholds();
        for _ in 0..1000 {
            a.tick(&t);
            assert!(a.tribalism.is_finite());
            assert!(a.tribalism >= 0.0);
        }
    }

    #[test]
    fn test_asynchronous_gap_collapse() {
        let a = Agent::new((5.0, 5.0), 3.0, 0.8, 0.1, 0.01, 0.001, 0.0, 0);
        let c = a.evaluate_collapse(100, &thresholds());
        assert!(c.is_some());
        assert_eq!(c.unwrap().collapse_type, CollapseType::AsynchronousGap);
    }

    #[test]
    fn test_hive_mind_transcendence() {
        let a = Agent::new((0.0, 0.0), 2.0, 0.2, 0.95, 0.001, 0.01, 0.001, 0);
        assert!(a.evaluate_transcendence(&thresholds()));
    }

    #[test]
    fn test_high_c_prevents_collapse() {
        let a = Agent::new((0.0, 0.0), 3.0, 0.8, 0.9, 0.01, 0.001, 0.0, 0);
        assert!(a.evaluate_collapse(100, &thresholds()).is_none());
    }

    #[test]
    fn test_destroy() {
        let mut a = Agent::new((0.0, 0.0), 1.0, 0.5, 0.3, 0.01, 0.001, 0.0, 0);
        a.destroy();
        assert_eq!(a.state, AgentState::Collapsed);
        assert!(!a.is_active());
    }

    #[test]
    fn test_collectivism_direct_formula() {
        // With drift = -0.001, starting at 0.5: C(500) = max(0, 0.5 - 0.5) = 0.0
        let mut a = Agent::new((0.0, 0.0), 0.1, 0.5, 0.5, 0.001, 0.005, -0.001, 0);
        let t = thresholds();
        for _ in 0..600 {
            a.tick(&t);
        }
        assert_eq!(a.collectivism, 0.0, "Should clamp to 0 after negative drift");
        assert!(a.collectivism.is_finite());
    }
}
