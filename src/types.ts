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

