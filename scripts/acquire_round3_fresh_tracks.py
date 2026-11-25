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
MANIFEST_PATH = os.path.join(BATCHES_DIR, "round_03_resolved_manifest.json")

ROUND_3_TRACKS = [
    {
        "id": "mnike",
        "num": 21,
        "title": "Mnike",
        "artist": "Tyler ICU & Tumelo.za",
        "query": "ytsearch1:Tyler ICU Mnike Official Music Video",
        "genre": "Amapiano Syncopated Log-Drum",
        "bpm": 113.0,
        "camelot": "11A",
        "meter": "4/4"
    },
    {
        "id": "two_nine_five",
        "num": 22,
        "title": "295",
        "artist": "Sidhu Moose Wala",
        "query": "ytsearch1:Sidhu Moose Wala 295 Official Audio Moosetape",
        "genre": "Punjabi Hip-Hop Dholak Groove",
        "bpm": 90.0,
        "camelot": "9A",
        "meter": "4/4"
    },
    {
        "id": "doja",
        "num": 23,
        "title": "Doja",
        "artist": "Central Cee",
        "query": "ytsearch1:Central Cee Doja Official Music Video",
        "genre": "UK Drill Sliding 808",
        "bpm": 140.0,
        "camelot": "1A",
        "meter": "4/4"
    },
    {
        "id": "great_spirit",
        "num": 24,
        "title": "Great Spirit",
        "artist": "Armin van Buuren vs Vini Vici",
        "query": "ytsearch1:Armin van Buuren vs Vini Vici Great Spirit Official Video",
        "genre": "Driving Psytrance Rolling Bassline",
        "bpm": 138.0,
        "camelot": "11A",
        "meter": "4/4"
    },
    {
        "id": "kar_gayi_chull",
        "num": 25,
        "title": "Kar Gayi Chull",
        "artist": "Badshah, Neha Kakkar",
        "query": "ytsearch1:Kar Gayi Chull Kapoor Sons Official Video Badshah",
        "genre": "Commercial House Dhol Beat",
        "bpm": 125.0,
        "camelot": "1A",
        "meter": "4/4"
    },
    {
        "id": "mi_gente",
        "num": 26,
        "title": "Mi Gente",
        "artist": "J Balvin, Willy William",
        "query": "ytsearch1:J Balvin Willy William Mi Gente Official Video",
        "genre": "Moombahton Trombone Hook",
        "bpm": 105.0,
        "camelot": "8A",
        "meter": "4/4"
    },
    {
        "id": "ghoomar",
        "num": 27,
        "title": "Ghoomar",
        "artist": "Shreya Ghoshal, Swaroop Khan",
        "query": "ytsearch1:Ghoomar Padmaavat Official Video Shreya Ghoshal",
        "genre": "Compound 6/8 Rajasthani Nagada Folk",
        "bpm": 116.0,
        "camelot": "11B",
        "meter": "6/8"
    },
    {
        "id": "opus",
        "num": 28,
        "title": "Opus",
        "artist": "Eric Prydz",
        "query": "ytsearch1:Eric Prydz Opus Official Video",
        "genre": "Progressive House Accelerating Arp",
        "bpm": 126.0,
        "camelot": "4A",
        "meter": "4/4"
    },
    {
        "id": "kun_faya_kun",
        "num": 29,
        "title": "Kun Faya Kun",
        "artist": "A.R. Rahman, Javed Ali",
        "query": "ytsearch1:Kun Faya Kun Rockstar Official Full Song A R Rahman",
        "genre": "Spiritual Sufi Tabla Harmonium",
        "bpm": 78.0,
        "camelot": "10B",
        "meter": "4/4"
    },
    {
        "id": "solar_system",
        "num": 30,
        "title": "Solar System",
        "artist": "Sub Focus",
        "query": "ytsearch1:Sub Focus Solar System Official Video",
        "genre": "High-Speed Drum & Bass Roll",
        "bpm": 174.0,
        "camelot": "1A",
        "meter": "4/4"
    }
]

async def acquire_round3():
    print("=" * 60)
    print("ACQUIRING ROUND 3 FRESH 10 TRACKS VIA EXACT RESOLVER APP CODE")
    print("With built-in 15-20s non-blocking delays between downloads")
    print("=" * 60)
    
    manifest = []
    
    # Check if manifest already has partial progress
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

    for t in ROUND_3_TRACKS:
        tid = t["id"]
        if tid in existing_map:
            print(f"Track [{t['num']}/30] {t['title']} already acquired: {existing_map[tid]['local_file']}")
            manifest.append(existing_map[tid])
            continue
            
        print(f"\n--- Resolving Track [{t['num']}/30]: {t['title']} — {t['artist']} ---")
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
    print(f"ROUND 3 ACQUISITION COMPLETE: {len(manifest)}/10 tracks successfully acquired.")
    print("=" * 60)

if __name__ == "__main__":
    asyncio.run(acquire_round3())
