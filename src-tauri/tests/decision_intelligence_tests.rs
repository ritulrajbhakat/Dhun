//! Stage 1 & Stage 2 Decision Intelligence Verification Suite (Comprehensive Corrective Pass)
//!
//! Validates:
//! 1. Kinematic derivatives (velocity, acceleration, direction, volatility, SNR, quality score)
//! 2. Zero-phase time-aware Gaussian smoothing and pure noise residual filtering
//! 3. Dimensionless SNR formulation (|v| * tau_scale / sigma_noise)
//! 4. Non-uniform sampling invariant derivative and smoothing estimation
//! 5. Input validation (rejection of non-monotonic timestamps, NaNs, infinities, invalid configs)
//! 6. Dual-threshold hysteresis and limit-cycle chatter prevention
//! 7. Candidate baseline protection (preventing emerging events from polluting baseline)
//! 8. Cold-start already-active signal initialization
//! 9. Temporal persistence debounce windows with retroactive onset and mass accounting
//! 10. Streaming vs. batch calculation consistency
//! 11. Full 8-state progression lifecycle traversal
//! 12. Meaningful region extraction, trapezoidal mass integration, and sustained duration tracking
//! 13. Mathematical spike rejection vs. sustained progression validation
//! 14. Multiple meaningful region segmentation
//! 15. Complete domain agnosticism (pure scalar time-series signals)

use dhun_lib::dj::intelligence::{
    analyze_kinematics, analyze_progression, classify_archetype, project_trajectory,
    quantify_uncertainty, rank_candidates, AdaptiveThresholdConfig, DecisionCriterion,
    Direction, InitialConditionPolicy, KinematicError, KinematicFilterConfig, KinematicMetrics,
    KinematicPoint, KinematicSignalTracker, MeaningfulRegion, PrioritizerConfig,
    ProgressionArchetype, ProgressionState, ProgressionStateMachine, UncertaintyConfig,
};
use dhun_lib::dj::energy::{SectionEnergyProfile, SetStoryteller};
use dhun_lib::dj::hook_usability::HookUsabilityEvaluator;
use dhun_lib::dj::selector::TrackSelector;
use dhun_lib::db::models::{
    HookCandidate, PlayableTrackCandidate, TimelineAlignmentStatus, Track, TrackAnalysis,
    TrackTimingProfile,
};
use std::f64::consts::PI;

// =========================================================================
// STAGE 1: KINEMATICS & DERIVATIVES VERIFICATION
// =========================================================================

#[test]
fn test_kinematics_constant_signal() {
    let config = KinematicFilterConfig::default();
    let points: Vec<KinematicPoint> = (0..50)
        .map(|i| KinematicPoint::new(i * 100, 0.75))
        .collect();

    let metrics = analyze_kinematics(&points, &config).expect("Batch analysis failed");
    assert_eq!(metrics.len(), 50);

    for m in &metrics {
        assert!((m.magnitude - 0.75).abs() < 1e-6);
        assert!(m.velocity.abs() < 1e-6, "Velocity should be 0, got {}", m.velocity);
        assert!(m.acceleration.abs() < 1e-6, "Acceleration should be 0, got {}", m.acceleration);
        assert_eq!(m.direction, Direction::Stationary);
        assert!(m.volatility < 1e-6);
    }
}

#[test]
fn test_kinematics_linear_ramp_ascending() {
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 200.0,
        stationary_deadband: 0.001,
        ..Default::default()
    };

    // Ramp: x(t) = 0.2 + 0.1 * t_sec
    let points: Vec<KinematicPoint> = (0..60)
        .map(|i| {
            let t_ms = i * 100;
            let t_sec = t_ms as f64 / 1000.0;
            KinematicPoint::new(t_ms, 0.2 + 0.1 * t_sec)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config).expect("Batch analysis failed");

    for m in &metrics[5..metrics.len() - 5] {
        let v_error = (m.velocity - 0.1).abs();
        assert!(
            v_error < 0.005,
            "Velocity error too high at t={}: got {}, expected 0.1",
            m.timestamp_ms,
            m.velocity
        );
        assert!(
            m.acceleration.abs() < 0.005,
            "Acceleration should be ~0, got {}",
            m.acceleration
        );
        assert_eq!(m.direction, Direction::Ascending);
    }
}

#[test]
fn test_kinematics_linear_ramp_descending() {
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 200.0,
        stationary_deadband: 0.001,
        ..Default::default()
    };

    let points: Vec<KinematicPoint> = (0..50)
        .map(|i| {
            let t_ms = i * 100;
            let t_sec = t_ms as f64 / 1000.0;
            KinematicPoint::new(t_ms, 0.9 - 0.05 * t_sec)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config).expect("Batch analysis failed");

    for m in &metrics[5..metrics.len() - 5] {
        let v_error = (m.velocity - (-0.05)).abs();
        assert!(v_error < 0.005, "Velocity error at t={}: got {}", m.timestamp_ms, m.velocity);
        assert_eq!(m.direction, Direction::Descending);
    }
}

#[test]
fn test_kinematics_parabolic_curve_exact_acceleration() {
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 300.0,
        ..Default::default()
    };

    let a_theo = 0.2;
    let points: Vec<KinematicPoint> = (0..50)
        .map(|i| {
            let t_ms = i * 200;
            let t_sec = t_ms as f64 / 1000.0;
            KinematicPoint::new(t_ms, 0.5 * a_theo * t_sec * t_sec)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config).expect("Batch analysis failed");

    for m in &metrics[8..metrics.len() - 8] {
        let t_sec = m.timestamp_ms as f64 / 1000.0;
        let v_expected = a_theo * t_sec;
        let v_diff = (m.velocity - v_expected).abs();
        let a_diff = (m.acceleration - a_theo).abs();

        assert!(
            v_diff < 0.05,
            "Velocity mismatch at t={}: got {}, expected {}",
            t_sec,
            m.velocity,
            v_expected
        );
        assert!(
            a_diff < 0.05,
            "Acceleration mismatch at t={}: got {}, expected {}",
            t_sec,
            m.acceleration,
            a_theo
        );
    }
}

#[test]
fn test_kinematics_sinusoidal_wave() {
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 200.0,
        ..Default::default()
    };

    let freq = 0.2;
    let omega = 2.0 * PI * freq;
    let points: Vec<KinematicPoint> = (0..100)
        .map(|i| {
            let t_ms = i * 100;
            let t_sec = t_ms as f64 / 1000.0;
            KinematicPoint::new(t_ms, (omega * t_sec).sin())
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config).expect("Batch analysis failed");

    let crest_point = metrics
        .iter()
        .min_by(|a, b| {
            let d_a = (a.timestamp_ms - 1250).abs();
            let d_b = (b.timestamp_ms - 1250).abs();
            d_a.cmp(&d_b)
        })
        .unwrap();

    assert!(crest_point.magnitude > 0.95, "Crest magnitude should be near 1.0");
    assert!(crest_point.velocity.abs() < 0.2, "Velocity at peak should be ~0, got {}", crest_point.velocity);
    assert!(crest_point.acceleration < -0.5, "Acceleration at peak must be negative");
}

#[test]
fn test_kinematics_pure_noise_residuals_and_dimensionless_snr() {
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 250.0,
        snr_timescale_seconds: 1.0,
        ..Default::default()
    };

    // Constant baseline of 0.5 with high-frequency +/- 0.04 jitter
    let points: Vec<KinematicPoint> = (0..50)
        .map(|i| {
            let noise = if i % 2 == 0 { 0.04 } else { -0.04 };
            KinematicPoint::new(i * 100, 0.5 + noise)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config).expect("Batch analysis failed");

    for m in &metrics[6..metrics.len() - 6] {
        // Smoothed magnitude should suppress alternating noise
        let smoothed_deviation = (m.magnitude - 0.5).abs();
        assert!(smoothed_deviation < 0.02, "Smoothed value should be near 0.5: got {}", m.magnitude);
        // Volatility measures pure residual noise std dev, which should be ~0.04
        assert!(m.volatility > 0.02, "Noise residual volatility should be detected: got {}", m.volatility);
        // On a flat signal with noise, velocity is near 0, so dimensionless SNR should be low
        assert!(m.snr < 1.0, "SNR on flat noisy signal should be low: got {}", m.snr);
    }
}

#[test]
fn test_kinematics_non_uniform_sampling() {
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 300.0,
        stationary_deadband: 0.001,
        ..Default::default()
    };

    // Non-uniform time steps: 50ms, 150ms, 80ms, 220ms, 100ms...
    let intervals_ms = [50, 150, 80, 220, 100, 180, 70, 250, 120, 90, 160, 140];
    let mut points = Vec::new();
    let mut current_t = 0;
    for &dt in intervals_ms.iter().cycle().take(60) {
        current_t += dt;
        let t_sec = current_t as f64 / 1000.0;
        // Constant velocity signal: x(t) = 0.1 * t
        points.push(KinematicPoint::new(current_t, 0.1 * t_sec));
    }

    let metrics = analyze_kinematics(&points, &config).expect("Non-uniform batch analysis failed");

    // Interior points should still accurately measure velocity of 0.1 units/sec despite irregular time intervals
    for m in &metrics[8..metrics.len() - 8] {
        let v_error = (m.velocity - 0.1).abs();
        assert!(
            v_error < 0.01,
            "Non-uniform velocity error at t={}: got {}, expected 0.1",
            m.timestamp_ms,
            m.velocity
        );
        assert_eq!(m.direction, Direction::Ascending);
    }
}

#[test]
fn test_kinematics_input_validation_and_errors() {
    let config = KinematicFilterConfig::default();

    // 1. Non-monotonic timestamp
    let non_mono = vec![
        KinematicPoint::new(100, 0.5),
        KinematicPoint::new(200, 0.6),
        KinematicPoint::new(150, 0.7), // Out of order!
    ];
    let err_mono = analyze_kinematics(&non_mono, &config).unwrap_err();
    assert!(matches!(err_mono, KinematicError::NonMonotonicTimestamp { prev_ms: 200, current_ms: 150 }));

    // 2. Duplicate timestamp
    let dup = vec![
        KinematicPoint::new(100, 0.5),
        KinematicPoint::new(100, 0.6), // Duplicate!
    ];
    let err_dup = analyze_kinematics(&dup, &config).unwrap_err();
    assert!(matches!(err_dup, KinematicError::NonMonotonicTimestamp { prev_ms: 100, current_ms: 100 }));

    // 3. NaN value
    let nan_pt = vec![
        KinematicPoint::new(100, 0.5),
        KinematicPoint::new(200, f64::NAN),
    ];
    let err_nan = analyze_kinematics(&nan_pt, &config).unwrap_err();
    assert!(matches!(err_nan, KinematicError::NonFiniteValue { timestamp_ms: 200, .. }));

    // 4. Infinite value
    let inf_pt = vec![
        KinematicPoint::new(100, f64::INFINITY),
    ];
    let err_inf = analyze_kinematics(&inf_pt, &config).unwrap_err();
    assert!(matches!(err_inf, KinematicError::NonFiniteValue { timestamp_ms: 100, .. }));

    // 5. Invalid config
    let bad_config = KinematicFilterConfig {
        time_bandwidth_ms: -10.0,
        ..Default::default()
    };
    assert!(bad_config.validate().is_err());
}

#[test]
fn test_kinematics_streaming_and_batch_consistency() {
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 250.0,
        max_history_duration_ms: 4000,
        ..Default::default()
    };

    // Smooth linear ramp with gradual slope
    let points: Vec<KinematicPoint> = (0..40)
        .map(|i| {
            let t_ms = i * 100;
            let t_sec = t_ms as f64 / 1000.0;
            KinematicPoint::new(t_ms, 0.2 + 0.15 * t_sec)
        })
        .collect();

    // Run batch
    let batch_metrics = analyze_kinematics(&points, &config).expect("Batch analysis failed");

    // Run streaming tracker
    let mut tracker = KinematicSignalTracker::new(config).expect("Tracker init failed");
    let mut stream_metrics = Vec::new();
    for p in &points {
        let m = tracker.update(p.timestamp_ms, p.value).expect("Tracker update failed");
        stream_metrics.push(m);
    }

    assert_eq!(batch_metrics.len(), stream_metrics.len());

    // Compare velocities over points where lookback window has matured
    for i in 15..35 {
        let bv = batch_metrics[i].velocity;
        let sv = stream_metrics[i].velocity;
        let diff = (bv - sv).abs();
        assert!(
            diff < 0.03,
            "Batch ({}) and streaming ({}) velocity mismatch at t={}",
            bv,
            sv,
            points[i].timestamp_ms
        );
    }
}

// =========================================================================
// STAGE 2: PROGRESSION, HYSTERESIS & ADAPTIVE BASELINE VERIFICATION
// =========================================================================

#[test]
fn test_candidate_baseline_protection_during_debounce() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.50,
        min_confirmation_ms: 300,
        baseline_tau_ms: 5000.0,
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(config).expect("SM init failed");

    // Feed quiescent baseline at 0.20 for 1000ms
    for t in (0..=1000).step_by(100) {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.20,
            magnitude: 0.20,
            velocity: 0.0,
            acceleration: 0.0,
            direction: Direction::Stationary,
            volatility: 0.002,
            snr: 0.0,
            quality_score: 0.5,
        };
        sm.update(&m);
    }

    let (baseline_enter_before, _) = sm.compute_thresholds(&KinematicMetrics {
        timestamp_ms: 1000,
        raw_magnitude: 0.20,
        magnitude: 0.20,
        velocity: 0.0,
        acceleration: 0.0,
        direction: Direction::Stationary,
        volatility: 0.002,
        snr: 0.0,
        quality_score: 0.5,
    });

    // Start an ascending candidate at t=1100ms (rises to 0.85 with v=0.1)
    // Debounce duration is 300ms, so it remains candidate from 1100ms to 1300ms
    for t in [1100, 1200] {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.85,
            magnitude: 0.85,
            velocity: 0.1,
            acceleration: 0.0,
            direction: Direction::Ascending,
            volatility: 0.002,
            snr: 10.0,
            quality_score: 0.8,
        };
        sm.update(&m);
    }

    // Baseline entry threshold must NOT have been contaminated by the candidate values!
    let (baseline_enter_during, _) = sm.compute_thresholds(&KinematicMetrics {
        timestamp_ms: 1200,
        raw_magnitude: 0.85,
        magnitude: 0.85,
        velocity: 0.1,
        acceleration: 0.0,
        direction: Direction::Ascending,
        volatility: 0.002,
        snr: 10.0,
        quality_score: 0.8,
    });

    assert!(
        (baseline_enter_during - baseline_enter_before).abs() < 1e-4,
        "Baseline was contaminated during candidate transition: before {}, during {}",
        baseline_enter_before,
        baseline_enter_during
    );
}

#[test]
fn test_cold_start_already_active_signal() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.50,
        initial_condition_policy: InitialConditionPolicy::ColdStartActive,
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(config).expect("SM init failed");

    // First point arrives already elevated at 0.85 with v=0.0 (stationary plateau)
    let m0 = KinematicMetrics {
        timestamp_ms: 500,
        raw_magnitude: 0.85,
        magnitude: 0.85,
        velocity: 0.0,
        acceleration: 0.0,
        direction: Direction::Stationary,
        volatility: 0.005,
        snr: 0.0,
        quality_score: 0.8,
    };
    let pt = sm.update(&m0);

    // Initial condition policy must directly recognize active Sustained state
    assert_eq!(pt.state, ProgressionState::Sustained);
    assert_eq!(sm.current_state(), ProgressionState::Sustained);

    // Complete region by dropping below exit threshold for >= 300ms confirmation debounce
    for t in [600, 700, 800, 900, 1000, 1100, 1200] {
        let val = if t >= 800 { 0.20 } else { 0.85 };
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: val,
            magnitude: val,
            velocity: if t >= 800 { -0.1 } else { 0.0 },
            acceleration: 0.0,
            direction: if t >= 800 { Direction::Descending } else { Direction::Stationary },
            volatility: 0.005,
            snr: 1.0,
            quality_score: 0.7,
        };
        sm.update(&m);
    }

    let regions = sm.completed_regions();
    assert_eq!(regions.len(), 1, "Cold start active event should create a valid meaningful region");
    assert_eq!(regions[0].onset_ms, 500, "Onset ms should be the initial observation timestamp");
}

#[test]
fn test_exact_retroactive_candidate_onset_accounting() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.50,
        min_confirmation_ms: 300, // 300ms persistence debounce
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(config).expect("SM init failed");

    // 0ms..500ms: baseline at 0.20
    for t in (0..=500).step_by(100) {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.20,
            magnitude: 0.20,
            velocity: 0.0,
            acceleration: 0.0,
            direction: Direction::Stationary,
            volatility: 0.002,
            snr: 0.0,
            quality_score: 0.5,
        };
        sm.update(&m);
    }

    // 600ms: candidate triggers Building (magnitude 0.75, v=0.08)
    // 700ms, 800ms: candidate held
    // 900ms: confirmation commits (600ms + 300ms = 900ms)
    for t in (600..=900).step_by(100) {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.75,
            magnitude: 0.75,
            velocity: 0.08,
            acceleration: 0.0,
            direction: Direction::Ascending,
            volatility: 0.002,
            snr: 8.0,
            quality_score: 0.8,
        };
        sm.update(&m);
    }

    assert_eq!(sm.current_state(), ProgressionState::Building);

    // Conclude the region
    let regions = sm.finalize();
    assert_eq!(regions.len(), 1);
    // Retroactive onset MUST be 600ms (when candidate began), not 900ms (when confirmed)
    assert_eq!(
        regions[0].onset_ms, 600,
        "Onset ms must be retroactively set to candidate trigger timestamp (600ms)"
    );
    // Integrated mass must include the points from 600ms to 900ms
    assert!(
        regions[0].integral_mass > 0.05,
        "Integrated mass must include candidate window points"
    );
}

#[test]
fn test_hysteresis_boundary_chatter_prevention() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.60,
        hysteresis_ratio: 0.15,
        min_confirmation_ms: 200,
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(config).expect("SM init failed");

    let m0 = KinematicMetrics {
        timestamp_ms: 0,
        raw_magnitude: 0.2,
        magnitude: 0.2,
        velocity: 0.0,
        acceleration: 0.0,
        direction: Direction::Stationary,
        volatility: 0.005,
        snr: 0.0,
        quality_score: 0.5,
    };
    sm.update(&m0);
    assert_eq!(sm.current_state(), ProgressionState::BelowSignificance);

    for i in 1..=5 {
        let m = KinematicMetrics {
            timestamp_ms: i * 100,
            raw_magnitude: 0.55 + i as f64 * 0.05,
            magnitude: 0.55 + i as f64 * 0.05,
            velocity: 0.05,
            acceleration: 0.0,
            direction: Direction::Ascending,
            volatility: 0.005,
            snr: 10.0,
            quality_score: 0.8,
        };
        sm.update(&m);
    }
    assert_eq!(sm.current_state(), ProgressionState::Building);

    // Oscillate between 0.54 and 0.58 (below theta_enter 0.60, but above theta_exit 0.51)
    for i in 6..=20 {
        let val = if i % 2 == 0 { 0.54 } else { 0.58 };
        let m = KinematicMetrics {
            timestamp_ms: i * 100,
            raw_magnitude: val,
            magnitude: val,
            velocity: 0.0,
            acceleration: 0.0,
            direction: Direction::Stationary,
            volatility: 0.01,
            snr: 1.0,
            quality_score: 0.7,
        };
        sm.update(&m);

        assert!(
            sm.current_state().is_active_progression(),
            "Chatter occurred at t={}: state dropped to {:?}",
            i * 100,
            sm.current_state()
        );
    }

    // Drop below exit threshold
    for i in 21..=25 {
        let m = KinematicMetrics {
            timestamp_ms: i * 100,
            raw_magnitude: 0.40,
            magnitude: 0.40,
            velocity: -0.05,
            acceleration: 0.0,
            direction: Direction::Descending,
            volatility: 0.005,
            snr: 5.0,
            quality_score: 0.8,
        };
        sm.update(&m);
    }

    assert_eq!(sm.current_state(), ProgressionState::Subsided);
}

#[test]
fn test_temporal_persistence_debounce_spike_rejection() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.60,
        min_confirmation_ms: 300,
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(config).expect("SM init failed");

    let m0 = KinematicMetrics {
        timestamp_ms: 0,
        raw_magnitude: 0.30,
        magnitude: 0.30,
        velocity: 0.0,
        acceleration: 0.0,
        direction: Direction::Stationary,
        volatility: 0.005,
        snr: 0.0,
        quality_score: 0.5,
    };
    sm.update(&m0);
    assert_eq!(sm.current_state(), ProgressionState::BelowSignificance);

    for t in [100, 200] {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.95,
            magnitude: 0.95,
            velocity: 0.1,
            acceleration: 0.0,
            direction: Direction::Ascending,
            volatility: 0.05,
            snr: 2.0,
            quality_score: 0.6,
        };
        sm.update(&m);
        assert_eq!(sm.current_state(), ProgressionState::BelowSignificance);
    }

    let m3 = KinematicMetrics {
        timestamp_ms: 300,
        raw_magnitude: 0.30,
        magnitude: 0.30,
        velocity: -0.1,
        acceleration: 0.0,
        direction: Direction::Descending,
        volatility: 0.05,
        snr: 2.0,
        quality_score: 0.6,
    };
    sm.update(&m3);

    assert_eq!(sm.current_state(), ProgressionState::BelowSignificance);
    assert!(sm.completed_regions().is_empty());
}

#[test]
fn test_full_8_state_progression_lifecycle() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.50,
        hysteresis_ratio: 0.15,
        min_confirmation_ms: 200,
        min_sustain_ms: 1000,
        building_velocity_threshold: 0.005,
        subduing_velocity_threshold: -0.005,
        sustain_drift_threshold: 0.015,
        recovery_velocity_threshold: 0.010,
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(config).expect("SM init failed");
    let mut observed_states = Vec::new();

    let record = |sm: &mut ProgressionStateMachine, m: &KinematicMetrics, obs: &mut Vec<ProgressionState>| {
        let pt = sm.update(m);
        if obs.last() != Some(&pt.state) {
            obs.push(pt.state);
        }
    };

    // State 1: BelowSignificance (t=0..300ms)
    for t in (0..=300).step_by(100) {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.20,
            magnitude: 0.20,
            velocity: 0.0,
            acceleration: 0.0,
            direction: Direction::Stationary,
            volatility: 0.002,
            snr: 0.0,
            quality_score: 0.5,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 2: Building (t=400..900ms)
    for t in (400..=900).step_by(100) {
        let v = 0.08;
        let val = 0.50 + (t - 400) as f64 * 0.0007;
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: val,
            magnitude: val,
            velocity: v,
            acceleration: 0.0,
            direction: Direction::Ascending,
            volatility: 0.005,
            snr: 10.0,
            quality_score: 0.8,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 3: Peaking (t=1000..1200ms)
    for t in (1000..=1200).step_by(100) {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.90,
            magnitude: 0.90,
            velocity: 0.002,
            acceleration: -0.01,
            direction: Direction::Stationary,
            volatility: 0.002,
            snr: 0.5,
            quality_score: 0.9,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 4: Sustained (t=1300..2600ms)
    for t in (1300..=2600).step_by(100) {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.89,
            magnitude: 0.89,
            velocity: 0.0,
            acceleration: 0.0,
            direction: Direction::Stationary,
            volatility: 0.002,
            snr: 0.0,
            quality_score: 0.95,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 5: Subduing (t=2700..3200ms)
    for t in (2700..=3200).step_by(100) {
        let val = 0.89 - (t - 2600) as f64 * 0.0005;
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: val,
            magnitude: val,
            velocity: -0.05,
            acceleration: 0.0,
            direction: Direction::Descending,
            volatility: 0.005,
            snr: 5.0,
            quality_score: 0.8,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 6: Recovering (t=3300..3700ms)
    for t in (3300..=3700).step_by(100) {
        let val = 0.60 + (t - 3200) as f64 * 0.0002;
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: val,
            magnitude: val,
            velocity: 0.02,
            acceleration: 0.0,
            direction: Direction::Ascending,
            volatility: 0.005,
            snr: 4.0,
            quality_score: 0.8,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // Resuming descent back to Subduing (t=3800..4100ms)
    for t in (3800..=4100).step_by(100) {
        let val = 0.68 - (t - 3700) as f64 * 0.0006;
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: val,
            magnitude: val,
            velocity: -0.06,
            acceleration: 0.0,
            direction: Direction::Descending,
            volatility: 0.005,
            snr: 6.0,
            quality_score: 0.8,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 7: Subsided (t=4200..4600ms)
    for t in (4200..=4600).step_by(100) {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.30,
            magnitude: 0.30,
            velocity: -0.02,
            acceleration: 0.0,
            direction: Direction::Descending,
            volatility: 0.005,
            snr: 2.0,
            quality_score: 0.7,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 8: Stable (t=4700..5100ms)
    for t in (4700..=5100).step_by(100) {
        let m = KinematicMetrics {
            timestamp_ms: t,
            raw_magnitude: 0.15,
            magnitude: 0.15,
            velocity: 0.0,
            acceleration: 0.0,
            direction: Direction::Stationary,
            volatility: 0.001,
            snr: 0.0,
            quality_score: 0.6,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    assert!(observed_states.contains(&ProgressionState::BelowSignificance));
    assert!(observed_states.contains(&ProgressionState::Building));
    assert!(observed_states.contains(&ProgressionState::Peaking));
    assert!(observed_states.contains(&ProgressionState::Sustained));
    assert!(observed_states.contains(&ProgressionState::Subduing));
    assert!(observed_states.contains(&ProgressionState::Recovering));
    assert!(observed_states.contains(&ProgressionState::Subsided));
    assert!(observed_states.contains(&ProgressionState::Stable));

    let regions = sm.completed_regions();
    assert_eq!(regions.len(), 1);
    let r = &regions[0];
    assert!(r.peak_magnitude >= 0.89);
    assert!(r.sustained_duration_ms >= 1000);
    assert!(r.integral_mass > 0.5);
    assert!(r.quality_score > 0.5);
}

#[test]
fn test_spike_rejection_vs_sustained_movement_contrast() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.50,
        min_confirmation_ms: 200,
        min_sustain_ms: 1000,
        ..Default::default()
    };

    // Scenario A: Transient Sharp Spike (0.98 peak, only 400ms)
    let mut spike_points = Vec::new();
    for t in (0..=2000).step_by(100) {
        let val = if t == 500 || t == 600 || t == 700 {
            0.98
        } else {
            0.20
        };
        spike_points.push(KinematicPoint::new(t, val));
    }
    let k_config = KinematicFilterConfig::default();
    let spike_kinematics = analyze_kinematics(&spike_points, &k_config).unwrap();
    let (_, spike_regions) = analyze_progression(&spike_kinematics, &config).unwrap();

    // Scenario B: Sustained Musical Movement (0.80 peak, 8,000ms duration)
    let mut sustained_points = Vec::new();
    for t in (0..=12000).step_by(200) {
        let val = if t >= 2000 && t <= 10000 {
            0.80
        } else {
            0.20
        };
        sustained_points.push(KinematicPoint::new(t, val));
    }
    let sustained_kinematics = analyze_kinematics(&sustained_points, &k_config).unwrap();
    let (_, sustained_regions) = analyze_progression(&sustained_kinematics, &config).unwrap();

    assert_eq!(sustained_regions.len(), 1);
    let b_region = &sustained_regions[0];

    let a_mass = spike_regions.first().map(|r| r.integral_mass).unwrap_or(0.0);
    let a_sustain = spike_regions.first().map(|r| r.sustained_duration_ms).unwrap_or(0);
    let a_qual = spike_regions.first().map(|r| r.quality_score).unwrap_or(0.0);

    assert!(
        b_region.integral_mass > a_mass * 10.0,
        "Sustained mass ({}) must be > 10x spike mass ({})",
        b_region.integral_mass,
        a_mass
    );
    assert!(
        b_region.sustained_duration_ms > a_sustain,
        "Sustained duration ({}ms) must exceed spike ({}ms)",
        b_region.sustained_duration_ms,
        a_sustain
    );
    assert!(
        b_region.quality_score > a_qual,
        "Sustained quality ({}) must exceed spike ({})",
        b_region.quality_score,
        a_qual
    );
}

#[test]
fn test_multiple_meaningful_regions_segmentation() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.50,
        min_confirmation_ms: 200,
        min_sustain_ms: 1000,
        ..Default::default()
    };
    let k_config = KinematicFilterConfig::default();

    let mut points = Vec::new();
    for t in (0..=15000).step_by(200) {
        let val = if t >= 2000 && t <= 5000 {
            0.80
        } else if t >= 9000 && t <= 13000 {
            0.85
        } else {
            0.20
        };
        points.push(KinematicPoint::new(t, val));
    }

    let kinematics = analyze_kinematics(&points, &k_config).unwrap();
    let (_, regions) = analyze_progression(&kinematics, &config).unwrap();

    assert_eq!(regions.len(), 2, "Should segment exactly two distinct meaningful regions");
    assert_eq!(regions[0].region_id, 1);
    assert!(regions[0].onset_ms >= 1800 && regions[0].onset_ms <= 2400);
    assert!(regions[0].offset_ms >= 5000 && regions[0].offset_ms <= 6000);

    assert_eq!(regions[1].region_id, 2);
    assert!(regions[1].onset_ms >= 8800 && regions[1].onset_ms <= 9400);
    assert!(regions[1].offset_ms >= 13000 && regions[1].offset_ms <= 14000);
}

// =========================================================================
// STAGE 3: UNCERTAINTY, PATTERN RECOGNITION & PRIORITIZATION VERIFICATION
// =========================================================================

#[test]
fn test_stage3_uncertainty_quantification() {
    let config = UncertaintyConfig {
        staleness_half_life_seconds: 4.0,
        target_sample_count: 10,
        interval_multiplier_z: 1.96,
        min_noise_floor: 1e-4,
        nominal_scale: 1.0,
    };

    // Case 1: Fresh observation with high sample support and low noise
    let fresh = quantify_uncertainty(0.70, 0.01, 10, 0.0, &config);
    assert!(fresh.freshness > 0.99);
    assert_eq!(fresh.sample_coverage, 1.0);
    assert!(fresh.epistemic_uncertainty < 0.05);
    assert!(fresh.lower_bound < 0.70 && fresh.upper_bound > 0.70);
    assert!(fresh.upper_bound - fresh.lower_bound < 0.06);

    // Case 2: Stale observation (8 seconds elapsed = 2 half-lives)
    let stale = quantify_uncertainty(0.70, 0.01, 10, 8.0, &config);
    assert!(stale.freshness < 0.30);
    assert!(stale.epistemic_uncertainty > 0.30);
    assert!(stale.total_uncertainty > fresh.total_uncertainty);
    // Credible interval should expand under staleness
    assert!(stale.upper_bound - stale.lower_bound > fresh.upper_bound - fresh.lower_bound);

    // Case 3: Sparse observation (only 2 samples out of 10 target)
    let sparse = quantify_uncertainty(0.70, 0.01, 2, 0.0, &config);
    assert_eq!(sparse.sample_coverage, 0.2);
    assert!(sparse.epistemic_uncertainty >= 0.40);
    assert!(sparse.upper_bound - sparse.lower_bound > fresh.upper_bound - fresh.lower_bound);
}

#[test]
fn test_stage3_pattern_archetype_recognition() {
    // 1. Monotonic Ascent: consecutive ascending metrics
    let ascent: Vec<KinematicMetrics> = (0..10)
        .map(|i| KinematicMetrics {
            timestamp_ms: i * 100,
            raw_magnitude: 0.1 * i as f64,
            magnitude: 0.1 * i as f64,
            velocity: 0.1,
            acceleration: 0.0,
            direction: Direction::Ascending,
            volatility: 0.001,
            snr: 10.0,
            quality_score: 0.9,
        })
        .collect();
    assert_eq!(classify_archetype(&ascent, 0.01), ProgressionArchetype::MonotonicAscent);

    // 2. Monotonic Descent: consecutive descending metrics
    let descent: Vec<KinematicMetrics> = (0..10)
        .map(|i| KinematicMetrics {
            timestamp_ms: i * 100,
            raw_magnitude: 1.0 - 0.1 * i as f64,
            magnitude: 1.0 - 0.1 * i as f64,
            velocity: -0.1,
            acceleration: 0.0,
            direction: Direction::Descending,
            volatility: 0.001,
            snr: 10.0,
            quality_score: 0.9,
        })
        .collect();
    assert_eq!(classify_archetype(&descent, 0.01), ProgressionArchetype::MonotonicDescent);

    // 3. Plateau: sustained high value with low velocity
    let plateau: Vec<KinematicMetrics> = (0..10)
        .map(|i| KinematicMetrics {
            timestamp_ms: i * 100,
            raw_magnitude: 0.85,
            magnitude: 0.85,
            velocity: 0.002,
            acceleration: 0.0,
            direction: Direction::Stationary,
            volatility: 0.001,
            snr: 0.2,
            quality_score: 0.9,
        })
        .collect();
    assert_eq!(classify_archetype(&plateau, 0.01), ProgressionArchetype::Plateau);

    // 4. Transient Spike: fast rise and fall with low duration
    let mut spike = Vec::new();
    for i in 0..7 {
        let val = if i == 3 { 0.95 } else { 0.20 };
        let v = if i < 3 { 0.25 } else if i == 3 { 0.0 } else { -0.25 };
        spike.push(KinematicMetrics {
            timestamp_ms: i * 100,
            raw_magnitude: val,
            magnitude: val,
            velocity: v,
            acceleration: 0.0,
            direction: if v > 0.01 { Direction::Ascending } else if v < -0.01 { Direction::Descending } else { Direction::Stationary },
            volatility: 0.01,
            snr: 5.0,
            quality_score: 0.5,
        });
    }
    assert_eq!(classify_archetype(&spike, 0.01), ProgressionArchetype::TransientSpike);
}

#[test]
fn test_stage3_trajectory_projection_damped() {
    let u_config = UncertaintyConfig::default();
    let unc = quantify_uncertainty(0.60, 0.02, 5, 0.0, &u_config);

    let current = KinematicMetrics {
        timestamp_ms: 1000,
        raw_magnitude: 0.60,
        magnitude: 0.60,
        velocity: 0.05,
        acceleration: -0.01,
        direction: Direction::Ascending,
        volatility: 0.02,
        snr: 2.5,
        quality_score: 0.8,
    };

    // Project 2 seconds into future with damping
    let proj = project_trajectory(&current, ProgressionArchetype::CrescendoPeak, 2.0, 0.2, &unc);

    assert_eq!(proj.horizon_seconds, 2.0);
    assert!(proj.projected_value > 0.60);
    assert!(proj.projected_velocity < current.velocity, "Velocity should be damped towards zero");
    assert!(proj.lower_bound < proj.projected_value && proj.upper_bound > proj.projected_value);
    assert!(proj.projection_confidence > 0.0 && proj.projection_confidence <= 1.0);
}

#[test]
fn test_stage3_decision_prioritizer_robust_ranking() {
    let p_config = PrioritizerConfig {
        risk_aversion_k: 0.5,
    };

    // Candidate A: High nominal score (0.85) but VERY high uncertainty (0.60)
    // Robust score = 0.85 - 0.5 * 0.60 = 0.55
    let cand_a = (
        "Candidate_A",
        vec![
            DecisionCriterion::hard("SafetyCheck", 1.0, 0.5),
            DecisionCriterion::soft("Score", 0.85, 1.0, 0.60),
        ],
    );

    // Candidate B: Slightly lower nominal score (0.75) but rock-solid low uncertainty (0.05)
    // Robust score = 0.75 - 0.5 * 0.05 = 0.725
    let cand_b = (
        "Candidate_B",
        vec![
            DecisionCriterion::hard("SafetyCheck", 1.0, 0.5),
            DecisionCriterion::soft("Score", 0.75, 1.0, 0.05),
        ],
    );

    // Candidate C: Disqualified by hard safety constraint
    let cand_c = (
        "Candidate_C",
        vec![
            DecisionCriterion::hard("SafetyCheck", 0.2, 0.5), // Fails min 0.5!
            DecisionCriterion::soft("Score", 0.99, 1.0, 0.0),
        ],
    );

    let ranked = rank_candidates(vec![cand_a, cand_b, cand_c], &p_config);

    assert_eq!(ranked.len(), 3);
    // Candidate B must outrank Candidate A due to robust uncertainty penalty
    assert_eq!(ranked[0].payload, "Candidate_B");
    assert!(ranked[0].is_eligible);
    assert_eq!(ranked[1].payload, "Candidate_A");
    assert!(ranked[1].is_eligible);
    // Candidate C must be ranked last and marked ineligible
    assert_eq!(ranked[2].payload, "Candidate_C");
    assert!(!ranked[2].is_eligible);
}

// =========================================================================
// STAGE 4: PRODUCTION INTEGRATION VERIFICATION
// =========================================================================

#[test]
fn test_stage4_set_storyteller_kinematics_and_progression() {
    let mut storyteller = SetStoryteller::new("baraat", 20);

    // Baraat warmup target: 0.78, buildup: 0.88, peak: 0.98
    // Simulate playing consecutive tracks escalating energy
    storyteller.record_track_played(0.75, false, false);
    storyteller.record_track_played(0.80, false, false);
    storyteller.record_track_played(0.86, false, false);
    storyteller.record_track_played(0.92, false, false);

    assert_eq!(storyteller.history_energies.len(), 4);
    assert!(storyteller.last_kinematic_metrics.is_some());
    let last_m = storyteller.last_kinematic_metrics.unwrap();
    // Velocity should be positive during escalation
    assert!(last_m.velocity > 0.0, "Escalating energy should produce positive kinematic velocity");

    // Evaluate candidate that continues upward momentum
    let profile = SectionEnergyProfile {
        intro_energy: 0.85,
        mix_in_energy: 0.90,
        body_energy: 0.95,
        peak_energy: 0.98,
        mix_out_energy: 0.88,
        has_iconic_hook: true,
        hook_sections: vec![],
        dynamic_range_db: 6.0,
    };

    let (score, reasons, _) = storyteller.evaluate_candidate_energy_fit(&profile, 0.95);
    assert!(score > 20.0);
    assert!(reasons.iter().any(|r| r.contains("Progression Intelligence") || r.contains("Peak") || r.contains("escalates")));
}

#[test]
fn test_stage4_hook_usability_with_meaningful_regions() {
    let mut timing_profile = TrackTimingProfile::default();
    timing_profile.downbeat_timestamps_ms = vec![50000, 55000, 60000, 65000, 70000, 75000, 80000];

    // Candidate 1: Falls inside a sustained meaningful region at 60s (past 15s intro guard)
    let c1 = HookCandidate {
        id: "c1".to_string(),
        track_id: "test_t1".to_string(),
        video_id: Some("v1".to_string()),
        video_start_ms: Some(60000),
        video_end_ms: Some(75000),
        audio_start_ms: 60000,
        audio_end_ms: 75000,
        dhun_musical_score: 0.85,
        audience_replay_score: Some(0.80),
        combined_hook_score: 0.82,
        alignment_status: TimelineAlignmentStatus::Verified,
        alignment_offset_ms: 0,
        dj_usability_score: 0.0,
        is_audience_popular: false,
        is_musical_hook: false,
        is_aligned_hook: false,
        is_usable_entry: false,
        is_usable_exit: false,
        candidate_type: "chorus".to_string(),
        entry_cue_ms: 0,
        exit_cue_ms: 0,
        usability_reasons: vec![],
        rejection_reasons: vec![],
    };

    let sustained_region = MeaningfulRegion {
        region_id: 1,
        onset_ms: 58000,
        peak_ms: 65000,
        offset_ms: 78000,
        duration_ms: 20000,
        sustained_duration_ms: 12000,
        peak_magnitude: 0.85,
        mean_magnitude: 0.80,
        integral_mass: 2.5,
        progression_state: ProgressionState::Sustained,
        quality_score: 0.88,
        is_complete: true,
    };

    let mut candidates = vec![c1];
    HookUsabilityEvaluator::evaluate_candidates_with_regions(
        &mut candidates,
        &timing_profile,
        &[],
        200000,
        &[sustained_region],
    );

    let res = &candidates[0];
    assert!(res.is_usable_entry);
    assert!(res.usability_reasons.iter().any(|r| r.contains("Audience Intelligence: Verified sustained replay progression")));
    assert!(res.dj_usability_score >= 0.80);
}

#[test]
fn test_stage4_selector_robust_selection() {
    let moods = dhun_lib::dj::moods::get_preset_moods();
    let mood = moods.into_iter().find(|m| m.id == "baraat").unwrap();

    let track_a = Track {
        id: "tr_a".to_string(),
        title: "Track A".to_string(),
        artist: "Artist A".to_string(),
        album: None,
        duration_ms: 180000,
        year: None,
        genre: "Bhangra".to_string(),
        subgenre: None,
        created_at: 0,
        updated_at: 0,
    };
    let analysis_a = TrackAnalysis {
        track_id: "tr_a".to_string(),
        bpm: 130.0,
        bpm_confidence: 0.95,
        key_name: "G#m".to_string(),
        camelot_key: "1A".to_string(),
        key_confidence: 0.95,
        time_signature_num: 4,
        time_signature_den: 4,
        downbeat_offset_ms: 0,
        phrase_length_bars: 8,
        overall_energy: 0.90,
        dance_suitability: 0.90,
        recommended_mix_in_ms: 0,
        recommended_mix_out_ms: 170000,
        analysis_version: 1,
        analyzed_at: 0,
        manual_override: false,
    };
    let asset_a = dhun_lib::db::models::AudioAsset {
        id: "asset_a".to_string(),
        track_id: "tr_a".to_string(),
        file_path: "track_a.mp3".to_string(),
        file_format: "mp3".to_string(),
        file_size_bytes: 5000000,
        sample_rate: 44100,
        bit_depth: Some(16),
        channels: 2,
        sha256_hash: "hash_a".to_string(),
        verification_status: "verified".to_string(),
        availability_status: "available".to_string(),
        last_verified_at: 0,
    };
    let cand_a = PlayableTrackCandidate {
        track: track_a,
        asset: asset_a,
        analysis: analysis_a,
        compatibility_score: 0.0,
        transition_style: "beatmatch".to_string(),
    };

    let track_b = Track {
        id: "tr_b".to_string(),
        title: "Track B".to_string(),
        artist: "Artist B".to_string(),
        album: None,
        duration_ms: 180000,
        year: None,
        genre: "Ambient".to_string(), // Mismatched genre for Baraat
        subgenre: None,
        created_at: 0,
        updated_at: 0,
    };
    let analysis_b = TrackAnalysis {
        track_id: "tr_b".to_string(),
        bpm: 70.0,
        bpm_confidence: 0.40,
        key_name: "C".to_string(),
        camelot_key: "8B".to_string(),
        key_confidence: 0.40,
        time_signature_num: 4,
        time_signature_den: 4,
        downbeat_offset_ms: 0,
        phrase_length_bars: 8,
        overall_energy: 0.30,
        dance_suitability: 0.20,
        recommended_mix_in_ms: 0,
        recommended_mix_out_ms: 170000,
        analysis_version: 1,
        analyzed_at: 0,
        manual_override: false,
    };
    let asset_b = dhun_lib::db::models::AudioAsset {
        id: "asset_b".to_string(),
        track_id: "tr_b".to_string(),
        file_path: "track_b.mp3".to_string(),
        file_format: "mp3".to_string(),
        file_size_bytes: 5000000,
        sample_rate: 44100,
        bit_depth: Some(16),
        channels: 2,
        sha256_hash: "hash_b".to_string(),
        verification_status: "verified".to_string(),
        availability_status: "available".to_string(),
        last_verified_at: 0,
    };
    let cand_b = PlayableTrackCandidate {
        track: track_b,
        asset: asset_b,
        analysis: analysis_b,
        compatibility_score: 0.0,
        transition_style: "beatmatch".to_string(),
    };

    let p_config = PrioritizerConfig::default();
    let selected = TrackSelector::score_and_select_robust(
        &[cand_a, cand_b],
        None,
        &mood,
        None,
        0.90,
        &[],
        &[],
        &[],
        None,
        &p_config,
    );

    assert!(selected.is_some());
    let chosen = selected.unwrap();
    assert_eq!(chosen.track.id, "tr_a", "Bhangra high-energy track must be selected over mismatched ambient track");
    assert!(chosen.compatibility_score > 0.70);
}

// =========================================================================
// STAGE 5: ADAPTIVE DATASET & CONTINUOUS VALIDATION PROTOCOL
// =========================================================================

/// Deterministic, reproducible linear congruential generator for seed-based continuous testing.
struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }

    fn next_range(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.next_f64()
    }

    fn next_gaussian(&mut self) -> f64 {
        // Box-Muller transform
        let u1 = self.next_f64().max(1e-15);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
    }
}

#[test]
fn test_protocol_category_a_randomized_datasets() {
    // Dataset ID: DS_A_RANDOM_20261121_01
    // Seed: 0x5EED_0001_A
    let mut rng = LcgRng::new(0x5EED_0001_A);
    let trials = 50;

    for trial in 0..trials {
        let nominal_scale = (10.0f64).powf(rng.next_range(-2.0, 3.0)); // 0.01 to 1000
        let base_dt_ms = rng.next_range(20.0, 200.0) as i64;
        let num_points = rng.next_range(30.0, 100.0) as usize;
        let noise_sigma = rng.next_range(0.001, 0.1) * nominal_scale;

        let mut points = Vec::with_capacity(num_points);
        let mut cur_t = 0i64;
        let mut cur_val = nominal_scale * rng.next_range(0.2, 0.8);
        let drift = nominal_scale * rng.next_range(-0.05, 0.05);

        for _ in 0..num_points {
            let val = cur_val + noise_sigma * rng.next_gaussian();
            points.push(KinematicPoint::new(cur_t, val));
            cur_t += base_dt_ms + (rng.next_range(-5.0, 5.0) as i64);
            cur_val += drift * (base_dt_ms as f64 / 1000.0);
        }

        let config = KinematicFilterConfig {
            time_bandwidth_ms: base_dt_ms as f64 * 3.0,
            nominal_scale,
            noise_floor: 1e-6 * nominal_scale,
            ..Default::default()
        };

        let metrics = analyze_kinematics(&points, &config)
            .unwrap_or_else(|e| panic!("Trial {} failed with error {:?}", trial, e));

        assert_eq!(metrics.len(), num_points);
        for m in &metrics {
            assert!(m.magnitude.is_finite());
            assert!(m.velocity.is_finite());
            assert!(m.acceleration.is_finite());
            assert!(m.snr >= 0.0 && m.snr.is_finite());
            assert!(m.quality_score >= 0.0 && m.quality_score <= 1.0);
        }
    }
}

#[test]
fn test_protocol_category_b_adversarial_datasets() {
    // Dataset ID: DS_B_ADVERSARIAL_20261121_01
    // Seed: 0x5EED_0002_B
    let mut rng = LcgRng::new(0x5EED_0002_B);

    // Adversarial Case 1: Extreme single-sample impulse spike (100x signal amplitude)
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 300.0,
        ..Default::default()
    };
    let mut points: Vec<KinematicPoint> = (0..50)
        .map(|i| KinematicPoint::new(i * 100, 0.20 + 0.005 * rng.next_gaussian()))
        .collect();
    points[25].value = 20.0; // massive adversarial impulse spike

    let metrics = analyze_kinematics(&points, &config).expect("Must survive extreme impulse");
    let mut sm = ProgressionStateMachine::new(AdaptiveThresholdConfig::default()).unwrap();
    let mut regions = Vec::new();
    for m in &metrics {
        sm.update(m);
    }
    regions.extend(sm.finalize());

    // Adversarial verification: an isolated spike must NEVER be confirmed as a Sustained region
    for r in &regions {
        assert_ne!(
            r.progression_state,
            ProgressionState::Sustained,
            "Isolated impulse spike must not produce a Sustained progression"
        );
        assert!(r.duration_ms < 1500, "Spike duration must be transient");
    }

    // Adversarial Case 2: Multi-cycle oscillation (period 1.2s, 100ms sampling)
    let mut osc_points = Vec::with_capacity(60);
    for i in 0..60 {
        let t_sec = (i * 100) as f64 / 1000.0;
        let val = 0.5 + 0.3 * (2.0 * PI * t_sec / 1.2).sin();
        osc_points.push(KinematicPoint::new(i * 100, val));
    }
    let osc_metrics = analyze_kinematics(&osc_points, &config).expect("Must survive oscillation");
    let archetype = classify_archetype(&osc_metrics, 0.01);
    assert_eq!(
        archetype,
        ProgressionArchetype::CyclicalOscillation,
        "Alternating oscillation must be recognized as CyclicalOscillation"
    );
}

#[test]
fn test_protocol_category_c_boundary_datasets() {
    // Dataset ID: DS_C_BOUNDARY_20261121_01
    // Seed: 0x5EED_0003_C
    let threshold_config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.50,
        hysteresis_ratio: 0.15,
        min_confirmation_ms: 300,
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(threshold_config).unwrap();

    // Boundary Case 1: Skimming exactly at threshold boundary (0.500001 then 0.499999)
    // Hysteresis must prevent rapid chatter
    let (theta_enter, theta_exit) = sm.compute_thresholds(&KinematicMetrics {
        timestamp_ms: 0,
        raw_magnitude: 0.50,
        magnitude: 0.50,
        velocity: 0.0,
        acceleration: 0.0,
        direction: Direction::Stationary,
        volatility: 0.0,
        snr: 0.0,
        quality_score: 1.0,
    });
    assert!((theta_enter - 0.50).abs() < 1e-6);
    assert!((theta_exit - 0.425).abs() < 1e-6); // 15% hysteresis band below 0.50

    // Feed oscillations between 0.45 and 0.51 (within hysteresis band)
    let mut state_flips = 0;
    let mut prev_state = sm.current_state();
    for i in 0..30 {
        let val = if i % 2 == 0 { 0.51 } else { 0.46 }; // above exit, skimming entry
        let m = KinematicMetrics {
            timestamp_ms: i * 50, // 50ms intervals (< 300ms confirmation)
            raw_magnitude: val,
            magnitude: val,
            velocity: if i % 2 == 0 { 0.02 } else { -0.02 },
            acceleration: 0.0,
            direction: Direction::Ascending,
            volatility: 0.01,
            snr: 2.0,
            quality_score: 0.8,
        };
        let pt = sm.update(&m);
        if pt.state != prev_state {
            state_flips += 1;
            prev_state = pt.state;
        }
    }
    // Debounce persistence and hysteresis must suppress chatter: state flips must be 0 or at most 1
    assert!(
        state_flips <= 1,
        "Hysteresis and debounce failed: observed {} state flips in boundary band",
        state_flips
    );
}

#[test]
fn test_protocol_category_d_generalization_datasets() {
    // Dataset ID: DS_D_GENERALIZATION_20261121_01
    // Seed: 0x5EED_0004_D
    // Generalization: Non-linear Sigmoidal Crescendo on unnormalized scale [0, 500]
    let scale = 500.0;
    let k_config = KinematicFilterConfig {
        time_bandwidth_ms: 250.0,
        nominal_scale: scale,
        noise_floor: 1e-4 * scale,
        ..Default::default()
    };

    let points: Vec<KinematicPoint> = (0..80)
        .map(|i| {
            let t_ms = i * 100;
            let t_sec = t_ms as f64 / 1000.0;
            // Logistic sigmoid: x(t) = scale / (1 + exp(-2 * (t - 4.0)))
            let val = scale / (1.0 + (-2.0 * (t_sec - 4.0)).exp());
            KinematicPoint::new(t_ms, val)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &k_config).expect("Sigmoid analysis failed");

    // Kinematic validation: peak velocity must occur around inflection point t = 4.0s (index 40)
    let max_v_metric = metrics
        .iter()
        .max_by(|a, b| a.velocity.partial_cmp(&b.velocity).unwrap())
        .unwrap();
    let inflection_t_sec = max_v_metric.timestamp_ms as f64 / 1000.0;
    assert!(
        (inflection_t_sec - 4.0).abs() <= 0.3,
        "Inflection point expected at ~4.0s, got {:.2}s",
        inflection_t_sec
    );

    // Damped trajectory projection must remain bounded
    let u_config = UncertaintyConfig {
        nominal_scale: scale,
        ..Default::default()
    };
    let unc = quantify_uncertainty(max_v_metric.magnitude, max_v_metric.volatility, 10, 0.0, &u_config);
    let projection = project_trajectory(max_v_metric, ProgressionArchetype::CrescendoPeak, 2.0, 0.5, &unc);

    assert!(projection.projected_value >= 0.0);
    assert!(projection.lower_bound <= projection.projected_value);
    assert!(projection.upper_bound >= projection.projected_value);
    assert!(projection.projection_confidence > 0.0 && projection.projection_confidence <= 1.0);
}

#[test]
fn test_protocol_category_e_stability_long_duration() {
    // Dataset ID: DS_E_STABILITY_20261121_02
    // Seed: 0x5EED_0005_E002
    // Continuous 5,000-point streaming simulation with drifting baseline and hard bounded-memory verification
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 200.0,
        max_history_duration_ms: 2000, // 2s lookback window
        ..Default::default()
    };
    let mut tracker = KinematicSignalTracker::new(config).unwrap();

    let mut sm = ProgressionStateMachine::new(AdaptiveThresholdConfig {
        baseline_tau_ms: 5000.0,
        ..Default::default()
    }).unwrap();

    let mut rng = LcgRng::new(0x5EED_0005_E002);
    let mut t_ms = 0i64;
    let mut baseline = 0.20;

    for i in 0..5000 {
        t_ms += 50;
        if i % 1000 == 0 {
            baseline += 0.05; // slow baseline drift
        }
        let noise = 0.01 * rng.next_gaussian();
        let val = baseline + noise;

        let m = tracker.update(t_ms, val).expect("Tracker must remain stable");
        let pt = sm.update(&m);

        assert!(m.magnitude.is_finite());
        assert!(m.velocity.is_finite());
        assert!(pt.theta_enter >= baseline - 0.05);

        // Hard Bounded Memory Assertion:
        // History buffer must strictly prune points older than 2000ms.
        // At 50ms per step, maximum retained samples is 2000 / 50 + 2 = 42 samples.
        let h_len = tracker.history_len();
        assert!(
            h_len <= 45,
            "History memory leaked at step {}: length {} exceeds bound 45",
            i,
            h_len
        );
    }
}

// =========================================================================
// PHASE 1 VERIFICATION: TARGETED MATHEMATICAL & ENGINEERING SUITES
// =========================================================================

#[test]
fn test_dispersion_filtering_lag_rejection() {
    // Linear ramp with zero noise: LOWESS guarantees exact reproduction, so centered residual dispersion must be near 0
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 200.0,
        ..Default::default()
    };
    let points: Vec<KinematicPoint> = (0..50)
        .map(|i| KinematicPoint::new(i * 100, 0.2 + 0.1 * (i as f64 / 10.0)))
        .collect();

    let metrics = analyze_kinematics(&points, &config).expect("Analysis failed");
    for m in &metrics[5..metrics.len() - 5] {
        assert!(
            m.volatility < 1e-4,
            "Linear ramp filtering lag incorrectly produced volatility at t={}: got {}",
            m.timestamp_ms,
            m.volatility
        );
    }
}

#[test]
fn test_dispersion_smooth_curvature() {
    // Pure quadratic curve: x(t) = 0.5 * a * t^2 with zero noise
    let a_theo = 0.1;
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 200.0,
        ..Default::default()
    };
    let points: Vec<KinematicPoint> = (0..50)
        .map(|i| {
            let t = (i * 100) as f64 / 1000.0;
            KinematicPoint::new(i * 100, 0.5 * a_theo * t * t)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config).expect("Analysis failed");
    // Residual dispersion reflects second-order curvature deviation from local line, but remains bounded
    for m in &metrics[10..metrics.len() - 10] {
        assert!(m.volatility < 0.01, "Curvature dispersion exceeded bound: got {}", m.volatility);
    }
}

#[test]
fn test_dispersion_genuine_observation_noise() {
    // Constant signal with known additive Gaussian noise sigma = 0.05
    let mut rng = LcgRng::new(0xBEEF_1234);
    let true_sigma = 0.05;
    let config = KinematicFilterConfig {
        time_bandwidth_ms: 300.0,
        ..Default::default()
    };
    let points: Vec<KinematicPoint> = (0..100)
        .map(|i| KinematicPoint::new(i * 100, 0.50 + true_sigma * rng.next_gaussian()))
        .collect();

    let metrics = analyze_kinematics(&points, &config).expect("Analysis failed");
    let mid = &metrics[30..70];
    let avg_vol: f64 = mid.iter().map(|m| m.volatility).sum::<f64>() / mid.len() as f64;
    let rel_err = (avg_vol - true_sigma).abs() / true_sigma;
    assert!(
        rel_err < 0.25,
        "Measured residual dispersion {:.4} deviated by {:.1}% from true noise sigma {:.4}",
        avg_vol,
        rel_err * 100.0,
        true_sigma
    );
}

#[test]
fn test_empirical_coverage_rate() {
    // Generate 500 trials with true Gaussian noise. Check empirical coverage of the uncertainty interval.
    let mut rng = LcgRng::new(0xC0DE_5678);
    let u_config = UncertaintyConfig {
        interval_multiplier_z: 1.96, // 95% nominal Gaussian coverage
        ..Default::default()
    };
    let mut covered = 0;
    let trials = 500;
    let true_val = 0.75;
    let noise_std = 0.04;

    for _ in 0..trials {
        let obs = true_val + noise_std * rng.next_gaussian();
        let unc = quantify_uncertainty(obs, noise_std, 5, 0.0, &u_config);
        if unc.contains(true_val) {
            covered += 1;
        }
    }

    let coverage_rate = covered as f64 / trials as f64;
    assert!(
        coverage_rate >= 0.90,
        "Empirical coverage rate {:.2}% fell below 90% target",
        coverage_rate * 100.0
    );
}

#[test]
fn test_uncertainty_input_validation() {
    let u_config = UncertaintyConfig::default();
    use dhun_lib::dj::intelligence::try_quantify_uncertainty;

    // Reject NaN
    assert!(try_quantify_uncertainty(f64::NAN, 0.01, 5, 0.0, &u_config).is_err());
    // Reject Infinity
    assert!(try_quantify_uncertainty(1.0, f64::INFINITY, 5, 0.0, &u_config).is_err());
    // Reject negative duration
    assert!(try_quantify_uncertainty(1.0, 0.01, 5, -1.0, &u_config).is_err());
}

#[test]
fn test_prioritizer_criterion_order_permutation_invariance() {
    let config = PrioritizerConfig::default();

    let c_bpm = DecisionCriterion::soft("bpm", 0.9, 0.3, 0.05);
    let c_key = DecisionCriterion::soft("key", 0.8, 0.3, 0.10);
    let c_energy = DecisionCriterion::soft("energy", 0.7, 0.4, 0.15);

    // Candidate A with order [bpm, key, energy]
    let cand_a1 = ("cand_a", vec![c_bpm.clone(), c_key.clone(), c_energy.clone()]);
    let cand_b1 = ("cand_b", vec![
        DecisionCriterion::soft("bpm", 0.6, 0.3, 0.1),
        DecisionCriterion::soft("key", 0.6, 0.3, 0.1),
        DecisionCriterion::soft("energy", 0.6, 0.4, 0.1),
    ]);

    // Permuted order: Candidate A with [energy, bpm, key], Candidate B with [key, energy, bpm]
    let cand_a2 = ("cand_a", vec![c_energy, c_bpm, c_key]);
    let cand_b2 = ("cand_b", vec![
        DecisionCriterion::soft("key", 0.6, 0.3, 0.1),
        DecisionCriterion::soft("energy", 0.6, 0.4, 0.1),
        DecisionCriterion::soft("bpm", 0.6, 0.3, 0.1),
    ]);

    let res1 = rank_candidates(vec![cand_a1, cand_b1], &config);
    let res2 = rank_candidates(vec![cand_a2, cand_b2], &config);

    assert_eq!(res1[0].payload, res2[0].payload);
    assert!((res1[0].composite_score - res2[0].composite_score).abs() < 1e-6);
    assert!((res1[0].robust_score - res2[0].robust_score).abs() < 1e-6);
    assert_eq!(res1[0].pareto_rank, res2[0].pareto_rank);
}

#[test]
fn test_prioritizer_detect_duplicate_and_missing_criteria() {
    let config = PrioritizerConfig::default();

    // Candidate with duplicate criteria name
    let duplicate_cand = (
        "dup_cand",
        vec![
            DecisionCriterion::soft("bpm", 0.9, 0.5, 0.05),
            DecisionCriterion::soft("bpm", 0.8, 0.5, 0.05), // Duplicate name
        ],
    );
    let res = rank_candidates(vec![duplicate_cand], &config);
    assert!(!res[0].is_eligible, "Duplicate criteria must trigger disqualification");

    // Candidate with missing criteria across candidates: Candidate A has [bpm, key], Candidate B has [bpm]
    let cand_a = ("cand_a", vec![
        DecisionCriterion::soft("bpm", 0.8, 0.5, 0.05),
        DecisionCriterion::soft("key", 0.8, 0.5, 0.05),
    ]);
    let cand_b = ("cand_b", vec![
        DecisionCriterion::soft("bpm", 0.8, 0.5, 0.05),
        // missing "key"
    ]);

    let ranked = rank_candidates(vec![cand_a, cand_b], &config);
    let evaluated_b = ranked.iter().find(|c| c.payload == "cand_b").unwrap();
    // Candidate B should have received missing-criterion penalty for "key"
    assert!(evaluated_b.criteria.iter().any(|c| c.name == "key" && c.score == 0.0 && c.uncertainty == 1.0));
    assert!(ranked[0].payload == "cand_a", "Candidate A with complete criteria must outrank B with missing criteria");
}

#[test]
fn test_pattern_scale_and_frequency_invariance() {
    // Generate the same crescendo peak at:
    // Scale 1.0, 100 Hz (10ms dt)
    // Scale 100.0, 20 Hz (50ms dt)
    // Scale 0.01, 10 Hz (100ms dt)
    let total_duration_sec = 2.0;

    let test_cases = vec![
        (1.0, 10),    // 10ms
        (100.0, 50),  // 50ms
        (0.01, 100),  // 100ms
    ];

    for (scale, dt_ms) in test_cases {
        let num_pts = (total_duration_sec * 1000.0 / dt_ms as f64) as usize;
        let points: Vec<KinematicPoint> = (0..num_pts)
            .map(|i| {
                let t_ms = i as i64 * dt_ms;
                let t_sec = t_ms as f64 / 1000.0;
                // Crescendo: starts at 0, accelerates to crest at 1.5s, then begins descent
                let val = scale * (1.0 - (t_sec - 1.5).powi(2) / 2.25).max(0.0);
                KinematicPoint::new(t_ms, val)
            })
            .collect();

        let config = KinematicFilterConfig {
            time_bandwidth_ms: 200.0,
            nominal_scale: scale,
            ..Default::default()
        };

        let metrics = analyze_kinematics(&points, &config).expect("Analysis failed");
        let archetype = classify_archetype(&metrics, 0.005 * scale);
        assert_eq!(
            archetype,
            ProgressionArchetype::CrescendoPeak,
            "Archetype must be CrescendoPeak for scale {} at dt {}ms, got {:?}",
            scale,
            dt_ms,
            archetype
        );
    }
}

#[test]
fn test_trajectory_projection_calculus_derivative() {
    // Mathematical invariant: d(Delta_x)/dt == v(t) for all t >= 0 and gamma >= 0.
    let m = KinematicMetrics {
        timestamp_ms: 0,
        raw_magnitude: 0.50,
        magnitude: 0.50,
        velocity: 0.08,
        acceleration: -0.02,
        direction: Direction::Ascending,
        volatility: 0.01,
        snr: 8.0,
        quality_score: 0.9,
    };
    let u_config = UncertaintyConfig::default();
    let unc = quantify_uncertainty(m.magnitude, m.volatility, 10, 0.0, &u_config);

    let gammas = [0.0, 0.05, 0.2, 1.0, 5.0];
    let horizons = [0.1, 0.5, 1.0, 2.5, 5.0];
    let eps = 1e-5;

    for &gamma in &gammas {
        for &t in &horizons {
            let proj_plus = project_trajectory(&m, ProgressionArchetype::MonotonicAscent, t + eps, gamma, &unc);
            let proj_minus = project_trajectory(&m, ProgressionArchetype::MonotonicAscent, t - eps, gamma, &unc);
            let proj_t = project_trajectory(&m, ProgressionArchetype::MonotonicAscent, t, gamma, &unc);

            let numerical_derivative = (proj_plus.projected_value - proj_minus.projected_value) / (2.0 * eps);
            let analytical_v = proj_t.projected_velocity;

            let diff = (numerical_derivative - analytical_v).abs();
            assert!(
                diff < 1e-4,
                "Calculus derivative failure at gamma={}, t={}: num_deriv {:.6} != analytical_v {:.6} (diff {:.2e})",
                gamma,
                t,
                numerical_derivative,
                analytical_v,
                diff
            );
        }
    }
}

#[test]
fn test_trajectory_projection_zero_damping_and_limits() {
    let m = KinematicMetrics {
        timestamp_ms: 0,
        raw_magnitude: 1.0,
        magnitude: 1.0,
        velocity: 0.10,
        acceleration: 0.02,
        direction: Direction::Ascending,
        volatility: 0.01,
        snr: 10.0,
        quality_score: 0.9,
    };
    let u_config = UncertaintyConfig::default();
    let unc = quantify_uncertainty(m.magnitude, m.volatility, 10, 0.0, &u_config);

    // Case 1: Zero damping gamma = 0 -> exact classical kinematic: x(t) = x0 + v0*t + 0.5*a0*t^2
    let t = 2.0;
    let proj_undamped = project_trajectory(&m, ProgressionArchetype::MonotonicAscent, t, 0.0, &unc);
    let expected_x = 1.0 + 0.10 * t + 0.5 * 0.02 * t * t;
    let expected_v = 0.10 + 0.02 * t;
    assert!((proj_undamped.projected_value - expected_x).abs() < 1e-5);
    assert!((proj_undamped.projected_velocity - expected_v).abs() < 1e-5);

    // Case 2: Long horizon with damping gamma = 1.0 -> velocity approaches 0, position converges
    let proj_long = project_trajectory(&m, ProgressionArchetype::MonotonicAscent, 100.0, 1.0, &unc);
    assert!(proj_long.projected_velocity < 1e-6, "Damped velocity must approach 0 at long horizons");
    assert!(proj_long.lower_bound <= proj_long.projected_value);
    assert!(proj_long.upper_bound >= proj_long.projected_value);
    // Projection confidence must decay towards 0 at long horizons
    assert!(proj_long.projection_confidence < 0.05);
}

#[test]
fn test_protocol_property_invariants() {
    // Invariant 1: Scale Invariance
    // Scaling signal by constant c scales velocity by c, acceleration by c, volatility by c,
    // and preserves dimensionless SNR exactly.
    let c = 42.0;
    let config_base = KinematicFilterConfig {
        time_bandwidth_ms: 200.0,
        nominal_scale: 1.0,
        ..Default::default()
    };
    let config_scaled = KinematicFilterConfig {
        time_bandwidth_ms: 200.0,
        nominal_scale: c,
        noise_floor: config_base.noise_floor * c,
        ..Default::default()
    };

    let pts_base: Vec<KinematicPoint> = (0..30)
        .map(|i| KinematicPoint::new(i * 100, 0.2 + 0.05 * (i as f64 / 10.0)))
        .collect();
    let pts_scaled: Vec<KinematicPoint> = pts_base
        .iter()
        .map(|p| KinematicPoint::new(p.timestamp_ms, p.value * c))
        .collect();

    let m_base = analyze_kinematics(&pts_base, &config_base).unwrap();
    let m_scaled = analyze_kinematics(&pts_scaled, &config_scaled).unwrap();

    for i in 5..25 {
        let v_ratio = m_scaled[i].velocity / m_base[i].velocity;
        assert!(
            (v_ratio - c).abs() < 1e-4,
            "Velocity scaling failed: expected ratio {}, got {}",
            c,
            v_ratio
        );
        let snr_diff = (m_scaled[i].snr - m_base[i].snr).abs();
        assert!(
            snr_diff < 0.01,
            "SNR must be scale-invariant: base {:.3}, scaled {:.3}",
            m_base[i].snr,
            m_scaled[i].snr
        );
    }

    // Invariant 2: Noise-Uncertainty Monotonicity
    let u_config = UncertaintyConfig::default();
    let unc_low = quantify_uncertainty(1.0, 0.01, 5, 0.0, &u_config);
    let unc_high = quantify_uncertainty(1.0, 0.10, 5, 0.0, &u_config);
    assert!(
        unc_high.total_uncertainty > unc_low.total_uncertainty,
        "Higher noise must strictly increase total uncertainty"
    );

    // Invariant 3: Staleness-Freshness Decay Monotonicity
    let unc_fresh = quantify_uncertainty(1.0, 0.02, 5, 1.0, &u_config);
    let unc_stale = quantify_uncertainty(1.0, 0.02, 5, 10.0, &u_config);
    assert!(
        unc_fresh.freshness > unc_stale.freshness,
        "Elapsed time must strictly reduce freshness"
    );
    assert!(
        unc_stale.total_uncertainty > unc_fresh.total_uncertainty,
        "Stale observation must have higher total uncertainty"
    );

    // Invariant 4: Hard Constraint Absolute Immunity
    let p_config = PrioritizerConfig::default();
    let candidates = vec![
        (
            "bad_candidate",
            vec![
                DecisionCriterion::hard("cooldown", 0.0, 1.0), // Fails hard constraint
                DecisionCriterion::soft("quality", 1.0, 1.0, 0.0), // Perfect soft score
            ],
        ),
        (
            "good_candidate",
            vec![
                DecisionCriterion::hard("cooldown", 1.0, 1.0), // Passes hard constraint
                DecisionCriterion::soft("quality", 0.7, 1.0, 0.1), // Moderate soft score
            ],
        ),
    ];
    let ranked = rank_candidates(candidates, &p_config);
    assert_eq!(ranked[0].payload, "good_candidate");
    assert!(!ranked[1].is_eligible);

    // Invariant 5: Non-Monotonic Timestamp Rejection
    let invalid_points = vec![
        KinematicPoint::new(100, 1.0),
        KinematicPoint::new(100, 1.2), // Duplicate timestamp
    ];
    let err = analyze_kinematics(&invalid_points, &config_base).unwrap_err();
    assert!(matches!(err, KinematicError::NonMonotonicTimestamp { .. }));
}


