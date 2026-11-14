import React, { useState } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import {
  QrCode,
  Check,
  X,
  FastForward,
  Star,
  Clock,
  User,
  Filter,
} from 'lucide-react';

export const RequestsScreen: React.FC = () => {
  const { guestRequests, approveRequest, rejectRequest, forceNextRequest } = useDhunStore();
  const [filterStatus, setFilterStatus] = useState<string>('all');

  const filtered = guestRequests.filter((r) => {
    if (filterStatus === 'all') return true;
    return r.status === filterStatus;
  });

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-6xl mx-auto space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-4 border-b border-dhun-border pb-4">
        <div>
          <h1 className="text-xl font-bold text-white flex items-center space-x-2">
            <QrCode className="w-5 h-5 text-dhun-primary" />
            <span>Guest Request Queue &amp; Moderation</span>
          </h1>
          <p className="text-xs text-slate-400">
            Requests submitted by guests through local phone QR code
          </p>
        </div>

        {/* Status Filters */}
        <div className="flex items-center space-x-1.5 bg-dhun-card p-1 rounded-xl border border-dhun-border">
          {['all', 'pending', 'approved', 'playing'].map((st) => (
            <button
              key={st}
              onClick={() => setFilterStatus(st)}
              className={`px-3 py-1.5 rounded-lg text-xs font-semibold uppercase tracking-wider transition-all ${
                filterStatus === st
                  ? 'bg-dhun-primary text-white'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              {st}
            </button>
          ))}
        </div>
      </div>

      {/* Requests List */}
      <div className="space-y-3">
        {filtered.length === 0 ? (
          <div className="text-center py-16 text-slate-500 text-sm bg-dhun-card/40 rounded-2xl border border-dashed border-dhun-border">
            No song requests match the selected filter.
          </div>
        ) : (
          filtered.map((req) => (
            <div
              key={req.id}
              className="bg-dhun-card border border-dhun-border rounded-xl p-4 flex flex-col md:flex-row items-start md:items-center justify-between gap-4"
            >
              <div className="space-y-1">
                <div className="flex items-center space-x-2">
                  <span className="font-bold text-sm text-white">{req.track_title || 'Song Request'}</span>
                  <span
                    className={`text-[10px] font-bold uppercase px-2 py-0.5 rounded border ${
                      req.status === 'playing'
                        ? 'bg-emerald-950 text-emerald-400 border-emerald-800'
                        : req.status === 'approved'
                        ? 'bg-cyan-950 text-cyan-400 border-cyan-800'
                        : req.status === 'pending'
                        ? 'bg-amber-950 text-amber-400 border-amber-800'
                        : 'bg-slate-800 text-slate-400 border-slate-700'
                    }`}
                  >
                    {req.status}
                  </span>
                  {req.priority_order < 50 && (
                    <span className="text-[10px] font-bold uppercase px-1.5 py-0.5 rounded bg-rose-950 text-rose-400 border border-rose-800">
                      High Priority
                    </span>
                  )}
                </div>

                <div className="flex items-center space-x-3 text-xs text-slate-400">
                  <span className="flex items-center space-x-1">
                    <User className="w-3 h-3 text-slate-500" />
                    <span>Requested by <strong className="text-slate-300">{req.guest_name || 'Guest'}</strong></span>
                  </span>
                  {req.host_note && (
                    <span className="italic text-amber-300/80">"{req.host_note}"</span>
                  )}
                </div>
              </div>

              {/* Host Control Actions */}
              <div className="flex items-center space-x-2 self-end md:self-center">
                {req.status === 'pending' && (
                  <>
                    <button
                      onClick={() => approveRequest(req.id)}
                      className="flex items-center space-x-1 px-3 py-1.5 rounded-lg bg-emerald-950 hover:bg-emerald-900 border border-emerald-700 text-emerald-300 text-xs font-bold transition-all"
                    >
                      <Check className="w-3.5 h-3.5" />
                      <span>Approve</span>
                    </button>
                    <button
                      onClick={() => rejectRequest(req.id)}
                      className="flex items-center space-x-1 px-3 py-1.5 rounded-lg bg-red-950 hover:bg-red-900 border border-red-700 text-red-300 text-xs font-bold transition-all"
                    >
                      <X className="w-3.5 h-3.5" />
                      <span>Reject</span>
                    </button>
                  </>
                )}

                <button
                  onClick={() => forceNextRequest(req.track_id)}
                  className="flex items-center space-x-1 px-3 py-1.5 rounded-lg bg-dhun-primary hover:bg-rose-600 text-white text-xs font-bold transition-all shadow-sm"
                >
                  <FastForward className="w-3.5 h-3.5" />
                  <span>Force Next</span>
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

