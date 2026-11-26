use std::path::PathBuf;
use std::time::Instant;

use dhun_lib::db::models::{LocalPartyRoom, LocalRoomSubmission};
use dhun_lib::db::Database;
use dhun_lib::dj::levels::DjLevel;
use dhun_lib::dj::moods::get_preset_moods;
use dhun_lib::dj::selector::{choose_technique_for_pair, evaluate_submission_dj_intelligence};
use dhun_lib::sync::firestore::{from_firestore_document, FirestoreClient, FirestoreConfig};

#[tokio::test]
async fn test_baraat_l3_autonomous_party_workflow() {
    println!("\n==================================================================");
    println!("DHUN — AUTONOMOUS PARTY START & QUEUE POPULATION ACCEPTANCE TEST");
    println!("Scenario: Empty SQLite → Firestore Ingestion → 10-Track Baraat L3 Set");
    println!("==================================================================\n");

    // ------------------------------------------------------------------
    // STEP 1: INITIALIZE FRESH, EMPTY LOCAL SQLITE DATABASE
    // ------------------------------------------------------------------
    println!("--- [Step 1] Initializing Fresh, Empty Local SQLite Database ---");
    let test_run_id = chrono::Utc::now().timestamp_millis();
    let temp_db_dir = std::env::temp_dir();
    let db_path = temp_db_dir.join(format!("dhun_empty_party_test_{}.db", test_run_id));

    println!("Creating fresh isolated database at: {:?}", db_path);
    let db = Database::new(&db_path).expect("Failed to initialize fresh SQLite database");

    // Verify 100% empty pre-flight condition
    let initial_playable = db.get_playable_candidates().unwrap_or_default();
    let initial_submissions = db.get_local_room_submissions("room_live_baraat_party").unwrap_or_default();
    println!("Pre-Flight Verification:");
    println!("  -> Global catalogue tracks in SQLite: {}", initial_playable.len());
    println!("  -> Room submissions in SQLite:        {}", initial_submissions.len());
    assert_eq!(initial_playable.len(), 0, "Initial SQLite catalogue must be empty");
    assert_eq!(initial_submissions.len(), 0, "Initial SQLite queue must be empty");
    println!("  -> CONFIRMED: Local SQLite is 100% empty. Zero pre-cached tracks exist.\n");

    // Create Local Party Room
    let room_id = "room_live_baraat_party".to_string();
    let room = LocalPartyRoom {
        room_id: room_id.clone(),
        room_code: "BARAAT".to_string(),
        party_name: "Live Cloud Baraat Street Procession".to_string(),
        status: "active".to_string(),
        dj_level: 3, // Level 3 Advanced Performance
        primary_mood_id: "baraat".to_string(),
        secondary_mood_id: None,
        host_token_hash: "host_token_hash_secret".to_string(),
        room_access_token_hash: "room_token_hash_guest".to_string(),
        created_at: test_run_id,
        started_at: Some(test_run_id),
        closed_at: None,
        updated_at: test_run_id,
    };
    db.create_local_party_room(&room).expect("Failed to create local party room");
    println!("-> Created local party room: '{}' (Mood: Baraat, DJ Level: L3)", room.room_id);

    // ------------------------------------------------------------------
    // STEP 2: AUTHENTICATE & RETRIEVE CANDIDATES FROM LIVE FIRESTORE
    // ------------------------------------------------------------------
    println!("\n--- [Step 2] Authenticating & Fetching Candidates from Live Cloud Firestore ---");
    dhun_lib::sync::load_dotenv_if_present();

    let project_id = std::env::var("DHUN_FIREBASE_PROJECT_ID")
        .or_else(|_| std::env::var("VITE_FIREBASE_PROJECT_ID"))
        .unwrap_or_else(|_| "dhun-716e6".to_string());
    let api_key = std::env::var("DHUN_FIREBASE_API_KEY")
        .or_else(|_| std::env::var("VITE_FIREBASE_API_KEY"))
        .ok()
        .filter(|k| !k.trim().is_empty());

    assert!(api_key.is_some(), "DHUN_FIREBASE_API_KEY must be configured");

    let client = FirestoreClient::new(FirestoreConfig {
        project_id: project_id.clone(),
        api_key,
    });

    let auth_start = Instant::now();
    let auth_token = client
        .authenticate_anonymous()
        .await
        .expect("Anonymous authentication against Google Identity Toolkit failed");
    let auth_duration = auth_start.elapsed();
    let is_correct_project = client.verify_token_project(&auth_token).unwrap_or(false);
    println!("-> Authentication: SUCCESS in {:.1} ms", auth_duration.as_secs_f64() * 1000.0);
    println!("-> Verified Project ID: {} (aud claim match: {})", project_id, is_correct_project);
    assert!(is_correct_project);

    // Record exact Firestore REST query
    let query_url = format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/catalogue?pageSize=300",
        project_id
    );
    println!("-> Executing Firestore Query: GET {}", query_url);
    let fetch_start = Instant::now();
    let raw_docs = client
        .list_catalogue(300)
        .await
        .expect("Failed to list catalogue from Cloud Firestore");
    let fetch_duration = fetch_start.elapsed();
    println!("-> Firestore REST Response: Retrieved {} total raw documents in {:.1} ms", raw_docs.len(), fetch_duration.as_secs_f64() * 1000.0);

    let mut canonical_candidates = Vec::new();
    let mut alias_vid_count = 0;
    let mut alias_rec_count = 0;

    for (doc_id, doc_json) in raw_docs {
        if doc_id.starts_with("vid_") {
            alias_vid_count += 1;
            continue;
        }
        if doc_id.starts_with("rec_") {
            alias_rec_count += 1;
            continue;
        }
        if !doc_id.starts_with("fp_") {
            continue;
        }

        if let Some(meta) = from_firestore_document(&doc_json) {
            canonical_candidates.push((doc_id, meta));
        }
    }

    println!("Document Partitioning Evidence:");
    println!("  -> Excluded YouTube Video ID aliases (vid_*): {}", alias_vid_count);
    println!("  -> Excluded Recording ID aliases (rec_*):      {}", alias_rec_count);
    println!("  -> Canonical Primary Candidates (fp_*):        {}", canonical_candidates.len());
    assert!(
        canonical_candidates.len() >= 50,
        "Expected at least 50 canonical candidate tracks from Cloud Firestore, found {}",
        canonical_candidates.len()
    );

    // ------------------------------------------------------------------
    // STEP 3: CONVERT CLOUD METADATA TO UNPLAYED CANDIDATES
    // ------------------------------------------------------------------
    println!("\n--- [Step 3] Converting Cloud Metadata to Unplayed Candidate Pool ---");
    let candidate_submissions: Vec<LocalRoomSubmission> = canonical_candidates
        .iter()
        .map(|(doc_id, meta)| {
            let vid = meta
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
                .unwrap_or_else(|| format!("vid_{}", &meta.canonical_fingerprint[..8]));

            LocalRoomSubmission {
                id: format!("sub_{}", &meta.canonical_fingerprint[..12]),
                room_id: room_id.clone(),
                submitted_by_type: "guest".to_string(),
                submitted_by_id: format!("guest_{}", &meta.canonical_fingerprint[..6]),
                submitted_by_name: "Crowd".to_string(),
                provider: "youtube".to_string(),
                source_url: meta.source_urls.first().cloned().unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", vid)),
                provider_video_id: vid,
                song_id: Some(meta.song_id.clone()),
                title: meta.title.clone(),
                artist: meta.artist.clone(),
                version_tag: meta.version_tag.clone(),
                thumbnail_url: None,
                duration_ms: Some(meta.duration_ms),
                bpm: meta.original_bpm,
                half_time_bpm: meta.half_time_bpm,
                double_time_bpm: meta.double_time_bpm,
                musical_key: meta.musical_key.clone(),
                camelot_key: meta.camelot_key.clone(),
                overall_energy: meta.overall_energy,
                danceability: meta.danceability,
                genre_tags_json: serde_json::to_string(&meta.genre_tags).unwrap_or_else(|_| "[]".to_string()),
                mood_tags_json: serde_json::to_string(&meta.mood_tags).unwrap_or_else(|_| "[]".to_string()),
                metadata_source: "firestore_live_catalogue".to_string(),
                acquisition_status: "cached_authorized".to_string(),
                acquisition_note: None,
                dj_status: "queued_for_dj".to_string(),
                dj_score: 0.0,
                local_cache_path: None,
                cache_id: Some(format!("cache_{}", doc_id)),
                created_at: meta.created_at,
                updated_at: meta.updated_at,
            }
        })
        .collect();

    println!("Converted {} Cloud Firestore tracks into candidate submissions.", candidate_submissions.len());

    // ------------------------------------------------------------------
    // STEP 4: AUTONOMOUS DJ PARTY START — OPENING TRACK SELECTION
    // ------------------------------------------------------------------
    println!("\n--- [Step 4] Autonomous Opening Track Selection (Party Start: Track 1) ---");
    let preset_moods = get_preset_moods();
    let baraat_mood = preset_moods
        .iter()
        .find(|m| m.id == "baraat")
        .expect("Baraat mood profile must exist in DHUN")
        .clone();

    let target_energy = baraat_mood.target_energy; // 0.96

    // At party start, queue is empty, currently_playing is None
    let mut opening_scores: Vec<(LocalRoomSubmission, f64)> = candidate_submissions
        .iter()
        .map(|cand| {
            let eval = evaluate_submission_dj_intelligence(
                cand,
                None, // No track playing yet
                &baraat_mood,
                None,
                target_energy,
                &candidate_submissions,
            );
            (cand.clone(), eval.total_score)
        })
        .collect();

    opening_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (mut opening_track, opening_score) = opening_scores
        .first()
        .expect("Must find opening track from candidate pool")
        .clone();

    opening_track.dj_status = "playing".to_string(); // Deck A playing
    opening_track.dj_score = opening_score;
    db.insert_local_room_submission(&opening_track)
        .expect("Failed to persist opening track into SQLite");

    println!("OPENING TRACK SELECTED FOR BARAAT (Track 1):");
    println!("  -> Title:               '{}' by '{}'", opening_track.title, opening_track.artist);
    println!("  -> Score:               {:.1} pts", opening_score);
    println!("  -> BPM:                 {:?} BPM", opening_track.bpm);
    println!("  -> Camelot Key:         {:?}", opening_track.camelot_key);
    println!("  -> Overall Energy:      {:?}", opening_track.overall_energy);
    println!("  -> Status in SQLite:    {}", opening_track.dj_status);

    // ------------------------------------------------------------------
    // STEP 5: AUTONOMOUS QUEUE AUTO-POPULATION (TRACKS 2 THROUGH 10)
    // ------------------------------------------------------------------
    println!("\n--- [Step 5] Autonomous Queue Population (Tracks 2 to 10) ---");
    let mut selected_sequence: Vec<LocalRoomSubmission> = vec![opening_track.clone()];
    let mut transition_records: Vec<(String, String, String)> = Vec::new(); // (From -> To, Technique, Reason)

    let mut selected_fingerprints: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(sid) = &opening_track.song_id {
        selected_fingerprints.insert(sid.clone());
    }

    let mut recent_artists: Vec<String> = vec![opening_track.artist.to_lowercase()];

    for step in 2..=10 {
        let last_selected = selected_sequence.last().unwrap();

        // Available unselected candidates
        let available_candidates: Vec<&LocalRoomSubmission> = candidate_submissions
            .iter()
            .filter(|c| {
                let sid = c.song_id.as_deref().unwrap_or("");
                !selected_fingerprints.contains(sid) && c.title != last_selected.title
            })
            .collect();

        assert!(!available_candidates.is_empty(), "Must have available candidates to fill queue");

        let unplayed_slice: Vec<LocalRoomSubmission> = available_candidates.iter().map(|&c| c.clone()).collect();

        let mut scored_candidates: Vec<(LocalRoomSubmission, f64, String, String)> = available_candidates
            .iter()
            .map(|&cand| {
                let eval = evaluate_submission_dj_intelligence(
                    cand,
                    Some(last_selected),
                    &baraat_mood,
                    None,
                    target_energy,
                    &unplayed_slice,
                );

                let mut total_score = eval.total_score;

                // Repetition penalty for immediate consecutive artist
                if recent_artists.last() == Some(&cand.artist.to_lowercase()) {
                    total_score -= 25.0;
                }

                let (tech_id, tech_reason) = choose_technique_for_pair(
                    DjLevel::Level3AdvancedPerformance,
                    Some(last_selected),
                    cand,
                    &baraat_mood,
                    0.95,
                );

                (cand.clone(), total_score, tech_id, tech_reason)
            })
            .collect();

        scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (mut chosen, score, tech_id, tech_reason) = scored_candidates
            .first()
            .expect("Must pick next track in sequence")
            .clone();

        // Position 2 is "prepared" (Deck B preloaded), Positions 3-10 are "queued_for_dj"
        chosen.dj_status = if step == 2 {
            "prepared".to_string()
        } else {
            "queued_for_dj".to_string()
        };
        chosen.dj_score = score;

        db.insert_local_room_submission(&chosen)
            .expect("Failed to insert queued track into SQLite");

        if let Some(sid) = &chosen.song_id {
            selected_fingerprints.insert(sid.clone());
        }
        recent_artists.push(chosen.artist.to_lowercase());

        let pair_label = format!("{} → {}", last_selected.title, chosen.title);
        transition_records.push((pair_label, tech_id.clone(), tech_reason.clone()));
        selected_sequence.push(chosen.clone());

        println!(
            "Track #{:02}: [{:13}] {:.1} pts | '{}' by '{}' | {:?} BPM | {:?} Key | Transition: {}",
            step,
            chosen.dj_status,
            score,
            chosen.title,
            chosen.artist,
            chosen.bpm.unwrap_or(0.0),
            chosen.camelot_key.as_deref().unwrap_or("??"),
            tech_id
        );
    }

    // ------------------------------------------------------------------
    // STEP 6: VERIFY LOCAL SQLITE QUEUE CONTENTS & PERSISTENCE
    // ------------------------------------------------------------------
    println!("\n--- [Step 6] Inspecting Local SQLite Queue State ---");
    let stored_submissions = db
        .get_local_room_submissions(&room_id)
        .expect("Failed to query local room submissions from SQLite");

    println!("Total Room Submissions in SQLite: {}", stored_submissions.len());
    assert_eq!(stored_submissions.len(), 10, "SQLite database must contain exactly 10 tracks");

    let playing_count = stored_submissions.iter().filter(|s| s.dj_status == "playing").count();
    let prepared_count = stored_submissions.iter().filter(|s| s.dj_status == "prepared").count();
    let queued_count = stored_submissions.iter().filter(|s| s.dj_status == "queued_for_dj").count();

    println!("SQLite Track Status Breakdown:");
    println!("  -> Active Playing (Deck A):  {}", playing_count);
    println!("  -> Prepared (Deck B Preload): {}", prepared_count);
    println!("  -> Queued for DJ:            {}", queued_count);

    assert_eq!(playing_count, 1, "Exactly 1 track must be playing");
    assert_eq!(prepared_count, 1, "Exactly 1 track must be prepared");
    assert_eq!(queued_count, 8, "Exactly 8 tracks must be queued for DJ");

    // ------------------------------------------------------------------
    // STEP 7: PRINT COMPLETE 10-TRACK AUTONOMOUS SET PROGRESSION
    // ------------------------------------------------------------------
    println!("\n==================================================================");
    println!("DHUN AUTONOMOUS BARAAT L3 10-TRACK DJ SET PROGRESSION");
    println!("==================================================================");
    for (i, track) in stored_submissions.iter().enumerate() {
        let bpm = track.bpm.unwrap_or(0.0);
        let key = track.camelot_key.as_deref().unwrap_or("??");
        let energy = track.overall_energy.unwrap_or(0.0);
        let genres = &track.genre_tags_json;
        println!(
            "Pos #{:02} [{:13}]: '{}' — {} | {:.1} BPM | Key: {} | Energy: {:.2} | Score: {:.1}",
            i + 1,
            track.dj_status,
            track.title,
            track.artist,
            bpm,
            key,
            energy,
            track.dj_score
        );
        if i > 0 {
            let (_, ref tech, ref reason) = transition_records[i - 1];
            println!("       └─ Transition from Track #{:02}: [{}] {}", i, tech, reason);
        }
    }
    println!("==================================================================\n");

    // Clean up temporary database file
    drop(db);
    let _ = std::fs::remove_file(&db_path);
    println!("Temporary isolated SQLite database removed successfully: {:?}", db_path);
    println!("DHUN AUTONOMOUS BARAAT L3 PARTY WORKFLOW ACCEPTANCE TEST: ALL PASS\n");
}
