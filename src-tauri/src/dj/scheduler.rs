use anyhow::Result;
use tracing::info;

use crate::audio::{AudioCommand, AudioEngine};
use crate::audio::decoder::decode_audio_file;
use crate::db::Database;
use crate::db::models::{PlayableTrackCandidate, PartySession};
use crate::dj::levels::DjLevel;
use crate::dj::moods::{get_preset_moods, MoodDefinition};
use crate::dj::selector::TrackSelector;
use crate::dj::transition::TransitionPlan;

pub struct DjScheduler {
    pub session: PartySession,
    pub mood: MoodDefinition,
    pub level: DjLevel,
    pub active_deck: usize, // 0 = Deck A, 1 = Deck B
    pub current_track: Option<PlayableTrackCandidate>,
    pub next_track: Option<PlayableTrackCandidate>,
    pub following_track: Option<PlayableTrackCandidate>,
    pub is_transitioning: bool,
    pub transition_progress: f32, // 0.0 to 1.0
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
        }
    }

    /// Selects upcoming tracks to keep active track loaded and at least 2 preloaded tracks ready
    pub fn ensure_queue_populated(&mut self, db: &Database) -> Result<()> {
        let candidates = db.get_playable_candidates()?;
        let requests = db.get_session_requests(&self.session.id)?;
        let _history = db.get_recent_sessions(10)?;

        if self.next_track.is_none() {
            if let Some(next) = TrackSelector::score_and_select(
                &candidates,
                self.current_track.as_ref(),
                &self.mood,
                self.mood.target_energy,
                &[],
                &requests,
            ) {
                self.next_track = Some(next);
            }
        }

        if self.following_track.is_none() {
            if let Some(following) = TrackSelector::score_and_select(
                &candidates,
                self.next_track.as_ref().or(self.current_track.as_ref()),
                &self.mood,
                self.mood.target_energy,
                &[],
                &requests,
            ) {
                self.following_track = Some(following);
            }
        }

        Ok(())
    }

    /// Preloads next track into the idle deck in the audio engine
    pub fn preload_next_deck(&mut self, audio: &AudioEngine) -> Result<()> {
        if let Some(next) = &self.next_track {
            let target_deck = 1 - self.active_deck; // alternate between 0 and 1
            info!("Preloading next track '{}' into Deck {}", next.track.title, target_deck);

            let decoded = decode_audio_file(&next.asset.file_path)?;
            audio.send_command(AudioCommand::LoadDeck {
                deck_id: target_deck,
                track_id: next.track.id.clone(),
                samples: decoded.samples,
                sample_rate: decoded.sample_rate,
                bpm: next.analysis.bpm,
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
                self.current_track = Some(next);
            }
        } else {
            audio.send_command(AudioCommand::Play { deck_id: self.active_deck })?;
        }

        // Repopulate next and following
        self.ensure_queue_populated(db)?;
        self.preload_next_deck(audio)?;

        Ok(())
    }

    /// Force Next track override by Host
    pub fn force_next_override(&mut self, track: PlayableTrackCandidate, audio: &AudioEngine) -> Result<()> {
        info!("Host invoked Force Next for track '{}'", track.track.title);
        self.next_track = Some(track);
        self.preload_next_deck(audio)?;
        // Trigger fast transition
        self.trigger_transition(audio, true)?;
        Ok(())
    }

    /// Trigger transition between current active deck and next deck
    pub fn trigger_transition(&mut self, audio: &AudioEngine, fast_cut: bool) -> Result<()> {
        let from_deck = self.active_deck;
        let to_deck = 1 - self.active_deck;

        let plan = if fast_cut {
            TransitionPlan::quick_cut(from_deck, to_deck)
        } else {
            let bpm = self.current_track.as_ref().map(|t| t.analysis.bpm).unwrap_or(128.0);
            TransitionPlan::default_phrase_swap(from_deck, to_deck, bpm)
        };

        // Start playback on incoming deck
        audio.send_command(AudioCommand::Play { deck_id: to_deck })?;

        // Crossfader target: -1.0 for Deck 0 (A), +1.0 for Deck 1 (B)
        let target_pos = if to_deck == 1 { 1.0 } else { -1.0 };
        audio.send_command(AudioCommand::SetCrossfader {
            position: target_pos,
            curve: plan.curve,
        })?;

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

        Ok(())
    }
}
