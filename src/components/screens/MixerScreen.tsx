import React from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { Sliders } from 'lucide-react';

export const MixerScreen: React.FC = () => {
  const {
    exposedDecks,
    internalDeckCapacity,
    decks,
    crossfader,
    setCrossfaderPos,
    updateDeckEq,
    updateDeckGain,
    updateDeckFilter,
    selectedMoodId,
    moodProfiles,
  } = useDhunStore();

  const activeMood = moodProfiles.find((m) => m.id === selectedMoodId) || moodProfiles[0];

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-6xl mx-auto space-y-6">
      <div className="flex items-center justify-between border-b border-dhun-border pb-4">
        <div>
          <h1 className="text-xl font-bold text-white flex items-center space-x-2">
            <Sliders className="w-5 h-5 text-amber-400" />
            <span>Web Audio {exposedDecks}-Deck Mixer ({internalDeckCapacity}-Deck Architecture)</span>
          </h1>
          <p className="text-xs text-slate-400">
            Active Transition Profile: {activeMood.transition_style} &bull; Automated DJ active across all 4 levels
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {Array.from({ length: exposedDecks }).map((_, i) => (
          <div key={i} className="bg-dhun-surface border border-dhun-border rounded-2xl p-4 space-y-3">
            <div className="text-sm font-black text-white flex justify-between">
              <span>Deck {String.fromCharCode(65 + i)}</span>
              <span className="text-[10px] font-mono text-amber-300">
                {i % 2 === 0 ? 'XF LEFT' : 'XF RIGHT'}
              </span>
            </div>

            <div className="space-y-2 text-xs">
              <div>
                <label className="text-slate-400">Gain ({Math.round(decks[i].gain * 100)}%)</label>
                <input
                  type="range"
                  min="0"
                  max="1.2"
                  step="0.05"
                  value={decks[i].gain}
                  onChange={(e) => updateDeckGain(i, Number(e.target.value))}
                  className="w-full accent-amber-400"
                />
              </div>
              <div>
                <label className="text-slate-400">Low EQ ({decks[i].eqLow} dB)</label>
                <input
                  type="range"
                  min="-24"
                  max="6"
                  value={decks[i].eqLow}
                  onChange={(e) => updateDeckEq(i, Number(e.target.value), decks[i].eqMid, decks[i].eqHigh)}
                  className="w-full accent-rose-400"
                />
              </div>
              <div>
                <label className="text-slate-400">Mid EQ ({decks[i].eqMid} dB)</label>
                <input
                  type="range"
                  min="-24"
                  max="6"
                  value={decks[i].eqMid}
                  onChange={(e) => updateDeckEq(i, decks[i].eqLow, Number(e.target.value), decks[i].eqHigh)}
                  className="w-full accent-sky-400"
                />
              </div>
              <div>
                <label className="text-slate-400">High EQ ({decks[i].eqHigh} dB)</label>
                <input
                  type="range"
                  min="-24"
                  max="6"
                  value={decks[i].eqHigh}
                  onChange={(e) => updateDeckEq(i, decks[i].eqLow, decks[i].eqMid, Number(e.target.value))}
                  className="w-full accent-teal-400"
                />
              </div>
              <div>
                <label className="text-slate-400">Filter ({decks[i].filter.toFixed(2)})</label>
                <input
                  type="range"
                  min="-1"
                  max="1"
                  step="0.05"
                  value={decks[i].filter}
                  onChange={(e) => updateDeckFilter(i, Number(e.target.value))}
                  className="w-full accent-purple-400"
                />
              </div>
            </div>
          </div>
        ))}
      </div>

      <div className="bg-dhun-surface border border-dhun-border rounded-2xl p-5 space-y-2">
        <div className="flex justify-between text-xs font-bold text-slate-300">
          <span>Decks A / C</span>
          <span>Master Crossfader</span>
          <span>Decks B / D</span>
        </div>
        <input
          type="range"
          min="-1"
          max="1"
          step="0.02"
          value={crossfader}
          onChange={(e) => setCrossfaderPos(Number(e.target.value))}
          className="w-full accent-amber-400"
        />
      </div>
    </div>
  );
};
