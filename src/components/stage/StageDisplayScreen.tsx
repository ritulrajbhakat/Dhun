import React, { useState, useEffect, useRef } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { StageVisualFxCanvas, FX_NAMES } from './StageVisualFxCanvas';
import { AnimatedDjCharacter } from './AnimatedDjCharacter';
import { webAudioEngine } from '../../audio/WebAudioEngine';
import { DjLevel } from '../../types';
import {
  Play,
  Square,
  Sliders,
  Music,
  Disc3,
  AlertTriangle,
  Link2,
  Check,
  X,
  Sparkles,
  Maximize2,
  Minimize2,
  Radio,
  Laptop,
  RefreshCw,
  ExternalLink,
  QrCode,
  CheckCircle2,
  AlertCircle,
  Copy,
  HardDrive,
  Wifi,
  CloudUpload,
  Download,
  Power,
} from 'lucide-react';

const StageOscilloscopeWaveform: React.FC<{ isLive: boolean }> = ({ isLive }) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let animId: number;
    const waveData = new Uint8Array(new ArrayBuffer(128));

    const draw = () => {
      const w = canvas.width;
      const h = canvas.height;
      ctx.clearRect(0, 0, w, h);

      const hasRealAudio = webAudioEngine.hasActiveAudio();
      if (hasRealAudio) {
        webAudioEngine.getWaveformData(waveData);
      }

      ctx.save();
      ctx.beginPath();
      ctx.lineWidth = hasRealAudio ? 2.5 : 1.5;
      ctx.strokeStyle = hasRealAudio
        ? '#2dd4bf'
        : isLive
        ? 'rgba(148, 163, 184, 0.45)'
        : 'rgba(148, 163, 184, 0.25)';
      ctx.shadowColor = hasRealAudio ? '#14b8a6' : 'transparent';
      ctx.shadowBlur = hasRealAudio ? 12 : 0;

      if (!hasRealAudio) {
        // Honest flat standby line when no decoded Web Audio buffer is actively playing.
        ctx.moveTo(0, h / 2);
        ctx.lineTo(w, h / 2);
        ctx.stroke();
      } else {
        const sliceWidth = w / 64;
        let x = 0;

        for (let i = 0; i < 64; i++) {
          const envelope = Math.sin((i / 63) * Math.PI);
          const audioOffset = (waveData[i * 2] - 128) / 128.0;
          const y = h / 2 + audioOffset * (h * 0.42) * (0.35 + 0.65 * envelope);

          if (i === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
          x += sliceWidth;
        }
        ctx.stroke();
      }
      ctx.restore();

      animId = requestAnimationFrame(draw);
    };

    draw();
    return () => cancelAnimationFrame(animId);
  }, [isLive]);

  return (
    <canvas
      ref={canvasRef}
      width={720}
      height={80}
      className="w-full max-w-2xl h-10 md:h-14 pointer-events-none opacity-90 mt-2 drop-shadow-[0_0_12px_rgba(45,212,191,0.5)]"
    />
  );
};

export const StageDisplayScreen: React.FC = () => {
  const {
    helperUrl,
    helperConnected,
    helperVersion,
    helperError,
    wsConnected,
    lanReachable,
    detectedLanIp,
    lanNotice,
    activeRoom,
    djSkillProfile,
    exposedDecks,
    internalDeckCapacity,
    guestUrl,
    qrSvg,
    submissions,
    recommendations,
    pendingIntakeJobs,
    techniqueLog,
    deckSnapshots,
    autoDjPlaybackState,
    autoDjStatusNote,
    cacheStats,
    syncOverview,
    moodProfiles,
    selectedMoodId,
    selectedDjLevel,
    crossfader,
    decks,
    setHelperUrl,
    checkHelperHealth,
    createLocalRoom,
    startLocalRoom,
    closeLocalRoom,
    clearLocalRoomSession,
    submitSong,
    evaluateDjPool,
    setDjLevel,
    setPartyMood,
    setCrossfaderPos,
    updateDeckEq,
  } = useDhunStore();

  const [currentFxIndex, setCurrentFxIndex] = useState(0);
  const [autoCycleFx, setAutoCycleFx] = useState(true);
  const [isFullscreen, setIsFullscreen] = useState(() => Boolean(document.fullscreenElement));
  const [showHostDrawer, setShowHostDrawer] = useState(false);
  const [activeDrawerTab, setActiveDrawerTab] = useState<'party' | 'submit' | 'dj' | 'fx'>('party');

  // Form states
  const [helperUrlDraft, setHelperUrlDraft] = useState(helperUrl);
  const [partyNameInput, setPartyNameInput] = useState('DHUN Celebration Night');
  const [youtubeUrlInput, setYoutubeUrlInput] = useState('');

  // Action feedback states
  const [isBusy, setIsBusy] = useState(false);
  const [isSubmittingLink, setIsSubmittingLink] = useState(false);
  const [launchingHelper, setLaunchingHelper] = useState(false);
  const [launchNotice, setLaunchNotice] = useState<string | null>(null);
  const [roomFeedback, setRoomFeedback] = useState<{ type: 'ok' | 'err'; text: string } | null>(null);
  const [submitFeedback, setSubmitFeedback] = useState<{ type: 'ok' | 'err'; text: string } | null>(null);
  const [copiedUrl, setCopiedUrl] = useState(false);

  const helperInstallerUrl =
    ((import.meta as any).env?.VITE_HELPER_DOWNLOAD_URL as string | undefined) ||
    'https://github.com/ritulrajbhakat/Dhun/actions';

  const handleLaunchHelperProtocol = () => {
    setLaunchingHelper(true);
    setLaunchNotice('Opening DHUN Local Helper via dhun://start… Confirm the browser prompt if shown.');
    // Trigger registered OS protocol handler (like Zoom/Spotify/Figma desktop launchers)
    window.location.href = 'dhun://start';

    let attempts = 0;
    const poll = window.setInterval(async () => {
      attempts += 1;
      await checkHelperHealth();
      const connectedNow = useDhunStore.getState().helperConnected;
      if (connectedNow) {
        window.clearInterval(poll);
        setLaunchingHelper(false);
        setLaunchNotice(null);
      } else if (attempts >= 12) {
        window.clearInterval(poll);
        setLaunchingHelper(false);
        setLaunchNotice(
          'Helper did not respond yet. If this is your first time on this laptop, download and run the one-click DHUN Local Helper Installer below.'
        );
      }
    }, 1250);
  };

  useEffect(() => {
    setHelperUrlDraft(helperUrl);
  }, [helperUrl]);

  // Mood selection automatically influences visual FX preset
  const activeMoodProfile =
    moodProfiles.find((m) => m.id === selectedMoodId) || moodProfiles[0];

  useEffect(() => {
    if (activeMoodProfile?.preferred_fx_index !== undefined) {
      setCurrentFxIndex(activeMoodProfile.preferred_fx_index % 15);
    }
  }, [activeMoodProfile?.id]);

  const isPartyLive = activeRoom?.status === 'live';
  const stageBpm = activeMoodProfile
    ? Math.round((activeMoodProfile.min_bpm + activeMoodProfile.max_bpm) / 2)
    : 124;
  const stageEnergy = activeMoodProfile ? activeMoodProfile.target_energy : 0.75;

  const currentlyPlayingSubmission =
    submissions.find((s) => s.dj_status === 'playing') || null;
  const preparedSubmission =
    submissions.find((s) => s.dj_status === 'prepared') || null;
  const topRecommendation = recommendations[0] || null;
  const latestTechnique = techniqueLog[0] || null;

  const activeWebAudioDeck = deckSnapshots.find((d) => d.isPlaying) || null;
  const preloadedWebAudioDeck =
    deckSnapshots.find((d) => !d.isPlaying && d.isLoaded && d.submissionId !== null) || null;

  const formatSec = (secs: number) => {
    const clean = Math.max(0, Math.floor(secs));
    const m = Math.floor(clean / 60);
    const s = clean % 60;
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  };

  const toggleFullscreen = () => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen?.().catch(() => {});
    } else {
      document.exitFullscreen?.().catch(() => {});
    }
  };

  // Keep fullscreen indicator synchronized with actual browser fullscreen state (e.g., Esc exit)
  useEffect(() => {
    const onFullscreenChange = () => {
      setIsFullscreen(Boolean(document.fullscreenElement));
    };
    document.addEventListener('fullscreenchange', onFullscreenChange);
    return () => document.removeEventListener('fullscreenchange', onFullscreenChange);
  }, []);

  // Keyboard shortcuts: 'M' for Host Console, 'F' for Fullscreen, 'Escape' to close
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      const target = e.target as HTMLElement | null;
      const isEditableTarget =
        Boolean(target) &&
        (['INPUT', 'SELECT', 'TEXTAREA'].includes(target!.tagName) || target!.isContentEditable);

      if (e.key === 'Escape') {
        setShowHostDrawer(false);
        return;
      }
      if (isEditableTarget) return;
      if (e.key === 'm' || e.key === 'M') setShowHostDrawer((prev) => !prev);
      if (e.key === 'f' || e.key === 'F') toggleFullscreen();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Auto-cycle visual FX every 25s
  useEffect(() => {
    if (!autoCycleFx) return;
    const fxTimer = setInterval(() => {
      setCurrentFxIndex((idx) => (idx + 1) % 15);
    }, 25000);
    return () => clearInterval(fxTimer);
  }, [autoCycleFx]);

  const djLevelOptions: { level: DjLevel; label: string }[] = [
    { level: 1, label: 'DJ LEVEL 1: Professional 2-Deck Mixer' },
    { level: 2, label: 'DJ LEVEL 2: Smart Mixing' },
    { level: 3, label: 'DJ LEVEL 3: Advanced Performance' },
    { level: 4, label: 'DJ LEVEL 4: Creative DJ Tools' },
  ];

  const handleSaveHelperUrl = async () => {
    setIsBusy(true);
    await setHelperUrl(helperUrlDraft);
    setIsBusy(false);
  };

  const handleCreateRoom = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!partyNameInput.trim()) return;
    setIsBusy(true);
    setRoomFeedback(null);
    try {
      await createLocalRoom(partyNameInput.trim(), selectedDjLevel, selectedMoodId);
      setRoomFeedback({
        type: 'ok',
        text: 'Private LAN party room created on your local DHUN helper. Press "Start Party" to begin automated DJ playback.',
      });
    } catch (err: any) {
      setRoomFeedback({
        type: 'err',
        text: err?.message || 'Could not create local party room. Ensure dhun-local-helper is running.',
      });
    } finally {
      setIsBusy(false);
    }
  };

  const handleStartRoom = async () => {
    setIsBusy(true);
    setRoomFeedback(null);
    try {
      await startLocalRoom();
      setRoomFeedback({
        type: 'ok',
        text: 'Local party is LIVE! Automated DJ is active and remains open until you explicitly close it.',
      });
    } catch (err: any) {
      setRoomFeedback({ type: 'err', text: err?.message || 'Failed to start local party.' });
    } finally {
      setIsBusy(false);
    }
  };

  const handleCloseRoom = async () => {
    setIsBusy(true);
    setRoomFeedback(null);
    try {
      const res = await closeLocalRoom();
      setRoomFeedback({
        type: 'ok',
        text: res
          ? `Party explicitly closed. Invalidated ${res.invalidated_guests} guest session(s) and purged ${res.purged_cache_files} temporary audio file(s).`
          : 'Party room closed.',
      });
    } catch (err: any) {
      setRoomFeedback({ type: 'err', text: err?.message || 'Failed to close local party room.' });
    } finally {
      setIsBusy(false);
    }
  };

  const handleSubmitSong = async (e: React.FormEvent) => {
    e.preventDefault();
    const rawUrl = youtubeUrlInput.trim();
    if (!rawUrl) return;
    setIsSubmittingLink(true);
    setSubmitFeedback(null);
    try {
      const res = await submitSong(rawUrl);
      setYoutubeUrlInput('');
      setSubmitFeedback({
        type: 'ok',
        text: `Accepted "${res.title}" into persistent intake queue (${res.acquisition_status}). Metadata, authorized audio check, and analysis are running in the background — you can submit another link right away.`,
      });
    } catch (err: any) {
      setSubmitFeedback({
        type: 'err',
        text: err?.message || 'Song submission rejected. Paste a valid YouTube or YouTube Music URL.',
      });
    } finally {
      setIsSubmittingLink(false);
    }
  };

  const handleCopyGuestUrl = () => {
    if (!guestUrl) return;
    navigator.clipboard.writeText(guestUrl);
    setCopiedUrl(true);
    setTimeout(() => setCopiedUrl(false), 2000);
  };

  // Honest Local Helper Connection Badge
  const statusBadge = (() => {
    if (!helperConnected) {
      return {
        dot: 'bg-rose-500',
        textClass: 'text-rose-400',
        label: 'Local Helper Offline',
      };
    }
    if (isPartyLive) {
      return {
        dot: 'bg-emerald-400 animate-ping',
        textClass: 'text-emerald-400',
        label: wsConnected ? 'LAN LIVE • Auto DJ Active' : 'LAN LIVE • Local Helper',
      };
    }
    if (activeRoom && activeRoom.status === 'created') {
      return {
        dot: 'bg-amber-400 animate-pulse',
        textClass: 'text-amber-300',
        label: `Room ${activeRoom.room_code} • Standby`,
      };
    }
    return {
      dot: 'bg-sky-400',
      textClass: 'text-sky-300',
      label: 'Local Helper Ready',
    };
  })();

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-black select-none font-sans text-white">
      {/* ============================================================ */}
      {/* LAYER 1: GIANT LED STAGE SCREEN PLAYING 15 VIDEO FX LOOPS    */}
      {/* ============================================================ */}
      <div className="absolute top-[2.8%] left-[12.2%] right-[12.5%] bottom-[26.2%] overflow-hidden bg-black z-0">
        <StageVisualFxCanvas
          bpm={stageBpm}
          energy={stageEnergy}
          isPlaying={isPartyLive}
          activeFxIndex={currentFxIndex}
          onFxChange={(idx) => setCurrentFxIndex(idx)}
        />

        {/* Central Golden Devanagari Title & Oscilloscope Waveform */}
        <div className="absolute inset-0 flex flex-col items-center justify-start pt-2 md:pt-4 pointer-events-none z-10 px-4">
          <h1
            className="text-6xl md:text-8xl font-black text-amber-400 tracking-wider"
            style={{
              textShadow:
                '0 2px 0 #000, 0 4px 0 #000, 0 6px 0 #000, 0 8px 16px rgba(0,0,0,0.95), 0 0 35px rgba(245,158,11,0.55)',
            }}
          >
            धुन डीजे
          </h1>
          <p className="text-xs md:text-sm font-bold tracking-[0.35em] uppercase text-white/95 drop-shadow-[0_2px_8px_rgba(0,0,0,0.9)] mt-1">
            {activeRoom && activeRoom.status !== 'closed'
              ? `${activeRoom.party_name} • ${activeMoodProfile.display_name}`
              : 'DHUN — LOCAL AUTOMATIC PARTY DJ'}
          </p>

          <StageOscilloscopeWaveform isLive={isPartyLive} />
        </div>
      </div>

      {/* ============================================================ */}
      {/* LAYER 2: STAGE WINDOW FRAME (LATERAL BULBS & CROWD OUTLINE)   */}
      {/* ============================================================ */}
      <div
        className="absolute inset-0 bg-cover bg-center select-none pointer-events-none z-10"
        style={{ backgroundImage: "url('./stage_window_frame.png')" }}
      />

      {/* ============================================================ */}
      {/* LAYER 3: ANIMATED RESIDENT DJ CHARACTER                      */}
      {/* ============================================================ */}
      <div className="absolute left-1/2 bottom-[17.5%] -translate-x-1/2 w-[460px] md:w-[540px] h-[480px] md:h-[560px] pointer-events-none z-20">
        <AnimatedDjCharacter
          bpm={stageBpm}
          energy={stageEnergy}
          moodId={selectedMoodId}
          isPlaying={isPartyLive}
        />
      </div>

      {/* ============================================================ */}
      {/* LAYER 4: FOREGROUND DJ CONSOLE DESK & MARIGOLD GARLANDS      */}
      {/* ============================================================ */}
      <div
        className="absolute bottom-0 left-0 right-0 h-[26.2%] bg-cover bg-top pointer-events-none z-30"
        style={{ backgroundImage: "url('./dj_desk_foreground.png')" }}
      />

      {/* ============================================================ */}
      {/* LAYER 5: THREE FLOATING FROSTED GLASS UI CARDS               */}
      {/* ============================================================ */}

      {/* 5A. TOP-LEFT: HONEST LOCAL HELPER STATUS, 4 DJ LEVELS, 14 MOODS */}
      <div className="absolute top-5 left-5 z-40 flex flex-col space-y-2 pointer-events-auto">
        <div
          onClick={() => setShowHostDrawer(true)}
          title="Open Host DJ Console & Local Party Helper Settings (Press 'M')"
          className="inline-flex flex-col bg-zinc-950/75 backdrop-blur-xl border border-white/10 px-3.5 py-2 rounded-xl shadow-2xl cursor-pointer hover:border-amber-400/40 transition-all"
        >
          <span className="text-[11px] font-bold text-slate-300 flex items-center justify-between gap-3">
            <span>Local Helper Status</span>
            <span className="text-[9px] font-mono text-amber-300/80 uppercase">Press M</span>
          </span>
          <div className="flex items-center space-x-1.5 mt-0.5">
            <span className={`w-2 h-2 rounded-full ${statusBadge.dot}`} />
            <span className={`text-[11px] font-semibold ${statusBadge.textClass}`}>
              {statusBadge.label}
            </span>
          </div>
        </div>

        {/* 4 DJ LEVELS Dropdown */}
        <div className="relative">
          <select
            value={selectedDjLevel}
            onChange={async (e) => {
              await setDjLevel(Number(e.target.value) as DjLevel);
            }}
            className="w-60 appearance-none bg-zinc-950/75 backdrop-blur-xl border border-white/10 hover:border-amber-400/50 text-white text-xs font-bold px-3.5 py-2 rounded-xl focus:outline-none focus:ring-1 focus:ring-amber-400 cursor-pointer shadow-xl transition-all"
          >
            {djLevelOptions.map((opt) => (
              <option key={opt.level} value={opt.level} className="bg-zinc-950 text-white">
                {opt.label}
              </option>
            ))}
          </select>
          <div className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-slate-400 text-xs">
            ▼
          </div>
        </div>

        {/* ALL 14 MOODS Dropdown */}
        <div className="relative">
          <select
            value={selectedMoodId}
            onChange={async (e) => {
              await setPartyMood(e.target.value);
            }}
            className="w-60 appearance-none bg-zinc-950/75 backdrop-blur-xl border border-white/10 hover:border-amber-400/50 text-white text-xs font-bold px-3.5 py-2 rounded-xl focus:outline-none focus:ring-1 focus:ring-amber-400 cursor-pointer shadow-xl transition-all"
          >
            {moodProfiles.map((m) => (
              <option key={m.id} value={m.id} className="bg-zinc-950 text-white">
                MOOD: {m.display_name}
              </option>
            ))}
          </select>
          <div className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-slate-400 text-xs">
            ▼
          </div>
        </div>

        {!isPartyLive && (
          <button
            onClick={() => {
              setActiveDrawerTab('party');
              setShowHostDrawer(true);
            }}
            className="flex items-center justify-center space-x-2 bg-gradient-to-r from-amber-500 to-rose-600 hover:from-amber-400 hover:to-rose-500 text-white text-xs font-black px-3.5 py-2 rounded-xl shadow-lg shadow-rose-950/60 transition-all"
          >
            <Radio className="w-3.5 h-3.5" />
            <span>{activeRoom && activeRoom.status === 'created' ? 'Start Local Party' : 'Setup Local Party'}</span>
          </button>
        )}
      </div>

      {/* 5B. BOTTOM-LEFT: AUTOMATED DJ DECK & CANDIDATE STATUS CARD */}
      <div className="absolute bottom-6 left-6 z-40 w-72 md:w-80 bg-zinc-950/80 backdrop-blur-2xl border border-white/15 p-4 rounded-3xl shadow-[0_20px_50px_rgba(0,0,0,0.9)] pointer-events-auto">
        <div className="flex items-center justify-between text-xs text-slate-400 mb-2 font-bold">
          <span className="text-white flex items-center space-x-1.5">
            <span
              className={`w-2 h-2 rounded-full ${
                activeWebAudioDeck
                  ? 'bg-emerald-400 animate-pulse'
                  : isPartyLive
                  ? 'bg-amber-400'
                  : 'bg-slate-500'
              }`}
            />
            <span>
              {activeWebAudioDeck
                ? `Playing • Deck ${String.fromCharCode(65 + activeWebAudioDeck.deckId)}`
                : isPartyLive
                ? 'Auto DJ Active'
                : 'DJ Standby'}
            </span>
          </span>
          <span className="text-[10px] font-mono text-teal-300 bg-teal-500/10 border border-teal-500/30 px-2 py-0.5 rounded-full">
            L{selectedDjLevel} • {exposedDecks}/{internalDeckCapacity} Decks
            {latestTechnique ? ` • ${latestTechnique.techniqueLabel}` : ''}
          </span>
        </div>

        {activeWebAudioDeck ? (
          <>
            <div className="flex items-center space-x-3 mb-2">
              <div className="w-11 h-11 rounded-xl bg-gradient-to-br from-emerald-500 via-teal-600 to-sky-600 flex items-center justify-center shadow-lg">
                <Disc3 className="w-6 h-6 text-white animate-[spin_4s_linear_infinite]" />
              </div>
              <div className="flex-1 min-w-0">
                <h4 className="text-sm font-extrabold text-white truncate drop-shadow">
                  {activeWebAudioDeck.title}
                </h4>
                <p className="text-xs text-slate-300 truncate">
                  {activeWebAudioDeck.artist}
                  {activeWebAudioDeck.bpm ? ` • ${Math.round(activeWebAudioDeck.bpm)} BPM` : ''}
                  {activeWebAudioDeck.camelotKey ? ` • ${activeWebAudioDeck.camelotKey}` : ''}
                </p>
              </div>
            </div>

            {/* Genuine AudioContext.currentTime progress bar (zero fake timers) */}
            <div className="space-y-1 mb-2.5">
              <div className="w-full h-1.5 bg-white/15 rounded-full overflow-hidden">
                <div
                  className="h-full bg-teal-400 rounded-full transition-all duration-200"
                  style={{
                    width: `${Math.min(
                      100,
                      (activeWebAudioDeck.positionSec / Math.max(0.1, activeWebAudioDeck.durationSec)) * 100
                    )}%`,
                  }}
                />
              </div>
              <div className="flex justify-between text-[10px] font-mono text-slate-400">
                <span>{formatSec(activeWebAudioDeck.positionSec)}</span>
                <span>
                  {preloadedWebAudioDeck
                    ? `Next: Deck ${String.fromCharCode(65 + preloadedWebAudioDeck.deckId)}`
                    : formatSec(activeWebAudioDeck.durationSec)}
                </span>
              </div>
            </div>
          </>
        ) : currentlyPlayingSubmission ? (
          <>
            <div className="flex items-center space-x-3 mb-2.5">
              {currentlyPlayingSubmission.thumbnail_url ? (
                <img
                  src={currentlyPlayingSubmission.thumbnail_url}
                  alt={currentlyPlayingSubmission.title}
                  className="w-11 h-11 rounded-xl object-cover shadow-lg border border-white/10"
                />
              ) : (
                <div className="w-11 h-11 rounded-xl bg-gradient-to-br from-emerald-600 via-teal-600 to-sky-500 flex items-center justify-center shadow-lg">
                  <Disc3 className="w-6 h-6 text-white" />
                </div>
              )}
              <div className="flex-1 min-w-0">
                <h4 className="text-sm font-extrabold text-white truncate drop-shadow">
                  {currentlyPlayingSubmission.title}
                </h4>
                <p className="text-xs text-slate-300 truncate">
                  {currentlyPlayingSubmission.artist}
                  {currentlyPlayingSubmission.bpm ? ` • ${Math.round(currentlyPlayingSubmission.bpm)} BPM` : ''}
                  {currentlyPlayingSubmission.camelot_key ? ` • ${currentlyPlayingSubmission.camelot_key}` : ''}
                </p>
              </div>
            </div>
          </>
        ) : preparedSubmission ? (
          <>
            <div className="flex items-center space-x-3 mb-2">
              {preparedSubmission.thumbnail_url ? (
                <img
                  src={preparedSubmission.thumbnail_url}
                  alt={preparedSubmission.title}
                  className="w-11 h-11 rounded-xl object-cover shadow-lg border border-white/10"
                />
              ) : (
                <div className="w-11 h-11 rounded-xl bg-gradient-to-br from-amber-600 via-orange-600 to-rose-500 flex items-center justify-center shadow-lg">
                  <Disc3 className="w-6 h-6 text-white" />
                </div>
              )}
              <div className="flex-1 min-w-0">
                <p className="text-[10px] font-bold uppercase tracking-wider text-amber-300">
                  Prepared Next • Not Playing Yet
                </p>
                <h4 className="text-sm font-extrabold text-white truncate drop-shadow">
                  {preparedSubmission.title}
                </h4>
                <p className="text-xs text-slate-300 truncate">
                  {preparedSubmission.artist}
                  {preparedSubmission.bpm ? ` • ${Math.round(preparedSubmission.bpm)} BPM` : ''}
                  {preparedSubmission.camelot_key ? ` • ${preparedSubmission.camelot_key}` : ''}
                </p>
              </div>
            </div>
            <div className="bg-amber-500/10 border border-amber-400/25 rounded-xl px-2.5 py-1.5 mb-2.5">
              <p className="text-[10px] text-amber-200 leading-tight">
                {autoDjStatusNote ||
                  `Score ${preparedSubmission.dj_score.toFixed(1)} • ${preparedSubmission.acquisition_status}`}
              </p>
            </div>
          </>
        ) : submissions.length > 0 ? (
          <div className="space-y-2 mb-2.5">
            <div className="flex items-center space-x-3">
              <div className="w-10 h-10 rounded-xl bg-white/5 border border-white/10 flex items-center justify-center shrink-0">
                <Music className="w-5 h-5 text-amber-300" />
              </div>
              <div className="flex-1 min-w-0">
                <h4 className="text-xs font-extrabold text-white truncate">
                  No Track Playing • {submissions.length} in Pool
                </h4>
                <p className="text-[11px] text-slate-300 truncate">
                  {pendingIntakeJobs > 0
                    ? `Processing ${pendingIntakeJobs} link(s) in background…`
                    : topRecommendation
                    ? `Suggested: ${topRecommendation.title} (${topRecommendation.technique_label})`
                    : 'Awaiting authorized playable audio'}
                </p>
              </div>
            </div>
            <div className="bg-amber-500/10 border border-amber-400/25 rounded-xl px-2.5 py-1.5">
              <p className="text-[10px] text-amber-200 leading-tight">
                {autoDjPlaybackState === 'awaiting_playable_audio' && autoDjStatusNote
                  ? autoDjStatusNote
                  : topRecommendation
                  ? `${topRecommendation.category_label} • Score ${topRecommendation.compatibility_score.toFixed(1)} • ${
                      topRecommendation.is_playable ? 'Playable now' : topRecommendation.playability_label.replace(/_/g, ' ')
                    }`
                  : 'Open Auto DJ & Cache (Press M) for live DJ recommendations.'}
              </p>
            </div>
          </div>
        ) : (
          <div className="py-3 text-center space-y-1.5 mb-2">
            <div className="w-10 h-10 mx-auto rounded-xl bg-white/5 border border-white/10 flex items-center justify-center">
              <Music className="w-5 h-5 text-slate-400" />
            </div>
            <h4 className="text-sm font-bold text-slate-200">Private DJ Pool Empty</h4>
            <p className="text-[11px] text-slate-400 leading-snug">
              {activeRoom && activeRoom.status !== 'closed'
                ? 'Submit a YouTube / YouTube Music link or let LAN guests scan the QR code.'
                : 'Start the local helper on this laptop and create a private LAN party room.'}
            </p>
          </div>
        )}

        <div className="flex items-center justify-between pt-2 border-t border-white/10 gap-2">
          <button
            onClick={() => {
              setActiveDrawerTab('submit');
              setShowHostDrawer(true);
            }}
            className="flex-1 py-1.5 rounded-xl bg-white/10 hover:bg-white/15 text-[11px] font-bold text-slate-200 flex items-center justify-center space-x-1.5 transition-colors"
          >
            <Link2 className="w-3.5 h-3.5 text-amber-400" />
            <span>Submit Song</span>
          </button>

          {activeRoom && activeRoom.status === 'created' && (
            <button
              onClick={handleStartRoom}
              disabled={isBusy}
              className="px-3 py-1.5 rounded-xl bg-teal-400 hover:bg-teal-300 text-black text-[11px] font-black flex items-center space-x-1 shadow-lg shadow-teal-400/30 transition-all"
            >
              <Play className="w-3.5 h-3.5 fill-current" />
              <span>Start</span>
            </button>
          )}

          {isPartyLive && (
            <button
              onClick={handleCloseRoom}
              disabled={isBusy}
              className="px-3 py-1.5 rounded-xl bg-rose-600/80 hover:bg-rose-500 text-white text-[11px] font-bold flex items-center space-x-1 transition-all"
              title="Explicitly close party room and clean temporary cache"
            >
              <Square className="w-3 h-3 fill-current" />
              <span>Close</span>
            </button>
          )}
        </div>
      </div>

      {/* 5C. BOTTOM-RIGHT: REAL REACHABLE LAN QR CARD (ZERO FAKE IPs) */}
      <div className="absolute bottom-6 right-6 z-40 w-64 md:w-72 bg-zinc-950/80 backdrop-blur-2xl border border-white/15 p-4 rounded-3xl shadow-[0_20px_50px_rgba(0,0,0,0.9)] pointer-events-auto flex flex-col items-center text-center">
        <h4 className="text-xs font-bold tracking-wider text-slate-200 uppercase mb-2.5 flex items-center gap-1.5">
          <QrCode className="w-3.5 h-3.5 text-amber-400" />
          <span>Same-Wi-Fi Guest QR</span>
        </h4>

        {activeRoom && activeRoom.status !== 'closed' && lanReachable && qrSvg ? (
          <>
            <div
              className="w-32 h-32 md:w-36 md:h-36 bg-white p-2 rounded-2xl shadow-inner flex items-center justify-center mb-2 overflow-hidden [&>svg]:w-full [&>svg]:h-full"
              dangerouslySetInnerHTML={{ __html: qrSvg }}
            />
            <p className="text-xs font-black tracking-wide text-white uppercase drop-shadow">
              SCAN TO REQUEST
            </p>
            <p className="text-[11px] font-bold text-amber-300 mt-0.5">
              स्कैन कर गाना मांगें &bull; Code: <span className="font-mono">{activeRoom.room_code}</span>
            </p>
            <button
              onClick={handleCopyGuestUrl}
              className="mt-1.5 px-2.5 py-1 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 text-[10px] font-mono text-slate-300 flex items-center gap-1.5 max-w-full"
              title="Copy LAN Guest Join URL"
            >
              <Wifi className="w-3 h-3 text-emerald-400 shrink-0" />
              <span className="truncate">{detectedLanIp}</span>
              {copiedUrl ? <Check className="w-3 h-3 text-emerald-400 shrink-0" /> : <Copy className="w-3 h-3 shrink-0" />}
            </button>
          </>
        ) : (
          <div className="w-full py-5 px-3 rounded-2xl bg-black/50 border border-dashed border-white/15 flex flex-col items-center justify-center space-y-2 mb-1">
            <QrCode className="w-10 h-10 text-slate-600" />
            <p className="text-xs font-bold text-slate-300">
              {activeRoom && activeRoom.status !== 'closed' && !lanReachable
                ? 'No Reachable LAN Wi-Fi'
                : 'Local LAN QR Inactive'}
            </p>
            <p className="text-[10px] text-slate-400 leading-relaxed">
              {lanNotice ||
                'Connect to your Local DHUN Helper and create a private LAN party room to generate a real Wi-Fi QR code.'}
            </p>
            <div className="flex flex-wrap items-center justify-center gap-1.5 mt-1">
              {!helperConnected && (
                <button
                  onClick={handleLaunchHelperProtocol}
                  disabled={launchingHelper}
                  className="px-2.5 py-1.5 rounded-lg bg-emerald-500/20 hover:bg-emerald-500/30 border border-emerald-400/40 text-emerald-300 text-[11px] font-bold transition-colors flex items-center gap-1"
                >
                  <Power className={`w-3 h-3 ${launchingHelper ? 'animate-spin' : ''}`} />
                  <span>{launchingHelper ? 'Launching…' : 'Launch Helper'}</span>
                </button>
              )}
              <button
                onClick={() => {
                  setActiveDrawerTab('party');
                  setShowHostDrawer(true);
                }}
                className="px-3 py-1.5 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 border border-amber-400/40 text-amber-300 text-[11px] font-bold transition-colors"
              >
                Open Local Setup
              </button>
            </div>
          </div>
        )}
      </div>

      {/* ============================================================ */}
      {/* HOST SLIDE-OVER CONSOLE DRAWER (HOTKEY 'M' OR CLICK STATUS)  */}
      {/* ============================================================ */}
      {showHostDrawer && (
        <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex justify-end transition-all">
          <div className="w-full max-w-xl h-full bg-zinc-950 border-l border-white/15 p-6 flex flex-col shadow-2xl overflow-y-auto">
            {/* Drawer Header */}
            <div className="flex items-center justify-between pb-4 border-b border-white/10">
              <div className="flex items-center space-x-2.5">
                <Sliders className="w-5 h-5 text-amber-400" />
                <div>
                  <h3 className="text-lg font-black text-white leading-none">DHUN Host Console</h3>
                  <p className="text-[11px] text-slate-400 mt-1">
                    Local Party Helper &bull; Private LAN Room &bull; Automated 4-Deck DJ
                  </p>
                </div>
              </div>
              <button
                onClick={() => setShowHostDrawer(false)}
                className="p-1.5 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-slate-300 hover:text-white"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* 4 Clean Tabs (Notice: ZERO Approval Queue, ZERO Global Catalogue Browser) */}
            <div className="flex space-x-1.5 my-4">
              <button
                onClick={() => setActiveDrawerTab('party')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'party'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400 hover:text-white'
                }`}
              >
                Local Party
              </button>
              <button
                onClick={() => setActiveDrawerTab('submit')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'submit'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400 hover:text-white'
                }`}
              >
                Submit Song ({submissions.length})
              </button>
              <button
                onClick={() => setActiveDrawerTab('dj')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'dj'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400 hover:text-white'
                }`}
              >
                Auto DJ &amp; Cache
              </button>
              <button
                onClick={() => setActiveDrawerTab('fx')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'fx'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400 hover:text-white'
                }`}
              >
                15 Video FX
              </button>
            </div>

            <div className="flex-1 overflow-y-auto space-y-4">
              {/* TAB 1: LOCAL DHUN HELPER & PRIVATE LAN PARTY ROOM */}
              {activeDrawerTab === 'party' && (
                <div className="space-y-4">
                  <div className="bg-zinc-900/90 p-4 rounded-xl border border-white/10 space-y-2.5">
                    <div className="flex items-center justify-between">
                      <h4 className="text-xs font-bold text-sky-300 uppercase flex items-center gap-1.5">
                        <Laptop className="w-3.5 h-3.5" />
                        <span>Host Local DHUN Helper</span>
                      </h4>
                      <span
                        className={`text-[10px] font-bold px-2 py-0.5 rounded-full ${
                          helperConnected
                            ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                            : 'bg-rose-500/20 text-rose-300 border border-rose-500/40'
                        }`}
                      >
                        {helperConnected ? `Running v${helperVersion || '2.0'}` : 'Not Detected'}
                      </span>
                    </div>

                    <div className="flex gap-2">
                      <input
                        type="url"
                        value={helperUrlDraft}
                        onChange={(e) => setHelperUrlDraft(e.target.value)}
                        placeholder="http://127.0.0.1:47821"
                        className="flex-1 bg-black/60 border border-white/15 px-3 py-1.5 text-xs rounded-lg text-white font-mono outline-none focus:border-sky-400"
                      />
                      <button
                        onClick={handleSaveHelperUrl}
                        disabled={isBusy}
                        className="px-3 py-1.5 bg-sky-500 hover:bg-sky-400 text-black font-extrabold text-xs rounded-lg flex items-center gap-1"
                      >
                        <RefreshCw className={`w-3.5 h-3.5 ${isBusy ? 'animate-spin' : ''}`} />
                        <span>Connect</span>
                      </button>
                    </div>

                    <p className="text-[11px] text-slate-400 leading-snug">
                      Party rooms run privately on this laptop&rsquo;s local helper — never in the cloud. Guests on the same Wi-Fi connect directly via LAN QR code.
                    </p>

                    {!helperConnected && (
                      <div className="space-y-2.5 pt-1">
                        <p className="text-[11px] text-rose-400 flex items-center gap-1.5">
                          <AlertCircle className="w-3.5 h-3.5 shrink-0" />
                          <span>
                            {helperError || 'Local helper is not running yet.'}
                          </span>
                        </p>

                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                          <button
                            type="button"
                            onClick={handleLaunchHelperProtocol}
                            disabled={launchingHelper}
                            className="py-2 px-3 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-black font-black text-xs flex items-center justify-center gap-1.5 transition-colors disabled:opacity-60"
                          >
                            <Power className={`w-3.5 h-3.5 ${launchingHelper ? 'animate-spin' : ''}`} />
                            <span>{launchingHelper ? 'Starting Helper…' : 'Launch Installed Helper'}</span>
                          </button>
                          <a
                            href={helperInstallerUrl}
                            target="_blank"
                            rel="noreferrer"
                            className="py-2 px-3 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 border border-amber-400/40 text-amber-200 font-bold text-xs flex items-center justify-center gap-1.5 transition-colors"
                          >
                            <Download className="w-3.5 h-3.5" />
                            <span>Download Windows Installer</span>
                          </a>
                        </div>

                        {launchNotice && (
                          <p className="text-[11px] text-sky-300 bg-sky-500/10 border border-sky-400/30 rounded-lg px-2.5 py-1.5">
                            {launchNotice}
                          </p>
                        )}

                        <p className="text-[10px] text-slate-400 leading-snug">
                          First time on this host laptop? Run the one-click <code className="text-slate-200">DHUN-Local-Helper-Setup.exe</code> installer once to install the helper, register <code className="text-slate-200">dhun://start</code>, and allow Private Wi-Fi QR access on port 47821.
                        </p>
                      </div>
                    )}
                  </div>

                  {/* Private LAN Party Room Lifecycle */}
                  <div className="bg-zinc-900/90 p-4 rounded-xl border border-white/10 space-y-3">
                    <h4 className="text-xs font-bold text-amber-300 uppercase flex items-center gap-1.5">
                      <Radio className="w-3.5 h-3.5" />
                      <span>Private Local Party Room</span>
                    </h4>

                    {!activeRoom || activeRoom.status === 'closed' ? (
                      <form onSubmit={handleCreateRoom} className="space-y-3">
                        <div>
                          <label className="text-[11px] text-slate-400 block mb-1">Party Room Name</label>
                          <input
                            type="text"
                            value={partyNameInput}
                            onChange={(e) => setPartyNameInput(e.target.value)}
                            placeholder="e.g. Sharma Wedding Sangeet"
                            className="w-full bg-black/60 border border-white/15 px-3 py-2 text-xs rounded-lg text-white outline-none focus:border-amber-400"
                          />
                        </div>
                        <button
                          type="submit"
                          disabled={!helperConnected || isBusy || !partyNameInput.trim()}
                          className="w-full py-2.5 bg-amber-500 hover:bg-amber-400 disabled:opacity-40 text-black font-black text-xs rounded-xl transition-all"
                        >
                          {isBusy ? 'Creating Private Room...' : 'Create Private Local Party Room'}
                        </button>
                      </form>
                    ) : (
                      <div className="space-y-3">
                        <div className="p-3 rounded-lg bg-black/50 border border-white/10 space-y-1.5 text-xs">
                          <div className="flex justify-between">
                            <span className="text-slate-400">Party Name:</span>
                            <span className="font-bold text-white">{activeRoom.party_name}</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-slate-400">Room Code (LAN):</span>
                            <span className="font-mono font-bold text-amber-300">{activeRoom.room_code}</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-slate-400">Reachable LAN IP:</span>
                            <span className="font-mono text-emerald-300">
                              {lanReachable ? detectedLanIp : 'Not Connected to Wi-Fi/LAN'}
                            </span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-slate-400">Room Status:</span>
                            <span className="font-bold uppercase text-emerald-400">
                              {activeRoom.status} (Explicit Close Only)
                            </span>
                          </div>
                        </div>

                        <div className="flex gap-2">
                          {activeRoom.status === 'created' && (
                            <button
                              onClick={handleStartRoom}
                              disabled={isBusy}
                              className="flex-1 py-2 bg-emerald-500 hover:bg-emerald-400 text-black font-black text-xs rounded-xl flex items-center justify-center gap-1.5"
                            >
                              <Play className="w-3.5 h-3.5 fill-current" />
                              <span>Start Automated DJ Party</span>
                            </button>
                          )}

                          {activeRoom.status === 'live' && (
                            <button
                              onClick={handleCloseRoom}
                              disabled={isBusy}
                              className="flex-1 py-2 bg-rose-600 hover:bg-rose-500 text-white font-bold text-xs rounded-xl flex items-center justify-center gap-1.5"
                            >
                              <Square className="w-3.5 h-3.5 fill-current" />
                              <span>Explicitly Close Party &amp; Purge Cache</span>
                            </button>
                          )}

                          <button
                            onClick={clearLocalRoomSession}
                            className="px-3 py-2 bg-zinc-800 hover:bg-zinc-700 text-slate-200 font-bold text-xs rounded-xl"
                          >
                            Reset UI
                          </button>
                        </div>
                      </div>
                    )}

                    {roomFeedback && (
                      <div
                        className={`text-xs px-3 py-2 rounded-lg border flex items-center gap-2 ${
                          roomFeedback.type === 'ok'
                            ? 'bg-emerald-950/50 border-emerald-500/40 text-emerald-300'
                            : 'bg-rose-950/50 border-rose-500/40 text-rose-300'
                        }`}
                      >
                        {roomFeedback.type === 'ok' ? (
                          <CheckCircle2 className="w-4 h-4 shrink-0" />
                        ) : (
                          <AlertCircle className="w-4 h-4 shrink-0" />
                        )}
                        <span>{roomFeedback.text}</span>
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* TAB 2: UNIFIED HOST & GUEST SONG SUBMISSION (ASYNC INTAKE QUEUE, NO APPROVAL QUEUE) */}
              {activeDrawerTab === 'submit' && (
                <div className="space-y-4">
                  <form
                    onSubmit={handleSubmitSong}
                    className="bg-zinc-900/90 p-4 rounded-xl border border-white/10 space-y-2.5"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <h4 className="text-xs font-bold text-amber-300 uppercase flex items-center gap-1.5">
                        <Link2 className="w-3.5 h-3.5" />
                        <span>Submit Song (Instant Async Intake)</span>
                      </h4>
                      {pendingIntakeJobs > 0 && (
                        <span className="px-2 py-0.5 rounded-full bg-sky-500/20 border border-sky-400/40 text-sky-300 text-[10px] font-mono font-bold flex items-center gap-1">
                          <RefreshCw className="w-2.5 h-2.5 animate-spin" />
                          <span>{pendingIntakeJobs} processing</span>
                        </span>
                      )}
                    </div>
                    <p className="text-[11px] text-slate-400">
                      Links enter the persistent background intake queue immediately so you can submit multiple tracks back-to-back without waiting. Metadata, authorized audio checks, and Symphonia analysis run asynchronously.
                    </p>
                    <div className="flex gap-2">
                      <input
                        type="url"
                        value={youtubeUrlInput}
                        onChange={(e) => setYoutubeUrlInput(e.target.value)}
                        placeholder="https://www.youtube.com/watch?v=..."
                        disabled={!activeRoom || activeRoom.status === 'closed'}
                        className="flex-1 bg-black/60 border border-white/15 px-3 py-1.5 text-xs rounded-lg text-white font-mono outline-none focus:border-amber-400 disabled:opacity-40"
                      />
                      <button
                        type="submit"
                        disabled={!activeRoom || activeRoom.status === 'closed' || isSubmittingLink || !youtubeUrlInput.trim()}
                        className="px-3.5 py-1.5 bg-amber-500 hover:bg-amber-400 disabled:opacity-40 text-black font-extrabold text-xs rounded-lg shrink-0"
                      >
                        {isSubmittingLink ? 'Queueing...' : 'Queue Link'}
                      </button>
                    </div>

                    {submitFeedback && (
                      <div
                        className={`text-xs px-3 py-2 rounded-lg border flex items-center gap-2 ${
                          submitFeedback.type === 'ok'
                            ? 'bg-emerald-950/50 border-emerald-500/40 text-emerald-300'
                            : 'bg-rose-950/50 border-rose-500/40 text-rose-300'
                        }`}
                      >
                        {submitFeedback.type === 'ok' ? (
                          <CheckCircle2 className="w-4 h-4 shrink-0" />
                        ) : (
                          <AlertCircle className="w-4 h-4 shrink-0" />
                        )}
                        <span>{submitFeedback.text}</span>
                      </div>
                    )}
                  </form>

                  {/* Private Party Candidate Pool (Host View Only — Guests Cannot See This) */}
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <h4 className="text-xs font-bold text-slate-400 uppercase">
                        Private DJ Candidate Pool ({submissions.length})
                      </h4>
                      <button
                        onClick={() => evaluateDjPool()}
                        disabled={!activeRoom || activeRoom.status === 'closed'}
                        className="text-[11px] font-bold text-teal-300 hover:text-teal-200 disabled:opacity-40"
                      >
                        Re-score &amp; Refresh Suggestions
                      </button>
                    </div>

                    {submissions.length === 0 ? (
                      <div className="p-6 text-center rounded-xl bg-zinc-900/50 border border-white/5 text-xs text-slate-500">
                        No songs in the private candidate pool yet. Submit a YouTube link above or invite LAN guests.
                      </div>
                    ) : (
                      <div className="space-y-2">
                        {submissions.map((sub) => {
                          const acqBadgeClass = (() => {
                            switch (sub.acquisition_status) {
                              case 'cached_authorized':
                                return 'bg-emerald-500/20 border-emerald-500/40 text-emerald-300';
                              case 'received':
                              case 'metadata_processing':
                              case 'awaiting_authorized_audio':
                              case 'analysing':
                              case 'pending':
                                return 'bg-sky-500/20 border-sky-500/40 text-sky-300';
                              case 'failed':
                                return 'bg-rose-500/20 border-rose-500/40 text-rose-300';
                              default:
                                return 'bg-amber-500/15 border-amber-500/30 text-amber-300';
                            }
                          })();
                          const acqLabel =
                            sub.acquisition_status === 'cached_authorized'
                              ? 'ready'
                              : sub.acquisition_status === 'restricted_by_provider'
                              ? 'metadata_only'
                              : sub.acquisition_status;

                          return (
                            <div
                              key={sub.id}
                              className="flex items-center justify-between gap-3 p-2.5 bg-zinc-900/70 rounded-xl border border-white/10"
                            >
                              <div className="flex items-center gap-3 min-w-0">
                                {sub.thumbnail_url ? (
                                  <img
                                    src={sub.thumbnail_url}
                                    alt={sub.title}
                                    className="w-11 h-8 rounded-md object-cover bg-black shrink-0"
                                  />
                                ) : (
                                  <div className="w-11 h-8 rounded-md bg-zinc-800 flex items-center justify-center shrink-0">
                                    <Music className="w-4 h-4 text-slate-500" />
                                  </div>
                                )}
                                <div className="min-w-0">
                                  <p className="text-xs font-bold text-white truncate">
                                    {sub.title}{' '}
                                    {sub.version_tag !== 'original' && (
                                      <span className="text-[9px] uppercase text-amber-300">[{sub.version_tag}]</span>
                                    )}
                                  </p>
                                  <p className="text-[11px] text-slate-400 truncate">
                                    {sub.artist} &bull; via {sub.submitted_by_type === 'host' ? 'Host' : sub.submitted_by_name} &bull; Score{' '}
                                    <span className="font-mono text-teal-300">{sub.dj_score.toFixed(1)}</span>
                                    {sub.bpm ? ` • ${Math.round(sub.bpm)} BPM` : ''}
                                    {sub.camelot_key ? ` • ${sub.camelot_key}` : ''}
                                  </p>
                                </div>
                              </div>

                              <div className="flex items-center gap-1.5 shrink-0">
                                <span className="px-1.5 py-0.5 rounded bg-teal-500/15 border border-teal-500/30 text-[9px] font-mono font-bold text-teal-300">
                                  {sub.dj_status}
                                </span>
                                <span
                                  className={`px-1.5 py-0.5 rounded border text-[9px] font-mono font-bold ${acqBadgeClass}`}
                                  title={sub.acquisition_note || 'Pipeline intake status'}
                                >
                                  {acqLabel}
                                </span>
                                <a
                                  href={sub.source_url}
                                  target="_blank"
                                  rel="noreferrer"
                                  className="p-1 text-slate-400 hover:text-white"
                                  title="Open Official Source"
                                >
                                  <ExternalLink className="w-3.5 h-3.5" />
                                </a>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* TAB 3: AUTOMATED 4-DECK DJ, SKILL PROFILE, EXPLAINABLE SUGGESTIONS & REAL TECHNIQUE LOG */}
              {activeDrawerTab === 'dj' && (
                <div className="space-y-4">
                  {/* Active DJ Level Skill Profile (Real Inherited Capabilities L1 -> L4) */}
                  {djSkillProfile && (
                    <div className="bg-zinc-900/90 p-3.5 rounded-xl border border-amber-400/25 space-y-2">
                      <div className="flex items-center justify-between gap-2">
                        <h4 className="text-xs font-bold text-amber-300 uppercase">
                          Level {djSkillProfile.level}: {djSkillProfile.name}
                        </h4>
                        <span className="text-[10px] font-mono text-teal-300 bg-teal-500/10 border border-teal-500/30 px-2 py-0.5 rounded-full">
                          {djSkillProfile.exposed_decks} Decks &bull; &plusmn;{djSkillProfile.max_tempo_stretch_pct}% Pitch
                        </span>
                      </div>
                      <p className="text-[11px] text-slate-300 leading-snug">{djSkillProfile.summary}</p>
                      <div className="flex flex-wrap gap-1 pt-0.5">
                        {djSkillProfile.unlocked_techniques.map((tech) => (
                          <span
                            key={tech}
                            className="px-1.5 py-0.5 rounded bg-white/5 border border-white/10 text-[9px] font-mono text-slate-300"
                          >
                            {tech.replace(/_/g, ' ')}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Explainable DJ Recommendations (Works even with incomplete metadata; honest playability) */}
                  <div className="bg-zinc-900/90 p-3.5 rounded-xl border border-white/10 space-y-2.5">
                    <div className="flex items-center justify-between">
                      <h4 className="text-xs font-bold text-teal-300 uppercase">
                        DJ Suggestions ({recommendations.length})
                      </h4>
                      <button
                        onClick={() => evaluateDjPool()}
                        disabled={!activeRoom || activeRoom.status === 'closed'}
                        className="text-[10px] font-bold text-amber-300 hover:text-amber-200 disabled:opacity-40"
                      >
                        Re-evaluate
                      </button>
                    </div>

                    {recommendations.length === 0 ? (
                      <p className="text-[11px] text-slate-400">
                        Submit songs to receive ranked DJ suggestions explaining compatibility, missing metadata, and recommended transition techniques.
                      </p>
                    ) : (
                      <div className="space-y-2">
                        {recommendations.map((rec) => {
                          const playabilityBadge = (() => {
                            if (rec.is_playable || rec.playability_label === 'playable_now') {
                              return {
                                cls: 'bg-emerald-500/20 border-emerald-500/40 text-emerald-300',
                                label: 'Playable Now',
                              };
                            }
                            if (rec.playability_label === 'processing') {
                              return {
                                cls: 'bg-sky-500/20 border-sky-500/40 text-sky-300',
                                label: 'Processing',
                              };
                            }
                            if (rec.playability_label === 'failed') {
                              return {
                                cls: 'bg-rose-500/20 border-rose-500/40 text-rose-300',
                                label: 'Audio Failed',
                              };
                            }
                            return {
                              cls: 'bg-amber-500/15 border-amber-500/35 text-amber-300',
                              label: 'Metadata Only • Not Playable',
                            };
                          })();

                          return (
                            <div
                              key={`${rec.category}-${rec.submission_id}`}
                              className="p-2.5 rounded-lg bg-black/50 border border-white/10 space-y-1.5"
                            >
                              <div className="flex items-center justify-between gap-2 flex-wrap">
                                <div className="flex items-center gap-1.5">
                                  <span className="px-1.5 py-0.5 rounded bg-teal-500/20 border border-teal-400/35 text-[9px] font-bold uppercase text-teal-200">
                                    {rec.category_label}
                                  </span>
                                  <span className={`px-1.5 py-0.5 rounded border text-[9px] font-bold ${playabilityBadge.cls}`}>
                                    {playabilityBadge.label}
                                  </span>
                                </div>
                                <span className="text-[10px] font-mono text-slate-300">
                                  Score <strong className="text-white">{rec.compatibility_score.toFixed(1)}</strong> &bull; Conf{' '}
                                  {Math.round(rec.confidence * 100)}%
                                </span>
                              </div>

                              <div>
                                <p className="text-xs font-bold text-white truncate">{rec.title}</p>
                                <p className="text-[10px] text-slate-400 truncate">{rec.artist}</p>
                              </div>

                              <div className="text-[10px] text-slate-300 space-y-0.5">
                                {rec.reasons.slice(0, 3).map((reason, idx) => (
                                  <p key={idx} className="leading-snug">
                                    &bull; {reason}
                                  </p>
                                ))}
                              </div>

                              <div className="text-[10px] bg-white/5 border border-white/10 rounded-md px-2 py-1 text-slate-200">
                                <span className="font-bold text-amber-300">Mix: {rec.technique_label}</span> — {rec.technique_reason}
                              </div>

                              <div className="flex flex-wrap gap-1 text-[9px] font-mono">
                                {rec.known_signals.map((sig) => (
                                  <span
                                    key={`k-${sig}`}
                                    className="px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/25"
                                  >
                                    ✓ {sig}
                                  </span>
                                ))}
                                {rec.missing_signals.map((sig) => (
                                  <span
                                    key={`m-${sig}`}
                                    className="px-1.5 py-0.5 rounded bg-zinc-800 text-slate-400 border border-white/10"
                                  >
                                    ? {sig}
                                  </span>
                                ))}
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>

                  {/* Real DJ Technique Execution Log (Honest report of actual WebAudio transitions) */}
                  <div className="bg-zinc-900/90 p-3.5 rounded-xl border border-white/10 space-y-2">
                    <div className="flex items-center justify-between">
                      <h4 className="text-xs font-bold text-sky-300 uppercase">
                        Executed Mix Technique Log ({techniqueLog.length})
                      </h4>
                      <span className="text-[9px] font-mono text-slate-400">Real DSP Telemetry</span>
                    </div>
                    {techniqueLog.length === 0 ? (
                      <p className="text-[11px] text-slate-400">
                        No automated transitions executed yet — techniques are logged here only when Web Audio genuinely performs a mix.
                      </p>
                    ) : (
                      <div className="space-y-1.5 max-h-40 overflow-y-auto pr-1">
                        {techniqueLog.slice(0, 8).map((entry) => (
                          <div
                            key={entry.id}
                            className="p-2 rounded-lg bg-black/50 border border-white/10 text-[10px] space-y-0.5"
                          >
                            <div className="flex items-center justify-between gap-2">
                              <span className="font-bold text-amber-300">
                                L{entry.djLevel} • {entry.techniqueLabel}
                              </span>
                              <span className="font-mono text-slate-400">
                                {entry.durationSec.toFixed(1)}s &bull; &times;{entry.tempoMatchRatio.toFixed(2)} &bull;{' '}
                                {entry.gainMatchDb >= 0 ? `+${entry.gainMatchDb.toFixed(1)}` : entry.gainMatchDb.toFixed(1)} dB
                              </span>
                            </div>
                            <p className="text-slate-200 truncate">
                              Deck {String.fromCharCode(65 + entry.fromDeckId)} ({entry.outgoingTitle}) &rarr; Deck{' '}
                              {String.fromCharCode(65 + entry.toDeckId)} ({entry.incomingTitle})
                            </p>
                            <p className="text-slate-400">
                              {entry.fallbackUsed && (
                                <span className="text-amber-300 font-bold mr-1">[Safe Fallback]</span>
                              )}
                              {entry.reason}
                            </p>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Local Temp Cache & Metadata-Only Sync Telemetry */}
                  <div className="grid grid-cols-2 gap-3">
                    <div className="bg-zinc-900/90 p-3.5 rounded-xl border border-white/10 space-y-1">
                      <div className="text-[10px] font-bold text-amber-300 uppercase flex items-center gap-1.5">
                        <HardDrive className="w-3.5 h-3.5" />
                        <span>Local Temp Cache</span>
                      </div>
                      <div className="text-lg font-black text-white font-mono">
                        {cacheStats.track_count} / {cacheStats.max_tracks} <span className="text-xs font-normal text-slate-400">tracks</span>
                      </div>
                      <p className="text-[10px] text-slate-400">
                        {cacheStats.locked_tracks} priority-locked &bull; Auto-purged on party close
                      </p>
                    </div>

                    <div className="bg-zinc-900/90 p-3.5 rounded-xl border border-white/10 space-y-1">
                      <div className="text-[10px] font-bold text-sky-300 uppercase flex items-center gap-1.5">
                        <CloudUpload className="w-3.5 h-3.5" />
                        <span>Metadata-Only Sync</span>
                      </div>
                      <div className="text-lg font-black text-white font-mono">
                        {syncOverview.synced_count} <span className="text-xs font-normal text-slate-400">synced</span>
                      </div>
                      <p className="text-[10px] text-slate-400">
                        {syncOverview.pending_count} offline queued &bull; Zero audio uploaded
                      </p>
                    </div>
                  </div>

                  {/* 4-Deck Web Audio Mixer Strip (2 Decks for Levels 1-2, 4 Decks for Levels 3-4) */}
                  <div className="bg-zinc-900/90 p-4 rounded-xl border border-white/10 space-y-3">
                    <div className="flex items-center justify-between">
                      <h4 className="text-xs font-bold text-amber-300 uppercase">
                        Web Audio Engine ({exposedDecks} Active / {internalDeckCapacity}-Deck Architecture)
                      </h4>
                      <span className="text-[10px] font-mono text-emerald-300">Auto DJ Enabled</span>
                    </div>

                    <div className="grid grid-cols-2 gap-3">
                      {Array.from({ length: exposedDecks }).map((_, deckId) => (
                        <div key={deckId} className="bg-black/50 p-3 rounded-xl border border-white/10 space-y-2">
                          <div className="text-xs font-bold text-white flex justify-between">
                            <span>Deck {String.fromCharCode(65 + deckId)}</span>
                            <span className="text-[10px] font-mono text-slate-400">
                              {deckId % 2 === 0 ? 'XF Left' : 'XF Right'}
                            </span>
                          </div>
                          <div>
                            <label className="text-[10px] text-slate-400">Low EQ ({decks[deckId].eqLow} dB)</label>
                            <input
                              type="range"
                              min="-24"
                              max="6"
                              value={decks[deckId].eqLow}
                              onChange={(e) =>
                                updateDeckEq(deckId, Number(e.target.value), decks[deckId].eqMid, decks[deckId].eqHigh)
                              }
                              className="w-full accent-rose-400"
                            />
                          </div>
                          <div>
                            <label className="text-[10px] text-slate-400">High EQ ({decks[deckId].eqHigh} dB)</label>
                            <input
                              type="range"
                              min="-24"
                              max="6"
                              value={decks[deckId].eqHigh}
                              onChange={(e) =>
                                updateDeckEq(deckId, decks[deckId].eqLow, decks[deckId].eqMid, Number(e.target.value))
                              }
                              className="w-full accent-teal-400"
                            />
                          </div>
                        </div>
                      ))}
                    </div>

                    <div>
                      <label className="text-[10px] font-bold text-amber-300 uppercase block mb-1">
                        Master Crossfader ({activeMoodProfile.transition_style})
                      </label>
                      <input
                        type="range"
                        min="-1"
                        max="1"
                        step="0.05"
                        value={crossfader}
                        onChange={(e) => setCrossfaderPos(Number(e.target.value))}
                        className="w-full accent-amber-400"
                      />
                    </div>
                  </div>
                </div>
              )}

              {/* TAB 4: 15 VIDEO FX LOOPS */}
              {activeDrawerTab === 'fx' && (
                <div className="space-y-3">
                  <div className="flex items-center justify-between pb-2 border-b border-white/10">
                    <h4 className="text-xs font-bold text-amber-300 uppercase">15 Pre-Saved Stage Visual FX</h4>
                    <label className="flex items-center space-x-2 text-xs text-slate-300 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={autoCycleFx}
                        onChange={(e) => setAutoCycleFx(e.target.checked)}
                        className="accent-amber-400"
                      />
                      <span>Auto-Cycle (25s)</span>
                    </label>
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    {FX_NAMES.map((name, idx) => (
                      <button
                        key={idx}
                        onClick={() => {
                          setCurrentFxIndex(idx);
                          setAutoCycleFx(false);
                        }}
                        className={`p-2.5 rounded-lg border text-left text-xs font-bold transition-all flex items-center justify-between ${
                          currentFxIndex === idx
                            ? 'bg-amber-500/20 border-amber-400 text-amber-300 shadow-md'
                            : 'bg-zinc-900 border-white/5 text-slate-400 hover:text-white'
                        }`}
                      >
                        <span className="truncate">FX {idx + 1}: {name}</span>
                        {currentFxIndex === idx && (
                          <Sparkles className="w-3 h-3 text-amber-400 flex-shrink-0 ml-1" />
                        )}
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* Drawer Footer */}
            <div className="pt-4 mt-4 border-t border-white/10 flex items-center justify-between">
              <button
                onClick={async () => {
                  await checkHelperHealth();
                }}
                className="px-3.5 py-2 bg-zinc-900 hover:bg-zinc-800 border border-white/10 rounded-xl text-xs font-bold text-slate-300 flex items-center space-x-1.5"
              >
                <RefreshCw className="w-3.5 h-3.5 text-sky-400" />
                <span>Refresh Local Helper</span>
              </button>

              <div className="flex items-center gap-2">
                {isPartyLive && (
                  <button
                    onClick={handleCloseRoom}
                    className="px-3.5 py-2 bg-red-950 hover:bg-red-900 border border-red-700 rounded-xl text-xs font-bold text-red-200 flex items-center space-x-1.5"
                  >
                    <AlertTriangle className="w-3.5 h-3.5 text-red-400" />
                    <span>Close Party</span>
                  </button>
                )}
                <button
                  onClick={toggleFullscreen}
                  className="p-2 bg-zinc-900 hover:bg-zinc-800 border border-white/10 rounded-xl text-slate-300"
                  title="Toggle Fullscreen (F)"
                >
                  {isFullscreen ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
