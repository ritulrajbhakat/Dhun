import { create } from 'zustand';
import { PartyStatus, MasterTelemetry, MoodProfile, AudioDeviceInfo, Track, GuestRequest, PlayableTrackCandidate } from '../types';
import { webAudioEngine } from '../audio/WebAudioEngine';
import {
  apiGetPartyStatus,
  apiGetTelemetry,
  apiGetMoodProfiles,
  apiGetAudioDevices,
  apiGetTracks,
  apiGetGuestRequests,
  apiStartParty,
  apiEmergencyFade,
  apiUpdateRequest,
  apiForceNext,
  apiDeckControl,
  apiSetCrossfader,
  apiScanLibraryFolder,
} from '../api/ipc';

export type ScreenView = 'home' | 'wizard' | 'live' | 'mixer' | 'library' | 'requests' | 'history' | 'settings' | 'diagnostics';

interface DeckState {
  eqLow: number;
  eqMid: number;
  eqHigh: number;
  gain: number;
  filter: number;
  tempo: number;
  loopBeats: number;
  loopActive: boolean;
  hotCues: (number | null)[];
}

interface DhunState {
  currentView: ScreenView;
  setCurrentView: (view: ScreenView) => void;

  partyStatus: PartyStatus;
  telemetry: MasterTelemetry | null;
  moodProfiles: MoodProfile[];
  audioDevices: AudioDeviceInfo[];
  tracks: Track[];
  guestRequests: GuestRequest[];

  masterVolume: number;
  crossfader: number;
  crossfaderCurve: string;
  selectedDeckCount: 2 | 4;

  decks: DeckState[];

  // Actions
  refreshAll: () => Promise<void>;
  startParty: (name: string, level: number, primaryMood: string, secondaryMood?: string, outputDevice?: string) => Promise<void>;
  emergencyFade: () => Promise<void>;
  approveRequest: (requestId: string) => Promise<void>;
  rejectRequest: (requestId: string) => Promise<void>;
  forceNextRequest: (requestId: string) => Promise<void>;
  setCrossfaderPos: (pos: number) => Promise<void>;
  updateDeckEq: (deckId: number, low: number, mid: number, high: number) => Promise<void>;
  updateDeckGain: (deckId: number, gain: number) => Promise<void>;
  updateDeckFilter: (deckId: number, filter: number) => Promise<void>;
  updateDeckTempo: (deckId: number, tempo: number) => Promise<void>;
  toggleDeckPlay: (deckId: number) => Promise<void>;
  setMasterVolume: (vol: number) => void;
  setDeckCount: (count: 2 | 4) => void;
  loadLocalAudioFile: (deckId: number, file: File) => Promise<void>;
  startBrowserAudio: () => Promise<void>;
  scanFolder: (folderPath: string) => Promise<number>;
}

const defaultDeckState = (): DeckState => ({
  eqLow: 0,
  eqMid: 0,
  eqHigh: 0,
  gain: 1.0,
  filter: 0.0,
  tempo: 1.0,
  loopBeats: 4,
  loopActive: false,
  hotCues: [null, null, null, null, null, null, null, null],
});

export const useDhunStore = create<DhunState>((set, get) => ({
  currentView: 'home',
  setCurrentView: (view) => set({ currentView: view }),

  partyStatus: { is_active: false },
  telemetry: null,
  moodProfiles: [],
  audioDevices: [],
  tracks: [],
  guestRequests: [],

  masterVolume: 1.0,
  crossfader: 0.0,
  crossfaderCurve: 'bass_swap',
  selectedDeckCount: 2,

  decks: [defaultDeckState(), defaultDeckState(), defaultDeckState(), defaultDeckState()],

  refreshAll: async () => {
    try {
      const [partyStatus, moods, devices, tracks, requests] = await Promise.all([
        apiGetPartyStatus(),
        apiGetMoodProfiles(),
        apiGetAudioDevices(),
        apiGetTracks(),
        apiGetGuestRequests(),
      ]);

      set({
        partyStatus,
        moodProfiles: moods,
        audioDevices: devices,
        tracks,
        guestRequests: requests,
        selectedDeckCount: (partyStatus.dj_level && partyStatus.dj_level >= 3) ? 4 : 2,
      });
    } catch (err) {
      console.error('Failed refreshing DHUN state:', err);
    }
  },

  startParty: async (name, level, primaryMood, secondaryMood, outputDevice = 'Default') => {
    const status = await apiStartParty({
      party_name: name,
      dj_level: level,
      primary_mood_id: primaryMood,
      secondary_mood_id: secondaryMood,
      output_device_name: outputDevice,
    });

    // Start WebAudio playback in browser mode
    const bpm = status.now_playing?.analysis.bpm || 128;
    const key = status.now_playing?.analysis.key_name || 'D Major';
    webAudioEngine.startPlayback(bpm, key);

    set({
      partyStatus: status,
      currentView: 'live',
      selectedDeckCount: (level >= 3) ? 4 : 2,
    });
  },

  startBrowserAudio: async () => {
    const bpm = get().partyStatus.now_playing?.analysis.bpm || 128;
    const key = get().partyStatus.now_playing?.analysis.key_name || 'D Major';
    await webAudioEngine.ensureRunning();
    webAudioEngine.startPlayback(bpm, key);
  },

  emergencyFade: async () => {
    webAudioEngine.emergencyFade(300);
    await apiEmergencyFade(300);
    set((state) => ({
      partyStatus: { ...state.partyStatus, is_active: false },
    }));
  },

  approveRequest: async (requestId) => {
    await apiUpdateRequest(requestId, 'approved', 10);
    const requests = await apiGetGuestRequests();
    set({ guestRequests: requests });
  },

  rejectRequest: async (requestId) => {
    await apiUpdateRequest(requestId, 'rejected');
    const requests = await apiGetGuestRequests();
    set({ guestRequests: requests });
  },

  forceNextRequest: async (requestId) => {
    await apiForceNext(requestId);
    const [requests, status] = await Promise.all([apiGetGuestRequests(), apiGetPartyStatus()]);
    set({ guestRequests: requests, partyStatus: status });
    const bpm = status.now_playing?.analysis.bpm || 128;
    const key = status.now_playing?.analysis.key_name || 'D Major';
    webAudioEngine.startPlayback(bpm, key);
  },

  setCrossfaderPos: async (pos) => {
    set({ crossfader: pos });
    webAudioEngine.setCrossfader(pos);
    await apiSetCrossfader(pos, get().crossfaderCurve);
  },

  updateDeckEq: async (deckId, low, mid, high) => {
    set((state) => {
      const decks = [...state.decks];
      decks[deckId] = { ...decks[deckId], eqLow: low, eqMid: mid, eqHigh: high };
      return { decks };
    });
    webAudioEngine.setDeckEq(deckId, low, mid, high);
    await apiDeckControl('eq', deckId, { lowDb: low, midDb: mid, highDb: high });
  },

  updateDeckGain: async (deckId, gain) => {
    set((state) => {
      const decks = [...state.decks];
      decks[deckId] = { ...decks[deckId], gain };
      return { decks };
    });
    webAudioEngine.setDeckGain(deckId, gain);
    await apiDeckControl('gain', deckId, { gain });
  },

  updateDeckFilter: async (deckId, filter) => {
    set((state) => {
      const decks = [...state.decks];
      decks[deckId] = { ...decks[deckId], filter };
      return { decks };
    });
    webAudioEngine.setDeckFilter(deckId, filter);
    await apiDeckControl('filter', deckId, { position: filter });
  },

  updateDeckTempo: async (deckId, tempo) => {
    set((state) => {
      const decks = [...state.decks];
      decks[deckId] = { ...decks[deckId], tempo };
      return { decks };
    });
    await apiDeckControl('tempo', deckId, { rate: tempo });
  },

  toggleDeckPlay: async (deckId) => {
    const isPlaying = get().telemetry?.decks[deckId]?.is_playing ?? false;
    if (isPlaying) {
      webAudioEngine.stopPlayback();
    } else {
      const bpm = get().partyStatus.now_playing?.analysis.bpm || 128;
      const key = get().partyStatus.now_playing?.analysis.key_name || 'D Major';
      webAudioEngine.startPlayback(bpm, key);
    }
    await apiDeckControl(isPlaying ? 'pause' : 'play', deckId);
  },

  setMasterVolume: (vol) => {
    set({ masterVolume: vol });
    webAudioEngine.setMasterVolume(vol);
  },

  setDeckCount: (count) => set({ selectedDeckCount: count }),

  loadLocalAudioFile: async (deckId: number, file: File) => {
    await webAudioEngine.loadUserAudioFile(deckId, file);
  },

  scanFolder: async (folderPath: string) => {
    const count = await apiScanLibraryFolder(folderPath);
    const tracks = await apiGetTracks();
    set({ tracks });
    return count;
  },
}));

