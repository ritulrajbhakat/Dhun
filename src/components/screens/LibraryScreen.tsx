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
  FolderOpen,
} from 'lucide-react';

export const LibraryScreen: React.FC = () => {
  const { tracks, scanFolder } = useDhunStore();
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedTrackId, setSelectedTrackId] = useState<string>('trk_01');
  const [showScanInput, setShowScanInput] = useState(false);
  const [scanPath, setScanPath] = useState('C:/Music');
  const [isScanning, setIsScanning] = useState(false);
  const [scanMessage, setScanMessage] = useState<string | null>(null);

  // Combine live database tracks with preloaded candidates
  const candidatePool = MOCK_TRACKS;
  const currentTracks = tracks.length > 0 ? tracks : candidatePool.map((c) => c.track);

  const displayTracks = currentTracks.filter(
    (t) =>
      t.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
      t.artist.toLowerCase().includes(searchTerm.toLowerCase()) ||
      t.genre.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const selectedCandidate =
    candidatePool.find((t) => t.track.id === selectedTrackId) || candidatePool[0];

  const handleScan = async () => {
    if (!scanPath.trim()) return;
    setIsScanning(true);
    setScanMessage(null);
    try {
      const count = await scanFolder(scanPath.trim());
      setScanMessage(`Successfully indexed ${count} tracks into library catalogue.`);
      setShowScanInput(false);
    } catch (err) {
      setScanMessage(`Scan error: ${err}`);
    } finally {
      setIsScanning(false);
    }
  };

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
            onClick={() => setShowScanInput((prev) => !prev)}
            className="flex items-center space-x-2 px-3 py-2 rounded-xl bg-dhun-card hover:bg-slate-800 border border-dhun-border text-xs font-semibold text-slate-200 transition-all shadow-sm"
          >
            <FolderPlus className="w-4 h-4 text-dhun-accent" />
            <span>Scan Music Folder</span>
          </button>
        </div>

        {/* Folder Scan Dialog */}
        {showScanInput && (
          <div className="bg-slate-900 border border-amber-500/30 rounded-xl p-4 space-y-2">
            <h3 className="text-xs font-bold text-amber-300 uppercase">Scan Local Music Directory</h3>
            <p className="text-[11px] text-slate-400">
              Recursively indexes MP3, WAV, FLAC, and M4A files, analyzes BPM, musical keys, downbeats, and energy phrasing.
            </p>
            <div className="flex space-x-2 pt-1">
              <input
                type="text"
                value={scanPath}
                onChange={(e) => setScanPath(e.target.value)}
                placeholder="Folder path (e.g. C:/Users/YourName/Music)..."
                className="flex-1 bg-black/60 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-white font-mono"
              />
              <button
                onClick={handleScan}
                disabled={isScanning}
                className="px-4 py-1.5 bg-amber-500 hover:bg-amber-400 text-black font-extrabold text-xs rounded-lg flex items-center space-x-1"
              >
                <FolderOpen className="w-3.5 h-3.5" />
                <span>{isScanning ? 'Analyzing...' : 'Scan Now'}</span>
              </button>
            </div>
          </div>
        )}

        {scanMessage && (
          <div className="p-3 bg-emerald-950/40 border border-emerald-800/60 rounded-xl text-xs text-emerald-300">
            {scanMessage}
          </div>
        )}

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
                <th className="py-2.5 px-3">Duration</th>
                <th className="py-2.5 px-3">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-dhun-border/40">
              {displayTracks.map((item) => {
                const isSelected = item.id === selectedTrackId;
                const durMin = Math.floor(item.duration_ms / 60000);
                const durSec = Math.floor((item.duration_ms % 60000) / 1000);
                return (
                  <tr
                    key={item.id}
                    onClick={() => setSelectedTrackId(item.id)}
                    className={`cursor-pointer transition-colors ${
                      isSelected
                        ? 'bg-rose-950/30 text-white font-semibold'
                        : 'hover:bg-dhun-card/60 text-slate-300'
                    }`}
                  >
                    <td className="py-3 px-3">
                      <div className="text-white font-medium truncate max-w-xs">{item.title}</div>
                      <div className="text-slate-400 text-[11px] truncate">{item.artist}</div>
                    </td>
                    <td className="py-3 px-3">
                      <span className="px-2 py-0.5 rounded bg-slate-800 text-[10px] text-slate-300">
                        {item.genre}
                      </span>
                    </td>
                    <td className="py-3 px-3 font-mono font-bold text-cyan-400">
                      {candidatePool.find((c) => c.track.id === item.id)?.analysis.bpm || 128}
                    </td>
                    <td className="py-3 px-3 font-mono text-slate-400">
                      {durMin}:{durSec.toString().padStart(2, '0')}
                    </td>
                    <td className="py-3 px-3">
                      <span className="inline-flex items-center space-x-1 text-emerald-400 text-[10px] bg-emerald-950/60 px-2 py-0.5 rounded border border-emerald-900">
                        <CheckCircle2 className="w-3 h-3" />
                        <span>Verified</span>
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {/* Track Analysis & Phrasing Inspector Card */}
      <div className="w-full lg:w-80 bg-dhun-surface border border-dhun-border rounded-2xl p-5 flex flex-col space-y-4">
        <h2 className="text-sm font-bold text-white flex items-center space-x-2">
          <Info className="w-4 h-4 text-cyan-400" />
          <span>Track Metadata &amp; Phrasing</span>
        </h2>

        {selectedCandidate ? (
          <div className="space-y-4 text-xs">
            <div className="p-3 bg-dhun-card rounded-xl border border-dhun-border/60">
              <div className="font-bold text-white text-sm">{selectedCandidate.track.title}</div>
              <div className="text-slate-400">{selectedCandidate.track.artist}</div>
              <div className="text-[10px] text-slate-500 font-mono mt-1">{selectedCandidate.asset.file_path}</div>
            </div>

            <div className="grid grid-cols-2 gap-2 text-center">
              <div className="p-2.5 bg-slate-900 rounded-xl border border-slate-800">
                <span className="text-[10px] text-slate-400 uppercase block">Tempo</span>
                <span className="font-mono font-bold text-cyan-400 text-sm">
                  {selectedCandidate.analysis.bpm} BPM
                </span>
              </div>
              <div className="p-2.5 bg-slate-900 rounded-xl border border-slate-800">
                <span className="text-[10px] text-slate-400 uppercase block">Camelot Key</span>
                <span className="font-mono font-bold text-amber-400 text-sm">
                  {selectedCandidate.analysis.camelot_key} ({selectedCandidate.analysis.key_name})
                </span>
              </div>
              <div className="p-2.5 bg-slate-900 rounded-xl border border-slate-800">
                <span className="text-[10px] text-slate-400 uppercase block">Overall Energy</span>
                <span className="font-mono font-bold text-emerald-400 text-sm">
                  {(selectedCandidate.analysis.overall_energy * 100).toFixed(0)}%
                </span>
              </div>
              <div className="p-2.5 bg-slate-900 rounded-xl border border-slate-800">
                <span className="text-[10px] text-slate-400 uppercase block">Dance Score</span>
                <span className="font-mono font-bold text-rose-400 text-sm">
                  {(selectedCandidate.analysis.dance_suitability * 100).toFixed(0)}%
                </span>
              </div>
            </div>

            <div className="p-3 bg-slate-900/80 rounded-xl border border-slate-800 space-y-1.5">
              <div className="text-[11px] font-bold text-slate-300 flex items-center justify-between">
                <span>Phrase Boundaries</span>
                <Sparkles className="w-3 h-3 text-amber-400" />
              </div>
              <div className="flex justify-between text-[11px] text-slate-400 font-mono">
                <span>Mix In Cue:</span>
                <span className="text-white">
                  {(selectedCandidate.analysis.recommended_mix_in_ms / 1000).toFixed(1)}s
                </span>
              </div>
              <div className="flex justify-between text-[11px] text-slate-400 font-mono">
                <span>Mix Out Boundary:</span>
                <span className="text-white">
                  {(selectedCandidate.analysis.recommended_mix_out_ms / 1000).toFixed(1)}s
                </span>
              </div>
            </div>
          </div>
        ) : (
          <p className="text-xs text-slate-500">Select a track to inspect phrasing metadata.</p>
        )}
      </div>
    </div>
  );
};
