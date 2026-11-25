//! Kinematics & Signal Dynamics Engine (Stage 1 - Corrected)
//!
//! Provides mathematically rigorous, domain-agnostic time-series kinematic analysis:
//! - Time-aware Gaussian smoothing to suppress observation noise across irregular sampling intervals
//! - First-order derivative: Velocity (v = dx/dt, rate of change per second)
//! - Second-order derivative: Acceleration (a = dv/dt, curvature / momentum per second^2)
//! - Directional classification: Ascending, Descending, Stationary
//! - Pure noise residual estimation: sigma_noise = std(raw - smoothed)
//! - Dimensionless Signal-to-Noise Ratio: SNR = (|v| * tau_scale) / (sigma_noise + epsilon)
//! - Empirical kinematic quality heuristic bounded in [0.0, 1.0]
//! - Strict monotonic timestamp and finite-value input validation
//! - Mathematical alignment between batch and streaming derivative calculations
//!
//! Domain-agnostic: operates exclusively on scalar numeric signals over continuous time.

use serde::{Deserialize, Serialize};

/// Direction of signal movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    /// Signal value is increasing with significant positive velocity.
    Ascending,
    /// Signal value is decreasing with significant negative velocity.
    Descending,
    /// Signal rate of change is within the stationary deadband.
    Stationary,
}

/// Errors occurring during kinematic analysis.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum KinematicError {
    #[error("Non-monotonic timestamp: previous {prev_ms} ms, current {current_ms} ms")]
    NonMonotonicTimestamp { prev_ms: i64, current_ms: i64 },
    #[error("Non-finite value encountered at {timestamp_ms} ms: {value}")]
    NonFiniteValue { timestamp_ms: i64, value: f64 },
    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },
}

/// A single timestamped scalar observation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KinematicPoint {
    /// Timestamp in milliseconds.
    pub timestamp_ms: i64,
    /// Scalar signal value.
    pub value: f64,
}

impl KinematicPoint {
    pub fn new(timestamp_ms: i64, value: f64) -> Self {
        Self {
            timestamp_ms,
            value,
        }
    }
}

/// Full kinematic metrics computed for a single point in time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KinematicMetrics {
    /// Timestamp in milliseconds.
    pub timestamp_ms: i64,
    /// Raw un-smoothed value.
    pub raw_magnitude: f64,
    /// Smoothed magnitude after local time-aware Gaussian noise filtering.
    pub magnitude: f64,
    /// First derivative: rate of change per second (units / sec).
    pub velocity: f64,
    /// Second derivative: acceleration / curvature per second squared (units / sec^2).
    pub acceleration: f64,
    /// Directional classification based on velocity and stationary deadband.
    pub direction: Direction,
    /// Estimated residual dispersion (sample standard deviation of centered residuals: (raw - smoothed) - mean).
    /// Note: This is an empirical estimate of observation variance around the smoothed trajectory,
    /// capturing measurement noise, unmodeled higher-order curvature, and transient impulses.
    pub volatility: f64,
    /// Dimensionless Signal-to-Noise Ratio: (|v| * tau_scale) / (volatility + epsilon).
    pub snr: f64,
    /// Empirical kinematic quality score heuristic bounded in [0.0, 1.0].
    /// Note: This is an empirical heuristic index, not a frequentist probability.
    pub quality_score: f64,
}

impl KinematicMetrics {
    /// Alias for estimated residual dispersion around the smoothed signal.
    #[inline]
    pub fn residual_dispersion(&self) -> f64 {
        self.volatility
    }

    /// Backwards-compatible alias for quality_score.
    #[inline]
    pub fn confidence(&self) -> f64 {
        self.quality_score
    }
}

/// Configuration parameters for the kinematic filter and derivative estimator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KinematicFilterConfig {
    /// Standard deviation for time-aware Gaussian smoothing kernel in milliseconds. Default: 300.0 ms.
    pub time_bandwidth_ms: f64,
    /// Characteristic timescale in seconds for converting velocity to dynamic signal swing in dimensionless SNR.
    /// Default: 1.0 second.
    pub snr_timescale_seconds: f64,
    /// Nominal signal magnitude scale for scale-invariant normalizations. Default: 1.0.
    pub nominal_scale: f64,
    /// Velocity magnitude below which signal is considered Stationary (units / sec).
    pub stationary_deadband: f64,
    /// Minimum standard deviation floor to avoid division by zero in SNR.
    pub noise_floor: f64,
    /// Minimum time delta (seconds) between samples to calculate derivatives safely.
    pub min_dt_seconds: f64,
    /// Maximum lookback history window duration in milliseconds for streaming tracker.
    pub max_history_duration_ms: i64,
}

impl Default for KinematicFilterConfig {
    fn default() -> Self {
        Self {
            time_bandwidth_ms: 300.0,
            snr_timescale_seconds: 1.0,
            nominal_scale: 1.0,
            stationary_deadband: 0.005,
            noise_floor: 1e-5,
            min_dt_seconds: 1e-4, // 0.1 ms floor
            max_history_duration_ms: 3000, // 3 seconds window
        }
    }
}

impl KinematicFilterConfig {
    /// Validate configuration invariants.
    pub fn validate(&self) -> Result<(), KinematicError> {
        if self.time_bandwidth_ms <= 0.0 || !self.time_bandwidth_ms.is_finite() {
            return Err(KinematicError::InvalidConfig {
                reason: "time_bandwidth_ms must be positive and finite".to_string(),
            });
        }
        if self.snr_timescale_seconds <= 0.0 || !self.snr_timescale_seconds.is_finite() {
            return Err(KinematicError::InvalidConfig {
                reason: "snr_timescale_seconds must be positive and finite".to_string(),
            });
        }
        if self.nominal_scale <= 0.0 || !self.nominal_scale.is_finite() {
            return Err(KinematicError::InvalidConfig {
                reason: "nominal_scale must be positive and finite".to_string(),
            });
        }
        if self.stationary_deadband < 0.0 || !self.stationary_deadband.is_finite() {
            return Err(KinematicError::InvalidConfig {
                reason: "stationary_deadband must be non-negative and finite".to_string(),
            });
        }
        if self.noise_floor <= 0.0 || !self.noise_floor.is_finite() {
            return Err(KinematicError::InvalidConfig {
                reason: "noise_floor must be positive and finite".to_string(),
            });
        }
        if self.min_dt_seconds <= 0.0 || !self.min_dt_seconds.is_finite() {
            return Err(KinematicError::InvalidConfig {
                reason: "min_dt_seconds must be positive and finite".to_string(),
            });
        }
        if self.max_history_duration_ms <= 0 {
            return Err(KinematicError::InvalidConfig {
                reason: "max_history_duration_ms must be positive".to_string(),
            });
        }
        Ok(())
    }
}

/// Streaming / Online kinematic tracker for continuous real-time observation feeds.
#[derive(Debug, Clone)]
pub struct KinematicSignalTracker {
    config: KinematicFilterConfig,
    raw_history: Vec<KinematicPoint>,
    smoothed_history: Vec<KinematicPoint>,
    last_timestamp_ms: Option<i64>,
    last_metrics: Option<KinematicMetrics>,
}

impl KinematicSignalTracker {
    /// Create a new tracker with the specified configuration.
    pub fn new(config: KinematicFilterConfig) -> Result<Self, KinematicError> {
        config.validate()?;
        Ok(Self {
            config,
            raw_history: Vec::with_capacity(32),
            smoothed_history: Vec::with_capacity(32),
            last_timestamp_ms: None,
            last_metrics: None,
        })
    }

    /// Reset internal state.
    pub fn reset(&mut self) {
        self.raw_history.clear();
        self.smoothed_history.clear();
        self.last_timestamp_ms = None;
        self.last_metrics = None;
    }

    /// Push a new timestamped observation and compute updated kinematic metrics.
    pub fn update(&mut self, timestamp_ms: i64, value: f64) -> Result<KinematicMetrics, KinematicError> {
        // Strict input validation
        if !value.is_finite() {
            return Err(KinematicError::NonFiniteValue { timestamp_ms, value });
        }
        if let Some(prev) = self.last_timestamp_ms {
            if timestamp_ms <= prev {
                return Err(KinematicError::NonMonotonicTimestamp {
                    prev_ms: prev,
                    current_ms: timestamp_ms,
                });
            }
        }
        self.last_timestamp_ms = Some(timestamp_ms);

        let point = KinematicPoint::new(timestamp_ms, value);
        self.raw_history.push(point);

        // Prune raw history outside lookback duration
        let cutoff_ms = timestamp_ms - self.config.max_history_duration_ms;
        self.raw_history.retain(|p| p.timestamp_ms >= cutoff_ms);

        let n = self.raw_history.len();
        if n == 1 {
            let m = KinematicMetrics {
                timestamp_ms,
                raw_magnitude: value,
                magnitude: value,
                velocity: 0.0,
                acceleration: 0.0,
                direction: Direction::Stationary,
                volatility: 0.0,
                snr: 0.0,
                quality_score: 0.2,
            };
            self.smoothed_history.push(KinematicPoint::new(timestamp_ms, value));
            self.last_metrics = Some(m);
            return Ok(m);
        }

        // Time-aware Gaussian smoothing over historical observations
        let smoothed_magnitude = compute_time_aware_smoothed(&self.raw_history, timestamp_ms, &self.config);
        self.smoothed_history.push(KinematicPoint::new(timestamp_ms, smoothed_magnitude));
        self.smoothed_history.retain(|p| p.timestamp_ms >= cutoff_ms);

        // Compute pure noise residual volatility
        let noise_volatility = compute_noise_volatility_streaming(&self.raw_history, &self.smoothed_history);

        // Streaming derivative estimation computed on SMOOTHED history (consistent with batch)
        let sn = self.smoothed_history.len();
        let (velocity, acceleration) = if sn == 2 {
            let dt = ((self.smoothed_history[1].timestamp_ms - self.smoothed_history[0].timestamp_ms) as f64 / 1000.0)
                .max(self.config.min_dt_seconds);
            let v = (self.smoothed_history[1].value - self.smoothed_history[0].value) / dt;
            (v, 0.0)
        } else {
            let p0 = self.smoothed_history[sn - 3];
            let p1 = self.smoothed_history[sn - 2];
            let p2 = self.smoothed_history[sn - 1];

            let dt1 = ((p1.timestamp_ms - p0.timestamp_ms) as f64 / 1000.0).max(self.config.min_dt_seconds);
            let dt2 = ((p2.timestamp_ms - p1.timestamp_ms) as f64 / 1000.0).max(self.config.min_dt_seconds);

            compute_boundary_derivatives_right(p0.value, p1.value, p2.value, dt1, dt2)
        };

        // Direction classification
        let direction = if velocity > self.config.stationary_deadband {
            Direction::Ascending
        } else if velocity < -self.config.stationary_deadband {
            Direction::Descending
        } else {
            Direction::Stationary
        };

        // Dimensionless SNR: (|v| * tau_scale) / noise_volatility
        let effective_noise = noise_volatility.max(self.config.noise_floor);
        let dynamic_signal_swing = velocity.abs() * self.config.snr_timescale_seconds;
        let snr = dynamic_signal_swing / effective_noise;

        // Quality heuristic score [0.0, 1.0]
        let history_span_ms = (timestamp_ms - self.raw_history[0].timestamp_ms) as f64;
        let history_coverage = (history_span_ms / (3.0 * self.config.time_bandwidth_ms)).clamp(0.0, 1.0);
        let normalized_noise = noise_volatility / self.config.nominal_scale;
        let stability_factor = (1.0 / (1.0 + normalized_noise)).clamp(0.0, 1.0);
        let snr_factor = (snr / (1.0 + snr)).clamp(0.0, 1.0);

        let quality_score = (history_coverage * 0.35 + stability_factor * 0.30 + snr_factor * 0.35)
            .clamp(0.0, 1.0);

        let metrics = KinematicMetrics {
            timestamp_ms,
            raw_magnitude: value,
            magnitude: smoothed_magnitude,
            velocity,
            acceleration,
            direction,
            volatility: noise_volatility,
            snr,
            quality_score,
        };

        self.last_metrics = Some(metrics);
        Ok(metrics)
    }

    /// Get the last computed metrics, if any.
    pub fn last_metrics(&self) -> Option<KinematicMetrics> {
        self.last_metrics
    }

    /// Number of points currently retained in the lookback history window.
    pub fn history_len(&self) -> usize {
        self.raw_history.len()
    }
}

/// Analyze an entire batch time series of KinematicPoints with zero-phase time-aware central filtering.
pub fn analyze_kinematics(
    points: &[KinematicPoint],
    config: &KinematicFilterConfig,
) -> Result<Vec<KinematicMetrics>, KinematicError> {
    config.validate()?;
    if points.is_empty() {
        return Ok(Vec::new());
    }

    // Step 1: Strict input validation
    for i in 0..points.len() {
        if !points[i].value.is_finite() {
            return Err(KinematicError::NonFiniteValue {
                timestamp_ms: points[i].timestamp_ms,
                value: points[i].value,
            });
        }
        if i > 0 && points[i].timestamp_ms <= points[i - 1].timestamp_ms {
            return Err(KinematicError::NonMonotonicTimestamp {
                prev_ms: points[i - 1].timestamp_ms,
                current_ms: points[i].timestamp_ms,
            });
        }
    }

    let n = points.len();
    if n == 1 {
        let p = points[0];
        return Ok(vec![KinematicMetrics {
            timestamp_ms: p.timestamp_ms,
            raw_magnitude: p.value,
            magnitude: p.value,
            velocity: 0.0,
            acceleration: 0.0,
            direction: Direction::Stationary,
            volatility: 0.0,
            snr: 0.0,
            quality_score: 0.2,
        }]);
    }

    // Step 2: Time-aware Gaussian smoothing
    let smoothed_values = smooth_series_time_aware(points, config);

    // Step 3: Compute pure noise residuals and local volatility
    let residuals: Vec<f64> = points
        .iter()
        .zip(smoothed_values.iter())
        .map(|(p, s)| p.value - s)
        .collect();

    // Step 4: Compute derivatives on smoothed values
    let mut metrics = Vec::with_capacity(n);

    for i in 0..n {
        let t_i = points[i].timestamp_ms;
        let raw_val = points[i].value;
        let smooth_val = smoothed_values[i];

        // Time-windowed noise volatility (residuals within 3 * sigma_t)
        let noise_volatility = compute_time_windowed_volatility(points, &residuals, i, config);

        let (velocity, acceleration) = if n == 2 {
            let dt = ((points[1].timestamp_ms - points[0].timestamp_ms) as f64 / 1000.0)
                .max(config.min_dt_seconds);
            let v = (smoothed_values[1] - smoothed_values[0]) / dt;
            (v, 0.0)
        } else if i == 0 {
            let dt1 = ((points[1].timestamp_ms - points[0].timestamp_ms) as f64 / 1000.0)
                .max(config.min_dt_seconds);
            let dt2 = ((points[2].timestamp_ms - points[1].timestamp_ms) as f64 / 1000.0)
                .max(config.min_dt_seconds);
            compute_boundary_derivatives_left(
                smoothed_values[0],
                smoothed_values[1],
                smoothed_values[2],
                dt1,
                dt2,
            )
        } else if i == n - 1 {
            let dt1 = ((points[n - 2].timestamp_ms - points[n - 3].timestamp_ms) as f64 / 1000.0)
                .max(config.min_dt_seconds);
            let dt2 = ((points[n - 1].timestamp_ms - points[n - 2].timestamp_ms) as f64 / 1000.0)
                .max(config.min_dt_seconds);
            compute_boundary_derivatives_right(
                smoothed_values[n - 3],
                smoothed_values[n - 2],
                smoothed_values[n - 1],
                dt1,
                dt2,
            )
        } else {
            let dt1 = ((points[i].timestamp_ms - points[i - 1].timestamp_ms) as f64 / 1000.0)
                .max(config.min_dt_seconds);
            let dt2 = ((points[i + 1].timestamp_ms - points[i].timestamp_ms) as f64 / 1000.0)
                .max(config.min_dt_seconds);
            compute_central_derivatives(
                smoothed_values[i - 1],
                smoothed_values[i],
                smoothed_values[i + 1],
                dt1,
                dt2,
            )
        };

        // Direction classification
        let direction = if velocity > config.stationary_deadband {
            Direction::Ascending
        } else if velocity < -config.stationary_deadband {
            Direction::Descending
        } else {
            Direction::Stationary
        };

        // Dimensionless SNR: (|v| * tau_scale) / noise_volatility
        let effective_noise = noise_volatility.max(config.noise_floor);
        let dynamic_signal_swing = velocity.abs() * config.snr_timescale_seconds;
        let snr = dynamic_signal_swing / effective_noise;

        // Quality heuristic score [0.0, 1.0]
        let t_span_left = (t_i - points[0].timestamp_ms) as f64;
        let t_span_right = (points[n - 1].timestamp_ms - t_i) as f64;
        let min_span = t_span_left.min(t_span_right);
        let coverage_factor = (min_span / (2.0 * config.time_bandwidth_ms)).clamp(0.1, 1.0);
        let normalized_noise = noise_volatility / config.nominal_scale;
        let stability_factor = (1.0 / (1.0 + normalized_noise)).clamp(0.0, 1.0);
        let snr_factor = (snr / (1.0 + snr)).clamp(0.0, 1.0);

        let quality_score = (coverage_factor * 0.35 + stability_factor * 0.30 + snr_factor * 0.35)
            .clamp(0.0, 1.0);

        metrics.push(KinematicMetrics {
            timestamp_ms: t_i,
            raw_magnitude: raw_val,
            magnitude: smooth_val,
            velocity,
            acceleration,
            direction,
            volatility: noise_volatility,
            snr,
            quality_score,
        });
    }

    Ok(metrics)
}

/// Time-aware Local Linear Kernel Smoother over a sequence of points.
///
/// Locally weighted linear regression eliminates boundary bias and non-uniform sampling distortion,
/// guaranteeing exact reproduction of linear trends with zero artificial curvature.
/// Normalizes time coordinates by kernel bandwidth for numerical scale-invariance and stability.
fn smooth_series_time_aware(points: &[KinematicPoint], config: &KinematicFilterConfig) -> Vec<f64> {
    let n = points.len();
    if n < 3 {
        return points.iter().map(|p| p.value).collect();
    }
    let mut smoothed = Vec::with_capacity(n);
    let sigma_t = config.time_bandwidth_ms;
    let sigma2 = 2.0 * sigma_t * sigma_t;
    let max_dt = 3.0 * sigma_t;

    for i in 0..n {
        let t_i = points[i].timestamp_ms;
        let mut s0 = 0.0;
        let mut s1 = 0.0;
        let mut s2 = 0.0;
        let mut t0 = 0.0;
        let mut t1 = 0.0;

        for p in points.iter() {
            let dt_ms = (p.timestamp_ms - t_i) as f64;
            if dt_ms.abs() <= max_dt {
                let weight = (-dt_ms * dt_ms / sigma2).exp();
                let tau = dt_ms / sigma_t; // dimensionless coordinate normalized by bandwidth
                s0 += weight;
                s1 += weight * tau;
                s2 += weight * tau * tau;
                t0 += weight * p.value;
                t1 += weight * tau * p.value;
            }
        }

        let det = s0 * s2 - s1 * s1;
        if det > 1e-7 {
            let c0 = (s2 * t0 - s1 * t1) / det;
            smoothed.push(c0);
        } else if s0 > 0.0 {
            smoothed.push(t0 / s0);
        } else {
            smoothed.push(points[i].value);
        }
    }

    smoothed
}

/// Time-aware Local Linear Kernel Smoother for streaming history.
fn compute_time_aware_smoothed(history: &[KinematicPoint], current_t: i64, config: &KinematicFilterConfig) -> f64 {
    let n = history.len();
    if n == 0 {
        return 0.0;
    }
    if n < 3 {
        return history[n - 1].value;
    }
    let sigma_t = config.time_bandwidth_ms;
    let sigma2 = 2.0 * sigma_t * sigma_t;
    let max_dt = 3.0 * sigma_t;

    let mut s0 = 0.0;
    let mut s1 = 0.0;
    let mut s2 = 0.0;
    let mut t0 = 0.0;
    let mut t1 = 0.0;

    for p in history.iter() {
        let dt_ms = (p.timestamp_ms - current_t) as f64;
        if dt_ms.abs() <= max_dt {
            let weight = (-dt_ms * dt_ms / sigma2).exp();
            let tau = dt_ms / sigma_t;
            s0 += weight;
            s1 += weight * tau;
            s2 += weight * tau * tau;
            t0 += weight * p.value;
            t1 += weight * tau * p.value;
        }
    }

    let det = s0 * s2 - s1 * s1;
    if det > 1e-7 {
        (s2 * t0 - s1 * t1) / det
    } else if s0 > 0.0 {
        t0 / s0
    } else {
        history[n - 1].value
    }
}

/// Compute sample standard deviation of centered noise residuals within a time window around index i.
fn compute_time_windowed_volatility(
    points: &[KinematicPoint],
    residuals: &[f64],
    i: usize,
    config: &KinematicFilterConfig,
) -> f64 {
    let t_i = points[i].timestamp_ms;
    let max_dt = (3.0 * config.time_bandwidth_ms) as i64;

    let mut sum = 0.0;
    let mut count = 0;

    for (p, res) in points.iter().zip(residuals.iter()) {
        if (p.timestamp_ms - t_i).abs() <= max_dt {
            sum += *res;
            count += 1;
        }
    }

    if count <= 1 {
        return 0.0;
    }

    let mean = sum / count as f64;
    let mut sum_sq_diff = 0.0;
    for (p, res) in points.iter().zip(residuals.iter()) {
        if (p.timestamp_ms - t_i).abs() <= max_dt {
            let diff = *res - mean;
            sum_sq_diff += diff * diff;
        }
    }

    (sum_sq_diff / (count - 1) as f64).sqrt()
}

/// Compute centered sample standard deviation of noise residuals for streaming history.
fn compute_noise_volatility_streaming(raw: &[KinematicPoint], smoothed: &[KinematicPoint]) -> f64 {
    let n = raw.len().min(smoothed.len());
    if n <= 1 {
        return 0.0;
    }

    let mut sum = 0.0;
    for i in 0..n {
        sum += raw[i].value - smoothed[i].value;
    }
    let mean = sum / n as f64;

    let mut sum_sq_diff = 0.0;
    for i in 0..n {
        let diff = (raw[i].value - smoothed[i].value) - mean;
        sum_sq_diff += diff * diff;
    }

    (sum_sq_diff / (n - 1) as f64).sqrt()
}

/// Central 3-point parabolic derivative for non-uniform spacing.
fn compute_central_derivatives(x0: f64, x1: f64, x2: f64, dt1: f64, dt2: f64) -> (f64, f64) {
    let denom = dt1 * dt2 * (dt1 + dt2);
    if denom <= 0.0 {
        return (0.0, 0.0);
    }
    let velocity = (dt1 * dt1 * x2 + (dt2 * dt2 - dt1 * dt1) * x1 - dt2 * dt2 * x0) / denom;
    let acceleration = 2.0 * (dt1 * x2 - (dt1 + dt2) * x1 + dt2 * x0) / denom;
    (velocity, acceleration)
}

/// Left boundary 3-point forward parabolic derivative.
fn compute_boundary_derivatives_left(x0: f64, x1: f64, x2: f64, dt1: f64, dt2: f64) -> (f64, f64) {
    let total_dt = dt1 + dt2;
    let denom = dt1 * dt2 * total_dt;
    if denom <= 0.0 {
        let v = if dt1 > 0.0 { (x1 - x0) / dt1 } else { 0.0 };
        return (v, 0.0);
    }
    let velocity = (-(2.0 * dt1 + dt2) * dt2 * x0 + total_dt * total_dt * x1 - dt1 * dt1 * x2) / denom;
    let acceleration = 2.0 * (dt2 * x0 - total_dt * x1 + dt1 * x2) / denom;
    (velocity, acceleration)
}

/// Right boundary 3-point backward parabolic derivative.
fn compute_boundary_derivatives_right(x0: f64, x1: f64, x2: f64, dt1: f64, dt2: f64) -> (f64, f64) {
    let total_dt = dt1 + dt2;
    let denom = dt1 * dt2 * total_dt;
    if denom <= 0.0 {
        let v = if dt2 > 0.0 { (x2 - x1) / dt2 } else { 0.0 };
        return (v, 0.0);
    }
    let velocity = (dt2 * dt2 * x0 - total_dt * total_dt * x1 + (dt1 + 2.0 * dt2) * dt1 * x2) / denom;
    let acceleration = 2.0 * (dt2 * x0 - total_dt * x1 + dt1 * x2) / denom;
    (velocity, acceleration)
}
