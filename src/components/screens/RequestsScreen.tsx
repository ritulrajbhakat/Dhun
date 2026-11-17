import React from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { Users, Sparkles } from 'lucide-react';

/**
 * Private Host Overview of Automated DJ Submissions.
 * Notice: DHUN intentionally has NO host approval/rejection queue.
 * All host and guest submissions enter the private DJ candidate pool automatically.
 */
export const RequestsScreen: React.FC = () => {
  const { submissions, evaluateDjPool } = useDhunStore();

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-5xl mx-auto space-y-6">
      <div className="flex items-center justify-between border-b border-dhun-border pb-4">
        <div>
          <h1 className="text-xl font-bold text-white flex items-center space-x-2">
            <Users className="w-5 h-5 text-dhun-primary" />
            <span>Automated DJ Submission Pool</span>
          </h1>
          <p className="text-xs text-slate-400">
            No manual approval queue &bull; Automated DJ evaluates mood, energy, BPM, and Camelot harmony
          </p>
        </div>
        <button
          onClick={() => evaluateDjPool()}
          className="px-3.5 py-2 rounded-xl bg-amber-500 hover:bg-amber-400 text-black text-xs font-black flex items-center gap-1.5"
        >
          <Sparkles className="w-3.5 h-3.5" />
          <span>Re-evaluate Pool</span>
        </button>
      </div>

      {submissions.length === 0 ? (
        <div className="text-center py-12 text-xs text-slate-400">
          No submissions received in this local party room yet.
        </div>
      ) : (
        <div className="space-y-2.5">
          {submissions.map((sub) => (
            <div
              key={sub.id}
              className="p-4 bg-dhun-surface border border-dhun-border rounded-2xl flex items-center justify-between gap-4"
            >
              <div className="min-w-0">
                <div className="text-sm font-bold text-white truncate">{sub.title}</div>
                <div className="text-xs text-slate-400 truncate">
                  {sub.artist} &bull; submitted by{' '}
                  <span className="text-amber-300">
                    {sub.submitted_by_type === 'host' ? 'Host' : sub.submitted_by_name}
                  </span>
                </div>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <span className="px-2.5 py-1 rounded-lg bg-teal-500/15 border border-teal-500/30 text-xs font-mono text-teal-300">
                  Score: {sub.dj_score.toFixed(1)}
                </span>
                <span className="px-2.5 py-1 rounded-lg bg-slate-800 text-xs font-mono text-slate-300 uppercase">
                  {sub.dj_status}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
