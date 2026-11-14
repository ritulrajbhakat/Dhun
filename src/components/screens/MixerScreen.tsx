import React, { useState } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import {
  Play,
  Pause,
  Disc3,
  Sliders,
  Volume2,
  FastForward,
  RotateCcw,
  Sparkles,
  Layers,
} from 'lucide-react';

export const MixerScreen: React.FC = () => {
  const {
    decks,
    selectedDeckCount,
    setDeckCount,
    updateDeckEq,
    updateDeckGain,
    updateDeckFilter,
    updateDeckTempo,
    toggleDeckPlay,
    crossfader,
    setCrossfaderPos,
    crossfaderCurve,
    telemetry,
    partyStatus,
  } = useDhunStore();

  const [activeTab, setActiveTab] = useState<'decks' | 'fx'>('decks');

  const deckColors = ['border-cyan-500 text-cyan-400', 'border-orange-500 text-orange-400', 'border-purple-500 text-purple-400', 'border-emerald-500 text-emerald-400'];
  const deckLabels = ['DECK A', 'DECK B', 'DECK C', 'DECK D'];

  return (
    <div className="flex-1 overflow-y-auto p-4 flex flex-col justify-between space-y-4 max-w-[1600px] mx-auto select-none">
      {/* Top Console Controls */}
      <div className="flex items-center justify-between bg-dhun-surface border border-dhun-border px-5 py-2.5 rounded-xl">
        <div className="flex items-center space-x-3">
          <Sliders className="w-5 h-5 text-dhun-primary" />
          <span className="font-black text-sm text-white tracking-wider">HARDWARE MIXING CONSOLE</span>
          <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-slate-800 text-slate-300">
            {selectedDeckCount} DECKS &bull; CPAL 32-BIT FLOAT DSP
          </span>
        </div>

        {/* Deck Count Switcher (2 or 4 decks) */}
        <div className="flex items-center space-x-2">
          <span className="text-xs text-slate-400 font-semibold">Decks:</span>
          <button
            onClick={() => setDeckCount(2)}
            className={`px-3 py-1 rounded-lg text-xs font-bold transition-all ${
              selectedDeckCount === 2
                ? 'bg-dhun-primary text-white'
                : 'bg-dhun-card text-slate-400 hover:text-slate-200'
            }`}
          >
            2 Decks
          </button>
          <button
            onClick={() => setDeckCount(4)}
            className={`px-3 py-1 rounded-lg text-xs font-bold transition-all ${
              selectedDeckCount === 4
                ? 'bg-dhun-primary text-white'
                : 'bg-dhun-card text-slate-400 hover:text-slate-200'
            }`}
          >
            4 Decks
          </button>
        </div>
      </div>

      {/* Virtual Decks Matrix */}
      <div className={`grid grid-cols-1 ${selectedDeckCount === 4 ? 'lg:grid-cols-4' : 'lg:grid-cols-2'} gap-4`}>
        {Array.from({ length: selectedDeckCount }).map((_, i) => {
          const deckState = decks[i];
          const deckTelem = telemetry?.decks[i];
          const isPlaying = deckTelem?.is_playing ?? false;
          const trackTitle = i === 0
            ? partyStatus.now_playing?.track.title || 'Gallan Goodiyaan'
            : i === 1
            ? partyStatus.next_track?.track.title || 'London Thumakda'
            : `Track ${i + 1}`;
          const trackArtist = i === 0
            ? partyStatus.now_playing?.track.artist || 'Diljit Dosanjh'
            : i === 1
            ? partyStatus.next_track?.track.artist || 'Labh Janjua'
            : 'Various Artists';

          return (
            <div
              key={i}
              className={`bg-dhun-card/90 border-2 ${i === 0 ? 'border-cyan-800/80' : i === 1 ? 'border-orange-800/80' : 'border-dhun-border'} rounded-2xl p-4 flex flex-col justify-between space-y-4 shadow-xl`}
            >
              {/* Deck Header */}
              <div className="flex items-center justify-between border-b border-dhun-border/70 pb-2">
                <span className={`text-xs font-black tracking-widest ${deckColors[i]}`}>
                  {deckLabels[i]}
                </span>
                <span className="text-[11px] font-mono text-cyan-300 font-bold">
                  {deckTelem?.bpm || 128.0} BPM ({((deckState.tempo - 1.0) * 100).toFixed(1)}%)
                </span>
              </div>

              {/* Track Title */}
              <div className="truncate">
                <div className="font-bold text-sm text-white truncate">{trackTitle}</div>
                <div className="text-xs text-slate-400 truncate">{trackArtist}</div>
              </div>

              {/* Waveform Visualization Canvas */}
              <div className="relative h-14 bg-slate-950 rounded-xl overflow-hidden border border-slate-800/80 flex items-center justify-center">
                <div className="absolute inset-0 flex items-center justify-around px-2 opacity-70">
                  {Array.from({ length: 48 }).map((_, barIdx) => {
                    const heightPct = Math.sin(barIdx * 0.4 + i) * 35 + 45;
                    const isBeat = barIdx % 4 === 0;
                    return (
                      <div
                        key={barIdx}
                        className={`w-1 rounded-full ${isBeat ? 'bg-rose-400' : 'bg-slate-600'}`}
                        style={{ height: `${heightPct}%` }}
                      />
                    );
                  })}
                </div>
                {/* Playhead Needle */}
                <div className="absolute top-0 bottom-0 left-1/2 w-0.5 bg-white shadow-[0_0_8px_#ffffff] z-10" />
              </div>

              {/* Interactive Jog Wheel */}
              <div className="flex items-center justify-center py-2">
                <div
                  onClick={() => toggleDeckPlay(i)}
                  className={`w-36 h-36 rounded-full bg-slate-950 border-4 border-slate-800 flex items-center justify-center cursor-pointer shadow-2xl relative group ${
                    isPlaying ? 'ring-2 ring-dhun-accent/50' : ''
                  }`}
                >
                  {/* Vinyl grooves */}
                  <div className="w-28 h-28 rounded-full border border-slate-800/80 flex items-center justify-center">
                    <div className="w-20 h-20 rounded-full border border-slate-800 flex items-center justify-center">
                      <div
                        className={`w-12 h-12 rounded-full ${
                          i === 0 ? 'bg-cyan-600' : i === 1 ? 'bg-orange-600' : 'bg-purple-600'
                        } flex items-center justify-center text-white ${
                          isPlaying ? 'animate-[spin_2.5s_linear_infinite]' : ''
                        }`}
                      >
                        <Disc3 className="w-6 h-6" />
                      </div>
                    </div>
                  </div>
                  {/* Scratch needle pointer indicator */}
                  <div className="absolute top-2 w-1.5 h-3 bg-white rounded-full opacity-80" />
                </div>
              </div>

              {/* DSP Strip: EQ & Filter */}
              <div className="grid grid-cols-4 gap-2 text-center text-[10px] font-mono text-slate-400 pt-2 border-t border-dhun-border/60">
                <div>
                  <span className="block mb-1 font-bold text-slate-300">HI</span>
                  <input
                    type="range"
                    min="-48"
                    max="6"
                    value={deckState.eqHigh}
                    onChange={(e) => updateDeckEq(i, deckState.eqLow, deckState.eqMid, parseFloat(e.target.value))}
                    className="w-full accent-dhun-primary h-1 bg-slate-800 rounded cursor-pointer"
                  />
                  <span>{deckState.eqHigh}dB</span>
                </div>
                <div>
                  <span className="block mb-1 font-bold text-slate-300">MID</span>
                  <input
                    type="range"
                    min="-48"
                    max="6"
                    value={deckState.eqMid}
                    onChange={(e) => updateDeckEq(i, deckState.eqLow, parseFloat(e.target.value), deckState.eqHigh)}
                    className="w-full accent-dhun-primary h-1 bg-slate-800 rounded cursor-pointer"
                  />
                  <span>{deckState.eqMid}dB</span>
                </div>
                <div>
                  <span className="block mb-1 font-bold text-slate-300">LOW</span>
                  <input
                    type="range"
                    min="-48"
                    max="6"
                    value={deckState.eqLow}
                    onChange={(e) => updateDeckEq(i, parseFloat(e.target.value), deckState.eqMid, deckState.eqHigh)}
                    className="w-full accent-dhun-primary h-1 bg-slate-800 rounded cursor-pointer"
                  />
                  <span>{deckState.eqLow}dB</span>
                </div>
                <div>
                  <span className="block mb-1 font-bold text-slate-300">FILTER</span>
                  <input
                    type="range"
                    min="-1"
                    max="1"
                    step="0.05"
                    value={deckState.filter}
                    onChange={(e) => updateDeckFilter(i, parseFloat(e.target.value))}
                    className="w-full accent-emerald-500 h-1 bg-slate-800 rounded cursor-pointer"
                  />
                  <span>{deckState.filter > 0 ? `HPF ${Math.round(deckState.filter * 100)}` : deckState.filter < 0 ? `LPF ${Math.round(-deckState.filter * 100)}` : 'OFF'}</span>
                </div>
              </div>

              {/* Performance Hot Cues & Play/Pause */}
              <div className="flex items-center space-x-2 pt-2">
                <button
                  onClick={() => toggleDeckPlay(i)}
                  className={`flex-1 py-2 rounded-xl flex items-center justify-center space-x-1.5 font-bold text-xs transition-all shadow-md active:scale-95 ${
                    isPlaying
                      ? 'bg-amber-600 hover:bg-amber-500 text-white'
                      : 'bg-emerald-600 hover:bg-emerald-500 text-white'
                  }`}
                >
                  {isPlaying ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
                  <span>{isPlaying ? 'PAUSE' : 'PLAY'}</span>
                </button>

                {/* Hot Cues 1 to 4 */}
                <div className="flex space-x-1">
                  {[1, 2, 3, 4].map((cueNum) => (
                    <button
                      key={cueNum}
                      className="w-8 h-8 rounded-lg bg-slate-800 hover:bg-rose-900/60 border border-slate-700 hover:border-rose-500 text-white font-mono font-bold text-xs transition-all active:scale-90"
                    >
                      {cueNum}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Central Crossfader Strip */}
      <div className="bg-dhun-surface border border-dhun-border p-4 rounded-2xl flex flex-col md:flex-row items-center justify-between gap-4">
        <div className="text-xs font-bold text-slate-400 uppercase tracking-wider flex items-center space-x-2">
          <span>SMART CROSSFADER:</span>
          <span className="text-emerald-400 font-mono font-bold">BASS SWAP ACTIVATED</span>
        </div>

        {/* Fader Slider */}
        <div className="flex-1 max-w-md w-full flex items-center space-x-4">
          <span className="text-xs font-bold font-mono text-cyan-400">A</span>
          <input
            type="range"
            min="-1"
            max="1"
            step="0.01"
            value={crossfader}
            onChange={(e) => setCrossfaderPos(parseFloat(e.target.value))}
            className="flex-1 accent-dhun-primary h-2 bg-slate-900 rounded-lg cursor-pointer"
          />
          <span className="text-xs font-bold font-mono text-orange-400">B</span>
        </div>

        <div className="text-xs font-mono text-slate-400">
          Position: <span className="text-white font-bold">{crossfader.toFixed(2)}</span>
        </div>
      </div>
    </div>
  );
};

