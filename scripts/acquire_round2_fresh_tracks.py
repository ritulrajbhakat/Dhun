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
MANIFEST_PATH = os.path.join(BATCHES_DIR, "round_02_resolved_manifest.json")

ROUND_2_TRACKS = [
    {
        "id": "gallan_goodiyaan",
        "num": 11,
        "title": "Gallan Goodiyaan",
        "artist": "Yashita Sharma, Farhan Akhtar, Shankar Mahadevan",
        "urls": [
            "https://www.youtube.com/watch?v=jCEdTq3j-0U",
            "https://www.youtube.com/watch?v=0k_PmM8m88A"
        ],
        "genre": "Bollywood Swing / Group Vocal",
        "bpm": 128.0,
        "camelot": "11B",
        "meter": "4/4"
    },
    {
        "id": "proper_patola",
        "num": 12,
        "title": "Proper Patola",
        "artist": "Diljit Dosanjh, Badshah, Aastha Gill",
        "urls": [
            "https://www.youtube.com/watch?v=sU_K27pB3H8",
            "https://www.youtube.com/watch?v=J3e3wJ7uM1Y"
        ],
        "genre": "Urban Punjabi / Desi Hip-Hop",
        "bpm": 95.0,
        "camelot": "4A",
        "meter": "4/4"
    },
    {
        "id": "gasolina",
        "num": 13,
        "title": "Gasolina",
        "artist": "Daddy Yankee",
        "urls": [
            "https://www.youtube.com/watch?v=CCF1_jI8Prk",
            "https://www.youtube.com/watch?v=qGKrc3A6HHM"
        ],
        "genre": "Reggaeton / Dembow",
        "bpm": 96.0,
        "camelot": "4A",
        "meter": "4/4"
    },
    {
        "id": "pepas",
        "num": 14,
        "title": "Pepas",
        "artist": "Farruko",
        "urls": [
            "https://www.youtube.com/watch?v=y8trd3gjJt0",
            "https://www.youtube.com/watch?v=QdZ5uW_9h9U"
        ],
        "genre": "Latin Guaracha / Tribal House",
        "bpm": 130.0,
        "camelot": "10A",
        "meter": "4/4"
    },
    {
        "id": "sauda_khara_khara",
        "num": 15,
        "title": "Sauda Khara Khara",
        "artist": "Diljit Dosanjh, Sukhbir, Dhvani Bhanushali",
        "urls": [
            "https://www.youtube.com/watch?v=p4vW7B1Ue3M",
            "https://www.youtube.com/watch?v=c_f4-V3G0jU"
        ],
        "genre": "Bhangra Club / Electro Dhol",
        "bpm": 132.0,
        "camelot": "9A",
        "meter": "4/4"
    },
    {
        "id": "kesariya",
        "num": 16,
        "title": "Kesariya",
        "artist": "Pritam, Arijit Singh",
        "urls": [
            "https://www.youtube.com/watch?v=BddP6PYo2gs",
            "https://www.youtube.com/watch?v=Qd1pSj6X5jA"
        ],
        "genre": "Compound 6/8 Indian Folk / Dadra",
        "bpm": 94.0,
        "camelot": "7B",
        "meter": "6/8"
    },
    {
        "id": "losing_it",
        "num": 17,
        "title": "Losing It",
        "artist": "FISHER",
        "urls": [
            "https://www.youtube.com/watch?v=vV1p_xW4h48",
            "https://www.youtube.com/watch?v=9_5x1J8Q0Y8"
        ],
        "genre": "Tech House / Peak Club",
        "bpm": 125.0,
        "camelot": "4A",
        "meter": "4/4"
    },
    {
        "id": "sicko_mode",
        "num": 18,
        "title": "Sicko Mode",
        "artist": "Travis Scott ft. Drake",
        "urls": [
            "https://www.youtube.com/watch?v=6ONRf7h3Mdk",
            "https://www.youtube.com/watch?v=d-JBBNg8YKs"
        ],
        "genre": "Trap / Half-Time Hip-Hop",
        "bpm": 155.0,
        "camelot": "10A",
        "meter": "4/4"
    },
    {
        "id": "dancin_krono",
        "num": 19,
        "title": "Dancin (Krono Remix)",
        "artist": "Aaron Smith ft. Luvli",
        "urls": [
            "https://www.youtube.com/watch?v=0XFudmaObLI",
            "https://www.youtube.com/watch?v=6_IBW84H9bA"
        ],
        "genre": "Melodic Deep House",
        "bpm": 120.0,
        "camelot": "11A",
        "meter": "4/4"
    },
    {
        "id": "raataan_lambiyan",
        "num": 20,
        "title": "Raataan Lambiyan",
        "artist": "Tanishk Bagchi, Jubin Nautiyal, Asees Kaur",
        "urls": [
            "https://www.youtube.com/watch?v=gvyUuxdRdR4",
            "https://www.youtube.com/watch?v=7X_A7eG5a4Q"
        ],
        "genre": "Acoustic Ballad / Romantic Pop",
        "bpm": 78.0,
        "camelot": "2B",
        "meter": "4/4"
    }
]

async def acquire_round2():
    print("=" * 60)
    print("ACQUIRING ROUND 2 FRESH 10 TRACKS VIA EXACT RESOLVER APP CODE")
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

    for t in ROUND_2_TRACKS:
        tid = t["id"]
        if tid in existing_map:
            print(f"Track [{t['num']}/20] {t['title']} already acquired: {existing_map[tid]['local_file']}")
            manifest.append(existing_map[tid])
            continue
            
        print(f"\n--- Resolving Track [{t['num']}/20]: {t['title']} — {t['artist']} ---")
        success = False
        
        for url in t["urls"]:
            print(f"Invoking resolve_audio with candidate URL: {url}...")
            req = ResolveRequest(
                source_url=url,
                title=t["title"],
                artist=t["artist"],
                version_tag="original"
            )
            try:
                # Calls exact resolver function from app.py
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
                    success = True
                    break
            except Exception as e:
                print(f"Attempt failed for {url}: {e}")
                print("Waiting 15s before next attempt...")
                await asyncio.sleep(15)

        if not success:
            print(f"CRITICAL WARNING: Failed to resolve {t['title']} across all candidates.")

        # Save incremental manifest after every track
        with open(MANIFEST_PATH, "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=2)

        # Enforce 15-20s gap between tracks as instructed
        print("Waiting 15 seconds before processing next track...")
        await asyncio.sleep(15)

    print("\n" + "=" * 60)
    print(f"ROUND 2 ACQUISITION COMPLETE: {len(manifest)}/10 tracks successfully acquired.")
    print("=" * 60)

if __name__ == "__main__":
    asyncio.run(acquire_round2())
