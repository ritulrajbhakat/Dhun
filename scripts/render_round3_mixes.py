import os
import sys
import json
import math
import subprocess
import numpy as np

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
BATCHES_DIR = os.path.join(CALIBRATION_DIR, "fresh_batches")
MANIFEST_PATH = os.path.join(BATCHES_DIR, "round_03_resolved_manifest.json")
OUTPUT_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings\round_03_stress_test"
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
VENV_BIN = os.path.join(BASE_DIR, "resolver-service", ".venv", "Scripts")
FFMPEG = os.path.join(VENV_BIN, "ffmpeg.exe")

os.makedirs(OUTPUT_DIR, exist_ok=True)

with open(MANIFEST_PATH, "r", encoding="utf-8") as f:
    tracks_list = json.load(f)

track_map = {t["id"]: t for t in tracks_list}

def load_audio(file_path, duration=None, offset=0):
    cmd = [
        FFMPEG, "-y", "-ss", str(offset), "-i", file_path
    ]
    if duration:
        cmd.extend(["-t", str(duration)])
    cmd.extend(["-f", "s16le", "-ac", "2", "-ar", "44100", "-"])
    
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    raw_data, _ = proc.communicate()
    samples = np.frombuffer(raw_data, dtype=np.int16).astype(np.float32) / 32768.0
    return samples.reshape(-1, 2)

def save_mp3(samples, output_path, bitrate="320k"):
    master_gain = 0.8414
    ceiling = 0.7079
    processed = samples * master_gain
    processed = np.clip(processed, -ceiling, ceiling)
    
    int_samples = (processed * 32767.0).astype(np.int16)
    
    cmd = [
        FFMPEG, "-y",
        "-f", "s16le", "-ac", "2", "-ar", "44100", "-i", "-",
        "-b:a", bitrate,
        output_path
    ]
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    proc.communicate(input=int_samples.tobytes())
    print(f"Saved: {output_path} ({os.path.getsize(output_path):,} bytes)")

print("=" * 60)
print("RENDERING ROUND 3 FRESH STRESS-TEST MIXES")
print("=" * 60)

sr = 44100

# MIX 01: Pair 1 (Doja -> Great Spirit) [Metric Modulation / Extreme BPM Jump]
# 70/140 BPM UK Drill (1A) -> 138 BPM Psytrance (11A)
print("\n[Mix 01] Rendering Pair 1: Doja -> Great Spirit...")
doja = load_audio(track_map["doja"]["local_path"], duration=22.0, offset=40.0)
spirit = load_audio(track_map["great_spirit"]["local_path"], duration=22.0, offset=50.0)

t_overlap = 5.5
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 2.0
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None] ** 0.8

p1_out = np.zeros((len(doja) + len(spirit) - n_overlap, 2), dtype=np.float32)
p1_out[:len(doja) - n_overlap] = doja[:len(doja) - n_overlap]
blend = doja[len(doja) - n_overlap:] * fade_out + spirit[:n_overlap] * fade_in
p1_out[len(doja) - n_overlap:len(doja)] = blend
p1_out[len(doja):] = spirit[n_overlap:]

out_mix_01 = os.path.join(OUTPUT_DIR, "01_Pair1_Doja_to_GreatSpirit_L3_MetricModulation.mp3")
save_mp3(p1_out, out_mix_01)

# MIX 02: Pair 2 (295 -> Mi Gente) [Vocal Safety & Anacrusis Protection]
# 90 BPM Punjabi Hip-Hop (9A) -> 105 BPM Moombahton (8A)
print("\n[Mix 02] Rendering Pair 2: 295 -> Mi Gente...")
two_nine_five = load_audio(track_map["two_nine_five"]["local_path"], duration=22.0, offset=45.0)
mi_gente = load_audio(track_map["mi_gente"]["local_path"], duration=22.0, offset=30.0)

t_overlap = 5.0
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 2.2
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None] ** 0.8

p2_out = np.zeros((len(two_nine_five) + len(mi_gente) - n_overlap, 2), dtype=np.float32)
p2_out[:len(two_nine_five) - n_overlap] = two_nine_five[:len(two_nine_five) - n_overlap]
blend = two_nine_five[len(two_nine_five) - n_overlap:] * fade_out + mi_gente[:n_overlap] * fade_in
p2_out[len(two_nine_five) - n_overlap:len(two_nine_five)] = blend
p2_out[len(two_nine_five):] = mi_gente[n_overlap:]

out_mix_02 = os.path.join(OUTPUT_DIR, "02_Pair2_295_to_MiGente_L2_VocalSafety.mp3")
save_mp3(p2_out, out_mix_02)

# MIX 03: Pair 3 (Ghoomar -> Mnike) [Cross-Meter 6/8 Folk to Straight 4/4 Amapiano]
# 116 BPM 6/8 Rajasthani Nagada (11B) -> 113 BPM 4/4 Amapiano Log-Drum (11A)
print("\n[Mix 03] Rendering Pair 3: Ghoomar -> Mnike...")
ghoomar = load_audio(track_map["ghoomar"]["local_path"], duration=24.0, offset=50.0)
mnike = load_audio(track_map["mnike"]["local_path"], duration=24.0, offset=35.0)

t_overlap = 4.5
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 2.5
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None]

p3_out = np.zeros((len(ghoomar) + len(mnike) - n_overlap, 2), dtype=np.float32)
p3_out[:len(ghoomar) - n_overlap] = ghoomar[:len(ghoomar) - n_overlap]
blend = ghoomar[len(ghoomar) - n_overlap:] * fade_out + mnike[:n_overlap] * fade_in
p3_out[len(ghoomar) - n_overlap:len(ghoomar)] = blend
p3_out[len(ghoomar):] = mnike[n_overlap:]

out_mix_03 = os.path.join(OUTPUT_DIR, "03_Pair3_Ghoomar_to_Mnike_L3_CrossMeterMetricHandOff.mp3")
save_mp3(p3_out, out_mix_03)

# MIX 04: Pair 4 (Kar Gayi Chull -> Opus) [Harmonic Energy Shift / Complex Camelot Carve]
# 125 BPM (1A / G# Minor) -> 126 BPM (4A / F Minor) - Minor-third Camelot distance
print("\n[Mix 04] Rendering Pair 4: Kar Gayi Chull -> Opus...")
chull = load_audio(track_map["kar_gayi_chull"]["local_path"], duration=22.0, offset=40.0)
opus = load_audio(track_map["opus"]["local_path"], duration=22.0, offset=60.0)

t_overlap = 5.0
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 2.5
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None] ** 0.8

p4_out = np.zeros((len(chull) + len(opus) - n_overlap, 2), dtype=np.float32)
p4_out[:len(chull) - n_overlap] = chull[:len(chull) - n_overlap]
blend = chull[len(chull) - n_overlap:] * fade_out + opus[:n_overlap] * fade_in
p4_out[len(chull) - n_overlap:len(chull)] = blend
p4_out[len(chull):] = opus[n_overlap:]

out_mix_04 = os.path.join(OUTPUT_DIR, "04_Pair4_KarGayiChull_to_Opus_L4_ComplexCamelotCarve.mp3")
save_mp3(p4_out, out_mix_04)

# MIX 05: Pair 5 (Kun Faya Kun -> Solar System) [Dynamic Energy Polarity: Ballad to Peak DnB Drop]
# 78 BPM Spiritual Sufi (10B) -> 174 BPM Drum & Bass (1A)
print("\n[Mix 05] Rendering Pair 5: Kun Faya Kun -> Solar System...")
kun = load_audio(track_map["kun_faya_kun"]["local_path"], duration=26.0, offset=55.0)
solar = load_audio(track_map["solar_system"]["local_path"], duration=24.0, offset=45.0)

t_overlap = 4.2
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 3.0
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None]

p5_out = np.zeros((len(kun) + len(solar) - n_overlap, 2), dtype=np.float32)
p5_out[:len(kun) - n_overlap] = kun[:len(kun) - n_overlap]
blend = kun[len(kun) - n_overlap:] * fade_out + solar[:n_overlap] * fade_in
p5_out[len(kun) - n_overlap:len(kun)] = blend
p5_out[len(kun):] = solar[n_overlap:]

out_mix_05 = os.path.join(OUTPUT_DIR, "05_Pair5_KunFayaKun_to_SolarSystem_L1_BalladToPeakDrop.mp3")
save_mp3(p5_out, out_mix_05)

# MIX 00: Continuous 10-Track Master Set Arc (Tracks 21–30)
# Progression:
# 1. Kun Faya Kun (78) -> 2. 295 (90) -> 3. Mi Gente (105) -> 4. Mnike (113) -> 5. Ghoomar (116)
# -> 6. Kar Gayi Chull (125) -> 7. Opus (126) -> 8. Great Spirit (138) -> 9. Doja (140/70) -> 10. Solar System (174)
print("\n[Mix 00] Rendering Continuous 10-Track Master Set Arc...")
cont_segments = [
    ("kun_faya_kun", 35.0, 50.0),
    ("two_nine_five", 35.0, 45.0),
    ("mi_gente", 35.0, 30.0),
    ("mnike", 35.0, 35.0),
    ("ghoomar", 35.0, 50.0),
    ("kar_gayi_chull", 35.0, 40.0),
    ("opus", 35.0, 60.0),
    ("great_spirit", 35.0, 50.0),
    ("doja", 35.0, 40.0),
    ("solar_system", 40.0, 45.0)
]

loaded_segments = []
for tid, dur, off in cont_segments:
    audio = load_audio(track_map[tid]["local_path"], duration=dur, offset=off)
    loaded_segments.append(audio)

overlap_sec = 4.5
n_ov = int(overlap_sec * sr)
f_out = np.linspace(1.0, 0.0, n_ov)[:, None]
f_in = np.linspace(0.0, 1.0, n_ov)[:, None]

master_mix = loaded_segments[0]
for i in range(1, len(loaded_segments)):
    next_seg = loaded_segments[i]
    prev_tail = master_mix[-n_ov:]
    next_head = next_seg[:n_ov]
    x_blend = prev_tail * f_out + next_head * f_in
    master_mix = np.vstack([master_mix[:-n_ov], x_blend, next_seg[n_ov:]])

out_mix_00 = os.path.join(OUTPUT_DIR, "00_DHUN_Round3_Continuous_10_Track_Party_Mix.mp3")
save_mp3(master_mix, out_mix_00)

print("\n" + "=" * 60)
print("ALL ROUND 3 STRESS-TEST MIXES RENDERED SUCCESSFULLY")
print("=" * 60)
