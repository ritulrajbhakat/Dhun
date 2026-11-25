use dhun_lib::metadata_service::{compute_canonical_fingerprint, extract_version_tag, CatalogueUpsertRequest};
use dhun_lib::sync::firestore::{from_firestore_document, to_firestore_document, FirestoreConfig};
use dhun_lib::sync::MetadataSyncManager;

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
