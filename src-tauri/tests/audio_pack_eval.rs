use std::path::PathBuf;
use dhun_lib::analysis::analyze_track_file;
use dhun_lib::dj::decision_engine::{
    plan_dj_transition, PartyDecisionContext, TempoQualityPolicy, TrackTransitionInput,
};
use dhun_lib::dj::levels::DjLevel;
use serde_json::json;

#[test]
#[ignore = "Requires external audio pack assets in resolver-service/cache or DHUN_AUDIO_PACK_DIR"]
fn test_audio_pack_real_track_analysis_and_transitions() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_cache = manifest_dir
        .parent()
        .map(|p| p.join("resolver-service").join("cache"))
        .unwrap_or_else(|| PathBuf::from("resolver-service/cache"));

    let cache_dir = std::env::var("DHUN_AUDIO_PACK_DIR")
        .map(PathBuf::from)
        .unwrap_or(default_cache);

    let tracks_info = vec![
        ("track_01", "Kala Chashma", "Amar Arshi, Badshah, Neha Kakkar", "k4yXQkG2s1E.mp3", vec!["bollywood".to_string(), "dance".to_string()]),
        ("track_02", "London Thumakda", "Labh Janjua, Sonu Kakkar, Neha Kakkar", "udra3Mfw2oo.mp3", vec!["wedding".to_string(), "punjabi".to_string(), "dhol".to_string()]),
        ("track_03", "Brown Munde", "AP Dhillon, Gurinder Gill, Shinda Kahlon", "VNs_cCtdbPc.mp3", vec!["punjabi".to_string(), "hiphop".to_string(), "trap".to_string()]),
        ("track_04", "One Kiss", "Calvin Harris, Dua Lipa", "DkeiKbqa02g.mp3", vec!["house".to_string(), "electronic".to_string(), "dance".to_string()]),
        ("track_05", "Nagada Sang Dhol", "Shreya Ghoshal, Osman Mir", "vK5E_aeBGYA.mp3", vec!["garba".to_string(), "traditional".to_string(), "bollywood".to_string()]),
        ("track_06", "Dholi Taro Dhol Baaje", "Kavita Krishnamurthy, Vinod Rathod", "4a25J3p0kVI.mp3", vec!["dandiya".to_string(), "garba".to_string(), "traditional".to_string()]),
    ];

    let missing_files: Vec<_> = tracks_info
        .iter()
        .filter(|(_, _, _, file_name, _)| !cache_dir.join(file_name).exists())
        .map(|(_, _, _, file_name, _)| *file_name)
        .collect();

    if !missing_files.is_empty() {
        eprintln!(
            "SKIPPED: audio_pack_eval requires local audio assets in {:?}. Missing files: {:?}. Set DHUN_AUDIO_PACK_DIR to provide external assets.",
            cache_dir, missing_files
        );
        return;
    }

    println!("\n=======================================================");
    println!("DHUN REAL AUDIO ANALYSIS — BATCH 1 (6 TRACKS)");
    println!("=======================================================\n");

    let mut analyzed_tracks = Vec::new();

    for (id, title, artist, file_name, genres) in &tracks_info {
        let file_path = cache_dir.join(file_name);
        println!("Analyzing: {} — {} (File: {})...", title, artist, file_name);
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
        println!("  -> Sections ({}):", sections.len());
        for (idx, sec) in sections.iter().enumerate().take(8) {
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
    println!("DHUN TRANSITION SCENARIOS — LEVELS 1 TO 4");
    println!("=======================================================\n");

    let scenarios = vec![
        (0, 1, "sangeet", "Scenario 1: Kala Chashma (Bollywood) -> London Thumakda (Wedding Dhol)"),
        (1, 2, "punjabi_tashan", "Scenario 2: London Thumakda (Dhol) -> Brown Munde (Punjabi Trap)"),
        (2, 3, "international_party", "Scenario 3: Brown Munde (Punjabi Trap) -> One Kiss (Electronic House)"),
        (3, 4, "festival_mode", "Scenario 4: One Kiss (Electronic House) -> Nagada Sang Dhol (3-Taali Garba)"),
        (4, 5, "garba", "Scenario 5: Nagada Sang Dhol (3-Taali Garba) -> Dholi Taro Dhol Baaje (Fast Dance)"),
        (5, 0, "desi_club", "Scenario 6: Dholi Taro Dhol Baaje (Dandiya) -> Kala Chashma (Bollywood Club)"),
    ];

    let mut transition_plans = Vec::new();

    for (out_idx, in_idx, mood, name) in &scenarios {
        let (out_id, out_title, out_art, out_dur, out_res, out_gen) = &analyzed_tracks[*out_idx];
        let (in_id, in_title, in_art, in_dur, in_res, in_gen) = &analyzed_tracks[*in_idx];

        println!("-------------------------------------------------------");
        println!("{}", name);
        println!("From: {} ({:.1} BPM, Camelot {})", out_title, out_res.analysis.bpm, out_res.analysis.camelot_key);
        println!("To:   {} ({:.1} BPM, Camelot {})", in_title, in_res.analysis.bpm, in_res.analysis.camelot_key);
        println!("Mood: {}", mood);

        let out_input = TrackTransitionInput {
            track_id: out_id.clone(),
            deck_id: 0,
            title: out_title.clone(),
            artist: out_art.clone(),
            duration_sec: *out_dur,
            current_pos_sec: (*out_dur - 25.0).max(0.0), // 25s before end
            bpm: Some(out_res.analysis.bpm),
            camelot_key: Some(out_res.analysis.camelot_key.clone()),
            overall_energy: Some(out_res.analysis.overall_energy),
            genre_tags: out_gen.clone(),
            sections: out_res.phrase_markers.clone(),
            timing_profile: Some(out_res.timing_profile.clone()),
            vocal_density: Some(if out_res.timing_profile.has_lead_vocal { 0.7 } else { 0.1 }),
            is_electronic_dance: out_gen.iter().any(|g| g == "house" || g == "electronic"),
            is_acoustic_ballad: false,
            is_percussion_heavy: out_res.timing_profile.kick_presence > 0.6,
            predicted_energy_direction: Some("rising".to_string()),
            vocal_style: None,
        };

        let in_input = TrackTransitionInput {
            track_id: in_id.clone(),
            deck_id: 1,
            title: in_title.clone(),
            artist: in_art.clone(),
            duration_sec: *in_dur,
            current_pos_sec: 0.0,
            bpm: Some(in_res.analysis.bpm),
            camelot_key: Some(in_res.analysis.camelot_key.clone()),
            overall_energy: Some(in_res.analysis.overall_energy),
            genre_tags: in_gen.clone(),
            sections: in_res.phrase_markers.clone(),
            timing_profile: Some(in_res.timing_profile.clone()),
            vocal_density: Some(if in_res.timing_profile.has_lead_vocal { 0.7 } else { 0.1 }),
            is_electronic_dance: in_gen.iter().any(|g| g == "house" || g == "electronic"),
            is_acoustic_ballad: false,
            is_percussion_heavy: in_res.timing_profile.kick_presence > 0.6,
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
                target_energy: 0.8,
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
        "transitions": transition_plans
    });

    let out_report_path = std::env::var("DHUN_EVAL_OUT_DIR")
        .map(|d| PathBuf::from(d).join("batch_1_dsp_analysis.json"))
        .unwrap_or_else(|_| std::env::temp_dir().join("batch_1_dsp_analysis.json"));
    if let Some(parent) = out_report_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out_report_path, serde_json::to_string_pretty(&report).unwrap()).expect("Failed to write analysis json");
    println!("\nWrote full DSP analysis report to: {:?}", out_report_path);
}
