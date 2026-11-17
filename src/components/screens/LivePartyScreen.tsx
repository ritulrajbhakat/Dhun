import React, { useEffect, useState } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import {
  AlertTriangle,
  QrCode,
  Music,
  Check,
  X,
  Sliders,
  Radio,
} from 'lucide-react';

export const LivePartyScreen: React.FC = () => {
  const {
    cloudParty,
    partyConfig,
    guestUrl,
    qrSvg,
    registeredSources,
    songRequests,
    approveSongRequest,
    rejectSongRequest,
    endCloudParty,
    setCurrentView,
  } = useDhunStore();

  const [elapsedSecs, setElapsedSecs] = useState(0);

  useEffect(() => {
    if (!cloudParty || cloudParty.status !== 'live' || !cloudParty.started_at) {
      setElapsedSecs(0);
      return;
    }
    const updateElapsed = () => {
      setElapsedSecs(Math.max(0, Math.floor((Date.now() - (cloudParty.started_at || Date.now())) / 1000)));
    };
    updateElapsed();
    const timer = setInterval(updateElapsed, 1000);
    return () => clearInterval(timer);
  }, [cloudParty]);

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-7xl mx-auto space-y-6">
      {/* Header Bar */}
      <div className="flex flex-wrap items-center justify-between gap-4 bg-dhun-surface/90 border border-dhun-border p-5 rounded-2xl">
        <div className="flex items-center space-x-3">
          <div className={`w-3 h-3 rounded-full ${cloudParty?.status === 'live' ? 'bg-emerald-500 animate-ping' : 'bg-slate-500'}`} />
          <div>
            <div className="flex items-center space-x-2">
              <h1 className="text-xl font-black text-white">{cloudParty?.party_name || 'No Active Cloud Party'}</h1>
              {partyConfig && (
                <>
                  <span className="text-[10px] uppercase font-bold px-2 py-0.5 rounded bg-rose-950 text-rose-400 border border-rose-800">
                    {partyConfig.dj_level}
                  </span>
                  <span className="text-[10px] uppercase font-bold px-2 py-0.5 rounded bg-amber-950 text-amber-400 border border-amber-800">
                    {partyConfig.primary_mood_id}
                  </span>
                </>
              )}
            </div>
            <p className="text-xs text-slate-400 mt-0.5">
              {cloudParty?.status === 'live' ? (
                <>
                  Live for <span className="font-mono text-emerald-400 font-bold">{formatTime(elapsedSecs)}</span> &bull; Cloud Party Active
                </>
              ) : (
                'Create & start a cloud party to go live'
              )}
            </p>
          </div>
        </div>

        <div className="flex items-center space-x-3">
          <button
            onClick={() => setCurrentView('library')}
            className="flex items-center space-x-1.5 px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-bold text-slate-200 border border-slate-700"
          >
            <Sliders className="w-4 h-4 text-amber-400" />
            <span>Music Sources ({registeredSources.length})</span>
          </button>

          {cloudParty?.status === 'live' && (
            <button
              onClick={() => endCloudParty()}
              className="flex items-center space-x-1.5 px-4 py-2 rounded-xl bg-red-950 hover:bg-red-900 border border-red-800 text-xs font-bold text-red-200"
            >
              <AlertTriangle className="w-4 h-4 text-red-400" />
              <span>End Party</span>
            </button>
          )}
        </div>
      </div>

      {/* Main Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Registered YouTube Catalog */}
        <div className="lg:col-span-2 bg-dhun-surface border border-dhun-border rounded-2xl p-5 space-y-4">
          <h2 className="text-sm font-bold text-white flex items-center gap-2">
            <Music className="w-4 h-4 text-amber-400" />
            <span>Registered Online Music Sources</span>
          </h2>
          {registeredSources.length === 0 ? (
            <div className="text-center py-10 text-xs text-slate-400">
              No YouTube tracks registered yet. Add official YouTube links to populate this party.
            </div>
          ) : (
            <div className="space-y-2">
              {registeredSources.map((src) => (
                <div key={src.id} className="p-3 bg-slate-900/70 border border-slate-800 rounded-xl flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="text-sm font-bold text-white truncate">{src.title}</div>
                    <div className="text-xs text-slate-400 truncate">{src.artist}</div>
                  </div>
                  <span className="px-2 py-0.5 rounded bg-amber-500/15 border border-amber-500/30 text-[10px] font-mono text-amber-300 shrink-0">
                    {src.playability_status}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* QR & Live Requests */}
        <div className="space-y-6">
          <div className="bg-dhun-surface border border-dhun-border rounded-2xl p-5 flex flex-col items-center text-center">
            <h3 className="text-xs font-bold uppercase text-slate-300 mb-3 flex items-center gap-1.5">
              <QrCode className="w-4 h-4 text-amber-400" />
              <span>Guest QR Portal</span>
            </h3>
            {qrSvg ? (
              <>
                <div
                  className="w-40 h-40 bg-white p-2 rounded-xl overflow-hidden [&>svg]:w-full [&>svg]:h-full mb-2"
                  dangerouslySetInnerHTML={{ __html: qrSvg }}
                />
                <p className="text-[11px] font-mono text-slate-400 truncate max-w-full">{guestUrl}</p>
              </>
            ) : (
              <div className="py-8 text-xs text-slate-500 flex flex-col items-center gap-2">
                <Radio className="w-6 h-6 opacity-40" />
                <span>Start a cloud party to generate QR code.</span>
              </div>
            )}
          </div>

          <div className="bg-dhun-surface border border-dhun-border rounded-2xl p-5 space-y-3">
            <h3 className="text-xs font-bold uppercase text-slate-300">Guest Requests ({songRequests.length})</h3>
            {songRequests.length === 0 ? (
              <p className="text-xs text-slate-500">No requests submitted yet.</p>
            ) : (
              songRequests.map((r) => (
                <div key={r.id} className="p-2.5 bg-slate-900 rounded-xl border border-slate-800 flex items-center justify-between gap-2">
                  <div className="min-w-0">
                    <div className="text-xs font-bold text-white truncate">{r.source_title}</div>
                    <div className="text-[11px] text-slate-400 truncate">by {r.guest_name || 'Guest'} &bull; {r.status}</div>
                  </div>
                  {r.status === 'pending' && (
                    <div className="flex gap-1">
                      <button
                        onClick={() => approveSongRequest(r.id)}
                        className="p-1.5 rounded bg-emerald-600/30 hover:bg-emerald-600 text-emerald-300"
                      >
                        <Check className="w-3.5 h-3.5" />
                      </button>
                      <button
                        onClick={() => rejectSongRequest(r.id, 'Declined')}
                        className="p-1.5 rounded bg-rose-600/30 hover:bg-rose-600 text-rose-300"
                      >
                        <X className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
