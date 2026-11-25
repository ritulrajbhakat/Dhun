import re
import json

with open(r'dhun/src-tauri/src/dj/moods.rs', 'r', encoding='utf-8') as f:
    text = f.read()

# Extract mood definitions
pattern = r'id:\s*"([^"]+)"\.to_string\(\),\s*display_name:\s*"([^"]+)"\.to_string\(\),\s*description:\s*"([^"]+)"\.to_string\(\),\s*min_bpm:\s*([0-9.]+),\s*max_bpm:\s*([0-9.]+),\s*target_energy:\s*([0-9.]+),\s*transition_style:\s*"([^"]+)"\.to_string\(\),\s*repetition_cooldown_mins:\s*([0-9]+),\s*artist_cooldown_mins:\s*([0-9]+)'
matches = re.findall(pattern, text)

print("=" * 70)
print(f"DISCOVERED {len(matches)} PRESET MOODS IN DHUN PRODUCTION ENGINE")
print("=" * 70)

moods = []
for m in matches:
    mood_info = {
        "id": m[0],
        "display_name": m[1],
        "description": m[2],
        "min_bpm": float(m[3]),
        "max_bpm": float(m[4]),
        "target_energy": float(m[5]),
        "transition_style": m[6],
        "repetition_cooldown_mins": int(m[7]),
        "artist_cooldown_mins": int(m[8])
    }
    moods.append(mood_info)
    print(f"[{m[0]:<16}] {m[1]:<20} | BPM: {m[3]:>5}-{m[4]:<5} | Energy: {m[5]:<4} | Style: {m[6]}")

with open(r'dhun/DHUN_CALIBRATION/discovered_production_moods.json', 'w', encoding='utf-8') as out:
    json.dump(moods, out, indent=2)

print("\nSaved discovered moods to dhun/DHUN_CALIBRATION/discovered_production_moods.json")
