import os
import sys
import json
import subprocess
import numpy as np

OUTPUT_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings\round_02_stress_test"
METRICS_PATH = os.path.join(OUTPUT_DIR, "round_02_audio_metrics.json")
MANIFEST_PATH = os.path.join(OUTPUT_DIR, "round_02_rendered_manifest.json")
VENV_BIN = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\.venv\Scripts"
FFMPEG = os.path.join(VENV_BIN, "ffmpeg.exe")

files = [f for f in os.listdir(OUTPUT_DIR) if f.endswith(".mp3")]

results = {}
manifest = []

print("=" * 60)
print("MEASURING ROUND 2 AUDIO METRICS (TRUE PEAK & HEADROOM)")
print("=" * 60)

for fname in sorted(files):
    fpath = os.path.join(OUTPUT_DIR, fname)
    # Decode mp3 to raw PCM to measure true peaks and clips
    cmd = [
        FFMPEG, "-y", "-i", fpath,
        "-f", "s16le", "-ac", "2", "-ar", "44100", "-"
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    raw_data, _ = proc.communicate()
    samples = np.frombuffer(raw_data, dtype=np.int16).astype(np.float32) / 32768.0
    
    # True peak calculation
    abs_samples = np.abs(samples)
    peak = float(np.max(abs_samples))
    peak_db = 20.0 * np.log10(peak) if peak > 0 else -100.0
    rms = float(np.sqrt(np.mean(samples ** 2)))
    rms_db = 20.0 * np.log10(rms) if rms > 0 else -100.0
    
    # Clipped samples count (>= 0.999)
    clipped = int(np.sum(abs_samples >= 0.999))
    duration_s = len(samples) / (44100.0 * 2.0)
    
    status = "COMPLIANT" if peak_db <= -1.0 and clipped == 0 else "NON_COMPLIANT"
    
    metrics = {
        "file_name": fname,
        "file_size": os.path.getsize(fpath),
        "duration_sec": round(duration_s, 2),
        "peak_amplitude": round(peak, 4),
        "true_peak_dbfs": round(peak_db, 2),
        "rms_dbfs": round(rms_db, 2),
        "clipped_samples": clipped,
        "headroom_target_passed": bool(peak_db <= -1.0),
        "zero_clip_guarantee": bool(clipped == 0),
        "status": status
    }
    results[fname] = metrics
    manifest.append(metrics)
    print(f"\n{fname}:")
    print(f"  Duration: {duration_s:.2f}s | Size: {os.path.getsize(fpath):,} bytes")
    print(f"  True Peak: {peak_db:.2f} dBTP | RMS: {rms_db:.2f} dBFS | Clipped: {clipped} | Status: {status}")

with open(METRICS_PATH, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2)

with open(MANIFEST_PATH, "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2)

print("\n" + "=" * 60)
print(f"Metrics saved to {METRICS_PATH}")
print(f"Manifest saved to {MANIFEST_PATH}")
print("=" * 60)
