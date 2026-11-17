import React from 'react';
import { Activity, HardDrive, Wifi, CloudUpload } from 'lucide-react';
import { useDhunStore } from '../../store/useDhunStore';

export const DiagnosticsScreen: React.FC = () => {
  const { helperConnected, lanReachable, detectedLanIp, activeRoom, cacheStats, syncOverview } = useDhunStore();

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-5xl mx-auto space-y-6">
      <div className="border-b border-dhun-border pb-4">
        <h1 className="text-xl font-bold text-white flex items-center space-x-2">
          <Activity className="w-5 h-5 text-dhun-primary" />
          <span>Local Helper Diagnostics</span>
        </h1>
        <p className="text-xs text-slate-400">Real-time health of LAN helper, rolling 15-track temp cache, and offline metadata sync outbox</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-1">
          <div className="text-xs text-slate-400 flex items-center gap-1.5">
            <Wifi className="w-4 h-4 text-emerald-400" />
            <span>LAN Network</span>
          </div>
          <div className="text-lg font-black text-white">
            {helperConnected ? (lanReachable ? detectedLanIp : 'Loopback Only') : 'Helper Offline'}
          </div>
          <div className="text-xs text-slate-400">Room: {activeRoom?.status || 'none'}</div>
        </div>

        <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-1">
          <div className="text-xs text-slate-400 flex items-center gap-1.5">
            <HardDrive className="w-4 h-4 text-amber-400" />
            <span>Temp Audio Cache</span>
          </div>
          <div className="text-lg font-black text-white font-mono">
            {cacheStats.track_count} / {cacheStats.max_tracks} tracks
          </div>
          <div className="text-xs text-slate-400">{cacheStats.locked_tracks} priority-locked</div>
        </div>

        <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-1">
          <div className="text-xs text-slate-400 flex items-center gap-1.5">
            <CloudUpload className="w-4 h-4 text-sky-400" />
            <span>Metadata Sync Outbox</span>
          </div>
          <div className="text-lg font-black text-white font-mono">
            {syncOverview.pending_count} pending / {syncOverview.synced_count} synced
          </div>
          <div className="text-xs text-slate-400">Metadata only — zero audio uploaded</div>
        </div>
      </div>
    </div>
  );
};
