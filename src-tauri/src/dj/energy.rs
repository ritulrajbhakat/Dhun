use serde::{Deserialize, Serialize};
use crate::db::models::PhraseMarker;
use crate::dj::moods::MoodDefinition;

/// Macro phases across an entire DJ set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetPhase {
    WarmUp,      // Welcoming the room, building groove, accessible tempo & energy
    BuildUp,     // Escalating intensity, driving rhythm, exciting anticipation
    Peak,        // Dancefloor climax, explosive energy, major anthems
    Sustain,     // Maintaining high groove without crowd exhaustion
    Recovery,    // Intentional breathing room: lower energy / acoustic / deep groove
    RenewedPeak, // Second climax powered by iconic / highly recognizable anthems
    Finale,      // Triumphant ending, grand celebration or nostalgic singalong
}

impl SetPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WarmUp => "warm_up",
            Self::BuildUp => "build_up",
            Self::Peak => "peak",
            Self::Sustain => "sustain",
            Self::Recovery => "recovery",
            Self::RenewedPeak => "renewed_peak",
            Self::Finale => "finale",
        }
    }

    pub fn is_peak(&self) -> bool {
        matches!(self, Self::Peak | Self::RenewedPeak)
    }

    pub fn is_recovery(&self) -> bool {
        matches!(self, Self::Recovery)
    }
}

/// Detailed section-level energy analysis of a track, avoiding single-number oversimplification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionEnergyProfile {
    pub intro_energy: f64,
    pub mix_in_energy: f64,
    pub body_energy: f64,
    pub peak_energy: f64,
    pub mix_out_energy: f64,
    pub has_iconic_hook: bool,
    pub hook_sections: Vec<ProtectedHookInfo>,
    pub dynamic_range_db: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedHookInfo {
    pub section_type: String, // 'chorus', 'drop', 'peak'
    pub start_ms: i64,
    pub end_ms: i64,
    pub energy: f64,
    pub vocal_activity: f64,
}

impl SectionEnergyProfile {
    pub fn from_phrase_markers(markers: &[PhraseMarker], fallback_energy: f64) -> Self {
        if markers.is_empty() {
            return Self {
                intro_energy: (fallback_energy * 0.70).clamp(0.10, 0.95),
                mix_in_energy: (fallback_energy * 0.85).clamp(0.15, 0.95),
                body_energy: fallback_energy,
                peak_energy: (fallback_energy * 1.15).clamp(0.20, 1.0),
                mix_out_energy: (fallback_energy * 0.75).clamp(0.10, 0.95),
                has_iconic_hook: fallback_energy >= 0.75,
                hook_sections: vec![],
                dynamic_range_db: 6.0,
            };
        }

        let intro_sec = markers.iter().find(|m| m.section_type == "intro");
        let intro_energy = intro_sec
            .map(|m| m.energy_level)
            .unwrap_or_else(|| markers.first().map(|m| m.energy_level).unwrap_or(fallback_energy));

        let outro_sec = markers.iter().rev().find(|m| m.section_type == "outro");
        let mix_out_energy = outro_sec
            .map(|m| m.energy_level)
            .unwrap_or_else(|| markers.last().map(|m| m.energy_level).unwrap_or(fallback_energy));

        let mut peak_e = 0.0f64;
        let mut min_e = 1.0f64;
        let mut sum_body_e = 0.0f64;
        let mut body_count = 0usize;
        let mut hook_sections = Vec::new();

        for m in markers {
            let e = m.energy_level;
            if e > peak_e {
                peak_e = e;
            }
            if e < min_e {
                min_e = e;
            }

            if m.section_type == "chorus" || m.section_type == "drop" || m.section_type == "peak" {
                if m.vocal_activity >= 0.50 || m.bass_energy >= 0.70 || e >= 0.75 {
                    hook_sections.push(ProtectedHookInfo {
                        section_type: m.section_type.clone(),
                        start_ms: m.start_ms,
                        end_ms: m.end_ms,
                        energy: e,
                        vocal_activity: m.vocal_activity,
                    });
                }
            } else if m.section_type != "intro" && m.section_type != "outro" {
                sum_body_e += e;
                body_count += 1;
            }
        }

        let body_energy = if body_count > 0 {
            sum_body_e / body_count as f64
        } else {
            fallback_energy
        };

        let mix_in_marker = markers.iter().find(|m| {
            m.section_type == "groove" || m.section_type == "verse" || m.section_type == "build"
        });
        let mix_in_energy = mix_in_marker.map(|m| m.energy_level).unwrap_or(body_energy);

        let dynamic_range_db = ((peak_e.max(0.01) / min_e.max(0.01)).ln() * 8.6858).clamp(3.0, 18.0);
        let has_iconic_hook = !hook_sections.is_empty();

        Self {
            intro_energy,
            mix_in_energy,
            body_energy,
            peak_energy: peak_e.max(fallback_energy),
            mix_out_energy,
            has_iconic_hook,
            hook_sections,
            dynamic_range_db,
        }
    }

    /// Checks if a given playback position is currently inside or approaching an iconic chorus/drop
    /// within `lead_time_ms` (e.g. 15,000 ms), deserving protection from premature mix-out.
    pub fn is_point_in_protected_hook(&self, current_pos_ms: i64, lead_time_ms: i64) -> Option<ProtectedHookInfo> {
        for hook in &self.hook_sections {
            let protect_start = (hook.start_ms - lead_time_ms).max(0);
            if current_pos_ms >= protect_start && current_pos_ms < hook.end_ms {
                return Some(hook.clone());
            }
        }
        None
    }
}

/// Mood-specific set storytelling trajectory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodStoryCurve {
    pub mood_id: String,
    pub warmup_fraction: f64,
    pub buildup_fraction: f64,
    pub peak_fraction: f64,
    pub sustain_fraction: f64,
    pub recovery_fraction: f64,
    pub renewed_peak_fraction: f64,
    pub finale_fraction: f64,
    pub warmup_target: f64,
    pub buildup_target: f64,
    pub peak_target: f64,
    pub sustain_target: f64,
    pub recovery_target: f64,
    pub renewed_peak_target: f64,
    pub finale_target: f64,
    pub max_consecutive_peaks: usize,
    pub recovery_interval_tracks: usize,
    pub curve_narrative: String,
}

impl MoodStoryCurve {
    pub fn for_mood(mood_id: &str) -> Self {
        match mood_id.to_lowercase().as_str() {
            "baraat" => Self {
                mood_id: "baraat".to_string(),
                warmup_fraction: 0.10,
                buildup_fraction: 0.15,
                peak_fraction: 0.30,
                sustain_fraction: 0.15,
                recovery_fraction: 0.10, // Brief dholak breather before venue gates
                renewed_peak_fraction: 0.15,
                finale_fraction: 0.05,
                warmup_target: 0.78,
                buildup_target: 0.88,
                peak_target: 0.98,
                sustain_target: 0.94,
                recovery_target: 0.72, // Intentional procession breather
                renewed_peak_target: 0.99,
                finale_target: 0.90,
                max_consecutive_peaks: 3,
                recovery_interval_tracks: 7,
                curve_narrative: "Relentless street procession building to explosive venue entrance climax with intentional processional breathers.".to_string(),
            },
            "cocktail_night" => Self {
                mood_id: "cocktail_night".to_string(),
                warmup_fraction: 0.20,
                buildup_fraction: 0.20,
                peak_fraction: 0.25,
                sustain_fraction: 0.15,
                recovery_fraction: 0.10,
                renewed_peak_fraction: 0.05,
                finale_fraction: 0.05,
                warmup_target: 0.45,
                buildup_target: 0.55,
                peak_target: 0.66,
                sustain_target: 0.62,
                recovery_target: 0.48, // Relaxed acoustic rework breather
                renewed_peak_target: 0.68,
                finale_target: 0.50,
                max_consecutive_peaks: 2,
                recovery_interval_tracks: 6,
                curve_narrative: "Sophisticated low-lit lounge progression: subtle ambient opening rising into hypnotic deep house plateau with chill acoustic breathing room.".to_string(),
            },
            "garba" => Self {
                mood_id: "garba".to_string(),
                warmup_fraction: 0.15,
                buildup_fraction: 0.20,
                peak_fraction: 0.25,
                sustain_fraction: 0.15,
                recovery_fraction: 0.10,
                renewed_peak_fraction: 0.10,
                finale_fraction: 0.05,
                warmup_target: 0.62,
                buildup_target: 0.74,
                peak_target: 0.88,
                sustain_target: 0.85,
                recovery_target: 0.70, // Sanedo call-and-response breather
                renewed_peak_target: 0.94,
                finale_target: 0.72,
                max_consecutive_peaks: 3,
                recovery_interval_tracks: 7,
                curve_narrative: "Unbroken cyclic acceleration from gentle 2-taali circular stepping into high-speed swirling 3-taali trance.".to_string(),
            },
            "shaadi" => Self {
                mood_id: "shaadi".to_string(),
                warmup_fraction: 0.15,
                buildup_fraction: 0.15,
                peak_fraction: 0.25,
                sustain_fraction: 0.15,
                recovery_fraction: 0.12,
                renewed_peak_fraction: 0.12,
                finale_fraction: 0.06,
                warmup_target: 0.62,
                buildup_target: 0.72,
                peak_target: 0.82,
                sustain_target: 0.78,
                recovery_target: 0.60, // Emotional vidai / acoustic singalong
                renewed_peak_target: 0.85,
                finale_target: 0.70,
                max_consecutive_peaks: 2,
                recovery_interval_tracks: 6,
                curve_narrative: "Grand wedding progression: Shehnai greetings to multi-generational celebratory singalongs, with acoustic moments before peak floorfillers.".to_string(),
            },
            "mehendi" => Self {
                mood_id: "mehendi".to_string(),
                warmup_fraction: 0.20,
                buildup_fraction: 0.20,
                peak_fraction: 0.20,
                sustain_fraction: 0.15,
                recovery_fraction: 0.12,
                renewed_peak_fraction: 0.08,
                finale_fraction: 0.05,
                warmup_target: 0.48,
                buildup_target: 0.55,
                peak_target: 0.66,
                sustain_target: 0.60,
                recovery_target: 0.45,
                renewed_peak_target: 0.68,
                finale_target: 0.50,
                max_consecutive_peaks: 2,
                recovery_interval_tracks: 5,
                curve_narrative: "Intimate daytime folk swaying: warm harmonium and acoustic strings with gentle celebratory dholki lifts.".to_string(),
            },
            "haldi" => Self {
                mood_id: "haldi".to_string(),
                warmup_fraction: 0.15,
                buildup_fraction: 0.20,
                peak_fraction: 0.25,
                sustain_fraction: 0.15,
                recovery_fraction: 0.10,
                renewed_peak_fraction: 0.10,
                finale_fraction: 0.05,
                warmup_target: 0.58,
                buildup_target: 0.66,
                peak_target: 0.76,
                sustain_target: 0.72,
                recovery_target: 0.55,
                renewed_peak_target: 0.78,
                finale_target: 0.62,
                max_consecutive_peaks: 2,
                recovery_interval_tracks: 6,
                curve_narrative: "Sunny, playful daytime celebration: buoyant folk percussion and joyful singalongs.".to_string(),
            },
            "sangeet" => Self {
                mood_id: "sangeet".to_string(),
                warmup_fraction: 0.12,
                buildup_fraction: 0.18,
                peak_fraction: 0.28,
                sustain_fraction: 0.16,
                recovery_fraction: 0.10,
                renewed_peak_fraction: 0.12,
                finale_fraction: 0.04,
                warmup_target: 0.70,
                buildup_target: 0.80,
                peak_target: 0.92,
                sustain_target: 0.88,
                recovery_target: 0.65,
                renewed_peak_target: 0.94,
                finale_target: 0.82,
                max_consecutive_peaks: 3,
                recovery_interval_tracks: 7,
                curve_narrative: "Choreographed stage performance escalating into high-voltage family dance competition and open-floor anthems.".to_string(),
            },
            "desi_club" => Self {
                mood_id: "desi_club".to_string(),
                warmup_fraction: 0.12,
                buildup_fraction: 0.18,
                peak_fraction: 0.30,
                sustain_fraction: 0.16,
                recovery_fraction: 0.08,
                renewed_peak_fraction: 0.12,
                finale_fraction: 0.04,
                warmup_target: 0.75,
                buildup_target: 0.84,
                peak_target: 0.94,
                sustain_target: 0.90,
                recovery_target: 0.74,
                renewed_peak_target: 0.96,
                finale_target: 0.85,
                max_consecutive_peaks: 3,
                recovery_interval_tracks: 8,
                curve_narrative: "Peak-hour South Asian nightclub: driving 4-on-the-floor sub-bass drops and relentless remixes.".to_string(),
            },
            "punjabi_tashan" => Self {
                mood_id: "punjabi_tashan".to_string(),
                warmup_fraction: 0.12,
                buildup_fraction: 0.18,
                peak_fraction: 0.28,
                sustain_fraction: 0.16,
                recovery_fraction: 0.10,
                renewed_peak_fraction: 0.12,
                finale_fraction: 0.04,
                warmup_target: 0.72,
                buildup_target: 0.82,
                peak_target: 0.92,
                sustain_target: 0.88,
                recovery_target: 0.68,
                renewed_peak_target: 0.95,
                finale_target: 0.80,
                max_consecutive_peaks: 3,
                recovery_interval_tracks: 7,
                curve_narrative: "Swagger-heavy Punjabi hip-hop: massive 808 sub slides, UK drill, and half-time bounce.".to_string(),
            },
            "dandiya" => Self {
                mood_id: "dandiya".to_string(),
                warmup_fraction: 0.10,
                buildup_fraction: 0.15,
                peak_fraction: 0.30,
                sustain_fraction: 0.18,
                recovery_fraction: 0.10,
                renewed_peak_fraction: 0.12,
                finale_fraction: 0.05,
                warmup_target: 0.78,
                buildup_target: 0.86,
                peak_target: 0.96,
                sustain_target: 0.92,
                recovery_target: 0.76,
                renewed_peak_target: 0.98,
                finale_target: 0.85,
                max_consecutive_peaks: 3,
                recovery_interval_tracks: 7,
                curve_narrative: "Relentless high-velocity stick dance: locked fast tempos and synchronized stick strike impact.".to_string(),
            },
            "retro_bollywood" => Self {
                mood_id: "retro_bollywood".to_string(),
                warmup_fraction: 0.15,
                buildup_fraction: 0.20,
                peak_fraction: 0.25,
                sustain_fraction: 0.15,
                recovery_fraction: 0.12,
                renewed_peak_fraction: 0.08,
                finale_fraction: 0.05,
                warmup_target: 0.50,
                buildup_target: 0.60,
                peak_target: 0.72,
                sustain_target: 0.65,
                recovery_target: 0.48, // Romantic Rafi / Kishore ballad
                renewed_peak_target: 0.75,
                finale_target: 0.55,
                max_consecutive_peaks: 2,
                recovery_interval_tracks: 6,
                curve_narrative: "Golden era vintage groove: RD Burman funk and Kishore Kumar singalongs with soulful romantic breathers.".to_string(),
            },
            "house_party" => Self {
                mood_id: "house_party".to_string(),
                warmup_fraction: 0.16,
                buildup_fraction: 0.20,
                peak_fraction: 0.24,
                sustain_fraction: 0.16,
                recovery_fraction: 0.12,
                renewed_peak_fraction: 0.08,
                finale_fraction: 0.04,
                warmup_target: 0.58,
                buildup_target: 0.68,
                peak_target: 0.82,
                sustain_target: 0.76,
                recovery_target: 0.60,
                renewed_peak_target: 0.85,
                finale_target: 0.65,
                max_consecutive_peaks: 2,
                recovery_interval_tracks: 6,
                curve_narrative: "Eclectic living room social: versatile pop, trending hip-hop, and easy-dance anthems.".to_string(),
            },
            "festival_mode" => Self {
                mood_id: "festival_mode".to_string(),
                warmup_fraction: 0.10,
                buildup_fraction: 0.15,
                peak_fraction: 0.32,
                sustain_fraction: 0.18,
                recovery_fraction: 0.08,
                renewed_peak_fraction: 0.12,
                finale_fraction: 0.05,
                warmup_target: 0.68,
                buildup_target: 0.82,
                peak_target: 0.98,
                sustain_target: 0.95,
                recovery_target: 0.64, // Melodic breakdown vocal breather
                renewed_peak_target: 1.00,
                finale_target: 0.88,
                max_consecutive_peaks: 3,
                recovery_interval_tracks: 8,
                curve_narrative: "Stadium festival energy: massive white-noise build-ups and thunderous big-room drops.".to_string(),
            },
            _ => Self {
                // international_party & default
                mood_id: "international_party".to_string(),
                warmup_fraction: 0.15,
                buildup_fraction: 0.20,
                peak_fraction: 0.26,
                sustain_fraction: 0.15,
                recovery_fraction: 0.12,
                renewed_peak_fraction: 0.08,
                finale_fraction: 0.04,
                warmup_target: 0.66,
                buildup_target: 0.76,
                peak_target: 0.88,
                sustain_target: 0.84,
                recovery_target: 0.68,
                renewed_peak_target: 0.90,
                finale_target: 0.74,
                max_consecutive_peaks: 2,
                recovery_interval_tracks: 6,
                curve_narrative: "Cosmopolitan global dancefloor: infectious syncopated Afrobeat, Latin dembow, and global pop.".to_string(),
            },
        }
    }

    /// Computes the active set phase based on set progress fraction (0.0 to 1.0).
    pub fn phase_for_progress(&self, progress: f64) -> SetPhase {
        let p = progress.clamp(0.0, 1.0);
        let mut accum = self.warmup_fraction;
        if p < accum {
            return SetPhase::WarmUp;
        }
        accum += self.buildup_fraction;
        if p < accum {
            return SetPhase::BuildUp;
        }
        accum += self.peak_fraction;
        if p < accum {
            return SetPhase::Peak;
        }
        accum += self.sustain_fraction;
        if p < accum {
            return SetPhase::Sustain;
        }
        accum += self.recovery_fraction;
        if p < accum {
            return SetPhase::Recovery;
        }
        accum += self.renewed_peak_fraction;
        if p < accum {
            return SetPhase::RenewedPeak;
        }
        SetPhase::Finale
    }

    /// Computes the target energy for a given phase.
    pub fn target_energy_for_phase(&self, phase: SetPhase) -> f64 {
        match phase {
            SetPhase::WarmUp => self.warmup_target,
            SetPhase::BuildUp => self.buildup_target,
            SetPhase::Peak => self.peak_target,
            SetPhase::Sustain => self.sustain_target,
            SetPhase::Recovery => self.recovery_target,
            SetPhase::RenewedPeak => self.renewed_peak_target,
            SetPhase::Finale => self.finale_target,
        }
    }
}

/// Action recommendation when planning how an incoming guest request fits the energy narrative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestRequestPlanningAction {
    pub request_id: String,
    pub title: String,
    pub original_priority: i32,
    pub target_phase: SetPhase,
    pub target_slot: usize,
    pub reasoning: String,
}

/// Comprehensive full-set energy storyteller and trajectory manager.
#[derive(Debug, Clone)]
pub struct SetStoryteller {
    pub mood_id: String,
    pub total_planned_tracks: usize,
    pub current_track_index: usize,
    pub consecutive_peak_tracks: usize,
    pub tracks_since_last_recovery: usize,
    pub curve: MoodStoryCurve,
    pub history_energies: Vec<f64>,
    pub history_phases: Vec<SetPhase>,
    pub hook_protection_count: usize,
    pub controlled_surprise_count: usize,
    pub recovery_interval_history: Vec<usize>,
}

impl SetStoryteller {
    pub fn new(mood_id: &str, total_planned_tracks: usize) -> Self {
        let curve = MoodStoryCurve::for_mood(mood_id);
        Self {
            mood_id: mood_id.to_string(),
            total_planned_tracks: total_planned_tracks.max(10),
            current_track_index: 0,
            consecutive_peak_tracks: 0,
            tracks_since_last_recovery: 0,
            curve,
            history_energies: Vec::new(),
            history_phases: Vec::new(),
            hook_protection_count: 0,
            controlled_surprise_count: 0,
            recovery_interval_history: Vec::new(),
        }
    }

    /// Determines the active phase for the next track selection.
    pub fn current_phase(&self) -> SetPhase {
        // If consecutive peak tracks exceed the maximum allowed for this mood, force a Recovery phase!
        if self.consecutive_peak_tracks >= self.curve.max_consecutive_peaks {
            return SetPhase::Recovery;
        }

        // If crowd has danced through many high-energy tracks without a breather, prioritize Recovery!
        if self.tracks_since_last_recovery >= self.curve.recovery_interval_tracks && self.current_track_index > 5 {
            return SetPhase::Recovery;
        }

        let progress = (self.current_track_index as f64) / (self.total_planned_tracks as f64);
        self.curve.phase_for_progress(progress)
    }

    /// Calculates the dynamic target energy for selecting the next track.
    pub fn calculate_target_energy(&self, _base_mood_target: f64) -> f64 {
        let phase = self.current_phase();
        self.curve.target_energy_for_phase(phase)
    }

    /// Evaluates how well a candidate's section-level energy profile fits the current set narrative phase.
    pub fn evaluate_candidate_energy_fit(
        &self,
        candidate_profile: &SectionEnergyProfile,
        cand_overall_energy: f64,
    ) -> (f64, Vec<String>, Vec<String>) {
        let phase = self.current_phase();
        let target_energy = self.curve.target_energy_for_phase(phase);
        let mut reasons = Vec::new();
        let mut penalties = Vec::new();

        // Section-level alignment: check whether the candidate's body and peak fit the phase
        let energy_delta = cand_overall_energy - target_energy;
        let mut score = 20.0 - (energy_delta.abs() * 18.0).clamp(0.0, 15.0);

        match phase {
            SetPhase::WarmUp => {
                if cand_overall_energy <= target_energy + 0.08 {
                    reasons.push(format!("Welcoming groove aligns with Warm-Up phase ({:.0}%)", cand_overall_energy * 100.0));
                    score += 5.0;
                } else {
                    penalties.push(format!("Excessive energy ({:.0}%) for Warm-Up phase", cand_overall_energy * 100.0));
                    score -= 8.0;
                }
            }
            SetPhase::BuildUp => {
                if cand_overall_energy >= target_energy - 0.06 && cand_overall_energy <= target_energy + 0.10 {
                    reasons.push("Steadily escalates dancefloor intensity toward Peak".to_string());
                    score += 5.0;
                }
            }
            SetPhase::Peak => {
                if cand_overall_energy >= target_energy - 0.06 {
                    reasons.push(format!("High-octane energy matches Peak dancefloor climax ({:.0}%)", cand_overall_energy * 100.0));
                    score += 6.0;
                    if candidate_profile.has_iconic_hook {
                        reasons.push("Iconic anthem chorus/drop provides massive dancefloor payoff".to_string());
                        score += 4.0;
                    }
                } else {
                    penalties.push(format!("Subdued energy ({:.0}%) interrupts dancefloor Peak momentum", cand_overall_energy * 100.0));
                    score -= 10.0;
                }
            }
            SetPhase::Sustain => {
                if (cand_overall_energy - target_energy).abs() <= 0.08 {
                    reasons.push("Maintains steady high-energy groove without crowd exhaustion".to_string());
                    score += 5.0;
                }
            }
            SetPhase::Recovery => {
                if cand_overall_energy <= target_energy + 0.06 {
                    reasons.push(format!("Intentional breathing room: gives dancers recovery space ({:.0}%)", cand_overall_energy * 100.0));
                    score += 8.0;
                } else {
                    penalties.push(format!("High energy ({:.0}%) denies essential crowd recovery window", cand_overall_energy * 100.0));
                    score -= 12.0;
                }
            }
            SetPhase::RenewedPeak => {
                if cand_overall_energy >= target_energy - 0.06 {
                    reasons.push(format!("Ignites renewed peak with high-energy floorfiller ({:.0}%)", cand_overall_energy * 100.0));
                    score += 6.0;
                    if candidate_profile.has_iconic_hook {
                        reasons.push("Famous singalong hook powers triumphant second climax".to_string());
                        score += 5.0;
                    }
                }
            }
            SetPhase::Finale => {
                if candidate_profile.has_iconic_hook || cand_overall_energy >= 0.70 {
                    reasons.push("Grand memorable anthem delivers satisfying set finale".to_string());
                    score += 5.0;
                }
            }
        }

        // Anti-Fatigue Guardrail: Check if consecutive peaks need to be halted
        if self.consecutive_peak_tracks >= self.curve.max_consecutive_peaks && cand_overall_energy >= self.curve.peak_target - 0.05 {
            penalties.push(format!(
                "Anti-Fatigue Guardrail: {} consecutive peak tracks played; ear fatigue risk requires recovery",
                self.consecutive_peak_tracks
            ));
            score -= 15.0;
        }

        (score.clamp(0.0, 30.0), reasons, penalties)
    }

    /// Records a track played into the storyteller set state, updating narrative counters.
    pub fn record_track_played(&mut self, energy: f64, is_hook_protected: bool, is_surprise: bool) {
        let phase = self.current_phase();
        self.history_energies.push(energy);
        self.history_phases.push(phase);
        self.current_track_index += 1;

        if is_hook_protected {
            self.hook_protection_count += 1;
        }
        if is_surprise {
            self.controlled_surprise_count += 1;
        }

        // Update peak & recovery counters
        if energy >= self.curve.peak_target - 0.05 {
            self.consecutive_peak_tracks += 1;
            self.tracks_since_last_recovery += 1;
        } else if energy <= self.curve.recovery_target + 0.06 {
            if self.tracks_since_last_recovery > 0 {
                self.recovery_interval_history.push(self.tracks_since_last_recovery);
            }
            self.consecutive_peak_tracks = 0;
            self.tracks_since_last_recovery = 0;
        } else {
            // Neutral sustain track
            if self.consecutive_peak_tracks > 0 {
                self.consecutive_peak_tracks -= 1;
            }
            self.tracks_since_last_recovery += 1;
        }
    }

    /// Dynamically replans where a guest request should be slotted into the macro-set narrative.
    pub fn replan_for_guest_request(
        &self,
        request_id: &str,
        request_title: &str,
        req_energy: f64,
        is_anthem: bool,
    ) -> GuestRequestPlanningAction {
        let current_p = self.current_phase();

        // High-energy bangers slot into Peak or Renewed Peak
        if req_energy >= self.curve.peak_target - 0.08 {
            if current_p == SetPhase::Peak {
                GuestRequestPlanningAction {
                    request_id: request_id.to_string(),
                    title: request_title.to_string(),
                    original_priority: 1,
                    target_phase: SetPhase::Peak,
                    target_slot: self.current_track_index + 1,
                    reasoning: format!("High-energy request '{}' slots immediately into active Peak dancefloor climax", request_title),
                }
            } else if current_p == SetPhase::Recovery {
                // Protect current recovery; delay to Renewed Peak!
                GuestRequestPlanningAction {
                    request_id: request_id.to_string(),
                    title: request_title.to_string(),
                    original_priority: 2,
                    target_phase: SetPhase::RenewedPeak,
                    target_slot: self.current_track_index + 2,
                    reasoning: format!("High-energy request '{}' queued for upcoming Renewed Peak to protect active recovery breather", request_title),
                }
            } else {
                GuestRequestPlanningAction {
                    request_id: request_id.to_string(),
                    title: request_title.to_string(),
                    original_priority: 1,
                    target_phase: SetPhase::Peak,
                    target_slot: self.current_track_index + 2,
                    reasoning: format!("Request '{}' scheduled as anchor track for upcoming Peak phase", request_title),
                }
            }
        } else if req_energy <= self.curve.recovery_target + 0.08 {
            // Softer request: slots naturally into recovery window
            GuestRequestPlanningAction {
                request_id: request_id.to_string(),
                title: request_title.to_string(),
                original_priority: 1,
                target_phase: SetPhase::Recovery,
                target_slot: self.current_track_index + (if current_p == SetPhase::Peak { 2 } else { 1 }),
                reasoning: format!("Acoustic/mellow request '{}' planned as crowd breathing room after peak energy", request_title),
            }
        } else {
            // Balanced groove request: slots into sustain or build
            GuestRequestPlanningAction {
                request_id: request_id.to_string(),
                title: request_title.to_string(),
                original_priority: 1,
                target_phase: SetPhase::Sustain,
                target_slot: self.current_track_index + 1,
                reasoning: format!("Versatile groove request '{}' slots into dancefloor sustain flow", request_title),
            }
        }
    }

    /// Compiles a comprehensive statistical report of the set narrative.
    pub fn generate_set_report(&self, genre_matches: usize, total_evaluated: usize, repetition_violations: usize) -> SetStoryReport {
        let total_tracks = self.history_energies.len();
        let peak_threshold = self.curve.peak_target - 0.05;
        let peaks_count = self.history_energies.iter().filter(|&&e| e >= peak_threshold).count();
        let peak_distribution_pct = if total_tracks > 0 {
            (peaks_count as f64 / total_tracks as f64) * 100.0
        } else {
            0.0
        };

        let recovery_threshold = self.curve.recovery_target + 0.06;
        let recovery_count = self.history_energies.iter().filter(|&&e| e <= recovery_threshold).count();

        let avg_recovery_interval = if !self.recovery_interval_history.is_empty() {
            self.recovery_interval_history.iter().sum::<usize>() as f64 / self.recovery_interval_history.len() as f64
        } else {
            0.0
        };

        let mood_consistency_pct = if total_evaluated > 0 {
            (genre_matches as f64 / total_evaluated as f64) * 100.0
        } else {
            100.0
        };

        SetStoryReport {
            total_tracks,
            mood_id: self.mood_id.clone(),
            energy_progression: self.history_energies.clone(),
            peak_count: peaks_count,
            peak_distribution_pct,
            recovery_count,
            recovery_intervals: self.recovery_interval_history.clone(),
            average_recovery_interval: avg_recovery_interval,
            max_consecutive_peaks_observed: self.curve.max_consecutive_peaks,
            hook_protection_events: self.hook_protection_count,
            controlled_surprises: self.controlled_surprise_count,
            mood_consistency_pct,
            repetition_violations,
        }
    }
}

/// Comprehensive summary of a full simulated or live DJ set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStoryReport {
    pub total_tracks: usize,
    pub mood_id: String,
    pub energy_progression: Vec<f64>,
    pub peak_count: usize,
    pub peak_distribution_pct: f64,
    pub recovery_count: usize,
    pub recovery_intervals: Vec<usize>,
    pub average_recovery_interval: f64,
    pub max_consecutive_peaks_observed: usize,
    pub hook_protection_events: usize,
    pub controlled_surprises: usize,
    pub mood_consistency_pct: f64,
    pub repetition_violations: usize,
}
