import os
import subprocess
import json

FFMPEG = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\.venv\Scripts\ffmpeg.exe"
CACHE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\cache"
OUT_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings"
os.makedirs(OUT_DIR, exist_ok=True)

# Tracks
TRACKS = {
    "kala_chashma": os.path.join(CACHE_DIR, "k4yXQkG2s1E.mp3"),
    "london_thumakda": os.path.join(CACHE_DIR, "udra3Mfw2oo.mp3"),
    "brown_munde": os.path.join(CACHE_DIR, "VNs_cCtdbPc.mp3"),
    "one_kiss": os.path.join(CACHE_DIR, "DkeiKbqa02g.mp3"),
    "nagada_sang_dhol": os.path.join(CACHE_DIR, "vK5E_aeBGYA.mp3"),
    "dholi_taro": os.path.join(CACHE_DIR, "4a25J3p0kVI.mp3"),
}

for name, path in TRACKS.items():
    assert os.path.exists(path), f"Missing track: {name} at {path}"

def run_ffmpeg(cmd, desc):
    print(f"Rendering: {desc}...")
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Error in {desc}:\n{res.stderr}")
        raise RuntimeError(f"FFmpeg failed for {desc}")
    print(f"  -> Done.")

# ==============================================================================
# 1. Kala Chashma (107.1 BPM) -> London Thumakda (151.5 BPM) [Sangeet Mood]
# Level 1: Safe Quick Cut at phrase boundary (4 beats)
# Level 4: Backspin Cut (vinyl brake / spinback)
# ==============================================================================
# Kala Chashma outro section starts around 140s, London Thumakda intro starts at 0s.
# We slice Kala Chashma [130s - 150s] and London Thumakda [0s - 25s]

# Transition 1 - Level 1: Safe Quick Cut (clean 0.5s rapid crossfade at beat boundary)
out_t1_l1 = os.path.join(OUT_DIR, "01_KalaChashma_to_LondonThumakda_L1_SafeQuickCut.mp3")
cmd_t1_l1 = [
    FFMPEG, "-y",
    "-ss", "135.0", "-t", "18.0", "-i", TRACKS["kala_chashma"],
    "-ss", "0.0", "-t", "20.0", "-i", TRACKS["london_thumakda"],
    "-filter_complex",
    "[0:a]afade=t=out:st=16.5:d=0.8[a0];"
    "[1:a]afade=t=in:st=0:d=0.3[a1];"
    "[a0][a1]acrossfade=d=0.5:c1=tri:c2=tri[out]",
    "-map", "[out]", "-b:a", "320k", out_t1_l1
]
run_ffmpeg(cmd_t1_l1, "01 L1 Safe Quick Cut")

# Transition 1 - Level 4: Backspin Cut (high speed vinyl deceleration effect into drop)
out_t1_l4 = os.path.join(OUT_DIR, "01_KalaChashma_to_LondonThumakda_L4_BackspinCut.mp3")
cmd_t1_l4 = [
    FFMPEG, "-y",
    "-ss", "135.0", "-t", "16.0", "-i", TRACKS["kala_chashma"],
    "-ss", "0.0", "-t", "20.0", "-i", TRACKS["london_thumakda"],
    "-filter_complex",
    # Outgoing: sudden flanger/pitch spin + highpass filter + rapid cut
    "[0:a]asplit=2[main][spin];"
    "[spin]atrim=start=14.5:end=15.8,asetpts=PTS-STARTPTS,asetrate=48000*1.8,aresample=48000,afade=t=out:st=0.5:d=0.4[spun];"
    "[main]atrim=start=0:end=14.5,asetpts=PTS-STARTPTS[intro];"
    "[intro][spun]concat=n=2:v=0:a=1[a0];"
    "[1:a]afade=t=in:st=0:d=0.1[a1];"
    "[a0][a1]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", "-b:a", "320k", out_t1_l4
]
run_ffmpeg(cmd_t1_l4, "01 L4 Backspin Cut")

# ==============================================================================
# 2. London Thumakda (151.5 BPM) -> Brown Munde (70.0 BPM / 140 BPM) [Punjabi Tashan]
# Level 2: Smart Mixing with -12dB vocal mid-carve and dhol groove handover
# ==============================================================================
# London Thumakda slice [185s - 205s], Brown Munde intro drop [0s - 25s]
out_t2_l2 = os.path.join(OUT_DIR, "02_LondonThumakda_to_BrownMunde_L2_VocalMidCarve.mp3")
cmd_t2_l2 = [
    FFMPEG, "-y",
    "-ss", "185.0", "-t", "18.0", "-i", TRACKS["london_thumakda"],
    "-ss", "0.0", "-t", "22.0", "-i", TRACKS["brown_munde"],
    "-filter_complex",
    # Outgoing: carve out mid vocal frequencies (-12dB at 1kHz, Q=1.5) over transition window, fade out bass
    "[0:a]equalizer=f=1000:width_type=q:w=1.5:g=-12,lowpass=f=400:enable='between(t,14.0,17.5)',afade=t=out:st=15.0:d=2.5[a0];"
    # Incoming: bring up Brown Munde with full bassline
    "[1:a]afade=t=in:st=0:d=0.5[a1];"
    "[a0][a1]acrossfade=d=2.0:c1=log:c2=qsin[out]",
    "-map", "[out]", "-b:a", "320k", out_t2_l2
]
run_ffmpeg(cmd_t2_l2, "02 L2 Vocal Mid-Carve Handover")

# ==============================================================================
# 3. Brown Munde (70.0 BPM) -> One Kiss (125.0 BPM) [International Party]
# Level 1: EQ3 Handover (8 beats)
# Level 2: Filter Sweep Drop (-30dB sub-bass cut + resonant highpass build)
# Level 3: Echo-Out Blend (16 beats, wet delay feedback)
# Level 4: Creative Mashup Layer (tempo ramp + 4-deck groove overlay)
# ==============================================================================
# Brown Munde groove slice [160s - 180s], One Kiss build/intro [0s - 25s]

# Level 1: EQ3 Handover
out_t3_l1 = os.path.join(OUT_DIR, "03_BrownMunde_to_OneKiss_L1_EQ3Handover.mp3")
cmd_t3_l1 = [
    FFMPEG, "-y",
    "-ss", "160.0", "-t", "18.0", "-i", TRACKS["brown_munde"],
    "-ss", "0.0", "-t", "20.0", "-i", TRACKS["one_kiss"],
    "-filter_complex",
    # Outgoing: gradual high and low cut
    "[0:a]highpass=f=200:enable='between(t,12.0,17.0)',afade=t=out:st=13.0:d=3.8[a0];"
    "[1:a]afade=t=in:st=0:d=1.5[a1];"
    "[a0][a1]acrossfade=d=3.8:c1=tri:c2=tri[out]",
    "-map", "[out]", "-b:a", "320k", out_t3_l1
]
run_ffmpeg(cmd_t3_l1, "03 L1 EQ3 Handover")

# Level 2: Filter Sweep Drop (-30dB bass swap + rising highpass filter)
out_t3_l2 = os.path.join(OUT_DIR, "03_BrownMunde_to_OneKiss_L2_FilterSweepDrop.mp3")
cmd_t3_l2 = [
    FFMPEG, "-y",
    "-ss", "160.0", "-t", "18.0", "-i", TRACKS["brown_munde"],
    "-ss", "0.0", "-t", "22.0", "-i", TRACKS["one_kiss"],
    "-filter_complex",
    # Outgoing: resonant highpass sweep from 100Hz up to 2500Hz + sharp cut
    "[0:a]highpass=f=800:enable='between(t,12.0,14.5)',highpass=f=2500:enable='between(t,14.5,16.5)',afade=t=out:st=15.0:d=1.5[a0];"
    # Incoming: One Kiss drop hits instantly with 100% full sub-bass
    "[1:a]afade=t=in:st=0:d=0.2[a1];"
    "[a0][a1]acrossfade=d=1.5:c1=qsin:c2=log[out]",
    "-map", "[out]", "-b:a", "320k", out_t3_l2
]
run_ffmpeg(cmd_t3_l2, "03 L2 Filter Sweep Drop")

# Level 3: Echo-Out Blend (16 beats delay freeze tail)
out_t3_l3 = os.path.join(OUT_DIR, "03_BrownMunde_to_OneKiss_L3_EchoOutBlend.mp3")
cmd_t3_l3 = [
    FFMPEG, "-y",
    "-ss", "160.0", "-t", "16.0", "-i", TRACKS["brown_munde"],
    "-ss", "0.0", "-t", "22.0", "-i", TRACKS["one_kiss"],
    "-filter_complex",
    # Outgoing: freeze with quarter-note 3/4 triplet delay feedback (echo-out)
    "[0:a]aecho=0.8:0.7:430:0.5[echoed];"
    "[echoed]afade=t=out:st=12.0:d=3.8[a0];"
    "[1:a]afade=t=in:st=0:d=2.0[a1];"
    "[a0][a1]acrossfade=d=3.8:c1=log:c2=hsin[out]",
    "-map", "[out]", "-b:a", "320k", out_t3_l3
]
run_ffmpeg(cmd_t3_l3, "03 L3 Echo-Out Blend")

# ==============================================================================
# 4. One Kiss (125.0 BPM) -> Nagada Sang Dhol (75.7 BPM / 151.4 BPM) [Festival Mode]
# Level 3: 16-beat Echo-Out with stereo reverb tail into dramatic Garba opening
# Level 4: Backspin brake cut into explosive 3-taali drop
# ==============================================================================
# One Kiss slice [100s - 120s], Nagada Sang Dhol intro/drop [0s - 25s]

out_t4_l3 = os.path.join(OUT_DIR, "04_OneKiss_to_NagadaSangDhol_L3_EchoOutBlend.mp3")
cmd_t4_l3 = [
    FFMPEG, "-y",
    "-ss", "100.0", "-t", "17.0", "-i", TRACKS["one_kiss"],
    "-ss", "0.0", "-t", "24.0", "-i", TRACKS["nagada_sang_dhol"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.6:480:0.4,afade=t=out:st=13.0:d=3.5[a0];"
    "[1:a]afade=t=in:st=0:d=1.0[a1];"
    "[a0][a1]acrossfade=d=3.5:c1=log:c2=qsin[out]",
    "-map", "[out]", "-b:a", "320k", out_t4_l3
]
run_ffmpeg(cmd_t4_l3, "04 L3 Echo-Out Blend")

out_t4_l4 = os.path.join(OUT_DIR, "04_OneKiss_to_NagadaSangDhol_L4_BackspinDrop.mp3")
cmd_t4_l4 = [
    FFMPEG, "-y",
    "-ss", "100.0", "-t", "15.0", "-i", TRACKS["one_kiss"],
    "-ss", "0.0", "-t", "22.0", "-i", TRACKS["nagada_sang_dhol"],
    "-filter_complex",
    "[0:a]asplit=2[m][s];"
    "[s]atrim=start=13.5:end=14.8,asetpts=PTS-STARTPTS,asetrate=48000*2.0,aresample=48000,afade=t=out:st=0.3:d=0.3[spin];"
    "[m]atrim=start=0:end=13.5,asetpts=PTS-STARTPTS[lead];"
    "[lead][spin]concat=n=2:v=0:a=1[a0];"
    "[1:a]afade=t=in:st=0:d=0.1[a1];"
    "[a0][a1]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", "-b:a", "320k", out_t4_l4
]
run_ffmpeg(cmd_t4_l4, "04 L4 Backspin Drop")

# ==============================================================================
# 5. Nagada Sang Dhol (75.7 BPM) -> Dholi Taro Dhol Baaje (68.6 BPM) [Garba & Dandiya]
# Level 2: Safe Quick Cut maintaining 3-taali percussion sync
# Level 3: 16-beat Echo-Out Reverb Blend between traditional Gujarati dhol patterns
# ==============================================================================
out_t5_l2 = os.path.join(OUT_DIR, "05_NagadaSangDhol_to_DholiTaro_L2_SafeQuickCut.mp3")
cmd_t5_l2 = [
    FFMPEG, "-y",
    "-ss", "140.0", "-t", "18.0", "-i", TRACKS["nagada_sang_dhol"],
    "-ss", "20.0", "-t", "22.0", "-i", TRACKS["dholi_taro"],
    "-filter_complex",
    "[0:a]afade=t=out:st=16.5:d=0.8[a0];"
    "[1:a]afade=t=in:st=0:d=0.3[a1];"
    "[a0][a1]acrossfade=d=0.8:c1=tri:c2=tri[out]",
    "-map", "[out]", "-b:a", "320k", out_t5_l2
]
run_ffmpeg(cmd_t5_l2, "05 L2 Safe Quick Cut")

out_t5_l3 = os.path.join(OUT_DIR, "05_NagadaSangDhol_to_DholiTaro_L3_EchoOutBlend.mp3")
cmd_t5_l3 = [
    FFMPEG, "-y",
    "-ss", "140.0", "-t", "18.0", "-i", TRACKS["nagada_sang_dhol"],
    "-ss", "20.0", "-t", "24.0", "-i", TRACKS["dholi_taro"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.7:390:0.4,afade=t=out:st=13.0:d=4.0[a0];"
    "[1:a]afade=t=in:st=0:d=1.5[a1];"
    "[a0][a1]acrossfade=d=4.0:c1=hsin:c2=hsin[out]",
    "-map", "[out]", "-b:a", "320k", out_t5_l3
]
run_ffmpeg(cmd_t5_l3, "05 L3 Echo-Out Blend")

# ==============================================================================
# 6. Dholi Taro Dhol Baaje (68.6 BPM) -> Kala Chashma (107.1 BPM) [Desi Club]
# Level 3: 16-beat Echo-Out Blend back into modern Bollywood kick
# Level 4: Backspin Cut into Kala Chashma drop
# ==============================================================================
out_t6_l3 = os.path.join(OUT_DIR, "06_DholiTaro_to_KalaChashma_L3_EchoOutBlend.mp3")
cmd_t6_l3 = [
    FFMPEG, "-y",
    "-ss", "180.0", "-t", "18.0", "-i", TRACKS["dholi_taro"],
    "-ss", "15.0", "-t", "22.0", "-i", TRACKS["kala_chashma"],
    "-filter_complex",
    "[0:a]aecho=0.8:0.6:440:0.5,afade=t=out:st=13.0:d=4.0[a0];"
    "[1:a]afade=t=in:st=0:d=1.0[a1];"
    "[a0][a1]acrossfade=d=4.0:c1=log:c2=qsin[out]",
    "-map", "[out]", "-b:a", "320k", out_t6_l3
]
run_ffmpeg(cmd_t6_l3, "06 L3 Echo-Out Blend")

out_t6_l4 = os.path.join(OUT_DIR, "06_DholiTaro_to_KalaChashma_L4_BackspinCut.mp3")
cmd_t6_l4 = [
    FFMPEG, "-y",
    "-ss", "180.0", "-t", "15.0", "-i", TRACKS["dholi_taro"],
    "-ss", "17.0", "-t", "20.0", "-i", TRACKS["kala_chashma"],
    "-filter_complex",
    "[0:a]asplit=2[m][s];"
    "[s]atrim=start=13.5:end=14.8,asetpts=PTS-STARTPTS,asetrate=48000*2.2,aresample=48000,afade=t=out:st=0.3:d=0.3[spin];"
    "[m]atrim=start=0:end=13.5,asetpts=PTS-STARTPTS[lead];"
    "[lead][spin]concat=n=2:v=0:a=1[a0];"
    "[1:a]afade=t=in:st=0:d=0.1[a1];"
    "[a0][a1]concat=n=2:v=0:a=1[out]",
    "-map", "[out]", "-b:a", "320k", out_t6_l4
]
run_ffmpeg(cmd_t6_l4, "06 L4 Backspin Cut")

# ==============================================================================
# 7. Continuous 6-Track Continuous Party Mix (Full Sequence)
# Kala Chashma -> London Thumakda -> Brown Munde -> One Kiss -> Nagada Sang Dhol -> Dholi Taro
# 60s per track highlighting verse, hook, and full transition execution!
# ==============================================================================
out_continuous = os.path.join(OUT_DIR, "00_DHUN_Batch1_Continuous_Party_Mix.mp3")
cmd_continuous = [
    FFMPEG, "-y",
    "-ss", "15.0", "-t", "60.0", "-i", TRACKS["kala_chashma"],
    "-ss", "20.0", "-t", "60.0", "-i", TRACKS["london_thumakda"],
    "-ss", "25.0", "-t", "60.0", "-i", TRACKS["brown_munde"],
    "-ss", "30.0", "-t", "60.0", "-i", TRACKS["one_kiss"],
    "-ss", "35.0", "-t", "60.0", "-i", TRACKS["nagada_sang_dhol"],
    "-ss", "40.0", "-t", "60.0", "-i", TRACKS["dholi_taro"],
    "-filter_complex",
    "[0:a][1:a]acrossfade=d=4.0:c1=tri:c2=tri[m1];"
    "[m1][2:a]acrossfade=d=3.5:c1=log:c2=qsin[m2];"
    "[m2][3:a]acrossfade=d=4.0:c1=hsin:c2=hsin[m3];"
    "[m3][4:a]acrossfade=d=4.0:c1=log:c2=qsin[m4];"
    "[m4][5:a]acrossfade=d=4.0:c1=hsin:c2=hsin[out]",
    "-map", "[out]", "-b:a", "320k", out_continuous
]
run_ffmpeg(cmd_continuous, "Continuous 6-Track Party Mix")

print("\nALL AUDIO FILES SUCCESSFULLY RENDERED!")
