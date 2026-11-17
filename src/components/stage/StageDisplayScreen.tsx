import React, { useState, useEffect, useRef } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { StageVisualFxCanvas, FX_NAMES } from './StageVisualFxCanvas';
import { AnimatedDjCharacter } from './AnimatedDjCharacter';
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
  Server,
  RefreshCw,
  ExternalLink,
  QrCode,
  CheckCircle2,
  AlertCircle,
  Copy,
} from 'lucide-react';

const StageOscilloscopeWaveform: React.FC<{ isLive: boolean; bpm: number }> = ({ isLive, bpm }) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let animId: number;
    let waveTime = 0;

    const draw = () => {
      const w = canvas.width;
      const h = canvas.height;
      ctx.clearRect(0, 0, w, h);

      waveTime += (isLive ? 0.045 : 0.012) * (bpm / 128);

      ctx.save();
      ctx.beginPath();
      ctx.lineWidth = 2.5;
      ctx.strokeStyle = isLive ? '#2dd4bf' : 'rgba(148, 163, 184, 0.35)';
      ctx.shadowColor = isLive ? '#14b8a6' : 'transparent';
      ctx.shadowBlur = isLive ? 12 : 0;

      const sliceWidth = w / 64;
      let x = 0;

      for (let i = 0; i < 64; i++) {
        const envelope = Math.sin((i / 63) * Math.PI);
        const amp = isLive ? 14 * envelope : 2 * envelope;
        const y = h / 2 + Math.sin(waveTime * 2.5 + i * 0.22) * amp;

        if (i === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
        x += sliceWidth;
      }

      ctx.stroke();

      if (isLive) {
        ctx.beginPath();
        ctx.lineWidth = 1.0;
        ctx.strokeStyle = 'rgba(45, 212, 191, 0.4)';
        ctx.shadowBlur = 6;
        x = 0;
        for (let i = 0; i < 64; i++) {
          const envelope = Math.sin((i / 63) * Math.PI);
          const y = h / 2 - Math.sin(waveTime * 2.5 + i * 0.22) * (8 * envelope);
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
  }, [isLive, bpm]);

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
    apiBaseUrl,
    backendConnected,
    backendVersion,
    backendError,
    wsConnected,
    cloudParty,
    partyConfig,
    guestUrl,
    qrSvg,
    registeredSources,
    songRequests,
    moodProfiles,
    setApiBaseUrl,
    checkBackendHealth,
    createCloudParty,
    startCloudParty,
    endCloudParty,
    clearActiveParty,
    registerYouTubeSource,
    approveSongRequest,
    rejectSongRequest,
    setDjLevel,
    setPartyMood,
  } = useDhunStore();

  const [currentFxIndex, setCurrentFxIndex] = useState(0);
  const [autoCycleFx, setAutoCycleFx] = useState(true);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [showHostDrawer, setShowHostDrawer] = useState(false);
  const [activeDrawerTab, setActiveDrawerTab] = useState<'party' | 'sources' | 'requests' | 'fx'>('party');

  // Form States
  const [apiUrlDraft, setApiUrlDraft] = useState(apiBaseUrl);
  const [partyNameInput, setPartyNameInput] = useState('DHUN Celebration Party');
  const [selectedDjType, setSelectedDjType] = useState('Smart');
  const [selectedMood, setSelectedMood] = useState('house_party');
  const [youtubeUrlInput, setYoutubeUrlInput] = useState('');

  // Action feedback states
  const [isBusy, setIsBusy] = useState(false);
  const [sourceFeedback, setSourceFeedback] = useState<{ type: 'ok' | 'err'; text: string } | null>(null);
  const [partyFeedback, setPartyFeedback] = useState<{ type: 'ok' | 'err'; text: string } | null>(null);
  const [copiedUrl, setCopiedUrl] = useState(false);

  // Sync local dropdowns with real persisted party config when loaded
  useEffect(() => {
    if (partyConfig?.primary_mood_id) {
      setSelectedMood(partyConfig.primary_mood_id);
    }
    if (partyConfig?.dj_level) {
      const map: Record<string, string> = {
        hands_off: 'Pro',
        balanced: 'Smart',
        curated: 'Advanced',
        manual: 'Creative',
      };
      setSelectedDjType(map[partyConfig.dj_level] || 'Smart');
    }
  }, [partyConfig]);

  useEffect(() => {
    setApiUrlDraft(apiBaseUrl);
  }, [apiBaseUrl]);

  const isPartyLive = cloudParty?.status === 'live';
  const activeMoodProfile = moodProfiles.find((m) => m.id === selectedMood) || moodProfiles[0];
  const stageBpm = activeMoodProfile ? Math.round((activeMoodProfile.min_bpm + activeMoodProfile.max_bpm) / 2) : 124;
  const stageEnergy = activeMoodProfile ? activeMoodProfile.target_energy : 0.75;

  // Honest lead source display (never hardcoded "Chaiyya Chaiyya")
  const leadSource = registeredSources.length > 0 ? registeredSources[0] : null;
  const pendingRequestsCount = songRequests.filter((r) => r.status === 'pending').length;

  const toggleFullscreen = () => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen();
      setIsFullscreen(true);
    } else {
      document.exitFullscreen();
      setIsFullscreen(false);
    }
  };

  // Keyboard shortcuts: 'M' for Host Console, 'F' for Fullscreen, 'Escape' to close drawer
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (['INPUT', 'SELECT', 'TEXTAREA'].includes((e.target as HTMLElement)?.tagName)) {
        return;
      }
      if (e.key === 'm' || e.key === 'M') {
        setShowHostDrawer((prev) => !prev);
      }
      if (e.key === 'Escape') {
        setShowHostDrawer(false);
      }
      if (e.key === 'f' || e.key === 'F') {
        toggleFullscreen();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Auto-cycle visual FX loops every 25 seconds when enabled
  useEffect(() => {
    if (!autoCycleFx) return;
    const fxTimer = setInterval(() => {
      setCurrentFxIndex((idx) => (idx + 1) % 15);
    }, 25000);
    return () => clearInterval(fxTimer);
  }, [autoCycleFx]);

  const djTypes = [
    { id: 'Pro', level: 1, key: 'hands_off', name: 'DJ TYPE: Hands-Off Auto' },
    { id: 'Smart', level: 2, key: 'balanced', name: 'DJ TYPE: Balanced Smart' },
    { id: 'Advanced', level: 3, key: 'curated', name: 'DJ TYPE: Curated Control' },
    { id: 'Creative', level: 4, key: 'manual', name: 'DJ TYPE: Manual Assist' },
  ];

  const handleCreateParty = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!partyNameInput.trim()) return;
    setIsBusy(true);
    setPartyFeedback(null);
    try {
      const djObj = djTypes.find((d) => d.id === selectedDjType) || djTypes[1];
      await createCloudParty(partyNameInput.trim(), selectedMood, djObj.key);
      setPartyFeedback({
        type: 'ok',
        text: 'Cloud party created! Register YouTube links and press "Go Live" to start.',
      });
    } catch (err: any) {
      setPartyFeedback({
        type: 'err',
        text: err?.message || 'Failed to create party. Verify Cloud Backend connection.',
      });
    } finally {
      setIsBusy(false);
    }
  };

  const handleStartParty = async () => {
    setIsBusy(true);
    setPartyFeedback(null);
    try {
      await startCloudParty();
      setPartyFeedback({ type: 'ok', text: 'Party is now LIVE! Guests can scan the QR code to request songs.' });
    } catch (err: any) {
      setPartyFeedback({ type: 'err', text: err?.message || 'Failed to start party.' });
    } finally {
      setIsBusy(false);
    }
  };

  const handleEndParty = async () => {
    setIsBusy(true);
    setPartyFeedback(null);
    try {
      await endCloudParty();
      setPartyFeedback({ type: 'ok', text: 'Party session ended gracefully.' });
    } catch (err: any) {
      setPartyFeedback({ type: 'err', text: err?.message || 'Failed to end party.' });
    } finally {
      setIsBusy(false);
    }
  };

  const handleRegisterSource = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!youtubeUrlInput.trim()) return;
    setIsBusy(true);
    setSourceFeedback(null);
    try {
      const added = await registerYouTubeSource(youtubeUrlInput.trim());
      setSourceFeedback({
        type: 'ok',
        text: `Registered "${added.title}" (${added.playability_status}).`,
      });
      setYoutubeUrlInput('');
    } catch (err: any) {
      setSourceFeedback({
        type: 'err',
        text: err?.message || 'Could not validate or register YouTube URL.',
      });
    } finally {
      setIsBusy(false);
    }
  };

  const handleSaveApiUrl = async () => {
    setIsBusy(true);
    await setApiBaseUrl(apiUrlDraft);
    setIsBusy(false);
  };

  const handleCopyGuestUrl = () => {
    if (!guestUrl) return;
    navigator.clipboard.writeText(guestUrl);
    setCopiedUrl(true);
    setTimeout(() => setCopiedUrl(false), 2000);
  };

  // Honest connection status text and badge color
  const connectionBadge = (() => {
    if (!backendConnected) {
      return {
        dot: 'bg-rose-500',
        textClass: 'text-rose-400',
        label: 'Backend Offline',
      };
    }
    if (isPartyLive) {
      return {
        dot: 'bg-emerald-400 animate-ping',
        textClass: 'text-emerald-400',
        label: wsConnected ? 'LIVE • Realtime Sync' : 'LIVE • Cloud Connected',
      };
    }
    if (cloudParty) {
      return {
        dot: 'bg-amber-400 animate-pulse',
        textClass: 'text-amber-300',
        label: `Party ${cloudParty.status.toUpperCase()} • Ready`,
      };
    }
    return {
      dot: 'bg-sky-400',
      textClass: 'text-sky-300',
      label: 'Cloud Connected • Setup',
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

        {/* Central Golden Devanagari Title & Waveform in LED Screen */}
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
            {cloudParty ? cloudParty.party_name : 'DHUN — PARTY REQUEST PLATFORM'}
          </p>

          <StageOscilloscopeWaveform isLive={isPartyLive} bpm={stageBpm} />
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
          moodId={selectedMood}
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

      {/* 5A. TOP-LEFT: HONEST CLOUD STATUS, DJ TYPE, MOOD DROPDOWNS */}
      <div className="absolute top-5 left-5 z-40 flex flex-col space-y-2 pointer-events-auto">
        {/* Cloud Session Status Pill */}
        <div
          onClick={() => setShowHostDrawer(true)}
          title="Click to open Host DJ Console & Cloud Party Settings (or press 'M')"
          className="inline-flex flex-col bg-zinc-950/75 backdrop-blur-xl border border-white/10 px-3.5 py-2 rounded-xl shadow-2xl cursor-pointer hover:border-amber-400/40 transition-all group"
        >
          <span className="text-[11px] font-bold text-slate-300 flex items-center justify-between gap-3">
            <span>Cloud Party Status</span>
            <span className="text-[9px] font-mono text-amber-300/80 uppercase">Press M</span>
          </span>
          <div className="flex items-center space-x-1.5 mt-0.5">
            <span className={`w-2 h-2 rounded-full ${connectionBadge.dot}`} />
            <span className={`text-[11px] font-semibold ${connectionBadge.textClass}`}>
              {connectionBadge.label}
            </span>
          </div>
        </div>

        {/* DJ TYPE Dropdown */}
        <div className="relative">
          <select
            value={selectedDjType}
            onChange={async (e) => {
              const val = e.target.value;
              setSelectedDjType(val);
              const found = djTypes.find((d) => d.id === val);
              if (found) {
                await setDjLevel(found.level);
              }
            }}
            className="w-56 appearance-none bg-zinc-950/75 backdrop-blur-xl border border-white/10 hover:border-amber-400/50 text-white text-xs font-bold px-3.5 py-2 rounded-xl focus:outline-none focus:ring-1 focus:ring-amber-400 cursor-pointer shadow-xl transition-all"
          >
            {djTypes.map((t) => (
              <option key={t.id} value={t.id} className="bg-zinc-950 text-white">
                {t.name}
              </option>
            ))}
          </select>
          <div className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-slate-400 text-xs">
            ▼
          </div>
        </div>

        {/* MOOD Dropdown (All 14 Curated Moods) */}
        <div className="relative">
          <select
            value={selectedMood}
            onChange={async (e) => {
              const moodVal = e.target.value;
              setSelectedMood(moodVal);
              await setPartyMood(moodVal);
            }}
            className="w-56 appearance-none bg-zinc-950/75 backdrop-blur-xl border border-white/10 hover:border-amber-400/50 text-white text-xs font-bold px-3.5 py-2 rounded-xl focus:outline-none focus:ring-1 focus:ring-amber-400 cursor-pointer shadow-xl transition-all"
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

        {/* Quick Launch Host Console CTA if no live party */}
        {!isPartyLive && (
          <button
            onClick={() => {
              setActiveDrawerTab('party');
              setShowHostDrawer(true);
            }}
            className="flex items-center justify-center space-x-2 bg-gradient-to-r from-amber-500 to-rose-600 hover:from-amber-400 hover:to-rose-500 text-white text-xs font-black px-3.5 py-2 rounded-xl shadow-lg shadow-rose-950/60 transition-all"
          >
            <Radio className="w-3.5 h-3.5" />
            <span>{cloudParty ? 'Manage & Go Live' : 'Setup Cloud Party'}</span>
          </button>
        )}
      </div>

      {/* 5B. BOTTOM-LEFT: HONEST NOW PLAYING / DECK STANDBY CARD */}
      <div className="absolute bottom-6 left-6 z-40 w-72 md:w-80 bg-zinc-950/80 backdrop-blur-2xl border border-white/15 p-4 rounded-3xl shadow-[0_20px_50px_rgba(0,0,0,0.9)] pointer-events-auto">
        <div className="flex items-center justify-between text-xs text-slate-400 mb-2 font-bold">
          <span className="text-white flex items-center space-x-1.5">
            <span
              className={`w-2 h-2 rounded-full ${
                isPartyLive ? 'bg-emerald-400 animate-pulse' : 'bg-slate-500'
              }`}
            />
            <span>{isPartyLive ? 'Active Stage Queue' : 'Party Standby'}</span>
          </span>
          <button
            onClick={() => {
              setActiveDrawerTab('sources');
              setShowHostDrawer(true);
            }}
            className="px-2 py-0.5 rounded-md bg-white/10 hover:bg-white/20 text-[10px] font-bold text-amber-300 transition-colors"
            title="Manage YouTube Music Sources"
          >
            Sources ({registeredSources.length})
          </button>
        </div>

        {leadSource ? (
          <>
            <div className="flex items-center space-x-3 mb-2.5">
              {leadSource.thumbnail_url ? (
                <img
                  src={leadSource.thumbnail_url}
                  alt={leadSource.title}
                  className="w-11 h-11 rounded-xl object-cover shadow-lg border border-white/10"
                />
              ) : (
                <div className="w-11 h-11 rounded-xl bg-gradient-to-br from-red-600 via-orange-600 to-amber-500 flex items-center justify-center shadow-lg">
                  <Disc3 className={`w-6 h-6 text-white ${isPartyLive ? 'animate-[spin_4s_linear_infinite]' : ''}`} />
                </div>
              )}
              <div className="flex-1 min-w-0">
                <h4 className="text-sm font-extrabold text-white truncate drop-shadow">
                  {leadSource.title}
                </h4>
                <p className="text-xs text-slate-300 truncate">{leadSource.artist}</p>
              </div>
            </div>

            <div className="bg-amber-500/10 border border-amber-400/25 rounded-xl px-2.5 py-1.5 mb-2.5">
              <p className="text-[10px] text-amber-200 font-medium leading-tight">
                Official YouTube Reference Registered &bull; Playability:{' '}
                <span className="font-mono font-bold text-amber-300">{leadSource.playability_status}</span>
                <br />
                Phase 2 will enable embedded YouTube Player audio playback.
              </p>
            </div>
          </>
        ) : (
          <div className="py-3 text-center space-y-1.5 mb-2">
            <div className="w-10 h-10 mx-auto rounded-xl bg-white/5 border border-white/10 flex items-center justify-center">
              <Music className="w-5 h-5 text-slate-400" />
            </div>
            <h4 className="text-sm font-bold text-slate-200">No Active Track Queued</h4>
            <p className="text-[11px] text-slate-400 leading-snug">
              {cloudParty
                ? 'Add official YouTube links in Host Console to populate the party catalog.'
                : 'Create a Cloud Party and register official YouTube tracks to begin.'}
            </p>
          </div>
        )}

        {/* Host Quick Action Controls */}
        <div className="flex items-center justify-between pt-2 border-t border-white/10 gap-2">
          <button
            onClick={() => {
              setActiveDrawerTab('sources');
              setShowHostDrawer(true);
            }}
            className="flex-1 py-1.5 rounded-xl bg-white/10 hover:bg-white/15 text-[11px] font-bold text-slate-200 flex items-center justify-center space-x-1.5 transition-colors"
          >
            <Link2 className="w-3.5 h-3.5 text-amber-400" />
            <span>Add YouTube Link</span>
          </button>

          {cloudParty && !isPartyLive && cloudParty.status !== 'ended' && (
            <button
              onClick={handleStartParty}
              disabled={isBusy}
              className="px-3 py-1.5 rounded-xl bg-teal-400 hover:bg-teal-300 text-black text-[11px] font-black flex items-center space-x-1 shadow-lg shadow-teal-400/30 transition-all"
            >
              <Play className="w-3.5 h-3.5 fill-current" />
              <span>Go Live</span>
            </button>
          )}

          {isPartyLive && (
            <button
              onClick={handleEndParty}
              disabled={isBusy}
              className="px-3 py-1.5 rounded-xl bg-rose-600/80 hover:bg-rose-500 text-white text-[11px] font-bold flex items-center space-x-1 transition-all"
            >
              <Square className="w-3 h-3 fill-current" />
              <span>End</span>
            </button>
          )}
        </div>
      </div>

      {/* 5C. BOTTOM-RIGHT: HONEST SCAN TO REQUEST FROSTED GLASS QR CARD */}
      <div className="absolute bottom-6 right-6 z-40 w-64 md:w-72 bg-zinc-950/80 backdrop-blur-2xl border border-white/15 p-4 rounded-3xl shadow-[0_20px_50px_rgba(0,0,0,0.9)] pointer-events-auto flex flex-col items-center text-center">
        <h4 className="text-xs font-bold tracking-wider text-slate-200 uppercase mb-2.5 flex items-center gap-1.5">
          <QrCode className="w-3.5 h-3.5 text-amber-400" />
          <span>Guest Request Portal</span>
        </h4>

        {cloudParty && qrSvg ? (
          <>
            <div
              className="w-32 h-32 md:w-36 md:h-36 bg-white p-2 rounded-2xl shadow-inner flex items-center justify-center mb-2.5 overflow-hidden [&>svg]:w-full [&>svg]:h-full"
              dangerouslySetInnerHTML={{ __html: qrSvg }}
            />
            <p className="text-xs font-black tracking-wide text-white uppercase drop-shadow">
              SCAN TO REQUEST
            </p>
            <p className="text-[11px] font-bold text-amber-300 mt-0.5">
              स्कैन कर गाना मांगें
            </p>
            <button
              onClick={handleCopyGuestUrl}
              className="mt-1.5 px-2.5 py-1 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 text-[10px] font-mono text-slate-300 flex items-center gap-1.5 max-w-full"
              title="Copy Guest Join URL"
            >
              <span className="truncate">{guestUrl?.replace(/^https?:\/\//, '')}</span>
              {copiedUrl ? <Check className="w-3 h-3 text-emerald-400 shrink-0" /> : <Copy className="w-3 h-3 shrink-0" />}
            </button>
          </>
        ) : (
          <div className="w-full py-5 px-3 rounded-2xl bg-black/50 border border-dashed border-white/15 flex flex-col items-center justify-center space-y-2 mb-1">
            <QrCode className="w-10 h-10 text-slate-600" />
            <p className="text-xs font-bold text-slate-300">QR Inactive</p>
            <p className="text-[10px] text-slate-400 leading-relaxed">
              Create &amp; connect a live Cloud Party to generate an authentic Guest QR code.
            </p>
            <button
              onClick={() => {
                setActiveDrawerTab('party');
                setShowHostDrawer(true);
              }}
              className="mt-1 px-3 py-1.5 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 border border-amber-400/40 text-amber-300 text-[11px] font-bold transition-colors"
            >
              Create Party
            </button>
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
                    Web Application &bull; Cloud Party Foundation (Phase 1)
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

            {/* Tab Navigation */}
            <div className="flex space-x-1.5 my-4">
              <button
                onClick={() => setActiveDrawerTab('party')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'party'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400 hover:text-white'
                }`}
              >
                Party &amp; Cloud
              </button>
              <button
                onClick={() => setActiveDrawerTab('sources')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'sources'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400 hover:text-white'
                }`}
              >
                Sources ({registeredSources.length})
              </button>
              <button
                onClick={() => setActiveDrawerTab('requests')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'requests'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400 hover:text-white'
                }`}
              >
                Requests ({pendingRequestsCount})
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

            {/* Tab Content */}
            <div className="flex-1 overflow-y-auto space-y-4">
              {/* TAB 1: PARTY & CLOUD BACKEND */}
              {activeDrawerTab === 'party' && (
                <div className="space-y-4">
                  {/* Cloud Backend Connection Configuration */}
                  <div className="bg-zinc-900/90 p-4 rounded-xl border border-white/10 space-y-2.5">
                    <div className="flex items-center justify-between">
                      <h4 className="text-xs font-bold text-sky-300 uppercase flex items-center gap-1.5">
                        <Server className="w-3.5 h-3.5" />
                        <span>Cloud Backend Endpoint</span>
                      </h4>
                      <span
                        className={`text-[10px] font-bold px-2 py-0.5 rounded-full ${
                          backendConnected
                            ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                            : 'bg-rose-500/20 text-rose-300 border border-rose-500/40'
                        }`}
                      >
                        {backendConnected ? `Connected v${backendVersion || '1.0'}` : 'Offline'}
                      </span>
                    </div>

                    <div className="flex gap-2">
                      <input
                        type="url"
                        value={apiUrlDraft}
                        onChange={(e) => setApiUrlDraft(e.target.value)}
                        placeholder="https://your-dhun-server.onrender.com"
                        className="flex-1 bg-black/60 border border-white/15 px-3 py-1.5 text-xs rounded-lg text-white font-mono outline-none focus:border-sky-400"
                      />
                      <button
                        onClick={handleSaveApiUrl}
                        disabled={isBusy}
                        className="px-3 py-1.5 bg-sky-500 hover:bg-sky-400 text-black font-extrabold text-xs rounded-lg flex items-center gap-1"
                      >
                        <RefreshCw className={`w-3.5 h-3.5 ${isBusy ? 'animate-spin' : ''}`} />
                        <span>Connect</span>
                      </button>
                    </div>

                    {!backendConnected && (
                      <p className="text-[11px] text-rose-400 flex items-center gap-1.5">
                        <AlertCircle className="w-3.5 h-3.5 shrink-0" />
                        <span>
                          {backendError || 'Backend unreachable.'} Deploy <code className="text-rose-200">dhun-server</code> (Render/Railway/Fly.io) or run locally on port 8080.
                        </span>
                      </p>
                    )}
                  </div>

                  {/* Party Lifecycle Panel */}
                  <div className="bg-zinc-900/90 p-4 rounded-xl border border-white/10 space-y-3">
                    <h4 className="text-xs font-bold text-amber-300 uppercase flex items-center gap-1.5">
                      <Radio className="w-3.5 h-3.5" />
                      <span>Party Session Management</span>
                    </h4>

                    {!cloudParty ? (
                      <form onSubmit={handleCreateParty} className="space-y-3">
                        <div>
                          <label className="text-[11px] text-slate-400 block mb-1">Party Name</label>
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
                          disabled={!backendConnected || isBusy || !partyNameInput.trim()}
                          className="w-full py-2.5 bg-amber-500 hover:bg-amber-400 disabled:opacity-40 text-black font-black text-xs rounded-xl transition-all"
                        >
                          {isBusy ? 'Creating Party...' : 'Create Cloud Party & Generate Host Token'}
                        </button>
                      </form>
                    ) : (
                      <div className="space-y-3">
                        <div className="p-3 rounded-lg bg-black/50 border border-white/10 space-y-1.5 text-xs">
                          <div className="flex justify-between">
                            <span className="text-slate-400">Party Name:</span>
                            <span className="font-bold text-white">{cloudParty.party_name}</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-slate-400">Party ID:</span>
                            <span className="font-mono text-amber-300">{cloudParty.id}</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-slate-400">Lifecycle Status:</span>
                            <span className="font-bold uppercase text-emerald-400">{cloudParty.status}</span>
                          </div>
                          <div className="flex justify-between">
                            <span className="text-slate-400">Realtime WebSocket:</span>
                            <span className={wsConnected ? 'text-emerald-400 font-bold' : 'text-amber-400'}>
                              {wsConnected ? 'Connected' : 'Reconnecting...'}
                            </span>
                          </div>
                        </div>

                        <div className="flex gap-2">
                          {cloudParty.status === 'created' && (
                            <button
                              onClick={handleStartParty}
                              disabled={isBusy}
                              className="flex-1 py-2 bg-emerald-500 hover:bg-emerald-400 text-black font-black text-xs rounded-xl flex items-center justify-center gap-1.5"
                            >
                              <Play className="w-3.5 h-3.5 fill-current" />
                              <span>Go Live</span>
                            </button>
                          )}

                          {cloudParty.status === 'live' && (
                            <button
                              onClick={handleEndParty}
                              disabled={isBusy}
                              className="flex-1 py-2 bg-rose-600 hover:bg-rose-500 text-white font-bold text-xs rounded-xl flex items-center justify-center gap-1.5"
                            >
                              <Square className="w-3.5 h-3.5 fill-current" />
                              <span>End Live Party</span>
                            </button>
                          )}

                          <button
                            onClick={clearActiveParty}
                            className="px-3 py-2 bg-zinc-800 hover:bg-zinc-700 text-slate-200 font-bold text-xs rounded-xl"
                          >
                            New Party
                          </button>
                        </div>
                      </div>
                    )}

                    {partyFeedback && (
                      <div
                        className={`text-xs px-3 py-2 rounded-lg border flex items-center gap-2 ${
                          partyFeedback.type === 'ok'
                            ? 'bg-emerald-950/50 border-emerald-500/40 text-emerald-300'
                            : 'bg-rose-950/50 border-rose-500/40 text-rose-300'
                        }`}
                      >
                        {partyFeedback.type === 'ok' ? (
                          <CheckCircle2 className="w-4 h-4 shrink-0" />
                        ) : (
                          <AlertCircle className="w-4 h-4 shrink-0" />
                        )}
                        <span>{partyFeedback.text}</span>
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* TAB 2: OFFICIAL ONLINE MUSIC SOURCES (ZERO LOCAL IMPORT) */}
              {activeDrawerTab === 'sources' && (
                <div className="space-y-4">
                  <form
                    onSubmit={handleRegisterSource}
                    className="bg-zinc-900/90 p-4 rounded-xl border border-white/10 space-y-2.5"
                  >
                    <h4 className="text-xs font-bold text-amber-300 uppercase flex items-center gap-1.5">
                      <Link2 className="w-3.5 h-3.5" />
                      <span>Register YouTube / YouTube Music Track</span>
                    </h4>
                    <p className="text-[11px] text-slate-400">
                      Paste an official link from <code className="text-slate-300">youtube.com/watch</code>,{' '}
                      <code className="text-slate-300">youtu.be</code>, or{' '}
                      <code className="text-slate-300">music.youtube.com</code>. Official metadata is fetched via oEmbed.
                    </p>
                    <div className="flex gap-2">
                      <input
                        type="url"
                        value={youtubeUrlInput}
                        onChange={(e) => setYoutubeUrlInput(e.target.value)}
                        placeholder="https://www.youtube.com/watch?v=..."
                        disabled={!cloudParty || !backendConnected || isBusy}
                        className="flex-1 bg-black/60 border border-white/15 px-3 py-1.5 text-xs rounded-lg text-white font-mono outline-none focus:border-amber-400 disabled:opacity-40"
                      />
                      <button
                        type="submit"
                        disabled={!cloudParty || !backendConnected || isBusy || !youtubeUrlInput.trim()}
                        className="px-3.5 py-1.5 bg-amber-500 hover:bg-amber-400 disabled:opacity-40 text-black font-extrabold text-xs rounded-lg"
                      >
                        {isBusy ? 'Adding...' : 'Register'}
                      </button>
                    </div>

                    {!cloudParty && (
                      <p className="text-[11px] text-amber-400">
                        Create a Cloud Party in the &ldquo;Party &amp; Cloud&rdquo; tab first to register sources.
                      </p>
                    )}

                    {sourceFeedback && (
                      <div
                        className={`text-xs px-3 py-2 rounded-lg border flex items-center gap-2 ${
                          sourceFeedback.type === 'ok'
                            ? 'bg-emerald-950/50 border-emerald-500/40 text-emerald-300'
                            : 'bg-rose-950/50 border-rose-500/40 text-rose-300'
                        }`}
                      >
                        {sourceFeedback.type === 'ok' ? (
                          <CheckCircle2 className="w-4 h-4 shrink-0" />
                        ) : (
                          <AlertCircle className="w-4 h-4 shrink-0" />
                        )}
                        <span>{sourceFeedback.text}</span>
                      </div>
                    )}
                  </form>

                  {/* Registered Sources Catalog */}
                  <div>
                    <h4 className="text-xs font-bold text-slate-400 uppercase mb-2">
                      Registered Party Catalog ({registeredSources.length})
                    </h4>
                    {registeredSources.length === 0 ? (
                      <div className="p-6 text-center rounded-xl bg-zinc-900/50 border border-white/5 text-xs text-slate-500">
                        No online music sources registered yet. Paste a YouTube link above to populate the party catalog.
                      </div>
                    ) : (
                      <div className="space-y-2">
                        {registeredSources.map((src) => (
                          <div
                            key={src.id}
                            className="flex items-center justify-between gap-3 p-2.5 bg-zinc-900/70 rounded-xl border border-white/10"
                          >
                            <div className="flex items-center gap-3 min-w-0">
                              {src.thumbnail_url ? (
                                <img
                                  src={src.thumbnail_url}
                                  alt={src.title}
                                  className="w-11 h-8 rounded-md object-cover bg-black shrink-0"
                                />
                              ) : (
                                <div className="w-11 h-8 rounded-md bg-zinc-800 flex items-center justify-center shrink-0">
                                  <Music className="w-4 h-4 text-slate-500" />
                                </div>
                              )}
                              <div className="min-w-0">
                                <p className="text-xs font-bold text-white truncate">{src.title}</p>
                                <p className="text-[11px] text-slate-400 truncate">
                                  {src.artist} &bull; <span className="font-mono">{src.provider_video_id}</span>
                                </p>
                              </div>
                            </div>

                            <div className="flex items-center gap-1.5 shrink-0">
                              <span className="px-1.5 py-0.5 rounded bg-red-500/20 border border-red-500/30 text-[9px] font-bold text-red-300 uppercase">
                                {src.provider === 'youtube_music' ? 'YT Music' : 'YouTube'}
                              </span>
                              <span
                                className="px-1.5 py-0.5 rounded bg-amber-500/15 border border-amber-500/30 text-[9px] font-mono font-bold text-amber-300"
                                title={src.error_reason || 'Prepared for Phase 2 YouTube IFrame Player'}
                              >
                                {src.playability_status}
                              </span>
                              <a
                                href={src.original_url}
                                target="_blank"
                                rel="noreferrer"
                                className="p-1 text-slate-400 hover:text-white"
                                title="Open on YouTube"
                              >
                                <ExternalLink className="w-3.5 h-3.5" />
                              </a>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* TAB 3: LIVE GUEST REQUESTS */}
              {activeDrawerTab === 'requests' && (
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <h4 className="text-xs font-bold text-amber-300 uppercase">Live Guest Song Requests</h4>
                    <span className="text-[10px] font-mono text-slate-400">
                      {songRequests.length} total
                    </span>
                  </div>

                  {songRequests.length === 0 ? (
                    <div className="p-6 text-center rounded-xl bg-zinc-900/50 border border-white/5 text-xs text-slate-400">
                      No requests yet. Once the party is live, guests can scan the QR code to request songs from your registered YouTube catalog!
                    </div>
                  ) : (
                    songRequests.map((req) => (
                      <div
                        key={req.id}
                        className="p-3 bg-zinc-900 rounded-xl border border-white/10 flex items-center justify-between gap-3"
                      >
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <p className="text-xs font-bold text-white truncate">
                              {req.source_title || 'Requested Song'}
                            </p>
                            <span
                              className={`text-[9px] font-bold uppercase px-1.5 py-0.5 rounded ${
                                req.status === 'approved'
                                  ? 'bg-emerald-500/20 text-emerald-300'
                                  : req.status === 'rejected'
                                  ? 'bg-rose-500/20 text-rose-300'
                                  : 'bg-amber-500/20 text-amber-300'
                              }`}
                            >
                              {req.status}
                            </span>
                          </div>
                          <p className="text-[11px] text-slate-400 truncate mt-0.5">
                            {req.source_artist} &bull; by{' '}
                            <span className="text-amber-300">{req.guest_name || 'Guest'}</span>
                          </p>
                          {req.custom_note && (
                            <p className="text-[10px] italic text-slate-300 mt-1">
                              &ldquo;{req.custom_note}&rdquo;
                            </p>
                          )}
                        </div>

                        {req.status === 'pending' && (
                          <div className="flex space-x-1.5 shrink-0">
                            <button
                              onClick={() => approveSongRequest(req.id)}
                              className="p-1.5 bg-emerald-600/30 hover:bg-emerald-600 border border-emerald-500 rounded text-emerald-300 hover:text-white transition-colors"
                              title="Approve Request"
                            >
                              <Check className="w-3.5 h-3.5" />
                            </button>
                            <button
                              onClick={() => rejectSongRequest(req.id, 'Declined by host')}
                              className="p-1.5 bg-rose-600/30 hover:bg-rose-600 border border-rose-500 rounded text-rose-300 hover:text-white transition-colors"
                              title="Reject Request"
                            >
                              <X className="w-3.5 h-3.5" />
                            </button>
                          </div>
                        )}
                      </div>
                    ))
                  )}
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
                  await checkBackendHealth();
                }}
                className="px-3.5 py-2 bg-zinc-900 hover:bg-zinc-800 border border-white/10 rounded-xl text-xs font-bold text-slate-300 flex items-center space-x-1.5"
              >
                <RefreshCw className="w-3.5 h-3.5 text-sky-400" />
                <span>Check Health</span>
              </button>

              <div className="flex items-center gap-2">
                {isPartyLive && (
                  <button
                    onClick={handleEndParty}
                    className="px-3.5 py-2 bg-red-950 hover:bg-red-900 border border-red-700 rounded-xl text-xs font-bold text-red-200 flex items-center space-x-1.5"
                  >
                    <AlertTriangle className="w-3.5 h-3.5 text-red-400" />
                    <span>End Party</span>
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
