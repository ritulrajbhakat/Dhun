import React from 'react';
import { Settings, Volume2, HardDrive, ShieldCheck, QrCode } from 'lucide-react';
import { useDhunStore } from '../../store/useDhunStore';

export const SettingsScreen: React.FC = () => {
  const { audioDevices } = useDhunStore();

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-4xl mx-auto space-y-6">
      <div className="border-b border-dhun-border pb-4">
        <h1 className="text-xl font-bold text-white flex items-center space-x-2">
          <Settings className="w-5 h-5 text-dhun-primary" />
          <span>Application Settings</span>
        </h1>
        <p className="text-xs text-slate-400">Configure audio hardware, rolling cache policies, and local networking</p>
      </div>

      {/* Audio Hardware Settings */}
      <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-4">
        <h2 className="text-sm font-bold uppercase tracking-wider text-slate-300 flex items-center space-x-2">
          <Volume2 className="w-4 h-4 text-dhun-accent" />
          <span>Audio Engine &amp; Device Setup</span>
        </h2>

        <div className="space-y-3 text-xs">
          <div>
            <label className="text-slate-400 block mb-1 font-semibold">Primary Output Soundcard</label>
            <select className="w-full bg-dhun-surface border border-dhun-border rounded-xl px-4 py-2.5 text-slate-200">
              {audioDevices.map((d) => (
                <option key={d.name} value={d.name}>{d.name} ({d.default_sample_rate}Hz)</option>
              ))}
            </select>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-slate-400 block mb-1 font-semibold">Buffer Size</label>
              <select defaultValue="512" className="w-full bg-dhun-surface border border-dhun-border rounded-xl px-4 py-2.5 text-slate-200">
                <option value="256">256 samples (~5.8ms)</option>
                <option value="512">512 samples (~11.6ms - Recommended)</option>
                <option value="1024">1024 samples (~23.2ms - High Stability)</option>
              </select>
            </div>

            <div>
              <label className="text-slate-400 block mb-1 font-semibold">Brickwall Peak Limiter</label>
              <select defaultValue="enabled" className="w-full bg-dhun-surface border border-dhun-border rounded-xl px-4 py-2.5 text-slate-200">
                <option value="enabled">Enabled (-0.1 dBFS ceiling)</option>
                <option value="bypass">Bypass</option>
              </select>
            </div>
          </div>
        </div>
      </div>

      {/* Cache & Storage Management */}
      <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-4">
        <h2 className="text-sm font-bold uppercase tracking-wider text-slate-300 flex items-center space-x-2">
          <HardDrive className="w-4 h-4 text-dhun-accent" />
          <span>Rolling Audio Cache Policy</span>
        </h2>

        <div className="text-xs text-slate-400 space-y-2">
          <p>
            DHUN preloads the next 2 tracks into memory and maintains an upcoming cache target of approximately 10 tracks.
            Active and queued audio assets are strictly protected by deck locks.
          </p>
          <div className="flex items-center justify-between pt-2">
            <span>Inactivity Cleanup:</span>
            <span className="font-bold text-white">After 3 hours of idle party state</span>
          </div>
          <div className="flex items-center justify-between">
            <span>Permanent Track Metadata:</span>
            <span className="font-bold text-emerald-400">Strictly preserved in SQLite</span>
          </div>
        </div>
      </div>
    </div>
  );
};

