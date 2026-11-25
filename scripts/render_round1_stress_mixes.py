import os
import sys
import json
import numpy as np

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
BATCHES_DIR = os.path.join(CALIBRATION_DIR, "fresh_batches")
MANIFEST_PATH = os.path.join(BATCHES_DIR, "round_01_resolved_manifest.json")
OUTPUT_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings\round_01_stress_test"
METRICS_PATH = os.path.join(OUTPUT_DIR, "round_01_audio_metrics.json")
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
VENV_BIN = os.path.join(BASE_DIR, "resolver-service", ".venv", "Scripts")

os.makedirs(OUTPUT_DIR, exist_ok=True)

if VENV_BIN not in os.environ["PATH"]:
    os.environ["PATH"] = VENV_BIN + os.pathsep + os.environ["PATH"]

def get_track_map():
    if not os.path.exists(MANIFEST_PATH):
        return {}
    with open(MANIFEST_PATH, "r", encoding="utf-8") as f:
        data = json.load(f)
    return {item["id"]: item for item in data}

print("Render script initialized.")
