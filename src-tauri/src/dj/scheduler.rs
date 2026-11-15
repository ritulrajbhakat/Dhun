use anyhow::Result;
use tracing::info;

use crate::audio::{AudioCommand, AudioEngine};
use crate::audio::decoder::decode_audio_file;
use crate::db::Database;
use crate::db::models::{PlayableTrackCandidate, PlaybackHistoryItem, PartySession};
use crate::dj::levels::DjLevel;
use crate::dj::moods::{get_preset_moods, MoodDefinition};
use crate::dj::selector::TrackSelector;
use crate::dj::transition::TransitionPlan;

pub struct DjScheduler {
    pub session: PartySession,
    pub mood: MoodDefinition,
    pub level: DjLevel,
    pub active_deck: usize, // 0 = Deck A, 1 = Deck B, 2 = Deck C, 3 = Deck D
    pub current_track: Option<PlayableTrackCandidate>,
    pub next_track: Option<PlayableTrackCandidate>,
    pub following_track: Option<PlayableTrackCandidate>,
    pub is_transitioning: bool,
    pub transition_progress: f32, // 0.0 to 1.0
    pub playback_history: Vec<PlaybackHistoryItem>,
    pub current_track_started_at_ms: i64,
}

impl DjScheduler {
    pub fn new(session: PartySession) -> Self {
        let preset_moods = get_preset_moods();
        let mood = preset_moods
            .into_iter()
            .find(|m| m.id == session.primary_mood_id)
            .unwrap_or_else(|| get_preset_moods().remove(0));

        let level = DjLevel::from_i32(session.dj_level);

        Self {
            session,
            mood,
            level,
            active_deck: 0,
            current_track: None,
            next_track: None,
            following_track: None,
            is_transitioning: false,
            transition_progress: 0.0,
            playback_history: Vec::new(),
            current_track_started_at_ms: 0,
        }
    }

    /// Helper to get IDs of all tracks currently queued or playing to prevent duplicates
    pub fn get_reserved_track_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        if let Some(c) = &self.current_track {
            ids.push(c.track.id.clone());
        }
        if let Some(n) = &self.next_track {
            ids.push(n.track.id.clone());
        }
        if let Some(f) = &self.following_track {
            ids.push(f.track.id.clone());
        }
        ids
    }

    /// Selects upcoming tracks with strict history and queue reservation
    pub fn ensure_queue_populated(&mut self, db: &Database) -> Result<()> {
        let candidates = db.get_playable_candidates()?;
        let requests = db.get_session_requests(&self.session.id)?;

        if self.next_track.is_none() {
            let reserved = self.get_reserved_track_ids();
            if let Some(next) = TrackSelector::score_and_select(
                &candidates,
                self.current_track.as_ref(),
                &self.mood,
                self.mood.target_energy,
                &self.playback_history,
                &requests,
                &reserved,
            ) {
                self.next_track = Some(next);
            }
        }

        if self.following_track.is_none() {
            let reserved = self.get_reserved_track_ids();
            if let Some(following) = TrackSelector::score_and_select(
                &candidates,
                self.next_track.as_ref().or(self.current_track.as_ref()),
                &self.mood,
                self.mood.target_energy,
                &self.playback_history,
                &requests,
                &reserved,
            ) {
                self.following_track = Some(following);
            }
        }

        Ok(())
    }

    /// Preloads next track into the designated idle deck in 4-deck rotation
    pub fn preload_next_deck(&mut self, audio: &AudioEngine) -> Result<()> {
        if let Some(next) = &self.next_track {
            let target_deck = (self.active_deck + 1) % 4; // 4-deck round-robin
            info!("Preloading next track '{}' into Deck {}", next.track.title, target_deck);

            let decoded = decode_audio_file(&next.asset.file_path)?;

            // Adjust tempo rate if Smart Mixing (Level 2+) to beatmatch active deck
            let rate = if let Some(curr) = &self.current_track {
                if self.level.supports_auto_sync() && next.analysis.bpm > 0.0 {
                    (curr.analysis.bpm / next.analysis.bpm).clamp(0.85, 1.15)
                } else {
                    1.0
                }
            } else {
                1.0
            };

            audio.send_command(AudioCommand::LoadDeck {
                deck_id: target_deck,
                track_id: next.track.id.clone(),
                samples: decoded.samples,
                sample_rate: decoded.sample_rate,
                bpm: next.analysis.bpm,
            })?;

            audio.send_command(AudioCommand::SetTempoRate {
                deck_id: target_deck,
                rate,
            })?;
        }
        Ok(())
    }

    /// Starts playing the party immediately
    pub fn start_playback(&mut self, audio: &AudioEngine, db: &Database) -> Result<()> {
        self.ensure_queue_populated(db)?;

        // If no track currently loaded, pull next track into active deck
        if self.current_track.is_none() {
            if let Some(next) = self.next_track.take() {
                let decoded = decode_audio_file(&next.asset.file_path)?;
                audio.send_command(AudioCommand::LoadDeck {
                    deck_id: self.active_deck,
                    track_id: next.track.id.clone(),
                    samples: decoded.samples,
                    sample_rate: decoded.sample_rate,
                    bpm: next.analysis.bpm,
                })?;
                audio.send_command(AudioCommand::Play { deck_id: self.active_deck })?;
                
                // Record history
                self.record_track_start(&next);
                self.current_track = Some(next);
            }
        } else {
            audio.send_command(AudioCommand::Play { deck_id: self.active_deck })?;
        }

        // Repopulate next and following with reservation
        self.ensure_queue_populated(db)?;
        self.preload_next_deck(audio)?;

        Ok(())
    }

    fn record_track_start(&mut self, candidate: &PlayableTrackCandidate) {
        let now = chrono::Utc::now().timestamp_millis();
        self.current_track_started_at_ms = now;
        self.playback_history.push(PlaybackHistoryItem {
            id: (self.playback_history.len() + 1) as i64,
            session_id: self.session.id.clone(),
            track_id: candidate.track.id.clone(),
            track_title: candidate.track.title.clone(),
            track_artist: candidate.track.artist.clone(),
            deck_id: self.active_deck as i32,
            played_at: now,
            duration_played_ms: 0,
            was_skipped: false,
            transition_type: "phrase_swap".to_string(),
        });
    }

    /// Periodic autonomous tick: monitors audio clock, track mix-out points, and auto-transitions
    pub fn check_and_auto_transition(&mut self, audio: &AudioEngine, db: &Database) -> Result<()> {
        if self.is_transitioning || self.current_track.is_none() {
            return Ok(());
        }

        let curr = self.current_track.as_ref().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        let elapsed_ms = (now - self.current_track_started_at_ms).max(0);

        let mix_out_ms = if curr.analysis.recommended_mix_out_ms > 0 {
            curr.analysis.recommended_mix_out_ms as i64
        } else {
            (curr.track.duration_ms - 8000).max(10000)
        };

        // If elapsed time has reached the phrase mix-out point or end of track
        if elapsed_ms >= mix_out_ms {
            info!(
                "Auto-DJ: Phrase boundary reached for '{}' (elapsed: {}ms, mix_out: {}ms). Auto-transitioning.",
                curr.track.title, elapsed_ms, mix_out_ms
            );
            self.trigger_transition(audio, false)?;
            self.ensure_queue_populated(db)?;
            let _ = self.preload_next_deck(audio);
        }

        Ok(())
    }

    /// Force Next track override by Host
    pub fn force_next_override(&mut self, track: PlayableTrackCandidate, audio: &AudioEngine) -> Result<()> {
        info!("Host invoked Force Next for track '{}'", track.track.title);
        self.next_track = Some(track);
        self.preload_next_deck(audio)?;
        self.trigger_transition(audio, true)?;
        Ok(())
    }

    /// Trigger transition across all 4 decks (0 -> 1 -> 2 -> 3 -> 0)
    pub fn trigger_transition(&mut self, audio: &AudioEngine, fast_cut: bool) -> Result<()> {
        if self.next_track.is_none() {
            return Ok(());
        }

        self.is_transitioning = true;
        let from_deck = self.active_deck;
        let to_deck = (self.active_deck + 1) % 4; // 4-deck rotation

        let plan = if fast_cut {
            TransitionPlan::quick_cut(from_deck, to_deck)
        } else {
            let bpm = self.current_track.as_ref().map(|t| t.analysis.bpm).unwrap_or(128.0);
            TransitionPlan::default_phrase_swap(from_deck, to_deck, bpm)
        };

        // Start playback on incoming deck
        audio.send_command(AudioCommand::Play { deck_id: to_deck })?;

        // Crossfader target: Decks 1 & 3 are Side B (+1.0), Decks 0 & 2 are Side A (-1.0)
        let target_pos = if to_deck == 1 || to_deck == 3 { 1.0 } else { -1.0 };
        audio.send_command(AudioCommand::SetCrossfader {
            position: target_pos,
            curve: plan.curve,
        })?;

        // Record history and start time for incoming track
        if let Some(incoming) = self.next_track.clone() {
            self.record_track_start(&incoming);
        }

        // Stop outgoing deck after transition duration
        let outgoing_deck = from_deck;
        let audio_cmd_tx = audio.cmd_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(plan.duration_ms)).await;
            let _ = audio_cmd_tx.try_send(AudioCommand::Stop { deck_id: outgoing_deck });
        });

        // Swap state
        self.active_deck = to_deck;
        self.current_track = self.next_track.take();
        self.next_track = self.following_track.take();
        self.is_transitioning = false;

        Ok(())
    }
}
