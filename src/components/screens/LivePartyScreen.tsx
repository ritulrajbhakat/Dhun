import React, { useEffect, useState } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { AlertTriangle, QrCode, Music, Sliders, Radio } from 'lucide-react';

export const LivePartyScreen: React.FC = () => {
  const {
    activeRoom,
    djLevelName,
    guestUrl,
    qrSvg,
    lanReachable,
    submissions,
    closeLocalRoom,
    setCurrentView,
  } = useDhunStore();

  const [elapsedSecs, setElapsedSecs] = useState(0);

  useEffect(() => {
    if (!activeRoom || activeRoom.status !== 'live' || !activeRoom.started_at) {
      setElapsedSecs(0);
      return;
    }
    const updateElapsed = () => {
      setElapsedSecs(Math.max(0, Math.floor((Date.now() - (activeRoom.started_at || Date.now())) / 1000)));
    };
    updateElapsed();
    const timer = setInterval(updateElapsed, 1000);
    return () => clearInterval(timer);
  }, [activeRoom]);

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-7xl mx-auto space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-4 bg-dhun-surface/90 border border-dhun-border p-5 rounded-2xl">
        <div className="flex items-center space-x-3">
          <div className={`w-3 h-3 rounded-full ${activeRoom?.status === 'live' ? 'bg-emerald-500 animate-ping' : 'bg-slate-500'}`} />
          <div>
            <div className="flex items-center space-x-2">
              <h1 className="text-xl font-black text-white">{activeRoom?.party_name || 'No Active Local Room'}</h1>
              {activeRoom && (
                <>
                  <span className="text-[10px] uppercase font-bold px-2 py-0.5 rounded bg-rose-950 text-rose-400 border border-rose-800">
                    {djLevelName}
                  </span>
                  <span className="text-[10px] uppercase font-bold px-2 py-0.5 rounded bg-amber-950 text-amber-400 border border-amber-800">
                    {activeRoom.primary_mood_id}
                  </span>
                </>
              )}
            </div>
            <p className="text-xs text-slate-400 mt-0.5">
              {activeRoom?.status === 'live' ? (
                <>
                  Live for <span className="font-mono text-emerald-400 font-bold">{formatTime(elapsedSecs)}</span> &bull; Private LAN Room ({activeRoom.room_code})
                </>
              ) : (
                'Create & start a private local party room on your host helper'
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
            <span>DJ Pool ({submissions.length})</span>
          </button>

          {activeRoom?.status === 'live' && (
            <button
              onClick={() => closeLocalRoom()}
              className="flex items-center space-x-1.5 px-4 py-2 rounded-xl bg-red-950 hover:bg-red-900 border border-red-800 text-xs font-bold text-red-200"
            >
              <AlertTriangle className="w-4 h-4 text-red-400" />
              <span>Close Party &amp; Purge Cache</span>
            </button>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 bg-dhun-surface border border-dhun-border rounded-2xl p-5 space-y-4">
          <h2 className="text-sm font-bold text-white flex items-center gap-2">
            <Music className="w-4 h-4 text-amber-400" />
            <span>Automated DJ Private Candidate Pool (Zero Approval Queue)</span>
          </h2>
          {submissions.length === 0 ? (
            <div className="text-center py-10 text-xs text-slate-400">
              No submissions yet. Both host and LAN guests submit YouTube / YouTube Music links directly into the DJ pool.
            </div>
          ) : (
            <div className="space-y-2">
              {submissions.map((sub) => (
                <div key={sub.id} className="p-3 bg-slate-900/70 border border-slate-800 rounded-xl flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="text-sm font-bold text-white truncate">{sub.title}</div>
                    <div className="text-xs text-slate-400 truncate">
                      {sub.artist} &bull; via {sub.submitted_by_name} &bull; DJ Score {sub.dj_score.toFixed(1)}
                    </div>
                  </div>
                  <span className="px-2 py-0.5 rounded bg-teal-500/15 border border-teal-500/30 text-[10px] font-mono text-teal-300 shrink-0">
                    {sub.dj_status}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="bg-dhun-surface border border-dhun-border rounded-2xl p-5 flex flex-col items-center text-center">
          <h3 className="text-xs font-bold uppercase text-slate-300 mb-3 flex items-center gap-1.5">
            <QrCode className="w-4 h-4 text-amber-400" />
            <span>Same-Wi-Fi Guest QR</span>
          </h3>
          {lanReachable && qrSvg ? (
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
              <span>Create a local room on a Wi-Fi/LAN network to generate QR code.</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
