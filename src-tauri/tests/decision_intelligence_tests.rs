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
    analyze_kinematics, analyze_progression, AdaptiveThresholdConfig, Direction,
    InitialConditionPolicy, KinematicError, KinematicFilterConfig, KinematicMetrics,
    KinematicPoint, KinematicSignalTracker, ProgressionState,
    ProgressionStateMachine,
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
