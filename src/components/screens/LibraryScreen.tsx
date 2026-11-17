import React, { useState } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { Music, Search, Link2, CheckCircle2, ExternalLink, AlertCircle, Sparkles } from 'lucide-react';

export const LibraryScreen: React.FC = () => {
  const { activeRoom, submissions, submitSong, helperConnected } = useDhunStore();
  const [searchTerm, setSearchTerm] = useState('');
  const [urlInput, setUrlInput] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [statusMessage, setStatusMessage] = useState<{ type: 'ok' | 'err'; text: string } | null>(null);

  const filtered = submissions.filter(
    (s) =>
      s.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
      s.artist.toLowerCase().includes(searchTerm.toLowerCase()) ||
      s.provider_video_id.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!urlInput.trim()) return;
    setIsSubmitting(true);
    setStatusMessage(null);
    try {
      const added = await submitSong(urlInput.trim());
      setStatusMessage({
        type: 'ok',
        text: `Submitted "${added.title}" directly to the automated DJ candidate pool (score ${added.dj_score.toFixed(1)}).`,
      });
      setUrlInput('');
    } catch (err: any) {
      setStatusMessage({ type: 'err', text: err?.message || 'Failed to submit YouTube URL' });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-6xl mx-auto flex flex-col gap-6">
      <div className="bg-dhun-surface border border-dhun-border rounded-2xl p-5 space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-bold text-white flex items-center space-x-2">
              <Music className="w-5 h-5 text-dhun-primary" />
              <span>Private Party DJ Candidate Pool</span>
            </h1>
            <p className="text-xs text-slate-400">
              YouTube &amp; YouTube Music URLs only &bull; Unified host &amp; guest submission flow (no approval queue)
            </p>
          </div>
          <div className="text-xs font-mono px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-800 text-slate-300">
            {submissions.length} Candidates
          </div>
        </div>

        <form onSubmit={handleSubmit} className="bg-slate-900/90 border border-slate-800 rounded-xl p-4 space-y-2.5">
          <label className="text-xs font-bold text-amber-300 uppercase flex items-center space-x-1.5">
            <Link2 className="w-4 h-4" />
            <span>Submit Official YouTube / YouTube Music Link</span>
          </label>
          <div className="flex gap-2">
            <input
              type="url"
              value={urlInput}
              onChange={(e) => setUrlInput(e.target.value)}
              placeholder="https://www.youtube.com/watch?v=..."
              disabled={!activeRoom || activeRoom.status === 'closed' || !helperConnected || isSubmitting}
              className="flex-1 bg-black/60 border border-slate-700 rounded-lg px-3 py-2 text-xs text-white font-mono outline-none focus:border-amber-400 disabled:opacity-50"
            />
            <button
              type="submit"
              disabled={!activeRoom || activeRoom.status === 'closed' || !helperConnected || isSubmitting || !urlInput.trim()}
              className="px-4 py-2 bg-amber-500 hover:bg-amber-400 disabled:opacity-40 text-black font-extrabold text-xs rounded-lg"
            >
              {isSubmitting ? 'Submitting...' : 'Submit to DJ'}
            </button>
          </div>

          {statusMessage && (
            <div
              className={`text-xs px-3 py-2 rounded-lg border flex items-center gap-2 ${
                statusMessage.type === 'ok'
                  ? 'bg-emerald-950/50 border-emerald-500/40 text-emerald-300'
                  : 'bg-rose-950/50 border-rose-500/40 text-rose-300'
              }`}
            >
              {statusMessage.type === 'ok' ? <CheckCircle2 className="w-4 h-4 shrink-0" /> : <AlertCircle className="w-4 h-4 shrink-0" />}
              <span>{statusMessage.text}</span>
            </div>
          )}
        </form>

        <div className="relative">
          <Search className="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            placeholder="Filter party submissions..."
            className="w-full bg-black/50 border border-slate-800 rounded-xl pl-9 pr-4 py-2 text-xs text-white outline-none focus:border-teal-400"
          />
        </div>

        {filtered.length === 0 ? (
          <div className="text-center py-12 text-slate-500 text-xs space-y-2">
            <Sparkles className="w-8 h-8 mx-auto opacity-30" />
            <p>No songs in this local party pool yet.</p>
          </div>
        ) : (
          <div className="space-y-2">
            {filtered.map((sub) => (
              <div
                key={sub.id}
                className="flex items-center justify-between gap-4 p-3 rounded-xl bg-slate-900/70 border border-slate-800"
              >
                <div className="min-w-0">
                  <div className="text-sm font-bold text-white truncate">{sub.title}</div>
                  <div className="text-xs text-slate-400 truncate">
                    {sub.artist} &bull; Score {sub.dj_score.toFixed(1)} &bull; {sub.dj_status}
                  </div>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <span className="px-2 py-0.5 rounded-full text-[10px] font-mono bg-amber-500/15 text-amber-300 border border-amber-500/30">
                    {sub.acquisition_status}
                  </span>
                  <a href={sub.source_url} target="_blank" rel="noreferrer" className="p-1.5 rounded-lg bg-slate-800 text-slate-300">
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
