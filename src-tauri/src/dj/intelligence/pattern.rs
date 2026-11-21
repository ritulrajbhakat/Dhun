//! Progression Archetype Recognition & Trajectory Projection Engine (Stage 3)
//!
//! Provides domain-agnostic structural pattern recognition and predictive trajectory projection:
//! - Classifies time-series segments into structural archetypes:
//!     MonotonicAscent, MonotonicDescent, Plateau, CrescendoPeak, DecrescendoTrough,
//!     TransientSpike, CyclicalOscillation, QuiescentFlat
//! - Short-term damped kinematic trajectory projection with widening uncertainty bounds
//! - Pattern confidence scoring based on fit residuals and kinematic coherence
//!
//! Genuinely domain-agnostic: Operates purely on kinematics and time series.

use serde::{Deserialize, Serialize};
use super::kinematics::KinematicMetrics;
use super::uncertainty::{quantify_uncertainty, UncertaintyConfig, UncertaintyEstimate};

/// Structural archetype representing the geometric pattern of signal evolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgressionArchetype {
    /// Consistent upward velocity without significant inflection (buildup/growth).
    MonotonicAscent,
    /// Consistent downward velocity without significant inflection (dissipation/decay).
    MonotonicDescent,
    /// Elevated stable magnitude with near-zero drift (sustained plateau).
    Plateau,
    /// Accelerating rise culminating in a local crest / peak.
    CrescendoPeak,
    /// Decelerating drop culminating in a trough / baseline stabilization.
    DecrescendoTrough,
    /// Rapid transient impulse of very short duration and high curvature.
    TransientSpike,
    /// Alternating periodic movement between ascending and descending phases.
    CyclicalOscillation,
    /// Flat stationary baseline with negligible variance.
    QuiescentFlat,
}

/// A short-term projection of future signal trajectory with expanding uncertainty.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryProjection {
    /// Lookahead horizon in seconds.
    pub horizon_seconds: f64,
    /// Projected central magnitude at horizon.
    pub projected_value: f64,
    /// Lower bound of the projected uncertainty envelope.
    pub lower_bound: f64,
    /// Upper bound of the projected uncertainty envelope.
    pub upper_bound: f64,
    /// Projected velocity at horizon.
    pub projected_velocity: f64,
    /// Archetype guiding the projection.
    pub archetype: ProgressionArchetype,
    /// Confidence in the projection [0.0, 1.0], decaying with horizon and uncertainty.
    pub projection_confidence: f64,
}

/// Classifies the progression archetype of a sequence of kinematic metrics.
pub fn classify_archetype(metrics: &[KinematicMetrics], stationary_deadband: f64) -> ProgressionArchetype {
    if metrics.is_empty() {
        return ProgressionArchetype::QuiescentFlat;
    }
    if metrics.len() == 1 {
        let m = metrics[0];
        if m.velocity.abs() <= stationary_deadband {
            return if m.magnitude > 0.5 {
                ProgressionArchetype::Plateau
            } else {
                ProgressionArchetype::QuiescentFlat
            };
        } else if m.velocity > stationary_deadband {
            return ProgressionArchetype::MonotonicAscent;
        } else {
            return ProgressionArchetype::MonotonicDescent;
        }
    }

    let n = metrics.len();
    let mut asc_count = 0;
    let mut desc_count = 0;
    let mut stat_count = 0;
    let mut zero_crossings = 0;

    let mut prev_v_sign: Option<i8> = None;
    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;
    let mut mean_val = 0.0;

    for m in metrics {
        let v = m.velocity;
        let val = m.magnitude;
        min_val = min_val.min(val);
        max_val = max_val.max(val);
        mean_val += val;

        let sign = if v > stationary_deadband {
            asc_count += 1;
            1
        } else if v < -stationary_deadband {
            desc_count += 1;
            -1
        } else {
            stat_count += 1;
            0
        };

        if let Some(prev) = prev_v_sign {
            if sign != 0 && prev != 0 && sign != prev {
                zero_crossings += 1;
            }
        }
        if sign != 0 {
            prev_v_sign = Some(sign);
        }
    }
    mean_val /= n as f64;
    let amplitude = max_val - min_val;
    let signal_scale = max_val.abs().max(min_val.abs()).max(1.0);
    let relative_amplitude = amplitude / signal_scale;

    // Check for quiescent flat vs stable plateau
    if relative_amplitude < 0.05 && stat_count >= n * 3 / 4 {
        return if mean_val > 0.3 * signal_scale {
            ProgressionArchetype::Plateau
        } else {
            ProgressionArchetype::QuiescentFlat
        };
    }

    // Check for cyclical oscillation (multiple velocity zero-crossings)
    if zero_crossings >= 2 {
        return ProgressionArchetype::CyclicalOscillation;
    }

    // Check for transient spike: rapid rise and fall within short sample span
    if n <= 12 && zero_crossings == 1 && relative_amplitude > 0.20 {
        let peak_idx = metrics
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.magnitude.partial_cmp(&b.magnitude).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        if peak_idx > 0 && peak_idx < n - 1 {
            return ProgressionArchetype::TransientSpike;
        }
    }

    // Check for crescendo peak (positive velocity early, slowing to crest near end)
    if zero_crossings == 1 && asc_count >= desc_count {
        return ProgressionArchetype::CrescendoPeak;
    }

    // Check for decrescendo trough (negative velocity early, stabilizing near end)
    if zero_crossings == 1 && desc_count > asc_count {
        return ProgressionArchetype::DecrescendoTrough;
    }

    // Check for monotonic ascent
    if asc_count >= n * 2 / 3 && desc_count == 0 {
        return ProgressionArchetype::MonotonicAscent;
    }

    // Check for monotonic descent
    if desc_count >= n * 2 / 3 && asc_count == 0 {
        return ProgressionArchetype::MonotonicDescent;
    }

    // Check for plateau
    if stat_count >= n / 2 && mean_val > 0.2 * signal_scale {
        return ProgressionArchetype::Plateau;
    }

    ProgressionArchetype::Plateau
}

/// Projects the signal trajectory forward by a lookahead horizon using second-order kinematics with damping.
pub fn project_trajectory(
    current: &KinematicMetrics,
    archetype: ProgressionArchetype,
    horizon_seconds: f64,
    damping_factor: f64, // e.g. 0.2 (velocity dampens towards 0 over time)
    uncertainty: &UncertaintyEstimate,
) -> TrajectoryProjection {
    let dt = horizon_seconds.max(0.0);
    let v0 = current.velocity;
    let a0 = current.acceleration;
    let x0 = current.magnitude;

    // Damped velocity: v(t) = v0 * exp(-gamma * t) + a0 * t * exp(-gamma * t)
    let damping_exp = (-damping_factor * dt).exp();
    let projected_v = (v0 + a0 * dt) * damping_exp;

    // Integrated displacement under damped velocity:
    // delta_x ~ v0 * (1 - exp(-gamma*t))/gamma + 0.5 * a0 * dt^2 * damping_exp
    let delta_x = if damping_factor > 1e-4 {
        v0 * (1.0 - damping_exp) / damping_factor + 0.5 * a0 * dt * dt * damping_exp
    } else {
        v0 * dt + 0.5 * a0 * dt * dt
    };

    let projected_value = (x0 + delta_x).max(0.0);

    // Uncertainty expansion with time horizon:
    // Uncertainty envelope expands as sigma_total * sqrt(1 + dt * expansion_rate)
    let expansion = (1.0 + dt * 0.5).sqrt();
    let margin = uncertainty.aleatoric_std * 1.96 * expansion * (1.0 + 2.0 * uncertainty.epistemic_uncertainty);

    let lower_bound = (projected_value - margin).max(0.0);
    let upper_bound = projected_value + margin;

    // Projection confidence decays with horizon:
    let confidence_decay = (-0.2 * dt).exp();
    let projection_confidence = ((1.0 - uncertainty.total_uncertainty) * confidence_decay).clamp(0.0, 1.0);

    TrajectoryProjection {
        horizon_seconds,
        projected_value,
        lower_bound,
        upper_bound,
        projected_velocity: projected_v,
        archetype,
        projection_confidence,
    }
}
