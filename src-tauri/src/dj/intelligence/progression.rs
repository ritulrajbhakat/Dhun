//! Progression Intelligence, Adaptive Thresholds & Hysteresis Engine (Stage 2 - Corrected)
//!
//! Provides domain-agnostic progression state management, anti-chatter hysteresis,
//! and meaningful region integration:
//! - Dual-threshold hysteresis: \theta_{enter} and \theta_{exit} prevent limit-cycle chatter
//! - Continuous-time leaky integrator baseline tracking with candidate transition protection
//! - Cold-start initial condition policy for signals already active at observation onset
//! - Temporal persistence debounce window with retroactive onset and mass accounting
//! - 8-State Progression Machine:
//!     1. BelowSignificance: Signal below entry threshold, quiescent
//!     2. Building: Rising above significance with positive velocity
//!     3. Peaking: Cresting at local peak, velocity zero-crossing with negative curvature
//!     4. Sustained: Elevated intensity maintained with low drift for \tau_{sustain}
//!     5. Subduing: Declining from peak/sustain toward baseline, negative velocity
//!     6. Subsided: Dropped below exit threshold, confirmed across persistence window
//!     7. Recovering: Direction reversal (v > 0) during decline before resetting to baseline
//!     8. Stable: Quiescent baseline state with minimal volatility and drift
//! - Meaningful Region Integration with trapezoidal mass and empirical quality heuristic.
//!
//! Domain-agnostic: Operates on any scalar kinematic time series.

use serde::{Deserialize, Serialize};
use super::kinematics::{Direction, KinematicError, KinematicMetrics};

/// Eight discrete states of signal progression lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgressionState {
    /// Signal value is below significance threshold; no active progression.
    BelowSignificance,
    /// Signal has crossed entry threshold with positive velocity; developing intensity.
    Building,
    /// Signal is cresting at a local maximum; velocity crossing zero with negative acceleration.
    Peaking,
    /// Signal remains elevated above threshold with minimal drift for a sustained duration.
    Sustained,
    /// Signal is declining from a peak/sustained state toward baseline; negative velocity.
    Subduing,
    /// Signal has dropped below the exit threshold and satisfied exit persistence.
    Subsided,
    /// Signal reversed direction to ascend again after subduing, without returning to baseline.
    Recovering,
    /// Signal is quiescent and stable at baseline with negligible derivatives.
    Stable,
}

impl ProgressionState {
    /// Whether this state represents active significance above baseline.
    pub fn is_active_progression(&self) -> bool {
        matches!(
            self,
            ProgressionState::Building
                | ProgressionState::Peaking
                | ProgressionState::Sustained
                | ProgressionState::Subduing
                | ProgressionState::Recovering
        )
    }

    /// Whether this state represents peak or sustained interest.
    pub fn is_peak_or_sustained(&self) -> bool {
        matches!(self, ProgressionState::Peaking | ProgressionState::Sustained)
    }
}

/// Policy for handling signals that are already elevated upon cold-start initial observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InitialConditionPolicy {
    /// If initial observation magnitude is >= theta_enter, immediately evaluate and initiate active progression.
    ColdStartActive,
    /// Always initialize in BelowSignificance, requiring an ascending transition to enter progression.
    StrictAscentOnly,
}

/// Errors occurring during progression analysis.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProgressionError {
    #[error("Kinematic error: {0}")]
    Kinematics(#[from] KinematicError),
    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },
}

/// Adaptive threshold parameters governing state transitions and hysteresis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveThresholdConfig {
    /// Fixed base threshold for entry (e.g. 0.50 for normalized [0, 1] signals).
    pub base_entry_threshold: f64,
    /// Multiplier k for statistical adaptation: \theta_{enter} = max(base, \mu_b + k * \sigma_b).
    pub k_sigma: f64,
    /// Hysteresis margin ratio: \theta_{exit} = \theta_{enter} * (1.0 - hysteresis_ratio).
    /// Typically 0.15 (15% drop below entry threshold required to exit).
    pub hysteresis_ratio: f64,
    /// Continuous-time baseline leaky integrator time constant in milliseconds. Default: 10,000 ms.
    pub baseline_tau_ms: f64,
    /// Minimum time in milliseconds a condition must persist before confirming state transition.
    pub min_confirmation_ms: i64,
    /// Minimum duration in milliseconds in plateau before transitioning to Sustained state.
    pub min_sustain_ms: i64,
    /// Velocity threshold above which signal is considered Building (units / sec).
    pub building_velocity_threshold: f64,
    /// Negative velocity threshold below which signal is considered Subduing (units / sec).
    pub subduing_velocity_threshold: f64,
    /// Maximum velocity drift allowed in Sustained or Peaking state (units / sec).
    pub sustain_drift_threshold: f64,
    /// Positive velocity required to transition from Subduing to Recovering (units / sec).
    pub recovery_velocity_threshold: f64,
    /// Relative scale for meaningful region mass normalization. Default: 1.0.
    pub nominal_mass_scale: f64,
    /// Reference saturation duration for meaningful regions in seconds. Default: 5.0.
    pub nominal_duration_sec: f64,
    /// Policy for initial observation state.
    pub initial_condition_policy: InitialConditionPolicy,
}

impl Default for AdaptiveThresholdConfig {
    fn default() -> Self {
        Self {
            base_entry_threshold: 0.50,
            k_sigma: 0.5,
            hysteresis_ratio: 0.15,      // 15% hysteresis band
            baseline_tau_ms: 10_000.0,   // 10s continuous-time baseline memory
            min_confirmation_ms: 300,    // 300ms persistence debounce
            min_sustain_ms: 1500,        // 1.5s plateau for Sustained state
            building_velocity_threshold: 0.005,
            subduing_velocity_threshold: -0.005,
            sustain_drift_threshold: 0.020,
            recovery_velocity_threshold: 0.010,
            nominal_mass_scale: 1.0,
            nominal_duration_sec: 5.0,
            initial_condition_policy: InitialConditionPolicy::ColdStartActive,
        }
    }
}

impl AdaptiveThresholdConfig {
    /// Validate configuration invariants.
    pub fn validate(&self) -> Result<(), ProgressionError> {
        if !self.base_entry_threshold.is_finite() {
            return Err(ProgressionError::InvalidConfig {
                reason: "base_entry_threshold must be finite".to_string(),
            });
        }
        if self.hysteresis_ratio <= 0.0 || self.hysteresis_ratio >= 1.0 || !self.hysteresis_ratio.is_finite() {
            return Err(ProgressionError::InvalidConfig {
                reason: "hysteresis_ratio must be strictly between 0.0 and 1.0".to_string(),
            });
        }
        if self.baseline_tau_ms <= 0.0 || !self.baseline_tau_ms.is_finite() {
            return Err(ProgressionError::InvalidConfig {
                reason: "baseline_tau_ms must be positive and finite".to_string(),
            });
        }
        if self.min_confirmation_ms < 0 {
            return Err(ProgressionError::InvalidConfig {
                reason: "min_confirmation_ms must be non-negative".to_string(),
            });
        }
        if self.min_sustain_ms < 0 {
            return Err(ProgressionError::InvalidConfig {
                reason: "min_sustain_ms must be non-negative".to_string(),
            });
        }
        if self.nominal_mass_scale <= 0.0 || !self.nominal_mass_scale.is_finite() {
            return Err(ProgressionError::InvalidConfig {
                reason: "nominal_mass_scale must be positive and finite".to_string(),
            });
        }
        if self.nominal_duration_sec <= 0.0 || !self.nominal_duration_sec.is_finite() {
            return Err(ProgressionError::InvalidConfig {
                reason: "nominal_duration_sec must be positive and finite".to_string(),
            });
        }
        Ok(())
    }
}

/// An integrated meaningful region representing an extended episode of signal significance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeaningfulRegion {
    /// Unique index of the region in the sequence.
    pub region_id: usize,
    /// Timestamp (ms) where progression began (entry threshold confirmed, retroactively exact).
    pub onset_ms: i64,
    /// Timestamp (ms) of the highest recorded magnitude in the region.
    pub peak_ms: i64,
    /// Timestamp (ms) where progression subsided below exit threshold.
    pub offset_ms: i64,
    /// Total duration of the region in milliseconds (offset - onset).
    pub duration_ms: i64,
    /// Total time spent in Sustained or Peaking state (milliseconds).
    pub sustained_duration_ms: i64,
    /// Peak smoothed magnitude achieved.
    pub peak_magnitude: f64,
    /// Time-weighted average magnitude across the region.
    pub mean_magnitude: f64,
    /// Integrated mass: area under the curve above exit threshold (magnitude * seconds).
    pub integral_mass: f64,
    /// Progression state at the conclusion or current evaluation.
    pub progression_state: ProgressionState,
    /// Empirical quality heuristic score [0.0, 1.0] derived from mass, duration, and SNR.
    /// Note: This is an empirical heuristic index, not a frequentist probability.
    pub quality_score: f64,
    /// Whether the region has finished (subsided) or is still actively evolving.
    pub is_complete: bool,
}

impl MeaningfulRegion {
    /// Backwards-compatible alias for quality_score.
    #[inline]
    pub fn confidence(&self) -> f64 {
        self.quality_score
    }
}

/// Snapshot of progression status for a single observation point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProgressionPoint {
    /// Timestamp in milliseconds.
    pub timestamp_ms: i64,
    /// Current progression state.
    pub state: ProgressionState,
    /// Effective entry threshold \theta_{enter} applied at this time.
    pub theta_enter: f64,
    /// Effective exit threshold \theta_{exit} applied at this time.
    pub theta_exit: f64,
    /// Underlying kinematic metrics.
    pub kinematics: KinematicMetrics,
}

/// State machine tracking signal progression and extracting meaningful regions in streaming context.
#[derive(Debug, Clone)]
pub struct ProgressionStateMachine {
    config: AdaptiveThresholdConfig,
    current_state: ProgressionState,
    state_entry_ms: i64,
    last_processed_ms: i64,
    candidate_state: Option<(ProgressionState, i64)>, // (candidate, candidate_onset_ms)
    candidate_buffer: Vec<KinematicMetrics>,          // buffered points during confirmation debounce
    active_region: Option<ActiveRegionBuilder>,
    completed_regions: Vec<MeaningfulRegion>,
    region_counter: usize,
    baseline_mean: f64,
    baseline_volatility: f64,
    last_baseline_update_ms: Option<i64>,
    is_initial_point: bool,
}

#[derive(Debug, Clone)]
struct ActiveRegionBuilder {
    region_id: usize,
    onset_ms: i64,
    peak_ms: i64,
    peak_magnitude: f64,
    accumulated_magnitude_dt: f64,
    accumulated_mass: f64,
    sustained_duration_ms: i64,
    last_point: Option<KinematicMetrics>,
    last_state: ProgressionState,
    snr_sum: f64,
    sample_count: usize,
}

impl ProgressionStateMachine {
    /// Create a new state machine with validated configuration.
    pub fn new(config: AdaptiveThresholdConfig) -> Result<Self, ProgressionError> {
        config.validate()?;
        let initial_baseline = config.base_entry_threshold * 0.5;
        Ok(Self {
            config,
            current_state: ProgressionState::BelowSignificance,
            state_entry_ms: 0,
            last_processed_ms: 0,
            candidate_state: None,
            candidate_buffer: Vec::new(),
            active_region: None,
            completed_regions: Vec::new(),
            region_counter: 0,
            baseline_mean: initial_baseline,
            baseline_volatility: 0.0,
            last_baseline_update_ms: None,
            is_initial_point: true,
        })
    }

    /// Reset the state machine to initial quiescent state.
    pub fn reset(&mut self) {
        let initial_baseline = self.config.base_entry_threshold * 0.5;
        self.current_state = ProgressionState::BelowSignificance;
        self.state_entry_ms = 0;
        self.last_processed_ms = 0;
        self.candidate_state = None;
        self.candidate_buffer.clear();
        self.active_region = None;
        self.completed_regions.clear();
        self.region_counter = 0;
        self.baseline_mean = initial_baseline;
        self.baseline_volatility = 0.0;
        self.last_baseline_update_ms = None;
        self.is_initial_point = true;
    }

    /// Current progression state.
    pub fn current_state(&self) -> ProgressionState {
        self.current_state
    }

    /// Completed meaningful regions.
    pub fn completed_regions(&self) -> &[MeaningfulRegion] {
        &self.completed_regions
    }

    /// Compute dynamic thresholds given quiescent baseline statistics.
    pub fn compute_thresholds(&self, _metrics: &KinematicMetrics) -> (f64, f64) {
        let adaptive_component = self.baseline_mean + self.config.k_sigma * self.baseline_volatility;
        let theta_enter = self
            .config
            .base_entry_threshold
            .max(adaptive_component);

        let hysteresis_band = theta_enter * self.config.hysteresis_ratio;
        let theta_exit = (theta_enter - hysteresis_band).max(0.0);

        (theta_enter, theta_exit)
    }

    /// Feed a kinematic measurement and update progression state and active regions.
    pub fn update(&mut self, metrics: &KinematicMetrics) -> ProgressionPoint {
        let t = metrics.timestamp_ms;
        let (theta_enter, theta_exit) = self.compute_thresholds(metrics);

        // Check for cold-start initial active condition
        if self.is_initial_point {
            self.is_initial_point = false;
            self.last_baseline_update_ms = Some(t);
            self.baseline_mean = metrics.magnitude;
            self.baseline_volatility = metrics.volatility;

            if self.config.initial_condition_policy == InitialConditionPolicy::ColdStartActive
                && metrics.magnitude >= theta_enter
            {
                // Cold-start active signal handling:
                let initial_state = if metrics.velocity > self.config.building_velocity_threshold {
                    ProgressionState::Building
                } else if metrics.velocity < self.config.subduing_velocity_threshold {
                    ProgressionState::Subduing
                } else {
                    ProgressionState::Sustained
                };

                self.current_state = initial_state;
                self.state_entry_ms = t;
                self.region_counter += 1;
                self.active_region = Some(ActiveRegionBuilder {
                    region_id: self.region_counter,
                    onset_ms: t,
                    peak_ms: t,
                    peak_magnitude: metrics.magnitude,
                    accumulated_magnitude_dt: 0.0,
                    accumulated_mass: 0.0,
                    sustained_duration_ms: 0,
                    last_point: Some(*metrics),
                    last_state: initial_state,
                    snr_sum: metrics.snr,
                    sample_count: 1,
                });

                self.last_processed_ms = t;
                return ProgressionPoint {
                    timestamp_ms: t,
                    state: self.current_state,
                    theta_enter,
                    theta_exit,
                    kinematics: *metrics,
                };
            }
        }

        // Candidate Protection: Update baseline ONLY when in quiescent state AND no active candidate transition is pending
        let is_candidate_active = self
            .candidate_state
            .map(|(s, _)| s.is_active_progression())
            .unwrap_or(false);

        if matches!(
            self.current_state,
            ProgressionState::BelowSignificance | ProgressionState::Stable
        ) && !is_candidate_active
        {
            let dt_ms = (t - self.last_baseline_update_ms.unwrap_or(t)).max(0);
            let tau_ms = self.config.baseline_tau_ms.max(100.0);
            let alpha = 1.0 - (-(dt_ms as f64) / tau_ms).exp();

            self.baseline_mean = (1.0 - alpha) * self.baseline_mean + alpha * metrics.magnitude;
            self.baseline_volatility = (1.0 - alpha) * self.baseline_volatility + alpha * metrics.volatility;
            self.last_baseline_update_ms = Some(t);
        }

        // Determine raw candidate state
        let raw_next = self.evaluate_next_state(metrics, theta_enter, theta_exit);

        // Apply temporal debounce persistence with retroactive candidate onset tracking
        let confirmed_transition = self.update_state_transition(raw_next, metrics);

        // Update active meaningful region
        self.update_meaningful_regions(metrics, theta_exit, confirmed_transition);

        self.last_processed_ms = t;

        ProgressionPoint {
            timestamp_ms: t,
            state: self.current_state,
            theta_enter,
            theta_exit,
            kinematics: *metrics,
        }
    }

    /// Evaluate target state based on kinematics, current state, and dual-threshold hysteresis.
    fn evaluate_next_state(
        &self,
        m: &KinematicMetrics,
        theta_enter: f64,
        theta_exit: f64,
    ) -> ProgressionState {
        let val = m.magnitude;
        let v = m.velocity;
        let a = m.acceleration;

        match self.current_state {
            ProgressionState::BelowSignificance | ProgressionState::Stable => {
                if val >= theta_enter && v >= self.config.building_velocity_threshold {
                    ProgressionState::Building
                } else if val < theta_exit && m.volatility < 0.01 && v.abs() < 0.005 {
                    ProgressionState::Stable
                } else {
                    ProgressionState::BelowSignificance
                }
            }

            ProgressionState::Building => {
                if val < theta_exit {
                    ProgressionState::Subsided
                } else if v < self.config.subduing_velocity_threshold {
                    ProgressionState::Subduing
                } else if v.abs() <= self.config.sustain_drift_threshold && a <= 0.0 {
                    ProgressionState::Peaking
                } else {
                    ProgressionState::Building
                }
            }

            ProgressionState::Peaking => {
                let time_in_crest = m.timestamp_ms - self.state_entry_ms;
                if val < theta_exit {
                    ProgressionState::Subsided
                } else if v < self.config.subduing_velocity_threshold {
                    ProgressionState::Subduing
                } else if time_in_crest >= self.config.min_sustain_ms
                    && v.abs() <= self.config.sustain_drift_threshold
                {
                    ProgressionState::Sustained
                } else {
                    ProgressionState::Peaking
                }
            }

            ProgressionState::Sustained => {
                if val < theta_exit {
                    ProgressionState::Subsided
                } else if v < self.config.subduing_velocity_threshold {
                    ProgressionState::Subduing
                } else if v > self.config.building_velocity_threshold {
                    ProgressionState::Building
                } else {
                    ProgressionState::Sustained
                }
            }

            ProgressionState::Subduing => {
                if val < theta_exit {
                    ProgressionState::Subsided
                } else if v >= self.config.recovery_velocity_threshold {
                    ProgressionState::Recovering
                } else {
                    ProgressionState::Subduing
                }
            }

            ProgressionState::Recovering => {
                if val < theta_exit {
                    ProgressionState::Subsided
                } else if v >= self.config.building_velocity_threshold && val >= theta_enter {
                    ProgressionState::Building
                } else if v < self.config.subduing_velocity_threshold {
                    ProgressionState::Subduing
                } else {
                    ProgressionState::Recovering
                }
            }

            ProgressionState::Subsided => {
                if val >= theta_enter && v >= self.config.building_velocity_threshold {
                    ProgressionState::Building
                } else if val < theta_exit && m.volatility < 0.01 && v.abs() < 0.005 {
                    ProgressionState::Stable
                } else {
                    ProgressionState::BelowSignificance
                }
            }
        }
    }

    /// Process debounce confirmation window for state transition.
    /// Returns Some((new_state, original_candidate_timestamp)) when a transition commits.
    fn update_state_transition(
        &mut self,
        raw_target: ProgressionState,
        m: &KinematicMetrics,
    ) -> Option<(ProgressionState, i64)> {
        let t = m.timestamp_ms;
        if raw_target == self.current_state {
            self.candidate_state = None;
            self.candidate_buffer.clear();
            return None;
        }

        match self.candidate_state {
            Some((cand, cand_t)) if cand == raw_target => {
                self.candidate_buffer.push(*m);
                let elapsed = t - cand_t;
                if elapsed >= self.config.min_confirmation_ms {
                    // Commit transition
                    self.current_state = raw_target;
                    self.state_entry_ms = t;
                    self.candidate_state = None;
                    Some((raw_target, cand_t))
                } else {
                    None
                }
            }
            _ => {
                // Initialize candidate state and buffer
                self.candidate_state = Some((raw_target, t));
                self.candidate_buffer.clear();
                self.candidate_buffer.push(*m);
                None
            }
        }
    }

    /// Update accumulation in active meaningful region or finalize it.
    fn update_meaningful_regions(
        &mut self,
        m: &KinematicMetrics,
        theta_exit: f64,
        confirmed_transition: Option<(ProgressionState, i64)>,
    ) {
        let is_active = self.current_state.is_active_progression();

        if is_active {
            if let Some((new_state, cand_t)) = confirmed_transition {
                if new_state == ProgressionState::Building && self.active_region.is_none() {
                    // Start a new meaningful region with RETROACTIVE onset at candidate timestamp
                    self.region_counter += 1;
                    let mut builder = ActiveRegionBuilder {
                        region_id: self.region_counter,
                        onset_ms: cand_t,
                        peak_ms: cand_t,
                        peak_magnitude: m.magnitude,
                        accumulated_magnitude_dt: 0.0,
                        accumulated_mass: 0.0,
                        sustained_duration_ms: 0,
                        last_point: None,
                        last_state: self.current_state,
                        snr_sum: 0.0,
                        sample_count: 0,
                    };

                    // Retroactively integrate all buffered points observed during the candidate confirmation window
                    for pt in &self.candidate_buffer {
                        accumulate_into_builder(&mut builder, pt, theta_exit, self.current_state);
                    }
                    self.candidate_buffer.clear();
                    self.active_region = Some(builder);
                    return;
                }
            }

            if let Some(builder) = &mut self.active_region {
                accumulate_into_builder(builder, m, theta_exit, self.current_state);
            }
        } else {
            // Signal is not in active progression. If we had an active region, finalize it.
            if let Some(builder) = self.active_region.take() {
                let offset_t = confirmed_transition.map(|(_, ct)| ct).unwrap_or(m.timestamp_ms);
                let region = finalize_region(builder, offset_t, false, &self.config);
                self.completed_regions.push(region);
            }
            if self.candidate_state.is_none() {
                self.candidate_buffer.clear();
            }
        }
    }

    /// Conclude the analysis stream and finalize any open region.
    pub fn finalize(&mut self) -> Vec<MeaningfulRegion> {
        if let Some(builder) = self.active_region.take() {
            let region = finalize_region(builder, self.last_processed_ms, true, &self.config);
            self.completed_regions.push(region);
        }
        self.completed_regions.clone()
    }
}

/// Accumulate a metric point into the active region builder.
fn accumulate_into_builder(
    builder: &mut ActiveRegionBuilder,
    m: &KinematicMetrics,
    theta_exit: f64,
    current_state: ProgressionState,
) {
    let dt_sec = if let Some(last) = builder.last_point {
        ((m.timestamp_ms - last.timestamp_ms) as f64 / 1000.0).max(0.0)
    } else {
        0.0
    };

    let dt_ms = (dt_sec * 1000.0) as i64;
    if current_state.is_peak_or_sustained() {
        builder.sustained_duration_ms += dt_ms;
    }

    builder.accumulated_magnitude_dt += m.magnitude * dt_sec;

    // Trapezoidal integral mass above exit threshold
    if let Some(last) = builder.last_point {
        let h1 = (last.magnitude - theta_exit).max(0.0);
        let h2 = (m.magnitude - theta_exit).max(0.0);
        let trap_area = 0.5 * (h1 + h2) * dt_sec;
        builder.accumulated_mass += trap_area;
    }

    if m.magnitude > builder.peak_magnitude {
        builder.peak_magnitude = m.magnitude;
        builder.peak_ms = m.timestamp_ms;
    }

    builder.last_point = Some(*m);
    builder.last_state = current_state;
    builder.snr_sum += m.snr;
    builder.sample_count += 1;
}

/// Helper to convert an active builder into a completed MeaningfulRegion.
fn finalize_region(
    builder: ActiveRegionBuilder,
    offset_ms: i64,
    is_end_of_stream: bool,
    config: &AdaptiveThresholdConfig,
) -> MeaningfulRegion {
    let duration_ms = (offset_ms - builder.onset_ms).max(0);
    let duration_sec = duration_ms as f64 / 1000.0;

    let mean_magnitude = if duration_sec > 0.0 {
        builder.accumulated_magnitude_dt / duration_sec
    } else {
        builder.peak_magnitude
    };

    let avg_snr = if builder.sample_count > 0 {
        builder.snr_sum / builder.sample_count as f64
    } else {
        0.0
    };

    let sustain_fraction = if duration_ms > 0 {
        (builder.sustained_duration_ms as f64 / duration_ms as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let mass_factor = (builder.accumulated_mass / (config.nominal_mass_scale + builder.accumulated_mass)).clamp(0.0, 1.0);
    let snr_factor = (avg_snr / (1.0 + avg_snr)).clamp(0.0, 1.0);
    let duration_factor = (duration_sec / config.nominal_duration_sec).clamp(0.0, 1.0);

    let quality_score = (sustain_fraction * 0.35 + mass_factor * 0.35 + duration_factor * 0.15 + snr_factor * 0.15)
        .clamp(0.0, 1.0);

    MeaningfulRegion {
        region_id: builder.region_id,
        onset_ms: builder.onset_ms,
        peak_ms: builder.peak_ms,
        offset_ms,
        duration_ms,
        sustained_duration_ms: builder.sustained_duration_ms,
        peak_magnitude: builder.peak_magnitude,
        mean_magnitude,
        integral_mass: builder.accumulated_mass,
        progression_state: builder.last_state,
        quality_score,
        is_complete: !is_end_of_stream,
    }
}

/// Complete batch progression analysis over a pre-computed sequence of kinematic metrics.
pub fn analyze_progression(
    metrics: &[KinematicMetrics],
    config: &AdaptiveThresholdConfig,
) -> Result<(Vec<ProgressionPoint>, Vec<MeaningfulRegion>), ProgressionError> {
    config.validate()?;
    let mut state_machine = ProgressionStateMachine::new(config.clone())?;
    let mut points = Vec::with_capacity(metrics.len());

    for m in metrics {
        let pt = state_machine.update(m);
        points.push(pt);
    }

    let regions = state_machine.finalize();
    Ok((points, regions))
}
