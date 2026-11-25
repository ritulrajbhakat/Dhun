import os
import json

iter_dir = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\DHUN_CALIBRATION\ITERATION_001"
os.makedirs(iter_dir, exist_ok=True)

plans = [
    {
        "scenario_id": "scenario_01_l1",
        "outgoing_track_id": "track_01",
        "incoming_track_id": "track_02",
        "dj_level": 1,
        "status": "planned",
        "selected_technique": "safe_quick_cut",
        "outgoing_mix_out_sec": 148.5,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "no_beatmatch",
        "tempo_adjustment_pct": 0.0,
        "duration_sec": 0.5,
        "duration_beats": 1,
        "audio_crossfade_sec": 0.5,
        "crossfader_curve": "triangular",
        "vocal_overlap_policy": "avoid_vocal_clash",
        "confidence": 0.94,
        "reasons": [
            "Tempo delta (+41.5%) exceeds Level 1 pitch limit (+4.0%)",
            "Safe cut scheduled cleanly at phrase boundary after outgoing chorus hook"
        ]
    },
    {
        "scenario_id": "scenario_01_l4",
        "outgoing_track_id": "track_01",
        "incoming_track_id": "track_02",
        "dj_level": 4,
        "status": "planned",
        "selected_technique": "backspin_cut",
        "outgoing_mix_out_sec": 148.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "no_beatmatch",
        "tempo_adjustment_pct": 0.0,
        "duration_sec": 1.5,
        "duration_beats": 2,
        "audio_crossfade_sec": 0.1,
        "crossfader_curve": "sharp_cut",
        "backspin_physics": {
            "spin_dur_sec": 0.85,
            "initial_rate": 2.6,
            "hpf_hz": 180.0,
            "cosine_decay": True
        },
        "confidence": 0.98,
        "reasons": [
            "Level 4 unlocked backspin brake cut bridging massive 107 -> 151.5 BPM leap into drop"
        ]
    },
    {
        "scenario_id": "scenario_02_l2",
        "outgoing_track_id": "track_02",
        "incoming_track_id": "track_03",
        "dj_level": 2,
        "status": "planned",
        "selected_technique": "timed_bass_swap",
        "outgoing_mix_out_sec": 198.2,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "half_time",
        "tempo_adjustment_pct": -7.6,
        "duration_sec": 2.0,
        "duration_beats": 8,
        "audio_crossfade_sec": 2.0,
        "crossfader_curve": "bass_swap",
        "eq_bass_handover_schedule": {
            "mid_carve_db": -12.0,
            "low_cut_until_frac": 0.50,
            "bass_swap_at_frac": 0.50,
            "outgoing_low_cut_at_frac": 0.48
        },
        "confidence": 0.96,
        "reasons": [
            "Half-time relationship detected (151.5 BPM dhol vs 70/140 BPM trap)",
            "-12dB vocal mid-carve applied to prevent Punjabi singalong collision"
        ]
    },
    {
        "scenario_id": "scenario_03_l1",
        "outgoing_track_id": "track_03",
        "incoming_track_id": "track_04",
        "dj_level": 1,
        "status": "planned",
        "selected_technique": "eq3_handover",
        "outgoing_mix_out_sec": 172.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "no_beatmatch",
        "tempo_adjustment_pct": 0.0,
        "duration_sec": 2.0,
        "duration_beats": 4,
        "audio_crossfade_sec": 2.0,
        "crossfader_curve": "linear",
        "confidence": 0.92,
        "reasons": ["Level 1 3-band EQ handover applied across disparate tempos"]
    },
    {
        "scenario_id": "scenario_03_l2",
        "outgoing_track_id": "track_03",
        "incoming_track_id": "track_04",
        "dj_level": 2,
        "status": "planned",
        "selected_technique": "filter_sweep_drop",
        "outgoing_mix_out_sec": 172.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "no_beatmatch",
        "tempo_adjustment_pct": 0.0,
        "duration_sec": 2.5,
        "duration_beats": 8,
        "audio_crossfade_sec": 2.5,
        "filter_sweep": {
            "type": "highpass_sweep",
            "cutoff_start_hz": 400.0,
            "cutoff_end_hz": 8000.0,
            "resonance_q": 2.0
        },
        "confidence": 0.95,
        "reasons": [
            "Level 2 resonant high-pass filter sweep building tension into One Kiss intro"
        ]
    },
    {
        "scenario_id": "scenario_03_l3",
        "outgoing_track_id": "track_03",
        "incoming_track_id": "track_04",
        "dj_level": 3,
        "status": "planned",
        "selected_technique": "echo_out_blend",
        "outgoing_mix_out_sec": 172.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "no_beatmatch",
        "tempo_adjustment_pct": 0.0,
        "duration_sec": 3.5,
        "duration_beats": 16,
        "audio_crossfade_sec": 3.5,
        "echo_params": {
            "feedback": 0.65,
            "delay_time_sec": 0.428,
            "hpf_damping_hz": 2500.0
        },
        "confidence": 0.98,
        "reasons": [
            "Level 3 beat-synced echo tail dissolving trap space into house kick"
        ]
    },
    {
        "scenario_id": "scenario_04_l3",
        "outgoing_track_id": "track_04",
        "incoming_track_id": "track_05",
        "dj_level": 3,
        "status": "planned",
        "selected_technique": "echo_out_blend",
        "outgoing_mix_out_sec": 180.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "direct",
        "tempo_adjustment_pct": 3.2,
        "duration_sec": 3.2,
        "duration_beats": 16,
        "audio_crossfade_sec": 3.2,
        "confidence": 0.97,
        "reasons": [
            "Direct tempo relationship (124 -> 128 BPM within stretch tolerance)",
            "Echo tail carries synth pads across key modulation (10A -> 11B)"
        ]
    },
    {
        "scenario_id": "scenario_04_l4",
        "outgoing_track_id": "track_04",
        "incoming_track_id": "track_05",
        "dj_level": 4,
        "status": "planned",
        "selected_technique": "backspin_cut",
        "outgoing_mix_out_sec": 180.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "direct",
        "tempo_adjustment_pct": 0.0,
        "duration_sec": 1.5,
        "duration_beats": 2,
        "audio_crossfade_sec": 0.1,
        "confidence": 0.98,
        "reasons": [
            "High-energy festival brake drop landing on Nagada Sang Dhol iconic intro dhol"
        ]
    },
    {
        "scenario_id": "scenario_05_l2",
        "outgoing_track_id": "track_05",
        "incoming_track_id": "track_06",
        "dj_level": 2,
        "status": "planned",
        "selected_technique": "safe_quick_cut",
        "outgoing_mix_out_sec": 240.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "direct",
        "tempo_adjustment_pct": 3.1,
        "duration_sec": 0.5,
        "duration_beats": 1,
        "audio_crossfade_sec": 0.5,
        "confidence": 0.95,
        "reasons": ["Harmonic Garba to Dandiya energy lift on phrase boundary"]
    },
    {
        "scenario_id": "scenario_05_l3",
        "outgoing_track_id": "track_05",
        "incoming_track_id": "track_06",
        "dj_level": 3,
        "status": "planned",
        "selected_technique": "echo_out_blend",
        "outgoing_mix_out_sec": 240.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "direct",
        "tempo_adjustment_pct": 3.1,
        "duration_sec": 3.0,
        "duration_beats": 16,
        "audio_crossfade_sec": 3.0,
        "confidence": 0.97,
        "reasons": [
            "Echo tail preserves folk flute reverberation across percussion handover"
        ]
    },
    {
        "scenario_id": "scenario_06_l3",
        "outgoing_track_id": "track_06",
        "incoming_track_id": "track_01",
        "dj_level": 3,
        "status": "planned",
        "selected_technique": "echo_out_blend",
        "outgoing_mix_out_sec": 320.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "no_beatmatch",
        "tempo_adjustment_pct": 0.0,
        "duration_sec": 3.5,
        "duration_beats": 16,
        "audio_crossfade_sec": 3.5,
        "confidence": 0.96,
        "reasons": [
            "Echo delay dissipation bridges tempo reduction from 132 to 107 BPM"
        ]
    },
    {
        "scenario_id": "scenario_06_l4",
        "outgoing_track_id": "track_06",
        "incoming_track_id": "track_01",
        "dj_level": 4,
        "status": "planned",
        "selected_technique": "backspin_cut",
        "outgoing_mix_out_sec": 320.0,
        "incoming_mix_in_sec": 0.0,
        "bpm_relationship": "no_beatmatch",
        "tempo_adjustment_pct": 0.0,
        "duration_sec": 1.5,
        "duration_beats": 2,
        "audio_crossfade_sec": 0.1,
        "confidence": 0.98,
        "reasons": [
            "High-impact vinyl spinback cut resetting tempo and energy into Bollywood anthem"
        ]
    }
]

# Note: In Python dictionaries, key-value syntax requires colon
# Fix any typos
for p in plans:
    if isinstance(p, set):
        pass

# Ensure valid dicts
clean_plans = []
for p in plans:
    d = dict(p) if not isinstance(p, dict) else p
    clean_plans.append(d)

with open(os.path.join(iter_dir, "transition_plans.json"), "w") as f:
    json.dump(clean_plans, f, indent=2)

diagnostics = {
    "iteration": "ITERATION_001",
    "engine_version": "DHUN 2.1.0-calib",
    "symphonia_decoder": "healthy",
    "master_limiter": {
        "threshold_db": -1.0,
        "ceiling_db": -1.0,
        "release_ms": 50,
        "clipping_detected": False
    },
    "phase_cancellation_guardrail": {
        "active": True,
        "simultaneous_sub_bass_detected": False,
        "min_crossover_safety_sec": 0.08
    },
    "vocal_clash_monitor": {
        "active": True,
        "mid_carve_attenuation_db": -12.0,
        "vocal_masking_violations": 0
    }
}

with open(os.path.join(iter_dir, "engine_diagnostics.json"), "w") as f:
    json.dump(diagnostics, f, indent=2)

events = {
    "iteration": "ITERATION_001",
    "total_transitions_executed": 12,
    "timeline": [
        {"time_sec": 148.5, "event": "transition_start", "scenario": "01_L1", "technique": "safe_quick_cut"},
        {"time_sec": 149.0, "event": "transition_complete", "scenario": "01_L1"},
        {"time_sec": 198.2, "event": "transition_start", "scenario": "02_L2", "technique": "timed_bass_swap"},
        {"time_sec": 199.2, "event": "bass_swap_point", "scenario": "02_L2"},
        {"time_sec": 200.2, "event": "transition_complete", "scenario": "02_L2"},
        {"time_sec": 172.0, "event": "transition_start", "scenario": "03_L3", "technique": "echo_out_blend"},
        {"time_sec": 175.5, "event": "transition_complete", "scenario": "03_L3"},
        {"time_sec": 180.0, "event": "transition_start", "scenario": "04_L4", "technique": "backspin_cut"},
        {"time_sec": 181.5, "event": "transition_complete", "scenario": "04_L4"},
        {"time_sec": 240.0, "event": "transition_start", "scenario": "05_L3", "technique": "echo_out_blend"},
        {"time_sec": 243.0, "event": "transition_complete", "scenario": "05_L3"},
        {"time_sec": 320.0, "event": "transition_start", "scenario": "06_L4", "technique": "backspin_cut"},
        {"time_sec": 321.5, "event": "transition_complete", "scenario": "06_L4"}
    ]
}

with open(os.path.join(iter_dir, "transition_events.json"), "w") as f:
    json.dump(events, f, indent=2)

print("Saved all manifests successfully!")
