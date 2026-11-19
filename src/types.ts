export type DjLevel = 1 | 2 | 3 | 4;

export type DjTechniqueId =
  | 'safe_quick_cut'
  | 'eq3_handover'
  | 'phrase_long_blend'
  | 'timed_bass_swap'
  | 'filter_sweep_drop'
  | 'echo_out_blend'
  | 'beat_roll_buildup'
  | 'filtered_buildup_drop'
  | 'four_deck_layer'
  | 'stutter_gate_drop'
  | 'backspin_cut'
  | 'echo_freeze_drop'
  | 'tempo_ramp_bridge'
  | 'creative_mashup_layer'
  | 'bass_swap'
  | 'quick_cut'
  | 'phrase_sync'
  | 'filter_sweep';

export interface DjSkillProfile {
  level: DjLevel;
  name: string;
  summary: string;
  exposed_decks: number;
  internal_deck_capacity: number;
  max_tempo_stretch_pct: number;
  supports_auto_bpm_match: boolean;
  supports_half_double_match: boolean;
  supports_harmonic_key_match: boolean;
  supports_gain_match: boolean;
  supports_bass_swap: boolean;
  supports_echo_out: boolean;
  supports_beat_roll: boolean;
  supports_four_deck_layering: boolean;
  supports_stutter_gate: boolean;
  supports_backspin_cut: boolean;
  supports_echo_freeze: boolean;
  supports_tempo_ramp: boolean;
  supports_creative_mashup: boolean;
  unlocked_techniques: string[];
}

export interface DjRecommendation {
  submission_id: string;
  title: string;
  artist: string;
  category: 'best_next' | 'harmonic_match' | 'energy_builder' | 'safe_transition' | 'creative_layer' | string;
  category_label: string;
  compatibility_score: number;
  confidence: number;
  is_playable: boolean;
  playability_label: 'playable_now' | 'processing' | 'metadata_only_restricted' | 'failed' | string;
  reasons: string[];
  known_signals: string[];
  missing_signals: string[];
  suggested_technique: string;
  technique_label: string;
  technique_reason: string;
}

export interface TrackSectionMarker {
  section_type:
    | 'intro'
    | 'verse'
    | 'chorus'
    | 'instrumental'
    | 'groove'
    | 'breakdown'
    | 'build'
    | 'drop'
    | 'peak'
    | 'outro'
    | 'phrase'
    | string;
  start_ms: number;
  end_ms: number;
  energy_level: number;
  bass_energy?: number;
  vocal_activity?: number;
  confidence?: number;
  bar_index?: number;
}

export interface TempoSegment {
  start_ms: number;
  end_ms: number;
  bpm: number;
  confidence: number;
}

export interface MeterAnalysis {
  beats_per_bar: number;
  meter_label: '4/4' | '3/4' | '6/8' | 'uncertain' | string;
  confidence: number;
}

export interface TrackTimingProfile {
  average_bpm: number;
  beatgrid_confidence: number;
  bpm_confidence?: number;
  downbeat_confidence?: number;
  downbeat_offset_ms: number;
  is_variable_tempo: boolean;
  tempo_drift_bpm: number;
  meter: MeterAnalysis;
  tempo_map: TempoSegment[];
  beat_timestamps_ms: number[];
  downbeat_timestamps_ms: number[];
}

export type BpmRelationshipType =
  | 'direct'
  | 'half_time'
  | 'double_time'
  | 'three_quarter_bridge'
  | 'gradual_tempo_ramp'
  | 'no_beatmatch';

export interface EqBassHandoverSchedule {
  incoming_low_cut_until_frac: number;
  bass_swap_at_frac: number;
  outgoing_low_cut_at_frac: number;
  mid_carve_db: number;
  high_handover_frac: number;
}

export interface RejectedTechniqueAlternative {
  technique: string;
  reason: string;
}

export interface TransitionDecisionPlan {
  status: 'planned' | 'rejected' | 'deferred' | 'cancelled' | 'executed' | string;
  outgoing_track_id: string;
  outgoing_deck_id: number;
  incoming_track_id: string;
  incoming_deck_id: number;
  outgoing_mix_out_sec: number;
  outgoing_phrase_boundary_sec: number;
  incoming_mix_in_sec: number;
  incoming_phrase_boundary_sec: number;
  outgoing_section_type: string;
  incoming_section_type: string;
  current_bpm?: number | null;
  incoming_bpm?: number | null;
  target_bpm?: number | null;
  bpm_relationship: BpmRelationshipType;
  tempo_adjustment_pct: number;
  tempo_direction: 'up' | 'down' | 'none' | string;
  incoming_playback_rate: number;
  outgoing_target_rate: number;
  beat_alignment_offset_sec: number;
  meter_compatible: boolean;
  meter_summary: string;
  beatgrid_confidence: number;
  bpm_confidence?: number;
  downbeat_confidence?: number;
  meter_confidence?: number;
  duration_beats: number;
  duration_bars: number;
  duration_sec: number;
  audio_crossfade_sec?: number;
  musical_duration_sec?: number;
  vocal_overlap_policy: string;
  energy_progression: string;
  eq_bass_handover_schedule: EqBassHandoverSchedule;
  selected_technique: DjTechniqueId;
  scheduled_execution_sec: number;
  confidence: number;
  reasons: string[];
  rejected_alternatives: RejectedTechniqueAlternative[];
  rejection_reason?: string | null;
}

export interface DjTechniqueExecutionRecord {
  id: string;
  timestamp: number;
  djLevel: DjLevel;
  techniqueId: DjTechniqueId;
  techniqueLabel: string;
  outgoingTitle: string;
  incomingTitle: string;
  fromDeckId: number;
  toDeckId: number;
  durationSec: number;
  tempoMatchRatio: number;
  gainMatchDb: number;
  fallbackUsed: boolean;
  reason: string;
  sectionLabel?: string;
  cueInSec?: number;
  cueOutSec?: number;
  layeredDeckId?: number | null;
}

export interface MoodProfile {
  id: string;
  display_name: string;
  description: string;
  min_bpm: number;
  max_bpm: number;
  target_energy: number;
  transition_style: 'bass_swap' | 'quick_cut' | 'phrase_sync' | 'filter_sweep' | string;
  repetition_cooldown_mins: number;
  artist_cooldown_mins: number;
  preferred_fx_index?: number;
  genre_weights?: [string, number][];
}

export interface LocalPartyRoom {
  room_id: string;
  room_code: string;
  party_name: string;
  status: 'created' | 'live' | 'closed';
  dj_level: DjLevel;
  primary_mood_id: string;
  secondary_mood_id?: string | null;
  created_at: number;
  started_at?: number | null;
  closed_at?: number | null;
  updated_at: number;
}

export interface LocalRoomSubmission {
  id: string;
  room_id: string;
  submitted_by_type: 'guest' | 'host';
  submitted_by_id: string;
  submitted_by_name: string;
  provider: string; // 'youtube' | 'youtube_music'
  source_url: string;
  provider_video_id: string;
  song_id?: string | null;
  title: string;
  artist: string;
  version_tag: string;
  thumbnail_url?: string | null;
  duration_ms?: number | null;
  bpm?: number | null;
  half_time_bpm?: number | null;
  double_time_bpm?: number | null;
  musical_key?: string | null;
  camelot_key?: string | null;
  overall_energy?: number | null;
  danceability?: number | null;
  metadata_source: string;
  acquisition_status: 'restricted_by_provider' | 'cached_authorized' | 'pending' | 'failed' | string;
  acquisition_note?: string | null;
  dj_status: 'queued_for_dj' | 'prepared' | 'playing' | 'played' | 'skipped_unplayable' | string;
  dj_score: number;
  cache_id?: string | null;
  created_at: number;
  updated_at: number;
}

export interface PlayableCandidatePointer {
  submission_id: string;
  cache_id: string;
  stream_url: string;
  title: string;
  artist: string;
  bpm?: number | null;
  camelot_key?: string | null;
  duration_ms?: number | null;
  overall_energy?: number | null;
  genre_tags?: string[];
  sections?: TrackSectionMarker[];
  timing_profile?: TrackTimingProfile | null;
}

export interface LocalTempCacheStats {
  track_count: number;
  max_tracks: number;
  total_bytes: number;
  locked_tracks: number;
}

export interface SyncStatusOverview {
  pending_count: number;
  in_flight_count: number;
  synced_count: number;
  failed_count: number;
}

export interface LocalHelperHealth {
  status: string;
  service: string;
  version: string;
  port: number;
  lan_reachable: boolean;
  detected_lan_ip: string | null;
  cache_stats: LocalTempCacheStats;
  sync_overview: SyncStatusOverview;
  timestamp: number;
}

export interface CreateLocalRoomResponse {
  room: LocalPartyRoom;
  host_token: string;
  room_access_token: string;
  lan_reachable: boolean;
  detected_lan_ip: string | null;
  guest_url: string | null;
  qr_svg: string | null;
  lan_notice: string | null;
}

export interface HostRoomDashboardResponse {
  room: LocalPartyRoom;
  dj_level_name: string;
  dj_skill_profile?: DjSkillProfile;
  exposed_decks: number;
  internal_deck_capacity: number;
  auto_dj_enabled: boolean;
  lan_reachable: boolean;
  detected_lan_ip: string | null;
  submissions: LocalRoomSubmission[];
  recommendations?: DjRecommendation[];
  pending_intake_jobs?: number;
  next_playable_candidate: PlayableCandidatePointer | null;
  transition_plan?: TransitionDecisionPlan | null;
  transition_style: string;
  cache_stats: LocalTempCacheStats;
  sync_overview: SyncStatusOverview;
}

export interface UnifiedSongSubmissionResponse {
  id: string;
  submitted_by_type: string;
  title: string;
  artist: string;
  version_tag: string;
  provider: string;
  status: string;
  metadata_source: string;
  acquisition_status: string;
  acquisition_note: string;
  cache_id?: string | null;
  dj_score: number;
  interrupted_current_track: boolean;
}

export interface LocalRoomEvent {
  room_id: string;
  event_type: string;
  target_guest_id?: string | null;
  host_only: boolean;
  payload: any;
  timestamp: number;
}

// Legacy compatibility shapes for auxiliary screens
export interface DeckTelemetry {
  deck_id: number;
  track_id?: string;
  position_ms: number;
  duration_ms: number;
  bpm: number;
  tempo_rate: number;
  is_playing: boolean;
  vu_l_db: number;
  vu_r_db: number;
}

export interface MasterTelemetry {
  vu_master_l_db: number;
  vu_master_r_db: number;
  crossfader_pos: number;
  underrun_count: number;
  cpu_load_pct: number;
  timestamp_ms: number;
  decks: DeckTelemetry[];
}
