import { create } from 'zustand';
import {
  CloudParty,
  DjLevel,
  GuestRequest,
  MasterTelemetry,
  MoodProfile,
  PartyConfiguration,
  PartyEventMessage,
  PartySongRequest,
  PartySourceReference,
  PartyStatus,
  Track,
} from '../types';
import { CURATED_MOODS } from '../api/ipc';
import { apiClient } from '../api/client';
import { partyWsClient } from '../api/websocket';

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

const DJ_LEVEL_TO_STR: Record<number, string> = {
  1: 'hands_off',
  2: 'balanced',
  3: 'curated',
  4: 'manual',
};

const STR_TO_DJ_LEVEL: Record<string, DjLevel> = {
  hands_off: 1,
  balanced: 2,
  curated: 3,
  manual: 4,
};

interface DhunState {
  currentView: ScreenView;
  setCurrentView: (view: ScreenView) => void;

  // Real Backend & WebSocket Connectivity State
  apiBaseUrl: string;
  backendConnected: boolean;
  backendVersion: string | null;
  backendError: string | null;
  wsConnected: boolean;

  // Active Cloud Party State (Zero Mock Fallbacks)
  cloudParty: CloudParty | null;
  partyConfig: PartyConfiguration | null;
  hostToken: string | null;
  guestUrl: string | null;
  qrSvg: string | null;
  registeredSources: PartySourceReference[];
  songRequests: PartySongRequest[];

  // Derived / UI state
  partyStatus: PartyStatus;
  telemetry: MasterTelemetry | null;
  moodProfiles: MoodProfile[];
  tracks: Track[];
  guestRequests: GuestRequest[];

  masterVolume: number;
  crossfader: number;
  crossfaderCurve: string;
  selectedDeckCount: 2 | 4;
  decks: DeckState[];

  // Actions
  setApiBaseUrl: (url: string) => Promise<void>;
  checkBackendHealth: () => Promise<boolean>;
  refreshAll: () => Promise<void>;
  createCloudParty: (
    partyName: string,
    moodId: string,
    djLevel: string
  ) => Promise<void>;
  startCloudParty: () => Promise<void>;
  endCloudParty: () => Promise<void>;
  clearActiveParty: () => void;
  registerYouTubeSource: (url: string) => Promise<PartySourceReference>;
  refreshPartyData: () => Promise<void>;
  approveSongRequest: (requestId: string) => Promise<void>;
  rejectSongRequest: (requestId: string, reason?: string) => Promise<void>;
  updatePartyConfig: (updates: Partial<PartyConfiguration>) => Promise<void>;
  setDjLevel: (level: number) => Promise<void>;
  setPartyMood: (moodId: string) => Promise<void>;
  setCrossfaderPos: (pos: number) => void;
  updateDeckEq: (deckId: number, low: number, mid: number, high: number) => void;
  updateDeckGain: (deckId: number, gain: number) => void;
  updateDeckFilter: (deckId: number, filter: number) => void;
  updateDeckTempo: (deckId: number, tempo: number) => void;
  setMasterVolume: (vol: number) => void;
  setDeckCount: (count: 2 | 4) => void;

  // Compatibility wrappers for auxiliary screens
  startParty: (
    name: string,
    level: number,
    primaryMood: string,
    secondaryMood?: string
  ) => Promise<void>;
  emergencyFade: () => Promise<void>;
  approveRequest: (requestId: string) => Promise<void>;
  rejectRequest: (requestId: string) => Promise<void>;
  forceNextRequest: (requestId: string) => Promise<void>;
  toggleDeckPlay: (deckId: number) => Promise<void>;
  pollTelemetry: () => Promise<void>;
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

let heartbeatInterval: ReturnType<typeof setInterval> | null = null;

function stopHeartbeat() {
  if (heartbeatInterval) {
    clearInterval(heartbeatInterval);
    heartbeatInterval = null;
  }
}

function derivePartyStatus(
  party: CloudParty | null,
  config: PartyConfiguration | null,
  guestUrl: string | null,
  qrSvg: string | null
): PartyStatus {
  if (!party) {
    return { is_active: false };
  }
  const djLvl = config ? STR_TO_DJ_LEVEL[config.dj_level] || 2 : 2;
  return {
    is_active: party.status === 'live',
    session_id: party.id,
    party_name: party.party_name,
    primary_mood_id: config?.primary_mood_id || 'house_party',
    dj_level: djLvl,
    guest_url: guestUrl || undefined,
    qr_svg: qrSvg || undefined,
  };
}

function mapRequests(requests: PartySongRequest[]): GuestRequest[] {
  return requests.map((r) => ({
    id: r.id,
    session_id: r.party_id,
    guest_id: r.guest_id,
    guest_name: r.guest_name,
    track_id: r.source_id,
    track_title: r.source_title,
    track_artist: r.source_artist,
    submitted_at: r.created_at,
    status: r.status as any,
    priority_order: Math.round(r.priority_score),
    host_note: r.custom_note,
  }));
}

export const useDhunStore = create<DhunState>((set, get) => {
  const handleWsEvent = (event: PartyEventMessage) => {
    const { cloudParty } = get();
    if (!cloudParty || event.party_id !== cloudParty.id) return;

    if (
      event.event_type === 'party.status_changed' ||
      event.event_type === 'party.config_updated' ||
      event.event_type === 'party.source_added' ||
      event.event_type === 'request.created' ||
      event.event_type === 'request.status_changed'
    ) {
      get().refreshPartyData();
    }
  };

  const startHeartbeatForParty = (partyId: string) => {
    stopHeartbeat();
    heartbeatInterval = setInterval(() => {
      const { cloudParty, hostToken } = get();
      if (cloudParty && cloudParty.id === partyId && cloudParty.status === 'live' && hostToken) {
        apiClient.sendHeartbeat(partyId).catch(() => {});
      }
    }, 25000);
  };

  return {
    currentView: 'home',
    setCurrentView: (view) => set({ currentView: view }),

    apiBaseUrl: apiClient.getBaseUrl(),
    backendConnected: false,
    backendVersion: null,
    backendError: null,
    wsConnected: false,

    cloudParty: null,
    partyConfig: null,
    hostToken: apiClient.getHostToken(),
    guestUrl: null,
    qrSvg: null,
    registeredSources: [],
    songRequests: [],

    partyStatus: { is_active: false },
    telemetry: null,
    moodProfiles: CURATED_MOODS,
    tracks: [],
    guestRequests: [],

    masterVolume: 1.0,
    crossfader: 0.0,
    crossfaderCurve: 'bass_swap',
    selectedDeckCount: 2,
    decks: [defaultDeckState(), defaultDeckState(), defaultDeckState(), defaultDeckState()],

    setApiBaseUrl: async (url: string) => {
      apiClient.setBaseUrl(url);
      set({ apiBaseUrl: apiClient.getBaseUrl() });
      await get().refreshAll();
    },

    checkBackendHealth: async () => {
      const res = await apiClient.checkHealth();
      set({
        backendConnected: res.connected,
        backendVersion: res.version || null,
        backendError: res.error || null,
      });
      return res.connected;
    },

    refreshAll: async () => {
      const isHealthy = await get().checkBackendHealth();
      if (!isHealthy) {
        return;
      }

      const savedPartyId = apiClient.getSavedPartyId();
      if (savedPartyId) {
        try {
          const details = await apiClient.getParty(savedPartyId);
          const [sources, requests] = await Promise.all([
            apiClient.listSources(savedPartyId),
            apiClient.listRequests(savedPartyId).catch(() => []),
          ]);

          const derived = derivePartyStatus(
            details.party,
            details.config || null,
            details.guest_url,
            details.qr_svg
          );

          set({
            cloudParty: details.party,
            partyConfig: details.config || null,
            guestUrl: details.guest_url,
            qrSvg: details.qr_svg,
            registeredSources: sources,
            songRequests: requests,
            guestRequests: mapRequests(requests),
            partyStatus: derived,
            selectedDeckCount: derived.dj_level && derived.dj_level >= 3 ? 4 : 2,
          });

          partyWsClient.connect(savedPartyId, handleWsEvent, (connected) => {
            set({ wsConnected: connected });
          });

          if (details.party.status === 'live') {
            startHeartbeatForParty(savedPartyId);
          }
        } catch (err) {
          // Saved party no longer exists on this backend
          apiClient.setSavedPartyId(null);
          set({
            cloudParty: null,
            partyConfig: null,
            guestUrl: null,
            qrSvg: null,
            registeredSources: [],
            songRequests: [],
            guestRequests: [],
            partyStatus: { is_active: false },
          });
        }
      }
    },

    refreshPartyData: async () => {
      const { cloudParty } = get();
      if (!cloudParty) return;
      try {
        const [details, sources, requests] = await Promise.all([
          apiClient.getParty(cloudParty.id),
          apiClient.listSources(cloudParty.id),
          apiClient.listRequests(cloudParty.id).catch(() => []),
        ]);

        const derived = derivePartyStatus(
          details.party,
          details.config || null,
          details.guest_url,
          details.qr_svg
        );

        set({
          cloudParty: details.party,
          partyConfig: details.config || null,
          guestUrl: details.guest_url,
          qrSvg: details.qr_svg,
          registeredSources: sources,
          songRequests: requests,
          guestRequests: mapRequests(requests),
          partyStatus: derived,
        });
      } catch (err) {
        console.error('Failed to refresh party data:', err);
      }
    },

    createCloudParty: async (partyName: string, moodId: string, djLevel: string) => {
      const created = await apiClient.createParty({
        party_name: partyName,
        primary_mood_id: moodId,
        dj_level: djLevel,
        allow_guest_requests: true,
        max_requests_per_guest: 5,
      });

      const partyObj: CloudParty = {
        id: created.party_id,
        party_name: created.party_name,
        status: created.status,
        created_at: created.created_at,
        updated_at: created.created_at,
        last_heartbeat_at: created.created_at,
      };

      const derived = derivePartyStatus(
        partyObj,
        created.config,
        created.guest_url,
        created.qr_svg
      );

      set({
        cloudParty: partyObj,
        partyConfig: created.config,
        hostToken: created.host_token,
        guestUrl: created.guest_url,
        qrSvg: created.qr_svg,
        registeredSources: [],
        songRequests: [],
        guestRequests: [],
        partyStatus: derived,
        backendConnected: true,
        backendError: null,
      });

      partyWsClient.connect(created.party_id, handleWsEvent, (connected) => {
        set({ wsConnected: connected });
      });
    },

    startCloudParty: async () => {
      const { cloudParty } = get();
      if (!cloudParty) throw new Error('No party created yet.');
      const updated = await apiClient.startParty(cloudParty.id);
      const { partyConfig, guestUrl, qrSvg } = get();
      set({
        cloudParty: updated,
        partyStatus: derivePartyStatus(updated, partyConfig, guestUrl, qrSvg),
      });
      startHeartbeatForParty(updated.id);
    },

    endCloudParty: async () => {
      const { cloudParty } = get();
      if (!cloudParty) return;
      const updated = await apiClient.endParty(cloudParty.id);
      stopHeartbeat();
      const { partyConfig, guestUrl, qrSvg } = get();
      set({
        cloudParty: updated,
        partyStatus: derivePartyStatus(updated, partyConfig, guestUrl, qrSvg),
      });
    },

    clearActiveParty: () => {
      stopHeartbeat();
      partyWsClient.disconnect();
      apiClient.setSavedPartyId(null);
      apiClient.setHostToken(null);
      set({
        cloudParty: null,
        partyConfig: null,
        hostToken: null,
        guestUrl: null,
        qrSvg: null,
        registeredSources: [],
        songRequests: [],
        guestRequests: [],
        partyStatus: { is_active: false },
        wsConnected: false,
      });
    },

    registerYouTubeSource: async (url: string) => {
      const { cloudParty } = get();
      if (!cloudParty) {
        throw new Error('Create a party first before registering music sources.');
      }
      const source = await apiClient.registerSource(cloudParty.id, url);
      set((state) => {
        const exists = state.registeredSources.some((s) => s.id === source.id);
        return {
          registeredSources: exists
            ? state.registeredSources
            : [...state.registeredSources, source],
        };
      });
      return source;
    },

    approveSongRequest: async (requestId: string) => {
      const { cloudParty } = get();
      if (!cloudParty) return;
      await apiClient.approveRequest(cloudParty.id, requestId);
      await get().refreshPartyData();
    },

    rejectSongRequest: async (requestId: string, reason?: string) => {
      const { cloudParty } = get();
      if (!cloudParty) return;
      await apiClient.rejectRequest(cloudParty.id, requestId, reason);
      await get().refreshPartyData();
    },

    updatePartyConfig: async (updates: Partial<PartyConfiguration>) => {
      const { cloudParty } = get();
      if (!cloudParty) return;
      const updated = await apiClient.updateConfig(cloudParty.id, updates);
      const { guestUrl, qrSvg } = get();
      set({
        partyConfig: updated,
        partyStatus: derivePartyStatus(cloudParty, updated, guestUrl, qrSvg),
        selectedDeckCount: (STR_TO_DJ_LEVEL[updated.dj_level] || 2) >= 3 ? 4 : 2,
      });
    },

    setDjLevel: async (level: number) => {
      const levelStr = DJ_LEVEL_TO_STR[level] || 'balanced';
      set({ selectedDeckCount: level >= 3 ? 4 : 2 });
      const { cloudParty } = get();
      if (cloudParty) {
        await get().updatePartyConfig({ dj_level: levelStr });
      }
    },

    setPartyMood: async (moodId: string) => {
      const { cloudParty } = get();
      if (cloudParty) {
        await get().updatePartyConfig({ primary_mood_id: moodId });
      } else {
        set((state) => ({
          partyStatus: { ...state.partyStatus, primary_mood_id: moodId },
        }));
      }
    },

    setCrossfaderPos: (pos) => set({ crossfader: pos }),
    updateDeckEq: (deckId, low, mid, high) =>
      set((state) => {
        const decks = [...state.decks];
        decks[deckId] = { ...decks[deckId], eqLow: low, eqMid: mid, eqHigh: high };
        return { decks };
      }),
    updateDeckGain: (deckId, gain) =>
      set((state) => {
        const decks = [...state.decks];
        decks[deckId] = { ...decks[deckId], gain };
        return { decks };
      }),
    updateDeckFilter: (deckId, filter) =>
      set((state) => {
        const decks = [...state.decks];
        decks[deckId] = { ...decks[deckId], filter };
        return { decks };
      }),
    updateDeckTempo: (deckId, tempo) =>
      set((state) => {
        const decks = [...state.decks];
        decks[deckId] = { ...decks[deckId], tempo };
        return { decks };
      }),
    setMasterVolume: (vol) => set({ masterVolume: vol }),
    setDeckCount: (count) => set({ selectedDeckCount: count }),

    // Compatibility wrappers mapping directly to cloud party operations
    startParty: async (name, level, primaryMood) => {
      const levelStr = DJ_LEVEL_TO_STR[level] || 'balanced';
      await get().createCloudParty(name, primaryMood, levelStr);
      await get().startCloudParty();
    },
    emergencyFade: async () => {
      await get().endCloudParty();
    },
    approveRequest: async (requestId) => {
      await get().approveSongRequest(requestId);
    },
    rejectRequest: async (requestId) => {
      await get().rejectSongRequest(requestId);
    },
    forceNextRequest: async (requestId) => {
      await get().approveSongRequest(requestId);
    },
    toggleDeckPlay: async () => {},
    pollTelemetry: async () => {},
  };
});
