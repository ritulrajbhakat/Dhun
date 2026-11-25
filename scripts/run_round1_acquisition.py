import os
import sys
import asyncio
import json
import time

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
RESOLVER_DIR = os.path.join(BASE_DIR, "resolver-service")
sys.path.insert(0, RESOLVER_DIR)

from app import resolve_audio, ResolveRequest, CACHE_DIR

CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
BATCHES_DIR = os.path.join(CALIBRATION_DIR, "fresh_batches")
MANIFEST_PATH = os.path.join(BATCHES_DIR, "round_01_resolved_manifest.json")

# Candidate URLs for the 4 remaining tracks (ranked by accessibility)
CANDIDATE_TRACKS = [
    {
        "id": "levitating",
        "num": 7,
        "title": "Levitating",
        "artist": "Dua Lipa",
        "urls": [
            "https://www.youtube.com/watch?v=N000qglmmY0",
            "https://www.youtube.com/watch?v=TUVcZfQe-Kw",
            "https://www.youtube.com/watch?v=miP66ukUkTw"
        ],
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
        "urls": [
            "https://www.youtube.com/watch?v=SMlGGRAB3Hc",
            "https://www.youtube.com/watch?v=kw4tT7SCmaY",
            "https://www.youtube.com/watch?v=8367ETnagHo"
        ],
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
        "urls": [
            "https://www.youtube.com/watch?v=wJSNHbiXTFI",
            "https://www.youtube.com/watch?v=V_2vH6-0Ct4",
            "https://www.youtube.com/watch?v=rkjNL4dX-U4"
        ],
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
        "urls": [
            "https://www.youtube.com/watch?v=81qmmlsIE3k",
            "https://www.youtube.com/watch?v=NUo8CKI34o4",
            "https://www.youtube.com/watch?v=Umqb9KENgmk"
        ],
        "genre": "Romantic Piano Ballad",
        "bpm": 87.0,
        "camelot": "4A",
        "meter": "4/4"
    }
]

# Tracks already acquired in cache
ALREADY_ACQUIRED = [
    {
        "id": "badtameez_dil",
        "num": 1,
        "title": "Badtameez Dil",
        "artist": "Pritam, Benny Dayal, Shefali Alvares",
        "file": "II2EO3Nw4m0.mp3",
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
        "file": "dndfQA1uleQ.mp3",
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
        "file": "U-l4ya3ejko.mp3",
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
        "file": "pc0crl_oETA.mp3",
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
        "file": "asYxxtiWUyw.mp3",
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
        "file": "9vMh9f41pqE.mp3",
        "genre": "Big Room Electro House",
        "bpm": 128.0,
        "camelot": "5A",
        "meter": "4/4"
    }
]

async def acquire_all():
    manifest = []
    
    # Add already acquired tracks
    for t in ALREADY_ACQUIRED:
        fpath = os.path.join(CACHE_DIR, t["file"])
        if os.path.exists(fpath):
            entry = dict(t)
            entry["local_path"] = fpath
            entry["local_file"] = t["file"]
            entry["status"] = "success"
            manifest.append(entry)
            print(f"Verified cached track [{t['num']}/10]: {t['title']} ({os.path.getsize(fpath)} bytes)")

    # Resolve remaining tracks via exact resolver app code with 20s delays
    for t in CANDIDATE_TRACKS:
        print(f"\n--- Resolving Track [{t['num']}/10]: {t['title']} — {t['artist']} ---")
        success = False
        for url in t["urls"]:
            print(f"Trying URL: {url}...")
            req = ResolveRequest(
                source_url=url,
                title=t["title"],
                artist=t["artist"],
                version_tag="original"
            )
            try:
                # Calls the exact resolver function from app.py
                result = await resolve_audio(req)
                print(f"Resolver success: {result}")
                audio_url = result.get("direct_https_audio_url", "")
                filename = os.path.basename(audio_url)
                local_path = os.path.join(CACHE_DIR, filename)

                if os.path.exists(local_path) and os.path.getsize(local_path) > 100000:
                    entry = dict(t)
                    entry["local_path"] = local_path
                    entry["local_file"] = filename
                    entry["resolved_title"] = result.get("title")
                    entry["resolved_duration"] = result.get("duration")
                    entry["status"] = "success"
                    manifest.append(entry)
                    print(f"Successfully acquired: {t['title']} -> {local_path} ({os.path.getsize(local_path)} bytes)")
                    success = True
                    break
            except Exception as e:
                print(f"Resolver attempt failed for {url}: {e}")
                print("Waiting 15 seconds before trying next candidate...")
                await asyncio.sleep(15)

        if not success:
            print(f"WARNING: All candidate URLs failed for {t['title']}")

        # Enforce 20s gap between song downloads
        print("Waiting 20 seconds before starting next track download...")
        await asyncio.sleep(20)

    # Sort and save manifest
    manifest.sort(key=lambda x: x.get("num", 0))
    with open(MANIFEST_PATH, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)

    print(f"\nRound 1 Manifest saved to {MANIFEST_PATH}: {len(manifest)}/10 tracks verified.")

if __name__ == "__main__":
    asyncio.run(acquire_all())
