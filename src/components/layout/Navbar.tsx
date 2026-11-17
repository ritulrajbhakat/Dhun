import React from 'react';
import { useDhunStore, ScreenView } from '../../store/useDhunStore';
import { Disc3, Radio, Sliders, Music, Settings, Activity } from 'lucide-react';

export const Navbar: React.FC = () => {
  const { currentView, setCurrentView, activeRoom } = useDhunStore();

  const items: { id: ScreenView; label: string; icon: React.ReactNode }[] = [
    { id: 'home', label: 'Home', icon: <Disc3 className="w-4 h-4" /> },
    { id: 'live', label: 'Live Party', icon: <Radio className="w-4 h-4" /> },
    { id: 'mixer', label: '4-Deck Mixer', icon: <Sliders className="w-4 h-4" /> },
    { id: 'library', label: 'DJ Pool', icon: <Music className="w-4 h-4" /> },
    { id: 'diagnostics', label: 'Diagnostics', icon: <Activity className="w-4 h-4" /> },
    { id: 'settings', label: 'Settings', icon: <Settings className="w-4 h-4" /> },
  ];

  return (
    <nav className="h-14 border-b border-dhun-border bg-dhun-surface px-4 flex items-center justify-between">
      <div className="flex items-center space-x-2">
        <span className="text-lg font-black text-amber-400">धुन DHUN</span>
        {activeRoom?.status === 'live' && (
          <span className="text-[10px] font-bold uppercase px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/40">
            LAN LIVE
          </span>
        )}
      </div>

      <div className="flex items-center space-x-1">
        {items.map((item) => (
          <button
            key={item.id}
            onClick={() => setCurrentView(item.id)}
            className={`px-3 py-1.5 rounded-lg text-xs font-bold flex items-center space-x-1.5 transition-colors ${
              currentView === item.id
                ? 'bg-amber-500/20 text-amber-300 border border-amber-400/40'
                : 'text-slate-400 hover:text-white'
            }`}
          >
            {item.icon}
            <span>{item.label}</span>
          </button>
        ))}
      </div>
    </nav>
  );
};
