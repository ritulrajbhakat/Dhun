import json
import os

output_json = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\DHUN_CALIBRATION\round_05_fresh_10_song_selection.json"

selection = {
    "calibration_round": 5,
    "batch_title": "Pre-200-Song Validation Mission: Round 5 Fresh 10-Song Stress-Test Selection (Tracks 51–60)",
    "auditor": "pro_dj_auditor",
    "strict_exclusion_check": {
        "regression_library_size": 50,
        "regression_tracks_excluded": [
            "Animals", "Titanium", "Don't Start Now", "Ghungroo", "Zingaat",
            "Malhari", "Big Dawgs", "Blinding Lights", "Chaiyya Chaiyya", "Perfect",
            "Badtameez Dil", "Mundian To Bach Ke", "FE!N", "Mwaki", "Chogada",
            "Tremor", "Levitating", "Afreen Afreen", "Baddadan", "Tum Hi Ho",
            "Gallan Goodiyaan", "Proper Patola", "Gasolina", "Pepas", "Sauda Khara Khara",
            "Kesariya", "Losing It", "Sicko Mode", "Dancin", "Raataan Lambiyan",
            "Mnike", "295", "Doja", "Great Spirit", "Kar Gayi Chull",
            "Mi Gente", "Ghoomar", "Opus", "Kun Faya Kun", "Solar System",
            "Agar Tum Saath Ho", "Baller", "Lean On", "Calm Down", "Deewani Mastani",
            "Aankh Marey", "Wake Me Up", "Danza Kuduro", "Montagem - PR Funk", "Cinema (Skrillex Remix)"
        ],
        "legacy_batch_1_excluded": [
            "Kala Chashma", "London Thumakda", "Brown Munde", "One Kiss", "Dholi Taro", "Levels"
        ],
        "reference_corpus_excluded": True,
        "overlap_count": 0,
        "isolation_status": "STRICT_ISOLATION_CONFIRMED"
    },
    "tracks": [
        {
            "track_number": 51,
            "track_id": "channa_mereya",
            "title": "Channa Mereya",
            "artist": "Arijit Singh / Pritam (Ae Dil Hai Mushkil)",
            "search_query": "ytsearch1:Channa Mereya Full Song Ae Dil Hai Mushkil Arijit Singh",
            "bpm": 74.0,
            "key": "C Major",
            "camelot": "8B",
            "meter": "4/4 Slow Acoustic Ballad / Shehnai & Strings",
            "energy_level": 3.0,
            "specific_challenge": "Intimate fingerpicked acoustic guitar intro, zero percussion in rubato verse, delicate vocal breakdown resolving with breath silence; tests Breakdown Alaap Sanctity, natural phrase decay down to -40 dBFS, and acoustic frequency isolation."
        },
        {
            "track_number": 52,
            "track_id": "last_last",
            "title": "Last Last",
            "artist": "Burna Boy",
            "search_query": "ytsearch1:Burna Boy Last Last Official Music Video",
            "bpm": 90.0,
            "key": "A Major / F# Minor",
            "camelot": "11B / 11A",
            "meter": "4/4 Global Afrobeats / Toni Braxton Sample",
            "energy_level": 7.0,
            "specific_challenge": "Soulful mid-tempo Afrobeat groove, conversational singing, acoustic horn hook, infectious swing; tests smooth vocal phrase completion and mid-tempo groove locking."
        },
        {
            "track_number": 53,
            "track_id": "taki_taki",
            "title": "Taki Taki",
            "artist": "DJ Snake ft. Selena Gomez, Ozuna, Cardi B",
            "search_query": "ytsearch1:DJ Snake Taki Taki ft Selena Gomez Ozuna Cardi B Official Video",
            "bpm": 96.0,
            "key": "F# Minor",
            "camelot": "6A",
            "meter": "4/4 Latin Moombahton / Dembow Tresillo",
            "energy_level": 8.5,
            "specific_challenge": "Multi-vocal international crossover, iconic pitched vocal flute lead, bilingual Spanish/English anacrusis pickup by Ozuna; tests lookahead vocal ducking and dembow low-end separation."
        },
        {
            "track_number": 54,
            "track_id": "love_nwantiti",
            "title": "Love Nwantiti (Ah Ah Ah)",
            "artist": "CKay",
            "search_query": "ytsearch1:CKay Love Nwantiti Ah Ah Ah Official Music Video",
            "bpm": 100.0,
            "key": "E Minor",
            "camelot": "9A",
            "meter": "4/4 Nigerian Afro-Pop / Emotive Afro-R&B",
            "energy_level": 7.0,
            "specific_challenge": "Global viral Afrobeat earworm, gentle guitar pluck, delicate falsetto vocal hooks; tests subtle mid-tempo groove swing and high-frequency shaker transparency."
        },
        {
            "track_number": 55,
            "track_id": "bom_diggy_diggy",
            "title": "Bom Diggy Diggy",
            "artist": "Zack Knight & Jasmin Walia (Sonu Ke Titu Ki Sweety)",
            "search_query": "ytsearch1:Bom Diggy Diggy Zack Knight Jasmin Walia Sonu Ke Titu Ki Sweety",
            "bpm": 102.0,
            "key": "F# Minor",
            "camelot": "11A",
            "meter": "4/4 Urban UK-Asian / Punjabi-Pop Dance",
            "energy_level": 8.5,
            "specific_challenge": "Bouncy club bassline, rapid call-and-response male/female vocals, catchy synth hook; tests rapid vocal ducking and speech formant separation."
        },
        {
            "track_number": 56,
            "track_id": "shubhaarambh",
            "title": "Shubhaarambh",
            "artist": "Amit Trivedi, Divya Kumar, Shruti Pathak (Kai Po Che)",
            "search_query": "ytsearch1:Shubhaarambh Kai Po Che Amit Trivedi Official Full Song",
            "bpm": 116.0,
            "key": "B Major / G# Minor",
            "camelot": "1B / 1A",
            "meter": "6/8 Compound Traditional Gujarati Garba / Folk",
            "energy_level": 8.0,
            "specific_challenge": "Authentic 6/8 live Gujarati dholak, shehnai lead, folk vocal taans; strictly forbids straight 4/4 crossfading; tests measure-boundary taan drop-swaps and compound-to-simple metric bridges."
        },
        {
            "track_number": 57,
            "track_id": "chammak_challo",
            "title": "Chammak Challo",
            "artist": "Akon & Hamsika Iyer (Ra.One / Vishal-Shekhar)",
            "search_query": "ytsearch1:Chammak Challo Full Song Ra One Akon Kareena Kapoor",
            "bpm": 108.0,
            "key": "B Minor",
            "camelot": "10A",
            "meter": "4/4 Global Bollywood-R&B Club Anthem",
            "energy_level": 9.0,
            "specific_challenge": "Cross-cultural vocal duet (Akon's English/Hindi crossover with traditional South Indian folk vocal), electro-dhol bounce; tests vocal formant masking prevention and multi-voice frequency carving."
        },
        {
            "track_number": 58,
            "track_id": "heads_will_roll",
            "title": "Heads Will Roll (A-Trak Remix)",
            "artist": "Yeah Yeah Yeahs",
            "search_query": "ytsearch1:Yeah Yeah Yeahs Heads Will Roll A-Trak Remix",
            "bpm": 128.0,
            "key": "F# Minor / A Major",
            "camelot": "11A / 11B",
            "meter": "4/4 Peak-Time Electro-House / Festival Anthem",
            "energy_level": 9.5,
            "specific_challenge": "Iconic electro-house synth riff, driving four-on-the-floor kick, colossal festival energy; tests build-up immunity and acoustic-to-EDM transient impact."
        },
        {
            "track_number": 59,
            "track_id": "mauja_hi_mauja",
            "title": "Mauja Hi Mauja",
            "artist": "Mika Singh (Jab We Met / Pritam)",
            "search_query": "ytsearch1:Mauja Hi Mauja Full Song Jab We Met Shahid Kapoor Kareena Kapoor",
            "bpm": 135.0,
            "key": "D Major / B Minor",
            "camelot": "10B / 10A",
            "meter": "4/4 High-Octane Commercial Bollywood-Bhangra Banger",
            "energy_level": 10.0,
            "specific_challenge": "Blistering 135 BPM live Punjabi dhol, brass fanfare, crowd-rousing sing-along chorus; tests high-speed Bhangra dance energy transfer and live percussion low-end dominance."
        },
        {
            "track_number": 60,
            "track_id": "winning_speech",
            "title": "Winning Speech",
            "artist": "Karan Aujla (Prod. Mxrci)",
            "search_query": "ytsearch1:Karan Aujla Winning Speech Official Music Video",
            "bpm": 140.0,
            "key": "E Minor",
            "camelot": "9A",
            "meter": "4/4 Half-Time Punjabi UK Drill / Sliding 808 Trap",
            "energy_level": 10.0,
            "specific_challenge": "Heavy sliding 808 sub-bass, dark drill strings, rapid-fire syncopated Punjabi flow; tests half-time octave transitions (140 <-> 70 BPM) and transient attack sharpness on line arrays."
        }
    ],
    "challenge_pairs": [
        {
            "pair_id": "pair_1",
            "title": "Metric Modulation / Extreme Cross-Genre Bass Swap",
            "dj_level": "Level 3/4",
            "outgoing_track": "59. Mauja Hi Mauja (135 BPM Bollywood Bhangra, 10B/10A)",
            "incoming_track": "60. Winning Speech (140 / 70 BPM Punjabi UK Drill, 9A)",
            "challenge_description": "Transitioning from a 135 BPM driving Bhangra live dhol groove into a 140 / 70 BPM dark half-time Punjabi drill sliding 808 sub-bass drop (+5 BPM acceleration, double-time to half-time metric modulation).",
            "required_technique": "48 dB/oct HPF at 180 Hz on the Bhangra dhol and brass fanfare, 3-bar dynamic tempo ramp from 135 to 140 BPM, and a surgical half-time drop-swap on Karan Aujla's introductory vocal punch into the heavy 70 BPM sliding 808 drop.",
            "success_criteria": "Zero kick galloping; strict sub-bass handover before the drill 808 drops; razor-sharp transient preservation on downbeat."
        },
        {
            "pair_id": "pair_2",
            "title": "Vocal Safety & Collision Prevention (Extended Anacrusis)",
            "dj_level": "Level 2",
            "outgoing_track": "52. Last Last (90 BPM Afrobeats, 11B/11A)",
            "incoming_track": "53. Taki Taki (96 BPM Latin Moombahton, 6A)",
            "challenge_description": "Burna Boy's emotive vocal outro on Bar 8 resolves on Beat 3.5, while Ozuna and DJ Snake's iconic vocal pickup ('Bailame como si fuera la última vez...') enters 450 ms before Beat 1.",
            "required_technique": "Lookahead dynamic vocal ducking (-6 dB in the 1.5–3 kHz formant band) on Burna Boy's vocal tail starting 500 ms prior to downbeat, isolating the Spanish vocal anacrusis pickup in crystalline solo air before the 96 BPM dembow kick drops on Beat 1.",
            "success_criteria": "100% vocal safety; zero syllable collision; clean low-end transition from Afrobeat percussion into 96 BPM dembow kick."
        },
        {
            "pair_id": "pair_3",
            "title": "Cross-Meter / Polyrhythmic Hand-Off (6/8 Garba to 4/4 Club Dance)",
            "dj_level": "Level 3",
            "outgoing_track": "56. Shubhaarambh (116 BPM 6/8 Gujarati Garba, 1B/1A)",
            "incoming_track": "55. Bom Diggy Diggy (102 BPM 4/4 UK Punjabi Club Dance, 11A)",
            "challenge_description": "Handing off from a traditional 6/8 compound Gujarati Garba rhythm (live dholak, shehnai, and folk clapping) into a straight 4/4 modern UK club groove (-14 BPM tempo ease).",
            "required_technique": "Bans unaligned multi-bar crossfades. Employs a measure-boundary drop-swap on Amit Trivedi and Shruti Pathak's vocal taan resolution on Bar 8, caught with a 3:2 metric delay pulse bridge (588 ms), dropping Zack Knight's punchy 4/4 club kick and bassline cleanly on Beat 1 of the new bar.",
            "success_criteria": "Complete elimination of polyrhythmic kick trainwrecks; seamless handover from traditional Indian Garba celebration into modern club bounce."
        },
        {
            "pair_id": "pair_4",
            "title": "Complex Camelot Key Modulation / Dynamic Energy Curve",
            "dj_level": "Level 4",
            "outgoing_track": "57. Chammak Challo (108 BPM, 10A / B Minor)",
            "incoming_track": "58. Heads Will Roll (A-Trak Remix) (128 BPM, 11A / F# Minor / 11B)",
            "challenge_description": "Traversing across a +20 BPM acceleration (108 to 128 BPM) and harmonic modulation from 10A (B Minor) to 11A (F# Minor, perfect 5th / dominant-tonic energy lift).",
            "required_technique": "48 dB/oct HPF at 220 Hz to sweep Akon's electro-dhol groove, paired with an accelerating 4-bar dynamic pitch-neutral elastic tempo stretch, dropping into A-Trak's iconic festival synth lead on Beat 1.",
            "success_criteria": "Seamless harmonic lift; zero acoustic hole during acceleration; explosive festival drop impact on Beat 1."
        },
        {
            "pair_id": "pair_5",
            "title": "Ballad to Peak Energy Drop / Drop on One",
            "dj_level": "Level 1",
            "outgoing_track": "51. Channa Mereya (74 BPM Acoustic Ballad, 8B / C Major)",
            "incoming_track": "59. Mauja Hi Mauja (135 BPM Commercial Bhangra, 10B / D Major)",
            "challenge_description": "Bridging from a heartbreaking, intimate Bollywood acoustic ballad (Arijit Singh over fingerpicked acoustic guitar and shehnai at 74 BPM) directly into a blistering 135 BPM commercial Bhangra dancefloor explosion (+61 BPM jump).",
            "required_technique": "Natural phrase decay down to -40 dBFS on Arijit Singh's final vocal breath ('Channa mereya mereya...'), followed by an accelerating 4-bar live Punjabi dholak roll riser, 250 ms of pre-drop dead silence on Beat 4, and explosive Beat 1 detonation on Mika Singh's brass fanfare and live dhol kick with zero digital clipping (<= -1.0 dBTP).",
            "success_criteria": "Preservation of ballad emotional weight; explosive festival release on Beat 1; True Peak <= -1.0 dBTP with 0 clipped samples."
        }
    ],
    "continuous_mix_arc": {
        "title": "Round 5 Continuous 10-Track Master Set Arc (Tracks 51–60)",
        "progression": [
            {"order": 1, "track_id": "channa_mereya", "bpm": 74.0, "role": "Acoustic Ballad Opening; intimate, soul-stirring vocal warmth."},
            {"order": 2, "track_id": "last_last", "bpm": 90.0, "role": "Afrobeats Groove; smooth mid-tempo swing, conversational vibe."},
            {"order": 3, "track_id": "taki_taki", "bpm": 96.0, "role": "Latin Moombahton; dembow bounce, bilingual vocal crossover."},
            {"order": 4, "track_id": "love_nwantiti", "bpm": 100.0, "role": "Afro-Pop / R&B; melodic earworm, smooth floor builder."},
            {"order": 5, "track_id": "bom_diggy_diggy", "bpm": 102.0, "role": "UK-Asian Club; bouncy bassline, male/female vocal energy."},
            {"order": 6, "track_id": "shubhaarambh", "bpm": 116.0, "role": "6/8 Traditional Gujarati Folk; authentic Garba dholak & shehnai celebration."},
            {"order": 7, "track_id": "chammak_challo", "bpm": 108.0, "role": "Bollywood Global Club; Akon crossover, high-energy electro-dhol."},
            {"order": 8, "track_id": "heads_will_roll", "bpm": 128.0, "role": "Peak-Time Electro House; driving festival mainstage synth."},
            {"order": 9, "track_id": "mauja_hi_mauja", "bpm": 135.0, "role": "High-Octane Commercial Bhangra; peak Bollywood dance euphoria."},
            {"order": 10, "track_id": "winning_speech", "bpm": 140.0, "role": "Punjabi UK Drill; heavy half-time sliding 808 bass grand finale."}
        ]
    }
}

os.makedirs(os.path.dirname(output_json), exist_ok=True)
with open(output_json, "w", encoding="utf-8") as f:
    json.dump(selection, f, indent=2)

print(f"Saved Round 5 selection JSON to {output_json}")
