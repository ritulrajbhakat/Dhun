import os
import sys
import json
import time

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
VENV_PYTHON = os.path.join(BASE_DIR, "resolver-service", ".venv", "Scripts", "python.exe")
CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
os.makedirs(CALIBRATION_DIR, exist_ok=True)

DIRECT_REFS = [
    {"num": 1, "lead": "User-supplied reference (video identity not resolved)", "url": "https://youtu.be/2TmiTSfVmmw"},
    {"num": 2, "lead": "Sush & Yohan — Diwali House Party Set 2026", "url": "https://youtu.be/7EeWZF2uGhI"},
    {"num": 3, "lead": "Sush & Yohan — Bun Maska Vol. 2", "url": "https://youtu.be/57J5UYoaXrc"},
    {"num": 4, "lead": "Sush & Yohan — 50 Mins of Chatpate Bollywood Bangers (live)", "url": "https://youtu.be/lQsH8kVKoxs"},
    {"num": 5, "lead": "User-supplied reference (video identity not resolved)", "url": "https://youtu.be/_jG0gSsTv_8"},
    {"num": 6, "lead": "Sush & Yohan — Bun Maska Vol. 1", "url": "https://youtu.be/f1l0UkVJQxk"},
    {"num": 7, "lead": "Sush & Yohan — Making Bollywood Nights Cool Again (Sazio live set)", "url": "https://youtu.be/Vo3-bmkC4Pw"},
    {"num": 8, "lead": "Sush & Yohan — 2025 Year End Megamix", "url": "https://youtu.be/eJw625yYmH4"},
    {"num": 9, "lead": "DJ Ganesh India — Live at Bastian, Mumbai (Bollywood mashups)", "url": "https://youtu.be/5pQR-QsCcdY"},
    {"num": 10, "lead": "DJ Dalal London — Non Stop Bollywood & South Party Mix (live club set)", "url": "https://youtu.be/fM6cF_xni7w"},
    {"num": 11, "lead": "DJ Dalal London — Retro Vs Techno (non-stop mix)", "url": "https://youtu.be/oNbQJdqGu6s"},
    {"num": 12, "lead": "DJ Tushar — Bollywood Party Mix", "url": "https://youtu.be/PnOAYZV_FS4"},
    {"num": 13, "lead": "DJ NYK — Bollywood Techno & Afro House Mix (Adhunyk Lab)", "url": "https://youtu.be/Dt9mhc4Rr4Q"},
    {"num": 14, "lead": "DJ Aqeel — Bollywood Techno Day live set", "url": "https://youtu.be/UiwCejCHM5w"},
    {"num": 15, "lead": "ReMan — Afro & Tech House live at Saga Festival", "url": "https://youtu.be/wu1w36gjXYc"},
    {"num": 16, "lead": "DJ Latitude — Afrohouse full set", "url": "https://youtu.be/22kAxuOx0Vo"},
    {"num": 17, "lead": "Alex Kade — 360° multi-genre Boiler Room-style live set", "url": "https://youtu.be/yzQLn9G8B5I"}
]

SEARCH_REFS = [
    (18, "Sush and Yohan live DJ set Bollywood mashups"),
    (19, "Sush and Yohan Diwali house party set"),
    (20, "Sush and Yohan Bollywood throwbacks live set"),
    (21, "Sush and Yohan 50 minutes Bollywood bangers live"),
    (22, "Sush and Yohan Bollywood house party mixtape"),
    (23, "DJ NYK Bollywood live DJ set full mix"),
    (24, "DJ NYK Adhunyk Bollywood techno Afro house set"),
    (25, "DJ NYK Bollywood melodic techno live set"),
    (26, "DJ Chetas Bollywood live DJ set full"),
    (27, "DJ Aqeel Bollywood live DJ set full mix"),
    (28, "DJ Ganesh India Bollywood live DJ set Mumbai"),
    (29, "DJ Dalal London Bollywood South Indian live DJ set"),
    (30, "DJ Dalal London retro Bollywood techno mix"),
    (31, "DJ Dalal London Bollywood party nonstop DJ mix"),
    (32, "DJ Shilpi Sharma Bollywood live DJ set"),
    (33, "DJ Ravish Bollywood live DJ set"),
    (34, "DJ Rav UK Bollywood live DJ set"),
    (35, "VDJ Mahe Bollywood South Indian DJ party mix"),
    (36, "DJ Dalal London Bollywood item songs live mix"),
    (37, "Bollywood DJ set Mumbai club live full set"),
    (38, "Bollywood DJ set Goa beach club live"),
    (39, "Bollywood DJ set rooftop party India live"),
    (40, "Bollywood wedding DJ set live full mix"),
    (41, "Bollywood house party DJ set nostalgic mashups"),
    (42, "Bollywood retro vs techno DJ mix"),
    (43, "Bollywood x Afro house DJ set"),
    (44, "Bollywood x tech house DJ set"),
    (45, "Bollywood x melodic techno DJ set"),
    (46, "Bollywood x EDM festival DJ set"),
    (47, "Bollywood Punjabi mashup DJ set live"),
    (48, "Punjabi Bollywood party DJ set full mix"),
    (49, "Desi hip hop Bollywood mashup DJ set"),
    (50, "South Indian kuthu Bollywood DJ set live"),
    (51, "Marathi Bollywood DJ set live mashup"),
    (52, "Gujarati garba DJ set live mix"),
    (53, "Tamil Telugu Hindi DJ mashup live set"),
    (54, "Indian wedding sangeet DJ set Bollywood mashups"),
    (55, "Indian club DJ set Bollywood house drops"),
    (56, "Desi house music DJ set live"),
    (57, "Indian DJ set crowd transitions Bollywood live"),
    (58, "DJ NYK Indian techno DJ set"),
    (59, "Indian melodic techno DJ set live"),
    (60, "Indian Afro house DJ set live"),
    (61, "Indian tech house DJ set full"),
    (62, "Indian progressive house DJ set live"),
    (63, "Indian EDM festival DJ set full"),
    (64, "Lost Stories live DJ set India"),
    (65, "Nucleya live DJ set full performance"),
    (66, "KSHMR live DJ set India full set"),
    (67, "Zaeden live DJ set India"),
    (68, "Anish Sood live DJ set India"),
    (69, "Sartek live DJ set India"),
    (70, "Progressive house Bollywood mashup DJ set"),
    (71, "Indian electronic music DJ set live club"),
    (72, "Desi Afro house DJ mix live"),
    (73, "Boiler Room DJ set house full set"),
    (74, "Boiler Room DJ set techno full set"),
    (75, "Boiler Room DJ set UK garage full set"),
    (76, "Boiler Room DJ set drum and bass full set"),
    (77, "Boiler Room DJ set disco house full set"),
    (78, "Boiler Room DJ set bass house full set"),
    (79, "Cercle DJ set melodic house full set"),
    (80, "Cercle DJ set afro house full set"),
    (81, "Cercle DJ set progressive house full set"),
    (82, "Cercle DJ set techno full set"),
    (83, "Mixmag Lab DJ set house full set"),
    (84, "Mixmag Lab DJ set techno full set"),
    (85, "Mixmag Lab DJ set drum and bass full set"),
    (86, "DJ Mag festival full set house"),
    (87, "DJ Mag festival full set techno"),
    (88, "Tomorrowland full DJ set 2025"),
    (89, "Tomorrowland full DJ set 2024"),
    (90, "Ultra Music Festival full DJ set live"),
    (91, "HÖR Berlin DJ set house"),
    (92, "HÖR Berlin DJ set techno"),
    (93, "Anjunadeep full DJ set live"),
    (94, "Defected Records DJ set live house"),
    (95, "Glitterbox DJ set live disco house"),
    (96, "Afterlife melodic techno full DJ set"),
    (97, "Keinemusik live DJ set full"),
    (98, "Black Coffee live DJ set full"),
    (99, "Shimza Afro house live DJ set"),
    (100, "Caiiro Afro house live DJ set")
]

def main():
    import yt_dlp

    ydl_opts = {
        'quiet': True,
        'no_warnings': True,
        'extract_flat': 'in_playlist',
        'skip_download': True,
    }

    resolved_records = []
    print(f"Starting resolution of all 100 references...")

    # Phase 1: Direct Links (1-17)
    print("\n--- PHASE 1: Resolving Direct Links (1–17) ---")
    for item in DIRECT_REFS:
        num = item["num"]
        url = item["url"]
        lead = item["lead"]
        print(f"[{num:03d}/100] Direct: {url} ({lead})")
        try:
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                info = ydl.extract_info(url, download=False)
                record = {
                    "num": num,
                    "type": "direct_url",
                    "lead_title": lead,
                    "requested_url": url,
                    "resolved_id": info.get("id"),
                    "resolved_url": f"https://www.youtube.com/watch?v={info.get('id')}",
                    "resolved_title": info.get("title"),
                    "uploader": info.get("uploader") or info.get("channel"),
                    "channel_id": info.get("channel_id"),
                    "duration_sec": info.get("duration"),
                    "view_count": info.get("view_count"),
                    "status": "available",
                    "is_full_set_or_mix": True if info.get("duration", 0) > 600 else False,
                    "description_preview": (info.get("description") or "")[:200].replace("\n", " ")
                }
                print(f"   -> RESOLVED: '{record['resolved_title']}' by {record['uploader']} ({record['duration_sec']}s)")
                resolved_records.append(record)
        except Exception as e:
            err_str = str(e)
            print(f"   -> FAILED / UNAVAILABLE: {err_str[:100]}")
            resolved_records.append({
                "num": num,
                "type": "direct_url",
                "lead_title": lead,
                "requested_url": url,
                "status": f"unavailable: {err_str[:80]}",
                "is_full_set_or_mix": False
            })

    # Phase 2: Search Queries (18-100)
    print("\n--- PHASE 2: Resolving YouTube Discovery Searches (18–100) ---")
    for num, query in SEARCH_REFS:
        search_query = f"ytsearch1:{query}"
        print(f"[{num:03d}/100] Searching: '{query}'")
        try:
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                res = ydl.extract_info(search_query, download=False)
                entries = res.get("entries") or []
                if entries:
                    info = entries[0]
                    record = {
                        "num": num,
                        "type": "search_resolved",
                        "search_query": query,
                        "requested_url": f"https://www.youtube.com/results?search_query={query.replace(' ', '+')}",
                        "resolved_id": info.get("id"),
                        "resolved_url": f"https://www.youtube.com/watch?v={info.get('id')}",
                        "resolved_title": info.get("title"),
                        "uploader": info.get("uploader") or info.get("channel"),
                        "channel_id": info.get("channel_id"),
                        "duration_sec": info.get("duration"),
                        "view_count": info.get("view_count"),
                        "status": "resolved",
                        "is_full_set_or_mix": True if info.get("duration", 0) > 600 else False,
                        "description_preview": (info.get("description") or "")[:200].replace("\n", " ")
                    }
                    print(f"   -> RESOLVED: '{record['resolved_title']}' by {record['uploader']} ({record['duration_sec']}s)")
                    resolved_records.append(record)
                else:
                    print(f"   -> NO RESULTS for query: '{query}'")
                    resolved_records.append({
                        "num": num,
                        "type": "search_resolved",
                        "search_query": query,
                        "status": "no_results_found"
                    })
        except Exception as e:
            print(f"   -> ERROR on '{query}': {e}")
            resolved_records.append({
                "num": num,
                "type": "search_resolved",
                "search_query": query,
                "status": f"error: {str(e)[:80]}"
            })

    out_file = os.path.join(CALIBRATION_DIR, "reference_discovery_100_resolved.json")
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(resolved_records, f, indent=2)

    print(f"\nAll 100 references processed! Total records: {len(resolved_records)}")
    print(f"Saved to: {out_file}")

if __name__ == "__main__":
    main()
