import json
import os

output_json = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\DHUN_CALIBRATION\round_02_fresh_10_song_selection.json"

selection = {
    "calibration_round": 2,
    "batch_title": "Round 2 Fresh 10-Song Stress-Test Selection (Songs 11–20)",
    "auditor": "pro_dj_auditor",
    "strict_exclusion_check": {
        "regression_library_size": 20,
        "regression_tracks_excluded": [
            "Animals", "Titanium", "Don't Start Now", "Ghungroo", "Zingaat",
            "Malhari", "Big Dawgs", "Blinding Lights", "Chaiyya Chaiyya", "Perfect",
            "Badtameez Dil", "Mundian To Bach Ke", "FE!N", "Mwaki", "Chogada",
            "Tremor", "Levitating", "Afreen Afreen", "Baddadan", "Tum Hi Ho"
        ],
        "reference_corpus_excluded": True,
        "batch_1_legacy_excluded": True,
        "overlap_count": 0,
        "isolation_status": "STRICT_ISOLATION_CONFIRMED"
    },
    "tracks": [
        {
            "track_number": 11,
            "track_id": "gallan_goodiyaan",
            "title": "Gallan Goodiyaan",
            "artist": "Yashita Sharma, Manish Kumar Tipu, Farhan Akhtar, Shankar Mahadevan, Sukhwinder Singh",
            "search_query": "Gallan Goodiyaan Official Full Song Dil Dhadakne Do T-Series",
            "bpm": 128.0,
            "key": "A Major",
            "camelot": "11B",
            "meter": "4/4 Fast Swing / Dholak Group Vocal",
            "energy_level": 9,
            "specific_challenge": "Massive group vocal overlapping with sudden big-band horn fanfare; multiple singers trading lines every 2 bars; requires strict phrase boundary detection and mid-frequency EQ carving."
        },
        {
            "track_number": 12,
            "track_id": "proper_patola",
            "title": "Proper Patola",
            "artist": "Diljit Dosanjh, Badshah, Aastha Gill",
            "search_query": "Proper Patola Namaste England Official Song Diljit Badshah",
            "bpm": 95.0,
            "key": "F Minor",
            "camelot": "4A",
            "meter": "4/4 Heavy Urban Punjabi Bounce",
            "energy_level": 8,
            "specific_challenge": "Heavy acoustic dhol roll mixed with an 808 sub-kick; rap verse entering with an offbeat vocal pickup; requires anacrusis protection and sub-bass separation from classic dhol."
        },
        {
            "track_number": 13,
            "track_id": "gasolina",
            "title": "Gasolina",
            "artist": "Daddy Yankee",
            "search_query": "Daddy Yankee Gasolina Official Music Video",
            "bpm": 96.0,
            "key": "F Minor",
            "camelot": "4A",
            "meter": "4/4 Syncopated Dembow Rhythm (3:3:2 Tresillo)",
            "energy_level": 9,
            "specific_challenge": "Iconic Dembow syncopation (dotted eighth-eighth-sixteenth pattern); rapid Puerto Rican rap flow; tests whether DHUN can align the syncopated Dembow kick/snare pattern against straight 4/4 or swing without rhythmic galloping."
        },
        {
            "track_number": 14,
            "track_id": "pepas",
            "title": "Pepas",
            "artist": "Farruko",
            "search_query": "Farruko Pepas Official Music Video",
            "bpm": 130.0,
            "key": "B Minor",
            "camelot": "10A",
            "meter": "4/4 Latin Guaracha Tribal Kick & Trumpet Lead",
            "energy_level": 10,
            "specific_challenge": "Rapid acceleration from a 96 BPM reggaeton-style vocal bridge into an explosive 130 BPM Latin Guaracha tribal trumpet drop; tests build-up immunity and intense acoustic trumpet frequency management (2-6 kHz)."
        },
        {
            "track_number": 15,
            "track_id": "sauda_khara_khara",
            "title": "Sauda Khara Khara",
            "artist": "Diljit Dosanjh, Sukhbir, Dhvani Bhanushali",
            "search_query": "Sauda Khara Khara Good Newwz Diljit Sukhbir Zee Music",
            "bpm": 132.0,
            "key": "E Minor",
            "camelot": "9A",
            "meter": "4/4 Driving Bhangra Club Four-on-the-Floor",
            "energy_level": 9,
            "specific_challenge": "Legendary Punjabi chant layered over a driving 132 BPM electro-dhol groove; tests vocal call-and-response phrase alignment and high-energy handover."
        },
        {
            "track_number": 16,
            "track_id": "kesariya",
            "title": "Kesariya",
            "artist": "Pritam, Arijit Singh",
            "search_query": "Kesariya Brahmastra Arijit Singh Pritam Official Video",
            "bpm": 94.0,
            "key": "F Major",
            "camelot": "7B",
            "meter": "6/8 Compound Indian Folk / Dadra feel",
            "energy_level": 7,
            "specific_challenge": "Subtle compound 6/8 acoustic sitar and tabla groove with microtonal vocal runs; transitions into/out of straight 4/4 without galloping; requires metric modulation and alaap sanctity."
        },
        {
            "track_number": 17,
            "track_id": "losing_it",
            "title": "Losing It",
            "artist": "FISHER",
            "search_query": "FISHER Losing It Official Music Video",
            "bpm": 125.0,
            "key": "F Minor",
            "camelot": "4A",
            "meter": "4/4 Driving Tech House Four-on-the-Floor",
            "energy_level": 10,
            "specific_challenge": "Massive rising foghorn synth bassline (80-300 Hz) that monopolizes the entire mix; requires aggressive dynamic carving and single-deck sub dominance to prevent catastrophic subwoofer distortion."
        },
        {
            "track_number": 18,
            "track_id": "sicko_mode",
            "title": "Sicko Mode",
            "artist": "Travis Scott ft. Drake",
            "search_query": "Travis Scott SICKO MODE Official Music Video Drake",
            "bpm": 155.0,
            "key": "B Minor",
            "camelot": "10A",
            "meter": "4/4 Half-Time Trap",
            "energy_level": 9,
            "specific_challenge": "Multiple abrupt structural beat-switches within the track itself; Drake's opening anacrusis verse ('Sun is down, freezin' cold...'); deep distorted 808 glides."
        },
        {
            "track_number": 19,
            "track_id": "dancin_krono",
            "title": "Dancin (Krono Remix)",
            "artist": "Aaron Smith ft. Luvli",
            "search_query": "Aaron Smith Dancin Krono Remix Official Video",
            "bpm": 120.0,
            "key": "F# Minor",
            "camelot": "11A",
            "meter": "4/4 Smooth Melodic House",
            "energy_level": 8,
            "specific_challenge": "Ultra-delicate female vocal with intimate breath pickups; warm analog sub-bass line; wide stereo imaging; tests vocal preservation and subtle sinusoidal equal-power blending."
        },
        {
            "track_number": 20,
            "track_id": "raataan_lambiyan",
            "title": "Raataan Lambiyan",
            "artist": "Tanishk Bagchi, Jubin Nautiyal, Asees Kaur",
            "search_query": "Raataan Lambiyan Shershaah Jubin Asees Official Video",
            "bpm": 78.0,
            "key": "F# Major",
            "camelot": "2B",
            "meter": "4/4 Slow Intimate Acoustic Ballad",
            "energy_level": 5,
            "specific_challenge": "Soft acoustic guitar arpeggios, whispering male/female dual vocals, tender rubato pauses, deep dynamic contrast; tests energy cliff bridging and ambient reverb halo dissolves."
        }
    ],
    "challenge_pairs": [
        {
            "pair_id": "pair_1",
            "title": "Metric Modulation / Extreme BPM Jump",
            "outgoing_track": "18. Sicko Mode (77.5 / 155 BPM Trap, 10A)",
            "incoming_track": "14. Pepas (130 BPM Latin Guaracha, 10A)",
            "challenge_description": "Jumping from 77.5 BPM half-time dark trap into a high-octane 130 BPM Latin tribal Guaracha drop (+68% tempo jump). Key is locked (10A -> 10A / B Minor), forcing pure rhythmic/metric modulation (dynamic elastic ramp, 3:4 metric delay bridge at 461.5 ms, or friction backspin drop on Beat 1).",
            "success_criteria": "Zero kick galloping; strict sub-bass handover before Guaracha kick drops; smooth transient preservation."
        },
        {
            "pair_id": "pair_2",
            "title": "Advanced Vocal Safety & Extended Anacrusis Protection",
            "outgoing_track": "12. Proper Patola (95 BPM Punjabi, 4A)",
            "incoming_track": "13. Gasolina (96 BPM Dembow, 4A)",
            "challenge_description": "Harmonic lock (4A -> 4A / F Minor) with near-matched tempo (95 -> 96 BPM), but severe vocal collision risk: Badshah's Punjabi rap verse ending precisely as Daddy Yankee's iconic Spanish anacrusis hook ('Zúmbale mambo...') enters on an offbeat pickup.",
            "success_criteria": "Lookahead vocal ducking (-6 dB at 1.8 kHz) on outgoing track; Spanish vocal pickup speaks in crystalline isolation; zero lyrical collision; clean transition from Punjabi dholak into Latin Dembow."
        },
        {
            "pair_id": "pair_3",
            "title": "Cross-Meter / Polyrhythmic Groove Hand-off",
            "outgoing_track": "16. Kesariya (94 BPM 6/8 Compound Dadra, 7B)",
            "incoming_track": "19. Dancin (Krono Remix) (120 BPM 4/4 Deep House, 11A)",
            "challenge_description": "Compound 6/8 acoustic sitar/tabla groove transitioning into straight 4/4 deep house kick and bassline. Multi-bar crossfading causes catastrophic polyrhythmic galloping.",
            "success_criteria": "Bans unaligned multi-bar crossfades; employs measure-boundary drop-swap on Bar 8 Beat 4 or 3:2 metric delay pulse bridge (500 ms) before dropping straight 4/4 kick."
        },
        {
            "pair_id": "pair_4",
            "title": "Harmonic Energy Shift / Complex Camelot Carve",
            "outgoing_track": "15. Sauda Khara Khara (132 BPM, 9A / E Minor)",
            "incoming_track": "17. Losing It (125 BPM, 4A / F Minor)",
            "challenge_description": "9A (E Minor) to 4A (F Minor) is a half-step chromatic key shift (5 Camelot steps apart) with extreme sonic density: Losing It's aggressive 80-300 Hz foghorn bass clashes with Sauda Khara Khara's electro-dhol.",
            "success_criteria": "48 dB/oct HPF at 240 Hz + -16 dB notch carving on E Minor fundamentals, or authentic physical friction backspin drop on Beat 1. Zero harmonic rub or bass clashing."
        },
        {
            "pair_id": "pair_5",
            "title": "Dynamic Energy Polarity (Ballad to Peak Festival Drop)",
            "outgoing_track": "20. Raataan Lambiyan (78 BPM Acoustic Ballad, 2B)",
            "incoming_track": "11. Gallan Goodiyaan (128 BPM Wedding Swing Anthem, 11B)",
            "challenge_description": "Moving from an intimate, whispering acoustic guitar ballad into a thunderous 128 BPM multi-singer Bollywood brass explosion.",
            "success_criteria": "Natural phrase decay to -40 dBFS, synthesized tension riser fill and pre-drop breath silence, Beat 1 big-band brass drop with full line-array impact and zero digital clipping (<= -1.0 dBTP)."
        }
    ],
    "continuous_mix_arc": {
        "title": "Round 2 Continuous 10-Track Festival Master Mix (Tracks 11–20)",
        "progression": [
            {"order": 1, "track_id": "raataan_lambiyan", "bpm": 78.0, "role": "Intimate acoustic opening; romantic mood setter."},
            {"order": 2, "track_id": "kesariya", "bpm": 94.0, "role": "Melodic vocal lift; compound 6/8 sitar & tabla swing."},
            {"order": 3, "track_id": "proper_patola", "bpm": 95.0, "role": "Urban Punjabi bounce; dholak & 808 groove handover."},
            {"order": 4, "track_id": "gasolina", "bpm": 96.0, "role": "High-energy Latin Reggaeton dembow pickup; sing-along."},
            {"order": 5, "track_id": "dancin_krono", "bpm": 120.0, "role": "Smooth Deep House transition; sophisticated groove."},
            {"order": 6, "track_id": "losing_it", "bpm": 125.0, "role": "Peak-time tech-house energy peak; thunderous foghorn bass."},
            {"order": 7, "track_id": "gallan_goodiyaan", "bpm": 128.0, "role": "Explosive Bollywood Punjabi wedding anthem; collective frenzy."},
            {"order": 8, "track_id": "pepas", "bpm": 130.0, "role": "Latin Guaracha tribal peak; explosive trumpet drop."},
            {"order": 9, "track_id": "sauda_khara_khara", "bpm": 132.0, "role": "High-speed Bhangra electro climax; driving dhol stomp."},
            {"order": 10, "track_id": "sicko_mode", "bpm": 155.0, "role": "Dark hip-hop half-time trap finale; heavy 808 sub-bass resolution."}
        ]
    }
}

os.makedirs(os.path.dirname(output_json), exist_ok=True)
with open(output_json, "w", encoding="utf-8") as f:
    json.dump(selection, f, indent=2)

print(f"Saved Round 2 selection JSON to {output_json}")
