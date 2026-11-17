export type DjLevel = 1 | 2 | 3 | 4;

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
  local_cache_path?: string | null;
  created_at: number;
  updated_at: number;
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
  exposed_decks: number;
  internal_deck_capacity: number;
  auto_dj_enabled: boolean;
  lan_reachable: boolean;
  detected_lan_ip: string | null;
  submissions: LocalRoomSubmission[];
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
