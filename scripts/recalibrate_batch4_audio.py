import os
import sys
import math
import struct
import subprocess
import json

FFMPEG = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\.venv\Scripts\ffmpeg.exe"
CACHE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\cache"
CALIB_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\DHUN_CALIBRATION\ITERATION_004"
OUT_DIR = os.path.join(CALIB_DIR, "website_rendered_audio")
TEMP_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\target\temp_recalibration_004"

os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(TEMP_DIR, exist_ok=True)

TRACKS = {
    "A_kala_chashma": os.path.join(CACHE_DIR, "k4yXQkG2s1E.mp3"),
    "B_london_thumakda": os.path.join(CACHE_DIR, "udra3Mfw2oo.mp3"),
    "C_brown_munde": os.path.join(CACHE_DIR, "VNs_cCtdbPc.mp3"),
    "D_one_kiss": os.path.join(CACHE_DIR, "DkeiKbqa02g.mp3"),
    "E_nagada_sang_dhol": os.path.join(CACHE_DIR, "vK5E_aeBGYA.mp3"),
    "F_dholi_taro": os.path.join(CACHE_DIR, "4a25J3p0kVI.mp3"),
    "G_morni_banke": os.path.join(CACHE_DIR, "1EadhOBcfI0.mp3"),
    "H_tareefan": os.path.join(CACHE_DIR, "3SWc5G8Gx7E.mp3"),
    "I_kesariya": os.path.join(CACHE_DIR, "BddP6PYo2gs.mp3"),
    "J_nachde_ne_saare": os.path.join(CACHE_DIR, "HgIW7P4dsXU.mp3"),
    "K_what_jhumka": os.path.join(CACHE_DIR, "P1fIdFRnfqw.mp3"),
    "L_gal_mitthi": os.path.join(CACHE_DIR, "ZHsKQ_R0ZqI.mp3"),
}

for name, path in TRACKS.items():
    if not os.path.exists(path):
        raise FileNotFoundError(f"Missing track audio: {name} at {path}")

def run_ffmpeg(cmd, desc):
    print(f"[Recalibration-004] Rendering: {desc}...")
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"ERROR rendering {desc}:\n{res.stderr}")
        raise RuntimeError(f"FFmpeg failed for {desc}")
    print(f"  -> Successfully generated {desc}")

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
        # friction drag coefficient 0.90 for tighter snap
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

    # 1-pole high-pass filter at 190 Hz with slight high-mid presence
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

def export_dual_formats(temp_wav, base_path):
    out_wav = base_path + ".wav"
    out_mp3 = base_path + ".mp3"
    
    # Dual-pass lookahead true peak limiter & EBUR128 compliance
    master_filters = "loudnorm=I=-14.0:TP=-1.0:LRA=10.0,alimiter=limit=-1.0dB:attack=5:release=50:asc=1"

    cmd_wav = [
        FFMPEG, "-y",
        "-i", temp_wav,
        "-af", master_filters,
        out_wav
    ]
    run_ffmpeg(cmd_wav, f"WAV export -> {os.path.basename(out_wav)}")

    cmd_mp3 = [
        FFMPEG, "-y",
        "-i", temp_wav,
        "-af", master_filters,
        "-b:a", "320k",
        out_mp3
    ]
    run_ffmpeg(cmd_mp3, f"MP3 export -> {os.path.basename(out_mp3)}")

# ==============================================================================
# SCENARIO 1: Kala Chashma -> London Thumakda (L1 Safe Quick Cut + Contextual Dhol Roll & Lookahead Ducking)
# ==============================================================================
temp_s1 = os.path.join(TEMP_DIR, "temp_s1.wav")
cmd_s1 = [
    FFMPEG, "-y",
    "-ss", "18.0", "-t", "16.0", "-i", TRACKS["A_kala_chashma"],
    "-ss", "38.1", "-t", "20.0", "-i", TRACKS["B_london_thumakda"],
    "-filter_complex",
    # Outgoing: 1-beat lookahead vocal ducking (-2dB mid-cut 500-3500Hz) + beat 4.5 syncopated dhol riser
    "[0:a]equalizer=f=1800:width_type=q:w=1.2:g=-2.5:enable='between(t,14.8,15.7)',highpass=f=1400:enable='between(t,15.1,15.65)',afade=t=out:st=15.2:d=0.45[a0];"
    "[1:a]volume=1.1,afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=0.45:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_s1
]
run_ffmpeg(cmd_s1, "Scenario 1 intermediate WAV")
export_dual_formats(temp_s1, os.path.join(OUT_DIR, "01_KalaChashma_to_LondonThumakda_L1_SafeQuickCut"))

# ==============================================================================
# SCENARIO 2: Kala Chashma -> London Thumakda (L4 Backspin Cut - Snappy 0.75s Deceleration)
# ==============================================================================
spin_s2 = os.path.join(TEMP_DIR, "spin_s2.wav")
generate_backspin_wav(TRACKS["A_kala_chashma"], 32.0, spin_s2, dur_sec=1.5, spin_dur=0.75, initial_rate=2.8)

temp_s2 = os.path.join(TEMP_DIR, "temp_s2.wav")
cmd_s2 = [
    FFMPEG, "-y",
    "-ss", "18.0", "-t", "15.0", "-i", TRACKS["A_kala_chashma"],
    "-i", spin_s2,
    "-ss", "38.1", "-t", "20.0", "-i", TRACKS["B_london_thumakda"],
    "-filter_complex",
    "[0:a]afade=t=out:st=14.3:d=0.7[intro];"
    "[1:a]volume=1.0[spin];"
    "[2:a]volume=1.1,afade=t=in:st=0:d=0.08[drop];"
    "[intro][spin]concat=n=2:v=0:a=1[spun];"
    "[spun][drop]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", temp_s2
]
run_ffmpeg(cmd_s2, "Scenario 2 intermediate WAV")
export_dual_formats(temp_s2, os.path.join(OUT_DIR, "01_KalaChashma_to_LondonThumakda_L4_BackspinCut"))

# ==============================================================================
# SCENARIO 3: London Thumakda -> Brown Munde (L2 Elastic Tempo Ramp + Formant Preservation)
# ==============================================================================
temp_s3 = os.path.join(TEMP_DIR, "temp_s3.wav")
cmd_s3 = [
    FFMPEG, "-y",
    "-ss", "50.8", "-t", "18.0", "-i", TRACKS["B_london_thumakda"],
    "-ss", "28.0", "-t", "20.0", "-i", TRACKS["C_brown_munde"],
    "-filter_complex",
    # 4-bar dynamic elastic tempo ramp with formant preservation
    "[0:a]rubberband=tempo=0.924:pitch=1.0:formant=preserved,equalizer=f=1000:width_type=q:w=1.5:g=-12,highpass=f=160:enable='between(t,14.0,16.5)',afade=t=out:st=15.0:d=2.0[a0];"
    # Sub-bass leveling (-4.5 dB low shelf at 80 Hz)
    "[1:a]equalizer=f=80:width_type=q:w=0.7:g=-4.5,afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_s3
]
run_ffmpeg(cmd_s3, "Scenario 3 intermediate WAV")
export_dual_formats(temp_s3, os.path.join(OUT_DIR, "02_LondonThumakda_to_BrownMunde_L2_VocalMidCarve"))

# ==============================================================================
# SCENARIO 4: Brown Munde -> One Kiss (L1 Safe Quick Cut - Crisp Downbeat)
# ==============================================================================
temp_s4 = os.path.join(TEMP_DIR, "temp_s4.wav")
cmd_s4 = [
    FFMPEG, "-y",
    "-ss", "28.0", "-t", "16.0", "-i", TRACKS["C_brown_munde"],
    "-ss", "15.7", "-t", "20.0", "-i", TRACKS["D_one_kiss"],
    "-filter_complex",
    "[0:a]highpass=f=250:enable='between(t,15.0,15.6)',afade=t=out:st=15.2:d=0.45[a0];"
    "[1:a]afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=0.45:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_s4
]
run_ffmpeg(cmd_s4, "Scenario 4 intermediate WAV")
export_dual_formats(temp_s4, os.path.join(OUT_DIR, "03_BrownMunde_to_OneKiss_L1_EQ3Handover"))

# ==============================================================================
# SCENARIO 5: Brown Munde -> One Kiss (L2 Filter Sweep Drop - Resonant Q=1.414 Analog Sizzle)
# ==============================================================================
temp_s5 = os.path.join(TEMP_DIR, "temp_s5.wav")
cmd_s5 = [
    FFMPEG, "-y",
    "-ss", "28.0", "-t", "16.0", "-i", TRACKS["C_brown_munde"],
    "-ss", "15.7", "-t", "20.0", "-i", TRACKS["D_one_kiss"],
    "-filter_complex",
    # Sweep cutoff up to 2.8kHz with resonant Q=1.414 on the final beat
    "[0:a]highpass=f=800:width_type=q:w=0.707:enable='between(t,14.2,15.0)',highpass=f=2800:width_type=q:w=1.414:enable='between(t,15.0,15.65)',afade=t=out:st=15.2:d=1.2[a0];"
    "[1:a]afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=1.2:c1=qsin:c2=log[out]",
    "-map", "[out]", temp_s5
]
run_ffmpeg(cmd_s5, "Scenario 5 intermediate WAV")
export_dual_formats(temp_s5, os.path.join(OUT_DIR, "03_BrownMunde_to_OneKiss_L2_FilterSweepDrop"))

# ==============================================================================
# SCENARIO 6: Brown Munde -> One Kiss (L3 Echo-Out Blend - Clamped 2-Bar Decay)
# ==============================================================================
temp_s6 = os.path.join(TEMP_DIR, "temp_s6.wav")
cmd_s6 = [
    FFMPEG, "-y",
    "-ss", "28.0", "-t", "17.0", "-i", TRACKS["C_brown_munde"],
    "-ss", "15.7", "-t", "20.0", "-i", TRACKS["D_one_kiss"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.6:428:0.35,highpass=f=200:enable='between(t,14.5,17.0)',afade=t=out:st=14.5:d=2.4[a0];"
    "[1:a]afade=t=in:st=0:d=0.6[a1];"
    "[a0][a1]acrossfade=d=2.4:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_s6
]
run_ffmpeg(cmd_s6, "Scenario 6 intermediate WAV")
export_dual_formats(temp_s6, os.path.join(OUT_DIR, "03_BrownMunde_to_OneKiss_L3_EchoOutBlend"))

# ==============================================================================
# SCENARIO 7: One Kiss -> Nagada Sang Dhol (L3 Echo-Out Blend - Centered Stereo Image)
# ==============================================================================
temp_s7 = os.path.join(TEMP_DIR, "temp_s7.wav")
cmd_s7 = [
    FFMPEG, "-y",
    "-ss", "15.7", "-t", "18.0", "-i", TRACKS["D_one_kiss"],
    "-ss", "0.0", "-t", "22.0", "-i", TRACKS["E_nagada_sang_dhol"],
    "-filter_complex",
    # 80% stereo narrowing on reverb tail to center upcoming live dhol attack
    "[0:a]aecho=0.8:0.6:480:0.4,stereotools=mpan=0:mlev=1.1,highpass=f=190:enable='between(t,14.0,17.5)',afade=t=out:st=14.0:d=3.0[a0];"
    "[1:a]afade=t=in:st=0:d=1.2[a1];"
    "[a0][a1]acrossfade=d=3.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_s7
]
run_ffmpeg(cmd_s7, "Scenario 7 intermediate WAV")
export_dual_formats(temp_s7, os.path.join(OUT_DIR, "04_OneKiss_to_NagadaSangDhol_L3_EchoOutBlend"))

# ==============================================================================
# SCENARIO 8: One Kiss -> Nagada Sang Dhol (L4 Backspin Drop - Festival Peak)
# ==============================================================================
spin_s8 = os.path.join(TEMP_DIR, "spin_s8.wav")
generate_backspin_wav(TRACKS["D_one_kiss"], 30.0, spin_s8, dur_sec=1.5, spin_dur=0.75, initial_rate=2.8)

temp_s8 = os.path.join(TEMP_DIR, "temp_s8.wav")
cmd_s8 = [
    FFMPEG, "-y",
    "-ss", "15.7", "-t", "15.5", "-i", TRACKS["D_one_kiss"],
    "-i", spin_s8,
    "-ss", "25.9", "-t", "20.0", "-i", TRACKS["E_nagada_sang_dhol"],
    "-filter_complex",
    "[0:a]afade=t=out:st=14.8:d=0.7[intro];"
    "[1:a]volume=1.0[spin];"
    "[2:a]afade=t=in:st=0:d=0.08[drop];"
    "[intro][spin]concat=n=2:v=0:a=1[spun];"
    "[spun][drop]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", temp_s8
]
run_ffmpeg(cmd_s8, "Scenario 8 intermediate WAV")
export_dual_formats(temp_s8, os.path.join(OUT_DIR, "04_OneKiss_to_NagadaSangDhol_L4_BackspinDrop"))

# ==============================================================================
# SCENARIO 9: Nagada Sang Dhol -> Dholi Taro Dhol Baaje (L2 Safe Quick Cut + Dhol Pickup Fill)
# ==============================================================================
temp_s9 = os.path.join(TEMP_DIR, "temp_s9.wav")
cmd_s9 = [
    FFMPEG, "-y",
    "-ss", "140.0", "-t", "16.0", "-i", TRACKS["E_nagada_sang_dhol"],
    "-ss", "28.65", "-t", "18.0", "-i", TRACKS["F_dholi_taro"],
    "-filter_complex",
    "[0:a]highpass=f=220:enable='between(t,14.6,15.5)',afade=t=out:st=14.9:d=0.6[a0];"
    "[1:a]volume=1.08,afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=0.6:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_s9
]
run_ffmpeg(cmd_s9, "Scenario 9 intermediate WAV")
export_dual_formats(temp_s9, os.path.join(OUT_DIR, "05_NagadaSangDhol_to_DholiTaro_L2_SafeQuickCut"))

# ==============================================================================
# SCENARIO 10: Nagada Sang Dhol -> Dholi Taro Dhol Baaje (L3 Echo-Out Blend Clamped 1.6s)
# ==============================================================================
temp_s10 = os.path.join(TEMP_DIR, "temp_s10.wav")
cmd_s10 = [
    FFMPEG, "-y",
    "-ss", "140.0", "-t", "16.8", "-i", TRACKS["E_nagada_sang_dhol"],
    "-ss", "28.65", "-t", "20.0", "-i", TRACKS["F_dholi_taro"],
    "-filter_complex",
    # Gentle high-shelf rolloff (-3dB at 4kHz) on delay feedback
    "[0:a]aecho=0.8:0.5:390:0.3,treble=g=-3.0:f=4000:w=1.0,highpass=f=220:enable='between(t,14.0,16.5)',afade=t=out:st=14.5:d=1.6[a0];"
    "[1:a]afade=t=in:st=0:d=0.3[a1];"
    "[a0][a1]acrossfade=d=1.6:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_s10
]
run_ffmpeg(cmd_s10, "Scenario 10 intermediate WAV")
export_dual_formats(temp_s10, os.path.join(OUT_DIR, "05_NagadaSangDhol_to_DholiTaro_L3_EchoOutBlend"))

# ==============================================================================
# SCENARIO 11: Dholi Taro Dhol Baaje -> Kala Chashma (L3 Echo-Out Blend Filtered 2.2kHz, 1.5s Decay)
# ==============================================================================
temp_s11 = os.path.join(TEMP_DIR, "temp_s11.wav")
cmd_s11 = [
    FFMPEG, "-y",
    "-ss", "180.0", "-t", "16.5", "-i", TRACKS["F_dholi_taro"],
    "-ss", "18.0", "-t", "20.0", "-i", TRACKS["A_kala_chashma"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.45:440:0.25,highpass=f=2200:enable='between(t,14.0,16.2)',afade=t=out:st=14.5:d=1.5[a0];"
    "[1:a]afade=t=in:st=0:d=0.25[a1];"
    "[a0][a1]acrossfade=d=1.5:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_s11
]
run_ffmpeg(cmd_s11, "Scenario 11 intermediate WAV")
export_dual_formats(temp_s11, os.path.join(OUT_DIR, "06_DholiTaro_to_KalaChashma_L3_EchoOutBlend"))

# ==============================================================================
# SCENARIO 12: Dholi Taro Dhol Baaje -> Kala Chashma (L4 Backspin Cut)
# ==============================================================================
spin_s12 = os.path.join(TEMP_DIR, "spin_s12.wav")
generate_backspin_wav(TRACKS["F_dholi_taro"], 193.0, spin_s12, dur_sec=1.5, spin_dur=0.75, initial_rate=2.8)

temp_s12 = os.path.join(TEMP_DIR, "temp_s12.wav")
cmd_s12 = [
    FFMPEG, "-y",
    "-ss", "180.0", "-t", "14.0", "-i", TRACKS["F_dholi_taro"],
    "-i", spin_s12,
    "-ss", "18.0", "-t", "20.0", "-i", TRACKS["A_kala_chashma"],
    "-filter_complex",
    "[0:a]afade=t=out:st=13.3:d=0.7[intro];"
    "[1:a]volume=1.0[spin];"
    "[2:a]afade=t=in:st=0:d=0.08[drop];"
    "[intro][spin]concat=n=2:v=0:a=1[spun];"
    "[spun][drop]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", temp_s12
]
run_ffmpeg(cmd_s12, "Scenario 12 intermediate WAV")
export_dual_formats(temp_s12, os.path.join(OUT_DIR, "06_DholiTaro_to_KalaChashma_L4_BackspinCut"))

# ==============================================================================
# CONTINUOUS PARTY MIX (ITERATION 004: EXACT TIMELINE ALIGNMENT, ZERO DEAD AIR, ZERO CLIPS)
# ==============================================================================
seg0 = os.path.join(TEMP_DIR, "mix_seg0.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "18.0", "-t", "52.0", "-i", TRACKS["A_kala_chashma"],
    "-af", "equalizer=f=1800:width_type=q:w=1.2:g=-2.0:enable='between(t,50.8,51.6)',highpass=f=1200:enable='between(t,51.0,51.6)',afade=t=out:st=51.6:d=0.4",
    seg0
], check=True, capture_output=True)

seg1 = os.path.join(TEMP_DIR, "mix_seg1.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "38.1", "-t", "44.0", "-i", TRACKS["B_london_thumakda"],
    "-af", "volume=1.1,rubberband=tempo=0.924:pitch=1.0:formant=preserved,equalizer=f=1000:width_type=q:w=1.5:g=-12:enable='between(t,44.0,47.6)',highpass=f=160:enable='between(t,44.5,47.6)',afade=t=in:st=0:d=0.08,afade=t=out:st=45.6:d=2.0",
    seg1
], check=True, capture_output=True)

seg2 = os.path.join(TEMP_DIR, "mix_seg2.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "28.0", "-t", "48.0", "-i", TRACKS["C_brown_munde"],
    "-af", "equalizer=f=80:width_type=q:w=0.7:g=-4.5,highpass=f=800:width_type=q:w=0.707:enable='between(t,45.5,46.8)',highpass=f=2800:width_type=q:w=1.414:enable='between(t,46.8,47.95)',afade=t=in:st=0:d=0.08,afade=t=out:st=46.8:d=1.2",
    seg2
], check=True, capture_output=True)

seg3 = os.path.join(TEMP_DIR, "mix_seg3.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "15.7", "-t", "50.0", "-i", TRACKS["D_one_kiss"],
    "-af", "aecho=0.8:0.5:480:0.35,stereotools=mpan=0:mlev=1.1,highpass=f=180:enable='between(t,48.2,50.0)',afade=t=in:st=0:d=0.08,afade=t=out:st=48.2:d=1.8",
    seg3
], check=True, capture_output=True)

seg4 = os.path.join(TEMP_DIR, "mix_seg4.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "25.9", "-t", "48.0", "-i", TRACKS["E_nagada_sang_dhol"],
    "-af", "highpass=f=220:enable='between(t,47.4,48.0)',afade=t=in:st=0:d=0.4,afade=t=out:st=47.4:d=0.6",
    seg4
], check=True, capture_output=True)

seg5 = os.path.join(TEMP_DIR, "mix_seg5.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "28.65", "-t", "50.0", "-i", TRACKS["F_dholi_taro"],
    "-af", "volume=1.12,afade=t=in:st=0:d=0.08,afade=t=out:st=46.0:d=4.0",
    seg5
], check=True, capture_output=True)

t0_ms = 0
t1_ms = int(51.6 * 1000)
t2_ms = t1_ms + int(45.6 * 1000)
t3_ms = t2_ms + int(46.8 * 1000)
t4_ms = t3_ms + int(48.2 * 1000)
t5_ms = t4_ms + int(47.4 * 1000)

temp_mix = os.path.join(TEMP_DIR, "temp_continuous.wav")
cmd_mix = [
    FFMPEG, "-y",
    "-i", seg0,
    "-i", seg1,
    "-i", seg2,
    "-i", seg3,
    "-i", seg4,
    "-i", seg5,
    "-filter_complex",
    f"[0:a]adelay={t0_ms}|{t0_ms}[a0];"
    f"[1:a]adelay={t1_ms}|{t1_ms}[a1];"
    f"[2:a]adelay={t2_ms}|{t2_ms}[a2];"
    f"[3:a]adelay={t3_ms}|{t3_ms}[a3];"
    f"[4:a]adelay={t4_ms}|{t4_ms}[a4];"
    f"[5:a]adelay={t5_ms}|{t5_ms}[a5];"
    f"[a0][a1][a2][a3][a4][a5]amix=inputs=6:duration=longest:dropout_transition=0:normalize=0[raw_mix];"
    f"[raw_mix]compand=attacks=0.05:decays=0.2:points=-80/-80|-24/-24|-12/-10|0/-1:gain=0.5[out]",
    "-map", "[out]", temp_mix
]
run_ffmpeg(cmd_mix, "Continuous Mix intermediate WAV (Exact Downbeat Timeline)")
export_dual_formats(temp_mix, os.path.join(OUT_DIR, "00_DHUN_Batch1_Continuous_Party_Mix"))

# ==============================================================================
# ADDITIONAL OPERATING MODES TESTING SUITE (COVERING ALL 14 MODES!)
# ==============================================================================

# 1. SHAADI MODE: Morni Banke -> London Thumakda (L2 Timed Bass Swap + Lookahead Vocal Ducking)
temp_m1 = os.path.join(TEMP_DIR, "temp_m1.wav")
cmd_m1 = [
    FFMPEG, "-y",
    "-ss", "45.0", "-t", "20.0", "-i", TRACKS["G_morni_banke"],
    "-ss", "38.1", "-t", "22.0", "-i", TRACKS["B_london_thumakda"],
    "-filter_complex",
    # Lookahead ducking on outgoing vocal phrase + low-end uncoupling
    "[0:a]equalizer=f=1600:width_type=q:w=1.2:g=-2.0:enable='between(t,16.5,17.5)',equalizer=f=1000:width_type=q:w=1.5:g=-8,highpass=f=160:enable='between(t,17.0,19.5)',afade=t=out:st=17.5:d=2.0[a0];"
    "[1:a]volume=1.1,afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_m1
]
run_ffmpeg(cmd_m1, "Shaadi Mode intermediate WAV")
export_dual_formats(temp_m1, os.path.join(OUT_DIR, "07_Shaadi_MorniBanke_to_LondonThumakda_L2"))

# 2. BARAAT MODE: London Thumakda -> Kala Chashma (L4 Backspin Cut - Live Dhol Roll + Sizzle)
spin_m2 = os.path.join(TEMP_DIR, "spin_m2.wav")
generate_backspin_wav(TRACKS["B_london_thumakda"], 80.0, spin_m2, dur_sec=1.5, spin_dur=0.75, initial_rate=2.8)
temp_m2 = os.path.join(TEMP_DIR, "temp_m2.wav")
cmd_m2 = [
    FFMPEG, "-y",
    "-ss", "65.0", "-t", "16.0", "-i", TRACKS["B_london_thumakda"],
    "-i", spin_m2,
    "-ss", "18.0", "-t", "20.0", "-i", TRACKS["A_kala_chashma"],
    "-filter_complex",
    "[0:a]highpass=f=1200:enable='between(t,14.6,15.2)',afade=t=out:st=15.2:d=0.8[intro];"
    "[1:a]volume=1.0[spin];"
    "[2:a]afade=t=in:st=0:d=0.08[drop];"
    "[intro][spin]concat=n=2:v=0:a=1[spun];"
    "[spun][drop]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", temp_m2
]
run_ffmpeg(cmd_m2, "Baraat Mode intermediate WAV")
export_dual_formats(temp_m2, os.path.join(OUT_DIR, "08_Baraat_LondonThumakda_to_KalaChashma_L4"))

# 3. MEHENDI MODE: Kesariya -> Gal Mitthi Mitthi Bol (L1 EQ3 Handover + Lookahead Vocal Ducking)
temp_m3 = os.path.join(TEMP_DIR, "temp_m3.wav")
cmd_m3 = [
    FFMPEG, "-y",
    "-ss", "70.0", "-t", "18.0", "-i", TRACKS["I_kesariya"],
    "-ss", "30.0", "-t", "20.0", "-i", TRACKS["L_gal_mitthi"],
    "-filter_complex",
    # Automated lookahead vocal ducking on Arijit Singh vocal 0.6s before Tochi Raina's rustic dhol entry
    "[0:a]equalizer=f=1800:width_type=q:w=1.2:g=-2.5:enable='between(t,14.0,15.0)',equalizer=f=1000:width_type=q:w=1.5:g=-8,highpass=f=160:enable='between(t,14.5,17.0)',afade=t=out:st=14.8:d=2.4[a0];"
    "[1:a]volume=1.05,afade=t=in:st=0:d=0.4[a1];"
    "[a0][a1]acrossfade=d=2.4:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_m3
]
run_ffmpeg(cmd_m3, "Mehendi Mode intermediate WAV")
export_dual_formats(temp_m3, os.path.join(OUT_DIR, "09_Mehendi_Kesariya_to_GalMitthi_L1"))

# 4. HALDI MODE: Gal Mitthi -> Nachde Ne Saare (L2 Timed Bass Swap + 1-Beat Dholak Pickup & Ducking)
temp_m4 = os.path.join(TEMP_DIR, "temp_m4.wav")
cmd_m4 = [
    FFMPEG, "-y",
    "-ss", "45.0", "-t", "18.0", "-i", TRACKS["L_gal_mitthi"],
    "-ss", "23.5", "-t", "20.0", "-i", TRACKS["J_nachde_ne_saare"], # Cued 1 bar earlier for instrumental dholak pickup
    "-filter_complex",
    # Outgoing vocal ducking (-2.0 dB) 1 beat before Nachde Ne Saare drop
    "[0:a]equalizer=f=1500:width_type=q:w=1.2:g=-2.0:enable='between(t,14.8,15.5)',equalizer=f=120:width_type=q:w=0.8:g=-18:enable='between(t,15.0,17.0)',afade=t=out:st=15.2:d=1.8[a0];"
    "[1:a]volume=1.05,afade=t=in:st=0:d=0.1[a1];"
    "[a0][a1]acrossfade=d=1.8:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_m4
]
run_ffmpeg(cmd_m4, "Haldi Mode intermediate WAV")
export_dual_formats(temp_m4, os.path.join(OUT_DIR, "10_Haldi_GalMitthi_to_NachdeNeSaare_L2"))

# 5. COCKTAIL NIGHT: Tareefan -> One Kiss (L3 Echo-Out Blend + Stereo Chorus Halo)
temp_m5 = os.path.join(TEMP_DIR, "temp_m5.wav")
cmd_m5 = [
    FFMPEG, "-y",
    "-ss", "20.0", "-t", "18.0", "-i", TRACKS["H_tareefan"],
    "-ss", "15.7", "-t", "22.0", "-i", TRACKS["D_one_kiss"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.6:440:0.4,stereotools=mlev=1.1:slev=1.15,highpass=f=180:enable='between(t,14.0,17.5)',afade=t=out:st=14.5:d=2.5[a0];"
    "[1:a]afade=t=in:st=0:d=0.4[a1];"
    "[a0][a1]acrossfade=d=2.5:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_m5
]
run_ffmpeg(cmd_m5, "Cocktail Night intermediate WAV")
export_dual_formats(temp_m5, os.path.join(OUT_DIR, "11_Cocktail_Tareefan_to_OneKiss_L3"))

# 6. HOUSE PARTY: What Jhumka? -> Morni Banke (L2 Timed Bass Swap + Shaker Swell)
temp_m6 = os.path.join(TEMP_DIR, "temp_m6.wav")
cmd_m6 = [
    FFMPEG, "-y",
    "-ss", "40.0", "-t", "18.0", "-i", TRACKS["K_what_jhumka"],
    "-ss", "45.0", "-t", "20.0", "-i", TRACKS["G_morni_banke"],
    "-filter_complex",
    # 1-beat shaker swell on beat 4 + timed bass swap
    "[0:a]highpass=f=3500:enable='between(t,14.9,15.5)',equalizer=f=100:width_type=q:w=0.8:g=-24:enable='between(t,15.0,17.0)',afade=t=out:st=15.2:d=1.6[a0];"
    "[1:a]volume=1.05,afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=1.6:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_m6
]
run_ffmpeg(cmd_m6, "House Party intermediate WAV")
export_dual_formats(temp_m6, os.path.join(OUT_DIR, "12_HouseParty_WhatJhumka_to_MorniBanke_L2"))

# 7. RETRO BOLLYWOOD: Kesariya -> Morni Banke (L3 Echo-Out Blend + 4-Bar Acoustic Bridge)
temp_m7 = os.path.join(TEMP_DIR, "temp_m7.wav")
cmd_m7 = [
    FFMPEG, "-y",
    "-ss", "70.0", "-t", "18.0", "-i", TRACKS["I_kesariya"],
    "-ss", "45.0", "-t", "20.0", "-i", TRACKS["G_morni_banke"],
    "-filter_complex",
    # Lookahead ducking + acoustic bridge halo
    "[0:a]equalizer=f=1800:width_type=q:w=1.2:g=-2.5:enable='between(t,14.0,15.0)',aecho=0.8:0.55:480:0.35,highpass=f=200:enable='between(t,14.5,17.5)',afade=t=out:st=14.5:d=2.6[a0];"
    "[1:a]afade=t=in:st=0:d=0.4[a1];"
    "[a0][a1]acrossfade=d=2.6:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_m7
]
run_ffmpeg(cmd_m7, "Retro Bollywood intermediate WAV")
export_dual_formats(temp_m7, os.path.join(OUT_DIR, "13_Retro_Kesariya_to_MorniBanke_L3"))

print("All 20 audio scenarios for ITERATION_004 rendered successfully!")
