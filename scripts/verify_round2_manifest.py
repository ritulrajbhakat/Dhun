import os
import json

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
MANIFEST_PATH = os.path.join(BASE_DIR, "DHUN_CALIBRATION", "fresh_batches", "round_02_resolved_manifest.json")

all_10 = [
    {
        "id": "gallan_goodiyaan",
        "num": 11,
        "title": "Gallan Goodiyaan",
        "artist": "Yashita Sharma, Farhan Akhtar, Shankar Mahadevan",
        "genre": "Bollywood Swing / Group Vocal",
        "bpm": 128.0,
        "camelot": "11B",
        "meter": "4/4",
        "local_file": "jCEdTq3j-0U.mp3",
        "local_path": os.path.join(CACHE_DIR, "jCEdTq3j-0U.mp3"),
        "resolved_title": "Gallan Goodiyaan Full Song",
        "resolved_duration": 278,
        "status": "success"
    },
    {
        "id": "proper_patola",
        "num": 12,
        "title": "Proper Patola",
        "artist": "Diljit Dosanjh, Badshah, Aastha Gill",
        "genre": "Urban Punjabi / Desi Hip-Hop",
        "bpm": 95.0,
        "camelot": "4A",
        "meter": "4/4",
        "local_file": "YmXJp4RtBCM.mp3",
        "local_path": os.path.join(CACHE_DIR, "YmXJp4RtBCM.mp3"),
        "resolved_title": "Proper Patola - Official Video",
        "resolved_duration": 180,
        "status": "success"
    },
    {
        "id": "gasolina",
        "num": 13,
        "title": "Gasolina",
        "artist": "Daddy Yankee",
        "genre": "Reggaeton / Dembow",
        "bpm": 96.0,
        "camelot": "4A",
        "meter": "4/4",
        "local_file": "CCF1_jI8Prk.mp3",
        "local_path": os.path.join(CACHE_DIR, "CCF1_jI8Prk.mp3"),
        "resolved_title": "Daddy Yankee - Gasolina (Video Oficial)",
        "resolved_duration": 254,
        "status": "success"
    },
    {
        "id": "pepas",
        "num": 14,
        "title": "Pepas",
        "artist": "Farruko",
        "genre": "Latin Guaracha / Tribal House",
        "bpm": 130.0,
        "camelot": "10A",
        "meter": "4/4",
        "local_file": "y8trd3gjJt0.mp3",
        "local_path": os.path.join(CACHE_DIR, "y8trd3gjJt0.mp3"),
        "resolved_title": "Farruko - Pepas (Official Video)",
        "resolved_duration": 294,
        "status": "success"
    },
    {
        "id": "sauda_khara_khara",
        "num": 15,
        "title": "Sauda Khara Khara",
        "artist": "Diljit Dosanjh, Sukhbir, Dhvani Bhanushali",
        "genre": "Bhangra Club / Electro Dhol",
        "bpm": 132.0,
        "camelot": "9A",
        "meter": "4/4",
        "local_file": "651huO7TOWI.mp3",
        "local_path": os.path.join(CACHE_DIR, "651huO7TOWI.mp3"),
        "resolved_title": "Sauda Khara Khara |Good Newwz| Akshay Kumar, Diljit",
        "resolved_duration": 222,
        "status": "success"
    },
    {
        "id": "kesariya",
        "num": 16,
        "title": "Kesariya",
        "artist": "Pritam, Arijit Singh",
        "genre": "Compound 6/8 Indian Folk / Dadra",
        "bpm": 94.0,
        "camelot": "7B",
        "meter": "6/8",
        "local_file": "BddP6PYo2gs.mp3",
        "local_path": os.path.join(CACHE_DIR, "BddP6PYo2gs.mp3"),
        "resolved_title": "Kesariya - Brahmastra | Arijit Singh",
        "resolved_duration": 172,
        "status": "success"
    },
    {
        "id": "losing_it",
        "num": 17,
        "title": "Losing It",
        "artist": "FISHER",
        "genre": "Tech House / Peak Club",
        "bpm": 125.0,
        "camelot": "4A",
        "meter": "4/4",
        "local_file": "u31thuMehjM.mp3",
        "local_path": os.path.join(CACHE_DIR, "u31thuMehjM.mp3"),
        "resolved_title": "FISHER - Losing It (Official Audio)",
        "resolved_duration": 249,
        "status": "success"
    },
    {
        "id": "sicko_mode",
        "num": 18,
        "title": "Sicko Mode",
        "artist": "Travis Scott ft. Drake",
        "genre": "Trap / Half-Time Hip-Hop",
        "bpm": 155.0,
        "camelot": "10A",
        "meter": "4/4",
        "local_file": "6ONRf7h3Mdk.mp3",
        "local_path": os.path.join(CACHE_DIR, "6ONRf7h3Mdk.mp3"),
        "resolved_title": "Travis Scott - SICKO MODE (Official Video) ft. Drake",
        "resolved_duration": 323,
        "status": "success"
    },
    {
        "id": "dancin_krono",
        "num": 19,
        "title": "Dancin (Krono Remix)",
        "artist": "Aaron Smith ft. Luvli",
        "genre": "Melodic Deep House",
        "bpm": 120.0,
        "camelot": "11A",
        "meter": "4/4",
        "local_file": "0XFudmaObLI.mp3",
        "local_path": os.path.join(CACHE_DIR, "0XFudmaObLI.mp3"),
        "resolved_title": "Aaron Smith - Dancin (KRONO Remix)",
        "resolved_duration": 256,
        "status": "success"
    },
    {
        "id": "raataan_lambiyan",
        "num": 20,
        "title": "Raataan Lambiyan",
        "artist": "Tanishk Bagchi, Jubin Nautiyal, Asees Kaur",
        "genre": "Acoustic Ballad / Romantic Pop",
        "bpm": 78.0,
        "camelot": "2B",
        "meter": "4/4",
        "local_file": "gvyUuxdRdR4.mp3",
        "local_path": os.path.join(CACHE_DIR, "gvyUuxdRdR4.mp3"),
        "resolved_title": "Raataan Lambiyan - Official Video | Shershaah",
        "resolved_duration": 151,
        "status": "success"
    }
]

verified = []
for t in all_10:
    path = t["local_path"]
    if os.path.exists(path) and os.path.getsize(path) > 100000:
        sz = os.path.getsize(path)
        print(f"Verified [{t['num']}/20] {t['title']}: {t['local_file']} ({sz:,} bytes)")
        verified.append(t)
    else:
        print(f"MISSING or too small: {t['title']} at {path}")

with open(MANIFEST_PATH, "w", encoding="utf-8") as f:
    json.dump(verified, f, indent=2)

print(f"\nTotal verified: {len(verified)}/10. Saved manifest to {MANIFEST_PATH}")
