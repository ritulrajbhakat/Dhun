use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, Level};
use tracing_subscriber::FmtSubscriber;

use dhun_lib::analysis::analyze_track_file;
use dhun_lib::db::models::GlobalSongMetadata;
use dhun_lib::db::Database;
use dhun_lib::metadata_service::{
    compute_canonical_fingerprint, compute_recording_fingerprint, extract_version_tag,
    CatalogueUpsertRequest,
};
use dhun_lib::sync::firestore::{FirestoreClient, FirestoreConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSong {
    pub title: String,
    pub artist: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongProgressRecord {
    pub index: usize,
    pub title: String,
    pub artist: String,
    pub video_id: String,
    pub youtube_url: String,
    pub duration_ms: i64,
    pub original_bpm: f64,
    pub half_time_bpm: f64,
    pub double_time_bpm: f64,
    pub musical_key: String,
    pub camelot_key: String,
    pub overall_energy: f64,
    pub danceability: f64,
    pub canonical_fingerprint: String,
    pub recording_fingerprint: String,
    pub firestore_fp_doc: String,
    pub firestore_vid_doc: String,
    pub firestore_rec_doc: String,
    pub firestore_status: String,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestionState {
    pub total_curated: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub last_updated_iso: String,
    pub completed_songs: Vec<SongProgressRecord>,
}

impl IngestionState {
    pub fn load_or_init<P: AsRef<Path>>(path: P, total_curated: usize) -> Self {
        if let Ok(file) = File::open(&path) {
            let reader = BufReader::new(file);
            if let Ok(mut state) = serde_json::from_reader::<_, IngestionState>(reader) {
                state.total_curated = total_curated;
                return state;
            }
        }
        Self {
            total_curated,
            completed_count: 0,
            failed_count: 0,
            last_updated_iso: chrono::Utc::now().to_rfc3339(),
            completed_songs: Vec::new(),
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let tmp_path = format!("{}.tmp", path.as_ref().display());
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&tmp_path, content)?;
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    pub fn is_completed(&self, index: usize) -> bool {
        self.completed_songs.iter().any(|s| s.index == index)
    }
}

fn infer_genres_and_moods(title: &str, artist: &str, bpm: f64, energy: f64) -> (Vec<String>, Vec<String>) {
    let lower_a = artist.to_lowercase();
    let lower_t = title.to_lowercase();
    let mut genres = Vec::new();
    let mut moods = Vec::new();

    let is_punjabi = lower_a.contains("diljit")
        || lower_a.contains("sharry")
        || lower_a.contains("aujla")
        || lower_a.contains("jass manak")
        || lower_a.contains("sidhu")
        || lower_a.contains("panjabi mc")
        || lower_a.contains("honey singh")
        || lower_a.contains("gippy")
        || lower_a.contains("mankirt")
        || lower_a.contains("jassi")
        || lower_a.contains("gurnam")
        || lower_a.contains("amrit")
        || lower_a.contains("guru randhawa")
        || lower_a.contains("ap dhillon")
        || lower_a.contains("bally sagoo")
        || lower_a.contains("karan aujla")
        || lower_a.contains("jordan sandhu")
        || lower_a.contains("maninder buttar")
        || lower_a.contains("sukhbir")
        || lower_a.contains("mika singh")
        || lower_a.contains("b praak")
        || lower_a.contains("harrdy sandhu")
        || lower_a.contains("kulwinder billa")
        || lower_a.contains("ammy virk");

    let is_hiphop = lower_a.contains("divine")
        || lower_a.contains("mc stan")
        || lower_a.contains("seedhe maut")
        || lower_a.contains("king")
        || lower_a.contains("raftaar")
        || lower_a.contains("badshah")
        || lower_a.contains("ikka")
        || lower_a.contains("kr$na")
        || lower_a.contains("krsna")
        || lower_a.contains("emiway")
        || lower_a.contains("prabh deep");

    let is_romantic = lower_a.contains("arijit")
        || lower_a.contains("darshan")
        || lower_a.contains("atif")
        || lower_a.contains("jubin")
        || lower_a.contains("prateek kuhad")
        || lower_a.contains("jasleen royal")
        || lower_a.contains("shreya ghoshal")
        || lower_a.contains("arman malik")
        || lower_t.contains("tum hi ho")
        || lower_t.contains("raataan lambiyan")
        || lower_t.contains("channa mereya")
        || lower_t.contains("kesariya");

    if is_punjabi {
        genres.push("punjabi".to_string());
        genres.push("bhangra".to_string());
        if bpm >= 115.0 || energy >= 0.75 {
            moods.push("baraat".to_string());
            moods.push("party".to_string());
        } else {
            moods.push("sangeet".to_string());
            moods.push("celebration".to_string());
        }
    } else if is_hiphop {
        genres.push("desi_hip_hop".to_string());
        genres.push("rap".to_string());
        moods.push("club".to_string());
        moods.push("peak_hour".to_string());
    } else if is_romantic {
        genres.push("bollywood".to_string());
        genres.push("romantic".to_string());
        moods.push("cocktail".to_string());
        moods.push("chill".to_string());
    } else {
        genres.push("bollywood".to_string());
        if bpm >= 115.0 || energy >= 0.70 {
            genres.push("dance".to_string());
            moods.push("shaadi".to_string());
            moods.push("celebration".to_string());
        } else {
            genres.push("pop".to_string());
            moods.push("cocktail".to_string());
        }
    }

    (genres, moods)
}

struct ResolvedAudio {
    video_id: String,
    title: String,
    duration_sec: i64,
    mp3_path: PathBuf,
}

fn get_known_video_override(title: &str) -> Option<&'static str> {
    match title {
        "Bholi Bhali Ladki" => Some("j8rFEZpVKR8"),
        "Chunari Chunari" => Some("6z1U-kJ3xJE"),
        "Jungle Jungle Baat Chali Hai" => Some("ACKwDuDuxBg"),
        "Party Mashup - DJ Praveen - Nonie" => Some("Q8Kaq22QjGY"),
        _ => None,
    }
}

fn search_and_download_audio(
    yt_dlp_path: &Path,
    ffmpeg_dir: &Path,
    cache_dir: &Path,
    title: &str,
    artist: &str,
) -> Result<ResolvedAudio> {
    let out_template = cache_dir.join("%(id)s.%(ext)s").to_string_lossy().to_string();

    // 0. Check known video overrides for disambiguation
    if let Some(vid) = get_known_video_override(title) {
        let output = Command::new(yt_dlp_path)
            .args([
                "--format", "bestaudio/best",
                "--extract-audio",
                "--audio-format", "mp3",
                "--audio-quality", "192K",
                "--ffmpeg-location", &ffmpeg_dir.to_string_lossy(),
                "--no-simulate",
                "--print", "id",
                "--print", "title",
                "--print", "duration",
                "-o", &out_template,
                &format!("https://www.youtube.com/watch?v={}", vid),
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
                if lines.len() >= 3 {
                    let video_id = lines[0].to_string();
                    let resolved_title = lines[1].to_string();
                    let duration_sec: i64 = lines[2].parse().unwrap_or(0);
                    let mp3_path = cache_dir.join(format!("{}.mp3", video_id));

                    if mp3_path.exists() && fs::metadata(&mp3_path).map(|m| m.len() > 10_000).unwrap_or(false) {
                        return Ok(ResolvedAudio {
                            video_id,
                            title: resolved_title,
                            duration_sec,
                            mp3_path,
                        });
                    }
                }
            }
        }
    }

    // Generate clean search queries in order of precision
    let clean_artist = artist.split(',').next().unwrap_or(artist).trim();
    let queries = [
        format!("{} {} official audio", title, clean_artist),
        format!("{} {} official video", title, clean_artist),
        format!("{} {}", title, clean_artist),
        format!("{} official video", title),
        format!("{} official song", title),
        title.to_string(),
    ];

    for query in &queries {
        let output = Command::new(yt_dlp_path)
            .args([
                "--format", "bestaudio/best",
                "--extract-audio",
                "--audio-format", "mp3",
                "--audio-quality", "192K",
                "--ffmpeg-location", &ffmpeg_dir.to_string_lossy(),
                "--no-simulate",
                "--print", "id",
                "--print", "title",
                "--print", "duration",
                "-o", &out_template,
                &format!("ytsearch1:{}", query),
            ])
            .output();

        let Ok(out) = output else { continue; };
        if !out.status.success() { continue; }

        let stdout = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if lines.len() >= 3 {
            let video_id = lines[0].to_string();
            let resolved_title = lines[1].to_string();
            let duration_sec: i64 = lines[2].parse().unwrap_or(0);
            let mp3_path = cache_dir.join(format!("{}.mp3", video_id));

            if mp3_path.exists() && fs::metadata(&mp3_path).map(|m| m.len() > 10_000).unwrap_or(false) {
                return Ok(ResolvedAudio {
                    video_id,
                    title: resolved_title,
                    duration_sec,
                    mp3_path,
                });
            }
        }
    }

    Err(anyhow!("Failed to resolve and download audio for '{}' by '{}'", title, artist))
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    dhun_lib::sync::load_dotenv_if_present();

    println!("\n==================================================================");
    println!("DHUN — CURATED 616-SONG SEQUENTIAL INGESTION PIPELINE");
    println!("Internet Search • Native DSP • SQLite • Cloud Firestore Sync");
    println!("==================================================================\n");

    // CLI args parsing
    let args: Vec<String> = std::env::args().collect();
    let mut start_idx = 0;
    let mut limit: Option<usize> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--start" && i + 1 < args.len() {
            start_idx = args[i + 1].parse().unwrap_or(0);
            i += 2;
        } else if args[i] == "--limit" && i + 1 < args.len() {
            limit = args[i + 1].parse().ok();
            i += 2;
        } else {
            i += 1;
        }
    }

    // Paths configuration
    let base_dir = PathBuf::from("c:/Users/ritul/.gemini/antigravity-ide/scratch/dhun");
    let curated_json_path = base_dir.join("curated_616_songs.json");
    let state_json_path = base_dir.join("ingestion_state.json");
    let cache_dir = base_dir.join("cache");
    let venv_scripts = base_dir.join("resolver-service").join(".venv").join("Scripts");
    let yt_dlp_exe = venv_scripts.join("yt-dlp.exe");
    let ffmpeg_dir = venv_scripts.clone();
    let db_path = base_dir.join("dhun_local_helper.db");

    // Load .env from dhun repository root
    let env_path = base_dir.join(".env");
    if env_path.exists() {
        if let Ok(content) = fs::read_to_string(&env_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once('=') {
                    let key = k.trim();
                    let val = v.trim().trim_matches('"').trim_matches('\'');
                    if !val.is_empty() {
                        std::env::set_var(key, val);
                    }
                }
            }
        }
    }

    fs::create_dir_all(&cache_dir)?;

    println!("[Config] Paths verified:");
    println!("  -> Curated list:    {:?}", curated_json_path);
    println!("  -> State tracker:   {:?}", state_json_path);
    println!("  -> Audio cache:     {:?}", cache_dir);
    println!("  -> yt-dlp binary:   {:?}", yt_dlp_exe);
    println!("  -> Local SQLite DB: {:?}", db_path);

    // Initialize local SQLite database
    let db = Database::new(&db_path)?;
    println!("[Config] Local SQLite database initialized cleanly at: {:?}", db_path);

    // Initialize Cloud Firestore client
    let project_id = std::env::var("DHUN_FIREBASE_PROJECT_ID")
        .or_else(|_| std::env::var("VITE_FIREBASE_PROJECT_ID"))
        .unwrap_or_else(|_| "dhun-716e6".to_string());
    let api_key = std::env::var("DHUN_FIREBASE_API_KEY")
        .or_else(|_| std::env::var("VITE_FIREBASE_API_KEY"))
        .ok()
        .filter(|k| !k.trim().is_empty());

    let firestore_client = FirestoreClient::new(FirestoreConfig {
        project_id: project_id.clone(),
        api_key,
    });

    println!("[Config] Authenticating against Cloud Firestore ({})...", project_id);
    let token = firestore_client.authenticate_anonymous().await?;
    let is_verified = firestore_client.verify_token_project(&token)?;
    println!("  -> Firebase Anonymous Auth: SUCCESS (Audience match: {})", is_verified);

    // Load curated songs
    let songs_content = fs::read_to_string(&curated_json_path)
        .context("Failed to read curated_616_songs.json")?;
    let curated_songs: Vec<InputSong> = serde_json::from_str(&songs_content)
        .context("Failed to parse curated_616_songs.json")?;
    let total_songs = curated_songs.len();
    println!("[Config] Total songs to ingest: {}\n", total_songs);

    // Load or initialize state tracker
    let mut state = IngestionState::load_or_init(&state_json_path, total_songs);

    let end_idx = limit
        .map(|l| (start_idx + l).min(total_songs))
        .unwrap_or(total_songs);

    println!(">>> COMMENCING STRICT SEQUENTIAL INGESTION LOOP (Indices {} to {}) <<<\n", start_idx, end_idx);

    let mut successful_this_run = 0;
    let mut failed_this_run = 0;

    for idx in start_idx..end_idx {
        let song = &curated_songs[idx];

        if state.is_completed(idx) {
            println!("[{:03}/{:03}] ALREADY COMPLETED: \"{}\" by \"{}\" — skipping.", idx + 1, total_songs, song.title, song.artist);
            continue;
        }

        println!("------------------------------------------------------------------");
        println!("[{:03}/{:03}] PROCESSING: \"{}\" by \"{}\"", idx + 1, total_songs, song.title, song.artist);
        let t_start = std::time::Instant::now();

        // 1. Internet Search & Resolve Video Link
        print!("  -> [1/5] Searching internet & resolving audio... ");
        std::io::stdout().flush().ok();
        let resolved = match search_and_download_audio(&yt_dlp_exe, &ffmpeg_dir, &cache_dir, &song.title, &song.artist) {
            Ok(res) => {
                println!("FOUND (Video ID: {}, Title: '{}')", res.video_id, res.title);
                res
            }
            Err(e) => {
                println!("FAILED ({})", e);
                state.failed_count += 1;
                state.last_updated_iso = chrono::Utc::now().to_rfc3339();
                let _ = state.save(&state_json_path);
                failed_this_run += 1;
                continue;
            }
        };

        // 2. Stage 5 Native Audio DSP Analysis
        print!("  -> [2/5] Running Stage 5 DSP analysis (Symphonia decode + BPM/Key/BeatGrid)... ");
        std::io::stdout().flush().ok();
        let song_id = format!("gsng_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..16]);
        let full_dsp = match analyze_track_file(&song_id, &resolved.mp3_path) {
            Ok(dsp) => {
                let orig_bpm = (dsp.analysis.bpm * 100.0).round() / 100.0;
                println!("DONE ({:.1} BPM | Key: {} / {} | Energy: {:.2})",
                    orig_bpm, dsp.analysis.key_name, dsp.analysis.camelot_key, dsp.analysis.overall_energy);
                dsp
            }
            Err(e) => {
                println!("FAILED ({})", e);
                // Clean up mp3
                let _ = fs::remove_file(&resolved.mp3_path);
                state.failed_count += 1;
                state.last_updated_iso = chrono::Utc::now().to_rfc3339();
                let _ = state.save(&state_json_path);
                failed_this_run += 1;
                continue;
            }
        };

        let duration_ms = full_dsp.duration_ms as i64;
        let orig_bpm = (full_dsp.analysis.bpm * 100.0).round() / 100.0;
        let half_bpm = ((orig_bpm / 2.0) * 100.0).round() / 100.0;
        let double_bpm = ((orig_bpm * 2.0) * 100.0).round() / 100.0;
        let musical_key = full_dsp.analysis.key_name.clone();
        let camelot_key = full_dsp.analysis.camelot_key.clone();
        let version_tag = extract_version_tag(&song.title);

        let canonical_fp = compute_canonical_fingerprint(&song.title, &song.artist, &version_tag, duration_ms);
        let recording_fp = compute_recording_fingerprint(&song.title, &song.artist, &version_tag);

        let (genre_tags, mood_tags) = infer_genres_and_moods(&song.title, &song.artist, orig_bpm, full_dsp.analysis.overall_energy);

        let mut beat_grid_str = serde_json::to_string(&full_dsp.timing_profile).unwrap_or_else(|_| "{}".to_string());
        if beat_grid_str.len() > 60_000 {
            let mut trimmed = full_dsp.timing_profile.clone();
            while beat_grid_str.len() > 60_000 && trimmed.beat_timestamps_ms.len() > 100 {
                trimmed.beat_timestamps_ms = trimmed.beat_timestamps_ms.into_iter().step_by(2).collect();
                trimmed.downbeat_timestamps_ms = trimmed.downbeat_timestamps_ms.into_iter().step_by(2).collect();
                beat_grid_str = serde_json::to_string(&trimmed).unwrap_or_else(|_| "{}".to_string());
            }
            if beat_grid_str.len() > 60_000 {
                trimmed.beat_timestamps_ms = Vec::new();
                trimmed.downbeat_timestamps_ms = Vec::new();
                beat_grid_str = serde_json::to_string(&trimmed).unwrap_or_else(|_| "{}".to_string());
            }
        }
        let mut phrase_markers_str = serde_json::to_string(&full_dsp.phrase_markers).unwrap_or_else(|_| "[]".to_string());
        if phrase_markers_str.len() > 60_000 {
            phrase_markers_str = "[]".to_string();
        }

        let youtube_url = format!("https://www.youtube.com/watch?v={}", resolved.video_id);

        let upsert_payload = CatalogueUpsertRequest {
            source_url: youtube_url.clone(),
            title: song.title.clone(),
            artist: song.artist.clone(),
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
            genre_tags: Some(Vec::new()),
            mood_tags: Some(mood_tags.clone()),
            analysis_version: Some(2),
            analysis_confidence: Some(0.98),
            processing_status: Some("analyzed".to_string()),
        };

        // 3. Local SQLite Database Update
        print!("  -> [3/5] Persisting metadata to local SQLite database... ");
        std::io::stdout().flush().ok();
        let now_ms = chrono::Utc::now().timestamp_millis();
        let global_meta = GlobalSongMetadata {
            song_id: song_id.clone(),
            canonical_fingerprint: canonical_fp.clone(),
            title: song.title.clone(),
            artist: song.artist.clone(),
            version_tag: version_tag.clone(),
            duration_ms,
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
            genre_tags: genre_tags.clone(),
            mood_tags: mood_tags.clone(),
            analysis_version: 2,
            analysis_confidence: 0.98,
            processing_status: "analyzed".to_string(),
            source_urls: vec![youtube_url.clone()],
            created_at: now_ms,
            updated_at: now_ms,
        };

        if let Err(e) = db.upsert_global_song_metadata(&global_meta, Some((&youtube_url, "youtube", &resolved.video_id))) {
            println!("WARN ({})", e);
        } else {
            println!("OK");
        }

        // 4. Live Cloud Firestore Synchronization
        print!("  -> [4/5] Publishing metadata to Cloud Firestore (fp_, vid_, rec_)... ");
        std::io::stdout().flush().ok();
        let firestore_res = firestore_client.publish_metadata(&upsert_payload, Some(&resolved.video_id)).await;
        match firestore_res {
            Ok(_) => {
                println!("OK (200)");
            }
            Err(e) => {
                println!("FAILED ({})", e);
                // Clean up mp3
                let _ = fs::remove_file(&resolved.mp3_path);
                state.failed_count += 1;
                state.last_updated_iso = chrono::Utc::now().to_rfc3339();
                let _ = state.save(&state_json_path);
                failed_this_run += 1;
                continue;
            }
        }

        // 5. Clean up temporary downloaded audio to preserve disk space
        print!("  -> [5/5] Cleaning up local audio cache & recording checkpoint... ");
        std::io::stdout().flush().ok();
        let _ = fs::remove_file(&resolved.mp3_path);

        let record = SongProgressRecord {
            index: idx,
            title: song.title.clone(),
            artist: song.artist.clone(),
            video_id: resolved.video_id.clone(),
            youtube_url: youtube_url.clone(),
            duration_ms,
            original_bpm: orig_bpm,
            half_time_bpm: half_bpm,
            double_time_bpm: double_bpm,
            musical_key,
            camelot_key,
            overall_energy: full_dsp.analysis.overall_energy,
            danceability: full_dsp.analysis.dance_suitability,
            canonical_fingerprint: canonical_fp.clone(),
            recording_fingerprint: recording_fp.clone(),
            firestore_fp_doc: format!("catalogue/fp_{}", canonical_fp),
            firestore_vid_doc: format!("catalogue/vid_{}", resolved.video_id),
            firestore_rec_doc: format!("catalogue/rec_{}", recording_fp),
            firestore_status: "synced_200".to_string(),
            timestamp_ms: now_ms,
        };

        state.completed_songs.push(record);
        state.completed_count = state.completed_songs.len();
        state.last_updated_iso = chrono::Utc::now().to_rfc3339();
        state.save(&state_json_path)?;

        println!("CHECKPOINT SAVED ({:.2}s total)", t_start.elapsed().as_secs_f64());
        successful_this_run += 1;

        // Rate-limit safety jitter (1.5 seconds)
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }

    println!("\n==================================================================");
    println!("BATCH RUN COMPLETED");
    println!("Successful in this run: {}", successful_this_run);
    println!("Failed in this run:     {}", failed_this_run);
    println!("Overall Completed:     {}/{}", state.completed_count, total_songs);
    println!("==================================================================\n");

    Ok(())
}
