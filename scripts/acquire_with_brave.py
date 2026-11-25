import os
import sys
import json
import yt_dlp

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
BATCHES_DIR = os.path.join(CALIBRATION_DIR, "fresh_batches")
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
VENV_BIN = os.path.join(BASE_DIR, "resolver-service", ".venv", "Scripts")

if VENV_BIN not in os.environ["PATH"]:
    os.environ["PATH"] = VENV_BIN + os.pathsep + os.environ["PATH"]

TRACKS_TO_ACQUIRE = [
    {
        "id": "levitating",
        "num": 7,
        "title": "Levitating",
        "artist": "Dua Lipa",
        "url": "https://www.youtube.com/watch?v=TUVcZfQe-Kw",
        "search_query": "Dua Lipa Levitating official video",
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
        "url": "https://www.youtube.com/watch?v=kw4tT7SCmaY",
        "search_query": "Afreen Afreen Coke Studio Season 9",
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
        "url": "https://www.youtube.com/watch?v=rkjNL4dX-U4",
        "search_query": "Chase & Status Baddadan official video",
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
        "url": "https://www.youtube.com/watch?v=Umqb9KENgmk",
        "search_query": "Tum Hi Ho Aashiqui 2 official song",
        "genre": "Romantic Piano Ballad",
        "bpm": 87.0,
        "camelot": "4A",
        "meter": "4/4"
    }
]

ydl_opts = {
    'format': 'bestaudio/best',
    'cookiesfrombrowser': ('brave', ),
    'postprocessors': [{
        'key': 'FFmpegExtractAudio',
        'preferredcodec': 'mp3',
        'preferredquality': '192',
    }],
    'outtmpl': os.path.join(CACHE_DIR, '%(id)s.%(ext)s'),
    'ffmpeg_location': VENV_BIN,
    'quiet': False,
    'no_warnings': True,
}

manifest_file = os.path.join(BATCHES_DIR, "round_01_resolved_manifest.json")
existing_manifest = []
if os.path.exists(manifest_file):
    with open(manifest_file, "r", encoding="utf-8") as f:
        existing_manifest = json.load(f)

print(f"Loaded existing manifest with {len(existing_manifest)} tracks.")

for item in TRACKS_TO_ACQUIRE:
    print(f"\n--- Acquiring: {item['title']} by {item['artist']} with Brave Cookies ---")
    try:
        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            info = ydl.extract_info(item['url'], download=True)
            vid = info.get('id')
            vtitle = info.get('title', item['title'])
            vdur = info.get('duration', 0)
            mp3_path = os.path.join(CACHE_DIR, f"{vid}.mp3")

            print(f"Successfully downloaded: {vtitle} ({vdur}s) -> {mp3_path}")
            
            track_entry = dict(item)
            track_entry["resolved_video_id"] = vid
            track_entry["resolved_title"] = vtitle
            track_entry["resolved_duration"] = vdur
            track_entry["resolved_url"] = item['url']
            track_entry["local_file"] = f"{vid}.mp3"
            track_entry["local_path"] = mp3_path
            
            # Update manifest
            existing_manifest = [x for x in existing_manifest if x.get("id") != item["id"]]
            existing_manifest.append(track_entry)
    except Exception as e:
        print(f"Error acquiring {item['title']} via {item['url']}: {e}")
        # Try search fallback with brave cookies
        try:
            print(f"Trying search fallback for {item['title']}...")
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                info = ydl.extract_info(f"ytsearch1:{item['search_query']}", download=True)
                if info and info.get('entries'):
                    entry = info['entries'][0]
                    vid = entry.get('id')
                    vtitle = entry.get('title', item['title'])
                    vdur = entry.get('duration', 0)
                    mp3_path = os.path.join(CACHE_DIR, f"{vid}.mp3")
                    print(f"Search fallback successful: {vtitle} -> {mp3_path}")
                    track_entry = dict(item)
                    track_entry["resolved_video_id"] = vid
                    track_entry["resolved_title"] = vtitle
                    track_entry["resolved_duration"] = vdur
                    track_entry["resolved_url"] = f"https://www.youtube.com/watch?v={vid}"
                    track_entry["local_file"] = f"{vid}.mp3"
                    track_entry["local_path"] = mp3_path
                    existing_manifest = [x for x in existing_manifest if x.get("id") != item["id"]]
                    existing_manifest.append(track_entry)
        except Exception as e2:
            print(f"Search fallback also failed for {item['title']}: {e2}")

# Sort by track number
existing_manifest.sort(key=lambda x: x.get("num", 0))

with open(manifest_file, "w", encoding="utf-8") as f:
    json.dump(existing_manifest, f, indent=2)

print(f"\nFinal Round 1 Manifest updated: {manifest_file} ({len(existing_manifest)}/10 tracks total)")
