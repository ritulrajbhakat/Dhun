import json
import os

output_json = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\DHUN_CALIBRATION\round_04_fresh_10_song_selection.json"

selection = {
    "calibration_round": 4,
    "batch_title": "Round 4 Fresh 10-Song Stress-Test Selection (Songs 31–40: Test Generalisation)",
    "auditor": "pro_dj_auditor",
    "strict_exclusion_check": {
        "regression_library_size": 40,
        "regression_tracks_excluded": [
            "Animals", "Titanium", "Don't Start Now", "Ghungroo", "Zingaat",
            "Malhari", "Big Dawgs", "Blinding Lights", "Chaiyya Chaiyya", "Perfect",
            "Badtameez Dil", "Mundian To Bach Ke", "FE!N", "Mwaki", "Chogada",
            "Tremor", "Levitating", "Afreen Afreen", "Baddadan", "Tum Hi Ho",
            "Gallan Goodiyaan", "Proper Patola", "Gasolina", "Pepas", "Sauda Khara Khara",
            "Kesariya", "Losing It", "Sicko Mode", "Dancin", "Raataan Lambiyan",
            "Mnike", "295", "Doja", "Great Spirit", "Kar Gayi Chull",
            "Mi Gente", "Ghoomar", "Opus", "Kun Faya Kun", "Solar System"
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
            "track_number": 31,
            "track_id": "agar_tum_saath_ho",
            "title": "Agar Tum Saath Ho",
            "artist": "Alka Yagnik & Arijit Singh (Tamasha)",
            "search_query": "ytsearch1:Agar Tum Saath Ho Full Song Tamasha Arijit Singh Alka Yagnik",
            "bpm": 80.0,
            "key": "G Major / E Minor",
            "camelot": "9B / 9A",
            "meter": "4/4 Slow Acoustic Ballad / Shehnai & Strings",
            "energy_level": 3,
            "specific_challenge": "Intimate acoustic guitar intro, zero percussion in rubato verse, delicate female/male vocal handoffs; tests Breakdown Alaap Sanctity, natural phrase decay down to -40 dBFS, and acoustic frequency isolation."
        },
        {
            "track_number": 32,
            "track_id": "baller",
            "title": "Baller",
            "artist": "Shubh (Prod. Ikky)",
            "search_query": "ytsearch1:Shubh Baller Official Music Video",
            "bpm": 96.0,
            "key": "F# Minor",
            "camelot": "6A",
            "meter": "4/4 West Coast Punjabi 808 Hip-Hop",
            "energy_level": 8,
            "specific_challenge": "Punchy sliding 808 sub-bass, rapid conversational Punjabi rap cadence, dry syncopated clap; tests sub-bass punch and rapid vocal handover without low-end mud."
        },
        {
            "track_number": 33,
            "track_id": "lean_on",
            "title": "Lean On",
            "artist": "Major Lazer & DJ Snake ft. MØ",
            "search_query": "ytsearch1:Major Lazer DJ Snake Lean On feat MO Official Music Video",
            "bpm": 98.0,
            "key": "A Minor",
            "camelot": "8A",
            "meter": "4/4 Global Moombahton / Dancehall Tresillo",
            "energy_level": 8,
            "specific_challenge": "Iconic pitched vocal chop lead melody, syncopated dembow tresillo kick, female vocal anacrusis pickup; tests vocal chop handover and Moombahton groove locking against straight hip-hop."
        },
        {
            "track_number": 34,
            "track_id": "calm_down",
            "title": "Calm Down",
            "artist": "Rema",
            "search_query": "ytsearch1:Rema Calm Down Official Music Video",
            "bpm": 107.0,
            "key": "B Major",
            "camelot": "2B",
            "meter": "4/4 Afrobeats Polyrhythmic Afro-Swing",
            "energy_level": 7,
            "specific_challenge": "Laid-back Nigerian Afro-swing shaker/rimshot polyrhythm, intimate conversational singing with Pidgin English phrasing, warm acoustic guitar riff; tests subtle mid-tempo groove swing and high-frequency shaker transparency."
        },
        {
            "track_number": 35,
            "track_id": "deewani_mastani",
            "title": "Deewani Mastani",
            "artist": "Shreya Ghoshal (Bajirao Mastani)",
            "search_query": "ytsearch1:Deewani Mastani Full Video Song Bajirao Mastani Shreya Ghoshal",
            "bpm": 114.0,
            "key": "Bb Major",
            "camelot": "3B",
            "meter": "6/8 Compound Indian Classical Mujra / Qawwali Chorus",
            "energy_level": 7.5,
            "specific_challenge": "Authentic 6/8 court meter with intricate sitar alaap, ghungroo ankle-bells, heavy tabla/dholak swings, and mass qawwali clapping chorus; strictly forbids straight 4/4 crossfading; tests measure-boundary taan drop-swap and compound-to-simple metric bridges."
        },
        {
            "track_number": 36,
            "track_id": "aankh_marey",
            "title": "Aankh Marey",
            "artist": "Neha Kakkar, Mika Singh, Kumar Sanu (Simmba)",
            "search_query": "ytsearch1:Aankh Marey Simmba Neha Kakkar Mika Singh Kumar Sanu",
            "bpm": 118.0,
            "key": "Eb Minor",
            "camelot": "2A",
            "meter": "4/4 Commercial Bollywood Electro-House Banger",
            "energy_level": 9,
            "specific_challenge": "Triple vocal polyphony (Neha Kakkar's punchy hook, Kumar Sanu's 90s nostalgia verse, Mika Singh's brassy delivery), aggressive electro-synth lead, brass horn hits; tests vocal formant masking prevention and multi-voice frequency carving."
        },
        {
            "track_number": 37,
            "track_id": "wake_me_up",
            "title": "Wake Me Up",
            "artist": "Avicii ft. Aloe Blacc",
            "search_query": "ytsearch1:Avicii Wake Me Up Official Video",
            "bpm": 124.0,
            "key": "A Major / F# Minor",
            "camelot": "11B / 11A",
            "meter": "4/4 Progressive Folk-House / Melodic EDM",
            "energy_level": 9,
            "specific_challenge": "Acoustic guitar strumming verse transitioning into colossal four-on-the-floor synth drop; Aloe Blacc's soulful vocal delivery; tests build-up immunity and acoustic-to-EDM transient impact."
        },
        {
            "track_number": 38,
            "track_id": "danza_kuduro",
            "title": "Danza Kuduro",
            "artist": "Don Omar ft. Lucenzo",
            "search_query": "ytsearch1:Don Omar Danza Kuduro ft Lucenzo Official Music Video",
            "bpm": 130.0,
            "key": "E Minor",
            "camelot": "9A",
            "meter": "4/4 Fast Latin Mambo / Merengue Dance Pulse",
            "energy_level": 9.5,
            "specific_challenge": "Blistering 130 BPM Latin accordion riff, bilingual Portuguese/Spanish vocal trade, rapid-fire rhythmic bounce; tests high-speed Latin dance energy transfer and accordion midrange frequency management."
        },
        {
            "track_number": 39,
            "track_id": "montagem_pr_funk",
            "title": "Montagem - PR Funk",
            "artist": "S3BZS / Mc Vitin LC",
            "search_query": "ytsearch1:S3BZS MONTAGEM PR FUNK Official Audio",
            "bpm": 130.0,
            "key": "F Minor",
            "camelot": "4A",
            "meter": "4/4 Brazilian Baile Funk / Tamborzão Syncopation",
            "energy_level": 10,
            "specific_challenge": "Aggressive, raw Brazilian Baile Funk syncopated tamborzão beat ('ch-ch-boom'), distorted 808 sub slides, Portuguese vocal stabs; total absence of standard four-on-the-floor kick; tests non-standard rhythmic synchronization against straight club beats."
        },
        {
            "track_number": 40,
            "track_id": "cinema_skrillex",
            "title": "Cinema (Skrillex Remix)",
            "artist": "Benny Benassi ft. Gary Go",
            "search_query": "ytsearch1:Benny Benassi Cinema Skrillex Remix Official",
            "bpm": 140.0,
            "key": "G# Minor",
            "camelot": "1A",
            "meter": "4/4 Half-Time Melodic Dubstep / Bass Climax",
            "energy_level": 10,
            "specific_challenge": "World-renowned melodic vocal verse building into an earth-shattering half-time dubstep growl drop (70 BPM half-time pulse at 140 BPM grid); tests extreme low-end dynamics, sub-bass growl isolation, and half-time metric modulation."
        }
    ],
    "challenge_pairs": [
        {
            "pair_id": "pair_1",
            "title": "Metric Modulation / Extreme BPM Jump",
            "outgoing_track": "39. Montagem - PR Funk (130 BPM Baile Funk, 4A)",
            "incoming_track": "40. Cinema (Skrillex Remix) (140 / 70 BPM Half-Time Dubstep, 1A)",
            "challenge_description": "Traversing from a 130 BPM syncopated Brazilian Baile Funk tamborzão beat directly into a 140 BPM / 70 BPM half-time dubstep build-up and drop (+10 BPM acceleration, shifting from syncopated 130 BPM tamborzão to 70 BPM half-time stadium sub-bass growl). Requires dynamic 4-bar elastic ramp (transients=crisp) from 130 to 140 BPM, an HPF sweep on the Baile Funk tamborzão percussion at 200 Hz, and a surgical half-time metric drop-swap on Gary Go's vocal drop ('You are my cinema...') into Skrillex's devastating 70 BPM half-time growl bass.",
            "success_criteria": "Absolute transient preservation; zero kick galloping; seamless transition from syncopated Latin/Brazilian funk into massive half-time dubstep drop."
        },
        {
            "pair_id": "pair_2",
            "title": "Advanced Vocal Safety & Extended Anacrusis Protection",
            "outgoing_track": "32. Baller (96 BPM Punjabi Hip-Hop, 6A)",
            "incoming_track": "33. Lean On (98 BPM Moombahton, 8A)",
            "challenge_description": "Shubh's fast-paced Punjabi rap verse concludes on Bar 8 Beat 3.75, while MØ's iconic vocal anacrusis pickup ('Blow a kiss, fire a gun...') starts 400 ms before Beat 1 of the new phrase.",
            "success_criteria": "Lookahead dynamic vocal ducking (-6 dB at 1.8 kHz) starting 450 ms prior to downbeat, isolating MØ's vocal pickup in crystalline solo air; zero syllable collision; clean low-end transition from West Coast Punjabi 808 into 98 BPM Moombahton dembow kick."
        },
        {
            "pair_id": "pair_3",
            "title": "Cross-Meter / Polyrhythmic Groove Hand-off",
            "outgoing_track": "35. Deewani Mastani (114 BPM 6/8 Compound Mujra Court Folk, 3B)",
            "incoming_track": "34. Calm Down (107 BPM 4/4 Afrobeats Afro-Swing, 2B)",
            "challenge_description": "Transitioning from compound 6/8 classical Indian court rhythm (sitar alaap, ghungroo bells, heavy tabla swing) into a laid-back 4/4 Nigerian Afro-swing groove (-7 BPM tempo ease). A standard multi-bar crossfade produces catastrophic rhythmic chaos.",
            "success_criteria": "Strictly bans unaligned crossfades; executes a measure-boundary drop-swap on Shreya Ghoshal's vocal taan cadence on Bar 8, caught with a 3:2 metric delay pulse bridge (560 ms), dropping Rema's gentle Afrobeat rimshot and guitar lick cleanly on Beat 1 of the 4/4 bar."
        },
        {
            "pair_id": "pair_4",
            "title": "Harmonic Energy Shift / Complex Camelot Carve",
            "outgoing_track": "36. Aankh Marey (118 BPM, 2A / Eb Minor)",
            "incoming_track": "37. Wake Me Up (124 BPM, 11B / A Major or 11A / F# Minor)",
            "challenge_description": "2A (Eb Minor) to 11A (F# Minor) / 11B (A Major) represents a +3 Camelot leap (tritone/minor-third harmonic distance) accompanied by a +6 BPM tempo acceleration. Clashing brass synth stabs from Aankh Marey against Avicii's acoustic guitar chord progression will create dissonant clashing frequencies.",
            "success_criteria": "48 dB/oct HPF at 220 Hz on Aankh Marey coupled with a -15 dB notch filter on the 2A fundamental (311 Hz), clearing the frequency band before Avicii's iconic acoustic guitar strumming enters via a smooth 16-beat filter opening, locking the groove seamlessly on Beat 1."
        },
        {
            "pair_id": "pair_5",
            "title": "Dynamic Energy Polarity (Ballad/Atmospheric to Peak Festival Drop)",
            "outgoing_track": "31. Agar Tum Saath Ho (80 BPM Acoustic Ballad, 9B / 9A)",
            "incoming_track": "38. Danza Kuduro (130 BPM Latin Mambo / Merengue, 9A)",
            "challenge_description": "Moving from an intensely emotional, introspective Bollywood acoustic ballad (Alka Yagnik and Arijit Singh over delicate guitar and shehnai at 80 BPM) directly into a blistering 130 BPM high-energy Latin festival banger (+50 BPM jump, identical Camelot key 9A/9B).",
            "success_criteria": "Natural phrase decay to -40 dBFS on the ballad's vocal resolution, followed by an accelerating 4-bar Latin percussion riser and accordion teaser, 200 ms of pre-drop breath silence, and explosive Beat 1 detonation of Lucenzo's accordion and Don Omar's chant with zero digital clipping (<= -1.0 dBTP)."
        }
    ],
    "continuous_mix_arc": {
        "title": "Round 4 Continuous 10-Track Master Set Arc (Tracks 31–40)",
        "progression": [
            {"order": 1, "track_id": "agar_tum_saath_ho", "bpm": 80.0, "role": "Intimate acoustic opening; Alka Yagnik & Arijit Singh soul-stirring ballad."},
            {"order": 2, "track_id": "baller", "bpm": 96.0, "role": "West Coast Punjabi 808 hip-hop pickup; heavy swagger & bass punch."},
            {"order": 3, "track_id": "lean_on", "bpm": 98.0, "role": "Global Moombahton anthem; pitched vocal chops & dembow swing."},
            {"order": 4, "track_id": "calm_down", "bpm": 107.0, "role": "Polyrhythmic Nigerian Afrobeats; laid-back Afro-swing warmth."},
            {"order": 5, "track_id": "deewani_mastani", "bpm": 114.0, "role": "Cross-meter royal 6/8 Mujra folk; sitar alaaps & qawwali chorus."},
            {"order": 6, "track_id": "aankh_marey", "bpm": 118.0, "role": "Commercial Bollywood electro-house party anthem; multi-vocal energy."},
            {"order": 7, "track_id": "wake_me_up", "bpm": 124.0, "role": "Melodic progressive EDM anthem; acoustic guitar build into four-on-the-floor."},
            {"order": 8, "track_id": "danza_kuduro", "bpm": 130.0, "role": "High-velocity Latin mambo/merengue peak; accordion festival frenzy."},
            {"order": 9, "track_id": "montagem_pr_funk", "bpm": 130.0, "role": "Aggressive Brazilian Baile Funk; syncopated tamborzão bass pressure."},
            {"order": 10, "track_id": "cinema_skrillex", "bpm": 140.0, "role": "Colossal half-time melodic dubstep drop; earth-shaking bass finale."}
        ]
    }
}

os.makedirs(os.path.dirname(output_json), exist_ok=True)
with open(output_json, "w", encoding="utf-8") as f:
    json.dump(selection, f, indent=2)

print(f"Saved Round 4 selection JSON to {output_json}")
