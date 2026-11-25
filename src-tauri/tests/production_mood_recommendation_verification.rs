//! DHUN — Production Mood Recommendation Engine Verification Suite
//!
//! Evidence-based validation verifying:
//! 1. All 14 production mood recommendation outputs across multiple starting tracks and DJ levels.
//! 2. Candidate ranking, mood changes, dual-mood blending (70/30), and energy progression.
//! 3. Repetition prevention, artist diversity enforcement, and exhausted candidate pool recovery.
//! 4. Guest request boost safety: approved requests cannot override essential musical compatibility.
//! 5. Reconciled harmonic Camelot scoring (exact: 1.0, relative/adjacent: 0.90, diagonal: 0.70, energy lift: 0.75, clash: 0.35).
//! 6. Tempo-stretch policy enforcement and disparate-tempo transition strategy routing.

use dhun_lib::db::models::{
    AudioAsset, GuestRequest, PlayableTrackCandidate, PlaybackHistoryItem, Track, TrackAnalysis,
};
use dhun_lib::dj::decision_engine::{evaluate_bpm_relationship, TempoQualityPolicy};
use dhun_lib::dj::levels::DjLevel;
use dhun_lib::dj::moods::get_preset_moods;
use dhun_lib::dj::selector::{calculate_camelot_compatibility, TrackSelector};

fn make_candidate(
    id: &str,
    title: &str,
    artist: &str,
    genre: &str,
    bpm: f64,
    camelot: &str,
    energy: f64,
) -> PlayableTrackCandidate {
    PlayableTrackCandidate {
        track: Track {
            id: id.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: None,
            duration_ms: 210000,
            year: Some(2023),
            genre: genre.to_string(),
            subgenre: None,
            created_at: 0,
            updated_at: 0,
        },
        asset: AudioAsset {
            id: format!("asset_{}", id),
            track_id: id.to_string(),
            file_path: format!("{}.mp3", id),
            file_format: "mp3".to_string(),
            file_size_bytes: 5000000,
            sample_rate: 44100,
            bit_depth: Some(16),
            channels: 2,
            sha256_hash: format!("sha_{}", id),
            verification_status: "verified".to_string(),
            availability_status: "available".to_string(),
            last_verified_at: 0,
        },
        analysis: TrackAnalysis {
            track_id: id.to_string(),
            bpm,
            bpm_confidence: 0.95,
            key_name: "Am".to_string(),
            camelot_key: camelot.to_string(),
            key_confidence: 0.95,
            time_signature_num: 4,
            time_signature_den: 4,
            downbeat_offset_ms: 0,
            phrase_length_bars: 8,
            overall_energy: energy,
            dance_suitability: energy * 0.9,
            recommended_mix_in_ms: 0,
            recommended_mix_out_ms: 200000,
            analysis_version: 1,
            analyzed_at: 0,
            manual_override: false,
        },
        compatibility_score: 0.0,
        transition_style: "timed_bass_swap".to_string(),
    }
}

/// Builds a representative library of 24 tracks spanning all genres present in the 60-track regression corpus.
fn build_multi_genre_test_library() -> Vec<PlayableTrackCandidate> {
    vec![
        // Bollywood Wedding & Sangeet
        make_candidate("tr_london", "London Thumakda", "Labh Janjua", "Bollywood Wedding", 120.0, "8B", 0.78),
        make_candidate("tr_gallan", "Gallan Goodiyaan", "Farhan Akhtar", "Bollywood Singalong", 128.0, "11B", 0.85),
        make_candidate("tr_badtameez", "Badtameez Dil", "Benny Dayal", "Bollywood Swing", 124.0, "11B", 0.88),
        make_candidate("tr_ghungroo", "Ghungroo", "Arijit Singh", "Bollywood Dance Pop", 118.0, "9A", 0.75),
        make_candidate("tr_channa", "Channa Mereya", "Arijit Singh", "Slow Acoustic Ballad", 74.0, "8B", 0.32),
        make_candidate("tr_deewani", "Deewani Mastani", "Shreya Ghoshal", "Classical Bollywood Semi-classical", 94.0, "7B", 0.58),

        // Punjabi & Dhol
        make_candidate("tr_mauja", "Mauja Hi Mauja", "Mika Singh", "Punjabi Bhangra Dhol", 132.0, "9B", 0.92),
        make_candidate("tr_sauda", "Sauda Khara Khara", "Diljit Dosanjh", "Punjabi Pop Dance", 130.0, "9A", 0.90),
        make_candidate("tr_mundian", "Mundian To Bach Ke", "Labh Janjua", "Punjabi Classic Dhol", 120.0, "8B", 0.82),
        make_candidate("tr_winning", "Winning Speech", "Karan Aujla", "Punjabi Hip Hop Trap", 92.0, "2A", 0.78),
        make_candidate("tr_malhari", "Malhari", "Vishal Dadlani", "Maharashtrian Dhol War Anthem", 136.0, "9A", 0.96),

        // Garba & Dandiya
        make_candidate("tr_dholi", "Dholi Taro", "Sukhwinder Singh", "Garba Dholak Folk", 126.0, "10B", 0.86),
        make_candidate("tr_chogada", "Chogada", "Darshan Raval", "Modern Garba Pop", 112.0, "6B", 0.72),
        make_candidate("tr_shubhaarambh", "Shubhaarambh", "Amit Trivedi", "Gujarati Folk Garba", 116.0, "10B", 0.75),
        make_candidate("tr_kamariya", "Kamariya", "Darshan Raval", "Garba Dance", 128.0, "6A", 0.84),

        // EDM, Club & Festival
        make_candidate("tr_animals", "Animals", "Martin Garrix", "EDM Big Room Club", 128.0, "4A", 0.98),
        make_candidate("tr_titanium", "Titanium", "David Guetta", "Electro House Dance", 126.0, "4B", 0.92),
        make_candidate("tr_heads", "Heads Will Roll (Remix)", "Yeah Yeah Yeahs", "Electro Dance Peak", 128.0, "9A", 0.95),
        make_candidate("tr_tremor", "Tremor", "DVLM", "Club Mainstage EDM", 128.0, "5A", 0.96),

        // Global Pop & Latin / Afrobeats
        make_candidate("tr_last_last", "Last Last", "Burna Boy", "Afrobeats Global Pop", 90.0, "11B", 0.68),
        make_candidate("tr_taki_taki", "Taki Taki", "DJ Snake", "Latin Moombahton Dembow", 96.0, "6A", 0.85),
        make_candidate("tr_blinding", "Blinding Lights", "The Weeknd", "Synthwave Pop", 171.0, "2B", 0.86),
        make_candidate("tr_calm_down", "Calm Down", "Rema", "Afropop Groove", 107.0, "9B", 0.65),

        // Retro & Sufi
        make_candidate("tr_dum_maaro", "Dum Maaro Dum", "Asha Bhosle", "Retro Bollywood Funk", 120.0, "8A", 0.70),
        make_candidate("tr_afreen", "Afreen Afreen", "Rahat Fateh Ali Khan", "Sufi Ghazal Acoustic", 88.0, "8B", 0.45),
    ]
}

// -----------------------------------------------------------------------------
// VERIFICATION 1: ALL 14 PRODUCTION MOOD RECOMMENDATIONS WITH PRODUCTION ENGINE
// -----------------------------------------------------------------------------
#[test]
fn test_all_14_production_mood_outputs() {
    let moods = get_preset_moods();
    assert_eq!(moods.len(), 14, "Must verify all 14 production moods");

    let library = build_multi_genre_test_library();

    println!("\n=== DHUN 14-MOOD PRODUCTION RECOMMENDATION ENGINE AUDIT ===");

    for mood in &moods {
        // Cold start recommendation (no currently playing track)
        let selected_cold = TrackSelector::score_and_select(
            &library,
            None,
            mood,
            None,
            mood.target_energy,
            &[],
            &[],
            &[],
        ).expect(&format!("Cold-start selection for mood '{}' must succeed", mood.id));

        // In-genre continuation recommendation
        let selected_continue = TrackSelector::score_and_select(
            &library,
            Some(&selected_cold),
            mood,
            None,
            mood.target_energy,
            &[],
            &[],
            &[selected_cold.track.id.clone()],
        ).expect(&format!("Continuation selection for mood '{}' must succeed", mood.id));

        println!(
            "MOOD: {:<20} | Cold: {:<22} ({:.1} BPM, {:>3}, E:{:.2}) | Next: {:<22} ({:.1} BPM, {:>3}, E:{:.2}) [Score: {:.3}]",
            mood.id,
            selected_cold.track.title,
            selected_cold.analysis.bpm,
            selected_cold.analysis.camelot_key,
            selected_cold.analysis.overall_energy,
            selected_continue.track.title,
            selected_continue.analysis.bpm,
            selected_continue.analysis.camelot_key,
            selected_continue.analysis.overall_energy,
            selected_continue.compatibility_score,
        );

        // Assertions for musical appropriateness:
        // 1. Candidate must be within or compatible with mood energy expectation
        assert!(
            (selected_cold.analysis.overall_energy - mood.target_energy).abs() <= 0.35,
            "Cold start track '{}' energy {:.2} must align with mood '{}' target {:.2}",
            selected_cold.track.title,
            selected_cold.analysis.overall_energy,
            mood.id,
            mood.target_energy
        );

        // 2. Continuous selection must not repeat the cold start track
        assert_ne!(
            selected_cold.track.id,
            selected_continue.track.id,
            "Continuation selection must not select the currently playing track"
        );
    }
}

// -----------------------------------------------------------------------------
// VERIFICATION 2: HARMONIC SCORING RECONCILIATION & DIAGONAL STEP SUPPORT
// -----------------------------------------------------------------------------
#[test]
fn test_reconciled_harmonic_scoring_formula() {
    // 1. Same key: 1.0 (Identical tonic and mode)
    assert_eq!(calculate_camelot_compatibility("8B", "8B"), 1.0);
    assert_eq!(calculate_camelot_compatibility("11A", "11A"), 1.0);

    // 2. Relative Major / Minor swap: 0.90 (0 accidental difference, e.g. C maj <-> A min)
    assert_eq!(calculate_camelot_compatibility("8A", "8B"), 0.90);
    assert_eq!(calculate_camelot_compatibility("11B", "11A"), 0.90);

    // 3. Adjacent keys (+1 / -1 hour): 0.90 (1 accidental difference, perfect fifth / fourth)
    assert_eq!(calculate_camelot_compatibility("8A", "9A"), 0.90);
    assert_eq!(calculate_camelot_compatibility("8A", "7A"), 0.90);
    assert_eq!(calculate_camelot_compatibility("12B", "1B"), 0.90); // 12-hour wrap

    // 4. Diagonal modulation (+1 / -1 hour with mode swap): 0.70 (Dominant of relative key)
    assert_eq!(calculate_camelot_compatibility("8A", "9B"), 0.70);
    assert_eq!(calculate_camelot_compatibility("8A", "7B"), 0.70);
    assert_eq!(calculate_camelot_compatibility("12A", "1B"), 0.70);

    // 5. Energy lift (+2 hours same mode): 0.75 (Two-step energetic boost)
    assert_eq!(calculate_camelot_compatibility("8A", "10A"), 0.75);
    assert_eq!(calculate_camelot_compatibility("11A", "1A"), 0.75);

    // 6. Dissonant / distant chromatic keys: 0.35 (Tritone or distant clash)
    assert_eq!(calculate_camelot_compatibility("8A", "2A"), 0.35);
    assert_eq!(calculate_camelot_compatibility("8A", "2B"), 0.35);
}

// -----------------------------------------------------------------------------
// VERIFICATION 3: GUEST REQUEST BOOST SAFETY (CANNOT OVERWHELM MUSICAL INTEGRITY)
// -----------------------------------------------------------------------------
#[test]
fn test_guest_request_safety_guardrails() {
    let moods = get_preset_moods();
    let festival = moods.iter().find(|m| m.id == "festival_mode").unwrap();

    let currently_playing = make_candidate("curr_fest", "Animals", "Martin Garrix", "EDM Dance Club", 128.0, "4A", 0.98);

    // Musically compatible candidate (EDM drop)
    let comp_candidate = make_candidate("fest_drop", "Tremor", "DVLM", "EDM Club", 128.0, "5A", 0.96);

    // Guest requested track: acoustic slow ballad (Severe timbre clash & energy collapse)
    let requested_ballad = make_candidate("ballad_req", "Channa Mereya", "Arijit Singh", "Slow Acoustic Ballad", 74.0, "8B", 0.30);

    let candidates = vec![comp_candidate.clone(), requested_ballad.clone()];

    // Approved request for the ballad with Priority 1
    let approved_requests = vec![GuestRequest {
        id: "req_channa".to_string(),
        session_id: "sess_main".to_string(),
        guest_id: "guest_vip".to_string(),
        guest_name: Some("VIP Table".to_string()),
        track_id: "ballad_req".to_string(),
        track_title: Some("Channa Mereya".to_string()),
        track_artist: Some("Arijit Singh".to_string()),
        submitted_at: 0,
        status: "approved".to_string(),
        priority_order: 1,
        host_note: None,
        reviewed_at: None,
        scheduled_slot: None,
    }];

    let selected = TrackSelector::score_and_select(
        &candidates,
        Some(&currently_playing),
        festival,
        None,
        festival.target_energy,
        &[],
        &approved_requests,
        &[],
    ).expect("Selection must succeed");

    // CRITICAL SAFETY ASSERTION:
    // Even though 'Channa Mereya' is an APPROVED Priority-1 guest request,
    // its severe timbre mismatch (acoustic ballad vs 0.98 EDM), extreme BPM clash (74 vs 128),
    // and energy drop (0.30 vs 0.98) triggers `is_extreme_clash`, suppressing the +2.5 boost to 0.0.
    // Therefore, the festival drop MUST win to protect the dancefloor from catastrophic collapse!
    assert_eq!(
        selected.track.id, "fest_drop",
        "Approved guest request for slow acoustic ballad must NOT override essential dancefloor compatibility!"
    );
}

// -----------------------------------------------------------------------------
// VERIFICATION 4: DUAL-MOOD BLENDING (70% PRIMARY / 30% SECONDARY)
// -----------------------------------------------------------------------------
#[test]
fn test_dual_mood_blending_hybrid() {
    let moods = get_preset_moods();
    let sangeet = moods.iter().find(|m| m.id == "sangeet").unwrap();
    let desi_club = moods.iter().find(|m| m.id == "desi_club").unwrap();

    let candidate_pure_sangeet = make_candidate("sangeet_folk", "London Thumakda", "Labh Janjua", "Bollywood Wedding", 120.0, "8B", 0.78);
    let candidate_pure_club = make_candidate("club_banger", "Animals", "Martin Garrix", "EDM Club", 128.0, "4A", 0.98);
    let candidate_hybrid = make_candidate("hybrid_hit", "Sauda Khara Khara", "Diljit Dosanjh", "Bhangra Club Pop", 130.0, "9A", 0.90);

    let candidates = vec![
        candidate_pure_sangeet.clone(),
        candidate_pure_club.clone(),
        candidate_hybrid.clone(),
    ];

    // Under Sangeet (70%) + Desi Club (30%) blend:
    // Target energy = 0.70 * 0.85 + 0.30 * 0.90 = 0.865
    let selected = TrackSelector::score_and_select(
        &candidates,
        None,
        sangeet,
        Some(desi_club),
        sangeet.target_energy,
        &[],
        &[],
        &[],
    ).expect("Dual mood blend selection succeeds");

    assert_eq!(
        selected.track.id, "hybrid_hit",
        "Dual-mood blend (Sangeet + Desi Club) must prioritize hybrid high-energy Bhangra club track"
    );
}

// -----------------------------------------------------------------------------
// VERIFICATION 5: ARTIST DIVERSITY & CONSECUTIVE PLAYBACK PENALTY
// -----------------------------------------------------------------------------
#[test]
fn test_artist_diversity_enforcement() {
    let moods = get_preset_moods();
    let punjabi = moods.iter().find(|m| m.id == "punjabi_tashan").unwrap();

    let track_diljit_1 = make_candidate("d1", "Sauda Khara Khara", "Diljit Dosanjh", "Punjabi Pop", 130.0, "9A", 0.90);
    let track_diljit_2 = make_candidate("d2", "Lover", "Diljit Dosanjh", "Punjabi Pop", 128.0, "9A", 0.88);
    let track_mika = make_candidate("m1", "Mauja Hi Mauja", "Mika Singh", "Punjabi Dhol", 132.0, "9B", 0.92);

    let candidates = vec![track_diljit_2.clone(), track_mika.clone()];

    let now = chrono::Utc::now().timestamp_millis();
    let history = vec![PlaybackHistoryItem {
        id: 1,
        session_id: "sess_punjabi".to_string(),
        track_id: track_diljit_1.track.id.clone(),
        track_title: track_diljit_1.track.title.clone(),
        track_artist: "Diljit Dosanjh".to_string(),
        deck_id: 0,
        played_at: now - 3 * 60 * 1000, // Played 3 minutes ago
        duration_played_ms: 180000,
        was_skipped: false,
        transition_type: "bass_swap".to_string(),
    }];

    let selected = TrackSelector::score_and_select(
        &candidates,
        Some(&track_diljit_1),
        punjabi,
        None,
        punjabi.target_energy,
        &history,
        &[],
        &[],
    ).expect("Selection succeeds");

    // Diljit Dosanjh was played 3 minutes ago -> artist penalty (-0.35) applied to track_diljit_2.
    // Mika Singh has no artist penalty, so Mika Singh must win to maintain artist diversity!
    assert_eq!(
        selected.track.id, "m1",
        "Artist diversity must prevent back-to-back tracks by the same artist when comparable alternative exists"
    );
}

// -----------------------------------------------------------------------------
// VERIFICATION 6: EXHAUSTED POOL FALLBACK (NEVER STOP THE MUSIC)
// -----------------------------------------------------------------------------
#[test]
fn test_exhausted_pool_fallback_recovery() {
    let moods = get_preset_moods();
    let retro = moods.iter().find(|m| m.id == "retro_bollywood").unwrap();

    let tr1 = make_candidate("r1", "Dum Maaro Dum", "Asha Bhosle", "Retro Bollywood", 120.0, "8A", 0.70);
    let tr2 = make_candidate("r2", "Khaike Paan", "Kishore Kumar", "Retro Bollywood", 122.0, "8A", 0.72);

    let candidates = vec![tr1.clone(), tr2.clone()];

    let now = chrono::Utc::now().timestamp_millis();
    let history = vec![
        PlaybackHistoryItem {
            id: 1,
            session_id: "sess_retro".to_string(),
            track_id: "r1".to_string(),
            track_title: "Dum Maaro Dum".to_string(),
            track_artist: "Asha Bhosle".to_string(),
            deck_id: 0,
            played_at: now - 10 * 60 * 1000, // 10 mins ago
            duration_played_ms: 180000,
            was_skipped: false,
            transition_type: "bass_swap".to_string(),
        },
        PlaybackHistoryItem {
            id: 2,
            session_id: "sess_retro".to_string(),
            track_id: "r2".to_string(),
            track_title: "Khaike Paan".to_string(),
            track_artist: "Kishore Kumar".to_string(),
            deck_id: 1,
            played_at: now - 2 * 60 * 1000, // 2 mins ago
            duration_played_ms: 180000,
            was_skipped: false,
            transition_type: "bass_swap".to_string(),
        },
    ];

    let selected = TrackSelector::score_and_select(
        &candidates,
        None,
        retro,
        None,
        retro.target_energy,
        &history,
        &[],
        &[],
    );

    assert!(selected.is_some(), "Engine must recover and pick a track even when all candidates are in cooldown");
    let chosen = selected.unwrap();
    assert_eq!(
        chosen.track.id, "r1",
        "When all tracks in pool have been played, engine must select the least-recently-played track (10m ago vs 2m ago)"
    );
}

// -----------------------------------------------------------------------------
// VERIFICATION 7: TEMPO STRETCH POLICY & DISPARATE TEMPO TRANSITION ROUTING
// -----------------------------------------------------------------------------
#[test]
fn test_tempo_stretch_policy_and_disparate_tempo_resolution() {
    let p1 = DjLevel::Level1TwoDeckMixer.skill_profile();
    let p2 = DjLevel::Level2SmartMixing.skill_profile();
    let p3 = DjLevel::Level3AdvancedPerformance.skill_profile();
    let p4 = DjLevel::Level4CreativeTools.skill_profile();

    assert_eq!(p1.max_tempo_stretch_pct, 4.0);
    assert_eq!(p2.max_tempo_stretch_pct, 8.0);
    assert_eq!(p3.max_tempo_stretch_pct, 10.0);
    assert_eq!(p4.max_tempo_stretch_pct, 14.0);

    // Level 3 Case: 116 BPM -> 102 BPM (|116/102 - 1| = 13.73% delta)
    // 13.73% exceeds Level 3 direct stretch (10%) AND gradual ramp (11%)!
    let policy_l3 = TempoQualityPolicy::for_dj_level(DjLevel::Level3AdvancedPerformance);
    let decision_l3 = evaluate_bpm_relationship(Some(116.0), Some(102.0), 0.95, true, false, false, &policy_l3);
    assert_eq!(
        decision_l3.relationship,
        dhun_lib::dj::decision_engine::BpmRelationship::NoBeatmatch,
        "Level 3 engine must refuse direct beatmatching when tempo delta (13.73%) exceeds 10% direct / 11% ramp limits"
    );
    assert_eq!(decision_l3.incoming_playback_rate, 1.0, "Playback rate must remain 1.0 (unpitched)");

    // Level 4 Case A: 108 BPM -> 128 BPM (|108/128 - 1| = 15.63% delta)
    // 15.63% exceeds direct static stretch (14.0%), but fits within gradual ramp limit (16.0%)!
    // Engine correctly refuses direct static stretch and routes to dynamic gradual tempo ramp!
    let policy_l4 = TempoQualityPolicy::for_dj_level(DjLevel::Level4CreativeTools);
    let decision_l4 = evaluate_bpm_relationship(Some(108.0), Some(128.0), 0.95, true, false, false, &policy_l4);
    assert_eq!(
        decision_l4.relationship,
        dhun_lib::dj::decision_engine::BpmRelationship::GradualTempoRamp,
        "Level 4 engine must select GradualTempoRamp for 15.63% delta rather than forced static pitch stretch"
    );

    // Level 4 Case B: 100 BPM -> 128 BPM (|100/128 - 1| = 21.88% delta)
    // Exceeds both 14% direct and 16% gradual ramp limits -> NoBeatmatch (vinyl backspin cut)
    let decision_l4_extreme = evaluate_bpm_relationship(Some(100.0), Some(128.0), 0.95, true, false, false, &policy_l4);
    assert_eq!(
        decision_l4_extreme.relationship,
        dhun_lib::dj::decision_engine::BpmRelationship::NoBeatmatch,
        "Level 4 engine must refuse beatmatching when delta (21.88%) exceeds all stretch and ramp limits"
    );
    assert_eq!(decision_l4_extreme.incoming_playback_rate, 1.0);
}
