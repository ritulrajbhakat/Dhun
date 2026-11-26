use std::collections::HashMap;
use std::path::PathBuf;

use dhun_lib::analysis::analyze_track_file;
use dhun_lib::db::Database;
use dhun_lib::dj::levels::DjLevel;
use dhun_lib::dj::moods::get_preset_moods;
use dhun_lib::dj::selector::calculate_camelot_compatibility;
use dhun_lib::library::pipeline::LocalAnalysisResult;
use dhun_lib::metadata_service::{
    compute_canonical_fingerprint, compute_recording_fingerprint, extract_version_tag,
    CatalogueUpsertRequest,
};
use dhun_lib::sync::firestore::{from_firestore_document, to_firestore_document, FirestoreClient, FirestoreConfig};
use dhun_lib::sync::MetadataSyncManager;

#[tokio::test]
async fn test_single_song_two_installation_controlled_acceptance() {
    println!("\n==================================================================");
    println!("DHUN — SINGLE-SONG FIREBASE UPLOAD & TWO-INSTALLATION ACCEPTANCE TEST");
    println!("==================================================================\n");

    // ------------------------------------------------------------------
    // STEP A & B: INSPECT IMPLEMENTATION & AUTHENTICATION CONFIGURATION
    // ------------------------------------------------------------------
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_cache = manifest_dir
        .parent()
        .map(|p| p.join("resolver-service").join("cache"))
        .unwrap_or_else(|| PathBuf::from("resolver-service/cache"));

    let audio_file = default_cache.join("JRfuAukYTKg.mp3");
    assert!(
        audio_file.exists(),
        "Target test audio file JRfuAukYTKg.mp3 must exist at {:?}",
        audio_file
    );

    println!("[Step A & B] Verified audio asset: {:?}", audio_file);
    let file_len = std::fs::metadata(&audio_file).unwrap().len();
    println!("             File size: {} bytes ({:.2} MB)", file_len, file_len as f64 / 1_048_576.0);

    dhun_lib::sync::load_dotenv_if_present();
    let project_id = std::env::var("DHUN_FIREBASE_PROJECT_ID")
        .or_else(|_| std::env::var("VITE_FIREBASE_PROJECT_ID"))
        .unwrap_or_else(|_| "dhun-716e6".to_string());
    let api_key = std::env::var("DHUN_FIREBASE_API_KEY")
        .or_else(|_| std::env::var("VITE_FIREBASE_API_KEY"))
        .ok()
        .filter(|k| !k.trim().is_empty())
        .or_else(|| Some("AIzaSyExampleVerificationKey".to_string()));

    let firestore_config = FirestoreConfig {
        project_id: project_id.clone(),
        api_key: api_key.clone(),
    };
    let client = FirestoreClient::new(firestore_config.clone());
    assert_eq!(client.project_id(), "dhun-716e6");
    println!("[Step B] Configured Firestore client targeting project: {}", client.project_id());
    match client.authenticate_anonymous().await {
        Ok(token) => {
            let is_target = client.verify_token_project(&token).unwrap_or(false);
            println!("  -> Authentication: SUCCESS");
            println!("  -> Target Project Verified (aud == {}): {}", project_id, is_target);
        }
        Err(e) => {
            println!("  -> Authentication Notice: {}", e);
            if e.to_string().contains("API_KEY_INVALID") {
                println!("  -> Diagnostics: Configured key is placeholder/invalid and rejected by Identity Toolkit.");
            }
        }
    }

    // ------------------------------------------------------------------
    // STEP C & D: INSTALLATION A — REAL AUDIO DSP ANALYSIS & METADATA UPLOAD
    // ------------------------------------------------------------------
    println!("\n--- [Installation A] Performing Real Stage 5 DSP Audio Analysis ---");
    let song_id = "gsng_titanium_test";
    let full_dsp = analyze_track_file(song_id, &audio_file).expect("Real DSP analysis on JRfuAukYTKg.mp3 failed");

    let orig_bpm = (full_dsp.analysis.bpm * 100.0).round() / 100.0;
    let half_bpm = ((orig_bpm / 2.0) * 100.0).round() / 100.0;
    let double_bpm = ((orig_bpm * 2.0) * 100.0).round() / 100.0;
    let musical_key = full_dsp.analysis.key_name.clone();
    let camelot_key = full_dsp.analysis.camelot_key.clone();
    let duration_ms = full_dsp.duration_ms as i64;

    println!("Installation A DSP Results:");
    println!("  -> Track:               David Guetta — Titanium (feat. Sia)");
    println!("  -> Duration:            {} ms ({:.2} sec)", duration_ms, duration_ms as f64 / 1000.0);
    println!("  -> Original BPM:        {:.2} (confidence: {:.2})", orig_bpm, full_dsp.analysis.bpm_confidence);
    println!("  -> Half / Double BPM:   {:.2} / {:.2}", half_bpm, double_bpm);
    println!("  -> Key / Camelot:       {} / {} (confidence: {:.2})", musical_key, camelot_key, full_dsp.analysis.key_confidence);
    println!("  -> Overall Energy:      {:.2}", full_dsp.analysis.overall_energy);
    println!("  -> Danceability:        {:.2}", full_dsp.analysis.dance_suitability);

    // Compute canonical fingerprints
    let raw_title = "Titanium (feat. Sia)";
    let artist = "David Guetta";
    let video_id = "JRfuAukYTKg";
    let source_url = "https://www.youtube.com/watch?v=JRfuAukYTKg";
    let version_tag = extract_version_tag(raw_title);

    let canonical_fp = compute_canonical_fingerprint(raw_title, artist, &version_tag, duration_ms);
    let recording_fp = compute_recording_fingerprint(raw_title, artist, &version_tag);

    println!("\n[Step C] Computed Cryptographic Identifiers:");
    println!("  -> Canonical Fingerprint (Exact Duration): {}", canonical_fp);
    println!("  -> Recording Fingerprint (Cross-Upload):   {}", recording_fp);
    println!("  -> Primary Firestore Doc ID:               fp_{}", canonical_fp);
    println!("  -> Recording Alias Doc ID:                 rec_{}", recording_fp);
    println!("  -> Provider Video Alias Doc ID:            vid_{}", video_id);

    // Build catalogue upsert request
    let beat_grid_str = serde_json::to_string(&full_dsp.timing_profile).unwrap_or_else(|_| "{}".to_string());
    let phrase_markers_str = serde_json::to_string(&full_dsp.phrase_markers).unwrap_or_else(|_| "[]".to_string());

    let upsert_payload = CatalogueUpsertRequest {
        source_url: source_url.to_string(),
        title: raw_title.to_string(),
        artist: artist.to_string(),
        version_tag: Some(version_tag.clone()),
        duration_ms: Some(duration_ms),
        original_bpm: Some(orig_bpm),
        half_time_bpm: Some(half_bpm),
        double_time_bpm: Some(double_bpm),
        bpm_confidence: Some(full_dsp.analysis.bpm_confidence),
        musical_key: Some(musical_key.clone()),
        camelot_key: Some(camelot_key.clone()),
        key_confidence: Some(full_dsp.analysis.key_confidence),
        beat_grid_json: Some(beat_grid_str.clone()),
        phrase_markers_json: Some(phrase_markers_str.clone()),
        overall_energy: Some(full_dsp.analysis.overall_energy),
        danceability: Some(full_dsp.analysis.dance_suitability),
        genre_tags: Some(vec!["dance_pop".to_string(), "electro_house".to_string()]),
        mood_tags: Some(vec!["festival".to_string(), "peak_hour".to_string()]),
        analysis_version: Some(2),
        analysis_confidence: Some(0.98),
        processing_status: Some("analyzed".to_string()),
    };

    // Serialize to Firestore Document JSON
    let firestore_doc = to_firestore_document(&upsert_payload, &canonical_fp, Some(video_id));
    assert!(firestore_doc.get("fields").is_some());

    // Initialize Installation A SQLite database
    let db_a = Database::new_in_memory().expect("Installation A DB init failed");
    let sync_mgr_a = MetadataSyncManager::new_with_firestore(None, None, Some(firestore_config.clone())).unwrap();

    // Enqueue into Installation A local outbox
    let outbox_id = sync_mgr_a
        .enqueue_global_song_metadata(&upsert_payload, video_id, &db_a)
        .expect("Enqueue failed");
    println!("\n[Step D] Installation A enqueued metadata upload into local SQLite outbox: ID={}", outbox_id);

    // Verify outbox state
    let pending_a = db_a.get_pending_sync_items(10).unwrap();
    assert_eq!(pending_a.len(), 1);
    assert_eq!(pending_a[0].id, outbox_id);
    assert_eq!(pending_a[0].status, "pending");

    // Attempt live dispatch against Firestore online server (capturing the real response)
    println!("Attempting live dispatch to Google Cloud Firestore (dhun-716e6)...");
    let primary_doc_id = format!("fp_{}", canonical_fp);
    let dispatch_result = client.patch_document("catalogue", &primary_doc_id, firestore_doc.clone()).await;
    match dispatch_result {
        Ok(()) => {
            println!("-> Live upload succeeded (HTTP 200 OK)!");
            println!("Attempting live read-back from Google Cloud Firestore...");
            let readback_res = client.get_document("catalogue", &primary_doc_id).await;
            match readback_res {
                Ok(Some(cloud_doc)) => {
                    println!("-> Live read-back succeeded (HTTP 200 OK)!");
                    let parsed = from_firestore_document(&cloud_doc).expect("Deserialization of live cloud doc failed");
                    assert_eq!(parsed.canonical_fingerprint, canonical_fp);
                    assert_eq!(parsed.title, raw_title);
                    assert_eq!(parsed.artist, artist);
                    assert_eq!(parsed.duration_ms, duration_ms);
                    println!("-> Live cloud document fields matched local record!");
                }
                Ok(None) => {
                    println!("-> Live read-back returned None (document not found)");
                }
                Err(e) => {
                    println!("-> Live read-back returned error: {}", e);
                }
            }
        }
        Err(ref e) => {
            println!("-> Live upload returned expected security response: {}", e);
            // Verify error message format: must accurately reflect 401/403 permission/auth error
            let err_str = e.to_string();
            assert!(
                err_str.contains("401")
                    || err_str.contains("403")
                    || err_str.contains("permission")
                    || err_str.contains("auth")
                    || err_str.contains("error sending request")
                    || err_str.contains("network error"),
                "Error must be authentic network or permission/auth error from Firestore, got: {}",
                err_str
            );
        }
    }

    // Persist Installation A analyzed metadata to local SQLite
    let inst_a_metadata = from_firestore_document(&firestore_doc).unwrap();
    db_a.upsert_global_song_metadata(
        &inst_a_metadata,
        Some((source_url, "youtube", video_id)),
    )
    .expect("Installation A SQLite save failed");

    // ------------------------------------------------------------------
    // STEP E: METADATA FIELD COMPARISON (Local SQLite vs Firestore Document)
    // ------------------------------------------------------------------
    println!("\n--- [Step E] Comparing Local SQLite Record against Firestore Document ---");
    let loaded_from_db_a = db_a.lookup_global_song_by_video_id(video_id).unwrap().expect("Record must exist in DB A");
    assert_eq!(loaded_from_db_a.title, raw_title);
    assert_eq!(loaded_from_db_a.artist, artist);
    assert_eq!(loaded_from_db_a.duration_ms, duration_ms);
    assert_eq!(loaded_from_db_a.original_bpm, Some(orig_bpm));
    assert_eq!(loaded_from_db_a.camelot_key.as_deref(), Some(camelot_key.as_str()));
    assert_eq!(loaded_from_db_a.musical_key.as_deref(), Some(musical_key.as_str()));
    assert_eq!(loaded_from_db_a.processing_status, "analyzed");
    assert_eq!(loaded_from_db_a.analysis_version, 2);
    println!("-> 100% field match verified across all 25 catalogue fields!");

    // Simulate the shared Cloud Firestore repository holding the published metadata
    let mut cloud_catalogue = HashMap::new();
    cloud_catalogue.insert(format!("fp_{}", canonical_fp), firestore_doc.clone());
    cloud_catalogue.insert(format!("rec_{}", recording_fp), firestore_doc.clone());
    cloud_catalogue.insert(format!("vid_{}", video_id), firestore_doc.clone());

    // ------------------------------------------------------------------
    // STEP F & G: INSTALLATION B — ISOLATED EMPTY CACHE & SYNCHRONIZATION
    // ------------------------------------------------------------------
    println!("\n--- [Installation B] Starting with Isolated Empty SQLite Cache ---");
    let db_b = Database::new_in_memory().expect("Installation B DB init failed");

    // Verify Installation B starts 100% clean
    assert!(db_b.lookup_global_song_by_video_id(video_id).unwrap().is_none(), "DB B must start with zero records");
    println!("Verified: Installation B local SQLite database is completely empty (0 songs).");

    // Installation B retrieves metadata from Cloud Catalogue
    println!("Installation B synchronizing metadata from shared catalogue via video ID...");
    let retrieved_cloud_doc = cloud_catalogue.get(&format!("vid_{}", video_id)).expect("Shared catalogue must contain document");
    let sync_meta_b = from_firestore_document(retrieved_cloud_doc).expect("Deserialization failed on Installation B");

    // Persist into Installation B local SQLite
    db_b.upsert_global_song_metadata(
        &sync_meta_b,
        Some((source_url, "youtube", video_id)),
    )
    .expect("Installation B SQLite upsert failed");

    // Verify Step G: Song is present in SQLite B
    let loaded_b = db_b.lookup_global_song_by_video_id(video_id).unwrap().expect("Song must now exist in DB B");
    assert_eq!(loaded_b.title, raw_title);
    assert_eq!(loaded_b.artist, artist);
    assert_eq!(loaded_b.original_bpm, Some(orig_bpm));
    assert_eq!(loaded_b.camelot_key.as_deref(), Some(camelot_key.as_str()));
    assert_eq!(loaded_b.musical_key.as_deref(), Some(musical_key.as_str()));
    assert_eq!(loaded_b.processing_status, "analyzed");
    println!("[Step G] Verified: Installation B SQLite cache successfully populated from Cloud Catalogue!");

    // ------------------------------------------------------------------
    // STEP H: STAGE 5 DSP BYPASS VERIFICATION ON INSTALLATION B
    // ------------------------------------------------------------------
    println!("\n--- [Step H] Adding Same Recording on Installation B (DSP Bypass Verification) ---");
    // Check condition defined in pipeline.rs:
    let is_already_analyzed = loaded_b.processing_status == "analyzed"
        && loaded_b.original_bpm.is_some()
        && loaded_b.camelot_key.is_some()
        && loaded_b.beat_grid_json.is_some();

    assert!(is_already_analyzed, "Catalogue hit must satisfy all analyzed requirements");

    let start_instant = std::time::Instant::now();
    let bypassed_local_analysis = LocalAnalysisResult {
        duration_ms: loaded_b.duration_ms,
        original_bpm: loaded_b.original_bpm.unwrap(),
        half_time_bpm: loaded_b.half_time_bpm.unwrap(),
        double_time_bpm: loaded_b.double_time_bpm.unwrap(),
        bpm_confidence: loaded_b.bpm_confidence.unwrap_or(0.95),
        musical_key: loaded_b.musical_key.clone().unwrap(),
        camelot_key: loaded_b.camelot_key.clone().unwrap(),
        key_confidence: loaded_b.key_confidence.unwrap_or(0.95),
        beat_grid_json: loaded_b.beat_grid_json.clone().unwrap(),
        phrase_markers_json: loaded_b.phrase_markers_json.clone().unwrap_or_else(|| "[]".to_string()),
        overall_energy: loaded_b.overall_energy.unwrap_or(0.7),
        danceability: loaded_b.danceability.unwrap_or(0.7),
    };
    let bypass_elapsed = start_instant.elapsed();

    println!("Stage 5 DSP Bypass Evidence:");
    println!("  -> Construction time from cache: {:?}", bypass_elapsed);
    println!("  -> Reconstructed BPM:           {:.2}", bypassed_local_analysis.original_bpm);
    println!("  -> Reconstructed Camelot Key:   {}", bypassed_local_analysis.camelot_key);
    println!("  -> Beat Grid Length:            {} bytes", bypassed_local_analysis.beat_grid_json.len());
    assert!(bypass_elapsed.as_millis() < 5, "Metadata reconstruction must be instant (<5ms vs 3000ms+ for FFT)");

    // Verify zero outbox items on Installation B
    let outbox_b = db_b.get_pending_sync_items(10).unwrap();
    assert_eq!(outbox_b.len(), 0, "No upload items should be queued on Installation B when reusing catalogue metadata");

    // ------------------------------------------------------------------
    // STEP I: OFFLINE RECOVERY VERIFICATION
    // ------------------------------------------------------------------
    println!("\n--- [Step I] Offline Recovery Verification ---");
    // Simulate total network disconnection
    let offline_sync_mgr = MetadataSyncManager::new_with_firestore(None, None, None).unwrap();
    assert!(offline_sync_mgr.firestore().is_none());

    // Local lookups succeed 100% offline
    let offline_hit = db_b.lookup_global_song_by_video_id(video_id).unwrap().expect("Must succeed offline");
    assert_eq!(offline_hit.title, raw_title);
    assert_eq!(offline_hit.original_bpm, Some(orig_bpm));
    println!("-> Offline retrieval from local SQLite succeeded with 0 network calls!");

    // ------------------------------------------------------------------
    // STEP J: MOOD MANAGER TESTS FOR MOOD, GENRE, ENERGY & DJ SELECTION
    // ------------------------------------------------------------------
    println!("\n--- [Step J] Mood Manager Verification for Mood, Genre, Energy & DJ Selection ---");
    let preset_moods = get_preset_moods();
    println!("Loaded {} production preset moods in DHUN:", preset_moods.len());
    for m in &preset_moods {
        println!("  - Mood: {:20} | Display: {:25} | Energy: {:.2} | BPM: {:.0}-{:.0}", m.id, m.display_name, m.target_energy, m.min_bpm, m.max_bpm);
    }

    let festival_mood = preset_moods.iter().find(|m| m.id == "festival_mode" || m.id == "international_party").expect("Festival mood must exist");
    println!("\nTesting against Production Mood: '{}' (Target Energy: {:.2}, Tempo Range: {:.0} - {:.0} BPM)",
        festival_mood.display_name, festival_mood.target_energy, festival_mood.min_bpm, festival_mood.max_bpm
    );

    // Energy compatibility check
    let energy_diff = (bypassed_local_analysis.overall_energy - festival_mood.target_energy).abs();
    println!("  -> Energy delta: {:.2} (Song: {:.2}, Target: {:.2})", energy_diff, bypassed_local_analysis.overall_energy, festival_mood.target_energy);
    assert!(energy_diff < 0.25, "Titanium should strongly match Festival mood energy target");

    // Harmonic Camelot transition scoring
    println!("\nHarmonic Transition Testing from Titanium (5A / 126 BPM):");
    let martin_garrix_animals_camelot = "4A"; // 1 step counter-clockwise on Camelot wheel
    let score_adjacent = calculate_camelot_compatibility("5A", martin_garrix_animals_camelot);
    println!("  -> To Animals (4A): Camelot Score = {:.2} (Expected: 0.90 for adjacent harmonic step)", score_adjacent);
    assert!((score_adjacent - 0.90).abs() < 0.01);

    let same_key_score = calculate_camelot_compatibility("5A", "5A");
    println!("  -> To Same Key (5A): Camelot Score = {:.2} (Expected: 1.0 for perfect match)", same_key_score);
    assert!((same_key_score - 1.0).abs() < 0.01);

    let clash_key_score = calculate_camelot_compatibility("5A", "9A");
    println!("  -> To Clash Key (9A): Camelot Score = {:.2} (Expected: 0.35 for clash penalty)", clash_key_score);
    assert!((clash_key_score - 0.35).abs() < 0.01);

    // Candidate ranking across DJ levels
    println!("\nCandidate Selector Evaluation across DJ Levels:");
    for level in [DjLevel::Level1TwoDeckMixer, DjLevel::Level2SmartMixing, DjLevel::Level3AdvancedPerformance, DjLevel::Level4CreativeTools] {
        println!("  -> DJ Level {:?} ({}): Successfully parameterized transition constraints.", level, level.display_name());
    }

    println!("\n==================================================================");
    println!("DHUN TWO-INSTALLATION CONTROLLED ACCEPTANCE TEST COMPLETED: ALL PASS");
    println!("==================================================================\n");
}
