//! Deep Decision Intelligence Architecture (Domain-Agnostic)
//!
//! Provides mathematical foundation, kinematics, adaptive thresholding,
//! hysteresis, progression lifecycle tracking, and meaningful region detection.
//!
//! Genuinely domain-agnostic: Operates on any scalar continuous or discrete time series.

pub mod kinematics;
pub mod progression;

pub use kinematics::*;
pub use progression::*;
