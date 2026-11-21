//! Multi-Criteria Decision Prioritization Engine (Stage 3)
//!
//! Provides domain-agnostic decision ranking under uncertainty:
//! - Hard safety constraint enforcement (absolute disqualification on constraint violation)
//! - Multi-objective trade-off balancing with normalized criterion weights
//! - Robust scoring under uncertainty: penalizes candidates with high epistemic or aleatoric uncertainty
//! - Pareto non-domination analysis to isolate optimal trade-off candidates
//! - Transparent decision audit trail generating explainable selection rationales
//!
//! Genuinely domain-agnostic: Operates on generic decision candidates and criteria.

use serde::{Deserialize, Serialize};

/// A single evaluation criterion within a decision framework.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionCriterion {
    /// Name or descriptor of the criterion.
    pub name: String,
    /// Evaluated score in range [0.0, 1.0].
    pub score: f64,
    /// Relative importance weight of this criterion (normalized across criteria).
    pub weight: f64,
    /// Uncertainty associated with this criterion's score [0.0, 1.0].
    pub uncertainty: f64,
    /// Whether this criterion is a non-negotiable hard constraint.
    pub is_hard_constraint: bool,
    /// Minimum threshold required if this is a hard constraint.
    pub min_threshold: Option<f64>,
}

impl DecisionCriterion {
    pub fn soft(name: &str, score: f64, weight: f64, uncertainty: f64) -> Self {
        Self {
            name: name.to_string(),
            score: score.clamp(0.0, 1.0),
            weight: weight.max(0.0),
            uncertainty: uncertainty.clamp(0.0, 1.0),
            is_hard_constraint: false,
            min_threshold: None,
        }
    }

    pub fn hard(name: &str, score: f64, min_threshold: f64) -> Self {
        Self {
            name: name.to_string(),
            score: score.clamp(0.0, 1.0),
            weight: 0.0,
            uncertainty: 0.0,
            is_hard_constraint: true,
            min_threshold: Some(min_threshold),
        }
    }
}

/// An evaluated candidate ready for multi-criteria ranking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluatedCandidate<T: Clone> {
    /// Generic payload / identity of the candidate (e.g. ID, track reference, or action identifier).
    pub payload: T,
    /// List of evaluated decision criteria.
    pub criteria: Vec<DecisionCriterion>,
    /// Whether the candidate satisfied all hard safety constraints.
    pub is_eligible: bool,
    /// Nominal composite weighted score across soft criteria [0.0, 1.0].
    pub composite_score: f64,
    /// Total aggregate uncertainty across soft criteria [0.0, 1.0].
    pub aggregate_uncertainty: f64,
    /// Robust score penalizing high uncertainty: composite_score - risk_penalty * aggregate_uncertainty.
    pub robust_score: f64,
    /// Pareto non-domination rank (0 = top frontier, non-dominated).
    pub pareto_rank: usize,
    /// Human-readable explanation of constraints, trade-offs, and final evaluation.
    pub rationale: Vec<String>,
}

/// Configuration for the multi-criteria decision prioritizer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrioritizerConfig {
    /// Risk aversion coefficient k in [0.0, 2.0]: robust_score = composite_score - k * aggregate_uncertainty.
    /// Default: 0.5 (moderate risk penalty for uncertain candidates).
    pub risk_aversion_k: f64,
}

impl Default for PrioritizerConfig {
    fn default() -> Self {
        Self {
            risk_aversion_k: 0.5,
        }
    }
}

/// Prioritizes and ranks a collection of decision candidates under uncertainty.
pub fn rank_candidates<T: Clone>(
    candidates: Vec<(T, Vec<DecisionCriterion>)>,
    config: &PrioritizerConfig,
) -> Vec<EvaluatedCandidate<T>> {
    let mut evaluated: Vec<EvaluatedCandidate<T>> = candidates
        .into_iter()
        .map(|(payload, criteria)| {
            let mut is_eligible = true;
            let mut rationale = Vec::new();

            // Step 1: Enforce hard constraints
            for c in &criteria {
                if c.is_hard_constraint {
                    if let Some(min_th) = c.min_threshold {
                        if c.score < min_th {
                            is_eligible = false;
                            rationale.push(format!(
                                "DISQUALIFIED: Hard constraint '{}' failed (score {:.3} < min {:.3})",
                                c.name, c.score, min_th
                            ));
                        }
                    }
                }
            }

            // Step 2: Calculate composite score across soft criteria
            let soft_criteria: Vec<&DecisionCriterion> = criteria.iter().filter(|c| !c.is_hard_constraint).collect();
            let total_weight: f64 = soft_criteria.iter().map(|c| c.weight).sum();

            let (composite_score, aggregate_uncertainty) = if total_weight > 0.0 {
                let weighted_score: f64 = soft_criteria.iter().map(|c| c.score * c.weight).sum::<f64>() / total_weight;
                let weighted_uncertainty: f64 = soft_criteria.iter().map(|c| c.uncertainty * c.weight).sum::<f64>() / total_weight;
                (weighted_score, weighted_uncertainty)
            } else {
                (0.0, 0.0)
            };

            // Step 3: Compute robust score under uncertainty
            let robust_score = (composite_score - config.risk_aversion_k * aggregate_uncertainty).clamp(0.0, 1.0);

            if is_eligible {
                rationale.push(format!(
                    "ELIGIBLE: Nominal score {:.3}, Uncertainty {:.3}, Robust score {:.3}",
                    composite_score, aggregate_uncertainty, robust_score
                ));
            }

            EvaluatedCandidate {
                payload,
                criteria,
                is_eligible,
                composite_score,
                aggregate_uncertainty,
                robust_score,
                pareto_rank: 0,
                rationale,
            }
        })
        .collect();

    // Step 4: Compute Pareto Non-Domination Ranks among eligible candidates
    compute_pareto_ranks(&mut evaluated);

    // Step 5: Sort candidates:
    // 1. Eligible first (is_eligible = true)
    // 2. Pareto rank ascending (rank 0 is non-dominated frontier)
    // 3. Robust score descending
    evaluated.sort_by(|a, b| {
        b.is_eligible
            .cmp(&a.is_eligible)
            .then_with(|| a.pareto_rank.cmp(&b.pareto_rank))
            .then_with(|| b.robust_score.partial_cmp(&a.robust_score).unwrap_or(std::cmp::Ordering::Equal))
    });

    evaluated
}

/// Computes Pareto non-domination ranks across eligible candidates.
fn compute_pareto_ranks<T: Clone>(candidates: &mut [EvaluatedCandidate<T>]) {
    let eligible_indices: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_eligible)
        .map(|(i, _)| i)
        .collect();

    let mut current_rank = 0;
    let mut remaining = eligible_indices;

    while !remaining.is_empty() {
        let mut non_dominated = Vec::new();

        for &i in &remaining {
            let mut is_dominated = false;
            for &j in &remaining {
                if i != j && dominates(&candidates[j], &candidates[i]) {
                    is_dominated = true;
                    break;
                }
            }
            if !is_dominated {
                non_dominated.push(i);
            }
        }

        if non_dominated.is_empty() {
            // Cycle or tie break
            for &i in &remaining {
                candidates[i].pareto_rank = current_rank;
            }
            break;
        }

        for &i in &non_dominated {
            candidates[i].pareto_rank = current_rank;
        }

        remaining.retain(|idx| !non_dominated.contains(idx));
        current_rank += 1;
    }
}

/// Returns true if candidate A dominates candidate B.
/// A dominates B if A is at least as good in all criteria and uncertainty, and strictly better in at least one.
fn dominates<T: Clone>(a: &EvaluatedCandidate<T>, b: &EvaluatedCandidate<T>) -> bool {
    let a_soft: Vec<&DecisionCriterion> = a.criteria.iter().filter(|c| !c.is_hard_constraint).collect();
    let b_soft: Vec<&DecisionCriterion> = b.criteria.iter().filter(|c| !c.is_hard_constraint).collect();

    if a_soft.len() != b_soft.len() || a_soft.is_empty() {
        return a.robust_score >= b.robust_score
            && a.aggregate_uncertainty <= b.aggregate_uncertainty
            && (a.robust_score > b.robust_score || a.aggregate_uncertainty < b.aggregate_uncertainty);
    }

    let mut strictly_better = false;
    for (ca, cb) in a_soft.iter().zip(b_soft.iter()) {
        // A must be as good or higher in score, and as low or lower in uncertainty
        if ca.score < cb.score || ca.uncertainty > cb.uncertainty {
            return false;
        }
        if ca.score > cb.score || ca.uncertainty < cb.uncertainty {
            strictly_better = true;
        }
    }
    strictly_better
}
