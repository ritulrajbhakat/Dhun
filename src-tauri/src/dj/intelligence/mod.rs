//! Deep Decision Intelligence Architecture (Domain-Agnostic)
//!
//! Provides mathematical foundation, kinematics, adaptive thresholding,
//! hysteresis, progression lifecycle tracking, meaningful region detection,
//! formal uncertainty quantification, archetype pattern recognition, and multi-criteria prioritization.
//!
//! Genuinely domain-agnostic: Operates on generic scalar continuous or discrete time series and decision models.

pub mod kinematics;
pub mod progression;
pub mod uncertainty;
pub mod pattern;
pub mod prioritizer;

pub use kinematics::*;
pub use progression::*;
pub use uncertainty::*;
pub use pattern::*;
pub use prioritizer::*;
