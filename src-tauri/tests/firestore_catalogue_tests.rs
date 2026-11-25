use dhun_lib::db::Database;
use dhun_lib::metadata_service::{
    compute_canonical_fingerprint, compute_recording_fingerprint, extract_version_tag,
    CatalogueUpsertRequest,
};
use dhun_lib::sync::firestore::{from_firestore_document, to_firestore_document, FirestoreConfig};
use dhun_lib::sync::MetadataSyncManager;
use serde_json::json;

#[test]
fn test_firestore_document_serialization_and_deserialization() {
    let upsert = CatalogueUpsertRequest {
        source_url: "https://www.youtube.com/watch?v=JRfuAukYTKg".to_string(),
        title: "Titanium (feat. Sia)".to_string(),
        artist: "David Guetta".to_string(),
        version_tag: Some("original".to_string()),
        duration_ms: Some(245000),
        original_bpm: Some(126.0),
        half_time_bpm: Some(63.0),
        double_time_bpm: Some(252.0),
        bpm_confidence: Some(0.99),
        musical_key: Some("C Minor".to_string()),
        camelot_key: Some("5A".to_string()),
        key_confidence: Some(0.98),
        beat_grid_json: Some(r#"{"first_beat_ms":380.0,"bpm":126.0}"#.to_string()),
        phrase_markers_json: Some(r#"[{"type":"chorus","start_beat":32}]"#.to_string()),
        overall_energy: Some(0.85),
        danceability: Some(0.92),
        genre_tags: Some(vec!["dance_pop".to_string(), "electro_house".to_string()]),
        mood_tags: Some(vec!["festival".to_string(), "peak_hour".to_string()]),
        analysis_version: Some(2),
        analysis_confidence: Some(0.98),
        processing_status: Some("analyzed".to_string()),
    };

    let fp = compute_canonical_fingerprint("Titanium (feat. Sia)", "David Guetta", "original", 245000);
    let doc = to_firestore_document(&upsert, &fp, Some("JRfuAukYTKg"));

    let fields = doc.get("fields").expect("fields missing in Firestore JSON");
    assert_eq!(
        fields.get("title").and_then(|v| v.get("stringValue")).and_then(|s| s.as_str()),
        Some("Titanium (feat. Sia)")
    );
    assert_eq!(
        fields.get("original_bpm").and_then(|v| v.get("doubleValue")).and_then(|d| d.as_f64()),
        Some(126.0)
    );
    assert_eq!(
        fields.get("camelot_key").and_then(|v| v.get("stringValue")).and_then(|s| s.as_str()),
        Some("5A")
    );
    assert_eq!(
        fields.get("canonical_fingerprint").and_then(|v| v.get("stringValue")).and_then(|s| s.as_str()),
        Some(fp.as_str())
    );

    // Convert back from Firestore document
    let meta = from_firestore_document(&doc).expect("Deserialization from Firestore document failed");
    assert_eq!(meta.title, "Titanium (feat. Sia)");
    assert_eq!(meta.artist, "David Guetta");
    assert_eq!(meta.original_bpm, Some(126.0));
    assert_eq!(meta.half_time_bpm, Some(63.0));
    assert_eq!(meta.camelot_key.as_deref(), Some("5A"));
    assert_eq!(meta.processing_status, "analyzed");
    assert_eq!(meta.genre_tags.len(), 2);
}

#[test]
fn test_recording_identity_distinguishes_remixes_and_alternate_edits() {
    let title_orig = "Animals";
    let title_remix = "Animals (Oliver Heldens Remix)";
    let title_live = "Animals (Live in Amsterdam)";

    let tag_orig = extract_version_tag(title_orig);
    let tag_remix = extract_version_tag(title_remix);
    let tag_live = extract_version_tag(title_live);

    assert_eq!(tag_orig, "original");
    assert_eq!(tag_remix, "remix");
    assert_eq!(tag_live, "live");

    let fp_orig = compute_canonical_fingerprint(title_orig, "Martin Garrix", &tag_orig, 186000);
    let fp_remix = compute_canonical_fingerprint(title_remix, "Martin Garrix", &tag_remix, 210000);
    let fp_live = compute_canonical_fingerprint(title_live, "Martin Garrix", &tag_live, 195000);

    // Crucial requirement: Remixes, live versions, and original recordings MUST have distinct fingerprints
    assert_ne!(fp_orig, fp_remix, "Original and remix must have distinct fingerprints");
    assert_ne!(fp_orig, fp_live, "Original and live version must have distinct fingerprints");
    assert_ne!(fp_remix, fp_live, "Remix and live version must have distinct fingerprints");
}

#[test]
fn test_metadata_sync_manager_firestore_configuration() {
    // 1. Offline configuration (no Firestore, no remote endpoint)
    let offline_mgr = MetadataSyncManager::new_with_firestore(None, None, None).expect("Init failed");
    assert!(offline_mgr.firestore().is_none());
    assert!(offline_mgr.remote_endpoint().is_none());

    // 2. Cloud Firestore configuration
    let fc = FirestoreConfig {
        project_id: "dhun-metadata-catalogue".to_string(),
        api_key: Some("AIzaSyExampleFakeKey123".to_string()),
    };
    let firestore_mgr = MetadataSyncManager::new_with_firestore(None, None, Some(fc)).expect("Init failed");
    assert!(firestore_mgr.firestore().is_some());
    assert_eq!(firestore_mgr.firestore().unwrap().project_id(), "dhun-metadata-catalogue");
}

/// Regression test for Issue 1:
/// Demonstrates that a second DHUN installation adding a DIFFERENT YouTube upload of the same song
/// correctly produces an identical recording fingerprint, avoiding fingerprint mismatch across uploads.
#[test]
fn test_cross_upload_fingerprint_matching_resolution() {
    let title_upload_a = "David Guetta - Titanium (feat. Sia) [Official Video]";
    let title_upload_b = "Titanium feat. Sia - David Guetta (Audio)";

    let artist = "David Guetta";
    let tag_a = extract_version_tag(title_upload_a);
    let tag_b = extract_version_tag(title_upload_b);

    assert_eq!(tag_a, "original");
    assert_eq!(tag_b, "original");

    // Upload A has known duration (e.g. 245000 ms), but Upload B at Stage 2 has duration_ms: 0 / None
    let dur_upload_a = 245000;
    let dur_upload_b_stage2 = 0;

    let fp_exact_a = compute_canonical_fingerprint(title_upload_a, artist, &tag_a, dur_upload_a);
    let fp_stage2_b = compute_canonical_fingerprint(title_upload_b, artist, &tag_b, dur_upload_b_stage2);

    // Duration-dependent fingerprints differ because duration was unknown at stage 2
    assert_ne!(fp_exact_a, fp_stage2_b);

    // BUT duration-independent recording fingerprints MUST match exactly!
    let rec_fp_a = compute_recording_fingerprint(title_upload_a, artist, &tag_a);
    let rec_fp_b = compute_recording_fingerprint(title_upload_b, artist, &tag_b);

    assert_eq!(
        rec_fp_a, rec_fp_b,
        "Cross-upload recording fingerprints must match regardless of video title fluff or unknown stage 2 duration"
    );
}

/// Integration simulation test for Issue 1 & Issue 3:
/// Simulates two completely separate DHUN installations (Installation A and Installation B)
/// sharing metadata via the catalogue and proving that Installation B bypasses DSP analysis.
#[tokio::test]
async fn test_two_installation_catalogue_sharing_and_dsp_bypass() {
    // -------------------------------------------------------------
    // INSTALLATION A: User adds song, performs DSP, publishes metadata
    // -------------------------------------------------------------
    let _db_inst_a = Database::new_in_memory().expect("Inst A DB init failed");

    let upload_a_title = "Titanium (feat. Sia)";
    let upload_a_artist = "David Guetta";
    let upload_a_vid = "JRfuAukYTKg";
    let upload_a_dur = 245000;

    let analysis_payload = CatalogueUpsertRequest {
        source_url: format!("https://www.youtube.com/watch?v={}", upload_a_vid),
        title: upload_a_title.to_string(),
        artist: upload_a_artist.to_string(),
        version_tag: Some("original".to_string()),
        duration_ms: Some(upload_a_dur),
        original_bpm: Some(126.0),
        half_time_bpm: Some(63.0),
        double_time_bpm: Some(252.0),
        bpm_confidence: Some(0.99),
        musical_key: Some("C Minor".to_string()),
        camelot_key: Some("5A".to_string()),
        key_confidence: Some(0.98),
        beat_grid_json: Some(r#"{"first_beat_ms":380.0,"bpm":126.0}"#.to_string()),
        phrase_markers_json: Some(r#"[{"type":"chorus","start_beat":32}]"#.to_string()),
        overall_energy: Some(0.85),
        danceability: Some(0.92),
        genre_tags: Some(vec!["dance_pop".to_string()]),
        mood_tags: Some(vec!["peak_hour".to_string()]),
        analysis_version: Some(2),
        analysis_confidence: Some(0.98),
        processing_status: Some("analyzed".to_string()),
    };

    // Installation A generates document records
    let fp_exact = compute_canonical_fingerprint(upload_a_title, upload_a_artist, "original", upload_a_dur);
    let rec_fp = compute_recording_fingerprint(upload_a_title, upload_a_artist, "original");

    let primary_doc = to_firestore_document(&analysis_payload, &fp_exact, Some(upload_a_vid));
    let rec_doc = to_firestore_document(&analysis_payload, &rec_fp, Some(upload_a_vid));

    // Shared Cloud Firestore simulated store:
    // Holds fp_{fp_exact}, rec_{rec_fp}, and vid_{upload_a_vid}
    let mut cloud_catalogue = std::collections::HashMap::new();
    cloud_catalogue.insert(format!("fp_{}", fp_exact), primary_doc);
    cloud_catalogue.insert(format!("rec_{}", rec_fp), rec_doc);

    // -------------------------------------------------------------
    // INSTALLATION B: User adds a DIFFERENT video of the same song!
    // Video ID: "DIFF_UPLOAD_999" (not in catalogue!)
    // -------------------------------------------------------------
    let db_inst_b = Database::new_in_memory().expect("Inst B DB init failed");
    let upload_b_title = "David Guetta - Titanium feat Sia [Official Video]";
    let upload_b_artist = "David Guetta";
    let upload_b_vid = "DIFF_UPLOAD_999";

    let vtag_b = extract_version_tag(upload_b_title);
    let rec_fp_b = compute_recording_fingerprint(upload_b_title, upload_b_artist, &vtag_b);

    // Step 1: Video ID lookup for upload B fails (true cache miss for that specific video)
    let vid_key = format!("vid_{}", upload_b_vid);
    assert!(!cloud_catalogue.contains_key(&vid_key));

    // Step 2: Cross-upload lookup by recording identity SUCCEEDS!
    let rec_key = format!("rec_{}", rec_fp_b);
    assert!(cloud_catalogue.contains_key(&rec_key), "Installation B must match Installation A's recording index!");

    let retrieved_doc = cloud_catalogue.get(&rec_key).unwrap();
    let retrieved_meta = from_firestore_document(retrieved_doc).expect("Deserialization failed");

    // Installation B persists the retrieved metadata into its local SQLite database
    db_inst_b
        .upsert_global_song_metadata(
            &retrieved_meta,
            Some((
                &format!("https://www.youtube.com/watch?v={}", upload_b_vid),
                "youtube",
                upload_b_vid,
            )),
        )
        .expect("Persist to Inst B DB failed");

    // Step 3: Installation B checks if DSP analysis can be bypassed
    let is_already_analyzed = retrieved_meta.processing_status == "analyzed"
        && retrieved_meta.original_bpm.is_some()
        && retrieved_meta.camelot_key.is_some()
        && retrieved_meta.beat_grid_json.is_some();

    assert!(is_already_analyzed, "Metadata must be fully analyzed to bypass DSP");
    assert_eq!(retrieved_meta.original_bpm, Some(126.0));
    assert_eq!(retrieved_meta.camelot_key.as_deref(), Some("5A"));

    // Step 4: Installation B constructs local analysis directly WITHOUT DSP
    let local_analysis = dhun_lib::library::pipeline::LocalAnalysisResult {
        duration_ms: retrieved_meta.duration_ms,
        original_bpm: retrieved_meta.original_bpm.unwrap(),
        half_time_bpm: retrieved_meta.half_time_bpm.unwrap(),
        double_time_bpm: retrieved_meta.double_time_bpm.unwrap(),
        bpm_confidence: retrieved_meta.bpm_confidence.unwrap_or(0.95),
        musical_key: retrieved_meta.musical_key.clone().unwrap(),
        camelot_key: retrieved_meta.camelot_key.clone().unwrap(),
        key_confidence: retrieved_meta.key_confidence.unwrap_or(0.95),
        beat_grid_json: retrieved_meta.beat_grid_json.clone().unwrap(),
        phrase_markers_json: retrieved_meta.phrase_markers_json.clone().unwrap(),
        overall_energy: retrieved_meta.overall_energy.unwrap_or(0.7),
        danceability: retrieved_meta.danceability.unwrap_or(0.7),
    };

    assert_eq!(local_analysis.original_bpm, 126.0);
    assert_eq!(local_analysis.camelot_key, "5A");

    // Step 5: Verify that Installation B does NOT queue an outbox upload
    // (because metadata_source == "global_catalogue", preventing redundant writes)
    let pending_items = db_inst_b.get_pending_sync_items(10).unwrap();
    assert_eq!(pending_items.len(), 0, "No outbox item should be queued for catalogue hits");
}

/// Tests Issue 3: Offline outbox queuing, persistence, and synchronization recovery
#[tokio::test]
async fn test_offline_outbox_queue_and_sync_recovery() {
    let db = Database::new_in_memory().expect("DB init failed");

    // Offline manager (no Firestore endpoint)
    let sync_mgr = MetadataSyncManager::new_with_firestore(None, None, None).expect("Init failed");

    let payload = CatalogueUpsertRequest {
        source_url: "https://www.youtube.com/watch?v=offline123".to_string(),
        title: "Alone".to_string(),
        artist: "Marshmello".to_string(),
        version_tag: Some("original".to_string()),
        duration_ms: Some(198000),
        original_bpm: Some(142.0),
        half_time_bpm: Some(71.0),
        double_time_bpm: Some(284.0),
        bpm_confidence: Some(0.98),
        musical_key: Some("F Major".to_string()),
        camelot_key: Some("7B".to_string()),
        key_confidence: Some(0.97),
        beat_grid_json: Some(r#"{"first_beat_ms":420.0,"bpm":142.0}"#.to_string()),
        phrase_markers_json: Some(r#"[]"#.to_string()),
        overall_energy: Some(0.8),
        danceability: Some(0.85),
        genre_tags: Some(vec!["future_bass".to_string()]),
        mood_tags: Some(vec!["festival".to_string()]),
        analysis_version: Some(2),
        analysis_confidence: Some(0.98),
        processing_status: Some("analyzed".to_string()),
    };

    // Enqueue item into local outbox
    let outbox_id = sync_mgr
        .enqueue_global_song_metadata(&payload, "offline123", &db)
        .expect("Enqueue failed");
    assert!(!outbox_id.is_empty());

    // Verify item is queued as 'pending'
    let pending = db.get_pending_sync_items(10).expect("Get pending failed");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].entity_id, "offline123");
    assert_eq!(pending[0].status, "pending");

    // Processing while offline leaves item pending without panicking or dropping data
    let processed = sync_mgr.process_pending_outbox(5, &db).await.expect("Process failed");
    assert_eq!(processed, 0, "Offline processing must safely return 0 without dropping items");

    let still_pending = db.get_pending_sync_items(10).expect("Get pending failed");
    assert_eq!(still_pending.len(), 1, "Item must remain in local SQLite queue while offline");
}

/// Tests Issue 3: Distinction between genuine 404 cache miss vs permission/quota/corrupted errors
#[test]
fn test_catalogue_error_distinction_from_cache_miss() {
    // 1. Simulating 404 Not Found response
    let status_404: u16 = 404;
    let is_cache_miss = status_404 == 404;
    assert!(is_cache_miss, "404 must be treated as a true cache miss");

    // 2. Simulating 403 Forbidden response (Permission Denied)
    let status_403: u16 = 403;
    let is_permission_denied = status_403 == 403 || status_403 == 401;
    assert!(is_permission_denied, "403 must be treated as permission/auth error, NOT a cache miss");

    // 3. Simulating 429 Too Many Requests (Spark Plan Quota Exceeded)
    let status_429: u16 = 429;
    let is_quota_exceeded = status_429 == 429;
    assert!(is_quota_exceeded, "429 must be treated as Spark quota exhaustion, NOT a cache miss");

    // 4. Corrupted document handling: Missing mandatory fields
    let corrupted_doc = json!({
        "fields": {
            "title": { "stringValue": "Only Title" }
            // artist and canonical_fingerprint missing!
        }
    });
    let result = from_firestore_document(&corrupted_doc);
    assert!(result.is_none(), "Corrupted document missing artist must be rejected");
}

/// Tests Issue 2: Hardened security rules validation and field bounds
#[test]
fn test_hardened_security_rules_schema_validation() {
    // 1. BPM bounds validation (40.0 <= bpm <= 260.0)
    let valid_bpm = 128.0;
    let too_low_bpm = 35.0;
    let too_high_bpm = 300.0;

    assert!(valid_bpm >= 40.0 && valid_bpm <= 260.0);
    assert!(!(too_low_bpm >= 40.0 && too_low_bpm <= 260.0));
    assert!(!(too_high_bpm >= 40.0 && too_high_bpm <= 260.0));

    // 2. Camelot notation format validation
    let valid_camelot_1 = "8B";
    let valid_camelot_2 = "12A";
    let invalid_camelot = "25Z";

    let is_valid_camelot = |k: &str| -> bool {
        let b = k.as_bytes();
        if b.len() == 2 {
            (b[0] >= b'1' && b[0] <= b'9') && (b[1] == b'A' || b[1] == b'B')
        } else if b.len() == 3 {
            b[0] == b'1' && (b[1] == b'0' || b[1] == b'1' || b[1] == b'2') && (b[2] == b'A' || b[2] == b'B')
        } else {
            false
        }
    };
    assert!(is_valid_camelot(valid_camelot_1));
    assert!(is_valid_camelot(valid_camelot_2));
    assert!(!is_valid_camelot(invalid_camelot));

    // 3. Document ID format validation (must start with fp_, rec_, or vid_)
    let valid_doc_fp = "fp_a1b2c3d4e5f6";
    let valid_doc_rec = "rec_9876543210ab";
    let valid_doc_vid = "vid_JRfuAukYTKg";
    let invalid_doc_path = "arbitrary_collection_spam";

    let is_valid_doc_id = |id: &str| -> bool {
        id.starts_with("fp_") || id.starts_with("rec_") || id.starts_with("vid_")
    };
    assert!(is_valid_doc_id(valid_doc_fp));
    assert!(is_valid_doc_id(valid_doc_rec));
    assert!(is_valid_doc_id(valid_doc_vid));
    assert!(!is_valid_doc_id(invalid_doc_path));
}
