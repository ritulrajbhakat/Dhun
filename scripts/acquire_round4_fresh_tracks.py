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
os.makedirs(BATCHES_DIR, exist_ok=True)
MANIFEST_PATH = os.path.join(BATCHES_DIR, "round_04_resolved_manifest.json")

ROUND_4_TRACKS = [
    {
        "id": "agar_tum_saath_ho",
        "num": 31,
        "title": "Agar Tum Saath Ho",
        "artist": "Alka Yagnik & Arijit Singh",
        "query": "ytsearch1:Agar Tum Saath Ho Full Song Tamasha Arijit Singh Alka Yagnik",
        "genre": "Slow Acoustic Ballad / Shehnai & Strings",
        "bpm": 80.0,
        "camelot": "9B",
        "meter": "4/4"
    },
    {
        "id": "baller",
        "num": 32,
        "title": "Baller",
        "artist": "Shubh",
        "query": "ytsearch1:Shubh Baller Official Music Video",
        "genre": "West Coast Punjabi 808 Hip-Hop",
        "bpm": 96.0,
        "camelot": "6A",
        "meter": "4/4"
    },
    {
        "id": "lean_on",
        "num": 33,
        "title": "Lean On",
        "artist": "Major Lazer & DJ Snake ft. MØ",
        "query": "ytsearch1:Major Lazer DJ Snake Lean On feat MO Official Music Video",
        "genre": "Global Moombahton / Dancehall Tresillo",
        "bpm": 98.0,
        "camelot": "8A",
        "meter": "4/4"
    },
    {
        "id": "calm_down",
        "num": 34,
        "title": "Calm Down",
        "artist": "Rema",
        "query": "ytsearch1:Rema Calm Down Official Music Video",
        "genre": "Afrobeats Polyrhythmic Afro-Swing",
        "bpm": 107.0,
        "camelot": "2B",
        "meter": "4/4"
    },
    {
        "id": "deewani_mastani",
        "num": 35,
        "title": "Deewani Mastani",
        "artist": "Shreya Ghoshal",
        "query": "ytsearch1:Deewani Mastani Full Video Song Bajirao Mastani Shreya Ghoshal",
        "genre": "Compound 6/8 Indian Classical Mujra Folk",
        "bpm": 114.0,
        "camelot": "3B",
        "meter": "6/8"
    },
    {
        "id": "aankh_marey",
        "num": 36,
        "title": "Aankh Marey",
        "artist": "Neha Kakkar, Mika Singh, Kumar Sanu",
        "query": "ytsearch1:Aankh Marey Simmba Neha Kakkar Mika Singh Kumar Sanu",
        "genre": "Commercial Bollywood Electro-House Banger",
        "bpm": 118.0,
        "camelot": "2A",
        "meter": "4/4"
    },
    {
        "id": "wake_me_up",
        "num": 37,
        "title": "Wake Me Up",
        "artist": "Avicii ft. Aloe Blacc",
        "query": "ytsearch1:Avicii Wake Me Up Official Video",
        "genre": "Progressive Folk-House / Melodic EDM",
        "bpm": 124.0,
        "camelot": "11B",
        "meter": "4/4"
    },
    {
        "id": "danza_kuduro",
        "num": 38,
        "title": "Danza Kuduro",
        "artist": "Don Omar ft. Lucenzo",
        "query": "ytsearch1:Don Omar Danza Kuduro ft Lucenzo Official Music Video",
        "genre": "Fast Latin Mambo / Merengue Dance Pulse",
        "bpm": 130.0,
        "camelot": "9A",
        "meter": "4/4"
    },
    {
        "id": "montagem_pr_funk",
        "num": 39,
        "title": "Montagem - PR Funk",
        "artist": "S3BZS / Mc Vitin LC",
        "query": "ytsearch1:S3BZS MONTAGEM PR FUNK Official Audio",
        "genre": "Brazilian Baile Funk / Tamborzão Syncopation",
        "bpm": 130.0,
        "camelot": "4A",
        "meter": "4/4"
    },
    {
        "id": "cinema_skrillex",
        "num": 40,
        "title": "Cinema (Skrillex Remix)",
        "artist": "Benny Benassi ft. Gary Go",
        "query": "ytsearch1:Benny Benassi Cinema Skrillex Remix Official",
        "genre": "Half-Time Melodic Dubstep / Bass Climax",
        "bpm": 140.0,
        "camelot": "1A",
        "meter": "4/4"
    }
]

async def acquire_round4():
    print("=" * 60)
    print("ACQUIRING ROUND 4 FRESH 10 TRACKS VIA EXACT RESOLVER APP CODE")
    print("With built-in 15-20s non-blocking delays between downloads")
    print("=" * 60)
    
    manifest = []
    
    existing_map = {}
    if os.path.exists(MANIFEST_PATH):
        try:
            with open(MANIFEST_PATH, "r", encoding="utf-8") as f:
                prev_manifest = json.load(f)
                for item in prev_manifest:
                    if os.path.exists(item.get("local_path", "")) and os.path.getsize(item.get("local_path", "")) > 100000:
                        existing_map[item["id"]] = item
        except Exception:
            pass

    for t in ROUND_4_TRACKS:
        tid = t["id"]
        if tid in existing_map:
            print(f"Track [{t['num']}/40] {t['title']} already acquired: {existing_map[tid]['local_file']}")
            manifest.append(existing_map[tid])
            continue
            
        print(f"\n--- Resolving Track [{t['num']}/40]: {t['title']} — {t['artist']} ---")
        q = t["query"]
        req = ResolveRequest(
            source_url=q,
            title=t["title"],
            artist=t["artist"],
            version_tag="original"
        )
        try:
            result = await resolve_audio(req)
            print(f"Resolver output: {result}")
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
                print(f"SUCCESS: {t['title']} saved to {local_path} ({os.path.getsize(local_path):,} bytes)")
        except Exception as e:
            print(f"Attempt failed for {q}: {e}")

        # Save incremental manifest after every track
        with open(MANIFEST_PATH, "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=2)

        # Enforce 15-20s gap between tracks as instructed
        print("Waiting 15 seconds before processing next track...")
        await asyncio.sleep(15)

    print("\n" + "=" * 60)
    print(f"ROUND 4 ACQUISITION COMPLETE: {len(manifest)}/10 tracks successfully acquired.")
    print("=" * 60)

if __name__ == "__main__":
    asyncio.run(acquire_round4())
