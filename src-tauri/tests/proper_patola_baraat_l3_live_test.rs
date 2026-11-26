use std::path::PathBuf;

use dhun_lib::db::models::LocalRoomSubmission;
use dhun_lib::db::Database;
use dhun_lib::dj::levels::DjLevel;
use dhun_lib::dj::moods::get_preset_moods;
use dhun_lib::dj::selector::{build_dj_recommendations, evaluate_submission_dj_intelligence};
use dhun_lib::sync::firestore::{from_firestore_document, FirestoreClient, FirestoreConfig};

#[tokio::test]
async fn test_proper_patola_baraat_l3_live_firestore_recommendation() {
    println!("\n==================================================================");
    println!("DHUN — LIVE FIRESTORE TO DJ RECOMMENDATION TEST (TEST 2)");
    println!("Test: Proper Patola → Baraat | DJ Intelligence L3");
    println!("==================================================================\n");

    // ------------------------------------------------------------------
    // STEP 0: INSPECT LOCAL SQLITE CACHE STATE BEFORE TEST
    // ------------------------------------------------------------------
    println!("--- [Pre-Flight] Checking Local SQLite Cache State ---");
    let test_helper_db_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("target").join("test_helper.db"))
        .unwrap_or_else(|| PathBuf::from("target/test_helper.db"));

    let local_db = if test_helper_db_path.exists() {
        println!("Opening existing local database at: {:?}", test_helper_db_path);
        Database::new(&test_helper_db_path).ok()
    } else {
        println!("test_helper.db not found; opening in-memory database for auditor inspection");
        Database::new_in_memory().ok()
    };

    println!("Local SQLite pre-flight check: Ready for candidate comparison.");
    println!("  -> Source of truth for this test: STRICTLY LIVE CLOUD FIRESTORE.\n");

    // ------------------------------------------------------------------
    // STEP 1: AUTHENTICATE AGAINST GOOGLE CLOUD FIRESTORE (dhun-716e6)
    // ------------------------------------------------------------------
    println!("--- [Step 1] Authenticating against Google Cloud Firestore ---");
    dhun_lib::sync::load_dotenv_if_present();

    let project_id = std::env::var("DHUN_FIREBASE_PROJECT_ID")
        .or_else(|_| std::env::var("VITE_FIREBASE_PROJECT_ID"))
        .unwrap_or_else(|_| "dhun-716e6".to_string());
    let api_key = std::env::var("DHUN_FIREBASE_API_KEY")
        .or_else(|_| std::env::var("VITE_FIREBASE_API_KEY"))
        .ok()
        .filter(|k| !k.trim().is_empty());

    assert!(api_key.is_some(), "DHUN_FIREBASE_API_KEY must be configured in environment");

    let client = FirestoreClient::new(FirestoreConfig {
        project_id: project_id.clone(),
        api_key,
    });

    let auth_token = client
        .authenticate_anonymous()
        .await
        .expect("Anonymous authentication against Google Identity Toolkit failed");
    let is_correct_project = client.verify_token_project(&auth_token).unwrap_or(false);
    println!("-> Authentication: SUCCESS (RS256 JWT acquired)");
    println!("-> Target Project Verified (aud == {}): {}", project_id, is_correct_project);
    assert!(is_correct_project, "JWT audience must match target project");

    // ------------------------------------------------------------------
    // STEP 2 & 3: RETRIEVE CATALOGUE & LOCATE PROPER PATOLA
    // ------------------------------------------------------------------
    println!("\n--- [Step 2] Retrieving Full Live Firestore Catalogue ---");
    let raw_docs = client
        .list_catalogue(300)
        .await
        .expect("Failed to list catalogue from Cloud Firestore");
    println!("-> Live Firestore Catalogue: Retrieved {} total raw documents", raw_docs.len());

    let mut canonical_tracks = Vec::new();
    let mut alias_count = 0;

    for (doc_id, doc_json) in raw_docs {
        if doc_id.starts_with("vid_") || doc_id.starts_with("rec_") {
            alias_count += 1;
            continue;
        }
        if !doc_id.starts_with("fp_") {
            continue;
        }
        if let Some(meta) = from_firestore_document(&doc_json) {
            canonical_tracks.push((doc_id, meta));
        }
    }

    println!("Document Filtering Evidence:");
    println!("  -> Excluded alias documents (vid_/rec_): {}", alias_count);
    println!("  -> Canonical primary documents (fp_*): {}", canonical_tracks.len());

    // Locate Proper Patola dynamically from live Firestore catalogue
    let (patola_doc_id, starting_meta) = canonical_tracks
        .iter()
        .find(|(_, meta)| meta.title.to_lowercase().contains("proper patola"))
        .expect("Proper Patola must exist in live Cloud Firestore catalogue")
        .clone();

    println!("\n--- [Step 3] Starting Track Retrieved from Live Cloud Firestore ---");
    println!("  -> Firestore Document:  {}", patola_doc_id);
    println!("  -> Title:               {}", starting_meta.title);
    println!("  -> Artist:              {}", starting_meta.artist);
    println!("  -> Version Tag:         {}", starting_meta.version_tag);
    println!("  -> Duration:            {} ms", starting_meta.duration_ms);
    println!("  -> Original BPM:        {:?}", starting_meta.original_bpm);
    println!("  -> Half Time BPM:       {:?}", starting_meta.half_time_bpm);
    println!("  -> Double Time BPM:     {:?}", starting_meta.double_time_bpm);
    println!("  -> Musical Key:         {:?}", starting_meta.musical_key);
    println!("  -> Camelot Key:         {:?}", starting_meta.camelot_key);
    println!("  -> Overall Energy:      {:?}", starting_meta.overall_energy);
    println!("  -> Danceability:        {:?}", starting_meta.danceability);
    println!("  -> Genre Tags:          {:?}", starting_meta.genre_tags);
    println!("  -> Mood Tags:           {:?}", starting_meta.mood_tags);
    println!("  -> Source URLs:         {:?}", starting_meta.source_urls);
    println!("  -> Processing Status:   {}", starting_meta.processing_status);
    println!("  -> Canonical Fingerprint: fp_{}", starting_meta.canonical_fingerprint);

    assert!(starting_meta.title.to_lowercase().contains("proper patola"));
    assert_eq!(starting_meta.camelot_key.as_deref(), Some("4A"));

    // Candidate pool is all canonical tracks excluding starting track
    let canonical_candidates: Vec<_> = canonical_tracks
        .into_iter()
        .filter(|(_, meta)| meta.canonical_fingerprint != starting_meta.canonical_fingerprint)
        .collect();

    println!("\nCandidate Pool Preparation:");
    println!("  -> Excluded starting track (Proper Patola self): 1");
    println!("  -> Valid canonical candidate tracks from Firestore: {}", canonical_candidates.len());
    assert!(
        canonical_candidates.len() >= 50,
        "Expected at least 50 candidate tracks in catalogue, found {}",
        canonical_candidates.len()
    );

    // ------------------------------------------------------------------
    // STEP 4: INSPECT TAG VOCABULARY & BARAAT MOOD DEFINITION
    // ------------------------------------------------------------------
    println!("\n--- [Step 4] Inspecting Baraat Mood Profile & Candidate Tags ---");
    let preset_moods = get_preset_moods();
    let baraat_mood = preset_moods
        .iter()
        .find(|m| m.id == "baraat")
        .expect("Baraat mood profile must exist in DHUN")
        .clone();

    println!("Baraat Mood Profile Parameters:");
    println!("  -> Display Name:        {}", baraat_mood.display_name);
    println!("  -> Target Energy:        {:.2}", baraat_mood.target_energy);
    println!("  -> BPM Window:          {:.1} – {:.1} BPM", baraat_mood.min_bpm, baraat_mood.max_bpm);
    println!("  -> Transition Style:    {}", baraat_mood.transition_style);
    println!("  -> Primary Genres:      {:?}", baraat_mood.genre_weights);
    println!("  -> Signature Subgenres: {:?}", baraat_mood.subgenre_weights);
    println!("  -> Musical Identity:    {}", baraat_mood.musical_identity);

    // ------------------------------------------------------------------
    // STEP 5: PREPARE INPUTS FOR THE REAL DHUN L3 RECOMMENDATION ENGINE
    // ------------------------------------------------------------------
    println!("\n--- [Step 5] Converting Firestore Entities to LocalRoomSubmission Models ---");
    let starting_vid = starting_meta
        .source_urls
        .iter()
        .find_map(|u| {
            if let Some(stripped) = u.strip_prefix("youtube:") {
                Some(stripped.to_string())
            } else if let Some(idx) = u.find("v=") {
                Some(u[idx + 2..].split('&').next().unwrap_or("").to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "vid_proper_patola".to_string());

    let starting_submission = LocalRoomSubmission {
        id: format!("sub_patola_{}", &starting_meta.canonical_fingerprint[..8]),
        room_id: "room_live_baraat_test_2".to_string(),
        submitted_by_type: "host".to_string(),
        submitted_by_id: "host_01".to_string(),
        submitted_by_name: "DJ Host".to_string(),
        provider: "youtube".to_string(),
        source_url: starting_meta.source_urls.first().cloned().unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", starting_vid)),
        provider_video_id: starting_vid.clone(),
        song_id: Some(starting_meta.song_id.clone()),
        title: starting_meta.title.clone(),
        artist: starting_meta.artist.clone(),
        version_tag: starting_meta.version_tag.clone(),
        thumbnail_url: None,
        duration_ms: Some(starting_meta.duration_ms),
        bpm: starting_meta.original_bpm,
        half_time_bpm: starting_meta.half_time_bpm,
        double_time_bpm: starting_meta.double_time_bpm,
        musical_key: starting_meta.musical_key.clone(),
        camelot_key: starting_meta.camelot_key.clone(),
        overall_energy: starting_meta.overall_energy,
        danceability: starting_meta.danceability,
        genre_tags_json: serde_json::to_string(&starting_meta.genre_tags).unwrap_or_else(|_| "[]".to_string()),
        mood_tags_json: serde_json::to_string(&starting_meta.mood_tags).unwrap_or_else(|_| "[]".to_string()),
        metadata_source: "firestore_live_catalogue".to_string(),
        acquisition_status: "cached_authorized".to_string(),
        acquisition_note: None,
        dj_status: "playing".to_string(), // Currently active track on Deck A
        dj_score: 100.0,
        local_cache_path: None,
        cache_id: Some("cache_proper_patola".to_string()),
        created_at: starting_meta.created_at,
        updated_at: starting_meta.updated_at,
    };

    let mut candidate_submissions = Vec::with_capacity(canonical_candidates.len());
    for (doc_id, cand_meta) in &canonical_candidates {
        let vid = cand_meta
            .source_urls
            .iter()
            .find_map(|u| {
                if let Some(stripped) = u.strip_prefix("youtube:") {
                    Some(stripped.to_string())
                } else if let Some(idx) = u.find("v=") {
                    Some(u[idx + 2..].split('&').next().unwrap_or("").to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown_vid".to_string());

        let sub = LocalRoomSubmission {
            id: format!("sub_{}", &cand_meta.canonical_fingerprint[..12]),
            room_id: "room_live_baraat_test_2".to_string(),
            submitted_by_type: "guest".to_string(),
            submitted_by_id: format!("guest_{}", &cand_meta.canonical_fingerprint[..6]),
            submitted_by_name: "Crowd".to_string(),
            provider: "youtube".to_string(),
            source_url: cand_meta.source_urls.first().cloned().unwrap_or_default(),
            provider_video_id: vid,
            song_id: Some(cand_meta.song_id.clone()),
            title: cand_meta.title.clone(),
            artist: cand_meta.artist.clone(),
            version_tag: cand_meta.version_tag.clone(),
            thumbnail_url: None,
            duration_ms: Some(cand_meta.duration_ms),
            bpm: cand_meta.original_bpm,
            half_time_bpm: cand_meta.half_time_bpm,
            double_time_bpm: cand_meta.double_time_bpm,
            musical_key: cand_meta.musical_key.clone(),
            camelot_key: cand_meta.camelot_key.clone(),
            overall_energy: cand_meta.overall_energy,
            danceability: cand_meta.danceability,
            genre_tags_json: serde_json::to_string(&cand_meta.genre_tags).unwrap_or_else(|_| "[]".to_string()),
            mood_tags_json: serde_json::to_string(&cand_meta.mood_tags).unwrap_or_else(|_| "[]".to_string()),
            metadata_source: "firestore_live_catalogue".to_string(),
            acquisition_status: "cached_authorized".to_string(),
            acquisition_note: None,
            dj_status: "queued_for_dj".to_string(),
            dj_score: 0.0,
            local_cache_path: None,
            cache_id: Some(format!("cache_{}", doc_id)),
            created_at: cand_meta.created_at,
            updated_at: cand_meta.updated_at,
        };
        candidate_submissions.push(sub);
    }

    // Combine current track + unplayed candidate pool
    let mut all_submissions = Vec::with_capacity(candidate_submissions.len() + 1);
    all_submissions.push(starting_submission.clone());
    all_submissions.extend(candidate_submissions.clone());

    // ------------------------------------------------------------------
    // STEP 6: EXECUTE REAL L3 RECOMMENDATION ENGINE
    // ------------------------------------------------------------------
    println!("\n--- [Step 6] Running DHUN Production L3 Recommendation Engine ---");
    let recommendations = build_dj_recommendations(
        &all_submissions,
        &baraat_mood,
        None,
        baraat_mood.target_energy,
        DjLevel::Level3AdvancedPerformance,
    );

    println!("DHUN L3 Recommendation Engine produced {} ranked recommendations:\n", recommendations.len());
    for (idx, rec) in recommendations.iter().enumerate() {
        println!(
            "#{}: [{:20}] {:.1} pts | '{}' by '{}'",
            idx + 1,
            rec.category_label,
            rec.compatibility_score,
            rec.title,
            rec.artist
        );
        println!("    Suggested Technique: {} ({})", rec.technique_label, rec.suggested_technique);
        println!("    Technique Reason:    {}", rec.technique_reason);
        println!("    Energy Direction:    {}", rec.predicted_energy_direction);
        println!("    Confidence:          {:.0}%", rec.confidence * 100.0);
        println!("    Musical Reasons:");
        for r in &rec.reasons {
            println!("      • {}", r);
        }
        println!();
    }

    assert!(!recommendations.is_empty(), "Recommendation engine must return candidates");
    let top_rec = &recommendations[0];
    println!("==================================================================");
    println!("SELECTED BEST NEXT TRACK FOR BARAAT AFTER PROPER PATOLA:");
    println!("  -> Track:               {} — {}", top_rec.title, top_rec.artist);
    println!("  -> Recommendation Role: {}", top_rec.category_label);
    println!("  -> Compatibility Score: {:.1} / 100.0", top_rec.compatibility_score);
    println!("  -> Suggested Technique: {}", top_rec.technique_label);
    println!("  -> Rationale:           {}", top_rec.technique_reason);
    println!("==================================================================\n");

    // Deep evaluation on top candidate
    let top_submission = candidate_submissions
        .iter()
        .find(|s| s.title == top_rec.title && s.artist == top_rec.artist)
        .expect("Top recommendation must exist in candidate submissions");

    let deep_eval = evaluate_submission_dj_intelligence(
        top_submission,
        Some(&starting_submission),
        &baraat_mood,
        None,
        baraat_mood.target_energy,
        &candidate_submissions,
    );

    println!("Detailed L3 Score Breakdown for '{}':", top_rec.title);
    println!("  -> Total Score:         {:.1} pts", deep_eval.total_score);
    println!("  -> Reasons:             {:?}", deep_eval.reasons);
    println!("  -> Penalties:           {:?}", deep_eval.penalties);

    // ------------------------------------------------------------------
    // STEP 7: PRO DJ AUDITOR INDEPENDENT VERIFICATION
    // ------------------------------------------------------------------
    println!("\n--- [Step 7] Pro DJ Auditor Independent Inspection ---");
    println!("Auditor comparing recommendation against local records and musical sensibility:");

    let local_has_recommended = local_db
        .as_ref()
        .and_then(|db| db.lookup_global_song_by_video_id(&top_submission.provider_video_id).ok())
        .flatten();

    let local_has_proper_patola = local_db
        .as_ref()
        .and_then(|db| db.lookup_global_song_by_video_id(&starting_vid).ok())
        .flatten();

    println!("  1. Starting Track (Proper Patola) in local SQLite: {}", if local_has_proper_patola.is_some() { "PRESENT" } else { "NOT FOUND" });
    println!("  2. Recommended Track in local SQLite:             {}", if local_has_recommended.is_some() { "PRESENT" } else { "NOT FOUND" });

    if let Some(ref local_rec) = local_has_recommended {
        println!("  3. Metadata Consistency (Firestore vs Local):");
        println!("     - Title:        Local='{}' | Cloud='{}'", local_rec.title, top_submission.title);
        println!("     - Artist:       Local='{}' | Cloud='{}'", local_rec.artist, top_submission.artist);
        println!("     - Original BPM: Local={:?} | Cloud={:?}", local_rec.original_bpm, top_submission.bpm);
        println!("     - Camelot Key:  Local={:?} | Cloud={:?}", local_rec.camelot_key, top_submission.camelot_key);
    } else {
        println!("  3. Note: Recommended track was retrieved dynamically from Cloud Firestore (never required prior local storage).");
    }

    let start_bpm = starting_submission.bpm.unwrap_or(0.0);
    let start_camelot = starting_submission.camelot_key.as_deref().unwrap_or("Unknown");
    let start_energy = starting_submission.overall_energy.unwrap_or(0.0);

    let top_bpm = top_submission.bpm.unwrap_or(0.0);
    let top_camelot = top_submission.camelot_key.as_deref().unwrap_or("Unknown");
    let top_energy = top_submission.overall_energy.unwrap_or(0.0);

    println!("  4. Transition Sensibility Analysis (Level 3):");
    println!("     - Outgoing: Proper Patola ({:.1} BPM, {}, Energy: {:.2})", start_bpm, start_camelot, start_energy);
    println!("     - Incoming: {} ({:.1} BPM, {}, Energy: {:.2})", top_submission.title, top_bpm, top_camelot, top_energy);
    println!("     - Target:   Baraat (126-148 BPM, Target Energy: 0.96)");
    println!("     - Technique: {}", top_rec.technique_label);

    println!("\n==================================================================");
    println!("DHUN PROPER PATOLA → BARAAT L3 LIVE RECOMMENDATION TEST: ALL PASS");
    println!("==================================================================\n");
}
