//! Formal Uncertainty Modeling & Empirical Interval Engine (Stage 3)
//!
//! Provides domain-agnostic uncertainty modeling:
//! - Separation of Aleatoric Uncertainty (estimated residual dispersion) from Epistemic Uncertainty (data sparsity, stale information)
//! - Temporal decay modeling for information freshness
//! - Empirical uncertainty intervals expanding under epistemic ignorance and noise
//! - Scale-invariant normalization relative to dynamic signal magnitude
//! - Transparent quantification of decision risk under incomplete or noisy information
//!
//! Genuinely domain-agnostic: Operates on generic continuous signals and statistical distributions.

use serde::{Deserialize, Serialize};

/// Errors occurring during uncertainty quantification.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum UncertaintyError {
    #[error("Non-finite input encountered: {name} = {value}")]
    NonFiniteInput { name: &'static str, value: f64 },
    #[error("Negative duration encountered: elapsed_seconds = {0}")]
    NegativeDuration(f64),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Comprehensive uncertainty estimate for a scalar observation or projection.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UncertaintyEstimate {
    /// Aleatoric uncertainty: estimated standard deviation of observation residuals.
    pub aleatoric_std: f64,
    /// Epistemic uncertainty: normalized degree of ignorance [0.0, 1.0] from data sparsity and staleness.
    pub epistemic_uncertainty: f64,
    /// Total combined uncertainty index [0.0, 1.0].
    pub total_uncertainty: f64,
    /// Lower bound of the empirical uncertainty interval.
    pub lower_bound: f64,
    /// Upper bound of the empirical uncertainty interval.
    pub upper_bound: f64,
    /// Degree of temporal freshness [0.0, 1.0], where 1.0 is immediate and 0.0 is completely stale.
    pub freshness: f64,
    /// Local sample coverage [0.0, 1.0] relative to target bandwidth support.
    pub sample_coverage: f64,
}

impl UncertaintyEstimate {
    /// Empirical coverage check: returns true if the true value lies within the uncertainty bounds.
    pub fn contains(&self, true_value: f64) -> bool {
        true_value >= self.lower_bound && true_value <= self.upper_bound
    }
}

/// Configuration parameters for uncertainty estimation and calibration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UncertaintyConfig {
    /// Characteristic half-life for temporal staleness in seconds.
    /// After this duration without updates, epistemic uncertainty due to staleness reaches 0.5.
    pub staleness_half_life_seconds: f64,
    /// Minimum required sample count within local support window to achieve full coverage.
    pub target_sample_count: usize,
    /// Empirical interval multiplier z (e.g. 1.96 for nominal Gaussian coverage, 2.0 for Chebyshev bounds).
    pub interval_multiplier_z: f64,
    /// Noise floor to prevent zero-uncertainty singularity.
    pub min_noise_floor: f64,
    /// Reference signal scale for scale-invariant normalized aleatoric noise. Default: 1.0.
    pub nominal_scale: f64,
}

impl Default for UncertaintyConfig {
    fn default() -> Self {
        Self {
            staleness_half_life_seconds: 5.0,
            target_sample_count: 5,
            interval_multiplier_z: 1.96,
            min_noise_floor: 1e-4,
            nominal_scale: 1.0,
        }
    }
}

impl UncertaintyConfig {
    pub fn validate(&self) -> Result<(), UncertaintyError> {
        if self.staleness_half_life_seconds <= 0.0 || !self.staleness_half_life_seconds.is_finite() {
            return Err(UncertaintyError::InvalidConfig("staleness_half_life_seconds must be positive and finite".to_string()));
        }
        if self.target_sample_count == 0 {
            return Err(UncertaintyError::InvalidConfig("target_sample_count must be at least 1".to_string()));
        }
        if self.interval_multiplier_z <= 0.0 || !self.interval_multiplier_z.is_finite() {
            return Err(UncertaintyError::InvalidConfig("interval_multiplier_z must be positive and finite".to_string()));
        }
        if self.min_noise_floor <= 0.0 || !self.min_noise_floor.is_finite() {
            return Err(UncertaintyError::InvalidConfig("min_noise_floor must be positive and finite".to_string()));
        }
        if self.nominal_scale <= 0.0 || !self.nominal_scale.is_finite() {
            return Err(UncertaintyError::InvalidConfig("nominal_scale must be positive and finite".to_string()));
        }
        Ok(())
    }
}

/// Quantifies uncertainty for a measurement given noise residuals, sample density, and age.
/// Validates all numeric inputs against NaNs, infinities, and negative durations.
pub fn try_quantify_uncertainty(
    central_value: f64,
    noise_residual_std: f64,
    local_sample_count: usize,
    elapsed_time_seconds: f64,
    config: &UncertaintyConfig,
) -> Result<UncertaintyEstimate, UncertaintyError> {
    config.validate()?;
    if !central_value.is_finite() {
        return Err(UncertaintyError::NonFiniteInput { name: "central_value", value: central_value });
    }
    if !noise_residual_std.is_finite() {
        return Err(UncertaintyError::NonFiniteInput { name: "noise_residual_std", value: noise_residual_std });
    }
    if !elapsed_time_seconds.is_finite() {
        return Err(UncertaintyError::NonFiniteInput { name: "elapsed_time_seconds", value: elapsed_time_seconds });
    }
    if elapsed_time_seconds < 0.0 {
        return Err(UncertaintyError::NegativeDuration(elapsed_time_seconds));
    }

    // 1. Aleatoric uncertainty: residual dispersion floor
    let aleatoric_std = noise_residual_std.max(config.min_noise_floor);

    // 2. Epistemic uncertainty component from sample sparsity:
    // Scale with degrees of freedom: sparsity = 1.0 - sqrt(min(N, N_target) / N_target)
    let eff_n = local_sample_count.min(config.target_sample_count) as f64;
    let sample_coverage = (local_sample_count as f64 / config.target_sample_count as f64).clamp(0.0, 1.0);
    let sparsity_uncertainty = if config.target_sample_count > 0 {
        (1.0 - (eff_n / config.target_sample_count as f64).sqrt()).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // 3. Epistemic uncertainty component from temporal staleness:
    let decay_rate = std::f64::consts::LN_2 / config.staleness_half_life_seconds;
    let freshness = (-elapsed_time_seconds * decay_rate).exp().clamp(0.0, 1.0);

    // Combined epistemic uncertainty: probabilistic union copula of sparsity and staleness
    // epistemic = 1.0 - (1.0 - sparsity) * freshness
    let epistemic_uncertainty = (1.0 - (1.0 - sparsity_uncertainty) * freshness).clamp(0.0, 1.0);

    // 4. Total combined uncertainty:
    // Scale-invariant normalized noise: relative to dynamic signal scale S_ref = max(|central_value|, nominal_scale)
    let ref_scale = central_value.abs().max(config.nominal_scale);
    let relative_dispersion = aleatoric_std / ref_scale;
    let normalized_noise = (relative_dispersion / (1.0 + relative_dispersion)).clamp(0.0, 1.0);
    let total_uncertainty = (1.0 - (1.0 - normalized_noise) * (1.0 - epistemic_uncertainty)).clamp(0.0, 1.0);

    // 5. Empirical uncertainty interval bounds:
    // Expands with residual dispersion and epistemic factor
    let expansion_factor = 1.0 + 2.0 * epistemic_uncertainty;
    let margin = config.interval_multiplier_z * aleatoric_std * expansion_factor;

    Ok(UncertaintyEstimate {
        aleatoric_std,
        epistemic_uncertainty,
        total_uncertainty,
        lower_bound: central_value - margin,
        upper_bound: central_value + margin,
        freshness,
        sample_coverage,
    })
}

/// Quantifies uncertainty for a measurement given noise residuals, sample density, and age.
/// Clamps invalid duration/NaNs to fallback bounds for backwards compatibility.
pub fn quantify_uncertainty(
    central_value: f64,
    noise_residual_std: f64,
    local_sample_count: usize,
    elapsed_time_seconds: f64,
    config: &UncertaintyConfig,
) -> UncertaintyEstimate {
    try_quantify_uncertainty(
        if central_value.is_finite() { central_value } else { 0.0 },
        if noise_residual_std.is_finite() { noise_residual_std.max(0.0) } else { config.min_noise_floor },
        local_sample_count,
        elapsed_time_seconds.max(0.0),
        config,
    ).unwrap_or_else(|_| UncertaintyEstimate {
        aleatoric_std: config.min_noise_floor,
        epistemic_uncertainty: 1.0,
        total_uncertainty: 1.0,
        lower_bound: central_value - 1.0,
        upper_bound: central_value + 1.0,
        freshness: 0.0,
        sample_coverage: 0.0,
    })
}
