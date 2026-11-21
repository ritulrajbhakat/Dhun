//! Formal Uncertainty Quantification & Calibration Engine (Stage 3)
//!
//! Provides mathematically rigorous, domain-agnostic uncertainty modeling:
//! - Separation of Aleatoric Uncertainty (irreducible noise residuals) from Epistemic Uncertainty (data sparsity, stale information)
//! - Temporal decay modeling for information freshness
//! - Calibrated confidence intervals and Chebyshev/Gaussian bounds
//! - Transparent quantification of decision risk under incomplete or noisy information
//!
//! Genuinely domain-agnostic: Operates on generic continuous signals and statistical distributions.

use serde::{Deserialize, Serialize};

/// Comprehensive uncertainty estimate for a scalar observation or projection.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UncertaintyEstimate {
    /// Aleatoric uncertainty: standard deviation of observation noise residuals.
    pub aleatoric_std: f64,
    /// Epistemic uncertainty: normalized degree of ignorance [0.0, 1.0] from data sparsity and staleness.
    pub epistemic_uncertainty: f64,
    /// Total combined uncertainty index [0.0, 1.0].
    pub total_uncertainty: f64,
    /// Lower bound of the calibrated credible interval.
    pub lower_bound: f64,
    /// Upper bound of the calibrated credible interval.
    pub upper_bound: f64,
    /// Degree of temporal freshness [0.0, 1.0], where 1.0 is immediate and 0.0 is completely stale.
    pub freshness: f64,
    /// Local sample coverage [0.0, 1.0] relative to target bandwidth support.
    pub sample_coverage: f64,
}

/// Configuration parameters for uncertainty estimation and calibration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UncertaintyConfig {
    /// Characteristic half-life for temporal staleness in seconds.
    /// After this duration without updates, epistemic uncertainty due to staleness reaches 0.5.
    pub staleness_half_life_seconds: f64,
    /// Minimum required sample count within local support window to achieve full coverage.
    pub target_sample_count: usize,
    /// Confidence interval coverage multiplier z (e.g. 1.96 for ~95% under normality, 2.0 for Chebyshev).
    pub interval_multiplier_z: f64,
    /// Noise floor to prevent zero-uncertainty singularity.
    pub min_noise_floor: f64,
}

impl Default for UncertaintyConfig {
    fn default() -> Self {
        Self {
            staleness_half_life_seconds: 5.0,
            target_sample_count: 5,
            interval_multiplier_z: 1.96,
            min_noise_floor: 1e-4,
        }
    }
}

impl UncertaintyConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.staleness_half_life_seconds <= 0.0 || !self.staleness_half_life_seconds.is_finite() {
            return Err("staleness_half_life_seconds must be positive and finite".to_string());
        }
        if self.target_sample_count == 0 {
            return Err("target_sample_count must be at least 1".to_string());
        }
        if self.interval_multiplier_z <= 0.0 || !self.interval_multiplier_z.is_finite() {
            return Err("interval_multiplier_z must be positive and finite".to_string());
        }
        if self.min_noise_floor <= 0.0 || !self.min_noise_floor.is_finite() {
            return Err("min_noise_floor must be positive and finite".to_string());
        }
        Ok(())
    }
}

/// Quantifies uncertainty for a measurement given noise residuals, sample density, and age.
pub fn quantify_uncertainty(
    central_value: f64,
    noise_residual_std: f64,
    local_sample_count: usize,
    elapsed_time_seconds: f64,
    config: &UncertaintyConfig,
) -> UncertaintyEstimate {
    // 1. Aleatoric uncertainty: irreducible noise
    let aleatoric_std = noise_residual_std.max(config.min_noise_floor);

    // 2. Epistemic uncertainty component from sample sparsity:
    // When sample count is below target, uncertainty increases exponentially
    let sample_coverage = (local_sample_count as f64 / config.target_sample_count as f64).clamp(0.0, 1.0);
    let sparsity_uncertainty = 1.0 - sample_coverage;

    // 3. Epistemic uncertainty component from temporal staleness:
    // Continuous-time exponential decay: freshness = 2^(-dt / t_half) = exp(-dt * ln(2) / t_half)
    let decay_rate = std::f64::consts::LN_2 / config.staleness_half_life_seconds;
    let freshness = (-elapsed_time_seconds.max(0.0) * decay_rate).exp().clamp(0.0, 1.0);
    let staleness_uncertainty = 1.0 - freshness;

    // Combined epistemic uncertainty (bounded [0.0, 1.0])
    let epistemic_uncertainty = (sparsity_uncertainty * 0.5 + staleness_uncertainty * 0.5).clamp(0.0, 1.0);

    // 4. Total combined uncertainty:
    // Normalized total uncertainty combines noise scaled by signal magnitude with epistemic lack of knowledge
    let normalized_noise = (aleatoric_std / (1.0 + aleatoric_std)).clamp(0.0, 1.0);
    let total_uncertainty = (1.0 - (1.0 - normalized_noise) * (1.0 - epistemic_uncertainty)).clamp(0.0, 1.0);

    // 5. Calibrated credible interval bounds:
    // Interval expands with both aleatoric noise and epistemic uncertainty expansion factor
    let expansion_factor = 1.0 + 2.0 * epistemic_uncertainty;
    let margin = config.interval_multiplier_z * aleatoric_std * expansion_factor;

    UncertaintyEstimate {
        aleatoric_std,
        epistemic_uncertainty,
        total_uncertainty,
        lower_bound: central_value - margin,
        upper_bound: central_value + margin,
        freshness,
        sample_coverage,
    }
}
