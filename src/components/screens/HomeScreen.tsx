import React from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { Radio, Sparkles, Sliders } from 'lucide-react';

export const HomeScreen: React.FC = () => {
  const { activeRoom, submissions, setCurrentView, helperConnected } = useDhunStore();

  return (
    <div className="flex-1 overflow-y-auto p-8 max-w-5xl mx-auto space-y-6">
      <div className="bg-dhun-surface border border-dhun-border rounded-3xl p-8 space-y-4">
        <div className="flex items-center space-x-2 text-amber-400 text-xs font-bold uppercase tracking-widest">
          <Sparkles className="w-4 h-4" />
          <span>DHUN धुन — Local Automatic Party DJ</span>
        </div>
        <h1 className="text-3xl font-black text-white">
          {activeRoom ? activeRoom.party_name : 'Ready to Host a Private LAN Party'}
        </h1>
        <p className="text-sm text-slate-400 max-w-2xl">
          {helperConnected
            ? 'Your Local DHUN Helper is running on this laptop. Create a private Wi-Fi party room, choose from 14 moods and 4 DJ levels, and let guests submit songs via QR code.'
            : 'Start dhun-local-helper on this laptop to host a private Wi-Fi party room.'}
        </p>

        <div className="flex gap-3 pt-2">
          <button
            onClick={() => setCurrentView('wizard')}
            className="px-5 py-2.5 bg-amber-500 hover:bg-amber-400 text-black font-black text-xs rounded-xl flex items-center gap-2"
          >
            <Radio className="w-4 h-4" />
            <span>Setup Local Party</span>
          </button>
          <button
            onClick={() => setCurrentView('library')}
            className="px-5 py-2.5 bg-slate-800 hover:bg-slate-700 text-white font-bold text-xs rounded-xl flex items-center gap-2"
          >
            <Sliders className="w-4 h-4" />
            <span>DJ Candidate Pool ({submissions.length})</span>
          </button>
        </div>
      </div>
    </div>
  );
};
