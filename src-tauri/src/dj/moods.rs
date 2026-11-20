use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodDefinition {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub target_energy: f64,
    pub transition_style: String,
    pub repetition_cooldown_mins: i32,
    pub artist_cooldown_mins: i32,
    pub genre_weights: Vec<(String, f64)>,

    // Enriched DJ Personality & Musical Identity Attributes
    #[serde(default = "default_musical_identity")]
    pub musical_identity: String,
    #[serde(default = "default_energy_curve")]
    pub energy_curve: String,
    #[serde(default = "default_bpm_flexibility")]
    pub bpm_flexibility: String,
    #[serde(default = "default_acceptable_tempo_relationships")]
    pub acceptable_tempo_relationships: Vec<String>,
    #[serde(default = "default_vocal_preference")]
    pub vocal_preference: String,
    #[serde(default = "default_preferred_percussion")]
    pub preferred_percussion: String,
    #[serde(default = "default_transition_preferences")]
    pub transition_preferences: Vec<String>,
    #[serde(default = "default_min_transition_beats")]
    pub min_transition_beats: u32,
    #[serde(default = "default_max_transition_beats")]
    pub max_transition_beats: u32,
    #[serde(default = "default_recovery_period_bars")]
    pub recovery_period_bars: u32,
    #[serde(default = "default_contrast_tolerance")]
    pub contrast_tolerance: f64,
    #[serde(default)]
    pub subgenre_weights: Vec<(String, f64)>,
}

fn default_musical_identity() -> String {
    "Versatile multi-genre party atmosphere".to_string()
}
fn default_energy_curve() -> String {
    "dynamic_waves".to_string()
}
fn default_bpm_flexibility() -> String {
    "moderate".to_string()
}
fn default_acceptable_tempo_relationships() -> Vec<String> {
    vec!["direct".to_string(), "mixable_step".to_string()]
}
fn default_vocal_preference() -> String {
    "singalong_hooks".to_string()
}
fn default_preferred_percussion() -> String {
    "versatile_pop_groove".to_string()
}
fn default_transition_preferences() -> Vec<String> {
    vec!["timed_bass_swap".to_string(), "eq3_handover".to_string()]
}
fn default_min_transition_beats() -> u32 {
    8
}
fn default_max_transition_beats() -> u32 {
    32
}
fn default_recovery_period_bars() -> u32 {
    24
}
fn default_contrast_tolerance() -> f64 {
    0.65
}

pub fn get_preset_moods() -> Vec<MoodDefinition> {
    vec![
        // 1. Shaadi: The Royal Indian Wedding Celebration
        MoodDefinition {
            id: "shaadi".to_string(),
            display_name: "Shaadi".to_string(),
            description: "Grand wedding celebrations combining celebratory Bollywood, joyful Punjabi singalongs, and timeless wedding anthems.".to_string(),
            min_bpm: 100.0,
            max_bpm: 128.0,
            target_energy: 0.76,
            transition_style: "bass_swap".to_string(),
            repetition_cooldown_mins: 120,
            artist_cooldown_mins: 45,
            genre_weights: vec![
                ("Bollywood".to_string(), 0.50),
                ("Punjabi".to_string(), 0.30),
                ("Wedding Remix".to_string(), 0.20),
            ],
            musical_identity: "Regal Indian wedding celebration — family-inclusive, high singalong energy, uplifting and joyous.".to_string(),
            energy_curve: "celebratory_bustle".to_string(),
            bpm_flexibility: "moderate".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string(), "mixable_step".to_string()],
            vocal_preference: "singalong_hooks".to_string(),
            preferred_percussion: "acoustic_dholak".to_string(),
            transition_preferences: vec![
                "timed_bass_swap".to_string(),
                "eq3_handover".to_string(),
                "phrase_long_blend".to_string(),
            ],
            min_transition_beats: 8,
            max_transition_beats: 32,
            recovery_period_bars: 32,
            contrast_tolerance: 0.70,
            subgenre_weights: vec![
                ("Bhangra Pop".to_string(), 0.90),
                ("Romantic Singalong".to_string(), 0.85),
                ("Wedding Dholak".to_string(), 0.95),
            ],
        },

        // 2. Baraat: High-Octane Street Procession & Dhol Madness
        MoodDefinition {
            id: "baraat".to_string(),
            display_name: "Baraat".to_string(),
            description: "Electrifying high-octane procession dhol beats, brass hits, and Punjabi hype.".to_string(),
            min_bpm: 126.0,
            max_bpm: 148.0,
            target_energy: 0.96,
            transition_style: "quick_cut".to_string(),
            repetition_cooldown_mins: 90,
            artist_cooldown_mins: 30,
            genre_weights: vec![
                ("Bhangra Dhol".to_string(), 0.50),
                ("Punjabi Pop".to_string(), 0.35),
                ("Brass Band".to_string(), 0.15),
            ],
            musical_identity: "High-adrenaline street procession — relentless acoustic Punjabi dhol power, brass fanfare, and hype chants.".to_string(),
            energy_curve: "relentless_peak".to_string(),
            bpm_flexibility: "moderate".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string(), "mixable_step".to_string()],
            vocal_preference: "hype_chants".to_string(),
            preferred_percussion: "live_punjabi_dhol".to_string(),
            transition_preferences: vec![
                "timed_bass_swap".to_string(),
                "safe_quick_cut".to_string(),
                "filter_sweep_drop".to_string(),
            ],
            min_transition_beats: 4,
            max_transition_beats: 8,
            recovery_period_bars: 64,
            contrast_tolerance: 0.35,
            subgenre_weights: vec![
                ("Live Dhol".to_string(), 1.0),
                ("Baraat Brass".to_string(), 0.95),
                ("Punjabi Hype".to_string(), 0.90),
            ],
        },

        // 3. Mehendi: Joyful Daytime Acoustic & Sufi Warmth
        MoodDefinition {
            id: "mehendi".to_string(),
            display_name: "Mehendi".to_string(),
            description: "Melodic folk, contemporary acoustic Bollywood, and warm joyful Sufi rhythms.".to_string(),
            min_bpm: 86.0,
            max_bpm: 112.0,
            target_energy: 0.58,
            transition_style: "phrase_sync".to_string(),
            repetition_cooldown_mins: 150,
            artist_cooldown_mins: 60,
            genre_weights: vec![
                ("Folk Bollywood".to_string(), 0.50),
                ("Sufi".to_string(), 0.30),
                ("Acoustic".to_string(), 0.20),
            ],
            musical_identity: "Intimate daytime acoustic warmth — playful lyrics, female choruses, and soulful Sufi melodies.".to_string(),
            energy_curve: "warm_acoustic".to_string(),
            bpm_flexibility: "moderate".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string(), "mixable_step".to_string()],
            vocal_preference: "acoustic_storytelling".to_string(),
            preferred_percussion: "warm_acoustic_percussion".to_string(),
            transition_preferences: vec![
                "eq3_handover".to_string(),
                "phrase_long_blend".to_string(),
            ],
            min_transition_beats: 16,
            max_transition_beats: 32,
            recovery_period_bars: 16,
            contrast_tolerance: 0.55,
            subgenre_weights: vec![
                ("Matka Percussion".to_string(), 0.90),
                ("Ghazal Pop".to_string(), 0.80),
                ("Folk Ballad".to_string(), 0.95),
            ],
        },

        // 4. Haldi: Sunny Turmeric Play & Festive Folk Bounce
        MoodDefinition {
            id: "haldi".to_string(),
            display_name: "Haldi".to_string(),
            description: "Upbeat traditional folk celebration, lively acoustic percussion, and festive melodies.".to_string(),
            min_bpm: 98.0,
            max_bpm: 122.0,
            target_energy: 0.68,
            transition_style: "phrase_sync".to_string(),
            repetition_cooldown_mins: 150,
            artist_cooldown_mins: 60,
            genre_weights: vec![
                ("Folk Acoustic".to_string(), 0.50),
                ("Traditional Ubtan".to_string(), 0.30),
                ("Celebration Pop".to_string(), 0.20),
            ],
            musical_identity: "Sun-drenched yellow celebration — playful folk bounce, joyful claps, and traditional wedding banter.".to_string(),
            energy_curve: "playful_bounce".to_string(),
            bpm_flexibility: "moderate".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string(), "mixable_step".to_string()],
            vocal_preference: "celebratory_chants".to_string(),
            preferred_percussion: "acoustic_dholak".to_string(),
            transition_preferences: vec![
                "eq3_handover".to_string(),
                "timed_bass_swap".to_string(),
            ],
            min_transition_beats: 8,
            max_transition_beats: 16,
            recovery_period_bars: 24,
            contrast_tolerance: 0.60,
            subgenre_weights: vec![
                ("Dholak Claps".to_string(), 0.95),
                ("Shehnai Melody".to_string(), 0.85),
                ("Festive Folk".to_string(), 0.90),
            ],
        },

        // 5. Sangeet: Stage Choreography & Family Dance Battles
        MoodDefinition {
            id: "sangeet".to_string(),
            display_name: "Sangeet".to_string(),
            description: "Dynamic dance competition and choreography anthems with energetic stage peaks.".to_string(),
            min_bpm: 116.0,
            max_bpm: 134.0,
            target_energy: 0.86,
            transition_style: "bass_swap".to_string(),
            repetition_cooldown_mins: 120,
            artist_cooldown_mins: 40,
            genre_weights: vec![
                ("Choreography Hits".to_string(), 0.60),
                ("Punjabi Dance".to_string(), 0.40),
            ],
            musical_identity: "Theatrical dance battle — performance-driven stage drops, dramatic builds, and strict beat-1 cueing.".to_string(),
            energy_curve: "theatrical_waves".to_string(),
            bpm_flexibility: "moderate".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string(), "mixable_step".to_string()],
            vocal_preference: "singalong_hooks".to_string(),
            preferred_percussion: "stage_choreography_hits".to_string(),
            transition_preferences: vec![
                "timed_bass_swap".to_string(),
                "filtered_buildup_drop".to_string(),
                "beat_roll_buildup".to_string(),
            ],
            min_transition_beats: 8,
            max_transition_beats: 16,
            recovery_period_bars: 24,
            contrast_tolerance: 0.65,
            subgenre_weights: vec![
                ("Stage Bollywood".to_string(), 1.0),
                ("Dance Battle".to_string(), 0.90),
                ("Commercial Bhangra".to_string(), 0.85),
            ],
        },

        // 6. Desi Club: Heavy 4x4 Commercial Remixes & Tech House Fusion
        MoodDefinition {
            id: "desi_club".to_string(),
            display_name: "Desi Club".to_string(),
            description: "Heavy 4x4 commercial desi remixes, tech house fusions, and club bangers.".to_string(),
            min_bpm: 124.0,
            max_bpm: 132.0,
            target_energy: 0.90,
            transition_style: "filter_sweep".to_string(),
            repetition_cooldown_mins: 120,
            artist_cooldown_mins: 45,
            genre_weights: vec![
                ("Desi Remix".to_string(), 0.50),
                ("Tech House".to_string(), 0.30),
                ("Club EDM".to_string(), 0.20),
            ],
            musical_identity: "Modern Indian nightclub — driving four-on-the-floor kick, heavy sidechained sub-bass, and seamless long mixing.".to_string(),
            energy_curve: "relentless_club_drive".to_string(),
            bpm_flexibility: "tight".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string()],
            vocal_preference: "club_refrains".to_string(),
            preferred_percussion: "four_on_the_floor_sub".to_string(),
            transition_preferences: vec![
                "timed_bass_swap".to_string(),
                "filter_sweep_drop".to_string(),
                "four_deck_layer".to_string(),
            ],
            min_transition_beats: 16,
            max_transition_beats: 32,
            recovery_period_bars: 32,
            contrast_tolerance: 0.50,
            subgenre_weights: vec![
                ("Bollywood 4x4".to_string(), 0.95),
                ("Desi Bass".to_string(), 0.90),
                ("Electro Punjabi".to_string(), 0.85),
            ],
        },

        // 7. Punjabi Tashan: Urban 808 Trap, UK Bhangra & Swagger Anthems
        MoodDefinition {
            id: "punjabi_tashan".to_string(),
            display_name: "Punjabi Tashan".to_string(),
            description: "Heavy 808s, UK Punjabi bhangra, swagger anthems, and modern urban vibes.".to_string(),
            min_bpm: 80.0,
            max_bpm: 144.0,
            target_energy: 0.92,
            transition_style: "quick_cut".to_string(),
            repetition_cooldown_mins: 90,
            artist_cooldown_mins: 30,
            genre_weights: vec![
                ("Punjabi Trap".to_string(), 0.50),
                ("Bhangra Heavy".to_string(), 0.35),
                ("Urban Desi".to_string(), 0.15),
            ],
            musical_identity: "Heavy urban swagger — sub-808 slides, UK Bhangra power, and seamless half-time/double-time transitions.".to_string(),
            energy_curve: "dynamic_swagger".to_string(),
            bpm_flexibility: "wide_multi_tempo".to_string(),
            acceptable_tempo_relationships: vec![
                "direct".to_string(),
                "half_time".to_string(),
                "double_time".to_string(),
            ],
            vocal_preference: "rap_cadence".to_string(),
            preferred_percussion: "live_punjabi_dhol".to_string(),
            transition_preferences: vec![
                "timed_bass_swap".to_string(),
                "echo_out_blend".to_string(),
                "safe_quick_cut".to_string(),
            ],
            min_transition_beats: 4,
            max_transition_beats: 16,
            recovery_period_bars: 32,
            contrast_tolerance: 0.75,
            subgenre_weights: vec![
                ("808 Trap".to_string(), 1.0),
                ("UK Drill Punjabi".to_string(), 0.95),
                ("Folk Dholak".to_string(), 0.80),
            ],
        },

        // 8. Garba: Continuous Navratri Folk Circle — 3-Taali & 2-Taali
        MoodDefinition {
            id: "garba".to_string(),
            display_name: "Garba".to_string(),
            description: "Traditional 3-Taali and 2-Taali continuous cyclic rhythm with accelerating tempo.".to_string(),
            min_bpm: 110.0,
            max_bpm: 144.0,
            target_energy: 0.84,
            transition_style: "phrase_sync".to_string(),
            repetition_cooldown_mins: 180,
            artist_cooldown_mins: 60,
            genre_weights: vec![
                ("Garba Folk".to_string(), 0.60),
                ("Sanedo".to_string(), 0.40),
            ],
            musical_identity: "Sacred Navratri circle dance — cyclic 3-taali clap rhythms, hypnotic trance, and accelerating chalti tempo.".to_string(),
            energy_curve: "progressive_ramp".to_string(),
            bpm_flexibility: "progressive_accelerating".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string(), "gradual_tempo_ramp".to_string()],
            vocal_preference: "cyclic_folk_chants".to_string(),
            preferred_percussion: "cyclic_3_taali".to_string(),
            transition_preferences: vec![
                "eq3_handover".to_string(),
                "phrase_long_blend".to_string(),
                "tempo_ramp_bridge".to_string(),
            ],
            min_transition_beats: 16,
            max_transition_beats: 32,
            recovery_period_bars: 48,
            contrast_tolerance: 0.20,
            subgenre_weights: vec![
                ("3-Taali Dhol".to_string(), 1.0),
                ("2-Taali Chalti".to_string(), 0.95),
                ("Dakla Devotional".to_string(), 0.90),
            ],
        },

        // 9. Dandiya: Rapid Synchronized Stick Striking & Sharp Accents
        MoodDefinition {
            id: "dandiya".to_string(),
            display_name: "Dandiya".to_string(),
            description: "Fast synchronized sticks tempo with driving percussion and folk celebration.".to_string(),
            min_bpm: 124.0,
            max_bpm: 146.0,
            target_energy: 0.90,
            transition_style: "phrase_sync".to_string(),
            repetition_cooldown_mins: 180,
            artist_cooldown_mins: 60,
            genre_weights: vec![
                ("Dandiya Raas".to_string(), 0.70),
                ("Fusion Beats".to_string(), 0.30),
            ],
            musical_identity: "High-velocity stick striking — sharp accented downbeats, fast acoustic percussion, and crisp rhythmic lock.".to_string(),
            energy_curve: "continuous_high_velocity".to_string(),
            bpm_flexibility: "moderate".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string(), "mixable_step".to_string()],
            vocal_preference: "fast_folk_call".to_string(),
            preferred_percussion: "sharp_dandiya_sticks".to_string(),
            transition_preferences: vec![
                "timed_bass_swap".to_string(),
                "safe_quick_cut".to_string(),
            ],
            min_transition_beats: 8,
            max_transition_beats: 16,
            recovery_period_bars: 32,
            contrast_tolerance: 0.25,
            subgenre_weights: vec![
                ("Raas Beat".to_string(), 1.0),
                ("Stick Rhythm".to_string(), 0.95),
                ("Fast Dholak".to_string(), 0.90),
            ],
        },

        // 10. Retro Bollywood: Golden Era 60s/70s/80s RD Burman Funk & Disco
        MoodDefinition {
            id: "retro_bollywood".to_string(),
            display_name: "Retro Bollywood".to_string(),
            description: "Timeless 70s and 80s RD Burman disco funk, Kishore Kumar, and golden hits.".to_string(),
            min_bpm: 88.0,
            max_bpm: 128.0,
            target_energy: 0.62,
            transition_style: "phrase_sync".to_string(),
            repetition_cooldown_mins: 180,
            artist_cooldown_mins: 60,
            genre_weights: vec![
                ("Disco Bollywood".to_string(), 0.60),
                ("Golden Classics".to_string(), 0.40),
            ],
            musical_identity: "Golden era vintage groove — RD Burman brass sections, analog slap bass, congas, and legendary vocal melodies.".to_string(),
            energy_curve: "groovy_warmth".to_string(),
            bpm_flexibility: "moderate".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string(), "mixable_step".to_string()],
            vocal_preference: "nostalgic_legends".to_string(),
            preferred_percussion: "organic_vintage_drums".to_string(),
            transition_preferences: vec![
                "eq3_handover".to_string(),
                "phrase_long_blend".to_string(),
                "safe_quick_cut".to_string(),
            ],
            min_transition_beats: 8,
            max_transition_beats: 32,
            recovery_period_bars: 16,
            contrast_tolerance: 0.45,
            subgenre_weights: vec![
                ("70s Funk".to_string(), 0.95),
                ("RD Burman Brass".to_string(), 1.0),
                ("Kishore Singalong".to_string(), 0.90),
            ],
        },

        // 11. House Party: Eclectic Living Room Social & Multi-Genre Pop
        MoodDefinition {
            id: "house_party".to_string(),
            display_name: "House Party".to_string(),
            description: "Versatile mix of international pop, trending hip-hop, and easy-dance tunes.".to_string(),
            min_bpm: 104.0,
            max_bpm: 126.0,
            target_energy: 0.72,
            transition_style: "bass_swap".to_string(),
            repetition_cooldown_mins: 120,
            artist_cooldown_mins: 45,
            genre_weights: vec![
                ("Pop".to_string(), 0.50),
                ("Hip-Hop".to_string(), 0.30),
                ("Dance".to_string(), 0.20),
            ],
            musical_identity: "Eclectic casual social gathering — accessible singalongs, warm pop hooks, and smooth multi-genre transitions.".to_string(),
            energy_curve: "casual_waves".to_string(),
            bpm_flexibility: "moderate".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string(), "mixable_step".to_string()],
            vocal_preference: "singalong_hooks".to_string(),
            preferred_percussion: "versatile_pop_groove".to_string(),
            transition_preferences: vec![
                "timed_bass_swap".to_string(),
                "filter_sweep_drop".to_string(),
                "eq3_handover".to_string(),
            ],
            min_transition_beats: 8,
            max_transition_beats: 16,
            recovery_period_bars: 24,
            contrast_tolerance: 0.85,
            subgenre_weights: vec![
                ("Chart Singalong".to_string(), 0.95),
                ("Nu-Disco Pop".to_string(), 0.85),
                ("Accessible Hip-Hop".to_string(), 0.80),
            ],
        },

        // 12. Cocktail Night: Sophisticated Deep House & Melodic Lounge
        MoodDefinition {
            id: "cocktail_night".to_string(),
            display_name: "Cocktail Night".to_string(),
            description: "Sophisticated deep house, melodic chill, smooth lounge, and classy reworks.".to_string(),
            min_bpm: 116.0,
            max_bpm: 124.0,
            target_energy: 0.60,
            transition_style: "filter_sweep".to_string(),
            repetition_cooldown_mins: 180,
            artist_cooldown_mins: 60,
            genre_weights: vec![
                ("Deep House".to_string(), 0.50),
                ("Melodic Lounge".to_string(), 0.30),
                ("Acoustic Rework".to_string(), 0.20),
            ],
            musical_identity: "Sophisticated low-lit lounge — hypnotic deep house, warm Rhodes chords, subtle shakers, and non-intrusive luxury.".to_string(),
            energy_curve: "smooth_hypnotic".to_string(),
            bpm_flexibility: "tight".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string()],
            vocal_preference: "soulful_lounge".to_string(),
            preferred_percussion: "deep_house_analog".to_string(),
            transition_preferences: vec![
                "phrase_long_blend".to_string(),
                "four_deck_layer".to_string(),
                "eq3_handover".to_string(),
            ],
            min_transition_beats: 32,
            max_transition_beats: 64,
            recovery_period_bars: 16,
            contrast_tolerance: 0.25,
            subgenre_weights: vec![
                ("Sunset House".to_string(), 0.95),
                ("Nu-Jazz Lounge".to_string(), 0.90),
                ("Minimal Deep".to_string(), 0.85),
            ],
        },

        // 13. Festival Mode: Stadium Mainstage Drops & Pyro Peaks
        MoodDefinition {
            id: "festival_mode".to_string(),
            display_name: "Festival Mode".to_string(),
            description: "Peak stadium energy, massive drops, EDM fusions, and explosive transitions.".to_string(),
            min_bpm: 126.0,
            max_bpm: 138.0,
            target_energy: 0.98,
            transition_style: "quick_cut".to_string(),
            repetition_cooldown_mins: 90,
            artist_cooldown_mins: 30,
            genre_weights: vec![
                ("Festival Big Room".to_string(), 0.50),
                ("EDM Drop".to_string(), 0.30),
                ("Stadium Anthem".to_string(), 0.20),
            ],
            musical_identity: "Stadium mainstage arena — massive supersaw risers, white noise sweeps, explosive drop payoffs, and peak crowd tension.".to_string(),
            energy_curve: "stadium_tension_drops".to_string(),
            bpm_flexibility: "tight".to_string(),
            acceptable_tempo_relationships: vec!["direct".to_string()],
            vocal_preference: "anthemic_drops".to_string(),
            preferred_percussion: "stadium_bigroom_kick".to_string(),
            transition_preferences: vec![
                "filtered_buildup_drop".to_string(),
                "stutter_gate_drop".to_string(),
                "echo_freeze_drop".to_string(),
                "beat_roll_buildup".to_string(),
            ],
            min_transition_beats: 8,
            max_transition_beats: 16,
            recovery_period_bars: 32,
            contrast_tolerance: 0.40,
            subgenre_weights: vec![
                ("Mainstage Big Room".to_string(), 1.0),
                ("Bass House".to_string(), 0.90),
                ("Electro Festival".to_string(), 0.85),
            ],
        },

        // 14. International Party: Cosmopolitan Global Groove — Afrobeat & Reggaeton
        MoodDefinition {
            id: "international_party".to_string(),
            display_name: "International Party".to_string(),
            description: "Global chart-toppers, reggaeton, Afrobeat, and commercial dance.".to_string(),
            min_bpm: 98.0,
            max_bpm: 130.0,
            target_energy: 0.82,
            transition_style: "bass_swap".to_string(),
            repetition_cooldown_mins: 120,
            artist_cooldown_mins: 45,
            genre_weights: vec![
                ("Global Pop".to_string(), 0.40),
                ("Afrobeat".to_string(), 0.35),
                ("Reggaeton".to_string(), 0.25),
            ],
            musical_identity: "Cosmopolitan global dancefloor — syncopated Afrobeat log drums, Latin dembow riddims, and infectious multilingual groove.".to_string(),
            energy_curve: "vibrant_body_groove".to_string(),
            bpm_flexibility: "wide_multi_tempo".to_string(),
            acceptable_tempo_relationships: vec![
                "direct".to_string(),
                "mixable_step".to_string(),
                "half_time".to_string(),
            ],
            vocal_preference: "multilingual_flow".to_string(),
            preferred_percussion: "afro_latin_syncopated".to_string(),
            transition_preferences: vec![
                "timed_bass_swap".to_string(),
                "echo_out_blend".to_string(),
                "filter_sweep_drop".to_string(),
            ],
            min_transition_beats: 8,
            max_transition_beats: 32,
            recovery_period_bars: 24,
            contrast_tolerance: 0.80,
            subgenre_weights: vec![
                ("Afrobeats Amapiano".to_string(), 0.95),
                ("Latin Dembow".to_string(), 0.90),
                ("Global Club".to_string(), 0.85),
            ],
        },
    ]
}

pub fn get_mood_by_id(id: &str) -> Option<MoodDefinition> {
    get_preset_moods()
        .into_iter()
        .find(|m| m.id.eq_ignore_ascii_case(id))
}
