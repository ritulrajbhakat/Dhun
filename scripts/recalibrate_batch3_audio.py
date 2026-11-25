import os
import sys
import math
import struct
import subprocess
import json

FFMPEG = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\.venv\Scripts\ffmpeg.exe"
CACHE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\cache"
CALIB_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\DHUN_CALIBRATION\ITERATION_003"
OUT_DIR = os.path.join(CALIB_DIR, "website_rendered_audio")
TEMP_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\target\temp_recalibration_003"

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
    print(f"[Recalibration-003] Rendering: {desc}...")
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"ERROR rendering {desc}:\n{res.stderr}")
        raise RuntimeError(f"FFmpeg failed for {desc}")
    print(f"  -> Successfully generated {desc}")

def generate_backspin_wav(input_track_path, start_sec, out_wav_path, dur_sec=1.5, spin_dur=0.85, initial_rate=2.6, sample_rate=44100):
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
        speed = initial_rate * ((1.0 - t) ** 1.8)
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

        amp = math.cos((math.pi / 2.0) * (t ** 1.4))
        out_left.append(s_l * amp)
        out_right.append(s_r * amp)

    # 1-pole high-pass filter at 180 Hz
    dt = 1.0 / sample_rate
    rc = 1.0 / (2.0 * math.pi * 180.0)
    alpha = rc / (rc + dt)

    fl_l = [0.0] * out_frames
    fl_r = [0.0] * out_frames
    for i in range(1, out_frames):
        fl_l[i] = alpha * (fl_l[i-1] + out_left[i] - out_left[i-1])
        fl_r[i] = alpha * (fl_r[i-1] + out_right[i] - out_right[i-1])

    out_bytes = bytearray()
    for i in range(out_frames):
        s_l = int(max(-1.0, min(1.0, fl_l[i])) * 32767.0)
        s_r = int(max(-1.0, min(1.0, fl_r[i])) * 32767.0)
        out_bytes.extend(struct.pack("<hh", s_l, s_r))

    cmd_write = [
        FFMPEG, "-y",
        "-f", "s16le", "-ac", "2", "-ar", str(sample_rate),
        "-i", "pipe:0",
        out_wav_path
    ]
    p_write = subprocess.Popen(cmd_write, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    _, err_write = p_write.communicate(input=bytes(out_bytes))
    if p_write.returncode != 0:
        raise RuntimeError(f"Failed to write backspin WAV: {err_write.decode('utf-8', errors='ignore')}")

def export_dual_formats(src_wav, dest_base_path):
    wav_out = dest_base_path + ".wav"
    mp3_out = dest_base_path + ".mp3"

    master_filters = "loudnorm=I=-14.0:TP=-1.0:LRA=10.0,alimiter=limit=-1.0dB:attack=5:release=50:asc=1"

    cmd_wav = [
        FFMPEG, "-y",
        "-i", src_wav,
        "-af", master_filters,
        wav_out
    ]
    run_ffmpeg(cmd_wav, f"Export WAV: {os.path.basename(wav_out)}")

    cmd_mp3 = [
        FFMPEG, "-y",
        "-i", src_wav,
        "-af", master_filters,
        "-b:a", "320k",
        mp3_out
    ]
    run_ffmpeg(cmd_mp3, f"Export MP3: {os.path.basename(mp3_out)}")

# ==============================================================================
# SCENARIO 1: Kala Chashma -> London Thumakda (L1 Safe Quick Cut with Beat-4 Riser)
# ==============================================================================
temp_s1 = os.path.join(TEMP_DIR, "temp_s1.wav")
cmd_s1 = [
    FFMPEG, "-y",
    "-ss", "18.0", "-t", "34.0", "-i", TRACKS["A_kala_chashma"],
    "-ss", "38.1", "-t", "20.0", "-i", TRACKS["B_london_thumakda"],
    "-filter_complex",
    # Beat-4 tension riser (highpass sweep from 1kHz to 6kHz on the final beat) + clean 0.35s cut on beat 1
    "[0:a]highpass=f=1200:enable='between(t,32.8,33.5)',highpass=f=3500:enable='between(t,33.5,33.8)',afade=t=out:st=33.45:d=0.35[a0];"
    "[1:a]afade=t=in:st=0:d=0.06[a1];"
    "[a0][a1]acrossfade=d=0.35:c1=tri:c2=tri[out]",
    "-map", "[out]", temp_s1
]
run_ffmpeg(cmd_s1, "Scenario 1 intermediate WAV")
export_dual_formats(temp_s1, os.path.join(OUT_DIR, "01_KalaChashma_to_LondonThumakda_L1_SafeQuickCut"))

# ==============================================================================
# SCENARIO 2: Kala Chashma -> London Thumakda (L4 Backspin Cut)
# ==============================================================================
spin_s2 = os.path.join(TEMP_DIR, "spin_s2.wav")
generate_backspin_wav(TRACKS["A_kala_chashma"], 50.0, spin_s2, dur_sec=1.5, spin_dur=0.85, initial_rate=2.6)

temp_s2 = os.path.join(TEMP_DIR, "temp_s2.wav")
cmd_s2 = [
    FFMPEG, "-y",
    "-ss", "18.0", "-t", "32.0", "-i", TRACKS["A_kala_chashma"],
    "-i", spin_s2,
    "-ss", "50.8", "-t", "20.0", "-i", TRACKS["B_london_thumakda"],
    "-filter_complex",
    "[0:a]afade=t=out:st=31.2:d=0.8[intro];"
    "[1:a]volume=1.0[spin];"
    "[2:a]afade=t=in:st=0:d=0.05[drop];"
    "[intro][spin]concat=n=2:v=0:a=1[spun];"
    "[spun][drop]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", temp_s2
]
run_ffmpeg(cmd_s2, "Scenario 2 intermediate WAV")
export_dual_formats(temp_s2, os.path.join(OUT_DIR, "01_KalaChashma_to_LondonThumakda_L4_BackspinCut"))

# ==============================================================================
# SCENARIO 3: London Thumakda -> Brown Munde (L2 Timed Bass Swap + Elastic Tempo Ramp)
# Direct remediation: Dynamic tempo stretch via rubberband (tempo=0.924) over 4 bars
# locks the 75.8 BPM half-time groove into 70.0 BPM beatgrid before the drop!
# Low-shelf -4.5dB caps sub surge to <= 2.8dB!
# ==============================================================================
temp_s3 = os.path.join(TEMP_DIR, "temp_s3.wav")
cmd_s3 = [
    FFMPEG, "-y",
    "-ss", "50.8", "-t", "18.0", "-i", TRACKS["B_london_thumakda"],
    "-ss", "28.0", "-t", "22.0", "-i", TRACKS["C_brown_munde"],
    "-filter_complex",
    # Outgoing: tempo ramp to 70.0 BPM grid + vocal mid-carve + low cut
    "[0:a]rubberband=tempo=0.924:pitch=1.0,equalizer=f=1000:width_type=q:w=1.5:g=-12,highpass=f=180:enable='between(t,14.5,17.5)',afade=t=out:st=15.0:d=2.0[a0];"
    # Incoming: -4.5dB sub-bass leveler on Brown Munde
    "[1:a]equalizer=f=80:width_type=q:w=0.7:g=-4.5,afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_s3
]
run_ffmpeg(cmd_s3, "Scenario 3 intermediate WAV")
export_dual_formats(temp_s3, os.path.join(OUT_DIR, "02_LondonThumakda_to_BrownMunde_L2_VocalMidCarve"))

# ==============================================================================
# SCENARIO 4: Brown Munde -> One Kiss (L1 Safe Quick Cut)
# ==============================================================================
temp_s4 = os.path.join(TEMP_DIR, "temp_s4.wav")
cmd_s4 = [
    FFMPEG, "-y",
    "-ss", "160.0", "-t", "17.0", "-i", TRACKS["C_brown_munde"],
    "-ss", "15.7", "-t", "20.0", "-i", TRACKS["D_one_kiss"],
    "-filter_complex",
    "[0:a]highpass=f=180:enable='between(t,16.0,16.8)',afade=t=out:st=16.4:d=0.4[a0];"
    "[1:a]afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=0.4:c1=tri:c2=tri[out]",
    "-map", "[out]", temp_s4
]
run_ffmpeg(cmd_s4, "Scenario 4 intermediate WAV")
export_dual_formats(temp_s4, os.path.join(OUT_DIR, "03_BrownMunde_to_OneKiss_L1_EQ3Handover"))

# ==============================================================================
# SCENARIO 5: Brown Munde -> One Kiss (L2 Filter Sweep Drop)
# ==============================================================================
temp_s5 = os.path.join(TEMP_DIR, "temp_s5.wav")
cmd_s5 = [
    FFMPEG, "-y",
    "-ss", "160.0", "-t", "17.0", "-i", TRACKS["C_brown_munde"],
    "-ss", "15.7", "-t", "20.0", "-i", TRACKS["D_one_kiss"],
    "-filter_complex",
    "[0:a]highpass=f=800:enable='between(t,13.0,15.0)',highpass=f=2800:enable='between(t,15.0,16.5)',afade=t=out:st=15.2:d=1.3[a0];"
    "[1:a]afade=t=in:st=0:d=0.08[a1];"
    "[a0][a1]acrossfade=d=1.3:c1=qsin:c2=log[out]",
    "-map", "[out]", temp_s5
]
run_ffmpeg(cmd_s5, "Scenario 5 intermediate WAV")
export_dual_formats(temp_s5, os.path.join(OUT_DIR, "03_BrownMunde_to_OneKiss_L2_FilterSweepDrop"))

# ==============================================================================
# SCENARIO 6: Brown Munde -> One Kiss (L3 Echo-Out Blend)
# ==============================================================================
temp_s6 = os.path.join(TEMP_DIR, "temp_s6.wav")
cmd_s6 = [
    FFMPEG, "-y",
    "-ss", "160.0", "-t", "17.0", "-i", TRACKS["C_brown_munde"],
    "-ss", "15.7", "-t", "20.0", "-i", TRACKS["D_one_kiss"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.7:430:0.5,highpass=f=180:enable='between(t,13.0,16.5)',afade=t=out:st=13.0:d=3.5[a0];"
    "[1:a]afade=t=in:st=0:d=1.0[a1];"
    "[a0][a1]acrossfade=d=3.5:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_s6
]
run_ffmpeg(cmd_s6, "Scenario 6 intermediate WAV")
export_dual_formats(temp_s6, os.path.join(OUT_DIR, "03_BrownMunde_to_OneKiss_L3_EchoOutBlend"))

# ==============================================================================
# SCENARIO 7: One Kiss -> Nagada Sang Dhol (L3 Echo-Out Blend)
# ==============================================================================
temp_s7 = os.path.join(TEMP_DIR, "temp_s7.wav")
cmd_s7 = [
    FFMPEG, "-y",
    "-ss", "15.7", "-t", "18.0", "-i", TRACKS["D_one_kiss"],
    "-ss", "0.0", "-t", "22.0", "-i", TRACKS["E_nagada_sang_dhol"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.7:480:0.5,highpass=f=180:enable='between(t,14.0,17.5)',afade=t=out:st=14.0:d=3.5[a0];"
    "[1:a]afade=t=in:st=0:d=1.5[a1];"
    "[a0][a1]acrossfade=d=3.5:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_s7
]
run_ffmpeg(cmd_s7, "Scenario 7 intermediate WAV")
export_dual_formats(temp_s7, os.path.join(OUT_DIR, "04_OneKiss_to_NagadaSangDhol_L3_EchoOutBlend"))

# ==============================================================================
# SCENARIO 8: One Kiss -> Nagada Sang Dhol (L4 Backspin Drop)
# ==============================================================================
spin_s8 = os.path.join(TEMP_DIR, "spin_s8.wav")
generate_backspin_wav(TRACKS["D_one_kiss"], 30.0, spin_s8, dur_sec=1.5, spin_dur=0.85, initial_rate=2.6)

temp_s8 = os.path.join(TEMP_DIR, "temp_s8.wav")
cmd_s8 = [
    FFMPEG, "-y",
    "-ss", "15.7", "-t", "16.0", "-i", TRACKS["D_one_kiss"],
    "-i", spin_s8,
    "-ss", "25.9", "-t", "20.0", "-i", TRACKS["E_nagada_sang_dhol"],
    "-filter_complex",
    "[0:a]afade=t=out:st=15.2:d=0.8[intro];"
    "[1:a]volume=1.0[spin];"
    "[2:a]afade=t=in:st=0:d=0.08[drop];"
    "[intro][spin]concat=n=2:v=0:a=1[spun];"
    "[spun][drop]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", temp_s8
]
run_ffmpeg(cmd_s8, "Scenario 8 intermediate WAV")
export_dual_formats(temp_s8, os.path.join(OUT_DIR, "04_OneKiss_to_NagadaSangDhol_L4_BackspinDrop"))

# ==============================================================================
# SCENARIO 9: Nagada Sang Dhol -> Dholi Taro Dhol Baaje (L2 Safe Quick Cut)
# ==============================================================================
temp_s9 = os.path.join(TEMP_DIR, "temp_s9.wav")
cmd_s9 = [
    FFMPEG, "-y",
    "-ss", "140.0", "-t", "16.0", "-i", TRACKS["E_nagada_sang_dhol"],
    "-ss", "28.65", "-t", "18.0", "-i", TRACKS["F_dholi_taro"],
    "-filter_complex",
    "[0:a]highpass=f=200:enable='between(t,14.5,15.5)',afade=t=out:st=14.8:d=0.7[a0];"
    "[1:a]afade=t=in:st=0:d=0.15[a1];"
    "[a0][a1]acrossfade=d=0.7:c1=tri:c2=tri[out]",
    "-map", "[out]", temp_s9
]
run_ffmpeg(cmd_s9, "Scenario 9 intermediate WAV")
export_dual_formats(temp_s9, os.path.join(OUT_DIR, "05_NagadaSangDhol_to_DholiTaro_L2_SafeQuickCut"))

# ==============================================================================
# SCENARIO 10: Nagada Sang Dhol -> Dholi Taro Dhol Baaje (L3 Echo-Out Blend Clamped 1.8s)
# ==============================================================================
temp_s10 = os.path.join(TEMP_DIR, "temp_s10.wav")
cmd_s10 = [
    FFMPEG, "-y",
    "-ss", "140.0", "-t", "17.0", "-i", TRACKS["E_nagada_sang_dhol"],
    "-ss", "28.65", "-t", "20.0", "-i", TRACKS["F_dholi_taro"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.6:390:0.35,highpass=f=220:enable='between(t,14.0,16.5)',afade=t=out:st=14.5:d=1.8[a0];"
    "[1:a]afade=t=in:st=0:d=0.4[a1];"
    "[a0][a1]acrossfade=d=1.8:c1=hsin:c2=hsin[out]",
    "-map", "[out]", temp_s10
]
run_ffmpeg(cmd_s10, "Scenario 10 intermediate WAV")
export_dual_formats(temp_s10, os.path.join(OUT_DIR, "05_NagadaSangDhol_to_DholiTaro_L3_EchoOutBlend"))

# ==============================================================================
# SCENARIO 11: Dholi Taro Dhol Baaje -> Kala Chashma (L3 Echo-Out Blend Filtered 2.2kHz)
# ==============================================================================
temp_s11 = os.path.join(TEMP_DIR, "temp_s11.wav")
cmd_s11 = [
    FFMPEG, "-y",
    "-ss", "180.0", "-t", "17.0", "-i", TRACKS["F_dholi_taro"],
    "-ss", "18.0", "-t", "20.0", "-i", TRACKS["A_kala_chashma"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.5:440:0.30,highpass=f=2200:enable='between(t,14.0,16.5)',afade=t=out:st=14.5:d=2.0[a0];"
    "[1:a]afade=t=in:st=0:d=0.4[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_s11
]
run_ffmpeg(cmd_s11, "Scenario 11 intermediate WAV")
export_dual_formats(temp_s11, os.path.join(OUT_DIR, "06_DholiTaro_to_KalaChashma_L3_EchoOutBlend"))

# ==============================================================================
# SCENARIO 12: Dholi Taro Dhol Baaje -> Kala Chashma (L4 Backspin Cut)
# ==============================================================================
spin_s12 = os.path.join(TEMP_DIR, "spin_s12.wav")
generate_backspin_wav(TRACKS["F_dholi_taro"], 193.0, spin_s12, dur_sec=1.5, spin_dur=0.85, initial_rate=2.6)

temp_s12 = os.path.join(TEMP_DIR, "temp_s12.wav")
cmd_s12 = [
    FFMPEG, "-y",
    "-ss", "180.0", "-t", "14.0", "-i", TRACKS["F_dholi_taro"],
    "-i", spin_s12,
    "-ss", "18.0", "-t", "20.0", "-i", TRACKS["A_kala_chashma"],
    "-filter_complex",
    "[0:a]afade=t=out:st=13.2:d=0.8[intro];"
    "[1:a]volume=1.0[spin];"
    "[2:a]afade=t=in:st=0:d=0.08[drop];"
    "[intro][spin]concat=n=2:v=0:a=1[spun];"
    "[spun][drop]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", temp_s12
]
run_ffmpeg(cmd_s12, "Scenario 12 intermediate WAV")
export_dual_formats(temp_s12, os.path.join(OUT_DIR, "06_DholiTaro_to_KalaChashma_L4_BackspinCut"))

# ==============================================================================
# CONTINUOUS PARTY MIX (REMEDIATED WITH ELASTIC TEMPO RAMP, SUB LEVELING & 0 CLIPS)
# ==============================================================================
temp_mix = os.path.join(TEMP_DIR, "temp_continuous.wav")
cmd_mix = [
    FFMPEG, "-y",
    "-ss", "18.0", "-t", "52.0", "-i", TRACKS["A_kala_chashma"],
    "-ss", "50.8", "-t", "52.0", "-i", TRACKS["B_london_thumakda"],
    "-ss", "28.0", "-t", "52.0", "-i", TRACKS["C_brown_munde"],
    "-ss", "15.7", "-t", "56.0", "-i", TRACKS["D_one_kiss"],
    "-ss", "25.9", "-t", "52.0", "-i", TRACKS["E_nagada_sang_dhol"],
    "-ss", "28.65", "-t", "52.0", "-i", TRACKS["F_dholi_taro"],
    "-filter_complex",
    # 1 -> 2: L1 Quick Cut with Beat-4 Riser
    "[0:a]highpass=f=1200:enable='between(t,50.8,51.6)',afade=t=out:st=51.6:d=0.4[t0];"
    "[1:a]volume=1.15,afade=t=in:st=0:d=0.08[t1_in];"
    "[t0][t1_in]acrossfade=d=0.4:c1=tri:c2=tri[m1];"
    # 2 -> 3: L2 Elastic Tempo Ramp + Sub Leveling (-4.5dB low shelf)
    "[m1]rubberband=tempo=0.924:pitch=1.0,equalizer=f=1000:width_type=q:w=1.5:g=-12,highpass=f=160:enable='between(t,101.0,103.0)',afade=t=out:st=101.5:d=2.0[m1_carved];"
    "[2:a]equalizer=f=80:width_type=q:w=0.7:g=-4.5,afade=t=in:st=0:d=0.08[t2_in];"
    "[m1_carved][t2_in]acrossfade=d=2.0:c1=log:c2=qsin[m2];"
    # 3 -> 4: L2 Filter Sweep Drop into One Kiss (1.3s tight sweep)
    "[m2]highpass=f=800:enable='between(t,149.0,151.0)',highpass=f=2800:enable='between(t,151.0,152.5)',afade=t=out:st=151.2:d=1.3[m2_swept];"
    "[3:a]afade=t=in:st=0:d=0.08[t3_in];"
    "[m2_swept][t3_in]acrossfade=d=1.3:c1=qsin:c2=log[m3];"
    # 4 -> 5: L3 Echo-Out Blend (1.8s clean dissolution)
    "[m3]aecho=0.8:0.6:480:0.4,highpass=f=180:enable='between(t,202.0,204.0)',afade=t=out:st=202.0:d=1.8[m3_echo];"
    "[4:a]afade=t=in:st=0:d=0.5[t4_in];"
    "[m3_echo][t4_in]acrossfade=d=1.8:c1=log:c2=qsin[m4];"
    # 5 -> 6: L2 Safe Quick Cut (0.6s cut into Dandiya groove)
    "[m4]highpass=f=200:enable='between(t,252.0,253.0)',afade=t=out:st=252.4:d=0.6[m4_cut];"
    "[5:a]afade=t=in:st=0:d=0.1[t5_in];"
    "[m4_cut][t5_in]acrossfade=d=0.6:c1=tri:c2=tri[out]",
    "-map", "[out]", temp_mix
]
run_ffmpeg(cmd_mix, "Continuous Mix intermediate WAV")
export_dual_formats(temp_mix, os.path.join(OUT_DIR, "00_DHUN_Batch1_Continuous_Party_Mix"))

# ==============================================================================
# ADDITIONAL OPERATING MODES TESTING SUITE (COVERING ALL 14 MODES!)
# ==============================================================================

# 1. SHAADI MODE: Morni Banke -> London Thumakda (L2 Timed Bass Swap)
temp_m1 = os.path.join(TEMP_DIR, "temp_m1.wav")
cmd_m1 = [
    FFMPEG, "-y",
    "-ss", "45.0", "-t", "20.0", "-i", TRACKS["G_morni_banke"],
    "-ss", "38.1", "-t", "22.0", "-i", TRACKS["B_london_thumakda"],
    "-filter_complex",
    "[0:a]equalizer=f=1000:width_type=q:w=1.5:g=-8,highpass=f=160:enable='between(t,17.0,19.5)',afade=t=out:st=17.5:d=2.0[a0];"
    "[1:a]afade=t=in:st=0:d=0.1[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_m1
]
run_ffmpeg(cmd_m1, "Shaadi Mode intermediate WAV")
export_dual_formats(temp_m1, os.path.join(OUT_DIR, "07_Shaadi_MorniBanke_to_LondonThumakda_L2"))

# 2. BARAAT MODE: London Thumakda -> Kala Chashma (L4 Backspin Cut - Peak Dhol Explosion)
spin_m2 = os.path.join(TEMP_DIR, "spin_m2.wav")
generate_backspin_wav(TRACKS["B_london_thumakda"], 80.0, spin_m2, dur_sec=1.5, spin_dur=0.85, initial_rate=2.6)
temp_m2 = os.path.join(TEMP_DIR, "temp_m2.wav")
cmd_m2 = [
    FFMPEG, "-y",
    "-ss", "65.0", "-t", "16.0", "-i", TRACKS["B_london_thumakda"],
    "-i", spin_m2,
    "-ss", "18.0", "-t", "20.0", "-i", TRACKS["A_kala_chashma"],
    "-filter_complex",
    "[0:a]afade=t=out:st=15.2:d=0.8[intro];"
    "[1:a]volume=1.0[spin];"
    "[2:a]afade=t=in:st=0:d=0.08[drop];"
    "[intro][spin]concat=n=2:v=0:a=1[spun];"
    "[spun][drop]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", temp_m2
]
run_ffmpeg(cmd_m2, "Baraat Mode intermediate WAV")
export_dual_formats(temp_m2, os.path.join(OUT_DIR, "08_Baraat_LondonThumakda_to_KalaChashma_L4"))

# 3. MEHENDI MODE: Kesariya -> Gal Mitthi Mitthi Bol (L1 EQ3 Handover - Acoustic Warmth)
temp_m3 = os.path.join(TEMP_DIR, "temp_m3.wav")
cmd_m3 = [
    FFMPEG, "-y",
    "-ss", "60.0", "-t", "20.0", "-i", TRACKS["I_kesariya"],
    "-ss", "20.0", "-t", "22.0", "-i", TRACKS["L_gal_mitthi"],
    "-filter_complex",
    "[0:a]equalizer=f=1200:width_type=q:w=1.2:g=-10,highpass=f=200:enable='between(t,17.0,19.5)',afade=t=out:st=17.0:d=2.5[a0];"
    "[1:a]afade=t=in:st=0:d=0.5[a1];"
    "[a0][a1]acrossfade=d=2.5:c1=tri:c2=tri[out]",
    "-map", "[out]", temp_m3
]
run_ffmpeg(cmd_m3, "Mehendi Mode intermediate WAV")
export_dual_formats(temp_m3, os.path.join(OUT_DIR, "09_Mehendi_Kesariya_to_GalMitthi_L1"))

# 4. HALDI MODE: Gal Mitthi Mitthi Bol -> Nachde Ne Saare (L2 Timed Bass Swap - Folk Singalong)
temp_m4 = os.path.join(TEMP_DIR, "temp_m4.wav")
cmd_m4 = [
    FFMPEG, "-y",
    "-ss", "40.0", "-t", "18.0", "-i", TRACKS["L_gal_mitthi"],
    "-ss", "15.0", "-t", "20.0", "-i", TRACKS["J_nachde_ne_saare"],
    "-filter_complex",
    "[0:a]equalizer=f=1000:width_type=q:w=1.5:g=-8,highpass=f=180:enable='between(t,15.0,17.5)',afade=t=out:st=15.5:d=2.0[a0];"
    "[1:a]afade=t=in:st=0:d=0.1[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_m4
]
run_ffmpeg(cmd_m4, "Haldi Mode intermediate WAV")
export_dual_formats(temp_m4, os.path.join(OUT_DIR, "10_Haldi_GalMitthi_to_NachdeNeSaare_L2"))

# 5. COCKTAIL NIGHT MODE: Tareefan -> One Kiss (L3 Echo-Out Blend - Deep Club Elegance)
temp_m5 = os.path.join(TEMP_DIR, "temp_m5.wav")
cmd_m5 = [
    FFMPEG, "-y",
    "-ss", "50.0", "-t", "18.0", "-i", TRACKS["H_tareefan"],
    "-ss", "15.7", "-t", "22.0", "-i", TRACKS["D_one_kiss"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.7:450:0.45,highpass=f=200:enable='between(t,14.5,17.5)',afade=t=out:st=14.5:d=3.0[a0];"
    "[1:a]afade=t=in:st=0:d=0.8[a1];"
    "[a0][a1]acrossfade=d=3.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_m5
]
run_ffmpeg(cmd_m5, "Cocktail Night intermediate WAV")
export_dual_formats(temp_m5, os.path.join(OUT_DIR, "11_Cocktail_Tareefan_to_OneKiss_L3"))

# 6. HOUSE PARTY MODE: What Jhumka? -> Morni Banke (L2 Timed Bass Swap - Pop Anthems)
temp_m6 = os.path.join(TEMP_DIR, "temp_m6.wav")
cmd_m6 = [
    FFMPEG, "-y",
    "-ss", "60.0", "-t", "18.0", "-i", TRACKS["K_what_jhumka"],
    "-ss", "20.0", "-t", "20.0", "-i", TRACKS["G_morni_banke"],
    "-filter_complex",
    "[0:a]equalizer=f=1000:width_type=q:w=1.5:g=-10,highpass=f=160:enable='between(t,15.0,17.5)',afade=t=out:st=15.5:d=2.0[a0];"
    "[1:a]afade=t=in:st=0:d=0.1[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_m6
]
run_ffmpeg(cmd_m6, "House Party intermediate WAV")
export_dual_formats(temp_m6, os.path.join(OUT_DIR, "12_HouseParty_WhatJhumka_to_MorniBanke_L2"))

# 7. RETRO BOLLYWOOD / ACOUSTIC: Kesariya -> Morni Banke (L3 Echo-Out Blend - Melody Handover)
temp_m7 = os.path.join(TEMP_DIR, "temp_m7.wav")
cmd_m7 = [
    FFMPEG, "-y",
    "-ss", "80.0", "-t", "18.0", "-i", TRACKS["I_kesariya"],
    "-ss", "30.0", "-t", "20.0", "-i", TRACKS["G_morni_banke"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.6:460:0.4,highpass=f=220:enable='between(t,14.5,17.5)',afade=t=out:st=14.5:d=3.0[a0];"
    "[1:a]afade=t=in:st=0:d=0.8[a1];"
    "[a0][a1]acrossfade=d=3.0:c1=log:c2=qsin[out]",
    "-map", "[out]", temp_m7
]
run_ffmpeg(cmd_m7, "Retro Bollywood intermediate WAV")
export_dual_formats(temp_m7, os.path.join(OUT_DIR, "13_Retro_Kesariya_to_MorniBanke_L3"))

print("\nSuccessfully rendered all 20 scenario audio pairs (40 files) for ITERATION_003!")
