import React, { useEffect, useState } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import {
  Disc,
  Play,
  Pause,
  SkipForward,
  AlertTriangle,
  QrCode,
  Music,
  Flame,
  Check,
  X,
  FastForward,
  Activity,
  Sliders,
  Volume2,
} from 'lucide-react';

export const LivePartyScreen: React.FC = () => {
  const {
    partyStatus,
    telemetry,
    guestRequests,
    approveRequest,
    rejectRequest,
    forceNextRequest,
    emergencyFade,
    setCurrentView,
    startBrowserAudio,
    loadLocalAudioFile,
  } = useDhunStore();

  const [elapsedSecs, setElapsedSecs] = useState(0);
  const [audioActive, setAudioActive] = useState(false);
  const fileInputRef = React.useRef<HTMLInputElement | null>(null);

  const handleStartAudio = async () => {
    await startBrowserAudio();
    setAudioActive(true);
  };

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      await loadLocalAudioFile(0, file);
      setAudioActive(true);
    }
  };

  useEffect(() => {
    const timer = setInterval(() => {
      setElapsedSecs((s) => s + 1);
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  };

  const nowPlaying = partyStatus.now_playing;
  const nextTrack = partyStatus.next_track;
  const followingTrack = partyStatus.following_track;

  const deck0 = telemetry?.decks[0];
  const progressPct = deck0 && deck0.duration_ms > 0
    ? (deck0.position_ms / deck0.duration_ms) * 100
    : 35;

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-7xl mx-auto space-y-6">
      {/* Header Bar */}
      <div className="flex flex-wrap items-center justify-between gap-4 bg-dhun-surface/90 border border-dhun-border p-5 rounded-2xl">
        <div className="flex items-center space-x-3">
          <div className="w-3 h-3 rounded-full bg-emerald-500 animate-ping" />
          <div>
            <div className="flex items-center space-x-2">
              <h1 className="text-xl font-black text-white">{partyStatus.party_name || 'Active DHUN Party'}</h1>
              <span className="text-[10px] uppercase font-bold px-2 py-0.5 rounded bg-rose-950 text-rose-400 border border-rose-800">
                Level {partyStatus.dj_level || 2}
              </span>
              <span className="text-[10px] uppercase font-bold px-2 py-0.5 rounded bg-amber-950 text-amber-400 border border-amber-800">
                {partyStatus.primary_mood_id || 'Shaadi'}
              </span>
            </div>
            <p className="text-xs text-slate-400 mt-0.5">
              Party running for <span className="font-mono text-emerald-400 font-bold">{formatTime(elapsedSecs)}</span> &bull; Automatic DJ Active
            </p>
          </div>
        </div>

        {/* Quick Actions */}
        <div className="flex items-center space-x-3">
          {!audioActive && (
            <button
              onClick={handleStartAudio}
              className="flex items-center space-x-2 px-4 py-2 rounded-xl bg-gradient-to-r from-dhun-primary to-dhun-accent hover:from-rose-600 hover:to-rose-500 text-white font-black text-xs shadow-lg shadow-rose-950/50 animate-pulse"
            >
              <Volume2 className="w-3.5 h-3.5" />
              <span>🔊 Unmute Sound</span>
            </button>
          )}

          <button
            onClick={() => setCurrentView('mixer')}
            className="flex items-center space-x-2 px-4 py-2 rounded-xl bg-dhun-card hover:bg-slate-800 border border-dhun-border text-xs font-bold text-slate-200 transition-all"
          >
            <Sliders className="w-3.5 h-3.5 text-slate-400" />
            <span>Open Console</span>
          </button>

          <button
            onClick={emergencyFade}
            className="flex items-center space-x-1.5 px-3 py-2 rounded-xl bg-red-950/80 hover:bg-red-900 border border-red-700/80 text-red-200 text-xs font-bold transition-all shadow-sm"
          >
            <AlertTriangle className="w-3.5 h-3.5 text-red-400" />
            <span>Emergency Fade</span>
          </button>
        </div>
      </div>

      {/* Live Audio Status Banner */}
      {!audioActive ? (
        <div className="bg-gradient-to-r from-rose-950/80 via-dhun-surface to-amber-950/80 border border-dhun-primary/50 rounded-2xl p-4 flex flex-wrap items-center justify-between gap-4 shadow-xl shadow-rose-950/40">
          <div className="flex items-center space-x-3">
            <div className="p-3 bg-dhun-primary/20 rounded-xl text-dhun-primary">
              <Volume2 className="w-6 h-6 text-dhun-accent animate-pulse" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-white">Browser Audio is Muted by Default</h3>
              <p className="text-xs text-slate-300">Brave and Chrome require one click to unlock live sound output in the browser.</p>
            </div>
          </div>
          <div className="flex items-center space-x-3">
            <button
              onClick={handleStartAudio}
              className="flex items-center space-x-2 px-5 py-2.5 rounded-xl bg-gradient-to-r from-dhun-primary to-dhun-accent hover:from-rose-600 hover:to-rose-500 text-white font-black text-xs shadow-lg shadow-rose-950/50 active:scale-95 transition-all"
            >
              <Volume2 className="w-4 h-4" />
              <span>🔊 Click to Unmute &amp; Hear Music</span>
            </button>
            <label className="flex items-center space-x-2 px-4 py-2.5 rounded-xl bg-dhun-card hover:bg-slate-800 border border-dhun-border text-xs font-bold text-slate-200 cursor-pointer transition-all">
              <Music className="w-4 h-4 text-amber-400" />
              <span>📁 Play My Own Song (.mp3)</span>
              <input
                ref={fileInputRef}
                type="file"
                accept="audio/*"
                onChange={handleFileUpload}
                className="hidden"
              />
            </label>
          </div>
        </div>
      ) : (
        <div className="bg-emerald-950/40 border border-emerald-600/40 rounded-xl px-4 py-2.5 flex items-center justify-between text-xs text-emerald-300">
          <div className="flex items-center space-x-2">
            <Volume2 className="w-4 h-4 text-emerald-400 animate-pulse" />
            <span><strong className="text-white">Live Web Audio Playing:</strong> Punjabi / Bollywood celebration groove at {partyStatus.now_playing?.analysis.bpm || 128} BPM ({partyStatus.now_playing?.analysis.key_name || 'D Major'}).</span>
          </div>
          <label className="flex items-center space-x-1 px-3 py-1 bg-emerald-900/60 hover:bg-emerald-800 rounded-lg text-white font-bold cursor-pointer transition-all text-[11px]">
            <Music className="w-3 h-3 text-amber-300" />
            <span>Load MP3</span>
            <input
              type="file"
              accept="audio/*"
              onChange={handleFileUpload}
              className="hidden"
            />
          </label>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left 2 Cols: Now Playing & Upcoming Queue */}
        <div className="lg:col-span-2 space-y-6">
          {/* Now Playing Hero */}
          <div className="relative rounded-3xl bg-gradient-to-br from-dhun-card to-dhun-surface border border-dhun-border p-6 overflow-hidden shadow-xl">
            <div className="flex items-start justify-between">
              <div className="flex items-center space-x-4">
                {/* Vinyl graphic */}
                <div className="relative w-24 h-24 rounded-full bg-slate-950 border-4 border-slate-800 flex items-center justify-center shadow-2xl">
                  <div className="w-10 h-10 rounded-full bg-dhun-primary flex items-center justify-center">
                    <Disc className="w-6 h-6 text-white animate-[spin_4s_linear_infinite]" />
                  </div>
                </div>

                <div>
                  <div className="text-[11px] font-bold uppercase tracking-wider text-dhun-accent mb-1 flex items-center space-x-1">
                    <span className="w-2 h-2 rounded-full bg-dhun-accent animate-pulse" />
                    <span>Now Playing (Deck A)</span>
                  </div>
                  <h2 className="text-2xl font-black text-white leading-tight">
                    {nowPlaying?.track.title || 'Gallan Goodiyaan'}
                  </h2>
                  <p className="text-sm text-slate-300 font-medium">
                    {nowPlaying?.track.artist || 'Diljit Dosanjh, Shankar Mahadevan'}
                  </p>
                  <div className="flex items-center space-x-2 mt-2">
                    <span className="text-[10px] font-mono font-bold px-2 py-0.5 rounded bg-slate-800 text-cyan-400 border border-cyan-800/40">
                      {nowPlaying?.analysis.bpm || 128} BPM
                    </span>
                    <span className="text-[10px] font-mono font-bold px-2 py-0.5 rounded bg-slate-800 text-amber-400 border border-amber-800/40">
                      Camelot {nowPlaying?.analysis.camelot_key || '10B'}
                    </span>
                    <span className="text-[10px] font-mono font-bold px-2 py-0.5 rounded bg-slate-800 text-emerald-400 border border-emerald-800/40">
                      Energy {Math.round((nowPlaying?.analysis.overall_energy || 0.92) * 100)}%
                    </span>
                  </div>
                </div>
              </div>
            </div>

            {/* Playhead Progress Bar */}
            <div className="mt-6 space-y-1.5">
              <div className="w-full h-2.5 bg-slate-900 rounded-full overflow-hidden border border-slate-800/80">
                <div
                  className="h-full bg-gradient-to-r from-dhun-primary to-dhun-accent rounded-full transition-all duration-300"
                  style={{ width: `${progressPct}%` }}
                />
              </div>
              <div className="flex justify-between text-[11px] font-mono text-slate-400">
                <span>{formatTime(Math.round(((deck0?.position_ms || 30000) / 1000)))}</span>
                <span>{formatTime(Math.round(((deck0?.duration_ms || 296000) / 1000)))}</span>
              </div>
            </div>
          </div>

          {/* Upcoming Playback Queue */}
          <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-4">
            <h3 className="text-sm font-bold uppercase tracking-wider text-slate-400">
              Deterministic Smart Queue
            </h3>

            <div className="space-y-3">
              {/* Next Track */}
              <div className="p-4 rounded-xl bg-dhun-surface border border-dhun-border flex items-center justify-between">
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 rounded-lg bg-orange-950/40 border border-orange-800/40 flex items-center justify-center text-orange-400 font-bold text-xs font-mono">
                    B
                  </div>
                  <div>
                    <div className="flex items-center space-x-2">
                      <span className="text-[10px] font-bold text-orange-400 uppercase tracking-wider">Up Next</span>
                      <span className="text-[10px] text-slate-400 font-mono">
                        (Preloaded &bull; {nextTrack?.transition_style || 'Bass Swap at 32 bars'})
                      </span>
                    </div>
                    <div className="font-bold text-sm text-white">
                      {nextTrack?.track.title || 'London Thumakda'}
                    </div>
                    <div className="text-xs text-slate-400">
                      {nextTrack?.track.artist || 'Labh Janjua, Sonu Kakkar'}
                    </div>
                  </div>
                </div>

                <div className="flex items-center space-x-2">
                  <span className="text-xs font-mono text-cyan-400 bg-slate-900 px-2 py-1 rounded">
                    {nextTrack?.analysis.bpm || 128} BPM
                  </span>
                  <button
                    onClick={() => forceNextRequest(nextTrack?.track.id || '')}
                    title="Force this track to mix in right now"
                    className="flex items-center space-x-1 px-3 py-1.5 rounded-lg bg-dhun-primary hover:bg-rose-600 text-white text-xs font-bold active:scale-95 transition-all shadow-sm"
                  >
                    <FastForward className="w-3.5 h-3.5" />
                    <span>Mix In Now</span>
                  </button>
                </div>
              </div>

              {/* Following Track */}
              <div className="p-4 rounded-xl bg-dhun-surface/60 border border-dhun-border/70 flex items-center justify-between opacity-80">
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 rounded-lg bg-slate-800 flex items-center justify-center text-slate-400 font-bold text-xs font-mono">
                    3
                  </div>
                  <div>
                    <span className="text-[10px] font-bold text-slate-500 uppercase tracking-wider">Following</span>
                    <div className="font-semibold text-sm text-slate-200">
                      {followingTrack?.track.title || 'Sauda Khara Khara'}
                    </div>
                    <div className="text-xs text-slate-400">
                      {followingTrack?.track.artist || 'Diljit Dosanjh, Sukhbir'}
                    </div>
                  </div>
                </div>
                <span className="text-xs font-mono text-slate-400 bg-slate-900 px-2 py-1 rounded">
                  {followingTrack?.analysis.bpm || 130} BPM
                </span>
              </div>
            </div>
          </div>
        </div>

        {/* Right Col: Guest QR Request Code & Moderation */}
        <div className="space-y-6">
          {/* Guest QR Connection Card */}
          <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 text-center space-y-4">
            <div className="flex items-center justify-center space-x-2 text-rose-400 font-bold text-xs uppercase tracking-wider">
              <QrCode className="w-4 h-4" />
              <span>Scan to Request Songs</span>
            </div>

            {/* Render QR code */}
            <div className="w-48 h-48 mx-auto bg-slate-900 p-2 rounded-2xl border border-dhun-border flex items-center justify-center shadow-inner">
              {partyStatus.qr_svg ? (
                <div
                  className="w-full h-full [&>svg]:w-full [&>svg]:h-full"
                  dangerouslySetInnerHTML={{ __html: partyStatus.qr_svg }}
                />
              ) : (
                <div className="text-xs text-slate-500">QR Code Initializing...</div>
              )}
            </div>

            <div className="space-y-1">
              <div className="text-xs font-mono font-bold text-emerald-400 break-all">
                {partyStatus.guest_url || 'http://192.168.1.100:3000/'}
              </div>
              <p className="text-[11px] text-slate-400">
                Connect your phone to the same Wi-Fi &bull; No app install needed
              </p>
            </div>
          </div>

          {/* Incoming Guest Requests Moderation */}
          <div className="bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-3">
            <div className="flex items-center justify-between">
              <h3 className="text-xs font-bold uppercase tracking-wider text-slate-300">
                Guest Requests ({guestRequests.filter(r => r.status === 'pending').length} Pending)
              </h3>
              <button
                onClick={() => setCurrentView('requests')}
                className="text-xs text-dhun-accent hover:underline font-semibold"
              >
                View All
              </button>
            </div>

            <div className="space-y-2 max-h-72 overflow-y-auto pr-1">
              {guestRequests.length === 0 ? (
                <div className="text-center text-xs text-slate-500 py-6">
                  No song requests yet. Guests can scan the QR code above.
                </div>
              ) : (
                guestRequests.map((req) => (
                  <div
                    key={req.id}
                    className="p-3 rounded-xl bg-dhun-surface border border-dhun-border flex items-center justify-between"
                  >
                    <div className="flex-1 mr-2 min-w-0">
                      <div className="font-semibold text-xs text-white truncate">
                        {req.track_title || 'Song Request'}
                      </div>
                      <div className="text-[11px] text-slate-400 truncate">
                        By <span className="text-slate-200 font-medium">{req.guest_name || 'Guest'}</span>
                      </div>
                    </div>

                    <div className="flex items-center space-x-1.5">
                      {req.status === 'pending' ? (
                        <>
                          <button
                            onClick={() => approveRequest(req.id)}
                            title="Approve request"
                            className="w-7 h-7 rounded-lg bg-emerald-950 hover:bg-emerald-900 border border-emerald-700 text-emerald-400 flex items-center justify-center transition-all"
                          >
                            <Check className="w-3.5 h-3.5" />
                          </button>
                          <button
                            onClick={() => rejectRequest(req.id)}
                            title="Reject request"
                            className="w-7 h-7 rounded-lg bg-red-950 hover:bg-red-900 border border-red-700 text-red-400 flex items-center justify-center transition-all"
                          >
                            <X className="w-3.5 h-3.5" />
                          </button>
                        </>
                      ) : (
                        <button
                          onClick={() => forceNextRequest(req.track_id)}
                          title="Force Next override"
                          className="px-2 py-1 rounded bg-rose-950 text-rose-400 border border-rose-800 text-[10px] font-bold uppercase hover:bg-rose-900"
                        >
                          Force Next
                        </button>
                      )}
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

