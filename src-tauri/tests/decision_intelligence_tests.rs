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

