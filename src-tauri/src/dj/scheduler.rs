use anyhow::Result;
use tracing::info;

use crate::audio::{AudioCommand, AudioEngine};
use crate::audio::decoder::decode_audio_file;
use crate::audio::dsp::crossfader::CrossfaderCurve;
use crate::db::Database;
use crate::db::models::{PlayableTrackCandidate, PlaybackHistoryItem, PartySession};
use crate::dj::energy::SetStoryteller;
use crate::dj::levels::DjLevel;
use crate::dj::moods::{get_preset_moods, MoodDefinition};
use crate::dj::selector::TrackSelector;
use crate::dj::transition::TransitionPlan;

pub struct DjScheduler {
    pub session: PartySession,
    pub mood: MoodDefinition,
    pub secondary_mood: Option<MoodDefinition>,
    pub level: DjLevel,
    pub active_deck: usize, // 0 = Deck A, 1 = Deck B, 2 = Deck C, 3 = Deck D
    pub current_track: Option<PlayableTrackCandidate>,
    pub next_track: Option<PlayableTrackCandidate>,
    pub following_track: Option<PlayableTrackCandidate>,
    pub is_transitioning: bool,
    pub transition_progress: f32, // 0.0 to 1.0
    pub transition_end_time_ms: i64,
    pub outgoing_deck: usize,
    pub playback_history: Vec<PlaybackHistoryItem>,
    pub current_track_started_at_ms: i64,
    pub storyteller: SetStoryteller,
}

impl DjScheduler {
    pub fn new(session: PartySession) -> Self {
        let preset_moods = get_preset_moods();
        let mood = preset_moods
            .iter()
            .find(|m| m.id == session.primary_mood_id)
            .cloned()
            .unwrap_or_else(|| get_preset_moods().remove(0));

        let secondary_mood = session.secondary_mood_id.as_ref().and_then(|sec_id| {
            preset_moods.iter().find(|m| m.id == *sec_id).cloned()
        });

        let level = DjLevel::from_i32(session.dj_level);
        let storyteller = SetStoryteller::new(&mood.id, 30);

        Self {
            session,
            mood,
            secondary_mood,
            level,
            active_deck: 0,
            current_track: None,
            next_track: None,
            following_track: None,
            is_transitioning: false,
            transition_progress: 0.0,
            transition_end_time_ms: 0,
            outgoing_deck: 0,
            playback_history: Vec::new(),
            current_track_started_at_ms: 0,
            storyteller,
        }
    }

    /// Initializes DJ Scheduler restoring any persistent playback history from SQLite
    pub fn new_with_db(session: PartySession, db: &Database) -> Self {
        let mut s = Self::new(session);
        match db.get_session_playback_history(&s.session.id) {
            Ok(history) => {
                s.playback_history = history;
            }
            Err(e) => {
                tracing::warn!("Failed to query session playback history: {}", e);
            }
        }
        s
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

    /// Selects upcoming tracks with strict history, queue reservation, and approved-only request boosting
    pub fn ensure_queue_populated(&mut self, db: &Database) -> Result<()> {
        let candidates = db.get_playable_candidates()?;
        let requests = db.get_session_requests(&self.session.id)?;
        let approved_requests: Vec<_> = requests
            .into_iter()
            .filter(|r| r.status == "approved")
            .collect();

        let target_energy = self.storyteller.calculate_target_energy(self.mood.target_energy);

        if self.next_track.is_none() {
            let reserved = self.get_reserved_track_ids();
            if let Some(next) = TrackSelector::score_and_select(
                &candidates,
                self.current_track.as_ref(),
                &self.mood,
                self.secondary_mood.as_ref(),
                target_energy,
                &self.playback_history,
                &approved_requests,
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
                self.secondary_mood.as_ref(),
                target_energy,
                &self.playback_history,
                &approved_requests,
                &reserved,
            ) {
                self.following_track = Some(following);
            }
        }

        Ok(())
    }

    /// Preloads next track into the designated idle deck in 2-deck or 4-deck rotation
    pub fn preload_next_deck(&mut self, audio: &AudioEngine) -> Result<()> {
        if let Some(next) = &self.next_track {
            let max_decks = self.level.max_decks();
            let target_deck = (self.active_deck + 1) % max_decks;
            info!("Preloading next track '{}' into Deck {} (DJ Level: {:?})", next.track.title, target_deck, self.level);

            let decoded = match decode_audio_file(&next.asset.file_path) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Failed to decode track '{}' ({}): {}", next.track.title, next.asset.file_path, e);
                    self.next_track = self.following_track.take();
                    return Err(anyhow::anyhow!("Preload decode error: {}", e));
                }
            };

            // Adjust tempo rate if Smart Mixing (Level 2+) to beatmatch active deck within ±8%
            let rate = if let Some(curr) = &self.current_track {
                if self.level.supports_auto_sync() && next.analysis.bpm > 0.0 && curr.analysis.bpm > 0.0 {
                    (curr.analysis.bpm / next.analysis.bpm).clamp(0.92, 1.08)
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

            // Downbeat & phrase alignment: seek to musical mix-in or downbeat point
            let cue_point_ms = if next.analysis.recommended_mix_in_ms > 0 {
                next.analysis.recommended_mix_in_ms as u64
            } else if next.analysis.downbeat_offset_ms > 0 {
                next.analysis.downbeat_offset_ms as u64
            } else {
                0
            };

            if cue_point_ms > 0 {
                audio.send_command(AudioCommand::Seek {
                    deck_id: target_deck,
                    position_ms: cue_point_ms,
                })?;
            }

            audio.send_command(AudioCommand::SetTempoRate {
                deck_id: target_deck,
                rate,
            })?;
        }
        Ok(())
    }

    /// Preloads next track into designated idle deck with automatic fallback retry loop
    /// if the selected audio file is missing or corrupted.
    pub fn preload_next_deck_with_db(&mut self, audio: &AudioEngine, db: &Database) -> Result<()> {
        let mut attempts = 0;
        while attempts < 3 {
            attempts += 1;
            if self.next_track.is_none() {
                let _ = self.ensure_queue_populated(db);
            }
            if let Some(next) = self.next_track.as_ref() {
                match decode_audio_file(&next.asset.file_path) {
                    Ok(decoded) => {
                        let max_decks = self.level.max_decks();
                        let target_deck = (self.active_deck + 1) % max_decks;
                        info!("Preloading next track '{}' into Deck {} (DJ Level: {:?})", next.track.title, target_deck, self.level);

                        let rate = if let Some(curr) = &self.current_track {
                            if self.level.supports_auto_sync() && next.analysis.bpm > 0.0 && curr.analysis.bpm > 0.0 {
                                (curr.analysis.bpm / next.analysis.bpm).clamp(0.92, 1.08)
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

                        let cue_point_ms = if next.analysis.recommended_mix_in_ms > 0 {
                            next.analysis.recommended_mix_in_ms as u64
                        } else if next.analysis.downbeat_offset_ms > 0 {
                            next.analysis.downbeat_offset_ms as u64
                        } else {
                            0
                        };
                        if cue_point_ms > 0 {
                            let _ = audio.send_command(AudioCommand::Seek {
                                deck_id: target_deck,
                                position_ms: cue_point_ms,
                            });
                        }

                        audio.send_command(AudioCommand::SetTempoRate {
                            deck_id: target_deck,
                            rate,
                        })?;
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!("Preload decode error for '{}' ({}): {}, quarantining and retrying", next.track.title, next.asset.file_path, e);
                        let _ = db.update_asset_verification_status(&next.asset.id, "corrupt", "quarantined");
                        self.next_track = self.following_track.take();
                        let _ = self.ensure_queue_populated(db);
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Starts playing the party immediately
    pub fn start_playback(&mut self, audio: &AudioEngine, db: &Database) -> Result<()> {
        self.ensure_queue_populated(db)?;

        // If no track currently loaded, pull next track into active deck with fallback retry loop
        while self.current_track.is_none() {
            if let Some(next) = self.next_track.take() {
                match decode_audio_file(&next.asset.file_path) {
                    Ok(decoded) => {
                        audio.send_command(AudioCommand::LoadDeck {
                            deck_id: self.active_deck,
                            track_id: next.track.id.clone(),
                            samples: decoded.samples,
                            sample_rate: decoded.sample_rate,
                            bpm: next.analysis.bpm,
                        })?;

                        let cue_point_ms = if next.analysis.recommended_mix_in_ms > 0 {
                            next.analysis.recommended_mix_in_ms as u64
                        } else if next.analysis.downbeat_offset_ms > 0 {
                            next.analysis.downbeat_offset_ms as u64
                        } else {
                            0
                        };
                        if cue_point_ms > 0 {
                            let _ = audio.send_command(AudioCommand::Seek {
                                deck_id: self.active_deck,
                                position_ms: cue_point_ms,
                            });
                        }

                        audio.send_command(AudioCommand::Play { deck_id: self.active_deck })?;
                        
                        // Record history
                        self.record_track_start(&next);
                        self.current_track = Some(next);
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to decode track '{}' ({}) on start, quarantining and retrying: {}", next.track.title, next.asset.file_path, e);
                        let _ = db.update_asset_verification_status(&next.asset.id, "corrupt", "quarantined");
                        self.ensure_queue_populated(db)?;
                    }
                }
            } else {
                break;
            }
        }

        // Repopulate next and following with reservation
        self.ensure_queue_populated(db)?;
        self.preload_next_deck_with_db(audio, db)?;

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

    /// Periodic autonomous tick: monitors real audio clock & playhead, track mix-out points, and auto-transitions
    pub fn check_and_auto_transition(&mut self, audio: &AudioEngine, db: &Database) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        // 1. If currently in a transition, check whether the crossfader ramp has completed
        if self.is_transitioning {
            if now >= self.transition_end_time_ms {
                info!("Auto-DJ: Transition ramp complete. Stopping outgoing deck {}", self.outgoing_deck);
                audio.send_command(AudioCommand::Stop { deck_id: self.outgoing_deck })?;
                self.is_transitioning = false;

                // Mark any requests matching the previous track as 'played', update storytelling trajectory, and persist history
                if let Some(prev) = &self.current_track {
                    self.storyteller.record_track_played(prev.analysis.overall_energy, false, false);
                    let played_duration = now - self.current_track_started_at_ms;
                    let _ = db.log_playback(
                        &self.session.id,
                        &prev.track.id,
                        self.outgoing_deck as i32,
                        played_duration,
                        false,
                        "phrase_swap",
                    );

                    if let Ok(requests) = db.get_session_requests(&self.session.id) {
                        for req in requests {
                            if req.track_id == prev.track.id && (req.status == "approved" || req.status == "scheduled") {
                                let _ = db.update_request_status(&req.id, "played", None);
                            }
                        }
                    }
                }

                // Promote incoming deck to active (2-deck or 4-deck rotation)
                let max_decks = self.level.max_decks();
                self.active_deck = (self.outgoing_deck + 1) % max_decks;
                self.current_track = self.next_track.take();

                // Repopulate next and preload next deck
                self.ensure_queue_populated(db)?;
                let _ = self.preload_next_deck_with_db(audio, db);
            }
            return Ok(());
        }

        if self.current_track.is_none() {
            return Ok(());
        }

        let curr = self.current_track.as_ref().unwrap();

        // 2. Query the actual playhead and playback status from the active virtual deck
        let Some((position_ms, is_playing)) = audio.get_deck_playhead(self.active_deck) else {
            // Strictly wait for authoritative audio-clock telemetry tick, NEVER guess with wall-clock time
            return Ok(());
        };

        // If the active deck is paused, pause the transition countdown completely
        if !is_playing {
            return Ok(());
        }

        let mix_out_ms = if curr.analysis.recommended_mix_out_ms > 0 {
            curr.analysis.recommended_mix_out_ms as u64
        } else {
            (curr.track.duration_ms as u64).saturating_sub(8000).max(10000)
        };

        // 3. Trigger transition when the real audio playhead reaches the phrase mix-out point
        if position_ms >= mix_out_ms {
            info!(
                "Auto-DJ: Audio playhead reached phrase boundary for '{}' (position: {}ms, mix_out: {}ms). Auto-transitioning.",
                curr.track.title, position_ms, mix_out_ms
            );
            self.trigger_transition(audio, false)?;
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

    /// Trigger transition across active rotation decks with level-specific crossfader curve
    pub fn trigger_transition(&mut self, audio: &AudioEngine, fast_cut: bool) -> Result<()> {
        if self.next_track.is_none() {
            return Ok(());
        }

        let from_deck = self.active_deck;
        let max_decks = self.level.max_decks();
        let to_deck = (self.active_deck + 1) % max_decks;

        let curve = match self.level {
            crate::dj::levels::DjLevel::Level1TwoDeckMixer => CrossfaderCurve::EqualPower,
            crate::dj::levels::DjLevel::Level2SmartMixing | crate::dj::levels::DjLevel::Level3AdvancedPerformance => CrossfaderCurve::BassSwap,
            crate::dj::levels::DjLevel::Level4CreativeTools => {
                if fast_cut {
                    CrossfaderCurve::SharpCut
                } else {
                    CrossfaderCurve::BassSwap
                }
            }
        };

        let mut plan = if fast_cut {
            TransitionPlan::quick_cut(from_deck, to_deck)
        } else {
            let bpm = self.current_track.as_ref().map(|t| t.analysis.bpm).unwrap_or(128.0);
            TransitionPlan::default_phrase_swap(from_deck, to_deck, bpm)
        };
        plan.curve = curve;

        self.is_transitioning = true;
        self.outgoing_deck = from_deck;
        self.transition_end_time_ms = chrono::Utc::now().timestamp_millis() + plan.duration_ms as i64;

        // Start playback on incoming deck
        audio.send_command(AudioCommand::Play { deck_id: to_deck })?;

        // Crossfader target: Decks 1 & 3 are Side B (+1.0), Decks 0 & 2 are Side A (-1.0)
        let target_pos = if to_deck == 1 || to_deck == 3 { 1.0 } else { -1.0 };
        audio.send_command(AudioCommand::StartTransition {
            target_pos,
            duration_ms: plan.duration_ms as u32,
            curve: plan.curve,
        })?;

        // Record history and start time for incoming track
        if let Some(incoming) = self.next_track.clone() {
            self.record_track_start(&incoming);
        }

        Ok(())
    }
}
