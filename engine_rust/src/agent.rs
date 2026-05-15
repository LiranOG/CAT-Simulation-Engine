// ============================================================================
// agent.rs - Agent state vectors and collapse dynamics.
// ============================================================================
// The agent module encodes the mathematical core of CAT. Technological capacity
// is represented by E(t), residual tribalism by T(t), and collective
// coordination by C(t). Each state vector is evaluated from its initial
// condition at every tick. This avoids cumulative numerical integration error
// and keeps the model auditable against the closed-form equations.
//
// Numerical basis:
//
//   E(t) = initial_energy * exp(r * t)
//   T(t) = initial_tribalism * max(0, 1 - alpha * ln(1 + t))
//   C(t) = clamp(initial_collectivism + delta * t, 0, 1)
//
// The energy exponent is capped before exp() and the resulting energy is capped
// again by ENERGY_ABS_MAX. The logarithm domain is safe for all t >= 0 because
// ln(1 + t) receives an argument of at least 1. Collectivism is bounded to the
// closed interval [0, 1]. These constraints are model invariants rather than
// user-interface conveniences.
//
// Data layout:
//
// The Agent struct uses #[repr(C)] and hot-field ordering. The fields read or
// written by tick() are placed first so the parallel update phase touches a
// small and predictable memory region. Position and UUID are cold fields used
// mainly by the spatial index and exporters.
// ============================================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Exponent argument cap; exp(50) is already far beyond modeled thresholds.
/// Any civilization with energy this far beyond E_critical has already
/// collapsed or transcended. Prevents f64 overflow without altering dynamics.
const MAX_EXPONENT: f64 = 50.0;

/// Hard energy ceiling independent of other parameters.
/// Ensures f64 remains finite; values beyond this are physically irrelevant.
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
    /// External catastrophe such as a gamma-ray burst, impact, or stellar event.
    ExogenousExtinction,
}

/// Explicit 1-byte discriminant. Without #[repr(u8)], repr(Rust) for a
/// unit-variant enum is u8 in practice but not guaranteed by spec. Making
/// it explicit removes a layout assumption from the hot-path struct.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
pub enum AgentState {
    Nascent = 0,
    Evolving = 1,
    Transcended = 2,
    Collapsed = 3,
}

/// A civilization. State vectors: E (energy), T (tribalism), C (collectivism).
///
/// # Equations (evaluated DIRECTLY from initial conditions each tick):
///
/// E(t) = min(E0 * exp(r * t), ENERGY_ABS_MAX) - overflow-safe exponential
/// T(t) = T0 * max(0, 1 - alpha * ln(1 + t)) - domain-safe logarithmic decay
/// C(t) = clamp(C0 + delta * t, 0, 1) - linear drift, direct formula
///
/// See module-level comment for full numerical stability analysis.
///
/// # Memory layout
///
/// #[repr(C)] with explicit field ordering. tick() accesses only cache lines
/// 0 through 1 (offsets 0 through 88). Cold fields support indexing/export.
/// See module-level DOD section for the full layout diagram.
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    // Hot cache line 0: initial conditions and growth parameters.
    // All 8 fields read by tick() on every invocation, in this order.
    pub initial_energy: f64,
    pub initial_tribalism: f64,
    pub initial_collectivism: f64,
    pub energy_growth_rate: f64,
    pub tribalism_decay_alpha: f64,
    pub collectivism_drift: f64,
    pub energy: f64,
    pub tribalism: f64,

    // Hot cache line 1: mutable state and clock.
    pub collectivism: f64,         // offset 64
    pub ticks_since_ignition: u64, // offset 72
    pub birth_tick: u64,           // offset 80
    /// Explicit 1-byte discriminant via #[repr(u8)].
    /// repr(C) inserts 7 bytes of padding here before the next f64-aligned
    /// field. Placing state after the u64 run minimises wasted alignment space.
    pub state: AgentState, // offset 88 (1 byte) + 7B padding

    // Cold cache line 1 tail: spatial position and identity.
    // Loaded into L1 cache together with the hot fields but never written by
    // tick(). Only accessed by QuadTree::rebuild() and the Exporter.
    pub position: (f64, f64), // offset 96
    pub id: Uuid,             // offset 112
                              // Total struct size: 128 bytes = 2 full cache lines.
}

/// Thresholds governing collapse, transcendence, and external extinction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollapseThresholds {
    /// Energy above which technology is existentially dangerous (~mid-Type-I).
    pub critical_energy: f64,
    /// Tribalism above which internal conflict weaponizes technology.
    pub survival_tribalism: f64,
    /// Collectivism above which hive-mind anomaly activates.
    pub hive_collectivism: f64,
    /// Per-tick probability of exogenous extinction (GRB, asteroid, etc.).
    pub exogenous_extinction_rate: f64,
    /// Max energy achievable without interstellar expansion.
    pub resource_ceiling: f64,
}

impl Default for CollapseThresholds {
    /// Return the baseline CAT thresholds used by CLI defaults and tests.
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
    /// Construct a civilization with explicit initial state vectors and rates.
    ///
    /// The constructor records initial values separately from mutable values so
    /// `tick()` can evaluate each equation from its analytical initial
    /// condition rather than integrating from the previous tick.
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
            // Hot cache line 0.
            initial_energy: energy,
            initial_tribalism: tribalism,
            initial_collectivism: collectivism,
            energy_growth_rate,
            tribalism_decay_alpha,
            collectivism_drift,
            energy,
            tribalism,
            // Hot cache line 1.
            collectivism,
            ticks_since_ignition: 0,
            birth_tick,
            state: AgentState::Nascent,
            // Cold
            position,
            id: Uuid::new_v4(),
        }
    }

    /// Verify that an agent's initial conditions and growth parameters are all
    /// finite and within physically valid ranges. Called once at spawn time and
    /// defensively at the top of tick() in debug builds.
    ///
    /// Returns false if any field is NaN, Inf, or violates a physical bound.
    /// The caller is responsible for neutralizing (destroying) a corrupt agent.
    #[inline(always)]
    pub fn state_vectors_valid(&self) -> bool {
        self.initial_energy.is_finite()
            && self.initial_energy >= 0.0
            && self.initial_tribalism.is_finite()
            && self.initial_tribalism >= 0.0
            && self.initial_tribalism <= 1.0
            && self.initial_collectivism.is_finite()
            && self.initial_collectivism >= 0.0
            && self.initial_collectivism <= 1.0
            && self.energy_growth_rate.is_finite()
            && self.energy_growth_rate >= 0.0
            && self.tribalism_decay_alpha.is_finite()
            && self.tribalism_decay_alpha >= 0.0
            && self.collectivism_drift.is_finite()
            && self.energy.is_finite()
            && self.energy >= 0.0
            && self.tribalism.is_finite()
            && self.tribalism >= 0.0
            && self.collectivism.is_finite()
            && self.collectivism >= 0.0
            && self.collectivism <= 1.0
    }

    /// Advance state vectors by one tick using direct analytical formulae.
    ///
    /// Energy follows E(t) = E0 * exp(r * t), tribalism follows
    /// T(t) = T0 * max(0, 1 - alpha * ln(1 + t)), and collectivism follows
    /// C(t) = clamp(C0 + delta * t, 0, 1). Direct evaluation avoids cumulative
    /// integration drift and makes each tick independently auditable.
    #[inline(always)]
    pub fn tick(&mut self, thresholds: &CollapseThresholds) {
        if self.state == AgentState::Collapsed || self.state == AgentState::Transcended {
            return;
        }

        // NaN/Inf guard for externally corrupted state.
        // In release builds this is a single branch; in debug it also logs.
        // If any vector is invalid, the agent is neutralised immediately.
        // This guards against corrupt deserialization or future inter-agent
        // mutation that inadvertently writes NaN into a neighbour's fields.
        if !self.state_vectors_valid() {
            log::error!(
                "Agent {} has corrupt state at tick {} (E={} T={} C={}). Neutralising.",
                self.id,
                self.birth_tick + self.ticks_since_ignition,
                self.energy,
                self.tribalism,
                self.collectivism,
            );
            self.state = AgentState::Collapsed;
            return;
        }

        self.ticks_since_ignition += 1;
        let t = self.ticks_since_ignition as f64;

        // Energy: E(t) = E0 * exp(r * t), with overflow prevention.
        // Exponent clamped at MAX_EXPONENT=50, far beyond
        // any physically meaningful collapse threshold. No f64 overflow possible.
        let exponent = (self.energy_growth_rate * t).min(MAX_EXPONENT);
        let raw_energy = (self.initial_energy * exponent.exp()).min(ENERGY_ABS_MAX);

        // Resource ceiling: tribal psychology blocks interstellar expansion.
        self.energy = if self.tribalism > thresholds.survival_tribalism {
            raw_energy.min(thresholds.resource_ceiling)
        } else {
            raw_energy
        };

        // Tribalism: T(t) = T0 * max(0, 1 - alpha * ln(1 + t)).
        // ln(1 + t) is safe for all t >= 0 because the argument is at least 1.
        // clamp prevents negative T. The formula evaluates from initial_tribalism,
        // so T in [0, initial_tribalism] is guaranteed by construction.
        let maturity = (1.0 - self.tribalism_decay_alpha * (1.0 + t).ln()).max(0.0);
        self.tribalism = self.initial_tribalism * maturity;

        // Hard physical bound: T must never exceed initial_tribalism.
        // Guaranteed analytically because maturity <= 1, but enforced as
        // a defence against future changes to the maturity formula.
        debug_assert!(
            self.tribalism <= self.initial_tribalism + f64::EPSILON,
            "Tribalism invariant violated: T({}) > T0({})",
            self.tribalism,
            self.initial_tribalism
        );

        // Collectivism: C(t) = clamp(C0 + delta * t, 0, 1).
        // Direct evaluation from initial_collectivism eliminates accumulated
        // floating-point rounding error over 100k+ tick runs.
        self.collectivism =
            (self.initial_collectivism + self.collectivism_drift * t).clamp(0.0, 1.0);

        // Post-computation sanity checks for debug builds.
        debug_assert!(
            self.energy.is_finite() && self.energy >= 0.0,
            "Energy NaN/Inf after tick: {}",
            self.energy
        );
        debug_assert!(
            self.tribalism.is_finite() && self.tribalism >= 0.0,
            "Tribalism NaN/Inf after tick: {}",
            self.tribalism
        );
        debug_assert!(
            self.collectivism >= 0.0 && self.collectivism <= 1.0,
            "Collectivism out of [0,1]: {}",
            self.collectivism
        );

        // State transition: Nascent to Evolving at technological ignition.
        if self.state == AgentState::Nascent && self.energy > 0.5 {
            self.state = AgentState::Evolving;
            log::info!(
                "Agent {} ignited at tick {}.",
                self.id,
                self.birth_tick + self.ticks_since_ignition
            );
        }
        // influence_radius is NOT computed here. It is a pure function of
        // self.energy, derived on demand via influence_radius(). Removing
        // this store saves one sqrt + multiply per active agent per tick.
    }

    /// Influence radius derived from current energy.
    ///
    /// This is a COMPUTED property, not a stored field. Saves 8 bytes per
    /// agent and eliminates one redundant store per tick.
    /// R = sqrt(E) * k where k = 0.1, the interaction scale factor.
    #[inline(always)]
    pub fn influence_radius(&self) -> f64 {
        // energy is capped at ENERGY_ABS_MAX = 1000, so sqrt is always finite.
        self.energy.sqrt() * 0.1
    }

    /// Collapse predicate: E > E_crit AND T > T_surv AND C < C_hive.
    #[inline(always)]
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
    #[inline(always)]
    pub fn evaluate_transcendence(&self, thresholds: &CollapseThresholds) -> bool {
        if self.state == AgentState::Collapsed || self.state == AgentState::Transcended {
            return false;
        }
        self.collectivism >= thresholds.hive_collectivism
            && self.energy > thresholds.critical_energy * 0.5
            && self.tribalism < thresholds.survival_tribalism * 0.5
    }

    /// Mark the civilization as collapsed after a predicate or guard failure.
    pub fn destroy(&mut self) {
        self.state = AgentState::Collapsed;
        log::warn!(
            "Agent {} collapsed. E={:.4} T={:.4} C={:.4}",
            self.id,
            self.energy,
            self.tribalism,
            self.collectivism
        );
    }

    /// Mark the civilization as a high-coordination survivor.
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

    /// Returns true only for states that participate in the tick loop.
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        matches!(self.state, AgentState::Nascent | AgentState::Evolving)
    }

    /// Civilizational Stress Index: G = E / (1 − T + ε).
    ///
    /// G is a dimensionless measure of the tension between a civilization's
    /// destructive technological capacity (E) and its remaining coordination
    /// safety margin (1 − T). It is the core diagnostic metric of the
    /// Asynchronous Gap hypothesis.
    ///
    /// # Physical interpretation
    ///
    /// - **Numerator E**: total energy budget. At E = E_critical, the civilization
    ///   holds enough capacity for existential-scale destruction or transformation.
    /// - **Denominator (1 − T)**: coordination safety margin. At T = 0 (no
    ///   tribalism) the margin is 1.0 — full governance bandwidth. At T = 1.0
    ///   (absolute tribalism) the margin collapses to ε, leaving no coordination
    ///   runway against the energy capacity in the numerator.
    /// - **G → ∞ analogue**: a civilization with E = 2.0 and T = 1.0 yields
    ///   G = 2.0 / 1e-10 = 2 × 10¹⁰ — correctly signaling maximal filter stress
    ///   without producing IEEE 754 Inf.
    ///
    /// # Role of ε = 1e-10
    ///
    /// ε is a model invariant, not a numerical approximation. T = 1.0 is a
    /// physically achievable transient state (a fully tribal civilization).
    /// The stress index must remain a finite f64 at this boundary to preserve
    /// the integrity of downstream statistics (Kahan sums, mean_async_gap).
    /// ε = 1e-10 is small enough to produce the correct qualitative signal
    /// (astronomically high stress) while remaining representable as a normal f64.
    #[inline(always)]
    pub fn asynchronous_gap(&self) -> f64 {
        self.energy / (1.0 - self.tribalism + 1e-10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Return default thresholds for agent unit tests.
    fn thresholds() -> CollapseThresholds {
        CollapseThresholds::default()
    }

    /// Verify the hot-field layout remains bounded to two cache lines.
    #[test]
    fn test_agent_struct_size() {
        // With #[repr(C)] and the declared field order, the struct is
        // 128 bytes = exactly 2 cache lines. Verify it hasn't grown.
        // If this fails, a field was added without reviewing cache impact.
        let size = std::mem::size_of::<Agent>();
        assert!(
            size <= 128,
            "Agent struct grew beyond 2 cache lines: {} bytes. \
             Review new field placement for cache-locality impact.",
            size
        );
    }

    /// Verify the compact state discriminant used by the data layout.
    #[test]
    fn test_agent_state_repr() {
        // #[repr(u8)] ensures the discriminant is exactly 1 byte.
        assert_eq!(std::mem::size_of::<AgentState>(), 1);
    }

    /// Verify constructor initialization and active-state semantics.
    #[test]
    fn test_agent_creation() {
        let a = Agent::new((10.0, 20.0), 0.1, 0.9, 0.1, 0.01, 0.005, 0.001, 0);
        assert_eq!(a.state, AgentState::Nascent);
        assert!(a.is_active());
        assert_eq!(a.initial_energy, 0.1);
        assert_eq!(a.initial_tribalism, 0.9);
    }

    /// Verify the exponential energy equation against an analytical value.
    #[test]
    fn test_exponential_energy_growth() {
        let mut a = Agent::new((0.0, 0.0), 1.0, 0.3, 0.5, 0.1, 0.001, 0.0, 0);
        let t = thresholds();
        for _ in 0..10 {
            a.tick(&t);
        }
        // E(10) = 1.0 * exp(0.1 * 10) = exp(1) ~= 2.718.
        let expected = (0.1_f64 * 10.0).exp();
        assert!((a.energy - expected).abs() < 1e-6);
    }

    /// Verify long-run energy remains finite under exponent capping.
    #[test]
    fn test_no_overflow_long_run() {
        // r=0.05 and 100,000 ticks gives exponent 5000, clamped to 50.
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

    /// Verify direct tribalism evaluation does not accumulate drift.
    #[test]
    fn test_tribalism_direct_formula() {
        let mut a = Agent::new((0.0, 0.0), 0.1, 0.9, 0.1, 0.001, 0.01, 0.0, 0);
        let t = thresholds();
        for _ in 0..9_901 {
            a.tick(&t);
        }
        // T(9901) = 0.9 * max(0, 1 - 0.01 * ln(9902))
        let expected = 0.9 * (1.0 - 0.01 * (9902.0_f64).ln()).max(0.0);
        assert!(
            (a.tribalism - expected).abs() < 1e-10,
            "Tribalism drift detected: expected {:.10}, got {:.10}",
            expected,
            a.tribalism
        );
        assert!(a.tribalism.is_finite() && a.tribalism >= 0.0);
    }

    /// Verify the logarithmic decay clamp prevents negative tribalism.
    #[test]
    fn test_tribalism_no_negative_domain() {
        // ln(1 + t) is nonnegative for t >= 0; max(0) prevents negative T.
        let mut a = Agent::new((0.0, 0.0), 0.1, 0.9, 0.1, 0.001, 0.1, 0.0, 0);
        let t = thresholds();
        for _ in 0..1_000 {
            a.tick(&t);
            assert!(a.tribalism.is_finite(), "Tribalism became non-finite");
            assert!(
                a.tribalism >= 0.0,
                "Tribalism went negative: {}",
                a.tribalism
            );
        }
    }

    /// Verify collectivism clamps at both endpoints under positive and negative drift.
    #[test]
    fn test_collectivism_bifurcation_boundary() {
        // Positive drift: C must reach 1.0 and stay clamped.
        let mut a = Agent::new((0.0, 0.0), 0.1, 0.5, 0.2, 0.001, 0.005, 0.001, 0);
        let t = thresholds();
        // Reaches C=1 at tick (1.0 - 0.2) / 0.001 = 800
        for _ in 0..2_000 {
            a.tick(&t);
        }
        assert_eq!(a.collectivism, 1.0, "Positive drift must clamp at C=1.0");

        // Negative drift: C must reach 0.0 and stay clamped.
        let mut b = Agent::new((0.0, 0.0), 0.1, 0.5, 0.3, 0.001, 0.005, -0.001, 0);
        for _ in 0..2_000 {
            b.tick(&t);
        }
        assert_eq!(b.collectivism, 0.0, "Negative drift must clamp at C=0.0");
    }

    /// Verify influence radius is derived from current energy.
    #[test]
    fn test_influence_radius_is_computed() {
        // influence_radius() must equal sqrt(energy) * 0.1, always finite.
        let a = Agent::new((0.0, 0.0), 4.0, 0.5, 0.3, 0.01, 0.001, 0.0, 0);
        let expected = 4.0_f64.sqrt() * 0.1; // 0.2
        assert!((a.influence_radius() - expected).abs() < 1e-12);
        assert!(a.influence_radius().is_finite());
    }

    /// Verify the three-condition Asynchronous Gap collapse predicate.
    #[test]
    fn test_asynchronous_gap_collapse() {
        let a = Agent::new((5.0, 5.0), 3.0, 0.8, 0.1, 0.01, 0.001, 0.0, 0);
        let c = a.evaluate_collapse(100, &thresholds());
        assert!(c.is_some());
        assert_eq!(c.unwrap().collapse_type, CollapseType::AsynchronousGap);
    }

    /// Verify high-collectivism agents can satisfy transcendence.
    #[test]
    fn test_hive_mind_transcendence() {
        let a = Agent::new((0.0, 0.0), 2.0, 0.2, 0.95, 0.001, 0.01, 0.001, 0);
        assert!(a.evaluate_transcendence(&thresholds()));
    }

    /// Verify high collectivism blocks the internal-collapse predicate.
    #[test]
    fn test_high_c_prevents_collapse() {
        let a = Agent::new((0.0, 0.0), 3.0, 0.8, 0.9, 0.01, 0.001, 0.0, 0);
        assert!(a.evaluate_collapse(100, &thresholds()).is_none());
    }

    /// Verify destroy() removes an agent from active participation.
    #[test]
    fn test_destroy() {
        let mut a = Agent::new((0.0, 0.0), 1.0, 0.5, 0.3, 0.01, 0.001, 0.0, 0);
        a.destroy();
        assert_eq!(a.state, AgentState::Collapsed);
        assert!(!a.is_active());
    }

    /// Verify the gap metric remains finite when T equals 1.0.
    #[test]
    fn test_gap_metric_finite_at_t1() {
        // When T=1.0, denominator = 1e-10. Must not produce Inf.
        let a = Agent::new((0.0, 0.0), 2.0, 1.0, 0.3, 0.01, 0.001, 0.0, 0);
        let gap = a.asynchronous_gap();
        assert!(
            gap.is_finite(),
            "Gap metric must be finite when T=1.0: got {}",
            gap
        );
    }
}
