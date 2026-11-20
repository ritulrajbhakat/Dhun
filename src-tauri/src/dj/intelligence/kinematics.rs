//! Kinematics & Signal Dynamics Engine (Stage 1)
//!
//! Provides mathematically rigorous, domain-agnostic time-series kinematic analysis:
//! - Low-lag Gaussian / moving-window smoothing to suppress observation noise
//! - First-order derivative: Velocity (v = dx/dt, rate of change per second)
//! - Second-order derivative: Acceleration (a = dv/dt, curvature / momentum per second^2)
//! - Directional classification: Ascending, Descending, Stationary
//! - Rolling volatility (standard deviation sigma)
//! - Signal-to-Noise Ratio (SNR = |v| / (sigma_noise + epsilon))
//! - Kinematic confidence estimation bounded in [0.0, 1.0]
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
    /// Smoothed magnitude after local noise filtering.
    pub magnitude: f64,
    /// First derivative: rate of change per second (units / sec).
    pub velocity: f64,
    /// Second derivative: acceleration / curvature per second squared (units / sec^2).
    pub acceleration: f64,
    /// Directional classification based on velocity and stationary deadband.
    pub direction: Direction,
    /// Rolling standard deviation of the signal within the local window.
    pub volatility: f64,
    /// Signal-to-Noise Ratio: ratio of velocity magnitude to local noise volatility.
    pub snr: f64,
    /// Confidence in the kinematic assessment in range [0.0, 1.0].
    pub confidence: f64,
}

/// Configuration parameters for the kinematic filter and derivative estimator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KinematicFilterConfig {
    /// Smoothing window half-width (samples on each side for local smoothing).
    /// Total window size = 2 * half_window + 1. Default: 2 (window of 5).
    pub half_window: usize,
    /// Standard deviation for Gaussian smoothing kernel (in units of samples). Default: 1.0.
    pub gaussian_sigma: f64,
    /// Velocity magnitude below which signal is considered Stationary (units / sec).
    pub stationary_deadband: f64,
    /// Minimum standard deviation floor to avoid division by zero in SNR.
    pub noise_floor: f64,
    /// Minimum time delta (seconds) between samples to calculate derivatives safely.
    pub min_dt_seconds: f64,
}

impl Default for KinematicFilterConfig {
    fn default() -> Self {
        Self {
            half_window: 2,
            gaussian_sigma: 1.0,
            stationary_deadband: 0.005,
            noise_floor: 1e-5,
            min_dt_seconds: 1e-4, // 0.1 ms floor
        }
    }
}

/// Streaming / Online kinematic tracker for continuous real-time observation feeds.
#[derive(Debug, Clone)]
pub struct KinematicSignalTracker {
    config: KinematicFilterConfig,
    history: Vec<KinematicPoint>,
    max_history: usize,
    last_metrics: Option<KinematicMetrics>,
}

impl KinematicSignalTracker {
    /// Create a new tracker with the specified configuration.
    pub fn new(config: KinematicFilterConfig) -> Self {
        let max_history = (config.half_window * 2 + 1).max(5);
        Self {
            config,
            history: Vec::with_capacity(max_history),
            max_history,
            last_metrics: None,
        }
    }

    /// Reset internal state.
    pub fn reset(&mut self) {
        self.history.clear();
        self.last_metrics = None;
    }

    /// Push a new timestamped observation and compute updated kinematic metrics.
    pub fn update(&mut self, timestamp_ms: i64, value: f64) -> KinematicMetrics {
        let point = KinematicPoint::new(timestamp_ms, value);
        self.history.push(point);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        let n = self.history.len();
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
                confidence: 0.2, // Low confidence with single point
            };
            self.last_metrics = Some(m);
            return m;
        }

        // Compute local smoothed value and statistics
        let smoothed_magnitude = compute_local_smoothed(&self.history, &self.config);
        let volatility = compute_local_std_dev(&self.history);

        // Derivative estimation using backward differences over available history
        let (velocity, acceleration) = if n == 2 {
            let dt = ((self.history[1].timestamp_ms - self.history[0].timestamp_ms) as f64 / 1000.0)
                .max(self.config.min_dt_seconds);
            let v = (self.history[1].value - self.history[0].value) / dt;
            (v, 0.0)
        } else {
            // 3-point parabolic backward derivative
            let p0 = self.history[n - 3];
            let p1 = self.history[n - 2];
            let p2 = self.history[n - 1];

            let dt1 = ((p1.timestamp_ms - p0.timestamp_ms) as f64 / 1000.0).max(self.config.min_dt_seconds);
            let dt2 = ((p2.timestamp_ms - p1.timestamp_ms) as f64 / 1000.0).max(self.config.min_dt_seconds);

            let v1 = (p1.value - p0.value) / dt1;
            let v2 = (p2.value - p1.value) / dt2;

            // Velocity at the newest point: linear extrapolation of velocity
            let dt_mid = 0.5 * (dt1 + dt2);
            let a = (v2 - v1) / dt_mid;
            let v = v2 + a * (0.5 * dt2);
            (v, a)
        };

        // Direction classification
        let direction = if velocity > self.config.stationary_deadband {
            Direction::Ascending
        } else if velocity < -self.config.stationary_deadband {
            Direction::Descending
        } else {
            Direction::Stationary
        };

        // Signal to Noise Ratio
        let effective_noise = volatility.max(self.config.noise_floor);
        let snr = velocity.abs() / effective_noise;

        // Kinematic confidence bounded in [0.0, 1.0]
        let history_factor = (n as f64 / self.max_history as f64).clamp(0.0, 1.0);
        let stability_factor = (1.0 / (1.0 + volatility)).clamp(0.0, 1.0);
        let confidence = (history_factor * 0.4 + stability_factor * 0.3 + (snr / (1.0 + snr)) * 0.3)
            .clamp(0.0, 1.0);

        let metrics = KinematicMetrics {
            timestamp_ms,
            raw_magnitude: value,
            magnitude: smoothed_magnitude,
            velocity,
            acceleration,
            direction,
            volatility,
            snr,
            confidence,
        };

        self.last_metrics = Some(metrics);
        metrics
    }

    /// Get the last computed metrics, if any.
    pub fn last_metrics(&self) -> Option<KinematicMetrics> {
        self.last_metrics
    }
}

/// Analyze an entire batch time series of KinematicPoints with zero-phase central difference filtering.
///
/// For interior points, uses symmetric Gaussian smoothing and 3-point parabolic central derivatives
/// to eliminate artificial phase lag. Boundary points employ forward/backward difference schemes.
pub fn analyze_kinematics(
    points: &[KinematicPoint],
    config: &KinematicFilterConfig,
) -> Vec<KinematicMetrics> {
    if points.is_empty() {
        return Vec::new();
    }

    let n = points.len();
    if n == 1 {
        let p = points[0];
        return vec![KinematicMetrics {
            timestamp_ms: p.timestamp_ms,
            raw_magnitude: p.value,
            magnitude: p.value,
            velocity: 0.0,
            acceleration: 0.0,
            direction: Direction::Stationary,
            volatility: 0.0,
            snr: 0.0,
            confidence: 0.2,
        }];
    }

    // Step 1: Smooth the signal using Gaussian kernel
    let smoothed_values = smooth_series(points, config);

    // Step 2: Compute derivatives (velocity & acceleration)
    let mut metrics = Vec::with_capacity(n);

    for i in 0..n {
        let t_i = points[i].timestamp_ms;
        let raw_val = points[i].value;
        let smooth_val = smoothed_values[i];

        // Compute local volatility in a window around i
        let w_start = i.saturating_sub(config.half_window);
        let w_end = (i + config.half_window + 1).min(n);
        let window_points = &points[w_start..w_end];
        let volatility = compute_local_std_dev(window_points);

        let (velocity, acceleration) = if n == 2 {
            let dt = ((points[1].timestamp_ms - points[0].timestamp_ms) as f64 / 1000.0)
                .max(config.min_dt_seconds);
            let v = (smoothed_values[1] - smoothed_values[0]) / dt;
            (v, 0.0)
        } else if i == 0 {
            // Forward difference with 3 points: parabolic fit at left boundary
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
            // Backward difference with 3 points: parabolic fit at right boundary
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
            // Central 3-point parabolic derivative for interior points
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

        // SNR
        let effective_noise = volatility.max(config.noise_floor);
        let snr = velocity.abs() / effective_noise;

        // Confidence
        let window_coverage = (w_end - w_start) as f64 / (config.half_window * 2 + 1) as f64;
        let stability_factor = (1.0 / (1.0 + volatility)).clamp(0.0, 1.0);
        let confidence = (window_coverage * 0.4 + stability_factor * 0.3 + (snr / (1.0 + snr)) * 0.3)
            .clamp(0.0, 1.0);

        metrics.push(KinematicMetrics {
            timestamp_ms: t_i,
            raw_magnitude: raw_val,
            magnitude: smooth_val,
            velocity,
            acceleration,
            direction,
            volatility,
            snr,
            confidence,
        });
    }

    metrics
}

/// Gaussian smoothing filter over a sequence of points.
fn smooth_series(points: &[KinematicPoint], config: &KinematicFilterConfig) -> Vec<f64> {
    let n = points.len();
    if n < 3 {
        return points.iter().map(|p| p.value).collect();
    }
    let mut smoothed = Vec::with_capacity(n);
    let hw = config.half_window;
    let sigma2 = 2.0 * config.gaussian_sigma * config.gaussian_sigma;

    for i in 0..n {
        let w_start = i.saturating_sub(hw);
        let w_end = (i + hw + 1).min(n);

        let mut sum_weights = 0.0;
        let mut weighted_val = 0.0;

        for (offset, p) in points[w_start..w_end].iter().enumerate() {
            let actual_idx = w_start + offset;
            let dist = (actual_idx as f64 - i as f64).abs();
            let weight = (-dist * dist / sigma2).exp();

            sum_weights += weight;
            weighted_val += weight * p.value;
        }

        if sum_weights > 0.0 {
            smoothed.push(weighted_val / sum_weights);
        } else {
            smoothed.push(points[i].value);
        }
    }

    smoothed
}

/// Smooth recent history points in streaming tracker.
fn compute_local_smoothed(history: &[KinematicPoint], config: &KinematicFilterConfig) -> f64 {
    let n = history.len();
    if n == 0 {
        return 0.0;
    }
    let target_idx = n - 1;
    let sigma2 = 2.0 * config.gaussian_sigma * config.gaussian_sigma;

    let mut sum_weights = 0.0;
    let mut weighted_val = 0.0;

    for (i, p) in history.iter().enumerate() {
        let dist = (target_idx as f64 - i as f64).abs();
        let weight = (-dist * dist / sigma2).exp();
        sum_weights += weight;
        weighted_val += weight * p.value;
    }

    if sum_weights > 0.0 {
        weighted_val / sum_weights
    } else {
        history[target_idx].value
    }
}

/// Compute standard deviation of scalar values in a slice.
fn compute_local_std_dev(points: &[KinematicPoint]) -> f64 {
    let n = points.len();
    if n <= 1 {
        return 0.0;
    }
    let mean = points.iter().map(|p| p.value).sum::<f64>() / n as f64;
    let var = points
        .iter()
        .map(|p| {
            let diff = p.value - mean;
            diff * diff
        })
        .sum::<f64>()
        / (n - 1) as f64;
    var.sqrt()
}

/// Central 3-point parabolic derivative for non-uniform spacing.
///
/// Given three points (x0, t0), (x1, t1), (x2, t2) where dt1 = t1 - t0 and dt2 = t2 - t1,
/// fits an osculating quadratic polynomial P(t) and calculates P'(t1) and P''(t1).
fn compute_central_derivatives(x0: f64, x1: f64, x2: f64, dt1: f64, dt2: f64) -> (f64, f64) {
    let denom = dt1 * dt2 * (dt1 + dt2);
    if denom <= 0.0 {
        return (0.0, 0.0);
    }

    // Velocity (first derivative at t1):
    // v = (dt1^2 * x2 + (dt2^2 - dt1^2) * x1 - dt2^2 * x0) / (dt1 * dt2 * (dt1 + dt2))
    let velocity = (dt1 * dt1 * x2 + (dt2 * dt2 - dt1 * dt1) * x1 - dt2 * dt2 * x0) / denom;

    // Acceleration (second derivative at t1):
    // a = 2 * (dt1 * x2 - (dt1 + dt2) * x1 + dt2 * x0) / denom
    let acceleration = 2.0 * (dt1 * x2 - (dt1 + dt2) * x1 + dt2 * x0) / denom;

    (velocity, acceleration)
}

/// Left boundary 3-point forward parabolic derivative.
///
/// Calculates P'(t0) and P''(t0) from (x0, t0), (x1, t1), (x2, t2).
fn compute_boundary_derivatives_left(x0: f64, x1: f64, x2: f64, dt1: f64, dt2: f64) -> (f64, f64) {
    let total_dt = dt1 + dt2;
    let denom = dt1 * dt2 * total_dt;
    if denom <= 0.0 {
        let v = if dt1 > 0.0 { (x1 - x0) / dt1 } else { 0.0 };
        return (v, 0.0);
    }

    // Analytical forward derivative of quadratic fit at t = 0 (t0):
    // v0 = (- (2*dt1 + dt2)*dt2 * x0 + total_dt^2 * x1 - dt1^2 * x2) / denom
    let velocity = (-(2.0 * dt1 + dt2) * dt2 * x0 + total_dt * total_dt * x1 - dt1 * dt1 * x2) / denom;
    let acceleration = 2.0 * (dt2 * x0 - total_dt * x1 + dt1 * x2) / denom;

    (velocity, acceleration)
}

/// Right boundary 3-point backward parabolic derivative.
///
/// Calculates P'(t2) and P''(t2) from (x0, t0), (x1, t1), (x2, t2).
fn compute_boundary_derivatives_right(x0: f64, x1: f64, x2: f64, dt1: f64, dt2: f64) -> (f64, f64) {
    let total_dt = dt1 + dt2;
    let denom = dt1 * dt2 * total_dt;
    if denom <= 0.0 {
        let v = if dt2 > 0.0 { (x2 - x1) / dt2 } else { 0.0 };
        return (v, 0.0);
    }

    // Analytical backward derivative of quadratic fit at t = total_dt (t2):
    // v2 = (dt2^2 * x0 - total_dt^2 * x1 + (dt1 + 2*dt2)*dt1 * x2) / denom
    let velocity = (dt2 * dt2 * x0 - total_dt * total_dt * x1 + (dt1 + 2.0 * dt2) * dt1 * x2) / denom;
    let acceleration = 2.0 * (dt2 * x0 - total_dt * x1 + dt1 * x2) / denom;

    (velocity, acceleration)
}
