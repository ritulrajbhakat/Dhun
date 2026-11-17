export type DjLevel = 1 | 2 | 3 | 4;

export interface MoodProfile {
  id: string;
  display_name: string;
  description: string;
  min_bpm: number;
  max_bpm: number;
  target_energy: number;
  transition_style: string;
  repetition_cooldown_mins: number;
  artist_cooldown_mins: number;
  genre_weights?: [string, number][];
}

export interface Track {
  id: string;
  title: string;
  artist: string;
  album?: string;
  duration_ms: number;
  year?: number;
  genre: string;
  subgenre?: string;
  created_at: number;
  updated_at: number;
}

export interface TrackAnalysis {
  track_id: string;
  bpm: number;
  bpm_confidence: number;
  key_name: string;
  camelot_key: string;
  key_confidence: number;
  downbeat_offset_ms: number;
  phrase_length_bars: number;
  overall_energy: number;
  dance_suitability: number;
  recommended_mix_in_ms: number;
  recommended_mix_out_ms: number;
}

export interface PlayableTrackCandidate {
  track: Track;
  asset: {
    id: string;
    file_path: string;
    file_format: string;
    verification_status: string;
    availability_status: string;
  };
  analysis: TrackAnalysis;
  compatibility_score: number;
  transition_style: string;
}

export interface GuestRequest {
  id: string;
  session_id: string;
  guest_id: string;
  guest_name?: string;
  track_id: string;
  track_title?: string;
  track_artist?: string;
  submitted_at: number;
  status: 'pending' | 'approved' | 'rejected' | 'scheduled' | 'playing' | 'played' | 'deferred';
  priority_order: number;
  host_note?: string;
}

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

export interface PartyStatus {
  is_active: boolean;
  session_id?: string;
  party_name?: string;
  primary_mood_id?: string;
  dj_level?: DjLevel;
  guest_url?: string;
  qr_svg?: string;
  now_playing?: PlayableTrackCandidate;
  next_track?: PlayableTrackCandidate;
  following_track?: PlayableTrackCandidate;
}

export interface AudioDeviceInfo {
  name: string;
  is_default: boolean;
  default_sample_rate: number;
  channels: number;
}

// ========================================================================
// Cloud Party Foundation Types (Phase 1)
// ========================================================================

export interface CloudParty {
  id: string;
  party_name: string;
  status: 'created' | 'live' | 'paused' | 'ended';
  created_at: number;
  updated_at: number;
  started_at?: number;
  ended_at?: number;
  last_heartbeat_at: number;
}

export interface PartyConfiguration {
  party_id: string;
  dj_level: string; // 'hands_off' | 'balanced' | 'curated' | 'manual'
  primary_mood_id: string;
  secondary_mood_id?: string;
  energy_progression: string;
  allow_guest_requests: boolean;
  max_requests_per_guest: number;
  request_cooldown_mins: number;
  require_host_approval: boolean;
  updated_at: number;
}

export interface PartySourceReference {
  id: string;
  party_id: string;
  provider: string; // 'youtube' | 'youtube_music'
  original_url: string;
  provider_video_id: string;
  title: string;
  artist: string;
  thumbnail_url?: string;
  duration_seconds?: number;
  verification_status: string;
  playback_method: string;
  playability_status: string; // 'playback_unverified' | 'playable' | 'unavailable'
  metadata_status: string;
  error_reason?: string;
  registered_at: number;
  updated_at: number;
}

export interface PartySongRequest {
  id: string;
  party_id: string;
  guest_id: string;
  guest_name?: string;
  source_id: string;
  source_title?: string;
  source_artist?: string;
  thumbnail_url?: string;
  custom_note?: string;
  status: 'pending' | 'approved' | 'rejected' | 'played' | 'cancelled';
  rejection_reason?: string;
  priority_score: number;
  created_at: number;
  updated_at: number;
}

export interface CreatePartyResponse {
  party_id: string;
  party_name: string;
  status: 'created' | 'live' | 'paused' | 'ended';
  host_token: string;
  guest_url: string;
  qr_svg: string;
  config: PartyConfiguration;
  created_at: number;
}

export interface PartyDetailsResponse {
  party: CloudParty;
  config?: PartyConfiguration;
  guest_url: string;
  qr_svg: string;
  sources_count: number;
  pending_requests_count: number;
}

export interface PartyEventMessage {
  party_id: string;
  event_type: string;
  payload: any;
  timestamp: number;
}


