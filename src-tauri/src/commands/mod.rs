use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::audio::{AudioCommand, AudioEngine};
use crate::audio::device::AudioDeviceManager;
use crate::audio::dsp::crossfader::CrossfaderCurve;
use crate::audio::telemetry::MasterTelemetry;
use crate::db::Database;
use crate::db::models::{GuestRequest, PartySession, Track};
use crate::dj::moods::{get_preset_moods, MoodDefinition};
use crate::dj::scheduler::DjScheduler;
use crate::library::scanner::LibraryScanner;
use crate::network::LocalPartyServer;

#[derive(Clone)]
pub struct AppContext {
    pub db: Database,
    pub audio: Arc<Mutex<AudioEngine>>,
    pub scheduler: Arc<Mutex<Option<DjScheduler>>>,
    pub server: Arc<Mutex<Option<LocalPartyServer>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartPartyPayload {
    pub party_name: String,
    pub dj_level: i32,
    pub primary_mood_id: String,
    pub secondary_mood_id: Option<String>,
    pub output_device_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PartyStatusResponse {
    pub is_active: bool,
    pub session_id: Option<String>,
    pub party_name: Option<String>,
    pub primary_mood_id: Option<String>,
    pub dj_level: Option<i32>,
    pub guest_url: Option<String>,
    pub qr_svg: Option<String>,
    pub now_playing: Option<crate::db::models::PlayableTrackCandidate>,
    pub next_track: Option<crate::db::models::PlayableTrackCandidate>,
    pub following_track: Option<crate::db::models::PlayableTrackCandidate>,
}

#[tauri::command]
pub async fn start_party(
    payload: StartPartyPayload,
    state: State<'_, AppContext>,
) -> Result<PartyStatusResponse, String> {
    let session_id = format!("ses_{}", uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string());
    let now = chrono::Utc::now().timestamp_millis();

    let db = state.db.clone();
    let audio_arc = state.audio.clone();
    let server_arc = state.server.clone();
    let scheduler_arc = state.scheduler.clone();

    // Stage 0: Cleanly terminate any active party session and silence existing playback
    *scheduler_arc.lock().unwrap() = None;
    if let Some(old_server) = server_arc.lock().unwrap().take() {
        old_server.stop();
    }
    {
        let audio = audio_arc.lock().unwrap();
        for d in 0..4 {
            let _ = audio.send_command(AudioCommand::Stop { deck_id: d });
        }
    }

    // Stage 1: Validate library & mood profile
    let candidates = db.get_playable_candidates().map_err(|e| e.to_string())?;
    if candidates.is_empty() {
        return Err("No playable audio tracks found in library. Please scan an audio folder before starting the party.".to_string());
    }
    let preset_moods = get_preset_moods();
    if !preset_moods.iter().any(|m| m.id == payload.primary_mood_id) {
        return Err(format!("Unknown mood profile ID: '{}'", payload.primary_mood_id));
    }

    // Stage 2: Hardware audio stream verification and device selection
    if !payload.output_device_name.is_empty() && payload.output_device_name != "Default" {
        match AudioEngine::new(Some(&payload.output_device_name)) {
            Ok(new_engine) => {
                let mut audio_lock = audio_arc.lock().unwrap();
                *audio_lock = new_engine;
            }
            Err(e) => {
                return Err(format!("Failed to initialize selected audio device '{}': {}", payload.output_device_name, e));
            }
        }
    }

    let session = PartySession {
        id: session_id.clone(),
        party_name: payload.party_name.clone(),
        start_time: now,
        end_time: None,
        dj_level: payload.dj_level,
        primary_mood_id: payload.primary_mood_id.clone(),
        secondary_mood_id: payload.secondary_mood_id,
        output_device_name: payload.output_device_name,
        status: "active".to_string(),
        total_tracks_played: 0,
        config_json: None,
    };

    // Stage 3: Start local guest server on port 3000 FIRST (fail-fast if port is unavailable)
    let server = LocalPartyServer::start(
        3000,
        session_id.clone(),
        session.party_name.clone(),
        session.primary_mood_id.clone(),
        db.clone(),
    ).await.map_err(|e| format!("Failed to start guest server: {}", e))?;

    let qr_svg = server.qr_svg.clone();
    let guest_url = server.guest_url.clone();

    // Stage 4: Initialize DJ Scheduler with persistent database history
    let mut scheduler = DjScheduler::new_with_db(session.clone(), &db);

    // Stage 5: Pre-decode and start playback (with complete rollback on failure)
    {
        let audio_guard = audio_arc.lock().unwrap();
        if let Err(e) = scheduler.start_playback(&*audio_guard, &db) {
            server.stop();
            for d in 0..4 {
                let _ = audio_guard.send_command(AudioCommand::Stop { deck_id: d });
            }
            return Err(format!("Failed to initialize audio playback: {}", e));
        }
    }

    // Stage 6: Commit session to database (with rollback on failure)
    if let Err(e) = db.create_session(&session) {
        server.stop();
        let audio_guard = audio_arc.lock().unwrap();
        for d in 0..4 {
            let _ = audio_guard.send_command(AudioCommand::Stop { deck_id: d });
        }
        return Err(format!("Failed to record party session: {}", e));
    }

    let now_playing = scheduler.current_track.clone();
    let next_track = scheduler.next_track.clone();
    let following_track = scheduler.following_track.clone();

    *server_arc.lock().unwrap() = Some(server);
    *scheduler_arc.lock().unwrap() = Some(scheduler);

    // Continuous autonomous background DJ ticker (every 500ms) with hardware hot-recovery
    {
        let sched_bg = scheduler_arc.clone();
        let audio_bg = audio_arc.clone();
        let db_bg = db.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
            loop {
                interval.tick().await;
                let mut guard = sched_bg.lock().unwrap();
                if let Some(sched) = guard.as_mut() {
                    let mut audio_lock = audio_bg.lock().unwrap();
                    let _ = audio_lock.check_and_recover_stream();
                    if let Err(e) = sched.check_and_auto_transition(&*audio_lock, &db_bg) {
                        tracing::error!("Auto-transition tick error: {}", e);
                    }
                } else {
                    break;
                }
            }
        });
    }

    Ok(PartyStatusResponse {
        is_active: true,
        session_id: Some(session_id),
        party_name: Some(payload.party_name),
        primary_mood_id: Some(payload.primary_mood_id),
        dj_level: Some(payload.dj_level),
        guest_url: Some(guest_url),
        qr_svg: Some(qr_svg),
        now_playing,
        next_track,
        following_track,
    })
}

#[tauri::command]
pub fn get_party_status(state: State<'_, AppContext>) -> Result<PartyStatusResponse, String> {
    let sched_lock = state.scheduler.lock().unwrap();
    let server_lock = state.server.lock().unwrap();

    match (&*sched_lock, &*server_lock) {
        (Some(sched), Some(srv)) => Ok(PartyStatusResponse {
            is_active: true,
            session_id: Some(sched.session.id.clone()),
            party_name: Some(sched.session.party_name.clone()),
            primary_mood_id: Some(sched.session.primary_mood_id.clone()),
            dj_level: Some(sched.session.dj_level),
            guest_url: Some(srv.guest_url.clone()),
            qr_svg: Some(srv.qr_svg.clone()),
            now_playing: sched.current_track.clone(),
            next_track: sched.next_track.clone(),
            following_track: sched.following_track.clone(),
        }),
        _ => Ok(PartyStatusResponse {
            is_active: false,
            session_id: None,
            party_name: None,
            primary_mood_id: None,
            dj_level: None,
            guest_url: None,
            qr_svg: None,
            now_playing: None,
            next_track: None,
            following_track: None,
        }),
    }
}

#[tauri::command]
pub fn get_telemetry(state: State<'_, AppContext>) -> MasterTelemetry {
    let audio = state.audio.lock().unwrap();
    audio.poll_telemetry().unwrap_or_else(MasterTelemetry::empty)
}

#[tauri::command]
pub fn emergency_fade(duration_ms: u32, state: State<'_, AppContext>) -> Result<(), String> {
    state.audio.lock().unwrap().send_command(AudioCommand::EmergencyFade { duration_ms }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn deck_play(deck_id: usize, state: State<'_, AppContext>) -> Result<(), String> {
    state.audio.lock().unwrap().send_command(AudioCommand::Play { deck_id }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn deck_pause(deck_id: usize, state: State<'_, AppContext>) -> Result<(), String> {
    state.audio.lock().unwrap().send_command(AudioCommand::Pause { deck_id }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn deck_seek(deck_id: usize, position_ms: u64, state: State<'_, AppContext>) -> Result<(), String> {
    state.audio.lock().unwrap().send_command(AudioCommand::Seek { deck_id, position_ms }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn deck_set_tempo(deck_id: usize, rate: f64, state: State<'_, AppContext>) -> Result<(), String> {
    state.audio.lock().unwrap().send_command(AudioCommand::SetTempoRate { deck_id, rate }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn deck_set_gain(deck_id: usize, gain: f32, state: State<'_, AppContext>) -> Result<(), String> {
    state.audio.lock().unwrap().send_command(AudioCommand::SetGain { deck_id, gain }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn deck_set_eq(deck_id: usize, low_db: f32, mid_db: f32, high_db: f32, state: State<'_, AppContext>) -> Result<(), String> {
    state.audio.lock().unwrap().send_command(AudioCommand::SetEq { deck_id, low_db, mid_db, high_db }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn deck_set_filter(deck_id: usize, position: f32, state: State<'_, AppContext>) -> Result<(), String> {
    state.audio.lock().unwrap().send_command(AudioCommand::SetFilter { deck_id, position }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_crossfader(position: f32, curve: String, state: State<'_, AppContext>) -> Result<(), String> {
    let cf_curve = match curve.as_str() {
        "linear" => CrossfaderCurve::Linear,
        "sharp_cut" => CrossfaderCurve::SharpCut,
        "bass_swap" => CrossfaderCurve::BassSwap,
        _ => CrossfaderCurve::EqualPower,
    };
    state.audio.lock().unwrap().send_command(AudioCommand::SetCrossfader { position, curve: cf_curve }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_master_volume(volume: f32, state: State<'_, AppContext>) -> Result<(), String> {
    state.audio.lock().unwrap().send_command(AudioCommand::SetMasterVolume { volume }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_mood_profiles() -> Vec<MoodDefinition> {
    get_preset_moods()
}

#[tauri::command]
pub fn get_audio_devices() -> Vec<crate::audio::device::AudioDeviceInfo> {
    AudioDeviceManager::new().list_output_devices()
}

#[tauri::command]
pub fn get_tracks(state: State<'_, AppContext>) -> Result<Vec<Track>, String> {
    state.db.get_all_tracks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scan_library_folder(folder_path: String, state: State<'_, AppContext>) -> Result<usize, String> {
    LibraryScanner::scan_directory(&folder_path, &state.db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_guest_requests(session_id: String, state: State<'_, AppContext>) -> Result<Vec<GuestRequest>, String> {
    state.db.get_session_requests(&session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_guest_request_status(
    request_id: String,
    status: String,
    priority: Option<i32>,
    state: State<'_, AppContext>,
) -> Result<(), String> {
    state.db.update_request_status(&request_id, &status, priority).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn force_next_request(request_id: String, state: State<'_, AppContext>) -> Result<(), String> {
    let mut sched_lock = state.scheduler.lock().unwrap();
    if let Some(sched) = sched_lock.as_mut() {
        let requests = state.db.get_session_requests(&sched.session.id).map_err(|e| e.to_string())?;
        if let Some(req) = requests.into_iter().find(|r| r.id == request_id) {
            let candidates = state.db.get_playable_candidates().map_err(|e| e.to_string())?;
            if let Some(target) = candidates.into_iter().find(|c| c.track.id == req.track_id) {
                let audio = state.audio.lock().unwrap();
                sched.force_next_override(target, &*audio).map_err(|e| e.to_string())?;
                let _ = state.db.update_request_status(&request_id, "scheduled", Some(1));
                return Ok(());
            }
        } else {
            // Check if request_id directly references a track in candidates
            let candidates = state.db.get_playable_candidates().map_err(|e| e.to_string())?;
            if let Some(target) = candidates.into_iter().find(|c| c.track.id == request_id) {
                let audio = state.audio.lock().unwrap();
                sched.force_next_override(target, &*audio).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }
    Err("Could not find request or schedule track".to_string())
}

#[tauri::command]
pub fn set_dj_level(level: i32, state: State<'_, AppContext>) -> Result<(), String> {
    let mut sched_lock = state.scheduler.lock().unwrap();
    if let Some(sched) = sched_lock.as_mut() {
        sched.level = crate::dj::levels::DjLevel::from_i32(level);
        sched.session.dj_level = level;
        state.db.update_session_dj_level(&sched.session.id, level).map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("No active party session".to_string())
}

#[tauri::command]
pub fn set_party_mood(mood_id: String, state: State<'_, AppContext>) -> Result<(), String> {
    let mut sched_lock = state.scheduler.lock().unwrap();
    if let Some(sched) = sched_lock.as_mut() {
        let preset_moods = get_preset_moods();
        if let Some(m) = preset_moods.into_iter().find(|m| m.id == mood_id) {
            sched.mood = m;
            sched.session.primary_mood_id = mood_id.clone();
            state.db.update_session_mood(&sched.session.id, &mood_id).map_err(|e| e.to_string())?;
            // Refresh upcoming queue and preload next deck based on new mood profile
            sched.ensure_queue_populated(&state.db).map_err(|e| e.to_string())?;
            let audio = state.audio.lock().unwrap();
            sched.preload_next_deck(&*audio).map_err(|e| e.to_string())?;
            return Ok(());
        } else {
            return Err(format!("Unknown mood profile ID: '{}'", mood_id));
        }
    }
    Err("No active party session".to_string())
}
