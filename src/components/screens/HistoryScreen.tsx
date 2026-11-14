import React from 'react';
import { History, Calendar, Disc3, Clock, Sparkles } from 'lucide-react';
import { useDhunStore } from '../../store/useDhunStore';

export const HistoryScreen: React.FC = () => {
  const { setCurrentView } = useDhunStore();

  const mockPastSessions = [
    {
      id: 'ses_01',
      name: 'Rohan & Priya Sangeet Celebration',
      date: 'Yesterday, 8:30 PM',
      duration: '4h 15m',
      level: 2,
      mood: 'Sangeet',
      tracksPlayed: 74,
      requestsReceived: 28,
    },
    {
      id: 'ses_02',
      name: 'Delhi House Party Night',
      date: 'Last Saturday, 9:00 PM',
      duration: '5h 40m',
      level: 2,
      mood: 'Desi Club',
      tracksPlayed: 92,
      requestsReceived: 41,
    },
  ];

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-6xl mx-auto space-y-6">
      <div className="flex items-center justify-between border-b border-dhun-border pb-4">
        <div>
          <h1 className="text-xl font-bold text-white flex items-center space-x-2">
            <History className="w-5 h-5 text-dhun-primary" />
            <span>Party Session History &amp; Logs</span>
          </h1>
          <p className="text-xs text-slate-400">Review previous parties, track playlists, and guest requests</p>
        </div>
      </div>

      <div className="space-y-4">
        {mockPastSessions.map((ses) => (
          <div
            key={ses.id}
            className="bg-dhun-card border border-dhun-border rounded-2xl p-5 flex flex-col md:flex-row items-start md:items-center justify-between gap-4"
          >
            <div className="space-y-1.5">
              <div className="flex items-center space-x-2">
                <span className="font-bold text-base text-white">{ses.name}</span>
                <span className="text-[10px] uppercase font-bold px-2 py-0.5 rounded bg-rose-950 text-rose-400 border border-rose-800">
                  Level {ses.level}
                </span>
                <span className="text-[10px] uppercase font-bold px-2 py-0.5 rounded bg-amber-950 text-amber-400 border border-amber-800">
                  {ses.mood}
                </span>
              </div>

              <div className="flex items-center space-x-4 text-xs text-slate-400">
                <span className="flex items-center space-x-1">
                  <Calendar className="w-3.5 h-3.5 text-slate-500" />
                  <span>{ses.date}</span>
                </span>
                <span className="flex items-center space-x-1">
                  <Clock className="w-3.5 h-3.5 text-slate-500" />
                  <span>{ses.duration}</span>
                </span>
                <span className="flex items-center space-x-1">
                  <Disc3 className="w-3.5 h-3.5 text-slate-500" />
                  <span>{ses.tracksPlayed} tracks played</span>
                </span>
              </div>
            </div>

            <button
              onClick={() => setCurrentView('wizard')}
              className="flex items-center space-x-1.5 px-4 py-2 rounded-xl bg-dhun-surface hover:bg-slate-800 border border-dhun-border text-xs font-semibold text-slate-200 transition-all self-end md:self-center"
            >
              <Sparkles className="w-3.5 h-3.5 text-dhun-accent" />
              <span>Clone Setup &amp; Launch</span>
            </button>
          </div>
        ))}
      </div>
    </div>
  );
};

