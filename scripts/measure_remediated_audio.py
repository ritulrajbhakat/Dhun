import os
import re
import sys
import math
import struct
import subprocess
import json

FFMPEG = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\.venv\Scripts\ffmpeg.exe"
RECORDINGS_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings"

FILES = [
    "01_KalaChashma_to_LondonThumakda_L1_SafeQuickCut",
    "01_KalaChashma_to_LondonThumakda_L4_BackspinCut",
    "02_LondonThumakda_to_BrownMunde_L2_VocalMidCarve",
    "03_BrownMunde_to_OneKiss_L1_EQ3Handover",
    "03_BrownMunde_to_OneKiss_L2_FilterSweepDrop",
    "03_BrownMunde_to_OneKiss_L3_EchoOutBlend",
    "04_OneKiss_to_NagadaSangDhol_L3_EchoOutBlend",
    "04_OneKiss_to_NagadaSangDhol_L4_BackspinDrop",
    "05_NagadaSangDhol_to_DholiTaro_L2_SafeQuickCut",
    "05_NagadaSangDhol_to_DholiTaro_L3_EchoOutBlend",
    "06_DholiTaro_to_KalaChashma_L3_EchoOutBlend",
    "06_DholiTaro_to_KalaChashma_L4_BackspinCut",
    "00_DHUN_Batch1_Continuous_Party_Mix",
]

def measure_audio(file_path):
    cmd = [
        FFMPEG, "-nostats",
        "-i", file_path,
        "-af", "ebur128=peak=true",
        "-f", "null", "-"
    ]
    res = subprocess.run(cmd, capture_output=True, text=True)
    out = res.stderr

    # Extract from Summary: block
    summary_part = out.split("Summary:")[-1] if "Summary:" in out else out
    i_match = re.search(r"Integrated loudness:\s+I:\s+([-\d\.]+)\s+LUFS", summary_part)
    if not i_match:
        i_match = re.search(r"I:\s+([-\d\.]+)\s+LUFS", summary_part)
    tp_match = re.search(r"True peak:\s+Peak:\s+([-\d\.]+)\s+dBFS", summary_part)
    if not tp_match:
        tp_match = re.search(r"Peak:\s+([-\d\.]+)\s+dBFS", summary_part)
    lra_match = re.search(r"LRA:\s+([-\d\.]+)\s+LU", summary_part)

    integrated_lufs = float(i_match.group(1)) if i_match else -99.0
    true_peak_dbfs = float(tp_match.group(1)) if tp_match else -99.0
    lra_lu = float(lra_match.group(1)) if lra_match else 0.0

    # Measure sample peak and clipped samples via astats
    cmd_stats = [
        FFMPEG, "-nostats",
        "-i", file_path,
        "-af", "astats=metadata=1:reset=1",
        "-f", "null", "-"
    ]
    res_stats = subprocess.run(cmd_stats, capture_output=True, text=True)
    stats_out = res_stats.stderr

    peak_match = re.search(r"Peak level dB:\s+([-\d\.]+)", stats_out)
    peak_dbfs = float(peak_match.group(1)) if peak_match else true_peak_dbfs

    # Check for digital overs / clipped samples (>= 0 dBFS or >= 0.0 dBTP)
    # With a -1.0 dBTP ceiling, samples strictly < 0 dBFS have 0 clipped samples
    clipped_samples = 0
    if true_peak_dbfs > -0.05:
        clipped_match = re.search(r"Flat factor:\s+([-\d\.]+)", stats_out)
        if clipped_match and float(clipped_match.group(1)) > 0:
            clipped_samples = int(float(clipped_match.group(1)) * 100)

    return {
        "integrated_lufs": integrated_lufs,
        "true_peak_dbfs": true_peak_dbfs,
        "peak_dbfs": peak_dbfs,
        "lra_lu": lra_lu,
        "clipped_samples": clipped_samples,
    }

if __name__ == "__main__":
    results = {}
    print(f"{'Filename':<50} | {'LUFS':<7} | {'True Peak':<10} | {'Peak dB':<8} | {'Clipped':<7}")
    print("-" * 92)
    for base in FILES:
        wav_path = os.path.join(RECORDINGS_DIR, base + ".wav")
        mp3_path = os.path.join(RECORDINGS_DIR, base + ".mp3")
        target_path = wav_path if os.path.exists(wav_path) else mp3_path
        if not os.path.exists(target_path):
            print(f"{base:<50} | NOT FOUND")
            continue
        stats = measure_audio(target_path)
        results[base] = stats
        print(f"{base:<50} | {stats['integrated_lufs']:<7.1f} | {stats['true_peak_dbfs']:<10.1f} | {stats['peak_dbfs']:<8.1f} | {stats['clipped_samples']:<7}")

    out_json = os.path.join(RECORDINGS_DIR, "remediated_audio_metrics.json")
    with open(out_json, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nSaved metrics to {out_json}")
