import os
import subprocess
import re
import math

FFMPEG = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\.venv\Scripts\ffmpeg.exe"
CACHE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\cache"
TEMP_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\target\temp_test_continuous"
os.makedirs(TEMP_DIR, exist_ok=True)

TRACKS = {
    "A": os.path.join(CACHE_DIR, "k4yXQkG2s1E.mp3"),
    "B": os.path.join(CACHE_DIR, "udra3Mfw2oo.mp3"),
    "C": os.path.join(CACHE_DIR, "VNs_cCtdbPc.mp3"),
    "D": os.path.join(CACHE_DIR, "DkeiKbqa02g.mp3"),
    "E": os.path.join(CACHE_DIR, "vK5E_aeBGYA.mp3"),
    "F": os.path.join(CACHE_DIR, "4a25J3p0kVI.mp3"),
}

# 1. Render Segment 0: Kala Chashma (52.0s)
# Outgoing cut at 51.6s, fadeout 51.6 -> 52.0
seg0 = os.path.join(TEMP_DIR, "seg0.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "18.0", "-t", "52.0", "-i", TRACKS["A"],
    "-af", "equalizer=f=1800:width_type=q:w=1.2:g=-2.0:enable='between(t,50.8,51.6)',highpass=f=1200:enable='between(t,51.0,51.6)',afade=t=out:st=51.6:d=0.4",
    seg0
], check=True, capture_output=True)

# 2. Render Segment 1: London Thumakda (input 44.0s -> stretched to ~47.6s)
# Hook entrance at 38.1s.
seg1 = os.path.join(TEMP_DIR, "seg1.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "38.1", "-t", "44.0", "-i", TRACKS["B"],
    "-af", "volume=1.1,rubberband=tempo=0.924:pitch=1.0:formant=preserved,equalizer=f=1000:width_type=q:w=1.5:g=-12:enable='between(t,44.0,47.6)',highpass=f=160:enable='between(t,44.5,47.6)',afade=t=in:st=0:d=0.08,afade=t=out:st=45.6:d=2.0",
    seg1
], check=True, capture_output=True)

# 3. Render Segment 2: Brown Munde (48.0s)
# Entrance at 28.0s. Sub-bass shelf -4.5dB at 80Hz.
# Over 46.8s - 48.0s: Resonant filter sweep drop (Q=1.414)
seg2 = os.path.join(TEMP_DIR, "seg2.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "28.0", "-t", "48.0", "-i", TRACKS["C"],
    "-af", "equalizer=f=80:width_type=q:w=0.7:g=-4.5,highpass=f=800:width_type=q:w=0.707:enable='between(t,45.5,46.8)',highpass=f=2800:width_type=q:w=1.414:enable='between(t,46.8,47.95)',afade=t=in:st=0:d=0.08,afade=t=out:st=46.8:d=1.2",
    seg2
], check=True, capture_output=True)

# 4. Render Segment 3: One Kiss (50.0s)
# Entrance at 15.7s.
# Over 48.2s - 50.0s: 1.8s Echo-out blend with stereotools centering
seg3 = os.path.join(TEMP_DIR, "seg3.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "15.7", "-t", "50.0", "-i", TRACKS["D"],
    "-af", "aecho=0.8:0.5:480:0.35,stereotools=mpan=0:mlev=1.1,highpass=f=180:enable='between(t,48.2,50.0)',afade=t=in:st=0:d=0.08,afade=t=out:st=48.2:d=1.8",
    seg3
], check=True, capture_output=True)

# 5. Render Segment 4: Nagada Sang Dhol (48.0s)
# Entrance at 25.9s (alaap to drop).
# Over 47.4s - 48.0s: highpass at 220Hz, clean 0.6s cut
seg4 = os.path.join(TEMP_DIR, "seg4.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "25.9", "-t", "48.0", "-i", TRACKS["E"],
    "-af", "highpass=f=220:enable='between(t,47.4,48.0)',afade=t=in:st=0:d=0.4,afade=t=out:st=47.4:d=0.6",
    seg4
], check=True, capture_output=True)

# 6. Render Segment 5: Dholi Taro (50.0s)
# Entrance at 28.65s (fast dandiya drop).
seg5 = os.path.join(TEMP_DIR, "seg5.wav")
subprocess.run([
    FFMPEG, "-y",
    "-ss", "28.65", "-t", "50.0", "-i", TRACKS["F"],
    "-af", "volume=1.12,afade=t=in:st=0:d=0.08,afade=t=out:st=46.0:d=4.0",
    seg5
], check=True, capture_output=True)

def get_duration(path):
    cmd = [FFMPEG, "-nostats", "-i", path, "-f", "null", "-"]
    res = subprocess.run(cmd, capture_output=True, text=True)
    m = re.search(r"Duration:\s+(\d+):(\d+):([\d\.]+)", res.stderr)
    if m:
        return int(m.group(1))*3600 + int(m.group(2))*60 + float(m.group(3))
    return 0.0

d0 = get_duration(seg0)
d1 = get_duration(seg1)
d2 = get_duration(seg2)
d3 = get_duration(seg3)
d4 = get_duration(seg4)
d5 = get_duration(seg5)
print(f"Segment durations: seg0={d0:.2f}s, seg1={d1:.2f}s, seg2={d2:.2f}s, seg3={d3:.2f}s, seg4={d4:.2f}s, seg5={d5:.2f}s")

# Exact Downbeat Timestamps:
# Cut 0->1: downbeat drops at 51.6s
t0_ms = 0
t1_ms = int(51.6 * 1000)

# Cut 1->2: Brown Munde drops at 51.6s + 45.6s (2s crossfade window from 45.6s to 47.6s)
t2_ms = t1_ms + int(45.6 * 1000)

# Cut 2->3: One Kiss drops at t2 + 46.8s (1.2s crossfade window from 46.8s to 48.0s)
t3_ms = t2_ms + int(46.8 * 1000)

# Cut 3->4: Nagada Sang Dhol drops at t3 + 48.2s (1.8s echo-out dissolution)
t4_ms = t3_ms + int(48.2 * 1000)

# Cut 4->5: Dholi Taro drops at t4 + 47.4s (0.6s clean dandiya cut)
t5_ms = t4_ms + int(47.4 * 1000)

print(f"Timeline cue points:")
print(f"  Track 0 (Kala Chashma):      {t0_ms/1000:.2f}s")
print(f"  Track 1 (London Thumakda):   {t1_ms/1000:.2f}s")
print(f"  Track 2 (Brown Munde):       {t2_ms/1000:.2f}s")
print(f"  Track 3 (One Kiss):          {t3_ms/1000:.2f}s")
print(f"  Track 4 (Nagada Sang Dhol):  {t4_ms/1000:.2f}s")
print(f"  Track 5 (Dholi Taro):        {t5_ms/1000:.2f}s")

mix_wav = os.path.join(TEMP_DIR, "test_continuous_aligned.wav")
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
    f"[raw_mix]compand=attacks=0.05:decays=0.2:points=-80/-80|-24/-24|-12/-10|0/-1:gain=0.5,"
    f"loudnorm=I=-14.0:TP=-1.0:LRA=10.0,"
    f"alimiter=limit=-1.0dB:attack=5:release=50:asc=1[out]",
    "-map", "[out]", mix_wav
]
subprocess.run(cmd_mix, check=True)
print("Continuous mix generated successfully with exact timeline alignment!")

# Silence detection check across the ENTIRE mix
cmd_silence = [
    FFMPEG, "-nostats",
    "-i", mix_wav,
    "-af", "silencedetect=noise=-30dB:d=0.3",
    "-f", "null", "-"
]
res_sil = subprocess.run(cmd_silence, capture_output=True, text=True)
silence_events = re.findall(r"silence_start:\s+([\d\.]+)", res_sil.stderr)
print(f"Silence events detected (>0.3s at -30dB): {len(silence_events)}")
for s in silence_events:
    print(f"  Silence at {s}s")
