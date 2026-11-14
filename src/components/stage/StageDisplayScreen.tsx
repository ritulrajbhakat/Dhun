import React, { useState, useEffect } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { StageVisualFxCanvas, FX_NAMES } from './StageVisualFxCanvas';
import { AnimatedDjCharacter } from './AnimatedDjCharacter';
import {
  Play,
  Pause,
  SkipForward,
  SkipBack,
  Volume2,
  VolumeX,
  Maximize2,
  Minimize2,
  Sliders,
  QrCode,
  Music,
  Disc3,
  Flame,
  AlertTriangle,
} from 'lucide-react';

export const StageDisplayScreen: React.FC = () => {
  const {
    partyStatus,
    telemetry,
    moodProfiles,
    startParty,
    setMasterVolume,
    masterVolume,
    emergencyFade,
    toggleDeckPlay,
    forceNextRequest,
    guestRequests,
    startBrowserAudio,
    loadLocalAudioFile,
  } = useDhunStore();

  const [currentFxIndex, setCurrentFxIndex] = useState(0);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isPlaying, setIsPlaying] = useState(true);
  const [currentTimeSec, setCurrentTimeSec] = useState(30);
  const [selectedDjType, setSelectedDjType] = useState('Advanced');
  const [selectedMood, setSelectedMood] = useState('house_party');
  const [audioActive, setAudioActive] = useState(false);
  const fileInputRef = React.useRef<HTMLInputElement | null>(null);

  const activeTrack = partyStatus.now_playing?.track;
  const activeAnalysis = partyStatus.now_playing?.analysis;

  const currentTitle = activeTrack?.title || 'Chaiyya Chaiyya';
  const currentArtist = activeTrack?.artist || 'Sukhwinder Singh, Sapna Awasthi';
  const currentBpm = activeAnalysis?.bpm || 128;
  const currentEnergy = activeAnalysis?.overall_energy || 0.88;
  const currentDurationSec = activeTrack?.duration_ms ? Math.floor(activeTrack.duration_ms / 1000) : 296;

  // Auto-cycle FX loops every 25 seconds
  useEffect(() => {
    const fxTimer = setInterval(() => {
      setCurrentFxIndex((idx) => (idx + 1) % 15);
    }, 25000);
    return () => clearInterval(fxTimer);
  }, []);

  // Track playback time counter
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

  const djTypes = [
    { id: 'Pro', name: 'DJ TYPE: Professional 2-Deck' },
    { id: 'Smart', name: 'DJ TYPE: Smart Mixing' },
    { id: 'Advanced', name: 'DJ TYPE: Advanced' },
    { id: 'Creative', name: 'DJ TYPE: Creative Tools' },
  ];

  const moodsList = [
    { id: 'house_party', name: 'MOOD: House Party' },
    { id: 'shaadi', name: 'MOOD: Shaadi' },
    { id: 'baraat', name: 'MOOD: Baraat (High Energy)' },
    { id: 'mehendi', name: 'MOOD: Mehendi' },
    { id: 'sangeet', name: 'MOOD: Sangeet' },
    { id: 'desi_club', name: 'MOOD: Desi Club' },
    { id: 'punjabi_tashan', name: 'MOOD: Punjabi Tashan' },
    { id: 'garba', name: 'MOOD: Garba & Dandiya' },
    { id: 'retro_bollywood', name: 'MOOD: Retro Bollywood' },
    { id: 'festival_mode', name: 'MOOD: Festival Mode' },
  ];

  const progressPercent = Math.min(100, (currentTimeSec / currentDurationSec) * 100);

  // LAN QR Code target URL
  const lanIp = partyStatus.guest_url || 'http://192.168.1.15:3000/request';

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-black select-none font-sans text-white">
      {/* ============================================================ */}
      {/* LAYER 1: BASE STAGE AMBIENCE & WARM FESTIVAL LIGHTS           */}
      {/* ============================================================ */}
      <div
        className="absolute inset-0 bg-cover bg-center pointer-events-none opacity-45 mix-blend-screen scale-105"
        style={{ backgroundImage: "url('/stage_backdrop.jpg')" }}
      />

      {/* Warm lateral stage festival bulbs */}
      <div className="absolute top-0 left-0 bottom-0 w-48 bg-gradient-to-r from-amber-600/20 via-rose-950/20 to-transparent pointer-events-none" />
      <div className="absolute top-0 right-0 bottom-0 w-48 bg-gradient-to-l from-amber-600/20 via-rose-950/20 to-transparent pointer-events-none" />

      {/* Top stage curtain and lighting rail */}
      <div className="absolute top-0 left-0 right-0 h-16 bg-gradient-to-b from-black via-black/80 to-transparent z-10 pointer-events-none" />

      {/* ============================================================ */}
      {/* LAYER 2: GIANT LED STAGE SCREEN PLAYING 15 VIDEO FX LOOPS    */}
      {/* ============================================================ */}
      <div className="absolute top-[8%] left-[10%] right-[10%] bottom-[24%] rounded-3xl overflow-hidden border-4 border-amber-900/40 shadow-[0_0_80px_rgba(245,158,11,0.25)] bg-slate-950">
        <StageVisualFxCanvas
          bpm={currentBpm}
          energy={currentEnergy}
          isPlaying={isPlaying}
          activeFxIndex={currentFxIndex}
          onFxChange={(idx) => setCurrentFxIndex(idx)}
        />

        {/* Central Golden Stage Typography: "धुन डीजे" / "DHUN - PARTY REQUEST PLAYLIST" */}
        <div className="absolute inset-0 flex flex-col items-center justify-start pt-6 md:pt-10 pointer-events-none z-10">
          <h1 className="text-5xl md:text-8xl font-black text-amber-300 tracking-wider drop-shadow-[0_6px_20px_rgba(0,0,0,0.95)]">
            धुन डीजे
          </h1>
          <p className="text-xs md:text-sm font-bold tracking-[0.3em] uppercase text-white/90 drop-shadow-[0_2px_8px_rgba(0,0,0,0.9)] mt-1">
            DHUN — PARTY REQUEST PLAYLIST
          </p>

          {/* Soundwave Bars Visualizer Graphic below text */}
          <div className="flex items-center space-x-1.5 mt-2 opacity-80">
            {[4, 12, 24, 16, 32, 20, 36, 18, 28, 14, 30, 22, 10, 26, 8].map((h, i) => (
              <div
                key={i}
                className="w-1 bg-amber-400 rounded-full animate-pulse"
                style={{
                  height: `${h * (isPlaying ? 1 : 0.25)}px`,
                  animationDuration: `${0.4 + (i % 5) * 0.15}s`,
                }}
              />
            ))}
          </div>
        </div>

        {/* Subtle FX Preset Indicator on stage screen corner */}
        <div className="absolute top-3 right-4 z-20 flex items-center space-x-2 bg-black/60 backdrop-blur-md px-3 py-1 rounded-full border border-white/10 text-[10px] text-amber-300 font-semibold">
          <Flame className="w-3 h-3 text-rose-500 animate-pulse" />
          <span>FX {currentFxIndex + 1}/15: {FX_NAMES[currentFxIndex]}</span>
        </div>
      </div>

      {/* ============================================================ */}
      {/* LAYER 3: ANIMATED DJ CHARACTER IN THE CENTER                 */}
      {/* ============================================================ */}
      <div className="absolute left-1/2 bottom-[14%] -translate-x-1/2 w-[520px] h-[520px] pointer-events-none z-20">
        <AnimatedDjCharacter
          bpm={currentBpm}
          energy={currentEnergy}
          moodId={selectedMood}
          isPlaying={isPlaying}
        />
      </div>

      {/* ============================================================ */}
      {/* LAYER 4: FRONT DJ CONSOLE DESK & MARIGOLD FLOWER GARLANDS    */}
      {/* ============================================================ */}
      <div className="absolute bottom-0 left-0 right-0 h-[22%] z-30 pointer-events-none bg-gradient-to-t from-black via-zinc-950/95 to-transparent flex flex-col justify-end">
        {/* DJ Mixer Hardware Turntables & Platter Silhouettes */}
        <div className="w-full max-w-5xl mx-auto flex items-center justify-between px-12 mb-3 opacity-90">
          {/* Deck A Platter */}
          <div className="relative w-28 h-28 rounded-full border-4 border-slate-700 bg-zinc-900/90 shadow-2xl flex items-center justify-center">
            <div className={`w-20 h-20 rounded-full border-2 border-slate-600 bg-black flex items-center justify-center ${isPlaying ? 'animate-[spin_4s_linear_infinite]' : ''}`}>
              <div className="w-8 h-8 rounded-full bg-rose-600 border border-white/40 flex items-center justify-center">
                <Disc3 className="w-4 h-4 text-white" />
              </div>
            </div>
          </div>

          {/* Center Mixer Strips & Crossfader */}
          <div className="h-16 w-64 bg-zinc-900/80 rounded-xl border border-slate-800 p-2 flex items-center justify-around">
            <div className="w-1.5 h-10 bg-slate-700 rounded-full flex flex-col justify-end p-0.5">
              <div className="w-full bg-emerald-400 rounded-full" style={{ height: isPlaying ? '70%' : '10%' }} />
            </div>
            <div className="w-24 h-3 bg-black rounded-full p-0.5 border border-slate-700 flex items-center">
              <div className="w-5 h-5 rounded bg-amber-500 shadow-md transform translate-x-3" />
            </div>
            <div className="w-1.5 h-10 bg-slate-700 rounded-full flex flex-col justify-end p-0.5">
              <div className="w-full bg-rose-400 rounded-full" style={{ height: isPlaying ? '60%' : '10%' }} />
            </div>
          </div>

          {/* Deck B Platter */}
          <div className="relative w-28 h-28 rounded-full border-4 border-slate-700 bg-zinc-900/90 shadow-2xl flex items-center justify-center">
            <div className={`w-20 h-20 rounded-full border-2 border-slate-600 bg-black flex items-center justify-center ${isPlaying ? 'animate-[spin_4s_linear_infinite]' : ''}`}>
              <div className="w-8 h-8 rounded-full bg-amber-500 border border-white/40 flex items-center justify-center">
                <Disc3 className="w-4 h-4 text-white" />
              </div>
            </div>
          </div>
        </div>

        {/* Marigold Flower Garlands (Genda Phool Decor) across desk */}
        <div className="w-full h-8 flex items-center justify-around overflow-hidden px-4 opacity-95">
          {Array.from({ length: 42 }).map((_, idx) => (
            <div
              key={idx}
              className={`w-6 h-6 rounded-full shadow-lg transform ${
                idx % 2 === 0 ? 'bg-amber-500 -translate-y-1' : 'bg-orange-600 translate-y-1'
              } border border-amber-300/40`}
            />
          ))}
        </div>
      </div>

      {/* ============================================================ */}
      {/* LAYER 5: THREE FLOATING FROSTED GLASS UI PANELS              */}
      {/* ============================================================ */}

      {/* 5A. TOP-LEFT: SESSION STATUS, DJ TYPE, MOOD DROPDOWNS */}
      <div className="absolute top-6 left-6 z-40 flex flex-col space-y-2.5 pointer-events-auto">
        {/* Local Session Status */}
        <div className="inline-flex items-center space-x-2.5 bg-black/70 backdrop-blur-xl border border-white/15 px-3.5 py-1.5 rounded-xl shadow-2xl">
          <div className="w-2.5 h-2.5 rounded-full bg-emerald-400 animate-ping" />
          <span className="text-xs font-bold text-slate-200">Local Session Status</span>
          <span className="text-[11px] font-semibold text-emerald-400 bg-emerald-950/80 px-2 py-0.5 rounded-md border border-emerald-800">
            Connected
          </span>
        </div>

        {/* DJ TYPE Dropdown */}
        <div className="relative">
          <select
            value={selectedDjType}
            onChange={(e) => setSelectedDjType(e.target.value)}
            className="w-56 appearance-none bg-black/75 backdrop-blur-xl border border-white/15 hover:border-amber-400/50 text-white text-xs font-bold px-3.5 py-2.5 rounded-xl focus:outline-none focus:ring-1 focus:ring-amber-400 cursor-pointer shadow-xl transition-all"
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

        {/* MOOD Dropdown */}
        <div className="relative">
          <select
            value={selectedMood}
            onChange={(e) => {
              setSelectedMood(e.target.value);
              startParty(partyStatus.party_name || 'DHUN Party', 2, e.target.value);
            }}
            className="w-56 appearance-none bg-black/75 backdrop-blur-xl border border-white/15 hover:border-amber-400/50 text-white text-xs font-bold px-3.5 py-2.5 rounded-xl focus:outline-none focus:ring-1 focus:ring-amber-400 cursor-pointer shadow-xl transition-all"
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

        {/* Unmute sound banner trigger if not active */}
        {!audioActive && (
          <button
            onClick={async () => {
              await startBrowserAudio();
              setAudioActive(true);
            }}
            className="flex items-center space-x-2 bg-gradient-to-r from-rose-600 to-amber-500 hover:from-rose-500 hover:to-amber-400 text-white text-xs font-extrabold px-3.5 py-2 rounded-xl shadow-lg shadow-rose-950/60 animate-bounce"
          >
            <Volume2 className="w-3.5 h-3.5" />
            <span>🔊 Unmute Live Audio</span>
          </button>
        )}
      </div>

      {/* 5B. TOP-RIGHT: FULLSCREEN & EMERGENCY CONTROLS */}
      <div className="absolute top-6 right-6 z-40 flex items-center space-x-3 pointer-events-auto">
        <label className="flex items-center space-x-1.5 px-3 py-1.5 bg-black/70 backdrop-blur-xl hover:bg-black/90 border border-white/15 rounded-xl text-xs font-bold text-amber-300 cursor-pointer transition-all">
          <Music className="w-3.5 h-3.5" />
          <span>Load Song (.mp3)</span>
          <input
            ref={fileInputRef}
            type="file"
            accept="audio/*"
            onChange={handleFileUpload}
            className="hidden"
          />
        </label>

        <button
          onClick={emergencyFade}
          title="Instant 300ms emergency master audio fade"
          className="flex items-center space-x-1.5 px-3 py-1.5 bg-red-950/80 hover:bg-red-900 border border-red-700/80 rounded-xl text-xs font-bold text-red-200 transition-all shadow-lg"
        >
          <AlertTriangle className="w-3.5 h-3.5 text-red-400" />
          <span>Emergency Fade</span>
        </button>

        <button
          onClick={toggleFullscreen}
          className="p-2 bg-black/70 backdrop-blur-xl hover:bg-black/90 border border-white/15 rounded-xl text-slate-300 hover:text-white transition-all shadow-lg"
          title="Toggle Fullscreen Mode"
        >
          {isFullscreen ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
        </button>
      </div>

      {/* 5C. BOTTOM-LEFT: NOW PLAYING FROSTED GLASS PLAYER CARD */}
      <div className="absolute bottom-6 left-6 z-40 w-80 bg-zinc-950/80 backdrop-blur-2xl border border-white/15 p-4 rounded-3xl shadow-[0_20px_50px_rgba(0,0,0,0.85)] pointer-events-auto">
        <div className="flex items-center justify-between text-xs text-slate-400 mb-2.5 font-bold">
          <span className="text-white flex items-center space-x-1.5">
            <span className="w-2 h-2 rounded-full bg-rose-500 animate-pulse" />
            <span>Now Playing</span>
          </span>
          <span className="text-[10px] text-amber-400 uppercase font-mono px-1.5 py-0.5 rounded bg-amber-950/60 border border-amber-800/60">
            {currentBpm} BPM
          </span>
        </div>

        {/* Track Artwork & Metadata */}
        <div className="flex items-center space-x-3 mb-3">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-rose-600 via-amber-600 to-indigo-700 flex items-center justify-center shadow-lg relative overflow-hidden">
            <Disc3 className={`w-7 h-7 text-white ${isPlaying ? 'animate-[spin_4s_linear_infinite]' : ''}`} />
          </div>
          <div className="flex-1 min-w-0">
            <h4 className="text-sm font-extrabold text-white truncate drop-shadow">{currentTitle}</h4>
            <p className="text-xs text-slate-300 truncate">{currentArtist}</p>
          </div>
        </div>

        {/* Progress Bar & Seek */}
        <div className="space-y-1 mb-3">
          <div
            className="w-full h-1.5 bg-white/20 rounded-full overflow-hidden cursor-pointer relative"
            onClick={(e) => {
              const rect = e.currentTarget.getBoundingClientRect();
              const pct = (e.clientX - rect.left) / rect.width;
              setCurrentTimeSec(Math.floor(pct * currentDurationSec));
            }}
          >
            <div
              className="h-full bg-gradient-to-r from-emerald-400 to-amber-400 rounded-full transition-all duration-300"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
          <div className="flex justify-between text-[10px] font-mono text-slate-400">
            <span>{formatTime(currentTimeSec)}</span>
            <span>{formatTime(currentDurationSec)}</span>
          </div>
        </div>

        {/* Transport Controls */}
        <div className="flex items-center justify-between pt-1">
          <button
            onClick={() => setCurrentTimeSec(0)}
            className="p-2 text-slate-300 hover:text-white transition-colors"
            title="Restart Track"
          >
            <SkipBack className="w-4 h-4" />
          </button>

          <button
            onClick={handleTogglePlay}
            className="w-10 h-10 rounded-full bg-emerald-500 hover:bg-emerald-400 text-black flex items-center justify-center shadow-lg shadow-emerald-500/40 active:scale-95 transition-all"
            title={isPlaying ? 'Pause Playback' : 'Play Audio'}
          >
            {isPlaying ? <Pause className="w-5 h-5 fill-current" /> : <Play className="w-5 h-5 fill-current ml-0.5" />}
          </button>

          <button
            onClick={handleNextTrack}
            className="p-2 text-slate-300 hover:text-white transition-colors"
            title="Next Track (Harmonic Mix In)"
          >
            <SkipForward className="w-4 h-4" />
          </button>

          {/* Master Volume */}
          <div className="flex items-center space-x-1.5 pl-2 border-l border-white/10">
            <Volume2 className="w-3.5 h-3.5 text-slate-400" />
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={masterVolume}
              onChange={(e) => setMasterVolume(parseFloat(e.target.value))}
              className="w-14 accent-emerald-400 h-1 bg-white/20 rounded cursor-pointer"
            />
          </div>
        </div>
      </div>

      {/* 5D. BOTTOM-RIGHT: SCAN TO REQUEST FROSTED GLASS QR CARD */}
      <div className="absolute bottom-6 right-6 z-40 w-72 bg-zinc-950/80 backdrop-blur-2xl border border-white/15 p-4 rounded-3xl shadow-[0_20px_50px_rgba(0,0,0,0.85)] pointer-events-auto flex flex-col items-center text-center">
        <h4 className="text-xs font-black tracking-wider text-slate-200 uppercase mb-3 flex items-center space-x-1.5">
          <QrCode className="w-3.5 h-3.5 text-amber-400" />
          <span>Scan to Request</span>
        </h4>

        {/* QR Code Graphic Box */}
        <div className="w-36 h-36 bg-white p-2 rounded-2xl shadow-inner flex items-center justify-center mb-3">
          <svg viewBox="0 0 100 100" className="w-full h-full">
            {/* Standard high-contrast QR Matrix representation */}
            <rect x="0" y="0" width="100" height="100" fill="#ffffff" />
            <rect x="8" y="8" width="28" height="28" fill="#000000" />
            <rect x="12" y="12" width="20" height="20" fill="#ffffff" />
            <rect x="16" y="16" width="12" height="12" fill="#000000" />

            <rect x="64" y="8" width="28" height="28" fill="#000000" />
            <rect x="68" y="12" width="20" height="20" fill="#ffffff" />
            <rect x="72" y="16" width="12" height="12" fill="#000000" />

            <rect x="8" y="64" width="28" height="28" fill="#000000" />
            <rect x="12" y="68" width="20" height="20" fill="#ffffff" />
            <rect x="16" y="72" width="12" height="12" fill="#000000" />

            {/* Pattern data pixels */}
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
          LAN: 192.168.1.15/request
        </p>
      </div>
    </div>
  );
};

