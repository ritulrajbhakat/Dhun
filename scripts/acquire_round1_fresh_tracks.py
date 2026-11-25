import os
import sys
import json
import yt_dlp

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
BATCHES_DIR = os.path.join(CALIBRATION_DIR, "fresh_batches")
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
VENV_BIN = os.path.join(BASE_DIR, "resolver-service", ".venv", "Scripts")

os.makedirs(BATCHES_DIR, exist_ok=True)
os.makedirs(CACHE_DIR, exist_ok=True)

if VENV_BIN not in os.environ["PATH"]:
    os.environ["PATH"] = VENV_BIN + os.pathsep + os.environ["PATH"]

ROUND_1_TRACKS = [
    {
        "id": "badtameez_dil",
        "num": 1,
        "title": "Badtameez Dil",
        "artist": "Pritam, Benny Dayal, Shefali Alvares",
        "search_query": "Badtameez Dil Yeh Jawaani Hai Deewani official song",
        "expected_duration": 252,
        "genre": "Bollywood Swing / Dance-Pop",
        "bpm": 152.0,
        "camelot": "11A",
        "meter": "4/4"
    },
    {
        "id": "mundian_to_bach_ke",
        "num": 2,
        "title": "Mundian To Bach Ke",
        "artist": "Panjabi MC ft. Labh Janjua",
        "search_query": "Panjabi MC Mundian To Bach Ke official audio",
        "expected_duration": 244,
        "genre": "UK Desi / Punjabi Bhangra",
        "bpm": 98.0,
        "camelot": "1A",
        "meter": "4/4"
    },
    {
        "id": "fein",
        "num": 3,
        "title": "FE!N",
        "artist": "Travis Scott ft. Playboi Carti",
        "search_query": "Travis Scott FE!N official audio",
        "expected_duration": 191,
        "genre": "Trap / Rage Hip-Hop",
        "bpm": 148.0,
        "camelot": "2B",
        "meter": "4/4"
    },
    {
        "id": "mwaki",
        "num": 4,
        "title": "Mwaki",
        "artist": "Zerb & Sofiya Nzau",
        "search_query": "Zerb Sofiya Nzau Mwaki official audio",
        "expected_duration": 208,
        "genre": "Afro House / Afro-Tech",
        "bpm": 122.0,
        "camelot": "8A",
        "meter": "4/4"
    },
    {
        "id": "chogada",
        "num": 5,
        "title": "Chogada",
        "artist": "Darshan Raval & Asees Kaur",
        "search_query": "Chogada Loveyatri official song",
        "expected_duration": 249,
        "genre": "Navratri Garba / Gujarati Folk",
        "bpm": 110.0,
        "camelot": "6B",
        "meter": "6/8"
    },
    {
        "id": "tremor",
        "num": 6,
        "title": "Tremor",
        "artist": "Dimitri Vegas, Like Mike & Martin Garrix",
        "search_query": "Dimitri Vegas Like Mike Martin Garrix Tremor original mix",
        "expected_duration": 294,
        "genre": "Big Room Electro House",
        "bpm": 128.0,
        "camelot": "5A",
        "meter": "4/4"
    },
    {
        "id": "levitating",
        "num": 7,
        "title": "Levitating",
        "artist": "Dua Lipa",
        "search_query": "Dua Lipa Levitating official audio",
        "expected_duration": 203,
        "genre": "Nu-Disco / Funk Pop",
        "bpm": 103.0,
        "camelot": "11A",
        "meter": "4/4"
    },
    {
        "id": "afreen_afreen",
        "num": 8,
        "title": "Afreen Afreen",
        "artist": "Rahat Fateh Ali Khan & Momina Mustehsan",
        "search_query": "Afreen Afreen Rahat Fateh Ali Khan Momina Mustehsan Coke Studio Season 9",
        "expected_duration": 405,
        "genre": "Sufi Qawwali / Fusion",
        "bpm": 84.0,
        "camelot": "8B",
        "meter": "4/4"
    },
    {
        "id": "baddadan",
        "num": 9,
        "title": "Baddadan",
        "artist": "Chase & Status, Bou ft. Flowdan",
        "search_query": "Chase & Status Bou Baddadan official audio",
        "expected_duration": 176,
        "genre": "Drum & Bass / Jungle",
        "bpm": 174.0,
        "camelot": "4A",
        "meter": "4/4"
    },
    {
        "id": "tum_hi_ho",
        "num": 10,
        "title": "Tum Hi Ho",
        "artist": "Arijit Singh / Mithoon",
        "search_query": "Tum Hi Ho Aashiqui 2 official song Arijit Singh",
        "expected_duration": 262,
        "genre": "Romantic Piano Ballad",
        "bpm": 87.0,
        "camelot": "4A",
        "meter": "4/4"
    }
]

# Save fresh batch JSON
batch_file = os.path.join(BATCHES_DIR, "round_01_songs_01_10.json")
with open(batch_file, "w", encoding="utf-8") as f:
    json.dump({"round": 1, "songs": ROUND_1_TRACKS}, f, indent=2)

print(f"Round 1 fresh batch cataloged in {batch_file}")

# Search and resolve official YouTube references
ydl_opts = {
    'format': 'bestaudio/best',
    'postprocessors': [{
        'key': 'FFmpegExtractAudio',
        'preferredcodec': 'mp3',
        'preferredquality': '192',
    }],
    'outtmpl': os.path.join(CACHE_DIR, '%(id)s.%(ext)s'),
    'ffmpeg_location': VENV_BIN,
    'quiet': True,
    'no_warnings': True,
}

resolved_manifest = []

for track in ROUND_1_TRACKS:
    q = track["search_query"]
    print(f"\nResolving [{track['num']}/10]: {track['title']} — {track['artist']}...")
    try:
        with yt_dlp.YoutubeDL({'quiet': True, 'no_warnings': True}) as ydl:
            search_res = ydl.extract_info(f"ytsearch1:{q}", download=False)
            if search_res and search_res.get('entries'):
                entry = search_res['entries'][0]
                vid = entry['id']
                vtitle = entry.get('title', '')
                vdur = entry.get('duration', 0)
                vurl = f"https://www.youtube.com/watch?v={vid}"
                mp3_path = os.path.join(CACHE_DIR, f"{vid}.mp3")

                print(f"  Found: {vtitle} ({vdur}s) [ID: {vid}]")

                # Download if not present
                if not os.path.exists(mp3_path):
                    print(f"  Downloading & converting to MP3...")
                    with yt_dlp.YoutubeDL(ydl_opts) as dl:
                        dl.download([vurl])
                    print(f"  Downloaded: {mp3_path}")
                else:
                    print(f"  Already cached: {mp3_path}")

                track_resolved = dict(track)
                track_resolved["resolved_video_id"] = vid
                track_resolved["resolved_title"] = vtitle
                track_resolved["resolved_duration"] = vdur
                track_resolved["resolved_url"] = vurl
                track_resolved["local_file"] = f"{vid}.mp3"
                track_resolved["local_path"] = mp3_path
                resolved_manifest.append(track_resolved)
            else:
                print(f"  Failed to find search results for query: {q}")
    except Exception as e:
        print(f"  Error resolving {track['title']}: {e}")

manifest_file = os.path.join(BATCHES_DIR, "round_01_resolved_manifest.json")
with open(manifest_file, "w", encoding="utf-8") as f:
    json.dump(resolved_manifest, f, indent=2)

print(f"\nRound 1 Resolved Manifest saved: {manifest_file} ({len(resolved_manifest)}/10 tracks resolved)")
