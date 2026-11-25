import os
import json

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
BATCHES_DIR = os.path.join(CALIBRATION_DIR, "fresh_batches")
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
MANIFEST_PATH = os.path.join(BATCHES_DIR, "round_01_resolved_manifest.json")

TRACKS = [
    {
        "id": "badtameez_dil",
        "num": 1,
        "title": "Badtameez Dil",
        "artist": "Pritam, Benny Dayal, Shefali Alvares",
        "file": "II2EO3Nw4m0.mp3",
        "genre": "Bollywood Swing / Dance-Pop",
        "bpm": 152.0,
        "camelot": "11A",
        "meter": "4/4",
        "duration": 271
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
        "meter": "4/4",
        "duration": 246
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
        "meter": "4/4",
        "duration": 193
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
        "meter": "4/4",
        "duration": 209
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
        "meter": "6/8",
        "duration": 256
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
        "meter": "4/4",
        "duration": 200
    },
    {
        "id": "levitating",
        "num": 7,
        "title": "Levitating",
        "artist": "Dua Lipa",
        "file": "N000qglmmY0.mp3",
        "genre": "Nu-Disco / Funk Pop",
        "bpm": 103.0,
        "camelot": "11A",
        "meter": "4/4",
        "duration": 210
    },
    {
        "id": "afreen_afreen",
        "num": 8,
        "title": "Afreen Afreen",
        "artist": "Rahat Fateh Ali Khan & Momina Mustehsan",
        "file": "SMlGGRAB3Hc.mp3",
        "genre": "Sufi Qawwali / Fusion",
        "bpm": 84.0,
        "camelot": "8B",
        "meter": "4/4",
        "duration": 400
    },
    {
        "id": "baddadan",
        "num": 9,
        "title": "Baddadan",
        "artist": "Chase & Status, Bou ft. Flowdan",
        "file": "wJSNHbiXTFI.mp3",
        "genre": "Drum & Bass / Jungle",
        "bpm": 174.0,
        "camelot": "4A",
        "meter": "4/4",
        "duration": 243
    },
    {
        "id": "tum_hi_ho",
        "num": 10,
        "title": "Tum Hi Ho",
        "artist": "Arijit Singh / Mithoon",
        "file": "Umqb9KENgmk.mp3",
        "genre": "Romantic Piano Ballad",
        "bpm": 87.0,
        "camelot": "4A",
        "meter": "4/4",
        "duration": 267
    }
]

manifest = []
for t in TRACKS:
    fpath = os.path.join(CACHE_DIR, t["file"])
    assert os.path.exists(fpath), f"File missing: {fpath}"
    size = os.path.getsize(fpath)
    entry = dict(t)
    entry["local_path"] = fpath
    entry["local_file"] = t["file"]
    entry["file_size"] = size
    entry["status"] = "success"
    manifest.append(entry)
    print(f"Track [{t['num']}/10] Verified: {t['title']} ({size} bytes) -> {t['file']}")

with open(MANIFEST_PATH, "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2)

print(f"\nAll 10/10 Round 1 Tracks verified and saved to {MANIFEST_PATH}")
