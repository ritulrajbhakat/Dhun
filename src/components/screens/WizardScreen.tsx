import React, { useState } from 'react';
import { useDhunStore } from '../../store/useDhunStore';
import {
  Sparkles,
  Sliders,
  Check,
  ArrowRight,
  ArrowLeft,
  Volume2,
  Users,
  ShieldCheck,
  Disc3,
  Flame,
} from 'lucide-react';

export const WizardScreen: React.FC = () => {
  const { moodProfiles, startParty } = useDhunStore();

  const [step, setStep] = useState<number>(1);
  const [partyName, setPartyName] = useState('My Wedding Sangeet');
  const [djLevel, setDjLevel] = useState<number>(2);
  const [primaryMood, setPrimaryMood] = useState('shaadi');
  const [secondaryMood, setSecondaryMood] = useState<string | undefined>('punjabi_tashan');
  const [outputDevice] = useState('Host Browser Audio Output');
  const [requireApproval, setRequireApproval] = useState(true);
  const [maxRequests, setMaxRequests] = useState(3);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const djLevels = [
    {
      level: 1,
      title: 'Level 1 — Professional Two-Deck Mixer',
      desc: 'Manual jog wheels, pitch control, 3-band EQ isolators, crossfader, headphone cue, and baseline auto-play safety.',
    },
    {
      level: 2,
      title: 'Level 2 — Smart Mixing (Recommended)',
      desc: 'Deterministic beat-grid downbeat alignment, Camelot harmonic key matching, smart bass swapping, and energy curves.',
    },
    {
      level: 3,
      title: 'Level 3 — Advanced Performance',
      desc: '4-deck virtual matrix, hot cue juggling, advanced beat loops, resonant filter sweeps, and MIDI integration.',
    },
    {
      level: 4,
      title: 'Level 4 — Creative DJ Tools',
      desc: 'Live mashup tools, phrase-based wordplay assistance, scratching controls, and creative rhythmic performance.',
    },
  ];

  const handleLaunchParty = async () => {
    setIsSubmitting(true);
    try {
      await startParty(partyName, djLevel, primaryMood, secondaryMood);
    } catch (err) {
      console.error(err);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto p-8 max-w-4xl mx-auto flex flex-col justify-between">
      <div className="space-y-6">
        {/* Wizard Progress Stepper */}
        <div className="flex items-center justify-between border-b border-dhun-border pb-4">
          <div>
            <h1 className="text-2xl font-black text-white flex items-center space-x-2">
              <Sparkles className="w-6 h-6 text-dhun-primary" />
              <span>Party Setup Wizard</span>
            </h1>
            <p className="text-xs text-slate-400">Step {step} of 6 &bull; Configure your autonomous DJ and party environment</p>
          </div>
          <div className="flex items-center space-x-1.5">
            {[1, 2, 3, 4, 5, 6].map((s) => (
              <div
                key={s}
                className={`w-8 h-2 rounded-full transition-all ${
                  s === step
                    ? 'bg-dhun-primary w-10'
                    : s < step
                    ? 'bg-rose-900/60'
                    : 'bg-slate-800'
                }`}
              />
            ))}
          </div>
        </div>

        {/* Step 1: DJ Level */}
        {step === 1 && (
          <div className="space-y-4 animate-in fade-in duration-200">
            <div>
              <h2 className="text-lg font-bold text-white">Select DJ Automation &amp; Capability Level</h2>
              <p className="text-xs text-slate-400">All levels feature continuous party playback; advanced levels unlock deeper manual controls.</p>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {djLevels.map((lvl) => (
                <div
                  key={lvl.level}
                  onClick={() => setDjLevel(lvl.level)}
                  className={`p-5 rounded-2xl border transition-all cursor-pointer relative ${
                    djLevel === lvl.level
                      ? 'bg-rose-950/20 border-dhun-primary shadow-lg shadow-rose-950/30'
                      : 'bg-dhun-card border-dhun-border hover:border-slate-600'
                  }`}
                >
                  {djLevel === lvl.level && (
                    <div className="absolute top-4 right-4 w-5 h-5 rounded-full bg-dhun-primary text-white flex items-center justify-center text-xs font-bold">
                      <Check className="w-3.5 h-3.5" />
                    </div>
                  )}
                  <h3 className="font-bold text-sm text-white mb-1.5">{lvl.title}</h3>
                  <p className="text-xs text-slate-400 leading-relaxed">{lvl.desc}</p>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Step 2: Party Mood & Secondary Mood */}
        {step === 2 && (
          <div className="space-y-4 animate-in fade-in duration-200">
            <div>
              <h2 className="text-lg font-bold text-white">Select Primary &amp; Secondary Party Mood</h2>
              <p className="text-xs text-slate-400">Controls BPM progression, Indian genre weights, and transition presets.</p>
            </div>

            <div>
              <label className="text-xs font-semibold text-slate-300 block mb-2 uppercase tracking-wider">Party Name</label>
              <input
                type="text"
                value={partyName}
                onChange={(e) => setPartyName(e.target.value)}
                className="w-full bg-dhun-card border border-dhun-border rounded-xl px-4 py-3 text-sm text-white focus:outline-none focus:border-dhun-primary"
                placeholder="e.g. Rohan &amp; Priya's Wedding Sangeet"
              />
            </div>

            <div className="space-y-2">
              <label className="text-xs font-semibold text-slate-300 block uppercase tracking-wider">Primary Mood</label>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-3 max-h-56 overflow-y-auto pr-1">
                {moodProfiles.map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => setPrimaryMood(m.id)}
                    className={`p-3 rounded-xl border text-left transition-all ${
                      primaryMood === m.id
                        ? 'bg-rose-950/30 border-dhun-primary text-white font-bold'
                        : 'bg-dhun-card border-dhun-border text-slate-300 hover:border-slate-600'
                    }`}
                  >
                    <div className="text-xs font-bold truncate">{m.display_name}</div>
                    <div className="text-[10px] text-slate-400 font-mono mt-0.5">{m.min_bpm}-{m.max_bpm} BPM</div>
                  </button>
                ))}
              </div>
            </div>

            <div className="space-y-2">
              <label className="text-xs font-semibold text-slate-300 block uppercase tracking-wider">Secondary Mood (Optional Blend)</label>
              <select
                value={secondaryMood || ''}
                onChange={(e) => setSecondaryMood(e.target.value || undefined)}
                className="w-full bg-dhun-card border border-dhun-border rounded-xl px-4 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-dhun-primary"
              >
                <option value="">None (Pure primary mood)</option>
                {moodProfiles.map((m) => (
                  <option key={m.id} value={m.id}>{m.display_name} ({m.min_bpm}-{m.max_bpm} BPM)</option>
                ))}
              </select>
            </div>
          </div>
        )}

        {/* Step 3: Music Selection */}
        {step === 3 && (
          <div className="space-y-4 animate-in fade-in duration-200">
            <div>
              <h2 className="text-lg font-bold text-white">Music Collection &amp; Crate Selection</h2>
              <p className="text-xs text-slate-400">Choose which music collection DHUN can automatically pull tracks from.</p>
            </div>

            <div className="space-y-3">
              <div className="p-4 rounded-xl bg-rose-950/20 border border-dhun-primary flex items-center justify-between">
                <div>
                  <div className="font-bold text-sm text-white">All Verified Playable Tracks</div>
                  <div className="text-xs text-slate-400">Entire library (5 verified local tracks available)</div>
                </div>
                <div className="w-5 h-5 rounded-full bg-dhun-primary text-white flex items-center justify-center text-xs font-bold">
                  <Check className="w-3.5 h-3.5" />
                </div>
              </div>

              <div className="p-4 rounded-xl bg-dhun-card border border-dhun-border/70 opacity-60 flex items-center justify-between">
                <div>
                  <div className="font-semibold text-sm text-slate-300">Wedding &amp; Sangeet Special Crate</div>
                  <div className="text-xs text-slate-400">Curated high-energy Bollywood &amp; Punjabi playlist</div>
                </div>
                <span className="text-[10px] uppercase font-bold text-slate-500 bg-slate-800 px-2 py-0.5 rounded">Optional</span>
              </div>
            </div>
          </div>
        )}

        {/* Step 4: Host Browser Speaker Output */}
        {step === 4 && (
          <div className="space-y-4 animate-in fade-in duration-200">
            <div>
              <h2 className="text-lg font-bold text-white">Host Laptop Audio Output</h2>
              <p className="text-xs text-slate-400">
                DHUN plays through your laptop browser&rsquo;s default system audio output (speakers, Bluetooth, or USB DJ interface). Guests do not stream party audio.
              </p>
            </div>

            <div className="space-y-3">
              <div className="p-4 rounded-xl border bg-rose-950/20 border-dhun-primary text-white flex items-center justify-between">
                <div className="flex items-center space-x-3">
                  <Volume2 className="w-5 h-5 text-dhun-accent" />
                  <div>
                    <div className="text-sm font-bold">{outputDevice}</div>
                    <div className="text-xs text-slate-400 font-mono">
                      Managed by Host OS &bull; Prepared for Phase 2 YouTube IFrame Player
                    </div>
                  </div>
                </div>
                <div className="w-5 h-5 rounded-full bg-dhun-primary text-white flex items-center justify-center text-xs font-bold">
                  <Check className="w-3.5 h-3.5" />
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Step 5: Guest Requests & Moderation */}
        {step === 5 && (
          <div className="space-y-4 animate-in fade-in duration-200">
            <div>
              <h2 className="text-lg font-bold text-white">Guest QR Code &amp; Request Policy</h2>
              <p className="text-xs text-slate-400">Configure phone requests over local Wi-Fi without internet requirement.</p>
            </div>

            <div className="space-y-4 bg-dhun-card border border-dhun-border p-5 rounded-2xl">
              <div className="flex items-center justify-between">
                <div>
                  <div className="font-bold text-sm text-white">Require Host Approval</div>
                  <div className="text-xs text-slate-400">Requests wait in pending list until approved by host</div>
                </div>
                <input
                  type="checkbox"
                  checked={requireApproval}
                  onChange={(e) => setRequireApproval(e.target.checked)}
                  className="w-5 h-5 accent-dhun-primary rounded cursor-pointer"
                />
              </div>

              <div className="border-t border-slate-800 pt-3 flex items-center justify-between">
                <div>
                  <div className="font-bold text-sm text-white">Max Requests Per Guest</div>
                  <div className="text-xs text-slate-400">Rate limit to prevent request spamming</div>
                </div>
                <select
                  value={maxRequests}
                  onChange={(e) => setMaxRequests(parseInt(e.target.value))}
                  className="bg-dhun-surface border border-dhun-border rounded-lg px-3 py-1.5 text-xs text-white"
                >
                  <option value={1}>1 song</option>
                  <option value={3}>3 songs (Recommended)</option>
                  <option value={5}>5 songs</option>
                  <option value={10}>10 songs</option>
                </select>
              </div>
            </div>
          </div>
        )}

        {/* Step 6: Summary & Launch */}
        {step === 6 && (
          <div className="space-y-4 animate-in fade-in duration-200">
            <div>
              <h2 className="text-lg font-bold text-white">Setup Summary</h2>
              <p className="text-xs text-slate-400">Review settings and launch party.</p>
            </div>

            <div className="bg-dhun-card border border-dhun-border p-6 rounded-2xl space-y-3 text-xs">
              <div className="flex justify-between py-1 border-b border-slate-800">
                <span className="text-slate-400">Party Name:</span>
                <span className="font-bold text-white">{partyName}</span>
              </div>
              <div className="flex justify-between py-1 border-b border-slate-800">
                <span className="text-slate-400">DJ Level:</span>
                <span className="font-bold text-dhun-accent">Level {djLevel}</span>
              </div>
              <div className="flex justify-between py-1 border-b border-slate-800">
                <span className="text-slate-400">Primary Mood:</span>
                <span className="font-bold text-white uppercase">{primaryMood}</span>
              </div>
              <div className="flex justify-between py-1 border-b border-slate-800">
                <span className="text-slate-400">Audio Output:</span>
                <span className="font-bold text-emerald-400">{outputDevice}</span>
              </div>
              <div className="flex justify-between py-1">
                <span className="text-slate-400">Guest Requests:</span>
                <span className="font-bold text-white">{requireApproval ? 'Host Moderated' : 'Auto-Approve'} ({maxRequests} per guest)</span>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Navigation Buttons */}
      <div className="flex items-center justify-between border-t border-dhun-border pt-4 mt-6">
        {step > 1 ? (
          <button
            type="button"
            onClick={() => setStep(step - 1)}
            className="flex items-center space-x-2 px-4 py-2.5 rounded-xl bg-dhun-card hover:bg-slate-800 border border-dhun-border text-slate-300 font-semibold text-xs transition-all"
          >
            <ArrowLeft className="w-4 h-4" />
            <span>Back</span>
          </button>
        ) : <div />}

        {step < 6 ? (
          <button
            type="button"
            onClick={() => setStep(step + 1)}
            className="flex items-center space-x-2 px-5 py-2.5 rounded-xl bg-dhun-primary hover:bg-rose-600 text-white font-bold text-xs shadow-md shadow-rose-950/40 transition-all"
          >
            <span>Next Step</span>
            <ArrowRight className="w-4 h-4" />
          </button>
        ) : (
          <button
            type="button"
            disabled={isSubmitting}
            onClick={handleLaunchParty}
            className="flex items-center space-x-2 px-6 py-3 rounded-xl bg-gradient-to-r from-dhun-primary to-dhun-accent hover:from-rose-600 hover:to-rose-500 text-white font-black text-sm shadow-xl shadow-rose-950/60 active:scale-95 transition-all"
          >
            <Sparkles className="w-4 h-4" />
            <span>{isSubmitting ? 'Starting Engine...' : 'START PARTY'}</span>
          </button>
        )}
      </div>
    </div>
  );
};

