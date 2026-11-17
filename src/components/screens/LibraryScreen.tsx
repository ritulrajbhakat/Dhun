import React, { useState } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import {
  Music,
  Search,
  Link2,
  CheckCircle2,
  ExternalLink,
  AlertCircle,
  Sparkles,
} from 'lucide-react';

export const LibraryScreen: React.FC = () => {
  const {
    cloudParty,
    registeredSources,
    registerYouTubeSource,
    backendConnected,
  } = useDhunStore();

  const [searchTerm, setSearchTerm] = useState('');
  const [urlInput, setUrlInput] = useState('');
  const [isRegistering, setIsRegistering] = useState(false);
  const [statusMessage, setStatusMessage] = useState<{ type: 'ok' | 'err'; text: string } | null>(null);

  const filteredSources = registeredSources.filter(
    (s) =>
      s.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
      s.artist.toLowerCase().includes(searchTerm.toLowerCase()) ||
      s.provider_video_id.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!urlInput.trim()) return;
    setIsRegistering(true);
    setStatusMessage(null);
    try {
      const added = await registerYouTubeSource(urlInput.trim());
      setStatusMessage({
        type: 'ok',
        text: `Registered "${added.title}" via official YouTube reference.`,
      });
      setUrlInput('');
    } catch (err: any) {
      setStatusMessage({
        type: 'err',
        text: err?.message || 'Failed to register YouTube URL',
      });
    } finally {
      setIsRegistering(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-6xl mx-auto flex flex-col gap-6">
      <div className="bg-dhun-surface border border-dhun-border rounded-2xl p-5 space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-bold text-white flex items-center space-x-2">
              <Music className="w-5 h-5 text-dhun-primary" />
              <span>Online Music Source Registry</span>
            </h1>
            <p className="text-xs text-slate-400">
              Official YouTube &amp; YouTube Music link registration &bull; Prepared for Phase 2 IFrame Player
            </p>
          </div>
          <div className="text-xs font-mono px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-800 text-slate-300">
            {registeredSources.length} Registered Tracks
          </div>
        </div>

        {/* Register YouTube URL Form */}
        <form onSubmit={handleRegister} className="bg-slate-900/90 border border-slate-800 rounded-xl p-4 space-y-2.5">
          <label className="text-xs font-bold text-amber-300 uppercase flex items-center space-x-1.5">
            <Link2 className="w-4 h-4" />
            <span>Add Official YouTube / YouTube Music Link</span>
          </label>
          <p className="text-[11px] text-slate-400">
            Paste a valid youtube.com/watch?v=..., youtu.be/..., or music.youtube.com/watch?v=... link.
          </p>
          <div className="flex gap-2">
            <input
              type="url"
              value={urlInput}
              onChange={(e) => setUrlInput(e.target.value)}
              placeholder="https://www.youtube.com/watch?v=..."
              disabled={!cloudParty || !backendConnected || isRegistering}
              className="flex-1 bg-black/60 border border-slate-700 rounded-lg px-3 py-2 text-xs text-white font-mono outline-none focus:border-amber-400 disabled:opacity-50"
            />
            <button
              type="submit"
              disabled={!cloudParty || !backendConnected || isRegistering || !urlInput.trim()}
              className="px-4 py-2 bg-amber-500 hover:bg-amber-400 disabled:opacity-40 text-black font-extrabold text-xs rounded-lg transition-all"
            >
              {isRegistering ? 'Registering...' : 'Register Link'}
            </button>
          </div>

          {!cloudParty && (
            <p className="text-[11px] text-rose-400 flex items-center gap-1.5 pt-1">
              <AlertCircle className="w-3.5 h-3.5" />
              <span>Create a party first to register tracks to the party catalog.</span>
            </p>
          )}

          {statusMessage && (
            <div
              className={`text-xs px-3 py-2 rounded-lg border flex items-center gap-2 ${
                statusMessage.type === 'ok'
                  ? 'bg-emerald-950/50 border-emerald-500/40 text-emerald-300'
                  : 'bg-rose-950/50 border-rose-500/40 text-rose-300'
              }`}
            >
              {statusMessage.type === 'ok' ? (
                <CheckCircle2 className="w-4 h-4 shrink-0" />
              ) : (
                <AlertCircle className="w-4 h-4 shrink-0" />
              )}
              <span>{statusMessage.text}</span>
            </div>
          )}
        </form>

        {/* Search Registered Catalog */}
        <div className="relative">
          <Search className="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            placeholder="Filter registered party sources..."
            className="w-full bg-black/50 border border-slate-800 rounded-xl pl-9 pr-4 py-2 text-xs text-white outline-none focus:border-teal-400"
          />
        </div>

        {/* Source List */}
        {filteredSources.length === 0 ? (
          <div className="text-center py-12 text-slate-500 text-xs space-y-2">
            <Sparkles className="w-8 h-8 mx-auto opacity-30" />
            <p>No online sources registered in this party yet.</p>
          </div>
        ) : (
          <div className="space-y-2">
            {filteredSources.map((src) => (
              <div
                key={src.id}
                className="flex items-center justify-between gap-4 p-3 rounded-xl bg-slate-900/70 border border-slate-800 hover:border-slate-700 transition-colors"
              >
                <div className="flex items-center gap-3 min-w-0">
                  {src.thumbnail_url ? (
                    <img
                      src={src.thumbnail_url}
                      alt={src.title}
                      className="w-12 h-9 object-cover rounded-lg bg-slate-800 shrink-0"
                    />
                  ) : (
                    <div className="w-12 h-9 rounded-lg bg-slate-800 flex items-center justify-center shrink-0">
                      <Music className="w-4 h-4 text-slate-500" />
                    </div>
                  )}
                  <div className="min-w-0">
                    <div className="text-sm font-bold text-white truncate">{src.title}</div>
                    <div className="text-xs text-slate-400 truncate">
                      {src.artist} &bull; ID: <span className="font-mono">{src.provider_video_id}</span>
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  <span className="px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-red-500/15 text-red-300 border border-red-500/30">
                    {src.provider === 'youtube_music' ? 'YT Music' : 'YouTube'}
                  </span>
                  <span
                    className="px-2 py-0.5 rounded-full text-[10px] font-bold uppercase bg-amber-500/15 text-amber-300 border border-amber-500/30"
                    title={src.error_reason || 'Queued for Phase 2 YouTube IFrame Player'}
                  >
                    {src.playability_status}
                  </span>
                  <a
                    href={src.original_url}
                    target="_blank"
                    rel="noreferrer"
                    className="p-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300"
                    title="Open Official Source"
                  >
                    <ExternalLink className="w-3.5 h-3.5" />
                  </a>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
