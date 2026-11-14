import React, { useState } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import { MOCK_TRACKS } from '../../api/ipc';
import {
  Music,
  Search,
  FolderPlus,
  CheckCircle2,
  Sliders,
  Sparkles,
  Info,
  Clock,
} from 'lucide-react';

export const LibraryScreen: React.FC = () => {
  const { tracks } = useDhunStore();
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedTrackId, setSelectedTrackId] = useState<string>('trk_01');

  const displayTracks = MOCK_TRACKS.filter(
    (t) =>
      t.track.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
      t.track.artist.toLowerCase().includes(searchTerm.toLowerCase()) ||
      t.track.genre.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const selectedCandidate = MOCK_TRACKS.find((t) => t.track.id === selectedTrackId) || MOCK_TRACKS[0];

  return (
    <div className="flex-1 overflow-y-auto p-6 max-w-7xl mx-auto flex flex-col lg:flex-row gap-6">
      {/* Track List Table */}
      <div className="flex-1 bg-dhun-surface border border-dhun-border rounded-2xl p-5 flex flex-col space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-bold text-white flex items-center space-x-2">
              <Music className="w-5 h-5 text-dhun-primary" />
              <span>Music Library &amp; Metadata</span>
            </h1>
            <p className="text-xs text-slate-400">Offline-first &bull; Independent permanent track metadata</p>
          </div>

          <button
            onClick={() => alert('Folder scan dialog: Choose a folder on your laptop containing MP3/WAV/FLAC files')}
            className="flex items-center space-x-2 px-3 py-2 rounded-xl bg-dhun-card hover:bg-slate-800 border border-dhun-border text-xs font-semibold text-slate-200 transition-all shadow-sm"
          >
            <FolderPlus className="w-4 h-4 text-dhun-accent" />
            <span>Scan Music Folder</span>
          </button>
        </div>

        {/* Search input */}
        <div className="relative">
          <input
            type="text"
            placeholder="Search by title, artist, genre..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full bg-dhun-card border border-dhun-border rounded-xl px-4 py-2.5 text-xs text-white pl-9 focus:outline-none focus:border-dhun-primary placeholder-slate-500"
          />
          <Search className="w-4 h-4 text-slate-500 absolute left-3 top-3" />
        </div>

        {/* Table */}
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-dhun-border text-slate-400 text-[11px] uppercase tracking-wider">
                <th className="py-2.5 px-3">Title &amp; Artist</th>
                <th className="py-2.5 px-3">Genre</th>
                <th className="py-2.5 px-3">BPM</th>
                <th className="py-2.5 px-3">Camelot Key</th>
                <th className="py-2.5 px-3">Energy</th>
                <th className="py-2.5 px-3">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-dhun-border/40">
              {displayTracks.map((item) => {
                const isSelected = item.track.id === selectedTrackId;
                return (
                  <tr
                    key={item.track.id}
                    onClick={() => setSelectedTrackId(item.track.id)}
                    className={`cursor-pointer transition-colors ${
                      isSelected
                        ? 'bg-rose-950/30 text-white font-semibold'
                        : 'hover:bg-dhun-card/60 text-slate-300'
                    }`}
                  >
                    <td className="py-3 px-3">
                      <div className="text-white font-medium truncate max-w-xs">{item.track.title}</div>
                      <div className="text-slate-400 text-[11px] truncate">{item.track.artist}</div>
                    </td>
                    <td className="py-3 px-3">
                      <span className="px-2 py-0.5 rounded bg-slate-800 text-[10px] text-slate-300">
                        {item.track.genre}
                      </span>
                    </td>
                    <td className="py-3 px-3 font-mono font-bold text-cyan-400">{item.analysis.bpm}</td>
                    <td className="py-3 px-3 font-mono font-bold text-amber-400">{item.analysis.camelot_key}</td>
                    <td className="py-3 px-3 font-mono text-emerald-400">
                      {Math.round(item.analysis.overall_energy * 100)}%
                    </td>
                    <td className="py-3 px-3">
                      <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] font-bold uppercase bg-emerald-950 text-emerald-400 border border-emerald-800">
                        <CheckCircle2 className="w-3 h-3" />
                        <span>Playable</span>
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {/* Selected Track Detail Inspector */}
      {selectedCandidate && (
        <div className="w-full lg:w-96 bg-dhun-card border border-dhun-border rounded-2xl p-5 space-y-4">
          <div className="flex items-center justify-between border-b border-dhun-border pb-3">
            <h2 className="text-sm font-bold text-white flex items-center space-x-2">
              <Info className="w-4 h-4 text-dhun-accent" />
              <span>Track Analysis Inspector</span>
            </h2>
            <span className="text-[10px] uppercase font-bold text-emerald-400 bg-emerald-950 px-2 py-0.5 rounded border border-emerald-800">
              Verified
            </span>
          </div>

          <div>
            <h3 className="text-base font-black text-white">{selectedCandidate.track.title}</h3>
            <p className="text-xs text-slate-300">{selectedCandidate.track.artist}</p>
            <p className="text-[11px] text-slate-500 font-mono mt-1">{selectedCandidate.asset.file_path}</p>
          </div>

          {/* Musical Characteristics Grid */}
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="bg-dhun-surface p-3 rounded-xl border border-dhun-border">
              <span className="text-[10px] text-slate-500 uppercase block font-semibold">BPM &amp; Confidence</span>
              <span className="text-base font-bold font-mono text-cyan-400">{selectedCandidate.analysis.bpm}</span>
              <span className="text-[10px] text-slate-400 ml-1.5 font-mono">
                ({Math.round(selectedCandidate.analysis.bpm_confidence * 100)}%)
              </span>
            </div>

            <div className="bg-dhun-surface p-3 rounded-xl border border-dhun-border">
              <span className="text-[10px] text-slate-500 uppercase block font-semibold">Camelot Key</span>
              <span className="text-base font-bold font-mono text-amber-400">
                {selectedCandidate.analysis.camelot_key}
              </span>
              <span className="text-[10px] text-slate-400 ml-1.5 font-mono">
                ({selectedCandidate.analysis.key_name})
              </span>
            </div>

            <div className="bg-dhun-surface p-3 rounded-xl border border-dhun-border">
              <span className="text-[10px] text-slate-500 uppercase block font-semibold">Energy &amp; Danceability</span>
              <span className="text-base font-bold font-mono text-emerald-400">
                {Math.round(selectedCandidate.analysis.overall_energy * 100)}%
              </span>
            </div>

            <div className="bg-dhun-surface p-3 rounded-xl border border-dhun-border">
              <span className="text-[10px] text-slate-500 uppercase block font-semibold">Downbeat Offset</span>
              <span className="text-base font-bold font-mono text-slate-300">
                {selectedCandidate.analysis.downbeat_offset_ms}ms
              </span>
            </div>
          </div>

          {/* Phrasing & Transition recommendations */}
          <div className="bg-dhun-surface p-4 rounded-xl border border-dhun-border space-y-2 text-xs">
            <span className="text-[10px] text-slate-500 uppercase block font-bold tracking-wider">
              Transition Planning
            </span>
            <div className="flex justify-between text-slate-300">
              <span>Recommended Mix-In:</span>
              <span className="font-mono text-white">
                {Math.round(selectedCandidate.analysis.recommended_mix_in_ms / 1000)}s (Intro exit)
              </span>
            </div>
            <div className="flex justify-between text-slate-300">
              <span>Recommended Mix-Out:</span>
              <span className="font-mono text-white">
                {Math.round(selectedCandidate.analysis.recommended_mix_out_ms / 1000)}s (Outro entry)
              </span>
            </div>
            <div className="flex justify-between text-slate-300">
              <span>Default Transition:</span>
              <span className="font-mono text-dhun-accent font-bold uppercase">
                {selectedCandidate.transition_style}
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

