import json
import os

output_json = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\DHUN_CALIBRATION\round_03_fresh_10_song_selection.json"

selection = {
    "calibration_round": 3,
    "batch_title": "Round 3 Fresh 10-Song Stress-Test Selection (Songs 21–30)",
    "auditor": "pro_dj_auditor",
    "strict_exclusion_check": {
        "regression_library_size": 30,
        "regression_tracks_excluded": [
            "Animals", "Titanium", "Don't Start Now", "Ghungroo", "Zingaat",
            "Malhari", "Big Dawgs", "Blinding Lights", "Chaiyya Chaiyya", "Perfect",
            "Badtameez Dil", "Mundian To Bach Ke", "FE!N", "Mwaki", "Chogada",
            "Tremor", "Levitating", "Afreen Afreen", "Baddadan", "Tum Hi Ho",
            "Gallan Goodiyaan", "Proper Patola", "Gasolina", "Pepas", "Sauda Khara Khara",
            "Kesariya", "Losing It", "Sicko Mode", "Dancin", "Raataan Lambiyan"
        ],
        "reference_corpus_excluded": True,
        "batch_1_legacy_excluded": True,
        "overlap_count": 0,
        "isolation_status": "STRICT_ISOLATION_CONFIRMED"
    },
    "tracks": [
        {
            "track_number": 21,
            "track_id": "mnike",
            "title": "Mnike",
            "artist": "Tyler ICU & Tumelo.za ft. Nandipha808, Ceeka RSA",
            "search_query": "Tyler ICU Mnike Official Music Video",
            "bpm": 113.0,
            "key": "F# Minor",
            "camelot": "11A",
            "meter": "4/4 Amapiano Syncopated Log-Drum Groove",
            "energy_level": 8,
            "specific_challenge": "Iconic South African log-drum bass stabs (45-90 Hz) with heavy syncopated sub-pitches; vocal chants trading offbeat accents; tests log-drum transient preservation and phase alignment."
        },
        {
            "track_number": 22,
            "track_id": "two_nine_five",
            "title": "295",
            "artist": "Sidhu Moose Wala",
            "search_query": "Sidhu Moose Wala 295 Official Audio Moosetape",
            "bpm": 90.0,
            "key": "E Minor",
            "camelot": "9A",
            "meter": "4/4 Heavy Hip-Hop Dholak Groove",
            "energy_level": 9,
            "specific_challenge": "Deep emotive vocal rap delivery with intense lyrical cadence; heavy punchy 808 sub and acoustic dholak; tests vocal phrase completion and low-end separation from modern club kicks."
        },
        {
            "track_number": 23,
            "track_id": "doja",
            "title": "Doja",
            "artist": "Central Cee",
            "search_query": "Central Cee Doja Official Music Video",
            "bpm": 140.0,
            "key": "G# Minor",
            "camelot": "1A",
            "meter": "4/4 UK Drill Sliding 808 / Syncopated Snare",
            "energy_level": 8,
            "specific_challenge": "Signature UK drill sliding 808 sub-bass, rapid British slang flow with anacrusis pickups; tests drill triplet hi-hat alignment and sub-bass pitch glide management."
        },
        {
            "track_number": 24,
            "track_id": "great_spirit",
            "title": "Great Spirit",
            "artist": "Armin van Buuren vs Vini Vici ft. Hilight Tribe",
            "search_query": "Armin van Buuren vs Vini Vici Great Spirit Official Music Video",
            "bpm": 138.0,
            "key": "F# Minor",
            "camelot": "11A",
            "meter": "4/4 Driving Psytrance Triplet Rolling Bassline",
            "energy_level": 10,
            "specific_challenge": "High-speed rolling psytrance bassline (triplet 16th sub stabs at 138 BPM) with tribal indigenous vocal chants; colossal build-up; tests rapid bassline transient separation and build-up immunity."
        },
        {
            "track_number": 25,
            "track_id": "kar_gayi_chull",
            "title": "Kar Gayi Chull",
            "artist": "Badshah, Fazilpuria, Sukriti Kakar, Neha Kakkar",
            "search_query": "Kar Gayi Chull Kapoor & Sons Official Video Badshah",
            "bpm": 125.0,
            "key": "G# Minor",
            "camelot": "1A",
            "meter": "4/4 Commercial House Dhol Beat",
            "energy_level": 9,
            "specific_challenge": "Rapid conversational vocal trades; bouncy club house kick layered with acoustic dholak; tests rapid vocal ducking and speech formant separation."
        },
        {
            "track_number": 26,
            "track_id": "mi_gente",
            "title": "Mi Gente",
            "artist": "J Balvin, Willy William",
            "search_query": "J Balvin Willy William Mi Gente Official Music Video",
            "bpm": 105.0,
            "key": "A Minor",
            "camelot": "8A",
            "meter": "4/4 Moombahton Trombone Hook / Tresillo Drop",
            "energy_level": 9,
            "specific_challenge": "Famous synthesized trombone pitch hook; syncopated Moombahton 105 BPM groove; tests vocal chop handover and midrange trumpet/trombone frequency carving."
        },
        {
            "track_number": 27,
            "track_id": "ghoomar",
            "title": "Ghoomar",
            "artist": "Shreya Ghoshal, Swaroop Khan",
            "search_query": "Ghoomar Padmaavat Official Video Song Deepika Padukone Shreya Ghoshal",
            "bpm": 116.0,
            "key": "A Major",
            "camelot": "11B",
            "meter": "6/8 Compound Rajasthani Nagada & Dholak Folk",
            "energy_level": 8,
            "specific_challenge": "Authentic Rajasthani royal nagada and dholak in compound 6/8 meter with soaring folk alaap by Swaroop Khan; strictly forbids straight 4/4 crossfading; tests measure-boundary taan drop-swaps and compound metric modulation."
        },
        {
            "track_number": 28,
            "track_id": "opus",
            "title": "Opus",
            "artist": "Eric Prydz",
            "search_query": "Eric Prydz Opus Official Music Video",
            "bpm": 126.0,
            "key": "F Minor",
            "camelot": "4A",
            "meter": "4/4 Progressive House Accelerating Arp",
            "energy_level": 9,
            "specific_challenge": "Legendary 3-minute accelerating synthesizer arpeggio build-up; tests strict build-up immunity and master dynamic headroom as the track swells from a whisper to a stadium-sized wall of sound."
        },
        {
            "track_number": 29,
            "track_id": "kun_faya_kun",
            "title": "Kun Faya Kun",
            "artist": "A.R. Rahman, Javed Ali, Mohit Chauhan",
            "search_query": "Kun Faya Kun Rockstar Official Full Song A R Rahman",
            "bpm": 78.0,
            "key": "D Major",
            "camelot": "10B",
            "meter": "4/4 Slow Spiritual Sufi Tabla & Harmonium",
            "energy_level": 6,
            "specific_challenge": "Deep spiritual Sufi atmosphere, delicate acoustic guitar and harmonium, sacred devotional vocal entries; tests Breakdown Alaap Sanctity (never cut mid-phrase) and microtonal shruti preservation."
        },
        {
            "track_number": 30,
            "track_id": "solar_system",
            "title": "Solar System",
            "artist": "Sub Focus",
            "search_query": "Sub Focus Solar System Official Music Video",
            "bpm": 174.0,
            "key": "G# Minor",
            "camelot": "1A",
            "meter": "4/4 High-Speed Drum & Bass Roll",
            "energy_level": 10,
            "specific_challenge": "Blistering 174 BPM two-step breakbeat, euphoric synth lead, deep rolling Reese sub-bass; tests half-time octave transitions (174 <-> 87 BPM) and transient attack sharpness on line arrays."
        }
    ],
    "challenge_pairs": [
        {
            "pair_id": "pair_1",
            "title": "Metric Modulation / Extreme BPM Jump",
            "outgoing_track": "23. Doja (70 / 140 BPM UK Drill, 1A)",
            "incoming_track": "24. Great Spirit (138 BPM Psytrance, 11A)",
            "challenge_description": "Jumping from a 70 BPM half-time UK drill sliding 808 groove into a blistering 138 BPM rolling triplet psytrance drop (+97% tempo jump). Requires 2:1 octave metric modulation or dynamic 4-bar elastic ramp (transients=crisp), plus 48 dB/oct HPF at 180 Hz to evacuate the drill 808 sub-bass before the 138 BPM psytrance kick and rolling bassline drop.",
            "success_criteria": "Zero kick galloping; strict sub-bass handover before rolling psytrance bassline drops; razor-sharp transient preservation."
        },
        {
            "pair_id": "pair_2",
            "title": "Advanced Vocal Safety & Extended Anacrusis Protection",
            "outgoing_track": "22. 295 (90 BPM Punjabi Hip-Hop, 9A)",
            "incoming_track": "26. Mi Gente (105 BPM Moombahton, 8A)",
            "challenge_description": "Sidhu Moose Wala's emotional, dense Punjabi rap cadence resolving on Bar 8, immediately met by J Balvin's iconic vocal anacrusis pickup hook ('Si tú te mueves mi gente no se detiene...') starting on Beat 4.5 (350ms before downbeat).",
            "success_criteria": "Lookahead vocal ducking (-6 dB at 1.8 kHz) on outgoing track; Spanish vocal pickup speaks in crystalline isolation; zero lyrical collision; clean transition from Punjabi dholak into Latin Dembow."
        },
        {
            "pair_id": "pair_3",
            "title": "Cross-Meter / Polyrhythmic Groove Hand-off",
            "outgoing_track": "27. Ghoomar (116 BPM 6/8 Rajasthani Folk, 11B)",
            "incoming_track": "21. Mnike (113 BPM 4/4 Amapiano, 11A)",
            "challenge_description": "Rajasthani compound 6/8 royal nagada/dholak rhythm transitioning into a syncopated 4/4 South African Amapiano log-drum groove. An unaligned crossfade causes catastrophic polyrhythmic galloping.",
            "success_criteria": "Bans unaligned multi-bar crossfades; employs measure-boundary taan drop-swap on Bar 8 Beat 4 combined with a 3:2 metric delay pulse bridge (531 ms) before dropping the deep Amapiano log-drum kick on Beat 1."
        },
        {
            "pair_id": "pair_4",
            "title": "Harmonic Energy Shift / Complex Camelot Carve",
            "outgoing_track": "25. Kar Gayi Chull (125 BPM, 1A / G# Minor)",
            "incoming_track": "28. Opus (126 BPM, 4A / F Minor)",
            "challenge_description": "1A (G# Minor) to 4A (F Minor) is a 3-step minor-third Camelot distance with intense dynamic contrast: high-energy commercial Bollywood house transitioning into an expansive, minimalist progressive synth arp. Clashing basslines and chord extensions will produce dissonant minor thirds.",
            "success_criteria": "Full-spectrum frequency evacuation: 48 dB/oct HPF at 240 Hz and -16 dB notch carving on 1A fundamentals, or a clean friction backspin drop-swap on Beat 1."
        },
        {
            "pair_id": "pair_5",
            "title": "Dynamic Energy Polarity (Ballad/Atmospheric to Peak Festival Drop)",
            "outgoing_track": "29. Kun Faya Kun (78 BPM Sacred Sufi Qawwali, 10B)",
            "incoming_track": "30. Solar System (174 / 87 BPM Drum & Bass, 1A)",
            "challenge_description": "Moving from an intimate, devotional acoustic Sufi Qawwali with harmonium and acoustic guitar into a 174 BPM peak-time drum & bass explosion.",
            "success_criteria": "Natural phrase decay to -40 dBFS, synthesized tension riser sweep and pre-drop breath silence, and explosive Beat 1 detonation with zero digital clipping (<= -1.0 dBTP)."
        }
    ],
    "continuous_mix_arc": {
        "title": "Round 3 Continuous 10-Track Master Set Arc (Tracks 21–30)",
        "progression": [
            {"order": 1, "track_id": "kun_faya_kun", "bpm": 78.0, "role": "Sacred atmospheric Sufi opening; soul-stirring harmonium & alaap."},
            {"order": 2, "track_id": "two_nine_five", "bpm": 90.0, "role": "Emotive Punjabi hip-hop groove pickup; heavy 808 & dholak."},
            {"order": 3, "track_id": "mi_gente", "bpm": 105.0, "role": "Global Latin Moombahton pickup; infectious trombone hook sing-along."},
            {"order": 4, "track_id": "mnike", "bpm": 113.0, "role": "South African Amapiano groove transition; syncopated log-drum power."},
            {"order": 5, "track_id": "ghoomar", "bpm": 116.0, "role": "Cross-meter Rajasthani royal folk explosion; live nagada & dholak."},
            {"order": 6, "track_id": "kar_gayi_chull", "bpm": 125.0, "role": "Bouncy commercial Bollywood house banger; collective sing-along."},
            {"order": 7, "track_id": "opus", "bpm": 126.0, "role": "Progressive house build-up; soaring hypnotic synth melody."},
            {"order": 8, "track_id": "great_spirit", "bpm": 138.0, "role": "High-speed psytrance rolling bass climax; tribal festival frenzy."},
            {"order": 9, "track_id": "doja", "bpm": 140.0, "role": "Half-time UK drill switch; sliding 808 sub-bass pressure."},
            {"order": 10, "track_id": "solar_system", "bpm": 174.0, "role": "High-octane drum & bass grand finale; euphoric line-array climax."}
        ]
    }
}

os.makedirs(os.path.dirname(output_json), exist_ok=True)
with open(output_json, "w", encoding="utf-8") as f:
    json.dump(selection, f, indent=2)

print(f"Saved Round 3 selection JSON to {output_json}")
