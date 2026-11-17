import React, { useState } from 'react';
import { Settings, Laptop, ShieldCheck, RefreshCw } from 'lucide-react';
import { useDhunStore } from '../../store/useDhunStore';

export const SettingsScreen: React.FC = () => {
  const { helperUrl, helperConnected, helperVersion, helperError, setHelperUrl } = useDhunStore();
  const [urlDraft, setUrlDraft] = useState(helperUrl);
  const [saving, setSaving] = useState(false);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    await setHelperUrl(urlDraft);
    setSaving(false);
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-4xl mx-auto space-y-6">
      <div className="border-b border-dhun-border pb-4">
        <h1 className="text-xl font-bold text-white flex items-center space-x-2">
          <Settings className="w-5 h-5 text-dhun-primary" />
          <span>Local DHUN Helper Settings</span>
        </h1>
        <p className="text-xs text-slate-400">Configure connection to the local helper service running on this host laptop</p>
      </div>

      <form onSubmit={handleSave} className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-bold uppercase tracking-wider text-slate-300 flex items-center space-x-2">
            <Laptop className="w-4 h-4 text-sky-400" />
            <span>Local Helper Loopback Endpoint</span>
          </h2>
          <span
            className={`text-xs font-bold px-2.5 py-0.5 rounded-full ${
              helperConnected
                ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                : 'bg-rose-500/20 text-rose-300 border border-rose-500/40'
            }`}
          >
            {helperConnected ? `Running (v${helperVersion || '2.0.0'})` : 'Not Detected'}
          </span>
        </div>

        <div className="space-y-2 text-xs">
          <label className="text-slate-400 block font-semibold">Local Helper URL</label>
          <div className="flex gap-2">
            <input
              type="url"
              value={urlDraft}
              onChange={(e) => setUrlDraft(e.target.value)}
              placeholder="http://127.0.0.1:47821"
              className="flex-1 bg-dhun-surface border border-dhun-border rounded-xl px-4 py-2.5 text-slate-200 font-mono"
            />
            <button
              type="submit"
              disabled={saving}
              className="px-4 py-2.5 bg-sky-500 hover:bg-sky-400 text-black font-bold rounded-xl flex items-center gap-1.5"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${saving ? 'animate-spin' : ''}`} />
              <span>Save &amp; Connect</span>
            </button>
          </div>
          {!helperConnected && helperError && <p className="text-rose-400 text-[11px]">{helperError}</p>}
        </div>
      </form>

      <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-3">
        <h2 className="text-sm font-bold uppercase tracking-wider text-slate-300 flex items-center space-x-2">
          <ShieldCheck className="w-4 h-4 text-emerald-400" />
          <span>Privacy, LAN Isolation &amp; Metadata Boundary</span>
        </h2>
        <p className="text-xs text-slate-400 leading-relaxed">
          Party rooms, guest tokens, private song submissions, and temporary local audio caches remain strictly on this laptop and your local Wi-Fi/LAN.
          Only reusable song analysis metadata is synchronized by the automated DJ engine to the separate Global Shared Song Metadata Catalogue.
        </p>
      </div>
    </div>
  );
};
