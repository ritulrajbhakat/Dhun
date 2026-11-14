import React from 'react';
import { useDhunStore, ScreenView } from '../../store/useDhunStore';
import {
  Disc3,
  Sliders,
  Music2,
  QrCode,
  Radio,
  History,
  Settings,
  Activity,
  AlertTriangle,
  Volume2,
  Sparkles,
} from 'lucide-react';

export const Navbar: React.FC = () => {
  const { currentView, setCurrentView, partyStatus, emergencyFade, masterVolume, setMasterVolume } = useDhunStore();

  const navItems: { view: ScreenView; label: string; icon: React.ReactNode; badge?: string }[] = [
    { view: 'home', label: 'Home', icon: <Radio className="w-4 h-4" /> },
    { view: 'wizard', label: 'Setup Party', icon: <Sparkles className="w-4 h-4" /> },
    {
      view: 'live',
      label: 'Live Party',
      icon: <Disc3 className="w-4 h-4" />,
      badge: partyStatus.is_active ? 'LIVE' : undefined,
    },
    { view: 'mixer', label: 'DJ Console', icon: <Sliders className="w-4 h-4" /> },
    { view: 'library', label: 'Music Library', icon: <Music2 className="w-4 h-4" /> },
    { view: 'requests', label: 'Guest Requests', icon: <QrCode className="w-4 h-4" /> },
    { view: 'history', label: 'History', icon: <History className="w-4 h-4" /> },
    { view: 'settings', label: 'Settings', icon: <Settings className="w-4 h-4" /> },
    { view: 'diagnostics', label: 'Diagnostics', icon: <Activity className="w-4 h-4" /> },
  ];

  return (
    <header className="h-16 bg-dhun-surface border-b border-dhun-border flex items-center justify-between px-5 select-none z-50">
      {/* Brand Identity */}
      <div
        className="flex items-center space-x-3 cursor-pointer group"
        onClick={() => setCurrentView('home')}
      >
        <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-dhun-primary to-dhun-accent flex items-center justify-center shadow-lg shadow-rose-950/40 group-hover:scale-105 transition-transform">
          <Disc3 className="w-6 h-6 text-white animate-[spin_10s_linear_infinite]" />
        </div>
        <div>
          <div className="flex items-center space-x-2">
            <span className="font-black text-xl tracking-wider text-white">DHUN</span>
            <span className="text-[10px] font-bold px-1.5 py-0.5 rounded bg-dhun-primary/20 text-dhun-accent border border-dhun-primary/40 uppercase">
              Pro DJ
            </span>
          </div>
          <p className="text-[11px] text-slate-400 font-medium">Your Party. Your DJ.</p>
        </div>
      </div>

      {/* Primary Navigation Tabs */}
      <nav className="flex items-center space-x-1 bg-dhun-card/80 p-1 rounded-xl border border-dhun-border/70">
        {navItems.map((item) => {
          const isActive = currentView === item.view;
          return (
            <button
              key={item.view}
              onClick={() => setCurrentView(item.view)}
              className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all relative ${
                isActive
                  ? 'bg-dhun-primary text-white shadow-md shadow-rose-900/30'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/40'
              }`}
            >
              {item.icon}
              <span>{item.label}</span>
              {item.badge && (
                <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse ml-1" />
              )}
            </button>
          );
        })}
      </nav>

      {/* Master Audio Controls & Emergency Stop */}
      <div className="flex items-center space-x-4">
        {/* Master Output Volume */}
        <div className="flex items-center space-x-2 bg-dhun-card px-3 py-1.5 rounded-lg border border-dhun-border">
          <Volume2 className="w-4 h-4 text-slate-400" />
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={masterVolume}
            onChange={(e) => setMasterVolume(parseFloat(e.target.value))}
            className="w-20 accent-dhun-primary h-1 bg-slate-700 rounded-lg cursor-pointer"
          />
          <span className="text-[11px] font-mono text-slate-300 w-8 text-right">
            {Math.round(masterVolume * 100)}%
          </span>
        </div>

        {/* Emergency Fade Button */}
        <button
          onClick={emergencyFade}
          title="Instantly fades out all music smoothly over 300ms without pops"
          className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-red-950/80 hover:bg-red-900 border border-red-700/80 text-red-200 text-xs font-bold transition-all active:scale-95 shadow-sm"
        >
          <AlertTriangle className="w-3.5 h-3.5 text-red-400 animate-bounce" />
          <span>EMERGENCY FADE</span>
        </button>
      </div>
    </header>
  );
};

