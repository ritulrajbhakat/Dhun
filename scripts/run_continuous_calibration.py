import os
import sys
import json
import math

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
FRESH_BATCHES_DIR = os.path.join(CALIBRATION_DIR, "fresh_batches")
REGRESSION_DIR = os.path.join(CALIBRATION_DIR, "regression_library")
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
RECORDINGS_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings\continuous_calibration"

os.makedirs(FRESH_BATCHES_DIR, exist_ok=True)
os.makedirs(REGRESSION_DIR, exist_ok=True)
os.makedirs(RECORDINGS_DIR, exist_ok=True)

# 12 Gatekeeper Criteria - Every single one must reach 99.0 - 100.0
SCORECARD_CRITERIA = [
    "phrase_alignment",
    "vocal_safety",
    "beat_synchronization",
    "bpm_and_tempo_decisions",
    "harmonic_compatibility",
    "meter_and_groove",
    "bass_and_percussion",
    "energy_progression",
    "transition_smoothness",
    "dj_techniques_and_effects",
    "audio_headroom_and_integrity",
    "overall_dj_decision_quality"
]

MINIMUM_ACCEPTANCE_SCORE = 99.0

# Initialize Baseline Regression Library (Original 10 Stress-Test Tracks)
INITIAL_REGRESSION_TRACKS = [
    {"id": "animals", "title": "Animals (Original Mix)", "artist": "Martin Garrix", "file": "DuFUtL8zUAk.mp3"},
    {"id": "titanium", "title": "Titanium (feat. Sia)", "artist": "David Guetta", "file": "JRfuAukYTKg.mp3"},
    {"id": "dont_start_now", "title": "Don't Start Now", "artist": "Dua Lipa", "file": "oygrmJFKYZY.mp3"},
    {"id": "ghungroo", "title": "Ghungroo (From 'War')", "artist": "Vishal-Shekhar, Arijit Singh & Shilpa Rao", "file": "qFkNATtc3mc.mp3"},
    {"id": "zingaat", "title": "Zingaat", "artist": "Ajay-Atul", "file": "8eong8XHDB4.mp3"},
    {"id": "malhari", "title": "Malhari (From 'Bajirao Mastani')", "artist": "Vishal Dadlani", "file": "sRlA7JWTj04.mp3"},
    {"id": "big_dawgs", "title": "Big Dawgs", "artist": "Hanumankind & Kalmi", "file": "hOHKltAiKXQ.mp3"},
    {"id": "blinding_lights", "title": "Blinding Lights", "artist": "The Weeknd", "file": "fHI8X4OXluQ.mp3"},
    {"id": "chaiyya_chaiyya", "title": "Chaiyya Chaiyya (From 'Dil Se')", "artist": "Sukhwinder Singh & Sapna Awasthi", "file": "49oz7P9d8IA.mp3"},
    {"id": "perfect", "title": "Perfect", "artist": "Ed Sheeran", "file": "1UQzJfsT2eo.mp3"}
]

def load_regression_library():
    reg_file = os.path.join(REGRESSION_DIR, "regression_library.json")
    if os.path.exists(reg_file):
        with open(reg_file, "r", encoding="utf-8") as f:
            return json.load(f)
    else:
        initial = {
            "version": "1.0",
            "tracks": INITIAL_REGRESSION_TRACKS,
            "total_count": len(INITIAL_REGRESSION_TRACKS),
            "rounds_passed": ["initial_calibration_matrix"]
        }
        with open(reg_file, "w", encoding="utf-8") as f:
            json.dump(initial, f, indent=2)
        return initial

def verify_fresh_batch_uniqueness(batch_tracks):
    """
    Enforces the critical rule:
    Previously tested songs cannot count as fresh validation songs.
    None of the previous round's songs or reference corpus songs may be reused.
    """
    reg_lib = load_regression_library()
    reg_ids = {t["id"].lower() for t in reg_lib.get("tracks", [])}
    reg_titles = {t["title"].lower() for t in reg_lib.get("tracks", [])}

    # Load 100-reference corpus to ensure no cross-contamination
    corpus_file = os.path.join(CALIBRATION_DIR, "reference_discovery_100_resolved.json")
    corpus_titles = set()
    if os.path.exists(corpus_file):
        with open(corpus_file, "r", encoding="utf-8") as f:
            cdata = json.load(f)
            for item in cdata:
                title = item.get("resolved_title") or item.get("lead_title") or ""
                corpus_titles.add(title.lower())

    violations = []
    for track in batch_tracks:
        tid = track["id"].lower()
        ttitle = track["title"].lower()
        if tid in reg_ids or ttitle in reg_titles:
            violations.append(f"Regression duplicate: {track['title']}")
        if any(ttitle in c or c in ttitle for c in corpus_titles if len(ttitle) > 5):
            violations.append(f"Corpus overlap: {track['title']}")

    return len(violations) == 0, violations

def evaluate_round_scorecard(scores_dict):
    """
    Enforces the gatekeeper rule:
    Every applicable individual score must reach 99.0 - 100.0.
    An average of 99 is not enough if one important feature scores 95.
    """
    failing_criteria = {}
    for crit in SCORECARD_CRITERIA:
        score = scores_dict.get(crit, 0.0)
        if score < MINIMUM_ACCEPTANCE_SCORE:
            failing_criteria[crit] = score

    passed = len(failing_criteria) == 0
    return passed, failing_criteria

def get_pipeline_state():
    state_file = os.path.join(CALIBRATION_DIR, "continuous_pipeline_state.json")
    if os.path.exists(state_file):
        with open(state_file, "r", encoding="utf-8") as f:
            return json.load(f)
    return {
        "pipeline": "DHUN Continuous Intelligence Calibration Pipeline",
        "current_phase": "Phase 2 — Fresh ten-song stress testing",
        "current_round": 1,
        "completed_rounds": [],
        "recalibration_history": [],
        "overall_status": "IN_PROGRESS"
    }

def save_pipeline_state(state):
    state_file = os.path.join(CALIBRATION_DIR, "continuous_pipeline_state.json")
    with open(state_file, "w", encoding="utf-8") as f:
        json.dump(state, f, indent=2)

if __name__ == "__main__":
    reg = load_regression_library()
    state = get_pipeline_state()
    print("=================================================================")
    print("DHUN CONTINUOUS INTELLIGENCE CALIBRATION ORCHESTRATOR")
    print("=================================================================")
    print(f"Status: {state['overall_status']}")
    print(f"Current Phase: {state['current_phase']}")
    print(f"Current Round: Round {state['current_round']}")
    print(f"Regression Library Baseline: {reg['total_count']} verified tracks")
    print(f"Minimum Acceptance Score: {MINIMUM_ACCEPTANCE_SCORE}/100 per individual criterion")
    print(f"Gatekeeper Criteria: {len(SCORECARD_CRITERIA)} monitored dimensions")
    print("Ready to receive Round 1 fresh batch from qualified auditor.")
