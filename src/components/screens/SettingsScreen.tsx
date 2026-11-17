import React, { useState } from 'react';
import { Settings, Server, ShieldCheck, RefreshCw } from 'lucide-react';
import { useDhunStore } from '../../store/useDhunStore';

export const SettingsScreen: React.FC = () => {
  const { apiBaseUrl, backendConnected, backendVersion, backendError, setApiBaseUrl } = useDhunStore();
  const [urlDraft, setUrlDraft] = useState(apiBaseUrl);
  const [saving, setSaving] = useState(false);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    await setApiBaseUrl(urlDraft);
    setSaving(false);
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-4xl mx-auto space-y-6">
      <div className="border-b border-dhun-border pb-4">
        <h1 className="text-xl font-bold text-white flex items-center space-x-2">
          <Settings className="w-5 h-5 text-dhun-primary" />
          <span>Cloud Backend &amp; Party Settings</span>
        </h1>
        <p className="text-xs text-slate-400">Configure your deployed DHUN cloud backend URL and security preferences</p>
      </div>

      {/* Cloud Backend Settings */}
      <form onSubmit={handleSave} className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-bold uppercase tracking-wider text-slate-300 flex items-center space-x-2">
            <Server className="w-4 h-4 text-sky-400" />
            <span>DHUN Cloud Server Endpoint</span>
          </h2>
          <span
            className={`text-xs font-bold px-2.5 py-0.5 rounded-full ${
              backendConnected
                ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
                : 'bg-rose-500/20 text-rose-300 border border-rose-500/40'
            }`}
          >
            {backendConnected ? `Connected (v${backendVersion || '1.0.0'})` : 'Offline'}
          </span>
        </div>

        <div className="space-y-2 text-xs">
          <label className="text-slate-400 block font-semibold">API Base URL</label>
          <div className="flex gap-2">
            <input
              type="url"
              value={urlDraft}
              onChange={(e) => setUrlDraft(e.target.value)}
              placeholder="https://your-dhun-server.onrender.com"
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
          {!backendConnected && backendError && (
            <p className="text-rose-400 text-[11px]">{backendError}</p>
          )}
        </div>
      </form>

      {/* Security & Compliance Policy */}
      <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-3">
        <h2 className="text-sm font-bold uppercase tracking-wider text-slate-300 flex items-center space-x-2">
          <ShieldCheck className="w-4 h-4 text-emerald-400" />
          <span>Source Policy &amp; Host Authentication</span>
        </h2>
        <p className="text-xs text-slate-400 leading-relaxed">
          DHUN Web Application operates exclusively with official YouTube &amp; YouTube Music link references.
          Host control tokens are SHA-256 hashed on the backend, and guests interact strictly through authenticated session tokens with rate limiting.
        </p>
      </div>
    </div>
  );
};
