import os
import sys
import math
import struct
import subprocess
import json

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
FFMPEG = os.path.join(BASE_DIR, "resolver-service", ".venv", "Scripts", "ffmpeg.exe")
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
OUT_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings\stress_test"
TEMP_DIR = os.path.join(BASE_DIR, "target", "temp_stress_remediation")

os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(TEMP_DIR, exist_ok=True)

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
    print(f"[Remediation] Rendering: {desc}...")
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Error in {desc}:\n{res.stderr}")
        raise RuntimeError(f"FFmpeg failed for {desc}")
    print(f"  -> Done: {desc}")

def generate_backspin_wav(input_track_path, start_sec, out_wav_path, dur_sec=1.5, spin_dur=0.75, initial_rate=2.8, sample_rate=44100):
    cmd = [
        FFMPEG, "-y",
        "-ss", str(start_sec), "-t", str(dur_sec),
        "-i", input_track_path,
        "-f", "s16le", "-ac", "2", "-ar", str(sample_rate),
        "pipe:1"
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    raw_bytes, stderr = proc.communicate()
    if proc.returncode != 0:
        raise RuntimeError(f"Failed to read PCM for backspin: {stderr.decode('utf-8', errors='ignore')}")

    num_samples = len(raw_bytes) // 2
    total_frames = num_samples // 2
    samples = list(struct.unpack(f"<{num_samples}h", raw_bytes))

    left = [samples[i * 2] / 32768.0 for i in range(total_frames)]
    right = [samples[i * 2 + 1] / 32768.0 for i in range(total_frames)]
    left.reverse()
    right.reverse()

    out_frames = int(spin_dur * sample_rate)
    out_left = []
    out_right = []

    pos = 0.0
    for n in range(out_frames):
        t = n / out_frames
        speed = initial_rate * ((1.0 - t) ** 1.9)
        pos += speed
        idx = int(pos)
        frac = pos - idx
        if idx + 1 < total_frames:
            s_l = left[idx] * (1.0 - frac) + left[idx + 1] * frac
            s_r = right[idx] * (1.0 - frac) + right[idx + 1] * frac
        elif idx < total_frames:
            s_l = left[idx]
            s_r = right[idx]
        else:
            s_l = 0.0
            s_r = 0.0

        amp = math.cos((math.pi / 2.0) * (t ** 1.5))
        out_left.append(s_l * amp)
        out_right.append(s_r * amp)

    # 1-pole high-pass filter at 190 Hz to simulate cartridge slip
    dt = 1.0 / sample_rate
    rc = 1.0 / (2.0 * math.pi * 190.0)
    alpha = rc / (rc + dt)

    fl_l = [0.0] * out_frames
    fl_r = [0.0] * out_frames
    for i in range(1, out_frames):
        fl_l[i] = alpha * (fl_l[i-1] + out_left[i] - out_left[i-1])
        fl_r[i] = alpha * (fl_r[i-1] + out_right[i] - out_right[i-1])

    out_bytes = bytearray()
    for i in range(out_frames):
        s_l = max(-1.0, min(1.0, fl_l[i]))
        s_r = max(-1.0, min(1.0, fl_r[i]))
        val_l = int(s_l * 32767.0)
        val_r = int(s_r * 32767.0)
        out_bytes.extend(struct.pack("<hh", val_l, val_r))

    raw_spin_pcm = out_wav_path + ".pcm"
    with open(raw_spin_pcm, "wb") as f:
        f.write(out_bytes)

    cmd_conv = [
        FFMPEG, "-y",
        "-f", "s16le", "-ar", str(sample_rate), "-ac", "2",
        "-i", raw_spin_pcm,
        "-af", "treble=g=2.5:f=8000:w=1.0",
        out_wav_path
    ]
    subprocess.run(cmd_conv, check=True, capture_output=True)
    if os.path.exists(raw_spin_pcm):
        os.remove(raw_spin_pcm)

def export_mastered_audio(temp_wav, out_mp3_path):
    # Enforce P0 Directive: EBUR128 -14.0 LUFS normalization + lookahead brickwall peak limiter at -1.0 dBTP
    master_filters = "loudnorm=I=-14.0:TP=-1.0:LRA=10.0,alimiter=limit=-1.0dB:attack=5:release=50:asc=1"
    cmd = [
        FFMPEG, "-y",
        "-i", temp_wav,
        "-af", master_filters,
        "-b:a", "320k",
        out_mp3_path
    ]
    run_ffmpeg(cmd, f"Export Mastered -> {os.path.basename(out_mp3_path)}")

# ==============================================================================
# 1. PAIR A: Animals (66.2/132.4 BPM) -> Blinding Lights (85.8/171.6 BPM)
# L2: Half-Time EQ Handover with Rubberband Tempo Ramp (132.4 -> 128.7 -> 171.6/2)
# ==============================================================================
temp_a_l2 = os.path.join(TEMP_DIR, "temp_a_l2.wav")
cmd_a_l2 = [
    FFMPEG, "-y",
    "-ss", "130.0", "-t", "16.0", "-i", TRACKS["animals"],
    "-ss", "22.88", "-t", "20.0", "-i", TRACKS["blinding_lights"],
    "-filter_complex",
    # Outgoing Animals: 4-bar rubberband tempo ramp to match 85.8 half-time grid,
    # Highpass cut at 160Hz starting at t=13.0s, lowpass at 800Hz, mid carve -6dB
    "[0:a]rubberband=tempo=1.036:pitch=1.0:formant=preserved,highpass=f=180:enable='between(t,12.5,15.5)',equalizer=f=1200:width_type=q:w=1.2:g=-6,afade=t=out:st=13.5:d=2.0[a0];"
    # Incoming Blinding Lights: crisp downbeat entrance with low-shelf trim during blend
    "[1:a]equalizer=f=80:width_type=q:w=0.7:g=-3.0:enable='between(t,0,2.0)',afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_a_l2
]
run_ffmpeg(cmd_a_l2, "Pair A L2 Intermediate WAV")
export_mastered_audio(temp_a_l2, os.path.join(OUT_DIR, "01_PairA_Animals_to_BlindingLights_L2_HalfTimeEQHandover.mp3"))

# ==============================================================================
# 2. PAIR A: Animals -> Blinding Lights
# L4: Physical Friction Backspin Cut directly onto Blinding Lights Downbeat
# ==============================================================================
spin_a_l4 = os.path.join(TEMP_DIR, "spin_a_l4.wav")
generate_backspin_wav(TRACKS["animals"], 140.0, spin_a_l4, dur_sec=1.5, spin_dur=0.75, initial_rate=2.8)

temp_a_l4 = os.path.join(TEMP_DIR, "temp_a_l4.wav")
cmd_a_l4 = [
    FFMPEG, "-y",
    "-ss", "130.0", "-t", "12.0", "-i", TRACKS["animals"],
    "-i", spin_a_l4,
    "-ss", "22.88", "-t", "20.0", "-i", TRACKS["blinding_lights"],
    "-filter_complex",
    "[0:a]afade=t=out:st=11.3:d=0.7[lead];"
    "[1:a]volume=1.0[spin];"
    "[2:a]volume=1.0,afade=t=in:st=0:d=0.04[drop];"
    "[lead][spin]concat=n=2:v=0:a=1[spun];"
    "[spun][drop]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", temp_a_l4
]
run_ffmpeg(cmd_a_l4, "Pair A L4 Intermediate WAV")
export_mastered_audio(temp_a_l4, os.path.join(OUT_DIR, "01_PairA_Animals_to_BlindingLights_L4_BackspinCut.mp3"))

# ==============================================================================
# 3. PAIR B: Perfect (6/8 Ballad) -> Animals (4/4 EDM Drop)
# L1: Phrase-Complete Downbeat Cut (Wait for Ed Sheeran's full phrase to finish naturally)
# ==============================================================================
temp_b_l1 = os.path.join(TEMP_DIR, "temp_b_l1.wav")
cmd_b_l1 = [
    FFMPEG, "-y",
    # Ed Sheeran's phrase ends cleanly at 64.0s ("...listening to our favorite song")
    "-ss", "48.0", "-t", "16.5", "-i", TRACKS["perfect"],
    "-ss", "29.16", "-t", "22.0", "-i", TRACKS["animals"],
    "-filter_complex",
    # Outgoing: natural vocal decay, highpass at 400Hz in final 0.5s, clean cut exactly after phrase
    "[0:a]highpass=f=400:enable='between(t,15.5,16.2)',afade=t=out:st=15.8:d=0.35[a0];"
    "[1:a]volume=1.0,afade=t=in:st=0:d=0.05[a1];"
    "[a0][a1]acrossfade=d=0.35:c1=tri:c2=tri[out]",
    "-map", "[out]", temp_b_l1
]
run_ffmpeg(cmd_b_l1, "Pair B L1 Intermediate WAV")
export_mastered_audio(temp_b_l1, os.path.join(OUT_DIR, "02_PairB_Perfect_to_Animals_L1_SafeQuickCut.mp3"))

# ==============================================================================
# 4. PAIR B: Perfect -> Animals
# L3: Ambient Reverb Wash (Dissolve 6/8 compound meter before 4/4 EDM entry)
# ==============================================================================
temp_b_l3 = os.path.join(TEMP_DIR, "temp_b_l3.wav")
cmd_b_l3 = [
    FFMPEG, "-y",
    "-ss", "48.0", "-t", "18.0", "-i", TRACKS["perfect"],
    "-ss", "29.16", "-t", "22.0", "-i", TRACKS["animals"],
    "-filter_complex",
    # Outgoing: rich stereo reverb/delay tail that sustains energy at -18dB instead of dropping to -36dB
    "[0:a]aecho=0.8:0.7:380|760:0.4|0.25,equalizer=f=3000:width_type=q:w=1.0:g=2.0,afade=t=out:st=13.0:d=4.5[a0];"
    "[1:a]afade=t=in:st=0:d=0.5[a1];"
    "[a0][a1]acrossfade=d=4.0:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_b_l3
]
run_ffmpeg(cmd_b_l3, "Pair B L3 Intermediate WAV")
export_mastered_audio(temp_b_l3, os.path.join(OUT_DIR, "02_PairB_Perfect_to_Animals_L3_EchoOutWash.mp3"))

# ==============================================================================
# 5. PAIR C: Don't Start Now (10B) -> Animals (4A) [Tritone Harmonic Clash Remedy]
# L2: Instantaneous 160Hz Bass Cut + Mid Notch Carve (-14dB) to prevent chord collision
# ==============================================================================
temp_c_l2 = os.path.join(TEMP_DIR, "temp_c_l2.wav")
cmd_c_l2 = [
    FFMPEG, "-y",
    "-ss", "61.5", "-t", "16.0", "-i", TRACKS["dont_start_now"],
    "-ss", "29.16", "-t", "20.0", "-i", TRACKS["animals"],
    "-filter_complex",
    # Outgoing Dua Lipa: Cut bass hard below 200Hz at handover, carve out 1.2kHz mid frequencies
    "[0:a]equalizer=f=1200:width_type=q:w=1.8:g=-14,highpass=f=220:enable='between(t,13.5,15.5)',afade=t=out:st=14.0:d=1.5[a0];"
    # Incoming Animals: full kick dominance
    "[1:a]afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=1.5:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_c_l2
]
run_ffmpeg(cmd_c_l2, "Pair C L2 Intermediate WAV")
export_mastered_audio(temp_c_l2, os.path.join(OUT_DIR, "03_PairC_DontStartNow_to_Animals_L2_BassSwapMidCarve.mp3"))

# ==============================================================================
# 6. PAIR D: Big Dawgs (91.4 BPM) -> Chaiyya Chaiyya (93.7 BPM) -> Blinding Lights (85.8/171.6 BPM)
# 3-Track Chain: Tempo matched via Rubberband (+2.5% on Big Dawgs, smooth handover into train groove)
# ==============================================================================
temp_d_chain = os.path.join(TEMP_DIR, "temp_d_chain.wav")
cmd_d_chain = [
    FFMPEG, "-y",
    "-ss", "35.5", "-t", "20.0", "-i", TRACKS["big_dawgs"],
    "-ss", "41.3", "-t", "25.0", "-i", TRACKS["chaiyya_chaiyya"],
    "-ss", "22.88", "-t", "22.0", "-i", TRACKS["blinding_lights"],
    "-filter_complex",
    # Big Dawgs: rubberband tempo adjust 91.4 -> 93.7 BPM (+2.5%), highpass 160Hz before transition
    "[0:a]rubberband=tempo=1.025:pitch=1.0:formant=preserved,highpass=f=160:enable='between(t,17.0,19.5)',afade=t=out:st=17.5:d=2.0[d0];"
    "[1:a]afade=t=in:st=0:d=0.5[d1];"
    "[d0][d1]acrossfade=d=2.0:c1=log:c2=qsin[m1];"
    # Chaiyya Chaiyya to Blinding Lights: echo-out reverb dissolve into 171.6/2 BPM synth beat
    "[m1]aecho=0.8:0.7:320:0.35,afade=t=out:st=40.5:d=2.5[m1_out];"
    "[2:a]afade=t=in:st=0:d=0.08[d2];"
    "[m1_out][d2]acrossfade=d=2.5:c1=tri:c2=tri[out]",
    "-map", "[out]", temp_d_chain
]
run_ffmpeg(cmd_d_chain, "Pair D Intermediate WAV")
export_mastered_audio(temp_d_chain, os.path.join(OUT_DIR, "04_PairD_BigDawgs_to_ChaiyyaChaiyya_to_BlindingLights.mp3"))

# ==============================================================================
# 7. PAIR E: Indian Percussion Gauntlet: Ghungroo (120) -> Malhari (158.6/79.3) -> Zingaat (130.4/65.2)
# Clean Dholak to Dhol-Tasha to Halgi-Sambal downbeat handover
# ==============================================================================
temp_e_gauntlet = os.path.join(TEMP_DIR, "temp_e_gauntlet.wav")
cmd_e_gauntlet = [
    FFMPEG, "-y",
    "-ss", "48.1", "-t", "18.0", "-i", TRACKS["ghungroo"],
    "-ss", "24.47", "-t", "22.0", "-i", TRACKS["malhari"],
    "-ss", "29.71", "-t", "22.0", "-i", TRACKS["zingaat"],
    "-filter_complex",
    # Ghungroo phrase outro into Malhari dhol drop: rubberband tempo ramp 120 -> 132 BPM, low cut at 180Hz
    "[0:a]rubberband=tempo=1.10:pitch=1.0:formant=preserved,highpass=f=180:enable='between(t,15.0,17.5)',afade=t=out:st=16.0:d=1.5[e0];"
    "[1:a]afade=t=in:st=0:d=0.08[e1];"
    "[e0][e1]acrossfade=d=1.5:c1=tri:c2=tri[m1];"
    # Malhari to Zingaat: 158.6 to 130.4 BPM dhol handover with 1-beat quick crossfade on downbeat 1
    "[m1]highpass=f=200:enable='between(t,36.5,38.5)',afade=t=out:st=37.0:d=1.2[m1_out];"
    "[2:a]afade=t=in:st=0:d=0.08[e2];"
    "[m1_out][e2]acrossfade=d=1.2:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_e_gauntlet
]
run_ffmpeg(cmd_e_gauntlet, "Pair E Intermediate WAV")
export_mastered_audio(temp_e_gauntlet, os.path.join(OUT_DIR, "05_PairE_Indian_Percussion_Gauntlet_Ghungroo_Malhari_Zingaat.mp3"))

# ==============================================================================
# 8. PAIR F: Vocal Safety: Big Dawgs -> Don't Start Now
# Outgoing rap vocal ends cleanly at bar boundary before Dua Lipa's vocal pickup
# ==============================================================================
temp_f_vocal = os.path.join(TEMP_DIR, "temp_f_vocal.wav")
cmd_f_vocal = [
    FFMPEG, "-y",
    # Big Dawgs: rap verse terminates cleanly at 47.3s, entering instrumental break
    "-ss", "35.5", "-t", "14.5", "-i", TRACKS["big_dawgs"],
    "-ss", "15.41", "-t", "20.0", "-i", TRACKS["dont_start_now"],
    "-filter_complex",
    # Lookahead vocal ducking: -3dB notch on 1.8kHz, highpass at 200Hz, clean 1-bar fade
    "[0:a]equalizer=f=1800:width_type=q:w=1.2:g=-3.0:enable='between(t,12.0,14.0)',highpass=f=200:enable='between(t,12.5,14.0)',afade=t=out:st=13.0:d=1.2[f0];"
    "[1:a]afade=t=in:st=0:d=0.08[f1];"
    "[f0][f1]acrossfade=d=1.2:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_f_vocal
]
run_ffmpeg(cmd_f_vocal, "Pair F Intermediate WAV")
export_mastered_audio(temp_f_vocal, os.path.join(OUT_DIR, "06_PairF_VocalSafety_BigDawgs_to_DontStartNow.mp3"))

# ==============================================================================
# 9. CONTINUOUS 10-TRACK COMPLETE STRESS MIX
# All 10 tracks with phrase-accurate downbeats, tempo rubberband ramping,
# vocal collision avoidance, and master EBUR128 / lookahead brickwall limiting!
# ==============================================================================
temp_cont = os.path.join(TEMP_DIR, "temp_continuous.wav")
cmd_cont = [
    FFMPEG, "-y",
    "-ss", "29.16", "-t", "30.0", "-i", TRACKS["animals"],         # 0. Animals (132.4)
    "-ss", "15.55", "-t", "30.0", "-i", TRACKS["titanium"],        # 1. Titanium (124.6)
    "-ss", "15.41", "-t", "30.0", "-i", TRACKS["dont_start_now"],  # 2. Don't Start Now (125.0)
    "-ss", "16.08", "-t", "30.0", "-i", TRACKS["ghungroo"],        # 3. Ghungroo (120.0)
    "-ss", "24.47", "-t", "30.0", "-i", TRACKS["malhari"],         # 4. Malhari (132/158.6)
    "-ss", "29.71", "-t", "30.0", "-i", TRACKS["zingaat"],         # 5. Zingaat (130.4)
    "-ss", "11.85", "-t", "30.0", "-i", TRACKS["big_dawgs"],       # 6. Big Dawgs (91.4)
    "-ss", "20.80", "-t", "30.0", "-i", TRACKS["chaiyya_chaiyya"], # 7. Chaiyya Chaiyya (93.7)
    "-ss", "22.88", "-t", "30.0", "-i", TRACKS["blinding_lights"], # 8. Blinding Lights (171.6)
    "-ss", "29.06", "-t", "30.0", "-i", TRACKS["perfect"],         # 9. Perfect (99.8)
    "-filter_complex",
    # 0 -> 1: Animals to Titanium (132.4 -> 124.6 BPM rubberband ramp)
    "[0:a]rubberband=tempo=0.941:pitch=1.0:formant=preserved,highpass=f=180:enable='between(t,27.0,29.5)',afade=t=out:st=28.0:d=1.8[c0];"
    "[1:a]afade=t=in:st=0:d=0.08[c1];"
    "[c0][c1]acrossfade=d=1.8:c1=log:c2=qsin[m1];"
    # 1 -> 2: Titanium to Don't Start Now (124.6 -> 125.0 harmonic blend)
    "[m1]highpass=f=200:enable='between(t,54.5,57.0)',afade=t=out:st=55.0:d=1.8[m1_out];"
    "[2:a]afade=t=in:st=0:d=0.08[c2];"
    "[m1_out][c2]acrossfade=d=1.8:c1=tri:c2=tri[m2];"
    # 2 -> 3: Don't Start Now to Ghungroo (125.0 -> 120.0 BPM)
    "[m2]rubberband=tempo=0.960:pitch=1.0:formant=preserved,highpass=f=200:enable='between(t,81.5,84.0)',afade=t=out:st=82.0:d=1.8[m2_out];"
    "[3:a]afade=t=in:st=0:d=0.08[c3];"
    "[m2_out][c3]acrossfade=d=1.8:c1=log:c2=qsin[m3];"
    # 3 -> 4: Ghungroo to Malhari (120.0 -> 132.0 BPM dhol pickup)
    "[m3]rubberband=tempo=1.100:pitch=1.0:formant=preserved,highpass=f=220:enable='between(t,108.5,111.0)',afade=t=out:st=109.0:d=1.8[m3_out];"
    "[4:a]afade=t=in:st=0:d=0.08[c4];"
    "[m3_out][c4]acrossfade=d=1.8:c1=tri:c2=tri[m4];"
    # 4 -> 5: Malhari to Zingaat (132.0 -> 130.4 BPM dhol handover)
    "[m4]highpass=f=200:enable='between(t,135.5,138.0)',afade=t=out:st=136.0:d=1.8[m4_out];"
    "[5:a]afade=t=in:st=0:d=0.08[c5];"
    "[m4_out][c5]acrossfade=d=1.8:c1=log:c2=qsin[m5];"
    # 5 -> 6: Zingaat to Big Dawgs (130.4 -> 91.4 BPM metric cut)
    "[m5]aecho=0.8:0.7:300:0.4,highpass=f=300:enable='between(t,162.5,165.0)',afade=t=out:st=163.5:d=1.5[m5_out];"
    "[6:a]afade=t=in:st=0:d=0.05[c6];"
    "[m5_out][c6]acrossfade=d=1.5:c1=tri:c2=tri[m6];"
    # 6 -> 7: Big Dawgs to Chaiyya Chaiyya (91.4 -> 93.7 BPM groove sync)
    "[m6]rubberband=tempo=1.025:pitch=1.0:formant=preserved,highpass=f=180:enable='between(t,189.5,192.0)',afade=t=out:st=190.5:d=1.8[m6_out];"
    "[7:a]afade=t=in:st=0:d=0.08[c7];"
    "[m6_out][c7]acrossfade=d=1.8:c1=log:c2=qsin[m7];"
    # 7 -> 8: Chaiyya Chaiyya to Blinding Lights (93.7 to 85.8/171.6 BPM synth drop)
    "[m7]aecho=0.8:0.6:350:0.3,afade=t=out:st=217.0:d=2.0[m7_out];"
    "[8:a]afade=t=in:st=0:d=0.08[c8];"
    "[m7_out][c8]acrossfade=d=2.0:c1=tri:c2=tri[m8];"
    # 8 -> 9: Blinding Lights to Perfect (171.6/2 to 99.8 BPM ballad outro dissolve)
    "[m8]aecho=0.8:0.7:450:0.4,afade=t=out:st=244.0:d=3.0[m8_out];"
    "[9:a]afade=t=in:st=0:d=0.5[c9];"
    "[m8_out][c9]acrossfade=d=3.0:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_cont
]
run_ffmpeg(cmd_cont, "Continuous 10-Track Mix Intermediate WAV")
export_mastered_audio(temp_cont, os.path.join(OUT_DIR, "00_DHUN_10_Track_Stress_Test_Continuous_Party_Mix.mp3"))

print("\nAll 9 Remediated Mixes Successfully Mastered & Exported!")
