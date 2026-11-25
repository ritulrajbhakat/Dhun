import os
import sys
import json
import math
import subprocess
import numpy as np

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
BATCHES_DIR = os.path.join(CALIBRATION_DIR, "fresh_batches")
MANIFEST_PATH = os.path.join(BATCHES_DIR, "round_04_resolved_manifest.json")
OUTPUT_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings\round_04_stress_test"
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
print("RENDERING ROUND 4 FRESH STRESS-TEST MIXES")
print("=" * 60)

sr = 44100

# MIX 01: Pair 1 (Montagem - PR Funk -> Cinema Skrillex) [Metric Modulation / Extreme BPM Jump]
# 130 BPM Baile Funk (4A) -> 140/70 BPM Half-time Dubstep (1A)
print("\n[Mix 01] Rendering Pair 1: Montagem - PR Funk -> Cinema Skrillex...")
montagem = load_audio(track_map["montagem_pr_funk"]["local_path"], duration=22.0, offset=40.0)
cinema = load_audio(track_map["cinema_skrillex"]["local_path"], duration=22.0, offset=50.0)

t_overlap = 5.5
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 2.0
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None] ** 0.8

p1_out = np.zeros((len(montagem) + len(cinema) - n_overlap, 2), dtype=np.float32)
p1_out[:len(montagem) - n_overlap] = montagem[:len(montagem) - n_overlap]
blend = montagem[len(montagem) - n_overlap:] * fade_out + cinema[:n_overlap] * fade_in
p1_out[len(montagem) - n_overlap:len(montagem)] = blend
p1_out[len(montagem):] = cinema[n_overlap:]

out_mix_01 = os.path.join(OUTPUT_DIR, "01_Pair1_Montagem_to_Cinema_L3_MetricModulation.mp3")
save_mp3(p1_out, out_mix_01)

# MIX 02: Pair 2 (Baller -> Lean On) [Vocal Safety & Anacrusis Protection]
# 96 BPM Punjabi Hip-Hop (6A) -> 98 BPM Moombahton (8A)
print("\n[Mix 02] Rendering Pair 2: Baller -> Lean On...")
baller = load_audio(track_map["baller"]["local_path"], duration=22.0, offset=40.0)
lean_on = load_audio(track_map["lean_on"]["local_path"], duration=22.0, offset=30.0)

t_overlap = 5.0
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 2.2
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None] ** 0.8

p2_out = np.zeros((len(baller) + len(lean_on) - n_overlap, 2), dtype=np.float32)
p2_out[:len(baller) - n_overlap] = baller[:len(baller) - n_overlap]
blend = baller[len(baller) - n_overlap:] * fade_out + lean_on[:n_overlap] * fade_in
p2_out[len(baller) - n_overlap:len(baller)] = blend
p2_out[len(baller):] = lean_on[n_overlap:]

out_mix_02 = os.path.join(OUTPUT_DIR, "02_Pair2_Baller_to_LeanOn_L2_VocalSafety.mp3")
save_mp3(p2_out, out_mix_02)

# MIX 03: Pair 3 (Deewani Mastani -> Calm Down) [Cross-Meter 6/8 Folk to Straight 4/4 Afro-Swing]
# 114 BPM 6/8 Mujra Court Folk (3B) -> 107 BPM 4/4 Afrobeats Afro-Swing (2B)
print("\n[Mix 03] Rendering Pair 3: Deewani Mastani -> Calm Down...")
deewani = load_audio(track_map["deewani_mastani"]["local_path"], duration=24.0, offset=50.0)
calm = load_audio(track_map["calm_down"]["local_path"], duration=24.0, offset=35.0)

t_overlap = 4.5
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 2.5
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None]

p3_out = np.zeros((len(deewani) + len(calm) - n_overlap, 2), dtype=np.float32)
p3_out[:len(deewani) - n_overlap] = deewani[:len(deewani) - n_overlap]
blend = deewani[len(deewani) - n_overlap:] * fade_out + calm[:n_overlap] * fade_in
p3_out[len(deewani) - n_overlap:len(deewani)] = blend
p3_out[len(deewani):] = calm[n_overlap:]

out_mix_03 = os.path.join(OUTPUT_DIR, "03_Pair3_DeewaniMastani_to_CalmDown_L3_CrossMeterMetricHandOff.mp3")
save_mp3(p3_out, out_mix_03)

# MIX 04: Pair 4 (Aankh Marey -> Wake Me Up) [Harmonic Energy Shift / Complex Camelot Carve]
# 118 BPM (2A / Eb Minor) -> 124 BPM (11B / A Major or 11A / F# Minor)
print("\n[Mix 04] Rendering Pair 4: Aankh Marey -> Wake Me Up...")
aankh = load_audio(track_map["aankh_marey"]["local_path"], duration=22.0, offset=40.0)
wake = load_audio(track_map["wake_me_up"]["local_path"], duration=22.0, offset=50.0)

t_overlap = 5.0
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 2.5
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None] ** 0.8

p4_out = np.zeros((len(aankh) + len(wake) - n_overlap, 2), dtype=np.float32)
p4_out[:len(aankh) - n_overlap] = aankh[:len(aankh) - n_overlap]
blend = aankh[len(aankh) - n_overlap:] * fade_out + wake[:n_overlap] * fade_in
p4_out[len(aankh) - n_overlap:len(aankh)] = blend
p4_out[len(aankh):] = wake[n_overlap:]

out_mix_04 = os.path.join(OUTPUT_DIR, "04_Pair4_AankhMarey_to_WakeMeUp_L4_ComplexCamelotCarve.mp3")
save_mp3(p4_out, out_mix_04)

# MIX 05: Pair 5 (Agar Tum Saath Ho -> Danza Kuduro) [Dynamic Energy Polarity: Ballad to Peak Drop]
# 80 BPM Acoustic Ballad (9B/9A) -> 130 BPM Latin Mambo / Merengue (9A)
print("\n[Mix 05] Rendering Pair 5: Agar Tum Saath Ho -> Danza Kuduro...")
agar = load_audio(track_map["agar_tum_saath_ho"]["local_path"], duration=26.0, offset=50.0)
danza = load_audio(track_map["danza_kuduro"]["local_path"], duration=24.0, offset=45.0)

t_overlap = 4.2
n_overlap = int(t_overlap * sr)
fade_out = np.linspace(1.0, 0.0, n_overlap)[:, None] ** 3.0
fade_in = np.linspace(0.0, 1.0, n_overlap)[:, None]

p5_out = np.zeros((len(agar) + len(danza) - n_overlap, 2), dtype=np.float32)
p5_out[:len(agar) - n_overlap] = agar[:len(agar) - n_overlap]
blend = agar[len(agar) - n_overlap:] * fade_out + danza[:n_overlap] * fade_in
p5_out[len(agar) - n_overlap:len(agar)] = blend
p5_out[len(agar):] = danza[n_overlap:]

out_mix_05 = os.path.join(OUTPUT_DIR, "05_Pair5_AgarTumSaathHo_to_DanzaKuduro_L1_BalladToPeakDrop.mp3")
save_mp3(p5_out, out_mix_05)

# MIX 00: Continuous 10-Track Master Set Arc (Tracks 31–40)
# Progression:
# 1. Agar Tum Saath Ho (80) -> 2. Baller (96) -> 3. Lean On (98) -> 4. Calm Down (107) -> 5. Deewani Mastani (114)
# -> 6. Aankh Marey (118) -> 7. Wake Me Up (124) -> 8. Danza Kuduro (130) -> 9. Montagem - PR Funk (130) -> 10. Cinema Skrillex (140/70)
print("\n[Mix 00] Rendering Continuous 10-Track Master Set Arc...")
cont_segments = [
    ("agar_tum_saath_ho", 35.0, 45.0),
    ("baller", 35.0, 40.0),
    ("lean_on", 35.0, 30.0),
    ("calm_down", 35.0, 35.0),
    ("deewani_mastani", 35.0, 50.0),
    ("aankh_marey", 35.0, 40.0),
    ("wake_me_up", 35.0, 50.0),
    ("danza_kuduro", 35.0, 45.0),
    ("montagem_pr_funk", 35.0, 40.0),
    ("cinema_skrillex", 40.0, 50.0)
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

out_mix_00 = os.path.join(OUTPUT_DIR, "00_DHUN_Round4_Continuous_10_Track_Party_Mix.mp3")
save_mp3(master_mix, out_mix_00)

print("\n" + "=" * 60)
print("ALL ROUND 4 STRESS-TEST MIXES RENDERED SUCCESSFULLY")
print("=" * 60)
