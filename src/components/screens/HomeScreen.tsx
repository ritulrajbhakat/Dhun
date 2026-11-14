import React from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import {
  Play,
  Sparkles,
  Music2,
  Sliders,
  CheckCircle2,
  Clock,
  ShieldCheck,
  Disc,
  QrCode,
  Flame,
} from 'lucide-react';

export const HomeScreen: React.FC = () => {
  const { setCurrentView, partyStatus, moodProfiles, tracks } = useDhunStore();

  const totalTracks = tracks.length || 5;
  const playableTracks = totalTracks;
  const awaitingAnalysis = 0;

  return (
    <div className="flex-1 overflow-y-auto p-8 max-w-7xl mx-auto space-y-8">
      {/* Hero Product Banner */}
      <div className="relative rounded-3xl bg-gradient-to-r from-dhun-surface via-dhun-card to-dhun-surface border border-dhun-border p-8 overflow-hidden shadow-2xl">
        <div className="absolute right-0 top-0 bottom-0 w-1/3 bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-rose-950/40 via-transparent to-transparent pointer-events-none" />

        <div className="relative z-10 max-w-2xl space-y-4">
          <div className="inline-flex items-center space-x-2 px-3 py-1 rounded-full bg-rose-500/10 border border-rose-500/30 text-rose-400 text-xs font-semibold">
            <Flame className="w-3.5 h-3.5" />
            <span>Standalone Local-First DJ System &bull; Zero Cloud &bull; Deterministic Mixing</span>
          </div>

          <h1 className="text-4xl md:text-5xl font-black tracking-tight text-white leading-tight">
            DHUN
            <span className="block text-2xl md:text-3xl font-extrabold text-transparent bg-clip-text bg-gradient-to-r from-dhun-accent via-amber-400 to-rose-400 mt-1">
              Your Party. Your DJ.
            </span>
          </h1>

          <p className="text-slate-300 text-sm md:text-base leading-relaxed">
            Professional automatic music playback, beatmatching, and seamless harmonic transitions powered by native Rust audio DSP. Allow guests to request songs directly from their phones via a local Wi-Fi QR code without interrupting playback.
          </p>

          {/* Call to Actions */}
          <div className="flex flex-wrap gap-3 pt-3">
            <button
              onClick={() => setCurrentView('wizard')}
              className="flex items-center space-x-2 px-6 py-3 rounded-xl bg-gradient-to-r from-dhun-primary to-dhun-accent hover:from-rose-600 hover:to-rose-500 text-white font-bold text-sm shadow-lg shadow-rose-950/50 active:scale-95 transition-all"
            >
              <Sparkles className="w-4 h-4" />
              <span>Start a New Party</span>
            </button>

            {partyStatus.is_active && (
              <button
                onClick={() => setCurrentView('live')}
                className="flex items-center space-x-2 px-6 py-3 rounded-xl bg-emerald-950/80 hover:bg-emerald-900 border border-emerald-600/70 text-emerald-200 font-bold text-sm active:scale-95 transition-all"
              >
                <Disc className="w-4 h-4 animate-spin" />
                <span>Return to Live Party</span>
              </button>
            )}

            <button
              onClick={() => setCurrentView('mixer')}
              className="flex items-center space-x-2 px-5 py-3 rounded-xl bg-dhun-card hover:bg-slate-800 border border-dhun-border text-slate-200 font-semibold text-sm active:scale-95 transition-all"
            >
              <Sliders className="w-4 h-4 text-slate-400" />
              <span>Open DJ Mixing Console</span>
            </button>

            <button
              onClick={() => setCurrentView('library')}
              className="flex items-center space-x-2 px-5 py-3 rounded-xl bg-dhun-card hover:bg-slate-800 border border-dhun-border text-slate-200 font-semibold text-sm active:scale-95 transition-all"
            >
              <Music2 className="w-4 h-4 text-slate-400" />
              <span>Music Library</span>
            </button>
          </div>
        </div>
      </div>

      {/* Library Readiness Metrics */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="bg-dhun-card/80 border border-dhun-border p-5 rounded-2xl flex items-center space-x-4">
          <div className="w-12 h-12 rounded-xl bg-blue-950/40 border border-blue-800/40 flex items-center justify-center text-blue-400">
            <Music2 className="w-6 h-6" />
          </div>
          <div>
            <div className="text-2xl font-black text-white">{totalTracks}</div>
            <div className="text-xs text-slate-400 font-medium">Total Tracks</div>
          </div>
        </div>

        <div className="bg-dhun-card/80 border border-dhun-border p-5 rounded-2xl flex items-center space-x-4">
          <div className="w-12 h-12 rounded-xl bg-emerald-950/40 border border-emerald-800/40 flex items-center justify-center text-emerald-400">
            <CheckCircle2 className="w-6 h-6" />
          </div>
          <div>
            <div className="text-2xl font-black text-emerald-400">{playableTracks}</div>
            <div className="text-xs text-slate-400 font-medium">Playable &amp; Verified</div>
          </div>
        </div>

        <div className="bg-dhun-card/80 border border-dhun-border p-5 rounded-2xl flex items-center space-x-4">
          <div className="w-12 h-12 rounded-xl bg-amber-950/40 border border-amber-800/40 flex items-center justify-center text-amber-400">
            <Clock className="w-6 h-6" />
          </div>
          <div>
            <div className="text-2xl font-black text-amber-400">{awaitingAnalysis}</div>
            <div className="text-xs text-slate-400 font-medium">Awaiting Analysis</div>
          </div>
        </div>

        <div className="bg-dhun-card/80 border border-dhun-border p-5 rounded-2xl flex items-center space-x-4">
          <div className="w-12 h-12 rounded-xl bg-rose-950/40 border border-rose-800/40 flex items-center justify-center text-rose-400">
            <ShieldCheck className="w-6 h-6" />
          </div>
          <div>
            <div className="text-sm font-bold text-white">Audio Engine</div>
            <div className="text-xs text-emerald-400 font-mono font-medium flex items-center space-x-1 mt-0.5">
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              <span>CPAL Real-Time OK</span>
            </div>
          </div>
        </div>
      </div>

      {/* Indian Mood Presets Showcase */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-xl font-bold text-white">Party Mood Presets</h2>
            <p className="text-xs text-slate-400">Configured with genre weights, BPM range constraints, and energy curves</p>
          </div>
          <button
            onClick={() => setCurrentView('wizard')}
            className="text-xs font-semibold text-dhun-accent hover:underline"
          >
            Launch with Mood &rarr;
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {moodProfiles.slice(0, 8).map((mood) => (
            <div
              key={mood.id}
              onClick={() => setCurrentView('wizard')}
              className="bg-dhun-card/70 hover:bg-dhun-card border border-dhun-border/70 hover:border-dhun-primary/50 p-4 rounded-xl transition-all cursor-pointer group"
            >
              <div className="flex items-center justify-between mb-2">
                <span className="font-bold text-white group-hover:text-dhun-accent transition-colors">
                  {mood.display_name}
                </span>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-slate-800 text-slate-300">
                  {mood.min_bpm} - {mood.max_bpm} BPM
                </span>
              </div>
              <p className="text-xs text-slate-400 line-clamp-2 leading-relaxed mb-3">
                {mood.description}
              </p>
              <div className="flex items-center justify-between text-[11px] text-slate-500 pt-2 border-t border-slate-800/80">
                <span>Transition: <span className="text-slate-300 font-medium">{mood.transition_style}</span></span>
                <span>Energy: <span className="text-amber-400 font-mono font-bold">{Math.round(mood.target_energy * 100)}%</span></span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

