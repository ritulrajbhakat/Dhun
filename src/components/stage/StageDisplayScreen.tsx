import React, { useState, useEffect, useRef } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { StageVisualFxCanvas, FX_NAMES } from './StageVisualFxCanvas';
import { AnimatedDjCharacter } from './AnimatedDjCharacter';
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

export const StageDisplayScreen: React.FC = () => {
  const {
    partyStatus,
    startParty,
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
  } = useDhunStore();

  const [currentFxIndex, setCurrentFxIndex] = useState(0);
  const [autoCycleFx, setAutoCycleFx] = useState(true);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isPlaying, setIsPlaying] = useState(true);
  const [currentTimeSec, setCurrentTimeSec] = useState(179); // 02:59 initial
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
  const currentDurationSec = activeTrack?.duration_ms ? Math.floor(activeTrack.duration_ms / 1000) : 296; // 04:56

  // 1. User interaction listener to immediately start WebAudio on first click/keypress
  useEffect(() => {
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

  // 2. Keyboard shortcut 'M' or 'Escape' to toggle host console drawer
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'm' || e.key === 'M') {
        setShowHostDrawer((prev) => !prev);
      }
      if (e.key === 'Escape') {
        setShowHostDrawer(false);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // 3. Auto-cycle FX loops every 25 seconds if enabled
  useEffect(() => {
    if (!autoCycleFx) return;
    const fxTimer = setInterval(() => {
      setCurrentFxIndex((idx) => (idx + 1) % 15);
    }, 25000);
    return () => clearInterval(fxTimer);
  }, [autoCycleFx]);

  // 4. Track playback time progress
  useEffect(() => {
    if (!isPlaying) return;
    const progressTimer = setInterval(() => {
      setCurrentTimeSec((t) => (t >= currentDurationSec ? 0 : t + 1));
    }, 1000);
    return () => clearInterval(progressTimer);
  }, [isPlaying, currentDurationSec]);

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  };

  const handleTogglePlay = async () => {
    if (!audioActive) {
      await startBrowserAudio();
      setAudioActive(true);
    }
    setIsPlaying(!isPlaying);
    await toggleDeckPlay(0);
  };

  const handleNextTrack = async () => {
    if (partyStatus.next_track) {
      await forceNextRequest(partyStatus.next_track.track.id);
      setCurrentTimeSec(0);
    }
  };

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      await loadLocalAudioFile(0, file);
      setAudioActive(true);
      setIsPlaying(true);
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

  const progressPercent = Math.min(100, (currentTimeSec / currentDurationSec) * 100);

  // Dynamic LAN IP
  const lanHost = typeof window !== 'undefined' && window.location.hostname !== 'localhost'
    ? `${window.location.hostname}:1420`
    : '192.168.1.15';

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
        <div className="absolute inset-0 flex flex-col items-center justify-start pt-2 md:pt-4 pointer-events-none z-10">
          <h1 className="text-6xl md:text-8xl font-black text-amber-400 tracking-wider drop-shadow-[0_4px_18px_rgba(0,0,0,0.95)]">
            धुन डीजे
          </h1>
          <p className="text-xs md:text-sm font-bold tracking-[0.35em] uppercase text-white/95 drop-shadow-[0_2px_8px_rgba(0,0,0,0.9)] mt-1">
            DHUN — PARTY REQUEST PLAYLIST
          </p>

          {/* Soundwave Bars Visualizer Graphic below text */}
          <div className="flex items-center space-x-1.5 mt-2 opacity-90">
            {[6, 14, 28, 18, 36, 22, 42, 20, 32, 16, 34, 24, 12, 30, 8].map((h, i) => (
              <div
                key={i}
                className="w-1 bg-amber-400 rounded-full animate-pulse"
                style={{
                  height: `${h * (isPlaying ? 1 : 0.25)}px`,
                  animationDuration: `${0.35 + (i % 5) * 0.12}s`,
                }}
              />
            ))}
          </div>
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
          <div className="flex items-center space-x-2">
            <span className="text-[11px] font-bold text-slate-300">Local Session Status</span>
            <Settings className="w-3 h-3 text-slate-400 group-hover:text-amber-400 transition-colors" />
          </div>
          <div className="flex items-center space-x-1.5 mt-0.5">
            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-ping" />
            <span className="text-[11px] font-semibold text-emerald-400">Connected</span>
          </div>
        </div>

        {/* DJ TYPE Dropdown */}
        <div className="relative">
          <select
            value={selectedDjType}
            onChange={(e) => setSelectedDjType(e.target.value)}
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
            onChange={(e) => {
              setSelectedMood(e.target.value);
              startParty(partyStatus.party_name || 'DHUN Party', 2, e.target.value);
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
            className="text-slate-400 hover:text-white p-0.5 rounded transition-colors"
            title="Open Console Drawer"
          >
            <Sliders className="w-3.5 h-3.5" />
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
              setCurrentTimeSec(Math.floor(pct * currentDurationSec));
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

        {/* Transport Controls */}
        <div className="flex items-center justify-between pt-0.5">
          <button
            onClick={() => setCurrentTimeSec(0)}
            className="p-1.5 text-slate-300 hover:text-white transition-colors"
            title="Restart Track"
          >
            <SkipBack className="w-4 h-4" />
          </button>

          <button
            onClick={handleTogglePlay}
            className="w-9 h-9 rounded-full bg-teal-400 hover:bg-teal-300 text-black flex items-center justify-center shadow-lg shadow-teal-400/40 active:scale-95 transition-all"
            title={isPlaying ? 'Pause Playback' : 'Play Audio'}
          >
            {isPlaying ? <Pause className="w-4 h-4 fill-current" /> : <Play className="w-4 h-4 fill-current ml-0.5" />}
          </button>

          <button
            onClick={handleNextTrack}
            className="p-1.5 text-slate-300 hover:text-white transition-colors"
            title="Next Track (Harmonic Mix)"
          >
            <SkipForward className="w-4 h-4" />
          </button>

          {/* Volume Control */}
          <div className="flex items-center space-x-1.5 pl-2 border-l border-white/10">
            <Volume2 className="w-3 h-3 text-slate-400" />
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={masterVolume}
              onChange={(e) => setMasterVolume(parseFloat(e.target.value))}
              className="w-12 accent-teal-400 h-1 bg-white/20 rounded cursor-pointer"
            />
          </div>
        </div>
      </div>

      {/* 5C. BOTTOM-RIGHT: SCAN TO REQUEST FROSTED GLASS QR CARD */}
      <div className="absolute bottom-6 right-6 z-40 w-64 md:w-72 bg-zinc-950/80 backdrop-blur-2xl border border-white/15 p-4 rounded-3xl shadow-[0_20px_50px_rgba(0,0,0,0.9)] pointer-events-auto flex flex-col items-center text-center">
        <h4 className="text-xs font-bold tracking-wider text-slate-200 uppercase mb-2.5">
          Scan to Request
        </h4>

        {/* Clean, High-Contrast QR Code Box */}
        <div className="w-32 h-32 md:w-36 md:h-36 bg-white p-2 rounded-2xl shadow-inner flex items-center justify-center mb-2.5">
          <svg viewBox="0 0 100 100" className="w-full h-full">
            <rect x="0" y="0" width="100" height="100" fill="#ffffff" />
            {/* Finder pattern Top-Left */}
            <rect x="8" y="8" width="28" height="28" fill="#000000" />
            <rect x="13" y="13" width="18" height="18" fill="#ffffff" />
            <rect x="17" y="17" width="10" height="10" fill="#000000" />
            {/* Finder pattern Top-Right */}
            <rect x="64" y="8" width="28" height="28" fill="#000000" />
            <rect x="69" y="13" width="18" height="18" fill="#ffffff" />
            <rect x="73" y="17" width="10" height="10" fill="#000000" />
            {/* Finder pattern Bottom-Left */}
            <rect x="8" y="64" width="28" height="28" fill="#000000" />
            <rect x="13" y="69" width="18" height="18" fill="#ffffff" />
            <rect x="17" y="73" width="10" height="10" fill="#000000" />
            {/* Data pixels */}
            <rect x="42" y="10" width="6" height="6" fill="#000000" />
            <rect x="52" y="14" width="6" height="6" fill="#000000" />
            <rect x="42" y="24" width="6" height="6" fill="#000000" />
            <rect x="50" y="30" width="6" height="6" fill="#000000" />
            <rect x="14" y="44" width="6" height="6" fill="#000000" />
            <rect x="24" y="48" width="6" height="6" fill="#000000" />
            <rect x="36" y="42" width="6" height="6" fill="#000000" />
            <rect x="46" y="46" width="6" height="6" fill="#000000" />
            <rect x="56" y="42" width="6" height="6" fill="#000000" />
            <rect x="66" y="46" width="6" height="6" fill="#000000" />
            <rect x="76" y="42" width="6" height="6" fill="#000000" />
            <rect x="86" y="46" width="6" height="6" fill="#000000" />
            <rect x="42" y="64" width="6" height="6" fill="#000000" />
            <rect x="52" y="68" width="6" height="6" fill="#000000" />
            <rect x="62" y="64" width="6" height="6" fill="#000000" />
            <rect x="72" y="68" width="6" height="6" fill="#000000" />
            <rect x="82" y="64" width="6" height="6" fill="#000000" />
            <rect x="42" y="82" width="6" height="6" fill="#000000" />
            <rect x="54" y="86" width="6" height="6" fill="#000000" />
            <rect x="66" y="82" width="6" height="6" fill="#000000" />
            <rect x="78" y="86" width="6" height="6" fill="#000000" />
          </svg>
        </div>

        <p className="text-xs font-black tracking-wide text-white uppercase drop-shadow">
          SCAN TO REQUEST
        </p>
        <p className="text-[11px] font-bold text-amber-300 mt-0.5">
          स्कैन कर गाना मांगें
        </p>
        <p className="text-[10px] font-mono text-slate-400 mt-1">
          LAN: {lanHost}/request
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
                        onClick={() => {
                          setIsScanning(true);
                          setTimeout(() => setIsScanning(false), 800);
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
