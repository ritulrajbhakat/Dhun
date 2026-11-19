import { create } from 'zustand';
import {
  DjLevel,
  DjRecommendation,
  DjSkillProfile,
  DjTechniqueExecutionRecord,
  LocalPartyRoom,
  LocalRoomEvent,
  LocalRoomSubmission,
  LocalTempCacheStats,
  MoodProfile,
  PlayableCandidatePointer,
  SyncStatusOverview,
  UnifiedSongSubmissionResponse,
} from '../types';
import { CURATED_MOODS } from '../api/ipc';
import { localHelperClient } from '../api/client';
import { localRoomWsClient } from '../api/websocket';
import {
  TransitionStyle,
  WebAudioDeckStatus,
  webAudioEngine,
} from '../audio/WebAudioEngine';

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

export type AutoDjPlaybackState =
  | 'standby'
  | 'awaiting_playable_audio'
  | 'preparing'
  | 'playing'
  | 'transitioning'
  | 'decode_error_recovered';

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
  djSkillProfile: DjSkillProfile | null;
  exposedDecks: 2 | 4;
  internalDeckCapacity: 4;
  autoDjEnabled: boolean;
  guestUrl: string | null;
  qrSvg: string | null;
  submissions: LocalRoomSubmission[];
  recommendations: DjRecommendation[];
  pendingIntakeJobs: number;
  techniqueLog: DjTechniqueExecutionRecord[];
  nextPlayableCandidate: PlayableCandidatePointer | null;
  transitionStyle: TransitionStyle;
  cacheStats: LocalTempCacheStats;
  syncOverview: SyncStatusOverview;

  // Real Web Audio 4-Deck Playback State (Driven strictly by actual AudioContext & decoded buffers)
  deckSnapshots: WebAudioDeckStatus[];
  autoDjPlaybackState: AutoDjPlaybackState;
  autoDjStatusNote: string | null;

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
  closeLocalRoom: () => Promise<{
    invalidated_guests: number;
    purged_cache_files: number;
    deferred_locked_files: number;
  } | null>;
  clearLocalRoomSession: () => void;
  submitSong: (url: string) => Promise<UnifiedSongSubmissionResponse>;
  evaluateDjPool: () => Promise<void>;
  driveAutoDjLoop: () => Promise<void>;
  setDjLevel: (level: DjLevel) => Promise<void>;
  setPartyMood: (moodId: string) => Promise<void>;

  // 4-Deck Web Audio Controls
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

let httpFallbackTimer: ReturnType<typeof setInterval> | null = null;
let isPreparingDeck = false;

function stopHttpFallbackPolling() {
  if (httpFallbackTimer) {
    clearInterval(httpFallbackTimer);
    httpFallbackTimer = null;
  }
}

export const useDhunStore = create<DhunState>((set, get) => {
  const ensureFallbackPolling = () => {
    if (httpFallbackTimer) return;
    httpFallbackTimer = setInterval(() => {
      const { activeRoom, wsConnected, helperConnected, pendingIntakeJobs } = get();
      if (!activeRoom || activeRoom.status === 'closed') {
        stopHttpFallbackPolling();
        return;
      }
      // Poll REST when WebSocket is unavailable (e.g. Safari) OR while background intake jobs are draining
      if (helperConnected && (!wsConnected || pendingIntakeJobs > 0)) {
        void get().refreshRoomDashboard().then(() => {
          if (get().activeRoom?.status === 'live') {
            void get().driveAutoDjLoop();
          }
        });
      }
    }, 3000);
  };

  const handleLocalRoomEvent = (evt: LocalRoomEvent) => {
    const { activeRoom } = get();
    if (!activeRoom || evt.room_id !== activeRoom.room_id) return;
    if (
      evt.event_type === 'room.status_changed' ||
      evt.event_type === 'room.config_updated' ||
      evt.event_type === 'room.closed' ||
      evt.event_type === 'dj.submission_received' ||
      evt.event_type === 'dj.submission_updated' ||
      evt.event_type === 'dj.submission_ready' ||
      evt.event_type === 'dj.evaluated' ||
      evt.event_type === 'dj.playback_updated'
    ) {
      void get().refreshRoomDashboard().then(() => {
        if (get().activeRoom?.status === 'live') {
          void get().driveAutoDjLoop();
        }
      });
    }
  };

  // Wire real Web Audio engine clock callbacks to the Automated DJ controller
  webAudioEngine.setCallbacks({
    onSnapshot: (deckSnapshots) => {
      set({ deckSnapshots });
    },
    onPreloadWindow: () => {
      // Preload next playable candidate onto an idle deck before current track finishes
      void get().driveAutoDjLoop();
    },
    onTechniqueExecuted: (execution) => {
      set((state) => ({
        techniqueLog: [execution, ...state.techniqueLog].slice(0, 25),
      }));
    },
    onTransitionStarted: (_fromDeck, _fromSubId, toDeckId, toSubId, execution) => {
      const { activeRoom } = get();
      set({
        autoDjPlaybackState: 'transitioning',
        autoDjStatusNote: `L${execution.djLevel} ${execution.techniqueLabel} → Deck ${String.fromCharCode(65 + toDeckId)} (${execution.incomingTitle})`,
      });
      if (activeRoom) {
        void localHelperClient
          .recordPlaybackEvent(activeRoom.room_id, {
            submission_id: toSubId,
            event: 'playing',
            deck_id: toDeckId,
          })
          .then(() => get().refreshRoomDashboard())
          .catch(() => {});
      }
    },
    onTrackEnded: (deckId, finishedSubId) => {
      const { activeRoom } = get();
      if (!activeRoom) return;
      void localHelperClient
        .recordPlaybackEvent(activeRoom.room_id, {
          submission_id: finishedSubId,
          event: 'played',
          deck_id: deckId,
        })
        .then(async (res) => {
          set({
            nextPlayableCandidate: res.next_playable_candidate,
            transitionStyle: (res.transition_style as TransitionStyle) || 'bass_swap',
          });
          await get().refreshRoomDashboard();

          // If another deck is already playing (via overlap transition), keep state 'playing'
          const activeDeck = webAudioEngine.getActivePlayingDeck();
          if (activeDeck) {
            set({ autoDjPlaybackState: 'playing', autoDjStatusNote: `Playing ${activeDeck.title}` });
            return;
          }

          // Otherwise start any prepared deck immediately or fetch next playable candidate
          const prepared = webAudioEngine.getPreparedIdleDeck();
          if (prepared && prepared.submissionId) {
            const started = webAudioEngine.startDeck(prepared.deckId, 1.0);
            if (started) {
              set({
                autoDjPlaybackState: 'playing',
                autoDjStatusNote: `Auto-advanced to ${prepared.title}`,
              });
              await localHelperClient.recordPlaybackEvent(activeRoom.room_id, {
                submission_id: prepared.submissionId,
                event: 'playing',
                deck_id: prepared.deckId,
              });
              return;
            }
          }

          await get().driveAutoDjLoop();
        })
        .catch(() => {});
    },
  });

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
    djSkillProfile: null,
    exposedDecks: 2,
    internalDeckCapacity: 4,
    autoDjEnabled: true,
    guestUrl: null,
    qrSvg: null,
    submissions: [],
    recommendations: [],
    pendingIntakeJobs: 0,
    techniqueLog: [],
    nextPlayableCandidate: null,
    transitionStyle: 'timed_bass_swap',
    cacheStats: { track_count: 0, max_tracks: 15, total_bytes: 0, locked_tracks: 0 },
    syncOverview: { pending_count: 0, in_flight_count: 0, synced_count: 0, failed_count: 0 },

    deckSnapshots: [],
    autoDjPlaybackState: 'standby',
    autoDjStatusNote: null,

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
        const style = (dash.transition_style as TransitionStyle) || 'timed_bass_swap';
        webAudioEngine.setDjLevel(dash.room.dj_level);
        webAudioEngine.setMoodId(dash.room.primary_mood_id);
        webAudioEngine.setTransitionStyle(style);

        set({
          activeRoom: dash.room,
          djLevelName: dash.dj_level_name,
          djSkillProfile: dash.dj_skill_profile || null,
          exposedDecks: dash.exposed_decks >= 4 ? 4 : 2,
          autoDjEnabled: dash.auto_dj_enabled,
          lanReachable: dash.lan_reachable,
          detectedLanIp: dash.detected_lan_ip,
          submissions: dash.submissions,
          recommendations: dash.recommendations || [],
          pendingIntakeJobs: dash.pending_intake_jobs ?? 0,
          nextPlayableCandidate: dash.next_playable_candidate,
          transitionStyle: style,
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
          ensureFallbackPolling();
          if (dash.room.status === 'live') {
            void get().driveAutoDjLoop();
          }
        }
      } catch {
        localHelperClient.setSavedRoomSession(null);
        localHelperClient.setHostToken(null);
        set({
          activeRoom: null,
          guestUrl: null,
          qrSvg: null,
          submissions: [],
          nextPlayableCandidate: null,
        });
      }
    },

    refreshRoomDashboard: async () => {
      const { activeRoom } = get();
      if (!activeRoom) return;
      try {
        const dash = await localHelperClient.getRoomDashboard(activeRoom.room_id);
        const style = (dash.transition_style as TransitionStyle) || 'timed_bass_swap';
        webAudioEngine.setDjLevel(dash.room.dj_level);
        webAudioEngine.setMoodId(dash.room.primary_mood_id);
        webAudioEngine.setTransitionStyle(style);
        set({
          activeRoom: dash.room,
          djLevelName: dash.dj_level_name,
          djSkillProfile: dash.dj_skill_profile || null,
          exposedDecks: dash.exposed_decks >= 4 ? 4 : 2,
          autoDjEnabled: dash.auto_dj_enabled,
          lanReachable: dash.lan_reachable,
          detectedLanIp: dash.detected_lan_ip,
          submissions: dash.submissions,
          recommendations: dash.recommendations || [],
          pendingIntakeJobs: dash.pending_intake_jobs ?? 0,
          nextPlayableCandidate: dash.next_playable_candidate,
          transitionStyle: style,
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
        nextPlayableCandidate: null,
        autoDjPlaybackState: 'standby',
        autoDjStatusNote: null,
        helperConnected: true,
        helperError: null,
      });

      localRoomWsClient.connect(res.room.room_id, handleLocalRoomEvent, (connected) => {
        set({ wsConnected: connected });
      });
      ensureFallbackPolling();
    },

    startLocalRoom: async () => {
      const { activeRoom } = get();
      if (!activeRoom) throw new Error('Create a local party room first.');
      await webAudioEngine.ensureRunning();
      const updated = await localHelperClient.startLocalRoom(activeRoom.room_id);
      set({ activeRoom: updated });
      await get().refreshRoomDashboard();
      await get().driveAutoDjLoop();
    },

    closeLocalRoom: async () => {
      const { activeRoom } = get();
      if (!activeRoom) return null;
      stopHttpFallbackPolling();
      // Gracefully fade out & stop all Web Audio decks BEFORE purging room cache files
      await webAudioEngine.emergencyFadeAndStop(250);
      const res = await localHelperClient.closeLocalRoom(activeRoom.room_id, true);
      localRoomWsClient.disconnect();
      set((state) => ({
        activeRoom: state.activeRoom
          ? { ...state.activeRoom, status: 'closed', closed_at: res.closed_at }
          : null,
        guestUrl: null,
        qrSvg: null,
        wsConnected: false,
        nextPlayableCandidate: null,
        autoDjPlaybackState: 'standby',
        autoDjStatusNote: null,
        deckSnapshots: webAudioEngine.getDeckSnapshots(),
      }));
      await get().checkHelperHealth();
      return {
        invalidated_guests: res.invalidated_guests,
        purged_cache_files: res.purged_cache_files,
        deferred_locked_files: res.deferred_locked_files,
      };
    },

    clearLocalRoomSession: () => {
      stopHttpFallbackPolling();
      localRoomWsClient.disconnect();
      localHelperClient.setSavedRoomSession(null);
      localHelperClient.setHostToken(null);
      set({
        activeRoom: null,
        guestUrl: null,
        qrSvg: null,
        lanNotice: null,
        submissions: [],
        nextPlayableCandidate: null,
        autoDjPlaybackState: 'standby',
        autoDjStatusNote: null,
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
      if (get().activeRoom?.status === 'live') {
        await get().driveAutoDjLoop();
      }
      return res;
    },

    evaluateDjPool: async () => {
      const { activeRoom } = get();
      if (!activeRoom) return;
      const evalRes = await localHelperClient.evaluateDjPool(activeRoom.room_id);
      const style = (evalRes.transition_style as TransitionStyle) || 'bass_swap';
      webAudioEngine.setTransitionStyle(style);
      set({
        nextPlayableCandidate: evalRes.next_playable_candidate,
        transitionStyle: style,
      });
      await get().refreshRoomDashboard();
      if (get().activeRoom?.status === 'live') {
        await get().driveAutoDjLoop();
      }
    },

    /**
     * Core Automated DJ Playback Controller:
     * Fetches authorized cached audio from `GET /api/local/rooms/:room_id/cache/:cache_id/stream`,
     * prepares an idle Web Audio deck via `prepareAuthorizedDeckBuffer()`, starts playback if idle
     * or stages it for automatic transition without interrupting the currently playing track.
     */
    driveAutoDjLoop: async () => {
      if (isPreparingDeck) return;
      const { activeRoom, nextPlayableCandidate, exposedDecks, submissions } = get();
      if (!activeRoom || activeRoom.status !== 'live') return;

      const playingDeck = webAudioEngine.getActivePlayingDeck();
      const preparedDecks = webAudioEngine.getPreparedIdleDecks();
      const preparedDeck = preparedDecks[0] || null;
      const loadedIds = new Set(
        webAudioEngine
          .getDeckSnapshots()
          .map((d) => d.submissionId)
          .filter((id): id is string => Boolean(id))
      );

      // Level 3–4: if we already have a playing deck + 1 prepared deck, optionally stage a 3rd groove-layer deck
      if (playingDeck && preparedDeck) {
        if (exposedDecks >= 4 && preparedDecks.length < 2) {
          const layerSub = submissions.find(
            (s) =>
              s.acquisition_status === 'cached_authorized' &&
              Boolean(s.cache_id) &&
              s.dj_status === 'queued_for_dj' &&
              !loadedIds.has(s.id)
          );
          const layerDeckId = layerSub ? webAudioEngine.findIdleDeck(exposedDecks) : null;
          if (layerSub && layerSub.cache_id && layerDeckId !== null) {
            isPreparingDeck = true;
            try {
              const layerBytes = await localHelperClient.fetchAuthorizedCachedAudio(
                activeRoom.room_id,
                layerSub.cache_id
              );
              await webAudioEngine.prepareAuthorizedDeckBuffer(layerDeckId, layerBytes, {
                submissionId: layerSub.id,
                cacheId: layerSub.cache_id,
                title: layerSub.title,
                artist: layerSub.artist,
                bpm: layerSub.bpm,
                camelotKey: layerSub.camelot_key,
                overallEnergy: layerSub.overall_energy,
              });
            } catch {
              // Optional 3rd-deck layer preload failure must never disrupt playing audio
            } finally {
              isPreparingDeck = false;
            }
          }
        }
        set({ autoDjPlaybackState: 'playing' });
        return;
      }

      let candidate = nextPlayableCandidate;
      if (!candidate || candidate.submission_id === playingDeck?.submissionId) {
        try {
          const fresh = await localHelperClient.evaluateDjPool(activeRoom.room_id);
          if (fresh.next_playable_candidate) {
            candidate = fresh.next_playable_candidate;
            set({ nextPlayableCandidate: candidate });
          }
        } catch {}
      }

      // No playable cached candidate exists right now
      if (!candidate) {
        if (!playingDeck) {
          const hasRestricted = submissions.some((s) => s.acquisition_status === 'restricted_by_provider');
          set({
            autoDjPlaybackState: 'awaiting_playable_audio',
            autoDjStatusNote: hasRestricted
              ? 'Queued YouTube links are metadata-only (restricted_by_provider). Playable playback requires an authorised audio source.'
              : 'Waiting for a playable authorised cached track to enter the DJ pool.',
          });
        }
        return;
      }

      // Avoid re-loading a candidate already active or preloaded on a deck
      if (
        playingDeck?.submissionId === candidate.submission_id ||
        preparedDeck?.submissionId === candidate.submission_id
      ) {
        return;
      }

      const idleDeckId = webAudioEngine.findIdleDeck(exposedDecks);
      if (idleDeckId === null) return;

      isPreparingDeck = true;
      if (!playingDeck) {
        set({
          autoDjPlaybackState: 'preparing',
          autoDjStatusNote: `Preparing "${candidate.title}" on Deck ${String.fromCharCode(65 + idleDeckId)}...`,
        });
      }

      try {
        const audioBytes = await localHelperClient.fetchAuthorizedCachedAudio(
          activeRoom.room_id,
          candidate.cache_id
        );

        const ok = await webAudioEngine.prepareAuthorizedDeckBuffer(idleDeckId, audioBytes, {
          submissionId: candidate.submission_id,
          cacheId: candidate.cache_id,
          title: candidate.title,
          artist: candidate.artist,
          bpm: candidate.bpm,
          camelotKey: candidate.camelot_key,
          overallEnergy: candidate.overall_energy,
          genreTags: candidate.genre_tags,
          sections: candidate.sections,
          timingProfile: candidate.timing_profile ?? null,
        });

        if (!ok) {
          return;
        }

        if (!webAudioEngine.getActivePlayingDeck()) {
          const started = webAudioEngine.startDeck(idleDeckId, 1.0);
          if (started) {
            set({
              autoDjPlaybackState: 'playing',
              autoDjStatusNote: `Playing "${candidate.title}" on Deck ${String.fromCharCode(65 + idleDeckId)}`,
            });
            const evRes = await localHelperClient.recordPlaybackEvent(activeRoom.room_id, {
              submission_id: candidate.submission_id,
              event: 'playing',
              deck_id: idleDeckId,
            });
            set({ nextPlayableCandidate: evRes.next_playable_candidate });
            // Immediately kick off background preload on the next idle deck!
            void get().driveAutoDjLoop();
          }
        } else {
          // Track preloaded onto idle deck: immediately plan musical transition ahead of time!
          if (playingDeck) {
            webAudioEngine.schedulePlannedTransition(playingDeck.deckId, idleDeckId);
          }
          await localHelperClient.recordPlaybackEvent(activeRoom.room_id, {
            submission_id: candidate.submission_id,
            event: 'prepared',
            deck_id: idleDeckId,
          });
          set({
            autoDjPlaybackState: 'playing',
            autoDjStatusNote: `Preloaded "${candidate.title}" on Deck ${String.fromCharCode(65 + idleDeckId)} for transition`,
          });
        }
      } catch (err: any) {
        // Failed preload/decode MUST NOT interrupt currently playing audio! Cancel any stale plan cleanly.
        const errMsg = err?.message || 'Audio decode or cache stream failure';
        webAudioEngine.cancelPendingTransition(`Incoming track preload failed: ${errMsg}`);
        await localHelperClient
          .recordPlaybackEvent(activeRoom.room_id, {
            submission_id: candidate.submission_id,
            event: 'decode_failed',
            deck_id: idleDeckId,
            error_detail: errMsg,
          })
          .then((res) => {
            set({
              nextPlayableCandidate: res.next_playable_candidate,
              autoDjPlaybackState: webAudioEngine.getActivePlayingDeck()
                ? 'playing'
                : 'decode_error_recovered',
              autoDjStatusNote: `Skipped unplayable/corrupt cache entry (${errMsg}) without interrupting playback.`,
            });
          })
          .catch(() => {});
      } finally {
        isPreparingDeck = false;
      }
    },

    setDjLevel: async (level: DjLevel) => {
      webAudioEngine.setDjLevel(level);
      webAudioEngine.cancelPendingTransition('DJ level changed — replanning for new skill level');
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
        await get().evaluateDjPool().catch(() => {});
      }
    },

    setPartyMood: async (moodId: string) => {
      webAudioEngine.setMoodId(moodId);
      const moodObj = get().moodProfiles.find((m) => m.id === moodId);
      if (moodObj) {
        webAudioEngine.setTargetEnergy(moodObj.target_energy);
      }
      webAudioEngine.cancelPendingTransition('Party mood updated — replanning transition for new mood');
      set({ selectedMoodId: moodId });
      const { activeRoom } = get();
      if (activeRoom && activeRoom.status !== 'closed') {
        const updated = await localHelperClient.updateRoomConfig(activeRoom.room_id, {
          primary_mood_id: moodId,
        });
        set({ activeRoom: updated });
        await get().evaluateDjPool().catch(() => {});
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
      webAudioEngine.setDeckTempo(deckId, tempo);
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
