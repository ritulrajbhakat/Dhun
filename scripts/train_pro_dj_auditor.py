import os
import json

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CALIBRATION_DIR = os.path.join(BASE_DIR, "DHUN_CALIBRATION")
JSON_PATH = os.path.join(CALIBRATION_DIR, "reference_discovery_100_resolved.json")

with open(JSON_PATH, "r", encoding="utf-8") as f:
    refs = json.load(f)

print(f"Loaded {len(refs)} references.")

# Categorize references
categories = {
    "bollywood_mashup_house_party": [],
    "desi_regional_club_percussion": [],
    "indian_melodic_techno_afro": [],
    "boiler_room_underground": [],
    "cercle_cinematic_deep": [],
    "festival_mainstage": [],
    "groove_house_disco_afro_tech": []
}

for r in refs:
    num = r["num"]
    title = (r.get("resolved_title") or r.get("lead_title") or "").lower()
    uploader = (r.get("uploader") or "").lower()
    q = (r.get("search_query") or "").lower()
    combined = f"{title} {uploader} {q}"

    if any(k in combined for k in ["sush & yohan", "sush and yohan", "chetas", "ganesh", "tushar", "diwali", "bun maska", "bastian", "house party"]):
        categories["bollywood_mashup_house_party"].append(r)
    elif any(k in combined for k in ["dalal", "south", "kuthu", "marathi", "garba", "dandiya", "punjabi", "bhangra", "sangeet", "wedding", "baraat", "haldi"]):
        categories["desi_regional_club_percussion"].append(r)
    elif any(k in combined for k in ["nyk", "aqeel", "adhunyk", "lost stories", "zaeden", "anish sood", "sartek", "melodic techno", "saga"]):
        categories["indian_melodic_techno_afro"].append(r)
    elif any(k in combined for k in ["boiler room", "fred again", "sara landry", "chase & status", "alex kade", "360"]):
        categories["boiler_room_underground"].append(r)
    elif any(k in combined for k in ["cercle", "anjunadeep", "einmusik", "afterlife", "anyma"]):
        categories["cercle_cinematic_deep"].append(r)
    elif any(k in combined for k in ["tomorrowland", "ultra", "guetta", "calvin harris", "festival", "kshmr", "nucleya"]):
        categories["festival_mainstage"].append(r)
    else:
        categories["groove_house_disco_afro_tech"].append(r)

print("\n--- Category Breakdown ---")
for cat, items in categories.items():
    print(f"  {cat}: {len(items)} references")

# Compile summary analysis
summary = {
    "total_references": len(refs),
    "resolved_count": sum(1 for r in refs if r.get("status") in ["available", "resolved"]),
    "categories_count": {cat: len(items) for cat, items in categories.items()},
    "direct_links_count": 17,
    "search_queries_count": 83,
    "all_references": refs
}

summary_path = os.path.join(CALIBRATION_DIR, "reference_discovery_summary.json")
with open(summary_path, "w", encoding="utf-8") as f:
    json.dump(summary, f, indent=2)

print(f"\nSummary saved to: {summary_path}")
