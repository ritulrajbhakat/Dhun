import os
import sys
import asyncio
import json
import time

# Ensure immediate unbuffered output in PowerShell
sys.stdout.reconfigure(line_buffering=True)

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
RESOLVER_DIR = os.path.join(BASE_DIR, "resolver-service")
sys.path.insert(0, RESOLVER_DIR)

from app import resolve_audio, ResolveRequest, CACHE_DIR

CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
BATCHES_DIR = os.path.join(CALIBRATION_DIR, "fresh_batches")
os.makedirs(BATCHES_DIR, exist_ok=True)
MANIFEST_PATH = os.path.join(BATCHES_DIR, "round_05_resolved_manifest.json")

ROUND_5_TRACKS = [
    {
        "id": "channa_mereya",
        "num": 51,
        "title": "Channa Mereya",
        "artist": "Arijit Singh / Pritam",
        "query": "ytsearch1:Channa Mereya Full Song Ae Dil Hai Mushkil Arijit Singh",
        "genre": "Slow Acoustic Ballad / Shehnai & Strings",
        "bpm": 74.0,
        "camelot": "8B",
        "meter": "4/4"
    },
    {
        "id": "last_last",
        "num": 52,
        "title": "Last Last",
        "artist": "Burna Boy",
        "query": "ytsearch1:Burna Boy Last Last Official Music Video",
        "genre": "Global Afrobeats / Toni Braxton Sample",
        "bpm": 90.0,
        "camelot": "11B",
        "meter": "4/4"
    },
    {
        "id": "taki_taki",
        "num": 53,
        "title": "Taki Taki",
        "artist": "DJ Snake ft. Selena Gomez, Ozuna, Cardi B",
        "query": "ytsearch1:DJ Snake Taki Taki ft Selena Gomez Ozuna Cardi B Official Video",
        "genre": "Latin Moombahton / Dembow Tresillo",
        "bpm": 96.0,
        "camelot": "6A",
        "meter": "4/4"
    },
    {
        "id": "love_nwantiti",
        "num": 54,
        "title": "Love Nwantiti (Ah Ah Ah)",
        "artist": "CKay",
        "query": "ytsearch1:CKay Love Nwantiti Ah Ah Ah Official Music Video",
        "genre": "Nigerian Afro-Pop / Emotive Afro-R&B",
        "bpm": 100.0,
        "camelot": "9A",
        "meter": "4/4"
    },
    {
        "id": "bom_diggy_diggy",
        "num": 55,
        "title": "Bom Diggy Diggy",
        "artist": "Zack Knight & Jasmin Walia",
        "query": "ytsearch1:Bom Diggy Diggy Zack Knight Jasmin Walia Sonu Ke Titu Ki Sweety",
        "genre": "Urban UK-Asian / Punjabi-Pop Dance",
        "bpm": 102.0,
        "camelot": "11A",
        "meter": "4/4"
    },
    {
        "id": "shubhaarambh",
        "num": 56,
        "title": "Shubhaarambh",
        "artist": "Amit Trivedi, Divya Kumar, Shruti Pathak",
        "query": "ytsearch1:Shubhaarambh Kai Po Che Amit Trivedi Official Full Song",
        "genre": "Compound 6/8 Traditional Gujarati Garba Folk",
        "bpm": 116.0,
        "camelot": "1B",
        "meter": "6/8"
    },
    {
        "id": "chammak_challo",
        "num": 57,
        "title": "Chammak Challo",
        "artist": "Akon & Hamsika Iyer",
        "query": "ytsearch1:Chammak Challo Full Song Ra One Akon Kareena Kapoor",
        "genre": "Global Bollywood-R&B Club Anthem",
        "bpm": 108.0,
        "camelot": "10A",
        "meter": "4/4"
    },
    {
        "id": "heads_will_roll",
        "num": 58,
        "title": "Heads Will Roll (A-Trak Remix)",
        "artist": "Yeah Yeah Yeahs",
        "query": "ytsearch1:Yeah Yeah Yeahs Heads Will Roll A-Trak Remix",
        "genre": "Peak-Time Electro-House / Festival Anthem",
        "bpm": 128.0,
        "camelot": "11A",
        "meter": "4/4"
    },
    {
        "id": "mauja_hi_mauja",
        "num": 59,
        "title": "Mauja Hi Mauja",
        "artist": "Mika Singh",
        "query": "ytsearch1:Mauja Hi Mauja Full Song Jab We Met Shahid Kapoor Kareena Kapoor",
        "genre": "High-Octane Commercial Bollywood-Bhangra Banger",
        "bpm": 135.0,
        "camelot": "10B",
        "meter": "4/4"
    },
    {
        "id": "winning_speech",
        "num": 60,
        "title": "Winning Speech",
        "artist": "Karan Aujla",
        "query": "ytsearch1:Karan Aujla Winning Speech Official Music Video",
        "genre": "Half-Time Punjabi UK Drill / Sliding 808 Trap",
        "bpm": 140.0,
        "camelot": "9A",
        "meter": "4/4"
    }
]

async def acquire_round5():
    print("=" * 60)
    print("ACQUIRING ROUND 5 (PRE-200-SONG REVALIDATION) FRESH 10 TRACKS")
    print("Exact resolver app code with 15-20s non-blocking gap")
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

    for t in ROUND_5_TRACKS:
        tid = t["id"]
        if tid in existing_map:
            print(f"Track [{t['num']}/60] {t['title']} already acquired: {existing_map[tid]['local_file']}")
            manifest.append(existing_map[tid])
            continue
            
        print(f"\n--- Resolving Track [{t['num']}/60]: {t['title']} — {t['artist']} ---")
        q = t["query"]
        req = ResolveRequest(
            source_url=q,
            title=t["title"],
            artist=t["artist"],
            version_tag="original"
        )
        try:
            result = await resolve_audio(req)
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
            else:
                print(f"WARNING: File {local_path} not found or too small.")
        except Exception as e:
            print(f"Attempt failed for {q}: {e}")

        # Save incremental manifest after every track
        with open(MANIFEST_PATH, "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=2)

    print("\n" + "=" * 60)
    print(f"ROUND 5 ACQUISITION COMPLETE: {len(manifest)}/10 tracks successfully acquired.")
    print("=" * 60)

if __name__ == "__main__":
    asyncio.run(acquire_round5())
