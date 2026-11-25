import os
import subprocess
import json

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
FFMPEG = os.path.join(BASE_DIR, "resolver-service", ".venv", "Scripts", "ffmpeg.exe")
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
OUT_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings\stress_test"
os.makedirs(OUT_DIR, exist_ok=True)

TRACKS = {
    "animals": os.path.join(CACHE_DIR, "DuFUtL8zUAk.mp3"),
    "titanium": os.path.join(CACHE_DIR, "JRfuAukYTKg.mp3"),
    "dont_start_now": os.path.join(CACHE_DIR, "oygrmJFKYZY.mp3"),
    "ghungroo": os.path.join(CACHE_DIR, "qFkNATtc3mc.mp3"),
    "zingaat": os.path.join(CACHE_DIR, "Rd9wF5fAnVw.mp3"),
    "malhari": os.path.join(CACHE_DIR, "sRlA7JWTj04.mp3"),
    "big_dawgs": os.path.join(CACHE_DIR, "hOHKltAiKXQ.mp3"),
    "blinding_lights": os.path.join(CACHE_DIR, "fHI8X4OXluQ.mp3"),
    "chaiyya_chaiyya": os.path.join(CACHE_DIR, "49oz7P9d8IA.mp3"),
    "perfect": os.path.join(CACHE_DIR, "1UQzJfsT2eo.mp3"),
}

for name, path in TRACKS.items():
    assert os.path.exists(path), f"Missing track: {name} at {path}"

def run_ffmpeg(cmd, desc):
    print(f"Rendering: {desc}...")
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Error in {desc}:\n{res.stderr}")
        raise RuntimeError(f"FFmpeg failed for {desc}")
    print(f"  -> Done: {desc}")

rendered_files = []

# ==============================================================================
# PAIR A: Animals (128 BPM) -> Blinding Lights (171 / 85.5 BPM) [Extreme Tempo / Half-time]
# ==============================================================================
# Transition A1 - Level 2: Metric Half-time EQ Handover (128 BPM to 85.5 BPM 3:4 sync)
out_a_l2 = os.path.join(OUT_DIR, "01_PairA_Animals_to_BlindingLights_L2_HalfTimeEQHandover.mp3")
cmd_a_l2 = [
    FFMPEG, "-y",
    "-ss", "75.0", "-t", "20.0", "-i", TRACKS["animals"],
    "-ss", "10.0", "-t", "25.0", "-i", TRACKS["blinding_lights"],
    "-filter_complex",
    "[0:a]lowpass=f=800:enable='between(t,15.0,19.5)',afade=t=out:st=16.0:d=3.5[a0];"
    "[1:a]afade=t=in:st=0:d=1.5[a1];"
    "[a0][a1]acrossfade=d=3.5:c1=log:c2=qsin[out]",
    "-map", "[out]", "-b:a", "320k", out_a_l2
]
run_ffmpeg(cmd_a_l2, "Pair A - L2 Half-Time EQ Handover")
rendered_files.append({"id": "pair_a_l2", "path": out_a_l2, "type": "Pair A Level 2"})

# Transition A2 - Level 4: Backspin Cut into Synth Drop
out_a_l4 = os.path.join(OUT_DIR, "01_PairA_Animals_to_BlindingLights_L4_BackspinCut.mp3")
cmd_a_l4 = [
    FFMPEG, "-y",
    "-ss", "75.0", "-t", "16.0", "-i", TRACKS["animals"],
    "-ss", "20.0", "-t", "22.0", "-i", TRACKS["blinding_lights"],
    "-filter_complex",
    "[0:a]asplit=2[m][s];"
    "[s]atrim=start=14.0:end=15.5,asetpts=PTS-STARTPTS,asetrate=48000*2.2,aresample=48000,afade=t=out:st=0.3:d=0.3[spin];"
    "[m]atrim=start=0:end=14.0,asetpts=PTS-STARTPTS[lead];"
    "[lead][spin]concat=n=2:v=0:a=1[a0];"
    "[1:a]afade=t=in:st=0:d=0.1[a1];"
    "[a0][a1]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", "-b:a", "320k", out_a_l4
]
run_ffmpeg(cmd_a_l4, "Pair A - L4 Backspin Cut")
rendered_files.append({"id": "pair_a_l4", "path": out_a_l4, "type": "Pair A Level 4"})

# ==============================================================================
# PAIR B: Perfect (6/8 Ballad) -> Animals (4/4 EDM Drop) [Meter & Energy Contrast]
# ==============================================================================
# Level 1: Safe Quick Cut on downbeat
out_b_l1 = os.path.join(OUT_DIR, "02_PairB_Perfect_to_Animals_L1_SafeQuickCut.mp3")
cmd_b_l1 = [
    FFMPEG, "-y",
    "-ss", "55.0", "-t", "18.0", "-i", TRACKS["perfect"],
    "-ss", "60.0", "-t", "22.0", "-i", TRACKS["animals"],
    "-filter_complex",
    "[0:a]afade=t=out:st=16.5:d=0.8[a0];"
    "[1:a]afade=t=in:st=0:d=0.2[a1];"
    "[a0][a1]acrossfade=d=0.6:c1=tri:c2=tri[out]",
    "-map", "[out]", "-b:a", "320k", out_b_l1
]
run_ffmpeg(cmd_b_l1, "Pair B - L1 Safe Quick Cut")
rendered_files.append({"id": "pair_b_l1", "path": out_b_l1, "type": "Pair B Level 1"})

# Level 3: Ambient Echo-Out Wash across meter boundary
out_b_l3 = os.path.join(OUT_DIR, "02_PairB_Perfect_to_Animals_L3_EchoOutWash.mp3")
cmd_b_l3 = [
    FFMPEG, "-y",
    "-ss", "55.0", "-t", "18.0", "-i", TRACKS["perfect"],
    "-ss", "60.0", "-t", "25.0", "-i", TRACKS["animals"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.7:500:0.5,afade=t=out:st=13.0:d=4.5[a0];"
    "[1:a]afade=t=in:st=0:d=1.5[a1];"
    "[a0][a1]acrossfade=d=4.0:c1=hsin:c2=hsin[out]",
    "-map", "[out]", "-b:a", "320k", out_b_l3
]
run_ffmpeg(cmd_b_l3, "Pair B - L3 Echo-Out Wash")
rendered_files.append({"id": "pair_b_l3", "path": out_b_l3, "type": "Pair B Level 3"})

# ==============================================================================
# PAIR C: Don't Start Now -> Animals [Harmonic Clash Penalty & Bass Swap]
# ==============================================================================
# Level 2: Smart EQ Bass Swap with -14dB mid vocal carve
out_c_l2 = os.path.join(OUT_DIR, "03_PairC_DontStartNow_to_Animals_L2_BassSwapMidCarve.mp3")
cmd_c_l2 = [
    FFMPEG, "-y",
    "-ss", "45.0", "-t", "18.0", "-i", TRACKS["dont_start_now"],
    "-ss", "60.0", "-t", "22.0", "-i", TRACKS["animals"],
    "-filter_complex",
    "[0:a]equalizer=f=1200:width_type=q:w=1.8:g=-14,lowpass=f=500:enable='between(t,14.0,17.5)',afade=t=out:st=15.0:d=2.5[a0];"
    "[1:a]afade=t=in:st=0:d=0.5[a1];"
    "[a0][a1]acrossfade=d=2.2:c1=log:c2=qsin[out]",
    "-map", "[out]", "-b:a", "320k", out_c_l2
]
run_ffmpeg(cmd_c_l2, "Pair C - L2 Bass Swap Mid Carve")
rendered_files.append({"id": "pair_c_l2", "path": out_c_l2, "type": "Pair C Level 2"})

# ==============================================================================
# PAIR D: Big Dawgs -> Chaiyya Chaiyya -> Blinding Lights [Genre Flip & Metric Modulation]
# ==============================================================================
out_d_chain = os.path.join(OUT_DIR, "04_PairD_BigDawgs_to_ChaiyyaChaiyya_to_BlindingLights.mp3")
cmd_d_chain = [
    FFMPEG, "-y",
    "-ss", "30.0", "-t", "25.0", "-i", TRACKS["big_dawgs"],
    "-ss", "45.0", "-t", "30.0", "-i", TRACKS["chaiyya_chaiyya"],
    "-ss", "60.0", "-t", "25.0", "-i", TRACKS["blinding_lights"],
    "-filter_complex",
    "[0:a][1:a]acrossfade=d=3.0:c1=log:c2=qsin[d1];"
    "[d1][2:a]acrossfade=d=3.5:c1=tri:c2=tri[out]",
    "-map", "[out]", "-b:a", "320k", out_d_chain
]
run_ffmpeg(cmd_d_chain, "Pair D - 3-Track Genre Flip & Metric Modulation")
rendered_files.append({"id": "pair_d_chain", "path": out_d_chain, "type": "Pair D Chain"})

# ==============================================================================
# PAIR E: Indian Percussion Gauntlet: Ghungroo -> Malhari -> Zingaat
# ==============================================================================
out_e_gauntlet = os.path.join(OUT_DIR, "05_PairE_Indian_Percussion_Gauntlet_Ghungroo_Malhari_Zingaat.mp3")
cmd_e_gauntlet = [
    FFMPEG, "-y",
    "-ss", "40.0", "-t", "28.0", "-i", TRACKS["ghungroo"],
    "-ss", "45.0", "-t", "28.0", "-i", TRACKS["malhari"],
    "-ss", "50.0", "-t", "28.0", "-i", TRACKS["zingaat"],
    "-filter_complex",
    "[0:a][1:a]acrossfade=d=2.8:c1=tri:c2=tri[e1];"
    "[e1][2:a]acrossfade=d=2.5:c1=log:c2=qsin[out]",
    "-map", "[out]", "-b:a", "320k", out_e_gauntlet
]
run_ffmpeg(cmd_e_gauntlet, "Pair E - Indian Percussion Gauntlet")
rendered_files.append({"id": "pair_e_gauntlet", "path": out_e_gauntlet, "type": "Pair E Gauntlet"})

# ==============================================================================
# PAIR F: Vocal Safety Stress Test: Big Dawgs -> Don't Start Now
# ==============================================================================
out_f_vocal = os.path.join(OUT_DIR, "06_PairF_VocalSafety_BigDawgs_to_DontStartNow.mp3")
cmd_f_vocal = [
    FFMPEG, "-y",
    "-ss", "60.0", "-t", "20.0", "-i", TRACKS["big_dawgs"],
    "-ss", "45.0", "-t", "22.0", "-i", TRACKS["dont_start_now"],
    "-filter_complex",
    "[0:a]equalizer=f=1000:width_type=q:w=1.5:g=-12,afade=t=out:st=17.0:d=2.5[a0];"
    "[1:a]afade=t=in:st=0:d=0.8[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", "-b:a", "320k", out_f_vocal
]
run_ffmpeg(cmd_f_vocal, "Pair F - Vocal Safety Rap Handover")
rendered_files.append({"id": "pair_f_vocal", "path": out_f_vocal, "type": "Pair F Vocal Safety"})

# ==============================================================================
# 7. CONTINUOUS 10-TRACK COMPLETE STRESS MIX
# All 10 songs seamlessly mixed with intelligent transition techniques!
# ==============================================================================
out_continuous = os.path.join(OUT_DIR, "00_DHUN_10_Track_Stress_Test_Continuous_Party_Mix.mp3")
cmd_continuous = [
    FFMPEG, "-y",
    "-ss", "20.0", "-t", "40.0", "-i", TRACKS["animals"],          # 1. Animals (128)
    "-ss", "45.0", "-t", "40.0", "-i", TRACKS["titanium"],         # 2. Titanium (126)
    "-ss", "30.0", "-t", "40.0", "-i", TRACKS["dont_start_now"],   # 3. Don't Start Now (124)
    "-ss", "30.0", "-t", "40.0", "-i", TRACKS["ghungroo"],         # 4. Ghungroo (118)
    "-ss", "45.0", "-t", "40.0", "-i", TRACKS["malhari"],          # 5. Malhari (132)
    "-ss", "50.0", "-t", "40.0", "-i", TRACKS["zingaat"],          # 6. Zingaat (134)
    "-ss", "30.0", "-t", "40.0", "-i", TRACKS["big_dawgs"],        # 7. Big Dawgs (90/180)
    "-ss", "45.0", "-t", "40.0", "-i", TRACKS["chaiyya_chaiyya"],  # 8. Chaiyya Chaiyya (85/171)
    "-ss", "60.0", "-t", "40.0", "-i", TRACKS["blinding_lights"],  # 9. Blinding Lights (171)
    "-ss", "40.0", "-t", "40.0", "-i", TRACKS["perfect"],          # 10. Perfect (95 ballad outro)
    "-filter_complex",
    "[0:a][1:a]acrossfade=d=3.0:c1=tri:c2=tri[m1];"
    "[m1][2:a]acrossfade=d=2.8:c1=log:c2=qsin[m2];"
    "[m2][3:a]acrossfade=d=2.5:c1=tri:c2=tri[m3];"
    "[m3][4:a]acrossfade=d=2.5:c1=log:c2=qsin[m4];"
    "[m4][5:a]acrossfade=d=2.5:c1=tri:c2=tri[m5];"
    "[m5][6:a]acrossfade=d=3.0:c1=log:c2=qsin[m6];"
    "[m6][7:a]acrossfade=d=3.0:c1=tri:c2=tri[m7];"
    "[m7][8:a]acrossfade=d=3.2:c1=log:c2=qsin[m8];"
    "[m8][9:a]acrossfade=d=3.5:c1=hsin:c2=hsin[out]",
    "-map", "[out]", "-b:a", "320k", out_continuous
]
run_ffmpeg(cmd_continuous, "Continuous 10-Track Complete Stress Mix")
rendered_files.append({"id": "continuous_10_track", "path": out_continuous, "type": "Continuous 10-Track Mix"})

# Save manifest of rendered audio
manifest_path = os.path.join(OUT_DIR, "rendered_mixes_manifest.json")
with open(manifest_path, "w", encoding="utf-8") as f:
    json.dump(rendered_files, f, indent=2)

print(f"\nAll {len(rendered_files)} stress-test mixes successfully rendered and logged to {manifest_path}!")
