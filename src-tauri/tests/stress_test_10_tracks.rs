use std::path::PathBuf;
use dhun_lib::analysis::{analyze_track_file, FullAnalysisResult};
use dhun_lib::dj::decision_engine::{
    plan_dj_transition, PartyDecisionContext, TempoQualityPolicy, TrackTransitionInput,
};
use dhun_lib::dj::levels::DjLevel;
use serde_json::json;

#[test]
fn test_10_track_stress_library_dsp_and_transitions() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_cache = manifest_dir
        .parent()
        .map(|p| p.join("resolver-service").join("cache"))
        .unwrap_or_else(|| PathBuf::from("resolver-service/cache"));

    let cache_dir = std::env::var("DHUN_AUDIO_PACK_DIR")
        .map(PathBuf::from)
        .unwrap_or(default_cache);

    let tracks_info = vec![
        ("animals", "Animals (Original Mix)", "Martin Garrix", "DuFUtL8zUAk.mp3", vec!["edm".to_string(), "big_room".to_string(), "festival".to_string()]),
        ("titanium", "Titanium (feat. Sia)", "David Guetta", "JRfuAukYTKg.mp3", vec!["dance_pop".to_string(), "electro_house".to_string()]),
        ("dont_start_now", "Don't Start Now", "Dua Lipa", "oygrmJFKYZY.mp3", vec!["nu_disco".to_string(), "pop".to_string(), "dance".to_string()]),
        ("ghungroo", "Ghungroo (From \"War\")", "Vishal-Shekhar, Arijit Singh & Shilpa Rao", "qFkNATtc3mc.mp3", vec!["bollywood".to_string(), "dance".to_string()]),
        ("zingaat", "Zingaat", "Ajay-Atul", "8eong8XHDB4.mp3", vec!["folk".to_string(), "dance".to_string(), "high_energy".to_string()]),
        ("malhari", "Malhari (From \"Bajirao Mastani\")", "Vishal Dadlani", "sRlA7JWTj04.mp3", vec!["dhol_tasha".to_string(), "battle_hymn".to_string(), "bollywood".to_string()]),
        ("big_dawgs", "Big Dawgs", "Hanumankind & Kalmi", "hOHKltAiKXQ.mp3", vec!["hiphop".to_string(), "trap".to_string(), "rap".to_string()]),
        ("blinding_lights", "Blinding Lights", "The Weeknd", "fHI8X4OXluQ.mp3", vec!["synthwave".to_string(), "pop".to_string(), "retro".to_string()]),
        ("chaiyya_chaiyya", "Chaiyya Chaiyya (From \"Dil Se\")", "Sukhwinder Singh & Sapna Awasthi", "49oz7P9d8IA.mp3", vec!["sufi_fusion".to_string(), "folk_percussion".to_string()]),
        ("perfect", "Perfect", "Ed Sheeran", "1UQzJfsT2eo.mp3", vec!["acoustic_ballad".to_string(), "pop".to_string(), "slow_dance".to_string()]),
    ];

    let missing_files: Vec<_> = tracks_info
        .iter()
        .filter(|(_, _, _, file_name, _)| !cache_dir.join(file_name).exists())
        .map(|(_, _, _, file_name, _)| *file_name)
        .collect();

    assert!(
        missing_files.is_empty(),
        "All 10 stress test tracks must exist in cache_dir: {:?}",
        missing_files
    );

    println!("\n=======================================================");
    println!("DHUN REAL AUDIO DSP ANALYSIS — FIXED 10-TRACK LIBRARY");
    println!("=======================================================\n");

    let mut analyzed_tracks: Vec<(String, String, String, f64, FullAnalysisResult, Vec<String>)> = Vec::new();

    for (id, title, artist, file_name, genres) in &tracks_info {
        let file_path = cache_dir.join(file_name);
        println!("Analyzing: {} — {} ({})", title, artist, file_name);
        let result = analyze_track_file(id, &file_path).expect("Track analysis failed");

        let duration_sec = result.duration_ms as f64 / 1000.0;
        let bpm = result.analysis.bpm;
        let camelot = result.analysis.camelot_key.clone();
        let key_name = result.analysis.key_name.clone();
        let pattern = &result.timing_profile.rhythmic_pattern_detected;
        let sections = &result.phrase_markers;

        println!("  -> Duration:      {:.2}s ({} ms @ {}Hz)", duration_sec, result.duration_ms, result.sample_rate);
        println!("  -> Detected BPM:  {:.2} (confidence: {:.2})", bpm, result.analysis.bpm_confidence);
        println!("  -> Alt BPMs:      {:?}", result.timing_profile.alternative_bpms);
        println!("  -> Key / Camelot: {} / {} (confidence: {:.2})", key_name, camelot, result.analysis.key_confidence);
        println!("  -> Downbeat:      {:.1}ms (conf: {:.2})", result.timing_profile.downbeat_offset_ms, result.timing_profile.downbeat_confidence);
        println!("  -> Beatgrid Conf: {:.2}", result.timing_profile.beatgrid_confidence);
        println!("  -> Rhythm Type:   {} (conf: {:.2})", pattern, result.timing_profile.rhythm_confidence);
        println!("  -> Percussion:    Kick={:.2}, Snare={:.2}, Hihat={:.2}, Syncopation={:.2}",
            result.timing_profile.kick_presence,
            result.timing_profile.snare_clap_presence,
            result.timing_profile.hihat_presence,
            result.timing_profile.syncopation_index
        );
        println!("  -> Energy Bands:  Sub-Bass={:.2}, Mid={:.2}, High={:.2}",
            result.timing_profile.sub_bass_energy,
            result.timing_profile.mid_energy,
            result.timing_profile.high_energy
        );
        println!("  -> Vocals:        Lead={}, Backing={}, VocalConf={:.2}",
            result.timing_profile.has_lead_vocal,
            result.timing_profile.has_backing_vocal,
            result.timing_profile.vocal_confidence
        );
        println!("  -> Sections ({} total):", sections.len());
        for (idx, sec) in sections.iter().enumerate().take(6) {
            println!("     [{}] {:12} @ {:6.1}s - {:6.1}s | energy: {:.2}",
                idx,
                sec.section_type,
                sec.start_ms as f64 / 1000.0,
                sec.end_ms as f64 / 1000.0,
                sec.energy_level
            );
        }
        println!();

        analyzed_tracks.push((
            id.to_string(),
            title.to_string(),
            artist.to_string(),
            duration_sec,
            result,
            genres.clone(),
        ));
    }

    println!("\n=======================================================");
    println!("DHUN TRANSITION PLANNING: 6 REQUIRED STRESS-TEST SCENARIOS");
    println!("=======================================================\n");

    let get_track = |tid: &str| -> (&str, &str, f64, &FullAnalysisResult, &Vec<String>) {
        let t = analyzed_tracks.iter().find(|(id, _, _, _, _, _)| id == tid).expect("Track not found");
        (&t.1, &t.2, t.3, &t.4, &t.5)
    };

    let scenarios = vec![
        ("SCENARIO_A_SAME_KEY_EXTREME_TEMPO", "animals", "blinding_lights", "festival_mode"),
        ("SCENARIO_A_REVERSE", "blinding_lights", "animals", "festival_mode"),
        ("SCENARIO_B_METER_AND_ENERGY_CONTRAST", "perfect", "animals", "house_party"),
        ("SCENARIO_B_REVERSE", "animals", "perfect", "cocktail_night"),
        ("SCENARIO_C_STRONG_HARMONIC_CLASH", "dont_start_now", "animals", "international_party"),
        ("SCENARIO_C_REVERSE", "animals", "dont_start_now", "international_party"),
        ("SCENARIO_D_HALF_TIME_GENRE_FLIP_1", "big_dawgs", "chaiyya_chaiyya", "punjabi_tashan"),
        ("SCENARIO_D_HALF_TIME_GENRE_FLIP_2", "chaiyya_chaiyya", "blinding_lights", "desi_club"),
        ("SCENARIO_E_INDIAN_PERCUSSION_GAUNTLET_1", "ghungroo", "malhari", "sangeet"),
        ("SCENARIO_E_INDIAN_PERCUSSION_GAUNTLET_2", "malhari", "zingaat", "baraat"),
        ("SCENARIO_F_VOCAL_SAFETY_1", "big_dawgs", "dont_start_now", "house_party"),
        ("SCENARIO_F_VOCAL_SAFETY_2", "big_dawgs", "chaiyya_chaiyya", "desi_club"),
    ];

    let mut transition_plans = Vec::new();

    for (name, out_id, in_id, mood) in scenarios {
        let (out_title, out_art, out_dur, out_res, out_gen) = get_track(out_id);
        let (in_title, in_art, in_dur, in_res, in_gen) = get_track(in_id);

        println!("--- [{}] {} -> {} (Mood: {}) ---", name, out_title, in_title, mood);

        let out_input = TrackTransitionInput {
            track_id: out_id.to_string(),
            deck_id: 0,
            title: out_title.to_string(),
            artist: out_art.to_string(),
            duration_sec: out_dur,
            current_pos_sec: (out_dur - 25.0).max(0.0),
            bpm: Some(out_res.analysis.bpm),
            camelot_key: Some(out_res.analysis.camelot_key.clone()),
            overall_energy: Some(out_res.analysis.overall_energy),
            genre_tags: out_gen.clone(),
            sections: out_res.phrase_markers.clone(),
            timing_profile: Some(out_res.timing_profile.clone()),
            vocal_density: Some(if out_res.timing_profile.has_lead_vocal { 0.7 } else { 0.1 }),
            is_electronic_dance: out_gen.iter().any(|g| g == "edm" || g == "dance"),
            is_acoustic_ballad: out_gen.iter().any(|g| g == "acoustic_ballad"),
            is_percussion_heavy: out_res.timing_profile.kick_presence > 0.6 || out_res.timing_profile.syncopation_index > 0.4,
            predicted_energy_direction: Some("steady".to_string()),
            vocal_style: None,
        };

        let in_input = TrackTransitionInput {
            track_id: in_id.to_string(),
            deck_id: 1,
            title: in_title.to_string(),
            artist: in_art.to_string(),
            duration_sec: in_dur,
            current_pos_sec: 0.0,
            bpm: Some(in_res.analysis.bpm),
            camelot_key: Some(in_res.analysis.camelot_key.clone()),
            overall_energy: Some(in_res.analysis.overall_energy),
            genre_tags: in_gen.clone(),
            sections: in_res.phrase_markers.clone(),
            timing_profile: Some(in_res.timing_profile.clone()),
            vocal_density: Some(if in_res.timing_profile.has_lead_vocal { 0.7 } else { 0.1 }),
            is_electronic_dance: in_gen.iter().any(|g| g == "edm" || g == "dance"),
            is_acoustic_ballad: in_gen.iter().any(|g| g == "acoustic_ballad"),
            is_percussion_heavy: in_res.timing_profile.kick_presence > 0.6 || in_res.timing_profile.syncopation_index > 0.4,
            predicted_energy_direction: Some("rising".to_string()),
            vocal_style: None,
        };

        for level in [
            DjLevel::Level1TwoDeckMixer,
            DjLevel::Level2SmartMixing,
            DjLevel::Level3AdvancedPerformance,
            DjLevel::Level4CreativeTools,
        ] {
            let ctx = PartyDecisionContext {
                dj_level: level,
                mood_id: mood.to_string(),
                target_energy: 0.85,
                recent_techniques: vec![],
                tempo_policy: TempoQualityPolicy::for_dj_level(level),
            };

            let plan = plan_dj_transition(&out_input, &in_input, &ctx);
            let stretch = plan.target_bpm.map(|t| ((t / out_res.analysis.bpm) - 1.0) * 100.0).unwrap_or(0.0);
            println!("  [{:?}] Technique: {:24} | Beats: {:2} | Stretch: {:+5.2}% | Status: {}",
                level,
                plan.selected_technique,
                plan.duration_beats,
                stretch,
                plan.status
            );

            transition_plans.push(json!({
                "scenario": name,
                "mood": mood,
                "dj_level": format!("{:?}", level),
                "outgoing": { "id": out_id, "title": out_title, "bpm": out_res.analysis.bpm, "key": out_res.analysis.camelot_key },
                "incoming": { "id": in_id, "title": in_title, "bpm": in_res.analysis.bpm, "key": in_res.analysis.camelot_key },
                "plan": plan
            }));
        }
    }

    // Also test all 14 moods on a representative transition (Animals -> Titanium)
    let all_14_moods = [
        "shaadi", "baraat", "mehendi", "haldi", "sangeet", "desi_club",
        "punjabi_tashan", "garba", "dandiya", "retro_bollywood",
        "house_party", "cocktail_night", "festival_mode", "international_party"
    ];

    println!("\n=======================================================");
    println!("DHUN MOOD COVERAGE: TESTING ALL 14 PARTY MOODS");
    println!("=======================================================\n");

    let (anim_title, anim_art, anim_dur, anim_res, anim_gen) = get_track("animals");
    let (tit_title, tit_art, tit_dur, tit_res, tit_gen) = get_track("titanium");

    let anim_input = TrackTransitionInput {
        track_id: "animals".to_string(),
        deck_id: 0,
        title: anim_title.to_string(),
        artist: anim_art.to_string(),
        duration_sec: anim_dur,
        current_pos_sec: (anim_dur - 25.0).max(0.0),
        bpm: Some(anim_res.analysis.bpm),
        camelot_key: Some(anim_res.analysis.camelot_key.clone()),
        overall_energy: Some(anim_res.analysis.overall_energy),
        genre_tags: anim_gen.clone(),
        sections: anim_res.phrase_markers.clone(),
        timing_profile: Some(anim_res.timing_profile.clone()),
        vocal_density: Some(0.1),
        is_electronic_dance: true,
        is_acoustic_ballad: false,
        is_percussion_heavy: true,
        predicted_energy_direction: Some("steady".to_string()),
        vocal_style: None,
    };

    let tit_input = TrackTransitionInput {
        track_id: "titanium".to_string(),
        deck_id: 1,
        title: tit_title.to_string(),
        artist: tit_art.to_string(),
        duration_sec: tit_dur,
        current_pos_sec: 0.0,
        bpm: Some(tit_res.analysis.bpm),
        camelot_key: Some(tit_res.analysis.camelot_key.clone()),
        overall_energy: Some(tit_res.analysis.overall_energy),
        genre_tags: tit_gen.clone(),
        sections: tit_res.phrase_markers.clone(),
        timing_profile: Some(tit_res.timing_profile.clone()),
        vocal_density: Some(0.8),
        is_electronic_dance: true,
        is_acoustic_ballad: false,
        is_percussion_heavy: true,
        predicted_energy_direction: Some("rising".to_string()),
        vocal_style: None,
    };

    let mut mood_coverage_plans = Vec::new();

    for mood in all_14_moods {
        let ctx = PartyDecisionContext {
            dj_level: DjLevel::Level3AdvancedPerformance,
            mood_id: mood.to_string(),
            target_energy: 0.85,
            recent_techniques: vec![],
            tempo_policy: TempoQualityPolicy::for_dj_level(DjLevel::Level3AdvancedPerformance),
        };
        let plan = plan_dj_transition(&anim_input, &tit_input, &ctx);
        println!("  Mood [{:20}] -> Technique: {:20} | Beats: {:2} | Status: {}",
            mood,
            plan.selected_technique,
            plan.duration_beats,
            plan.status
        );
        mood_coverage_plans.push(json!({
            "mood": mood,
            "technique": plan.selected_technique,
            "beats": plan.duration_beats,
            "status": plan.status
        }));
    }

    let report = json!({
        "analyzed_tracks": analyzed_tracks.iter().map(|(id, title, artist, dur, res, gen)| {
            json!({
                "id": id,
                "title": title,
                "artist": artist,
                "duration_sec": dur,
                "genres": gen,
                "bpm": res.analysis.bpm,
                "bpm_confidence": res.analysis.bpm_confidence,
                "alternative_bpms": res.timing_profile.alternative_bpms,
                "key_name": res.analysis.key_name,
                "camelot_key": res.analysis.camelot_key,
                "key_confidence": res.analysis.key_confidence,
                "downbeat_offset_ms": res.timing_profile.downbeat_offset_ms,
                "downbeat_confidence": res.timing_profile.downbeat_confidence,
                "beatgrid_confidence": res.timing_profile.beatgrid_confidence,
                "rhythmic_pattern": res.timing_profile.rhythmic_pattern_detected,
                "rhythm_confidence": res.timing_profile.rhythm_confidence,
                "kick_presence": res.timing_profile.kick_presence,
                "snare_clap_presence": res.timing_profile.snare_clap_presence,
                "hihat_presence": res.timing_profile.hihat_presence,
                "syncopation_index": res.timing_profile.syncopation_index,
                "sub_bass_energy": res.timing_profile.sub_bass_energy,
                "mid_energy": res.timing_profile.mid_energy,
                "high_energy": res.timing_profile.high_energy,
                "has_lead_vocal": res.timing_profile.has_lead_vocal,
                "has_backing_vocal": res.timing_profile.has_backing_vocal,
                "vocal_confidence": res.timing_profile.vocal_confidence,
                "sections": res.phrase_markers.iter().map(|s| json!({
                    "type": s.section_type,
                    "start_sec": s.start_ms as f64 / 1000.0,
                    "end_sec": s.end_ms as f64 / 1000.0,
                    "energy": s.energy_level,
                    "sub_bass": s.sub_bass_energy,
                    "mid": s.mid_energy,
                    "high": s.high_energy,
                    "lufs_est": s.loudness_lufs_est
                })).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>(),
        "stress_scenarios": transition_plans,
        "mood_coverage": mood_coverage_plans
    });

    let calib_dir = manifest_dir.parent().unwrap().join("DHUN_CALIBRATION");
    let out_report_path = calib_dir.join("10_track_dsp_analysis.json");
    let _ = std::fs::create_dir_all(&calib_dir);
    std::fs::write(&out_report_path, serde_json::to_string_pretty(&report).unwrap()).expect("Failed to write report json");
    println!("\nWrote complete 10-track DSP analysis report to: {:?}", out_report_path);
}
