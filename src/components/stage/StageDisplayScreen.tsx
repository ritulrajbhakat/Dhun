import React, { useState, useEffect, useRef } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { StageVisualFxCanvas, FX_NAMES } from './StageVisualFxCanvas';
import { AnimatedDjCharacter } from './AnimatedDjCharacter';
import { webAudioEngine } from '../../audio/WebAudioEngine';
import { generateQrSvg } from '../../utils/qr';
import { isTauri, apiDeckControl } from '../../api/ipc';
import {
  Play,
  Pause,
  SkipForward,
  SkipBack,
  Volume2,
  Sliders,
  Music,
  Disc3,
  AlertTriangle,
  FolderOpen,
  Check,
  X,
  Settings,
  Sparkles,
  Maximize2,
  Minimize2,
} from 'lucide-react';

const AudioOscilloscopeWaveform: React.FC<{ isPlaying: boolean; bpm: number }> = ({ isPlaying, bpm }) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let animId: number;
    let waveTime = 0;
    const waveData = new Uint8Array(128);

    const draw = () => {
      const w = canvas.width;
      const h = canvas.height;
      ctx.clearRect(0, 0, w, h);

      webAudioEngine.getWaveformData(waveData);
      waveTime += 0.05 * (bpm / 128);

      ctx.save();
      ctx.beginPath();
      ctx.lineWidth = 2.5;
      ctx.strokeStyle = '#2dd4bf'; // teal-400
      ctx.shadowColor = '#14b8a6';
      ctx.shadowBlur = 12;

      const sliceWidth = w / 64;
      let x = 0;

      for (let i = 0; i < 64; i++) {
        const v = isPlaying ? waveData[i * 2] / 128.0 : 1.0;
        const synthWobble = isPlaying ? Math.sin(waveTime * 3 + i * 0.18) * 10 : 0;
        const y = (v * h) / 2 + synthWobble;

        if (i === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
        x += sliceWidth;
      }

      ctx.stroke();

      // Mirror reflection glow
      ctx.beginPath();
      ctx.lineWidth = 1.0;
      ctx.strokeStyle = 'rgba(45, 212, 191, 0.45)';
      ctx.shadowBlur = 6;
      x = 0;
      for (let i = 0; i < 64; i++) {
        const v = isPlaying ? waveData[i * 2] / 128.0 : 1.0;
        const synthWobble = isPlaying ? Math.sin(waveTime * 3 + i * 0.18) * 5 : 0;
        const y = h / 2 - (v * h / 2 - h / 2) - synthWobble;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
        x += sliceWidth;
      }
      ctx.stroke();
      ctx.restore();

      animId = requestAnimationFrame(draw);
    };

    draw();
    return () => cancelAnimationFrame(animId);
  }, [isPlaying, bpm]);

  return (
    <canvas
      ref={canvasRef}
      width={720}
      height={80}
      className="w-full max-w-2xl h-10 md:h-14 pointer-events-none opacity-90 mt-2 drop-shadow-[0_0_12px_rgba(45,212,191,0.6)]"
    />
  );
};

export const StageDisplayScreen: React.FC = () => {
  const {
    partyStatus,
    telemetry,
    pollTelemetry,
    startParty,
    setDjLevel,
    setPartyMood,
    setMasterVolume,
    masterVolume,
    emergencyFade,
    toggleDeckPlay,
    forceNextRequest,
    guestRequests,
    startBrowserAudio,
    loadLocalAudioFile,
    approveRequest,
    rejectRequest,
    scanFolder,
  } = useDhunStore();

  const [currentFxIndex, setCurrentFxIndex] = useState(0);
  const [autoCycleFx, setAutoCycleFx] = useState(true);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [localIsPlaying, setLocalIsPlaying] = useState(true);
  const [localCurrentTimeSec, setLocalCurrentTimeSec] = useState(179); // 02:59 initial
  const [selectedDjType, setSelectedDjType] = useState('Advanced');
  const [selectedMood, setSelectedMood] = useState('house_party');
  const [audioActive, setAudioActive] = useState(false);
  const [showHostDrawer, setShowHostDrawer] = useState(false);
  const [activeDrawerTab, setActiveDrawerTab] = useState<'library' | 'mixer' | 'requests' | 'fx'>('library');
  const [folderPathInput, setFolderPathInput] = useState('C:/Users/ritul/Music/Celebration_Tracks');
  const [isScanning, setIsScanning] = useState(false);

  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const activeTrack = partyStatus.now_playing?.track;
  const activeAnalysis = partyStatus.now_playing?.analysis;

  const currentTitle = activeTrack?.title || 'Chaiyya Chaiyya';
  const currentArtist = activeTrack?.artist || 'Chaiyeyya';
  const currentBpm = activeAnalysis?.bpm || 128;
  const currentEnergy = activeAnalysis?.overall_energy || 0.88;

  // Real-time audio playhead synchronization from native engine telemetry
  const activeDeckTelem = telemetry?.decks?.find((d) => d.is_playing) || telemetry?.decks?.[0];
  const isPlaying = isTauri && activeDeckTelem ? activeDeckTelem.is_playing : localIsPlaying;
  const currentDurationSec = isTauri && activeDeckTelem && activeDeckTelem.duration_ms > 0
    ? Math.floor(activeDeckTelem.duration_ms / 1000)
    : (activeTrack?.duration_ms ? Math.floor(activeTrack.duration_ms / 1000) : 296);
  const currentTimeSec = isTauri && activeDeckTelem && activeDeckTelem.duration_ms > 0
    ? Math.min(currentDurationSec, Math.floor(activeDeckTelem.position_ms / 1000))
    : localCurrentTimeSec;

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  };

  const handleTogglePlay = async () => {
    if (!isTauri && !audioActive) {
      await startBrowserAudio();
      setAudioActive(true);
    }
    setLocalIsPlaying(!isPlaying);
    await toggleDeckPlay(0);
  };

  const handleNextTrack = async () => {
    if (partyStatus.next_track) {
      await forceNextRequest(partyStatus.next_track.track.id);
      setLocalCurrentTimeSec(0);
    }
  };

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      await loadLocalAudioFile(0, file);
      setAudioActive(true);
      setLocalIsPlaying(true);
    }
  };

  const toggleFullscreen = () => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen();
      setIsFullscreen(true);
    } else {
      document.exitFullscreen();
      setIsFullscreen(false);
    }
  };

  // 0. Periodic telemetry polling in native Tauri desktop runtime
  useEffect(() => {
    if (!isTauri) return;
    const telemTimer = setInterval(() => {
      pollTelemetry();
    }, 100);
    return () => clearInterval(telemTimer);
  }, [pollTelemetry]);

  // 1. User interaction listener to start WebAudio in browser demo mode
  useEffect(() => {
    if (isTauri) return;
    const unlockAudio = async () => {
      await startBrowserAudio();
      setAudioActive(true);
    };
    window.addEventListener('pointerdown', unlockAudio, { once: true });
    window.addEventListener('keydown', unlockAudio, { once: true });
    return () => {
      window.removeEventListener('pointerdown', unlockAudio);
      window.removeEventListener('keydown', unlockAudio);
    };
  }, [startBrowserAudio]);

  // 2. Keyboard shortcuts: 'M' for host console, 'Space' for play/pause, 'F' for fullscreen
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
      if (e.code === 'Space') {
        e.preventDefault();
        handleTogglePlay();
      }
      if (e.key === 'f' || e.key === 'F') {
        toggleFullscreen();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isPlaying, audioActive]);

  // 3. Auto-cycle FX loops every 25 seconds if enabled
  useEffect(() => {
    if (!autoCycleFx) return;
    const fxTimer = setInterval(() => {
      setCurrentFxIndex((idx) => (idx + 1) % 15);
    }, 25000);
    return () => clearInterval(fxTimer);
  }, [autoCycleFx]);

  // 4. Track playback time progress (browser demo mode only - Tauri is driven by audio clock)
  useEffect(() => {
    if (isTauri || !localIsPlaying) return;
    const progressTimer = setInterval(() => {
      setLocalCurrentTimeSec((t) => (t >= currentDurationSec ? 0 : t + 1));
    }, 1000);
    return () => clearInterval(progressTimer);
  }, [localIsPlaying, currentDurationSec]);

  // 4 DJ Levels
  const djTypes = [
    { id: 'Pro', name: 'DJ TYPE: Professional 2-Deck' },
    { id: 'Smart', name: 'DJ TYPE: Smart Mixing' },
    { id: 'Advanced', name: 'DJ TYPE: Advanced' },
    { id: 'Creative', name: 'DJ TYPE: Creative Tools' },
  ];

  // All 14 Indian & International Mood Presets
  const moodsList = [
    { id: 'house_party', name: 'MOOD: House Party' },
    { id: 'shaadi', name: 'MOOD: Shaadi' },
    { id: 'baraat', name: 'MOOD: Baraat' },
    { id: 'mehendi', name: 'MOOD: Mehendi' },
    { id: 'haldi', name: 'MOOD: Haldi' },
    { id: 'sangeet', name: 'MOOD: Sangeet' },
    { id: 'desi_club', name: 'MOOD: Desi Club' },
    { id: 'punjabi_tashan', name: 'MOOD: Punjabi Tashan' },
    { id: 'garba', name: 'MOOD: Garba' },
    { id: 'dandiya', name: 'MOOD: Dandiya' },
    { id: 'retro_bollywood', name: 'MOOD: Retro Bollywood' },
    { id: 'cocktail_night', name: 'MOOD: Cocktail Night' },
    { id: 'festival_mode', name: 'MOOD: Festival Mode' },
    { id: 'international_party', name: 'MOOD: International Party' },
  ];

  const progressPercent = Math.min(100, (currentTimeSec / Math.max(1, currentDurationSec)) * 100);

  // Single source of truth for guest mobile requests (derived strictly from backend guest_url)
  const rawGuestUrl = partyStatus.guest_url || (typeof window !== 'undefined' && window.location.hostname !== 'localhost' && window.location.hostname !== '127.0.0.1'
    ? `http://${window.location.hostname}:3000/`
    : 'http://192.168.1.15:3000/');

  const guestRequestUrl = rawGuestUrl.endsWith('/') ? `${rawGuestUrl}request` : `${rawGuestUrl}/request`;
  const displayedLanAddress = guestRequestUrl.replace(/^https?:\/\//, '');
  const qrSvgHtml = partyStatus.qr_svg || generateQrSvg(guestRequestUrl, 144);

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-black select-none font-sans text-white">
      {/* ============================================================ */}
      {/* LAYER 1: GIANT LED STAGE SCREEN PLAYING 15 VIDEO FX LOOPS    */}
      {/* Positioned precisely inside the stage screen opening         */}
      {/* ============================================================ */}
      <div className="absolute top-[2.8%] left-[12.2%] right-[12.5%] bottom-[26.2%] overflow-hidden bg-black z-0">
        <StageVisualFxCanvas
          bpm={currentBpm}
          energy={currentEnergy}
          isPlaying={isPlaying}
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
            DHUN — PARTY REQUEST PLAYLIST
          </p>

          {/* Real Audio-Synthesizer Oscilloscope Waveform spanning across LED stage */}
          <AudioOscilloscopeWaveform isPlaying={isPlaying} bpm={currentBpm} />
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
      {/* Grooves, head-bobs, pumps arm, and bounces to mood & BPM     */}
      {/* ============================================================ */}
      <div className="absolute left-1/2 bottom-[17.5%] -translate-x-1/2 w-[460px] md:w-[540px] h-[480px] md:h-[560px] pointer-events-none z-20">
        <AnimatedDjCharacter
          bpm={currentBpm}
          energy={currentEnergy}
          moodId={selectedMood}
          isPlaying={isPlaying}
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
      {/* Pixel-matched to the user's reference artwork                */}
      {/* ============================================================ */}

      {/* 5A. TOP-LEFT: SESSION STATUS, DJ TYPE, MOOD DROPDOWNS */}
      <div className="absolute top-5 left-5 z-40 flex flex-col space-y-2 pointer-events-auto">
        {/* Local Session Status Pill */}
        <div
          onClick={() => setShowHostDrawer(true)}
          title="Click to open Host DJ Console & Music Library (or press 'M')"
          className="inline-flex flex-col bg-zinc-950/75 backdrop-blur-xl border border-white/10 px-3.5 py-2 rounded-xl shadow-2xl cursor-pointer hover:border-amber-400/40 transition-all group"
        >
          <span className="text-[11px] font-bold text-slate-300">Local Session Status</span>
          <div className="flex items-center space-x-1.5 mt-0.5">
            <span className={`w-2 h-2 rounded-full ${partyStatus.is_active || !isTauri ? 'bg-emerald-400 animate-ping' : 'bg-amber-400 animate-pulse'}`} />
            <span className={`text-[11px] font-semibold ${partyStatus.is_active || !isTauri ? 'text-emerald-400' : 'text-amber-400'}`}>
              {partyStatus.is_active ? 'Connected' : (!isTauri ? 'Connected' : 'Offline / Setup')}
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
              const levelMap: Record<string, number> = { Pro: 1, Smart: 2, Advanced: 3, Creative: 4 };
              await setDjLevel(levelMap[val] || 2);
            }}
            className="w-52 appearance-none bg-zinc-950/75 backdrop-blur-xl border border-white/10 hover:border-amber-400/50 text-white text-xs font-bold px-3.5 py-2 rounded-xl focus:outline-none focus:ring-1 focus:ring-amber-400 cursor-pointer shadow-xl transition-all"
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

        {/* MOOD Dropdown (All 14 Moods) */}
        <div className="relative">
          <select
            value={selectedMood}
            onChange={async (e) => {
              const moodVal = e.target.value;
              setSelectedMood(moodVal);
              await setPartyMood(moodVal);
            }}
            className="w-52 appearance-none bg-zinc-950/75 backdrop-blur-xl border border-white/10 hover:border-amber-400/50 text-white text-xs font-bold px-3.5 py-2 rounded-xl focus:outline-none focus:ring-1 focus:ring-amber-400 cursor-pointer shadow-xl transition-all"
          >
            {moodsList.map((m) => (
              <option key={m.id} value={m.id} className="bg-zinc-950 text-white">
                {m.name}
              </option>
            ))}
          </select>
          <div className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-slate-400 text-xs">
            ▼
          </div>
        </div>

        {/* Floating unmute notification if user has not yet interacted */}
        {!audioActive && (
          <button
            onClick={async () => {
              await startBrowserAudio();
              setAudioActive(true);
            }}
            className="flex items-center space-x-2 bg-gradient-to-r from-rose-600 to-amber-500 hover:from-rose-500 hover:to-amber-400 text-white text-xs font-black px-3.5 py-1.5 rounded-xl shadow-lg shadow-rose-950/60 animate-bounce"
          >
            <Volume2 className="w-3.5 h-3.5" />
            <span>🔊 Click to Unmute Live DJ Audio</span>
          </button>
        )}
      </div>

      {/* 5B. BOTTOM-LEFT: NOW PLAYING FROSTED GLASS CARD */}
      <div className="absolute bottom-6 left-6 z-40 w-72 md:w-80 bg-zinc-950/80 backdrop-blur-2xl border border-white/15 p-4 rounded-3xl shadow-[0_20px_50px_rgba(0,0,0,0.9)] pointer-events-auto">
        <div className="flex items-center justify-between text-xs text-slate-400 mb-2 font-bold">
          <span className="text-white flex items-center space-x-1.5">
            <span className="w-2 h-2 rounded-full bg-rose-500 animate-pulse" />
            <span>Now Playing</span>
          </span>
          <button
            onClick={() => setShowHostDrawer(true)}
            className="w-5 h-5 rounded-md bg-white/10 hover:bg-white/20 flex items-center justify-center text-slate-300 hover:text-white transition-colors"
            title="Open Console Drawer (or press 'M')"
          >
            <span className="w-2 h-0.5 bg-current rounded-full" />
          </button>
        </div>

        {/* Track Artwork & Metadata */}
        <div className="flex items-center space-x-3 mb-2.5">
          <div className="w-11 h-11 rounded-xl bg-gradient-to-br from-red-600 via-orange-600 to-amber-500 flex items-center justify-center shadow-lg relative overflow-hidden">
            <Disc3 className={`w-6 h-6 text-white ${isPlaying ? 'animate-[spin_4s_linear_infinite]' : ''}`} />
          </div>
          <div className="flex-1 min-w-0">
            <h4 className="text-sm font-extrabold text-white truncate drop-shadow">{currentTitle}</h4>
            <p className="text-xs text-slate-300 truncate">{currentArtist}</p>
          </div>
        </div>

        {/* Progress Bar with Cyan/Teal fill & Seek Knob */}
        <div className="space-y-1 mb-2.5">
          <div
            className="w-full h-1.5 bg-white/20 rounded-full overflow-visible cursor-pointer relative"
            onClick={(e) => {
              const rect = e.currentTarget.getBoundingClientRect();
              const pct = (e.clientX - rect.left) / rect.width;
              const targetSec = Math.floor(pct * currentDurationSec);
              setLocalCurrentTimeSec(targetSec);
              if (isTauri) {
                apiDeckControl('seek', 0, { positionMs: targetSec * 1000 });
              }
            }}
          >
            <div
              className="h-full bg-teal-400 rounded-full transition-all duration-300 relative"
              style={{ width: `${progressPercent}%` }}
            >
              <div className="absolute right-0 top-1/2 -translate-y-1/2 w-2.5 h-2.5 rounded-full bg-teal-200 shadow-md shadow-teal-400" />
            </div>
          </div>
          <div className="flex justify-between text-[10px] font-mono text-slate-400">
            <span>{formatTime(currentTimeSec)}</span>
            <span>{formatTime(currentDurationSec)}</span>
          </div>
        </div>

        {/* Transport Controls (Clean 3-Button Center Layout Matching Mockup) */}
        <div className="flex items-center justify-around pt-1 px-4">
          <button
            onClick={() => {
              setLocalCurrentTimeSec(0);
              if (isTauri) {
                apiDeckControl('seek', 0, { positionMs: 0 });
              }
            }}
            className="p-2 text-slate-300 hover:text-white transition-colors"
            title="Restart Track"
          >
            <SkipBack className="w-4 h-4" />
          </button>

          <button
            onClick={handleTogglePlay}
            className="w-10 h-10 rounded-full bg-teal-400 hover:bg-teal-300 text-black flex items-center justify-center shadow-lg shadow-teal-400/40 active:scale-95 transition-all"
            title={isPlaying ? 'Pause Playback' : 'Play Audio'}
          >
            {isPlaying ? <Pause className="w-4 h-4 fill-current" /> : <Play className="w-4 h-4 fill-current ml-0.5" />}
          </button>

          <button
            onClick={handleNextTrack}
            className="p-2 text-slate-300 hover:text-white transition-colors"
            title="Next Track (Harmonic Mix)"
          >
            <SkipForward className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* 5C. BOTTOM-RIGHT: SCAN TO REQUEST FROSTED GLASS QR CARD */}
      <div className="absolute bottom-6 right-6 z-40 w-64 md:w-72 bg-zinc-950/80 backdrop-blur-2xl border border-white/15 p-4 rounded-3xl shadow-[0_20px_50px_rgba(0,0,0,0.9)] pointer-events-auto flex flex-col items-center text-center">
        <h4 className="text-xs font-bold tracking-wider text-slate-200 uppercase mb-2.5">
          Scan to Request
        </h4>

        {/* Authentic, High-Contrast QR Code Box */}
        <div
          className="w-32 h-32 md:w-36 md:h-36 bg-white p-2 rounded-2xl shadow-inner flex items-center justify-center mb-2.5 overflow-hidden [&>svg]:w-full [&>svg]:h-full"
          dangerouslySetInnerHTML={{ __html: qrSvgHtml }}
        />

        <p className="text-xs font-black tracking-wide text-white uppercase drop-shadow">
          SCAN TO REQUEST
        </p>
        <p className="text-[11px] font-bold text-amber-300 mt-0.5">
          स्कैन कर गाना मांगें
        </p>
        <p className="text-[10px] font-mono text-slate-400 mt-1">
          LAN: {displayedLanAddress}
        </p>
      </div>

      {/* ============================================================ */}
      {/* DISCREET HOST SLIDE-OVER DRAWER (HOTKEY 'M' OR CLICK STATUS) */}
      {/* Hidden by default so Stage Display is completely immersive  */}
      {/* ============================================================ */}
      {showHostDrawer && (
        <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex justify-end transition-all">
          <div className="w-full max-w-xl h-full bg-zinc-950 border-l border-white/15 p-6 flex flex-col shadow-2xl overflow-y-auto">
            {/* Drawer Header */}
            <div className="flex items-center justify-between pb-4 border-b border-white/10">
              <div className="flex items-center space-x-2.5">
                <Sliders className="w-5 h-5 text-amber-400" />
                <h3 className="text-lg font-black text-white">DHUN Host DJ Console</h3>
              </div>
              <button
                onClick={() => setShowHostDrawer(false)}
                className="p-1.5 rounded-lg bg-zinc-900 hover:bg-zinc-800 text-slate-300 hover:text-white"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Tab Navigation */}
            <div className="flex space-x-2 my-4">
              <button
                onClick={() => setActiveDrawerTab('library')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'library'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400'
                }`}
              >
                Music Library
              </button>
              <button
                onClick={() => setActiveDrawerTab('mixer')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'mixer'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400'
                }`}
              >
                4-Deck Mixer
              </button>
              <button
                onClick={() => setActiveDrawerTab('requests')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'requests'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400'
                }`}
              >
                Guest Requests ({guestRequests.length})
              </button>
              <button
                onClick={() => setActiveDrawerTab('fx')}
                className={`flex-1 py-2 text-xs font-bold rounded-lg border transition-all ${
                  activeDrawerTab === 'fx'
                    ? 'bg-amber-500/20 border-amber-400 text-amber-300'
                    : 'bg-zinc-900 border-white/5 text-slate-400'
                }`}
              >
                15 Video FX
              </button>
            </div>

            {/* Tab Content */}
            <div className="flex-1 overflow-y-auto">
              {activeDrawerTab === 'library' && (
                <div className="space-y-4">
                  <div className="bg-zinc-900/80 p-4 rounded-xl border border-white/10">
                    <h4 className="text-xs font-bold text-amber-300 uppercase mb-2">Local Music Folder Scanner</h4>
                    <div className="flex space-x-2">
                      <input
                        type="text"
                        value={folderPathInput}
                        onChange={(e) => setFolderPathInput(e.target.value)}
                        placeholder="C:/Music or folder path..."
                        className="flex-1 bg-black/60 border border-white/15 px-3 py-1.5 text-xs rounded-lg text-white font-mono"
                      />
                      <button
                        onClick={async () => {
                          if (!folderPathInput.trim()) return;
                          setIsScanning(true);
                          try {
                            await scanFolder(folderPathInput.trim());
                          } catch (e) {
                            console.error('Scan error:', e);
                          } finally {
                            setIsScanning(false);
                          }
                        }}
                        className="px-3 py-1.5 bg-amber-500 hover:bg-amber-400 text-black font-extrabold text-xs rounded-lg flex items-center space-x-1"
                      >
                        <FolderOpen className="w-3.5 h-3.5" />
                        <span>{isScanning ? 'Scanning...' : 'Scan Folder'}</span>
                      </button>
                    </div>
                  </div>

                  {/* Manual Single MP3 Import */}
                  <label className="flex items-center justify-center space-x-2 p-3 bg-zinc-900/60 hover:bg-zinc-900 border border-dashed border-white/20 rounded-xl cursor-pointer text-xs font-bold text-slate-300">
                    <Music className="w-4 h-4 text-amber-400" />
                    <span>Import Local MP3 File Directly</span>
                    <input
                      ref={fileInputRef}
                      type="file"
                      accept="audio/*"
                      onChange={handleFileUpload}
                      className="hidden"
                    />
                  </label>

                  {/* Preloaded Celebration Library */}
                  <div>
                    <h4 className="text-xs font-bold text-slate-400 uppercase mb-2">Available Celebration Tracks</h4>
                    <div className="space-y-1.5">
                      {[
                        { title: 'Chaiyya Chaiyya', artist: 'Sukhwinder Singh', bpm: 128, key: 'D Major', energy: '88%' },
                        { title: 'Gallan Goodiyaan', artist: 'Diljit Dosanjh', bpm: 128, key: 'G Major', energy: '92%' },
                        { title: 'Sauda Khara Khara', artist: 'Diljit Dosanjh, Sukhbir', bpm: 130, key: 'F Minor', energy: '95%' },
                        { title: 'Kala Chashma', artist: 'Amar Arshi, Badshah', bpm: 126, key: 'C# Minor', energy: '96%' },
                        { title: 'Chogada (Garba Dhol)', artist: 'Darshan Raval', bpm: 124, key: 'A Major', energy: '94%' },
                        { title: 'Mundian To Bach Ke', artist: 'Panjabi MC', bpm: 100, key: 'Ab Minor', energy: '93%' },
                        { title: 'Aankh Marey', artist: 'Mika Singh, Neha Kakkar', bpm: 124, key: 'F Major', energy: '91%' },
                      ].map((t, idx) => (
                        <div
                          key={idx}
                          className="flex items-center justify-between p-2.5 bg-zinc-900/60 hover:bg-zinc-900 rounded-lg border border-white/5"
                        >
                          <div>
                            <p className="text-xs font-bold text-white">{t.title}</p>
                            <p className="text-[11px] text-slate-400">{t.artist}</p>
                          </div>
                          <div className="flex items-center space-x-3 text-[11px] font-mono text-amber-400">
                            <span>{t.bpm} BPM</span>
                            <span>{t.key}</span>
                            <button
                              onClick={() => {
                                handleTogglePlay();
                              }}
                              className="px-2 py-1 bg-amber-500/20 hover:bg-amber-500/40 border border-amber-400/40 rounded text-amber-300 font-bold"
                            >
                              Load Deck
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              )}

              {activeDrawerTab === 'mixer' && (
                <div className="space-y-4">
                  <div className="grid grid-cols-2 gap-4">
                    {/* Deck A Strip */}
                    <div className="bg-zinc-900/80 p-3 rounded-xl border border-rose-500/30">
                      <h4 className="text-xs font-bold text-rose-400 uppercase mb-2">Deck A (Active)</h4>
                      <div className="space-y-2 text-xs">
                        <div>
                          <label className="text-[10px] text-slate-400">Low (Kill -36dB)</label>
                          <input type="range" min="-36" max="6" defaultValue="0" className="w-full accent-rose-400" />
                        </div>
                        <div>
                          <label className="text-[10px] text-slate-400">Mid (Sweep)</label>
                          <input type="range" min="-36" max="6" defaultValue="0" className="w-full accent-rose-400" />
                        </div>
                        <div>
                          <label className="text-[10px] text-slate-400">High (Air)</label>
                          <input type="range" min="-36" max="6" defaultValue="0" className="w-full accent-rose-400" />
                        </div>
                      </div>
                    </div>

                    {/* Deck B Strip */}
                    <div className="bg-zinc-900/80 p-3 rounded-xl border border-teal-500/30">
                      <h4 className="text-xs font-bold text-teal-400 uppercase mb-2">Deck B (Preloaded)</h4>
                      <div className="space-y-2 text-xs">
                        <div>
                          <label className="text-[10px] text-slate-400">Low (Kill -36dB)</label>
                          <input type="range" min="-36" max="6" defaultValue="0" className="w-full accent-teal-400" />
                        </div>
                        <div>
                          <label className="text-[10px] text-slate-400">Mid (Sweep)</label>
                          <input type="range" min="-36" max="6" defaultValue="0" className="w-full accent-teal-400" />
                        </div>
                        <div>
                          <label className="text-[10px] text-slate-400">High (Air)</label>
                          <input type="range" min="-36" max="6" defaultValue="0" className="w-full accent-teal-400" />
                        </div>
                      </div>
                    </div>
                  </div>

                  <div className="bg-zinc-900/80 p-3 rounded-xl border border-white/10">
                    <label className="text-xs font-bold text-amber-300 uppercase block mb-1">Smart Bass Swap Crossfader</label>
                    <input type="range" min="-1" max="1" step="0.05" defaultValue="-0.9" className="w-full accent-amber-400" />
                    <div className="flex justify-between text-[10px] text-slate-400 mt-1">
                      <span>Deck A</span>
                      <span>Center</span>
                      <span>Deck B</span>
                    </div>
                  </div>
                </div>
              )}

              {activeDrawerTab === 'requests' && (
                <div className="space-y-3">
                  <h4 className="text-xs font-bold text-amber-300 uppercase">Live Guest Song Requests</h4>
                  {guestRequests.length === 0 ? (
                    <p className="text-xs text-slate-400">No requests yet. Guests can scan the QR code to submit songs!</p>
                  ) : (
                    guestRequests.map((req) => (
                      <div key={req.id} className="p-3 bg-zinc-900 rounded-xl border border-white/10 flex items-center justify-between">
                        <div>
                          <p className="text-xs font-bold text-white">{req.track_title}</p>
                          <p className="text-[11px] text-slate-400">{req.track_artist} • requested by <span className="text-amber-300">{req.guest_name}</span></p>
                        </div>
                        <div className="flex space-x-1.5">
                          <button
                            onClick={() => approveRequest(req.id)}
                            className="p-1.5 bg-emerald-600/30 hover:bg-emerald-600 border border-emerald-500 rounded text-emerald-300"
                            title="Approve (+1.5 score boost)"
                          >
                            <Check className="w-3.5 h-3.5" />
                          </button>
                          <button
                            onClick={() => rejectRequest(req.id)}
                            className="p-1.5 bg-rose-600/30 hover:bg-rose-600 border border-rose-500 rounded text-rose-300"
                            title="Reject"
                          >
                            <X className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    ))
                  )}
                </div>
              )}

              {activeDrawerTab === 'fx' && (
                <div className="space-y-3">
                  <div className="flex items-center justify-between pb-2 border-b border-white/10">
                    <h4 className="text-xs font-bold text-amber-300 uppercase">15 Pre-Saved Video FX Loops</h4>
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
                        {currentFxIndex === idx && <Sparkles className="w-3 h-3 text-amber-400 flex-shrink-0 ml-1" />}
                      </button>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* Drawer Footer Emergency Controls */}
            <div className="pt-4 mt-auto border-t border-white/10 flex items-center justify-between">
              <button
                onClick={emergencyFade}
                className="px-4 py-2 bg-red-950 hover:bg-red-900 border border-red-700 rounded-xl text-xs font-bold text-red-200 flex items-center space-x-1.5"
              >
                <AlertTriangle className="w-3.5 h-3.5 text-red-400" />
                <span>Emergency 300ms Fade</span>
              </button>
              <button
                onClick={toggleFullscreen}
                className="p-2 bg-zinc-900 hover:bg-zinc-800 border border-white/10 rounded-xl text-slate-300"
              >
                {isFullscreen ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
