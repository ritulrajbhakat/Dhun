import { create } from 'zustand';
import {
  DjLevel,
  LocalPartyRoom,
  LocalRoomEvent,
  LocalRoomSubmission,
  LocalTempCacheStats,
  MoodProfile,
  SyncStatusOverview,
  UnifiedSongSubmissionResponse,
} from '../types';
import { CURATED_MOODS } from '../api/ipc';
import { localHelperClient } from '../api/client';
import { localRoomWsClient } from '../api/websocket';
import { webAudioEngine } from '../audio/WebAudioEngine';

export type ScreenView =
  | 'home'
  | 'wizard'
  | 'live'
  | 'mixer'
  | 'library'
  | 'requests'
  | 'history'
  | 'settings'
  | 'diagnostics';

interface DeckControlState {
  eqLow: number;
  eqMid: number;
  eqHigh: number;
  gain: number;
  filter: number;
  tempo: number;
}

interface DhunState {
  currentView: ScreenView;
  setCurrentView: (view: ScreenView) => void;

  // Local DHUN Helper Connection & LAN State
  helperUrl: string;
  helperConnected: boolean;
  helperVersion: string | null;
  helperError: string | null;
  wsConnected: boolean;
  lanReachable: boolean;
  detectedLanIp: string | null;
  lanNotice: string | null;

  // Private Local Party Room State (Zero Cloud Parties, Zero Approval Queue)
  activeRoom: LocalPartyRoom | null;
  djLevelName: string;
  exposedDecks: 2 | 4;
  internalDeckCapacity: 4;
  autoDjEnabled: boolean;
  guestUrl: string | null;
  qrSvg: string | null;
  submissions: LocalRoomSubmission[];
  cacheStats: LocalTempCacheStats;
  syncOverview: SyncStatusOverview;

  // All 14 Curated Moods & 4-Deck Web Audio Controls
  moodProfiles: MoodProfile[];
  selectedMoodId: string;
  selectedDjLevel: DjLevel;
  masterVolume: number;
  crossfader: number;
  decks: DeckControlState[];

  // Actions
  setHelperUrl: (url: string) => Promise<void>;
  checkHelperHealth: () => Promise<boolean>;
  refreshAll: () => Promise<void>;
  refreshRoomDashboard: () => Promise<void>;
  createLocalRoom: (
    partyName: string,
    djLevel: DjLevel,
    primaryMoodId: string,
    secondaryMoodId?: string
  ) => Promise<void>;
  startLocalRoom: () => Promise<void>;
  closeLocalRoom: () => Promise<{ invalidated_guests: number; purged_cache_files: number } | null>;
  clearLocalRoomSession: () => void;
  submitSong: (url: string) => Promise<UnifiedSongSubmissionResponse>;
  evaluateDjPool: () => Promise<void>;
  setDjLevel: (level: DjLevel) => Promise<void>;
  setPartyMood: (moodId: string) => Promise<void>;

  // 4-Deck Web Audio Controls (Isolated from network/DB callbacks)
  setCrossfaderPos: (pos: number) => void;
  updateDeckEq: (deckId: number, low: number, mid: number, high: number) => void;
  updateDeckGain: (deckId: number, gain: number) => void;
  updateDeckFilter: (deckId: number, filter: number) => void;
  updateDeckTempo: (deckId: number, tempo: number) => void;
  setMasterVolume: (vol: number) => void;
  emergencyFade: () => void;
}

const defaultDeck = (): DeckControlState => ({
  eqLow: 0,
  eqMid: 0,
  eqHigh: 0,
  gain: 1.0,
  filter: 0.0,
  tempo: 1.0,
});

const DJ_LEVEL_NAMES: Record<DjLevel, string> = {
  1: 'Professional 2-Deck Mixer',
  2: 'Smart Mixing',
  3: 'Advanced Performance',
  4: 'Creative DJ Tools',
};

export const useDhunStore = create<DhunState>((set, get) => {
  const handleLocalRoomEvent = (evt: LocalRoomEvent) => {
    const { activeRoom } = get();
    if (!activeRoom || evt.room_id !== activeRoom.room_id) return;
    if (
      evt.event_type === 'room.status_changed' ||
      evt.event_type === 'room.config_updated' ||
      evt.event_type === 'room.closed' ||
      evt.event_type === 'dj.submission_received' ||
      evt.event_type === 'dj.evaluated'
    ) {
      get().refreshRoomDashboard();
    }
  };

  return {
    currentView: 'home',
    setCurrentView: (view) => set({ currentView: view }),

    helperUrl: localHelperClient.getBaseUrl(),
    helperConnected: false,
    helperVersion: null,
    helperError: null,
    wsConnected: false,
    lanReachable: false,
    detectedLanIp: null,
    lanNotice: null,

    activeRoom: null,
    djLevelName: DJ_LEVEL_NAMES[2],
    exposedDecks: 2,
    internalDeckCapacity: 4,
    autoDjEnabled: true,
    guestUrl: null,
    qrSvg: null,
    submissions: [],
    cacheStats: { track_count: 0, max_tracks: 15, total_bytes: 0, locked_tracks: 0 },
    syncOverview: { pending_count: 0, in_flight_count: 0, synced_count: 0, failed_count: 0 },

    moodProfiles: CURATED_MOODS,
    selectedMoodId: 'house_party',
    selectedDjLevel: 2,
    masterVolume: 0.85,
    crossfader: -1.0,
    decks: [defaultDeck(), defaultDeck(), defaultDeck(), defaultDeck()],

    setHelperUrl: async (url: string) => {
      localHelperClient.setBaseUrl(url);
      set({ helperUrl: localHelperClient.getBaseUrl() });
      await get().refreshAll();
    },

    checkHelperHealth: async () => {
      const res = await localHelperClient.checkHealth();
      if (res.connected && res.health) {
        set({
          helperConnected: true,
          helperVersion: res.health.version,
          helperError: null,
          lanReachable: res.health.lan_reachable,
          detectedLanIp: res.health.detected_lan_ip,
          cacheStats: res.health.cache_stats,
          syncOverview: res.health.sync_overview,
        });
        return true;
      }
      set({
        helperConnected: false,
        helperVersion: null,
        helperError: res.error || 'Local DHUN helper unreachable',
        lanReachable: false,
        detectedLanIp: null,
      });
      return false;
    },

    refreshAll: async () => {
      const ok = await get().checkHelperHealth();
      if (!ok) return;

      const savedRoomId = localHelperClient.getSavedRoomId();
      if (!savedRoomId || !localHelperClient.getHostToken()) return;

      try {
        const dash = await localHelperClient.getRoomDashboard(savedRoomId);
        const savedQr = localHelperClient.getSavedGuestQr();
        set({
          activeRoom: dash.room,
          djLevelName: dash.dj_level_name,
          exposedDecks: dash.exposed_decks >= 4 ? 4 : 2,
          autoDjEnabled: dash.auto_dj_enabled,
          lanReachable: dash.lan_reachable,
          detectedLanIp: dash.detected_lan_ip,
          submissions: dash.submissions,
          cacheStats: dash.cache_stats,
          syncOverview: dash.sync_overview,
          selectedMoodId: dash.room.primary_mood_id,
          selectedDjLevel: dash.room.dj_level,
          guestUrl: dash.room.status === 'closed' ? null : savedQr.guestUrl,
          qrSvg: dash.room.status === 'closed' ? null : savedQr.qrSvg,
        });

        if (dash.room.status !== 'closed') {
          localRoomWsClient.connect(savedRoomId, handleLocalRoomEvent, (connected) => {
            set({ wsConnected: connected });
          });
        }
      } catch {
        localHelperClient.setSavedRoomSession(null);
        localHelperClient.setHostToken(null);
        set({
          activeRoom: null,
          guestUrl: null,
          qrSvg: null,
          submissions: [],
        });
      }
    },

    refreshRoomDashboard: async () => {
      const { activeRoom } = get();
      if (!activeRoom) return;
      try {
        const dash = await localHelperClient.getRoomDashboard(activeRoom.room_id);
        set({
          activeRoom: dash.room,
          djLevelName: dash.dj_level_name,
          exposedDecks: dash.exposed_decks >= 4 ? 4 : 2,
          autoDjEnabled: dash.auto_dj_enabled,
          lanReachable: dash.lan_reachable,
          detectedLanIp: dash.detected_lan_ip,
          submissions: dash.submissions,
          cacheStats: dash.cache_stats,
          syncOverview: dash.sync_overview,
          selectedMoodId: dash.room.primary_mood_id,
          selectedDjLevel: dash.room.dj_level,
        });
      } catch (err) {
        console.error('Failed refreshing local party room dashboard:', err);
      }
    },

    createLocalRoom: async (partyName, djLevel, primaryMoodId, secondaryMoodId) => {
      const res = await localHelperClient.createLocalRoom({
        party_name: partyName,
        dj_level: djLevel,
        primary_mood_id: primaryMoodId,
        secondary_mood_id: secondaryMoodId,
      });

      set({
        activeRoom: res.room,
        selectedDjLevel: res.room.dj_level,
        selectedMoodId: res.room.primary_mood_id,
        djLevelName: DJ_LEVEL_NAMES[res.room.dj_level] || DJ_LEVEL_NAMES[2],
        exposedDecks: res.room.dj_level >= 3 ? 4 : 2,
        lanReachable: res.lan_reachable,
        detectedLanIp: res.detected_lan_ip,
        guestUrl: res.guest_url,
        qrSvg: res.qr_svg,
        lanNotice: res.lan_notice,
        submissions: [],
        helperConnected: true,
        helperError: null,
      });

      localRoomWsClient.connect(res.room.room_id, handleLocalRoomEvent, (connected) => {
        set({ wsConnected: connected });
      });
    },

    startLocalRoom: async () => {
      const { activeRoom } = get();
      if (!activeRoom) throw new Error('Create a local party room first.');
      await webAudioEngine.ensureRunning();
      const updated = await localHelperClient.startLocalRoom(activeRoom.room_id);
      set({ activeRoom: updated });
      await get().refreshRoomDashboard();
    },

    closeLocalRoom: async () => {
      const { activeRoom } = get();
      if (!activeRoom) return null;
      webAudioEngine.emergencyFade(250);
      const res = await localHelperClient.closeLocalRoom(activeRoom.room_id);
      localRoomWsClient.disconnect();
      set((state) => ({
        activeRoom: state.activeRoom
          ? { ...state.activeRoom, status: 'closed', closed_at: res.closed_at }
          : null,
        guestUrl: null,
        qrSvg: null,
        wsConnected: false,
      }));
      await get().checkHelperHealth();
      return {
        invalidated_guests: res.invalidated_guests,
        purged_cache_files: res.purged_cache_files,
      };
    },

    clearLocalRoomSession: () => {
      localRoomWsClient.disconnect();
      localHelperClient.setSavedRoomSession(null);
      localHelperClient.setHostToken(null);
      set({
        activeRoom: null,
        guestUrl: null,
        qrSvg: null,
        lanNotice: null,
        submissions: [],
        wsConnected: false,
      });
    },

    submitSong: async (url: string) => {
      const { activeRoom } = get();
      if (!activeRoom) {
        throw new Error('Create a local party room before submitting a song.');
      }
      const res = await localHelperClient.submitSong(activeRoom.room_id, url);
      await get().refreshRoomDashboard();
      return res;
    },

    evaluateDjPool: async () => {
      const { activeRoom } = get();
      if (!activeRoom) return;
      await localHelperClient.evaluateDjPool(activeRoom.room_id);
      await get().refreshRoomDashboard();
    },

    setDjLevel: async (level: DjLevel) => {
      set({
        selectedDjLevel: level,
        djLevelName: DJ_LEVEL_NAMES[level],
        exposedDecks: level >= 3 ? 4 : 2,
      });
      const { activeRoom } = get();
      if (activeRoom && activeRoom.status !== 'closed') {
        const updated = await localHelperClient.updateRoomConfig(activeRoom.room_id, {
          dj_level: level,
        });
        set({ activeRoom: updated });
      }
    },

    setPartyMood: async (moodId: string) => {
      set({ selectedMoodId: moodId });
      const { activeRoom } = get();
      if (activeRoom && activeRoom.status !== 'closed') {
        const updated = await localHelperClient.updateRoomConfig(activeRoom.room_id, {
          primary_mood_id: moodId,
        });
        set({ activeRoom: updated });
        await localHelperClient.evaluateDjPool(activeRoom.room_id).catch(() => {});
        await get().refreshRoomDashboard();
      }
    },

    setCrossfaderPos: (pos) => {
      set({ crossfader: pos });
      webAudioEngine.setCrossfader(pos);
    },

    updateDeckEq: (deckId, low, mid, high) => {
      set((state) => {
        const decks = [...state.decks];
        decks[deckId] = { ...decks[deckId], eqLow: low, eqMid: mid, eqHigh: high };
        return { decks };
      });
      webAudioEngine.setDeckEq(deckId, low, mid, high);
    },

    updateDeckGain: (deckId, gain) => {
      set((state) => {
        const decks = [...state.decks];
        decks[deckId] = { ...decks[deckId], gain };
        return { decks };
      });
      webAudioEngine.setDeckGain(deckId, gain);
    },

    updateDeckFilter: (deckId, filter) => {
      set((state) => {
        const decks = [...state.decks];
        decks[deckId] = { ...decks[deckId], filter };
        return { decks };
      });
      webAudioEngine.setDeckFilter(deckId, filter);
    },

    updateDeckTempo: (deckId, tempo) => {
      set((state) => {
        const decks = [...state.decks];
        decks[deckId] = { ...decks[deckId], tempo };
        return { decks };
      });
    },

    setMasterVolume: (vol) => {
      set({ masterVolume: vol });
      webAudioEngine.setMasterVolume(vol);
    },

    emergencyFade: () => {
      webAudioEngine.emergencyFade(300);
    },
  };
});
