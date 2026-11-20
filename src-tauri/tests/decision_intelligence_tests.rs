//! Stage 1 & Stage 2 Decision Intelligence Verification Suite
//!
//! Validates:
//! 1. Kinematic derivatives (velocity, acceleration, direction, volatility, SNR, confidence)
//! 2. Zero-phase Gaussian smoothing and high-frequency noise filtering
//! 3. Dual-threshold hysteresis and limit-cycle chatter prevention
//! 4. Temporal persistence debounce windows
//! 5. Full 8-state progression lifecycle traversal
//! 6. Meaningful region extraction, trapezoidal mass integration, and sustained duration tracking
//! 7. Mathematical spike rejection vs. sustained progression validation
//! 8. Complete domain agnosticism (pure scalar time-series signals)

use dhun_lib::dj::intelligence::{
    analyze_kinematics, analyze_progression, AdaptiveThresholdConfig, Direction,
    KinematicFilterConfig, KinematicMetrics, KinematicPoint, KinematicSignalTracker,
    ProgressionState, ProgressionStateMachine,
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

    let metrics = analyze_kinematics(&points, &config);
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
        half_window: 2,
        stationary_deadband: 0.001,
        ..Default::default()
    };

    // Ramp: x(t) = 0.2 + 0.1 * t_sec (t in ms: t_sec = t / 1000.0)
    // Theoretical velocity = 0.1 units/sec, acceleration = 0.0 units/sec^2
    let points: Vec<KinematicPoint> = (0..60)
        .map(|i| {
            let t_ms = i * 100; // every 100ms
            let t_sec = t_ms as f64 / 1000.0;
            KinematicPoint::new(t_ms, 0.2 + 0.1 * t_sec)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config);

    // Interior points (skipping first and last 2 points due to boundary window)
    for m in &metrics[3..metrics.len() - 3] {
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
        half_window: 2,
        stationary_deadband: 0.001,
        ..Default::default()
    };

    // Ramp: x(t) = 0.9 - 0.05 * t_sec
    // Theoretical velocity = -0.05 units/sec
    let points: Vec<KinematicPoint> = (0..50)
        .map(|i| {
            let t_ms = i * 100;
            let t_sec = t_ms as f64 / 1000.0;
            KinematicPoint::new(t_ms, 0.9 - 0.05 * t_sec)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config);

    for m in &metrics[3..metrics.len() - 3] {
        let v_error = (m.velocity - (-0.05)).abs();
        assert!(v_error < 0.005, "Velocity error at t={}: got {}", m.timestamp_ms, m.velocity);
        assert_eq!(m.direction, Direction::Descending);
    }
}

#[test]
fn test_kinematics_parabolic_curve_exact_acceleration() {
    let config = KinematicFilterConfig {
        half_window: 2,
        gaussian_sigma: 1.0,
        ..Default::default()
    };

    // Parabola: x(t) = 0.5 * a * t^2 with a = 0.2 units/sec^2
    // v(t) = a * t = 0.2 * t
    // a(t) = 0.2
    let a_theo = 0.2;
    let points: Vec<KinematicPoint> = (0..50)
        .map(|i| {
            let t_ms = i * 200; // every 200ms
            let t_sec = t_ms as f64 / 1000.0;
            KinematicPoint::new(t_ms, 0.5 * a_theo * t_sec * t_sec)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config);

    // Interior points where Gaussian smoothing has full coverage
    for m in &metrics[5..metrics.len() - 5] {
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
        half_window: 2,
        gaussian_sigma: 1.0,
        ..Default::default()
    };

    // x(t) = sin(omega * t) with omega = 2 * pi * 0.2 Hz (period = 5s)
    let freq = 0.2;
    let omega = 2.0 * PI * freq;
    let points: Vec<KinematicPoint> = (0..100)
        .map(|i| {
            let t_ms = i * 100;
            let t_sec = t_ms as f64 / 1000.0;
            KinematicPoint::new(t_ms, (omega * t_sec).sin())
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config);

    // When sin(omega*t) ~ 1 (crest at t = 1.25s), velocity should be ~0 and acceleration should be negative
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
    assert!(crest_point.acceleration < -0.5, "Acceleration at peak must be negative (downward curvature)");
}

#[test]
fn test_kinematics_noise_filtering_and_snr() {
    let config = KinematicFilterConfig {
        half_window: 3,
        gaussian_sigma: 1.2,
        ..Default::default()
    };

    // Constant baseline of 0.5 with high-frequency +/- 0.05 jitter
    let points: Vec<KinematicPoint> = (0..40)
        .map(|i| {
            let noise = if i % 2 == 0 { 0.05 } else { -0.05 };
            KinematicPoint::new(i * 100, 0.5 + noise)
        })
        .collect();

    let metrics = analyze_kinematics(&points, &config);

    for m in &metrics[5..metrics.len() - 5] {
        // Gaussian smoothing should compress +/-0.05 alternating jitter down to near zero
        let smoothed_deviation = (m.magnitude - 0.5).abs();
        assert!(
            smoothed_deviation < 0.02,
            "Noise should be filtered: smoothed {}, raw {}",
            m.magnitude,
            m.raw_magnitude
        );
        // Volatility detects the jitter
        assert!(m.volatility > 0.03, "Volatility should register jitter");
    }
}

#[test]
fn test_kinematics_streaming_tracker() {
    let config = KinematicFilterConfig::default();
    let mut tracker = KinematicSignalTracker::new(config);

    let mut last_v = 0.0;
    for i in 0..30 {
        let t_ms = i * 100;
        let t_sec = t_ms as f64 / 1000.0;
        let m = tracker.update(t_ms, 0.1 + 0.2 * t_sec);
        last_v = m.velocity;
    }

    // Streaming velocity should converge towards 0.2 units/sec
    assert!(
        (last_v - 0.2).abs() < 0.03,
        "Streaming velocity should be ~0.2, got {}",
        last_v
    );
}

#[test]
fn test_kinematics_edge_cases() {
    let config = KinematicFilterConfig::default();

    // 1. Empty
    let empty_res = analyze_kinematics(&[], &config);
    assert!(empty_res.is_empty());

    // 2. Single point
    let single = vec![KinematicPoint::new(100, 0.42)];
    let single_res = analyze_kinematics(&single, &config);
    assert_eq!(single_res.len(), 1);
    assert_eq!(single_res[0].magnitude, 0.42);
    assert_eq!(single_res[0].velocity, 0.0);
    assert_eq!(single_res[0].direction, Direction::Stationary);

    // 3. Two points
    let two = vec![KinematicPoint::new(0, 0.0), KinematicPoint::new(1000, 1.0)];
    let two_res = analyze_kinematics(&two, &config);
    assert_eq!(two_res.len(), 2);
    assert!((two_res[0].velocity - 1.0).abs() < 1e-4);
}

// =========================================================================
// STAGE 2: PROGRESSION & HYSTERESIS VERIFICATION
// =========================================================================

#[test]
fn test_hysteresis_boundary_chatter_prevention() {
    // Setup adaptive threshold with 15% hysteresis:
    // base_entry = 0.60 => theta_enter = 0.60
    // theta_exit = 0.60 * (1 - 0.15) = 0.51
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.60,
        hysteresis_ratio: 0.15,
        min_confirmation_ms: 200,
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(config);

    // Initial low baseline
    let m0 = KinematicMetrics {
        timestamp_ms: 0,
        raw_magnitude: 0.2,
        magnitude: 0.2,
        velocity: 0.0,
        acceleration: 0.0,
        direction: Direction::Stationary,
        volatility: 0.005,
        snr: 0.0,
        confidence: 0.5,
    };
    sm.update(&m0);
    assert_eq!(sm.current_state(), ProgressionState::BelowSignificance);

    // Enter Building state with strong ascent across theta_enter (0.60)
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
            confidence: 0.8,
        };
        sm.update(&m);
    }
    // At t=500ms, magnitude is 0.80 (> 0.60) and held for > 200ms -> committed Building
    assert_eq!(sm.current_state(), ProgressionState::Building);

    // Now simulate boundary fluctuations between 0.54 and 0.58
    // Both values are BELOW theta_enter (0.60), but ABOVE theta_exit (0.51).
    // In a single-threshold system, this would cause 10+ rapid flipping transitions!
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
            confidence: 0.7,
        };
        sm.update(&m);

        // Hysteresis GUARANTEES state does NOT flip to BelowSignificance or Subsided!
        assert!(
            sm.current_state().is_active_progression(),
            "Chatter occurred at t={}: state dropped to {:?}",
            i * 100,
            sm.current_state()
        );
    }

    // Now drop firmly below theta_exit (0.51) to 0.40 for > 200ms
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
            confidence: 0.8,
        };
        sm.update(&m);
    }

    // Confirmed drop below exit threshold commits Subsided
    assert_eq!(sm.current_state(), ProgressionState::Subsided);
}

#[test]
fn test_temporal_persistence_debounce_spike_rejection() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.60,
        min_confirmation_ms: 300, // 300ms confirmation window
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(config);

    // 0ms: baseline
    let m0 = KinematicMetrics {
        timestamp_ms: 0,
        raw_magnitude: 0.30,
        magnitude: 0.30,
        velocity: 0.0,
        acceleration: 0.0,
        direction: Direction::Stationary,
        volatility: 0.005,
        snr: 0.0,
        confidence: 0.5,
    };
    sm.update(&m0);
    assert_eq!(sm.current_state(), ProgressionState::BelowSignificance);

    // Transient spike: jumps to 0.95 at 100ms, stays at 200ms (only 100ms duration, < 300ms)
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
            confidence: 0.6,
        };
        sm.update(&m);
        // Candidate state is Building, but NOT committed because persistence < 300ms
        assert_eq!(sm.current_state(), ProgressionState::BelowSignificance);
    }

    // Immediately collapses back to baseline at 300ms
    let m3 = KinematicMetrics {
        timestamp_ms: 300,
        raw_magnitude: 0.30,
        magnitude: 0.30,
        velocity: -0.1,
        acceleration: 0.0,
        direction: Direction::Descending,
        volatility: 0.05,
        snr: 2.0,
        confidence: 0.6,
    };
    sm.update(&m3);

    // Remains BelowSignificance throughout the spike!
    assert_eq!(sm.current_state(), ProgressionState::BelowSignificance);
    assert!(sm.completed_regions().is_empty(), "No region should be generated for unconfirmed spike");
}

#[test]
fn test_full_8_state_progression_lifecycle() {
    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.50,
        hysteresis_ratio: 0.15, // exit threshold = 0.425
        min_confirmation_ms: 200,
        min_sustain_ms: 1000,
        building_velocity_threshold: 0.005,
        subduing_velocity_threshold: -0.005,
        sustain_drift_threshold: 0.015,
        recovery_velocity_threshold: 0.010,
        ..Default::default()
    };

    let mut sm = ProgressionStateMachine::new(config);
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
            confidence: 0.5,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 2: Building (t=400..900ms, value rises to 0.85)
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
            confidence: 0.8,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 3: Peaking (t=1000..1200ms, cresting at 0.90, velocity slows near 0)
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
            confidence: 0.9,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 4: Sustained (t=1300..2600ms, held at ~0.90 for > 1.0s)
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
            confidence: 0.95,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 5: Subduing (t=2700..3200ms, velocity negative, value drops from 0.89 to 0.60)
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
            confidence: 0.8,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 6: Recovering (t=3300..3700ms, temporary rebound with v > 0.01, value rises from 0.60 to 0.68)
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
            confidence: 0.8,
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
            confidence: 0.8,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 7: Subsided (t=4200..4600ms, dropped below theta_exit 0.425 to 0.30 for > 200ms)
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
            confidence: 0.7,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // State 8: Stable (t=4700..5100ms, flat quiescent baseline at 0.15)
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
            confidence: 0.6,
        };
        record(&mut sm, &m, &mut observed_states);
    }

    // Verify all key lifecycle states were traversed:
    assert!(observed_states.contains(&ProgressionState::BelowSignificance));
    assert!(observed_states.contains(&ProgressionState::Building));
    assert!(observed_states.contains(&ProgressionState::Peaking));
    assert!(observed_states.contains(&ProgressionState::Sustained));
    assert!(observed_states.contains(&ProgressionState::Subduing));
    assert!(observed_states.contains(&ProgressionState::Recovering));
    assert!(observed_states.contains(&ProgressionState::Subsided));
    assert!(observed_states.contains(&ProgressionState::Stable));

    // Verify region extraction recorded the single continuous episode
    let regions = sm.completed_regions();
    assert_eq!(regions.len(), 1, "Should have extracted exactly 1 meaningful region");
    let r = &regions[0];
    assert!(r.peak_magnitude >= 0.89);
    assert!(r.sustained_duration_ms >= 1000, "Sustained duration must be >= 1000ms");
    assert!(r.integral_mass > 0.5, "Integrated mass must be substantial");
    assert!(r.confidence > 0.5, "Confidence must be high for full sustained progression");
}

#[test]
fn test_spike_rejection_vs_sustained_movement_contrast() {
    // This directly tests DHUN's central principle:
    // "DHUN must not optimize exclusively for the highest value. It must identify
    // meaningful regions, understand how they develop, recognize when they are
    // sustained, and understand how they subside."

    let config = AdaptiveThresholdConfig {
        base_entry_threshold: 0.50,
        min_confirmation_ms: 200,
        min_sustain_ms: 1000,
        ..Default::default()
    };

    // --- Scenario A: Transient Sharp Spike ---
    // Peak is HIGHER (0.98), but total duration is only 400ms.
    let mut spike_points = Vec::new();
    for t in (0..=2000).step_by(100) {
        let val = if t == 500 || t == 600 || t == 700 {
            0.98 // Very high instantaneous peak
        } else {
            0.20
        };
        spike_points.push(KinematicPoint::new(t, val));
    }
    let k_config = KinematicFilterConfig::default();
    let spike_kinematics = analyze_kinematics(&spike_points, &k_config);
    let (_, spike_regions) = analyze_progression(&spike_kinematics, &config);

    // --- Scenario B: Sustained Musical Movement ---
    // Peak is LOWER (0.80), but elevated and sustained for 8,000ms (8 seconds).
    let mut sustained_points = Vec::new();
    for t in (0..=12000).step_by(200) {
        let val = if t >= 2000 && t <= 10000 {
            0.80 // Sustained elevated plateau
        } else {
            0.20
        };
        sustained_points.push(KinematicPoint::new(t, val));
    }
    let sustained_kinematics = analyze_kinematics(&sustained_points, &k_config);
    let (_, sustained_regions) = analyze_progression(&sustained_kinematics, &config);

    // Scenario B must produce a validated meaningful region with vastly superior mass and sustained duration
    assert_eq!(sustained_regions.len(), 1);
    let b_region = &sustained_regions[0];

    // Scenario A either produces 0 regions (due to persistence rejection) or an ephemeral low-mass region
    let a_mass = spike_regions.first().map(|r| r.integral_mass).unwrap_or(0.0);
    let a_sustain = spike_regions.first().map(|r| r.sustained_duration_ms).unwrap_or(0);
    let a_conf = spike_regions.first().map(|r| r.confidence).unwrap_or(0.0);

    // Verify mathematical dominance of sustained progression over isolated peak
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
        b_region.confidence > a_conf,
        "Sustained confidence ({}) must exceed spike ({})",
        b_region.confidence,
        a_conf
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

    // Two distinct high-energy movements separated by a quiet interval
    // Movement 1: 2,000ms - 5,000ms (value ~0.80)
    // Quiet:      5,000ms - 9,000ms (value ~0.20)
    // Movement 2: 9,000ms - 13,000ms (value ~0.85)
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

    let kinematics = analyze_kinematics(&points, &k_config);
    let (_, regions) = analyze_progression(&kinematics, &config);

    assert_eq!(
        regions.len(),
        2,
        "Should segment exactly two distinct meaningful regions"
    );

    assert_eq!(regions[0].region_id, 1);
    assert!(regions[0].onset_ms >= 1800 && regions[0].onset_ms <= 2400);
    assert!(regions[0].offset_ms >= 5000 && regions[0].offset_ms <= 6000);

    assert_eq!(regions[1].region_id, 2);
    assert!(regions[1].onset_ms >= 8800 && regions[1].onset_ms <= 9400);
    assert!(regions[1].offset_ms >= 13000 && regions[1].offset_ms <= 14000);
}
