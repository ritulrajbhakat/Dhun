import { PartyStatus, MasterTelemetry, MoodProfile, AudioDeviceInfo, Track, GuestRequest, PlayableTrackCandidate } from '../types';

// Check if running inside native Tauri runtime
export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// Sample Indian Party Tracks for offline fallback & rich testing
export const MOCK_TRACKS: PlayableTrackCandidate[] = [
  {
    track: {
      id: 'trk_01',
      title: 'Gallan Goodiyaan',
      artist: 'Diljit Dosanjh, Shankar Mahadevan',
      album: 'Dil Dhadakne Do',
      duration_ms: 296000,
      genre: 'Bollywood Dance',
      created_at: Date.now() - 1000000,
      updated_at: Date.now() - 1000000,
    },
    asset: {
      id: 'ast_01',
      file_path: 'C:/Music/Gallan_Goodiyaan.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_01',
      bpm: 128.0,
      bpm_confidence: 0.98,
      key_name: 'D Major',
      camelot_key: '10B',
      key_confidence: 0.95,
      downbeat_offset_ms: 120,
      phrase_length_bars: 32,
      overall_energy: 0.92,
      dance_suitability: 0.96,
      recommended_mix_in_ms: 15000,
      recommended_mix_out_ms: 280000,
    },
    compatibility_score: 0.95,
    transition_style: 'bass_swap',
  },
  {
    track: {
      id: 'trk_02',
      title: 'London Thumakda',
      artist: 'Labh Janjua, Sonu Kakkar, Neha Kakkar',
      album: 'Queen',
      duration_ms: 230000,
      genre: 'Punjabi Wedding',
      created_at: Date.now() - 900000,
      updated_at: Date.now() - 900000,
    },
    asset: {
      id: 'ast_02',
      file_path: 'C:/Music/London_Thumakda.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_02',
      bpm: 128.0,
      bpm_confidence: 0.96,
      key_name: 'G Major',
      camelot_key: '9B',
      key_confidence: 0.92,
      downbeat_offset_ms: 80,
      phrase_length_bars: 32,
      overall_energy: 0.88,
      dance_suitability: 0.94,
      recommended_mix_in_ms: 12000,
      recommended_mix_out_ms: 215000,
    },
    compatibility_score: 0.92,
    transition_style: 'phrase_sync',
  },
  {
    track: {
      id: 'trk_03',
      title: 'Sauda Khara Khara',
      artist: 'Diljit Dosanjh, Sukhbir, Dhvani Bhanushali',
      album: 'Good Newwz',
      duration_ms: 228000,
      genre: 'Bhangra Dhol',
      created_at: Date.now() - 800000,
      updated_at: Date.now() - 800000,
    },
    asset: {
      id: 'ast_03',
      file_path: 'C:/Music/Sauda_Khara_Khara.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_03',
      bpm: 130.0,
      bpm_confidence: 0.99,
      key_name: 'F Minor',
      camelot_key: '4A',
      key_confidence: 0.94,
      downbeat_offset_ms: 150,
      phrase_length_bars: 32,
      overall_energy: 0.95,
      dance_suitability: 0.98,
      recommended_mix_in_ms: 14000,
      recommended_mix_out_ms: 212000,
    },
    compatibility_score: 0.89,
    transition_style: 'quick_cut',
  },
  {
    track: {
      id: 'trk_04',
      title: 'Kala Chashma',
      artist: 'Amar Arshi, Badshah, Neha Kakkar',
      album: 'Baar Baar Dekho',
      duration_ms: 247000,
      genre: 'Desi Club',
      created_at: Date.now() - 700000,
      updated_at: Date.now() - 700000,
    },
    asset: {
      id: 'ast_04',
      file_path: 'C:/Music/Kala_Chashma.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_04',
      bpm: 106.0,
      bpm_confidence: 0.97,
      key_name: 'C# Minor',
      camelot_key: '12A',
      key_confidence: 0.91,
      downbeat_offset_ms: 100,
      phrase_length_bars: 32,
      overall_energy: 0.91,
      dance_suitability: 0.95,
      recommended_mix_in_ms: 18000,
      recommended_mix_out_ms: 230000,
    },
    compatibility_score: 0.86,
    transition_style: 'filter_sweep',
  },
  {
    track: {
      id: 'trk_05',
      title: 'Chogada',
      artist: 'Darshan Raval, Asees Kaur',
      album: 'Loveyatri',
      duration_ms: 249000,
      genre: 'Garba Folk',
      created_at: Date.now() - 600000,
      updated_at: Date.now() - 600000,
    },
    asset: {
      id: 'ast_05',
      file_path: 'C:/Music/Chogada.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_05',
      bpm: 122.0,
      bpm_confidence: 0.98,
      key_name: 'A Major',
      camelot_key: '11B',
      key_confidence: 0.93,
      downbeat_offset_ms: 110,
      phrase_length_bars: 32,
      overall_energy: 0.87,
      dance_suitability: 0.96,
      recommended_mix_in_ms: 16000,
      recommended_mix_out_ms: 235000,
    },
    compatibility_score: 0.91,
    transition_style: 'phrase_sync',
  },
  {
    track: {
      id: 'trk_06',
      title: 'Balam Pichkari',
      artist: 'Vishal Dadlani, Shalmali Kholgade',
      album: 'Yeh Jawaani Hai Deewani',
      duration_ms: 289000,
      genre: 'Bollywood Dance',
      created_at: Date.now() - 500000,
      updated_at: Date.now() - 500000,
    },
    asset: {
      id: 'ast_06',
      file_path: 'C:/Music/Balam_Pichkari.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_06',
      bpm: 120.0,
      bpm_confidence: 0.97,
      key_name: 'C Major',
      camelot_key: '8B',
      key_confidence: 0.93,
      downbeat_offset_ms: 120,
      phrase_length_bars: 32,
      overall_energy: 0.90,
      dance_suitability: 0.94,
      recommended_mix_in_ms: 15000,
      recommended_mix_out_ms: 274000,
    },
    compatibility_score: 0.88,
    transition_style: 'bass_swap',
  },
  {
    track: {
      id: 'trk_07',
      title: 'Mundian To Bach Ke',
      artist: 'Panjabi MC',
      album: 'Legalised',
      duration_ms: 244000,
      genre: 'Punjabi Pop',
      created_at: Date.now() - 400000,
      updated_at: Date.now() - 400000,
    },
    asset: {
      id: 'ast_07',
      file_path: 'C:/Music/Mundian_To_Bach_Ke.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_07',
      bpm: 100.0,
      bpm_confidence: 0.98,
      key_name: 'Ab Minor',
      camelot_key: '1A',
      key_confidence: 0.95,
      downbeat_offset_ms: 120,
      phrase_length_bars: 32,
      overall_energy: 0.93,
      dance_suitability: 0.96,
      recommended_mix_in_ms: 12000,
      recommended_mix_out_ms: 232000,
    },
    compatibility_score: 0.85,
    transition_style: 'quick_cut',
  },
  {
    track: {
      id: 'trk_08',
      title: 'Aankh Marey',
      artist: 'Neha Kakkar, Mika Singh, Kumar Sanu',
      album: 'Simmba',
      duration_ms: 212000,
      genre: 'Desi Club',
      created_at: Date.now() - 300000,
      updated_at: Date.now() - 300000,
    },
    asset: {
      id: 'ast_08',
      file_path: 'C:/Music/Aankh_Marey.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_08',
      bpm: 124.0,
      bpm_confidence: 0.97,
      key_name: 'F Major',
      camelot_key: '7B',
      key_confidence: 0.92,
      downbeat_offset_ms: 120,
      phrase_length_bars: 32,
      overall_energy: 0.91,
      dance_suitability: 0.95,
      recommended_mix_in_ms: 14000,
      recommended_mix_out_ms: 198000,
    },
    compatibility_score: 0.90,
    transition_style: 'filter_sweep',
  },
  {
    track: {
      id: 'trk_09',
      title: 'Kamariya',
      artist: 'Darshan Raval',
      album: 'Mitron',
      duration_ms: 188000,
      genre: 'Garba Folk',
      created_at: Date.now() - 200000,
      updated_at: Date.now() - 200000,
    },
    asset: {
      id: 'ast_09',
      file_path: 'C:/Music/Kamariya.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_09',
      bpm: 124.0,
      bpm_confidence: 0.96,
      key_name: 'F# Major',
      camelot_key: '2B',
      key_confidence: 0.91,
      downbeat_offset_ms: 120,
      phrase_length_bars: 32,
      overall_energy: 0.89,
      dance_suitability: 0.95,
      recommended_mix_in_ms: 15000,
      recommended_mix_out_ms: 173000,
    },
    compatibility_score: 0.89,
    transition_style: 'phrase_sync',
  },
  {
    track: {
      id: 'trk_10',
      title: 'Disco Deewane',
      artist: 'Benny Dayal, Sunidhi Chauhan, Vishal Dadlani',
      album: 'Student of the Year',
      duration_ms: 342000,
      genre: 'Retro Bollywood',
      created_at: Date.now() - 100000,
      updated_at: Date.now() - 100000,
    },
    asset: {
      id: 'ast_10',
      file_path: 'C:/Music/Disco_Deewane.mp3',
      file_format: 'mp3',
      verification_status: 'verified',
      availability_status: 'available',
    },
    analysis: {
      track_id: 'trk_10',
      bpm: 128.0,
      bpm_confidence: 0.98,
      key_name: 'Bb Major',
      camelot_key: '6B',
      key_confidence: 0.94,
      downbeat_offset_ms: 120,
      phrase_length_bars: 32,
      overall_energy: 0.85,
      dance_suitability: 0.92,
      recommended_mix_in_ms: 15000,
      recommended_mix_out_ms: 327000,
    },
    compatibility_score: 0.92,
    transition_style: 'bass_swap',
  },
];

export const MOCK_MOODS: MoodProfile[] = [
  { id: 'shaadi', display_name: 'Shaadi', description: 'Grand wedding celebrations combining Bollywood, Punjabi, and wedding remixes.', min_bpm: 100, max_bpm: 130, target_energy: 0.75, transition_style: 'bass_swap', repetition_cooldown_mins: 120, artist_cooldown_mins: 45 },
  { id: 'baraat', display_name: 'Baraat', description: 'Electrifying high-octane procession dhol beats, brass hits, and Punjabi hype.', min_bpm: 126, max_bpm: 145, target_energy: 0.95, transition_style: 'quick_cut', repetition_cooldown_mins: 90, artist_cooldown_mins: 30 },
  { id: 'mehendi', display_name: 'Mehendi', description: 'Melodic folk, contemporary acoustic Bollywood, and warm joyful Sufi rhythms.', min_bpm: 85, max_bpm: 110, target_energy: 0.55, transition_style: 'phrase_sync', repetition_cooldown_mins: 150, artist_cooldown_mins: 60 },
  { id: 'haldi', display_name: 'Haldi', description: 'Upbeat traditional folk celebration, lively acoustic percussion, and festive melodies.', min_bpm: 95, max_bpm: 118, target_energy: 0.65, transition_style: 'phrase_sync', repetition_cooldown_mins: 150, artist_cooldown_mins: 60 },
  { id: 'sangeet', display_name: 'Sangeet', description: 'Dynamic dance choreography anthems with energetic stage peaks.', min_bpm: 115, max_bpm: 135, target_energy: 0.85, transition_style: 'bass_swap', repetition_cooldown_mins: 120, artist_cooldown_mins: 40 },
  { id: 'desi_club', display_name: 'Desi Club', description: 'Heavy 4x4 commercial desi remixes, tech house fusions, and club bangers.', min_bpm: 124, max_bpm: 132, target_energy: 0.90, transition_style: 'filter_sweep', repetition_cooldown_mins: 120, artist_cooldown_mins: 45 },
  { id: 'punjabi_tashan', display_name: 'Punjabi Tashan', description: 'Heavy 808s, UK Punjabi bhangra, swagger anthems, and modern urban vibes.', min_bpm: 128, max_bpm: 142, target_energy: 0.90, transition_style: 'quick_cut', repetition_cooldown_mins: 90, artist_cooldown_mins: 30 },
  { id: 'garba', display_name: 'Garba', description: 'Traditional 3-Taali and 2-Taali continuous cyclic rhythm with accelerating tempo.', min_bpm: 110, max_bpm: 140, target_energy: 0.80, transition_style: 'phrase_sync', repetition_cooldown_mins: 180, artist_cooldown_mins: 60 },
  { id: 'dandiya', display_name: 'Dandiya', description: 'Fast synchronized sticks tempo with driving percussion and folk celebration.', min_bpm: 120, max_bpm: 144, target_energy: 0.88, transition_style: 'phrase_sync', repetition_cooldown_mins: 180, artist_cooldown_mins: 60 },
  { id: 'retro_bollywood', display_name: 'Retro Bollywood', description: 'Timeless 70s and 80s RD Burman disco funk, Kishore Kumar, and golden hits.', min_bpm: 90, max_bpm: 125, target_energy: 0.60, transition_style: 'phrase_sync', repetition_cooldown_mins: 180, artist_cooldown_mins: 60 },
  { id: 'house_party', display_name: 'House Party', description: 'Versatile mix of international pop, trending hip-hop, and easy-dance tunes.', min_bpm: 105, max_bpm: 126, target_energy: 0.70, transition_style: 'bass_swap', repetition_cooldown_mins: 120, artist_cooldown_mins: 45 },
  { id: 'cocktail_night', display_name: 'Cocktail Night', description: 'Sophisticated deep house, melodic chill, smooth lounge, and classy reworks.', min_bpm: 118, max_bpm: 124, target_energy: 0.60, transition_style: 'filter_sweep', repetition_cooldown_mins: 180, artist_cooldown_mins: 60 },
  { id: 'festival_mode', display_name: 'Festival Mode', description: 'Peak stadium energy, massive drops, EDM fusions, and explosive transitions.', min_bpm: 126, max_bpm: 138, target_energy: 0.98, transition_style: 'quick_cut', repetition_cooldown_mins: 90, artist_cooldown_mins: 30 },
  { id: 'international_party', display_name: 'International Party', description: 'Global chart-toppers, reggaeton, Afrobeat, and commercial dance.', min_bpm: 100, max_bpm: 130, target_energy: 0.80, transition_style: 'bass_swap', repetition_cooldown_mins: 120, artist_cooldown_mins: 45 },
];

let mockPartyState: PartyStatus = {
  is_active: false,
};

let mockRequests: GuestRequest[] = [
  {
    id: 'req_101',
    session_id: 'ses_mock',
    guest_id: 'gst_99a',
    guest_name: 'Aman',
    track_id: 'trk_01',
    track_title: 'Gallan Goodiyaan',
    track_artist: 'Diljit Dosanjh',
    submitted_at: Date.now() - 120000,
    status: 'approved',
    priority_order: 10,
    host_note: 'Groom entrance song!',
  },
  {
    id: 'req_102',
    session_id: 'ses_mock',
    guest_id: 'gst_77b',
    guest_name: 'Priya',
    track_id: 'trk_05',
    track_title: 'Chogada',
    track_artist: 'Darshan Raval',
    submitted_at: Date.now() - 60000,
    status: 'pending',
    priority_order: 100,
  },
];

export async function apiStartParty(payload: {
  party_name: string;
  dj_level: number;
  primary_mood_id: string;
  secondary_mood_id?: string;
  output_device_name: string;
}): Promise<PartyStatus> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('start_party', { payload });
  }

  // Simulated fallback
  mockPartyState = {
    is_active: true,
    session_id: 'ses_local_' + Math.random().toString(36).substring(2, 8),
    party_name: payload.party_name,
    primary_mood_id: payload.primary_mood_id,
    dj_level: payload.dj_level as any,
    guest_url: 'http://192.168.1.100:3000/',
    qr_svg: `<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg" class="w-full h-full"><rect width="100" height="100" fill="#12141a"/><path d="M10 10h30v30h-30zM15 15v20h20v-20zM60 10h30v30h-30zM65 15v20h20v-20zM10 60h30v30h-30zM15 65v20h20v-20zM45 45h10v10h-10zM60 60h10v10h-10zM75 75h15v15h-15z" fill="#f43f5e"/></svg>`,
    now_playing: MOCK_TRACKS[0],
    next_track: MOCK_TRACKS[1],
    following_track: MOCK_TRACKS[2],
  };
  return mockPartyState;
}

export async function apiGetPartyStatus(): Promise<PartyStatus> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_party_status');
  }
  return mockPartyState;
}

export async function apiGetTelemetry(): Promise<MasterTelemetry> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_telemetry');
  }

  // Simulated VU metering & position playback for UI
  const now = Date.now();
  const posA = (now % 240000);
  const posB = ((now + 120000) % 240000);

  return {
    vu_master_l_db: mockPartyState.is_active ? -6.0 + Math.random() * 3.5 : -96.0,
    vu_master_r_db: mockPartyState.is_active ? -6.2 + Math.random() * 3.5 : -96.0,
    crossfader_pos: -0.15,
    underrun_count: 0,
    cpu_load_pct: 1.4,
    timestamp_ms: now,
    decks: [
      {
        deck_id: 0,
        track_id: mockPartyState.now_playing?.track.id,
        position_ms: posA,
        duration_ms: 296000,
        bpm: 128.0,
        tempo_rate: 1.0,
        is_playing: mockPartyState.is_active,
        vu_l_db: mockPartyState.is_active ? -3.0 + Math.random() * 2 : -96.0,
        vu_r_db: mockPartyState.is_active ? -3.0 + Math.random() * 2 : -96.0,
      },
      {
        deck_id: 1,
        track_id: mockPartyState.next_track?.track.id,
        position_ms: posB,
        duration_ms: 230000,
        bpm: 128.0,
        tempo_rate: 1.0,
        is_playing: false,
        vu_l_db: -96.0,
        vu_r_db: -96.0,
      },
    ],
  };
}

export async function apiEmergencyFade(durationMs: number = 300): Promise<void> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('emergency_fade', { durationMs });
  }
  if (mockPartyState) {
    mockPartyState.is_active = false;
  }
}

export async function apiGetMoodProfiles(): Promise<MoodProfile[]> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_mood_profiles');
  }
  return MOCK_MOODS;
}

export async function apiGetAudioDevices(): Promise<AudioDeviceInfo[]> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_audio_devices');
  }
  return [
    { name: 'Speakers / Headphones (Realtek Audio)', is_default: true, default_sample_rate: 48000, channels: 2 },
    { name: 'Pioneer DJ DDJ-FLX4 Audio Output', is_default: false, default_sample_rate: 44100, channels: 4 },
  ];
}

export async function apiGetTracks(): Promise<Track[]> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_tracks');
  }
  return MOCK_TRACKS.map(c => c.track);
}

export async function apiGetGuestRequests(): Promise<GuestRequest[]> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_guest_requests', { sessionId: mockPartyState.session_id || 'default' });
  }
  return mockRequests;
}

export async function apiUpdateRequest(requestId: string, status: string, priority?: number): Promise<void> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('update_guest_request_status', { requestId, status, priority });
  }
  const req = mockRequests.find(r => r.id === requestId);
  if (req) {
    req.status = status as any;
    if (priority !== undefined) req.priority_order = priority;
  }
}

export async function apiForceNext(requestId: string): Promise<void> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('force_next_request', { requestId });
  }
  const req = mockRequests.find(r => r.id === requestId);
  if (req) {
    req.status = 'playing';
    req.priority_order = 1;
    const candidate = MOCK_TRACKS.find(t => t.track.id === req.track_id);
    if (candidate) {
      mockPartyState.next_track = candidate;
    }
  }
}

export async function apiDeckControl(action: string, deckId: number, params: any = {}): Promise<void> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    switch (action) {
      case 'play': return invoke('deck_play', { deckId });
      case 'pause': return invoke('deck_pause', { deckId });
      case 'seek': return invoke('deck_seek', { deckId, positionMs: params.positionMs });
      case 'tempo': return invoke('deck_set_tempo', { deckId, rate: params.rate });
      case 'gain': return invoke('deck_set_gain', { deckId, gain: params.gain });
      case 'eq': return invoke('deck_set_eq', { deckId, lowDb: params.lowDb, midDb: params.midDb, highDb: params.highDb });
      case 'filter': return invoke('deck_set_filter', { deckId, position: params.position });
    }
  }
  // Console logging in mock mode
  console.log(`[Deck ${deckId}] ${action}:`, params);
}

export async function apiSetCrossfader(position: number, curve: string = 'bass_swap'): Promise<void> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('set_crossfader', { position, curve });
  }
}

export async function apiScanLibraryFolder(folderPath: string): Promise<number> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('scan_library_folder', { folderPath });
  }
  return MOCK_TRACKS.length;
}

export async function apiSetDjLevel(level: number): Promise<void> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('set_dj_level', { level });
  }
  if (mockPartyState) {
    mockPartyState.dj_level = level as any;
  }
  console.log(`[IPC Mock] Set DJ Level to ${level}`);
}

export async function apiSetPartyMood(moodId: string): Promise<void> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('set_party_mood', { moodId });
  }
  if (mockPartyState) {
    mockPartyState.primary_mood_id = moodId;
  }
  console.log(`[IPC Mock] Set Party Mood to ${moodId}`);
}

// --- Download, Cache & Sync API ---

export async function apiDownloadTrack(url: string, expectedSha256?: string, customFilename?: string): Promise<any> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('download_track', { url, expectedSha256, customFilename });
  }
  console.log(`[IPC Mock] Download Track: ${url}`);
  return {
    track_id: `trk_mock_${Date.now()}`,
    asset_id: `ast_mock_${Date.now()}`,
    title: customFilename || 'Downloaded Audio Anthem',
    artist: 'DHUN Guest Artist',
    duration_ms: 210000,
    bpm: 128.0,
    key: '8B',
    sha256_hash: 'mocksha256hashdownloadedfile',
    verification_status: 'verified',
  };
}

export async function apiCancelDownload(taskId: string): Promise<void> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('cancel_download', { taskId });
  }
  console.log(`[IPC Mock] Cancel Download: ${taskId}`);
}

export async function apiGetDownloadTasks(limit?: number): Promise<any[]> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_download_tasks', { limit });
  }
  return [];
}

export async function apiGetCacheStats(): Promise<{ total_files: number; total_bytes: number; locked_files: number; locked_bytes: number; max_cache_bytes: number }> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_cache_stats');
  }
  return {
    total_files: 10,
    total_bytes: 45000000,
    locked_files: 2,
    locked_bytes: 9000000,
    max_cache_bytes: 5368709120,
  };
}

export async function apiPurgeCache(): Promise<number> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('purge_cache');
  }
  console.log('[IPC Mock] Purge cache requested');
  return 3;
}

export async function apiVerifyAudioAsset(assetId: string): Promise<boolean> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('verify_audio_asset', { assetId });
  }
  return true;
}

export async function apiSyncMetadataCorrection(correction: any): Promise<string> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('sync_metadata_correction', { correction });
  }
  return `sync_mock_${Date.now()}`;
}

export async function apiTriggerSync(): Promise<number> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('trigger_sync');
  }
  return 0;
}

export async function apiGetSyncStatus(): Promise<{ pending_count: number; in_flight_count: number; synced_count: number; failed_count: number }> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('get_sync_status');
  }
  return {
    pending_count: 0,
    in_flight_count: 0,
    synced_count: 5,
    failed_count: 0,
  };
}



