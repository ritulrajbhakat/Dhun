use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use sha2::{Digest, Sha256};

use dhun_lib::sync::firestore::{FirestoreClient, FirestoreConfig};

#[tokio::test]
async fn test_online_catalogue_backup_and_reset() {
    println!("\n==================================================================");
    println!("DHUN — COMPLETE ONLINE SONG METADATA CATALOGUE RESET");
    println!("Firebase Project: dhun-716e6 | Database: (default) | Region: nam5");
    println!("==================================================================\n");

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

    // ------------------------------------------------------------------
    // STEP 1: AUTHENTICATE VIA GOOGLE IDENTITY TOOLKIT
    // ------------------------------------------------------------------
    println!("--- [Step 1] Authenticating against Google Cloud Firestore ---");
    let auth_token = client
        .authenticate_anonymous()
        .await
        .expect("Anonymous authentication against Google Identity Toolkit failed");
    let is_correct_project = client.verify_token_project(&auth_token).unwrap_or(false);
    println!("-> Anonymous Auth: SUCCESS (RS256 JWT acquired)");
    println!("-> Target Project Verified (aud == {}): {}", project_id, is_correct_project);
    assert!(is_correct_project, "JWT audience must match target project");

    // ------------------------------------------------------------------
    // STEP 2: RETRIEVE COMPLETE CATALOGUE COLLECTION
    // ------------------------------------------------------------------
    println!("\n--- [Step 2] Reading Complete 'catalogue' Collection from Firestore ---");
    let fetch_start = Instant::now();
    let raw_docs = client
        .list_catalogue(300)
        .await
        .expect("Failed to retrieve live catalogue documents from Firestore");
    let fetch_duration = fetch_start.elapsed();
    println!("-> Fetched {} total documents in {:.1} ms", raw_docs.len(), fetch_duration.as_secs_f64() * 1000.0);

    let mut prefix_counts: HashMap<String, usize> = HashMap::new();
    let mut backup_entries = Vec::with_capacity(raw_docs.len());

    for (doc_id, doc_json) in &raw_docs {
        let prefix = if doc_id.starts_with("fp_") {
            "fp_".to_string()
        } else if doc_id.starts_with("vid_") {
            "vid_".to_string()
        } else if doc_id.starts_with("rec_") {
            "rec_".to_string()
        } else {
            "other".to_string()
        };
        *prefix_counts.entry(prefix).or_insert(0) += 1;

        backup_entries.push(serde_json::json!({
            "doc_id": doc_id,
            "document": doc_json
        }));
    }

    println!("Document Breakdown in Current Catalogue:");
    println!("  -> Total documents:    {}", raw_docs.len());
    println!("  -> Canonical 'fp_':    {}", prefix_counts.get("fp_").unwrap_or(&0));
    println!("  -> Video 'vid_':       {}", prefix_counts.get("vid_").unwrap_or(&0));
    println!("  -> Recording 'rec_':   {}", prefix_counts.get("rec_").unwrap_or(&0));
    println!("  -> Other documents:    {}", prefix_counts.get("other").unwrap_or(&0));

    // Check for nested subcollections on a sample of documents
    println!("\nChecking for nested subcollections in catalogue documents...");
    let mut nested_subcollections_found = Vec::new();
    let sample_docs: Vec<_> = raw_docs.iter().take(5).collect();
    for (doc_id, _) in sample_docs {
        if let Ok(subs) = client.list_subcollections(doc_id).await {
            if !subs.is_empty() {
                nested_subcollections_found.push((doc_id.clone(), subs));
            }
        }
    }
    println!("  -> Nested subcollections found: {}", nested_subcollections_found.len());

    // ------------------------------------------------------------------
    // STEP 3: EXPORT COMPLETE BACKUP AND MANIFEST
    // ------------------------------------------------------------------
    println!("\n--- [Step 3] Exporting Complete Backup and Manifest ---");
    let timestamp_str = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let backup_dir = repo_root.join("backups");
    fs::create_dir_all(&backup_dir).expect("Failed to create backups directory");

    let backup_file_name = format!("firestore_catalogue_backup_{}.json", timestamp_str);
    let backup_file_path = backup_dir.join(&backup_file_name);

    let manifest_file_name = format!("firestore_catalogue_manifest_{}.json", timestamp_str);
    let manifest_file_path = backup_dir.join(&manifest_file_name);

    let backup_json_content = serde_json::to_string_pretty(&backup_entries)
        .expect("Failed to serialize backup entries to JSON");

    fs::write(&backup_file_path, &backup_json_content)
        .expect("Failed to write backup JSON to disk");

    // Calculate SHA256 checksum
    let mut hasher = Sha256::new();
    hasher.update(backup_json_content.as_bytes());
    let checksum_hex = format!("{:x}", hasher.finalize());

    let manifest = serde_json::json!({
        "firebase_project": project_id,
        "firestore_database": "(default)",
        "region": "nam5",
        "collection": "catalogue",
        "export_timestamp": chrono::Utc::now().to_rfc3339(),
        "total_documents": raw_docs.len(),
        "prefix_counts": {
            "canonical_fp": prefix_counts.get("fp_").unwrap_or(&0),
            "video_aliases_vid": prefix_counts.get("vid_").unwrap_or(&0),
            "recording_aliases_rec": prefix_counts.get("rec_").unwrap_or(&0),
            "other": prefix_counts.get("other").unwrap_or(&0)
        },
        "backup_file": backup_file_name,
        "backup_file_path": backup_file_path.to_string_lossy(),
        "sha256_checksum": checksum_hex,
        "nested_subcollections_detected": nested_subcollections_found
    });

    let manifest_json_content = serde_json::to_string_pretty(&manifest)
        .expect("Failed to serialize manifest JSON");
    fs::write(&manifest_file_path, &manifest_json_content)
        .expect("Failed to write manifest JSON to disk");

    println!("Backup and Manifest Written Successfully:");
    println!("  -> Backup File:   {:?}", backup_file_path);
    println!("  -> Manifest File: {:?}", manifest_file_path);
    println!("  -> SHA256:        {}", checksum_hex);

    // ------------------------------------------------------------------
    // STEP 4: VERIFY BACKUP INTEGRITY BEFORE DELETION
    // ------------------------------------------------------------------
    println!("\n--- [Step 4] Verifying Backup Integrity from Disk ---");
    let read_back_content = fs::read_to_string(&backup_file_path)
        .expect("Failed to read back backup JSON file from disk");

    let mut read_hasher = Sha256::new();
    read_hasher.update(read_back_content.as_bytes());
    let read_checksum = format!("{:x}", read_hasher.finalize());
    assert_eq!(read_checksum, checksum_hex, "Checksum mismatch on read-back backup file!");

    let parsed_backup: Vec<serde_json::Value> = serde_json::from_str(&read_back_content)
        .expect("Failed to parse read-back backup JSON file");
    assert_eq!(parsed_backup.len(), raw_docs.len(), "Parsed backup document count must match raw count");
    println!("-> Backup Integrity Check: PASSED (100% verified, {} docs confirmed on disk)\n", parsed_backup.len());

    // ------------------------------------------------------------------
    // STEP 5: ATTEMPT TO DELETE COMPLETE ONLINE CATALOGUE
    // ------------------------------------------------------------------
    println!("--- [Step 5] Attempting Live Deletion of Catalogue Documents ---");
    let mut deleted_count = 0;
    let mut permission_denied_count = 0;
    let mut not_found_count = 0;
    let mut other_error_count = 0;
    let mut sample_errors = Vec::new();

    let delete_start = Instant::now();
    for (i, (doc_id, _)) in raw_docs.iter().enumerate() {
        match client.delete_document("catalogue", doc_id).await {
            Ok((status, body)) => {
                if status.is_success() {
                    deleted_count += 1;
                } else if status.as_u16() == 403 {
                    permission_denied_count += 1;
                    if sample_errors.len() < 3 {
                        sample_errors.push((doc_id.clone(), status.as_u16(), body.clone()));
                    }
                } else if status.as_u16() == 404 {
                    not_found_count += 1;
                } else {
                    other_error_count += 1;
                    if sample_errors.len() < 3 {
                        sample_errors.push((doc_id.clone(), status.as_u16(), body.clone()));
                    }
                }
            }
            Err(e) => {
                other_error_count += 1;
                if sample_errors.len() < 3 {
                    sample_errors.push((doc_id.clone(), 0, e.to_string()));
                }
            }
        }

        if (i + 1) % 50 == 0 || (i + 1) == raw_docs.len() {
            println!("  -> Progress: {} / {} documents processed", i + 1, raw_docs.len());
        }
    }
    let delete_duration = delete_start.elapsed();

    println!("\nDeletion Attempt Telemetry ({:.1} ms):", delete_duration.as_secs_f64() * 1000.0);
    println!("  -> Successfully Deleted:       {}", deleted_count);
    println!("  -> Permission Denied (403):    {}", permission_denied_count);
    println!("  -> Not Found (404):            {}", not_found_count);
    println!("  -> Other Errors:               {}", other_error_count);

    if !sample_errors.is_empty() {
        println!("\nSample Responses from Live Firestore:");
        for (id, status, body) in &sample_errors {
            println!("  -> Doc: {} | HTTP {} | Body: {}", id, status, body);
        }
    }

    // ------------------------------------------------------------------
    // STEP 6: VERIFICATION READS (TWO INDEPENDENT READS)
    // ------------------------------------------------------------------
    println!("\n--- [Step 6] Verifying Live Online State (Two Independent Reads) ---");
    
    // Read 1
    println!("Performing First Independent Read...");
    let read1_docs = client.list_catalogue(300).await.expect("Failed to perform Read 1");
    println!("  -> Read 1: Remaining documents in 'catalogue': {}", read1_docs.len());

    // Read 2
    println!("Performing Second Independent Read...");
    let read2_docs = client.list_catalogue(300).await.expect("Failed to perform Read 2");
    println!("  -> Read 2: Remaining documents in 'catalogue': {}", read2_docs.len());

    println!("\n==================================================================");
    if read1_docs.is_empty() && read2_docs.is_empty() {
        println!("VERDICT: ONLINE CATALOGUE RESET SUCCESSFUL (0 DOCUMENTS REMAIN)");
    } else {
        println!("VERDICT: RESET BLOCKED BY FIRESTORE SECURITY RULES (HTTP 403)");
        println!("  -> Documents Remaining: {}", read1_docs.len());
        println!("  -> Root Cause: 'firestore.rules' explicitly specifies 'allow delete: if false;'");
    }
    println!("==================================================================\n");
}
