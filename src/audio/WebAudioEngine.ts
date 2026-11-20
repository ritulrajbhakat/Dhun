// DHUN 4-Deck Web Audio Real-Time Mixing & Level 1–4 Automated DJ Technique Engine
//
// Each DJ Level (1–4) unlocks genuinely audible, deterministic Web Audio DSP techniques
// while strictly inheriting lower-level capabilities:
//   Level 1 — Professional 2-Deck: clean 3-band EQ handover, phrase-aligned long blend, gentle ±4% pitch assist, safe quick cut
//   Level 2 — Smart Mixing: auto BPM + half/double-time sync (±8%), RMS gain matching, timed bass swap, resonant filter sweep
//   Level 3 — Advanced Performance: 4-deck layering, beat-synced echo-out tail, beat-roll build-up, filtered build-up & drop
//   Level 4 — Creative DJ: EQ-carved dual-deck mashup layering, 16th-note stutter gate drop, vinyl backspin brake cut,
//                          high-feedback echo freeze drop, and smooth tempo-ramp bridge.
//
// Progress and transitions are driven exclusively by actual AudioContext.currentTime & decoded AudioBuffers.
// Every executed transition is logged truthfully via `onTechniqueExecuted` (never decorative labels).

import {
  DjLevel,
  DjTechniqueExecutionRecord,
  DjTechniqueId,
  TrackSectionMarker,
  TrackTimingProfile,
  TransitionDecisionPlan,
} from '../types';
import {
  TransitionTimelineScheduler,
  createTempoPolicyForLevel,
  TrackDecisionInput,
} from './TransitionDecisionEngine';

export type TransitionStyle = DjTechniqueId;

export interface DeckTrackMetadata {
  submissionId: string;
  cacheId: string;
  title: string;
  artist: string;
  bpm?: number | null;
  camelotKey?: string | null;
  overallEnergy?: number | null;
  genreTags?: string[];
  sections?: TrackSectionMarker[];
  timingProfile?: TrackTimingProfile | null;
}

export interface IntraTrackJumpPlan {
  fromSec: number;
  toSec: number;
  targetLabel: string;
}

export interface MusicalPerformanceWindow {
  cueInSec: number;
  cueOutSec: number;
  sectionLabel: string;
  sectionSignature: string;
  fallbackUsed: boolean;
  reason: string;
  intraTrackJump: IntraTrackJumpPlan | null;
}

interface WebAudioDeckChain {
  deckId: number;
  channelGain: GainNode;
  eqLow: BiquadFilterNode;
  eqMid: BiquadFilterNode;
  eqHigh: BiquadFilterNode;
  filter: BiquadFilterNode;
  gateGain: GainNode; // Rhythmic stutter/gate & beat-roll envelope
  echoSend: GainNode; // Send into beat-synced feedback delay
  echoDelay: DelayNode;
  echoFeedback: GainNode;
  echoFilter: BiquadFilterNode;
  crossfadeGain: GainNode;
  buffer: AudioBuffer | null;
  rmsGainFactor: number; // Computed RMS loudness normalization factor
  sourceNode: AudioBufferSourceNode | null;
  meta: DeckTrackMetadata | null;
  isPlaying: boolean;
  startedAtCtxTime: number;
  offsetSec: number;
  cueInSec: number;
  cueOutSec: number;
  sectionLabel: string;
  sectionFallbackUsed: boolean;
  intraTrackJump: IntraTrackJumpPlan | null;
  intraJumpExecuted: boolean;
  preloadSignaled: boolean;
  transitionTriggered: boolean;
  isLayering: boolean;
  cleanupTimer: ReturnType<typeof setTimeout> | null;
}

export interface WebAudioDeckStatus {
  deckId: number;
  submissionId: string | null;
  title: string | null;
  artist: string | null;
  isLoaded: boolean;
  isPlaying: boolean;
  positionSec: number;
  durationSec: number;
  bpm: number | null;
  camelotKey: string | null;
  sectionLabel?: string | null;
  cueInSec?: number;
  cueOutSec?: number;
}

export interface AutoDjEngineCallbacks {
  onPreloadWindow?: (activeDeckId: number, activeSubmissionId: string) => void;
  onTransitionStarted?: (
    fromDeckId: number,
    fromSubmissionId: string,
    toDeckId: number,
    toSubmissionId: string,
    execution: DjTechniqueExecutionRecord
  ) => void;
  onTechniqueExecuted?: (execution: DjTechniqueExecutionRecord) => void;
  onTrackEnded?: (deckId: number, submissionId: string) => void;
  onSnapshot?: (snapshots: WebAudioDeckStatus[]) => void;
}

export const TECHNIQUE_LABELS: Record<string, string> = {
  safe_quick_cut: 'Safe Quick Cut',
  eq3_handover: '3-Band EQ Handover',
  phrase_long_blend: 'Phrase-Aligned Long Blend',
  timed_bass_swap: 'Timed Bass Swap',
  filter_sweep_drop: 'Resonant Filter Sweep Drop',
  echo_out_blend: 'Beat-Synced Echo-Out Blend',
  beat_roll_buildup: 'Beat Roll Build-Up',
  filtered_buildup_drop: 'Filtered Build-Up & Drop',
  four_deck_layer: '4-Deck Groove Layering',
  stutter_gate_drop: 'Rhythmic Stutter Gate Drop',
  backspin_cut: 'Vinyl Backspin Brake Cut',
  echo_freeze_drop: 'Echo Freeze Drop',
  tempo_ramp_bridge: 'Tempo Ramp Bridge',
  creative_mashup_layer: 'Creative Dual-Deck Mashup Layer',
  bass_swap: 'Timed Bass Swap',
  quick_cut: 'Safe Quick Cut',
  phrase_sync: 'Phrase-Aligned Long Blend',
  filter_sweep: 'Resonant Filter Sweep Drop',
};

const LEVEL_TECHNIQUE_SET: Record<DjLevel, Set<string>> = {
  1: new Set(['safe_quick_cut', 'eq3_handover', 'phrase_long_blend']),
  2: new Set([
    'safe_quick_cut',
    'eq3_handover',
    'phrase_long_blend',
    'timed_bass_swap',
    'filter_sweep_drop',
  ]),
  3: new Set([
    'safe_quick_cut',
    'eq3_handover',
    'phrase_long_blend',
    'timed_bass_swap',
    'filter_sweep_drop',
    'echo_out_blend',
    'beat_roll_buildup',
    'filtered_buildup_drop',
    'four_deck_layer',
  ]),
  4: new Set([
    'safe_quick_cut',
    'eq3_handover',
    'phrase_long_blend',
    'timed_bass_swap',
    'filter_sweep_drop',
    'echo_out_blend',
    'beat_roll_buildup',
    'filtered_buildup_drop',
    'four_deck_layer',
    'stutter_gate_drop',
    'backspin_cut',
    'echo_freeze_drop',
    'tempo_ramp_bridge',
    'creative_mashup_layer',
  ]),
};

function normalizeTechniqueName(raw: string): DjTechniqueId {
  switch (raw) {
    case 'bass_swap':
      return 'timed_bass_swap';
    case 'quick_cut':
      return 'safe_quick_cut';
    case 'phrase_sync':
      return 'phrase_long_blend';
    case 'filter_sweep':
      return 'filter_sweep_drop';
    default:
      return (TECHNIQUE_LABELS[raw] ? raw : 'eq3_handover') as DjTechniqueId;
  }
}

/**
 * Computes RMS loudness normalization factor for an AudioBuffer so Level 2+ gain matching
 * prevents abrupt volume jumps between differently mastered tracks.
 */
function computeBufferRmsGain(buffer: AudioBuffer): number {
  const ch = buffer.getChannelData(0);
  const step = Math.max(1, Math.floor(ch.length / 20000));
  let sumSq = 0;
  let count = 0;
  for (let i = 0; i < ch.length; i += step) {
    const s = ch[i];
    sumSq += s * s;
    count += 1;
  }
  const rms = Math.sqrt(sumSq / Math.max(1, count));
  if (rms < 0.01) return 1.0;
  // Target steady club loudness RMS ~0.20, safely clamped between -6 dB and +6 dB
  return Math.max(0.5, Math.min(2.0, 0.2 / rms));
}

/**
 * Resolves the exact deterministic technique allowed at `djLevel`, enforcing safe fallbacks
 * when BPM/beat-grid metadata is missing so DHUN never fakes beat alignment.
 */
export function resolveTechniqueForLevel(
  djLevel: DjLevel,
  fromMeta: DeckTrackMetadata | null,
  toMeta: DeckTrackMetadata | null,
  requested: string
): { technique: DjTechniqueId; fallbackUsed: boolean; reason: string; tempoRatio: number } {
  const normalized = normalizeTechniqueName(requested);
  const allowed = LEVEL_TECHNIQUE_SET[djLevel] || LEVEL_TECHNIQUE_SET[2];
  const fromBpm = fromMeta?.bpm ?? null;
  const toBpm = toMeta?.bpm ?? null;
  const bpmKnown = Boolean(fromBpm && toBpm && fromBpm > 40 && toBpm > 40);

  // Compute safe tempo match ratio (supporting half/double-time on Level 2+)
  let tempoRatio = 1.0;
  let bpmDeltaPct = 1.0;
  if (bpmKnown && fromBpm && toBpm) {
    const maxStretch =
      djLevel === 1 ? 0.04 : djLevel === 2 ? 0.08 : djLevel === 3 ? 0.1 : 0.14;
    const directRatio = fromBpm / toBpm;
    const halfRatio = fromBpm / (toBpm * 2);
    const doubleRatio = fromBpm / (toBpm * 0.5);

    let bestRatio = directRatio;
    if (djLevel >= 2) {
      const candidates = [directRatio, halfRatio, doubleRatio];
      candidates.sort((a, b) => Math.abs(1 - a) - Math.abs(1 - b));
      bestRatio = candidates[0];
    }

    bpmDeltaPct = Math.abs(1 - bestRatio);
    if (bpmDeltaPct <= maxStretch) {
      tempoRatio = bestRatio;
    }
  }

  // Safety fallback when BPM/beat metadata is missing
  if (!bpmKnown) {
    if (djLevel >= 3) {
      return {
        technique: 'echo_out_blend',
        fallbackUsed: true,
        reason: 'BPM unverified — using Level 3+ beat-free echo-out tail into clean incoming drop.',
        tempoRatio: 1.0,
      };
    }
    return {
      technique: 'safe_quick_cut',
      fallbackUsed: true,
      reason: 'BPM/beat grid unverified — safe quick cut prevents off-beat clashing.',
      tempoRatio: 1.0,
    };
  }

  // Respect requested technique if unlocked at this DJ level
  if (allowed.has(normalized)) {
    return {
      technique: normalized,
      fallbackUsed: false,
      reason: `Level ${djLevel} (${TECHNIQUE_LABELS[normalized]}) executed with real BPM match (${(tempoRatio * 100).toFixed(1)}% pitch).`,
      tempoRatio,
    };
  }

  // Gate down gracefully to the highest idiomatic technique unlocked for this DJ level
  if (djLevel === 1) {
    const t: DjTechniqueId = bpmDeltaPct <= 0.04 ? 'phrase_long_blend' : 'eq3_handover';
    return {
      technique: t,
      fallbackUsed: true,
      reason: `Level 1 ceiling applied: ${TECHNIQUE_LABELS[t]} (2-deck mix).`,
      tempoRatio,
    };
  }
  if (djLevel === 2) {
    const t: DjTechniqueId = bpmDeltaPct <= 0.08 ? 'timed_bass_swap' : 'filter_sweep_drop';
    return {
      technique: t,
      fallbackUsed: true,
      reason: `Level 2 Smart Mixing applied: ${TECHNIQUE_LABELS[t]} with auto gain & tempo match.`,
      tempoRatio,
    };
  }
  if (djLevel === 3) {
    return {
      technique: 'echo_out_blend',
      fallbackUsed: true,
      reason: 'Level 3 Advanced Performance applied: Beat-Synced Echo-Out Blend.',
      tempoRatio,
    };
  }
  return {
    technique: 'creative_mashup_layer',
    fallbackUsed: false,
    reason: 'Level 4 Creative DJ: EQ-carved dual-deck mashup layering.',
    tempoRatio,
  };
}

/**
 * Deterministic musical-moment & section-boundary planner.
 *
 * Uses actual DSP-analyzed phrase/section markers (`meta.sections`) and BPM.
 * Never fabricates semantic labels or makes arbitrary mid-bar cuts:
 * - Every cue-in and cue-out point snaps to analyzed phrase boundaries.
 * - Tracks previously played section signatures to avoid repeating the same moment.
 * - Falls back honestly to full-track / safe phrase boundaries whenever section
 *   markers are missing, low-confidence (< 0.55), or the track is short (< 24s).
 */
export function planMusicalPerformanceWindow(
  meta: DeckTrackMetadata | null,
  durationSec: number,
  djLevel: DjLevel,
  targetEnergy: number,
  playedSections: Set<string>
): MusicalPerformanceWindow {
  const safeDur = Math.max(0.1, durationSec);
  const sections = meta?.sections || [];
  const subId = meta?.submissionId || 'unknown';
  const hasBpm = Boolean(meta?.bpm && meta.bpm > 40);

  // Strict honesty: fall back to safe full-track playback if no reliable section analysis exists
  const confidentSections = sections.filter(
    (s) => (s.confidence ?? 0) >= 0.55 && s.end_ms > s.start_ms + 2500
  );
  if (!hasBpm || safeDur < 24 || confidentSections.length < 2) {
    return {
      cueInSec: 0,
      cueOutSec: safeDur,
      sectionLabel: 'full_track_fallback',
      sectionSignature: `${subId}:0-${Math.round(safeDur * 1000)}`,
      fallbackUsed: true,
      reason:
        !hasBpm
          ? 'BPM unverified — playing full track without forced section cuts.'
          : safeDur < 24
          ? 'Short track (<24s) — playing full duration without section trimming.'
          : 'Section confidence below threshold — safe full-track phrase fallback.',
      intraTrackJump: null,
    };
  }

  // Filter out intros/outros when searching for primary musical body
  const bodySections = confidentSections.filter(
    (s) => s.section_type !== 'intro' && s.section_type !== 'outro'
  );
  const candidates = bodySections.length > 0 ? bodySections : confidentSections;

  // Score candidate entry sections by party energy fit, musical role, and anti-repetition
  const scoreSection = (s: TrackSectionMarker): number => {
    const sig = `${subId}:${s.start_ms}-${s.end_ms}`;
    let sc = 1.0 - Math.abs(s.energy_level - targetEnergy);
    if (playedSections.has(sig)) sc -= 1.25; // Strong anti-repeat penalty
    if (djLevel >= 3) {
      if (s.section_type === 'build' || s.section_type === 'drop' || s.section_type === 'peak') {
        sc += 0.35;
      } else if (s.section_type === 'groove') {
        sc += 0.2;
      }
    } else if (djLevel === 2) {
      if (s.section_type === 'groove' || s.section_type === 'peak') sc += 0.25;
    } else {
      // Level 1 prefers clean early body entry (first groove/verse after intro)
      sc += s.start_ms < safeDur * 350 ? 0.35 : 0.0;
    }
    return sc;
  };

  let entryIdx = 0;
  let bestScore = -999;
  for (let i = 0; i < candidates.length; i++) {
    const sc = scoreSection(candidates[i]);
    if (sc > bestScore) {
      bestScore = sc;
      entryIdx = i;
    }
  }

  const chosenStart = candidates[entryIdx];
  const fullStartIdx = Math.max(
    0,
    sections.findIndex((s) => s.start_ms === chosenStart.start_ms && s.end_ms === chosenStart.end_ms)
  );

  // Determine phrase-aligned exit boundary based on DJ Level
  const targetWindowSec =
    djLevel === 4 ? 52 : djLevel === 3 ? 68 : djLevel === 2 ? 110 : 140;

  const cueInSec = Math.max(0, Math.min(safeDur - 12, chosenStart.start_ms / 1000));
  let cueOutSec = safeDur;
  let endSectionType = chosenStart.section_type;

  // Walk forward along real analyzed phrase boundaries until reaching target musical moment length
  for (let i = fullStartIdx; i < sections.length; i++) {
    const sec = sections[i];
    const endSec = Math.min(safeDur, sec.end_ms / 1000);
    const span = endSec - cueInSec;
    if (sec.section_type === 'outro' && (sec.confidence ?? 0) >= 0.50 && sec.start_ms / 1000 - cueInSec >= 18) {
      cueOutSec = Math.min(safeDur, sec.start_ms / 1000);
      endSectionType = i > fullStartIdx ? sections[i - 1].section_type : chosenStart.section_type;
      break;
    }
    if (span >= targetWindowSec && span >= 24) {
      cueOutSec = endSec;
      endSectionType = sec.section_type;
      break;
    }
  }

  cueOutSec = Math.max(Math.min(safeDur, cueInSec + 16), Math.min(safeDur, cueOutSec));

  // Level 3–4 optional intra-track beat-aligned jump: skip a mid-window low-energy breakdown
  // straight into a confident build/drop when party energy target is high (>= 0.70)
  let intraTrackJump: IntraTrackJumpPlan | null = null;
  if (djLevel >= 3 && targetEnergy >= 0.70) {
    for (let i = fullStartIdx + 1; i < sections.length - 1; i++) {
      const s = sections[i];
      const sStart = s.start_ms / 1000;
      const sEnd = s.end_ms / 1000;
      if (
        sStart > cueInSec + 8 &&
        sEnd < cueOutSec - 8 &&
        s.section_type === 'breakdown' &&
        (s.confidence ?? 0) >= 0.60
      ) {
        const nextHigh = sections
          .slice(i + 1)
          .find(
            (n) =>
              (n.section_type === 'build' || n.section_type === 'drop' || n.section_type === 'peak') &&
              (n.confidence ?? 0) >= 0.60 &&
              n.start_ms / 1000 < safeDur - 10
          );
        if (nextHigh && nextHigh.start_ms > s.start_ms + 4000) {
          intraTrackJump = {
            fromSec: sStart,
            toSec: nextHigh.start_ms / 1000,
            targetLabel: nextHigh.section_type,
          };
          // Extend cueOutSec so the post-jump drop/peak actually plays out cleanly!
          const postJumpEnd = Math.min(safeDur, nextHigh.end_ms / 1000);
          if (postJumpEnd > cueOutSec) {
            cueOutSec = postJumpEnd;
          }
          break;
        }
      }
    }
  }

  const label =
    chosenStart.section_type === endSectionType
      ? chosenStart.section_type
      : `${chosenStart.section_type}→${endSectionType}`;

  return {
    cueInSec: Number(cueInSec.toFixed(2)),
    cueOutSec: Number(cueOutSec.toFixed(2)),
    sectionLabel: label,
    sectionSignature: `${subId}:${chosenStart.start_ms}-${chosenStart.end_ms}`,
    fallbackUsed: false,
    reason: `Phrase-aligned ${label} moment (${cueInSec.toFixed(1)}s–${cueOutSec.toFixed(1)}s) for L${djLevel} at ${Math.round(targetEnergy * 100)}% energy.`,
    intraTrackJump,
  };
}

export class FourDeckWebAudioEngine {
  private ctx: BaseAudioContext | null = null;
  private masterGain: GainNode | null = null;
  private limiter: DynamicsCompressorNode | null = null;
  private analyser: AnalyserNode | null = null;
  private decks: WebAudioDeckChain[] = [];
  private crossfaderPos = -1.0;
  private djLevel: DjLevel = 2;
  private activeMoodId = 'house_party';
  private targetEnergy = 0.75;
  private activeTransitionStyle: TransitionStyle = 'timed_bass_swap';
  private callbacks: AutoDjEngineCallbacks = {};
  private clockMonitorId: ReturnType<typeof setInterval> | null = null;
  private playedSectionSignatures: Set<string> = new Set();
  private timelineScheduler = new TransitionTimelineScheduler();
  private recentTechniques: string[] = [];

  public init(customCtx?: BaseAudioContext): void {
    if (this.ctx) return;
    if (customCtx) {
      this.ctx = customCtx;
    } else {
      if (typeof window === 'undefined') return;
      const AudioCtx =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      if (!AudioCtx) return;
      this.ctx = new AudioCtx();
    }

    // Master bus -> Brickwall Limiter -> Analyser -> System Speakers
    this.masterGain = this.ctx.createGain();
    this.masterGain.gain.setValueAtTime(0.85, this.ctx.currentTime);

    this.limiter = this.ctx.createDynamicsCompressor();
    this.limiter.threshold.setValueAtTime(-1.0, this.ctx.currentTime);
    this.limiter.knee.setValueAtTime(0.0, this.ctx.currentTime);
    this.limiter.ratio.setValueAtTime(20.0, this.ctx.currentTime);
    this.limiter.attack.setValueAtTime(0.002, this.ctx.currentTime);
    this.limiter.release.setValueAtTime(0.05, this.ctx.currentTime);

    this.analyser = this.ctx.createAnalyser();
    this.analyser.fftSize = 256;
    this.analyser.smoothingTimeConstant = 0.8;

    this.masterGain.connect(this.limiter);
    this.limiter.connect(this.analyser);
    this.analyser.connect(this.ctx.destination);

    // Build all 4 independent deck DSP chains (Decks 0, 1, 2, 3) with Gate + Beat-Synced Echo FX
    this.decks = [0, 1, 2, 3].map((deckId) => {
      const ctx = this.ctx!;
      const channelGain = ctx.createGain();
      channelGain.gain.setValueAtTime(1.0, ctx.currentTime);

      const eqLow = ctx.createBiquadFilter();
      eqLow.type = 'lowshelf';
      eqLow.frequency.value = 240;

      const eqMid = ctx.createBiquadFilter();
      eqMid.type = 'peaking';
      eqMid.frequency.value = 1000;
      eqMid.Q.value = 1.0;

      const eqHigh = ctx.createBiquadFilter();
      eqHigh.type = 'highshelf';
      eqHigh.frequency.value = 3800;

      const filter = ctx.createBiquadFilter();
      filter.type = 'allpass';
      filter.Q.value = 1.8;

      const gateGain = ctx.createGain();
      gateGain.gain.setValueAtTime(1.0, ctx.currentTime);

      const crossfadeGain = ctx.createGain();
      crossfadeGain.gain.setValueAtTime(deckId === 0 ? 1.0 : 0.0, ctx.currentTime);

      // Beat-synced Echo-Out / Echo-Freeze feedback loop (tapped pre-crossfader so tails decay naturally after cut!)
      const echoSend = ctx.createGain();
      echoSend.gain.setValueAtTime(0.0, ctx.currentTime);

      const echoDelay = ctx.createDelay(2.0);
      echoDelay.delayTime.setValueAtTime(0.375, ctx.currentTime);

      const echoFilter = ctx.createBiquadFilter();
      echoFilter.type = 'highpass';
      echoFilter.frequency.setValueAtTime(360, ctx.currentTime);

      const echoFeedback = ctx.createGain();
      echoFeedback.gain.setValueAtTime(0.45, ctx.currentTime);

      // Dry path
      channelGain.connect(eqLow);
      eqLow.connect(eqMid);
      eqMid.connect(eqHigh);
      eqHigh.connect(filter);
      filter.connect(gateGain);
      gateGain.connect(crossfadeGain);
      crossfadeGain.connect(this.masterGain!);

      // Wet echo send path (runs into master independently so echo tails linger over incoming drop)
      gateGain.connect(echoSend);
      echoSend.connect(echoDelay);
      echoDelay.connect(echoFilter);
      echoFilter.connect(echoFeedback);
      echoFeedback.connect(echoDelay);
      echoFilter.connect(this.masterGain!);

      return {
        deckId,
        channelGain,
        eqLow,
        eqMid,
        eqHigh,
        filter,
        gateGain,
        echoSend,
        echoDelay,
        echoFeedback,
        echoFilter,
        crossfadeGain,
        buffer: null,
        rmsGainFactor: 1.0,
        sourceNode: null,
        meta: null,
        isPlaying: false,
        startedAtCtxTime: 0,
        offsetSec: 0,
        cueInSec: 0,
        cueOutSec: 0,
        sectionLabel: 'full_track_fallback',
        sectionFallbackUsed: true,
        intraTrackJump: null,
        intraJumpExecuted: false,
        preloadSignaled: false,
        transitionTriggered: false,
        isLayering: false,
        cleanupTimer: null,
      };
    });
  }

  public setCallbacks(cbs: AutoDjEngineCallbacks): void {
    this.callbacks = cbs;
  }

  public setDjLevel(level: DjLevel): void {
    this.djLevel = level;
  }

  public setMoodId(moodId: string): void {
    this.activeMoodId = moodId || 'house_party';
  }

  public setTargetEnergy(energy: number): void {
    this.targetEnergy = Math.max(0.2, Math.min(1.0, energy));
  }

  public getActiveTransitionPlan(): TransitionDecisionPlan | null {
    return this.timelineScheduler.getActivePlan();
  }

  public getTransitionDiagnostics(): TransitionDecisionPlan[] {
    return this.timelineScheduler.getDiagnosticLog();
  }

  public cancelPendingTransition(reason: string): TransitionDecisionPlan | null {
    return this.timelineScheduler.cancelActivePlan(reason);
  }

  private buildDeckDecisionInput(deck: WebAudioDeckChain): TrackDecisionInput | null {
    if (!deck.buffer || !deck.meta) return null;
    const now = this.ctx?.currentTime ?? 0;
    const currentPosSec = deck.isPlaying ? Math.max(0, now - deck.startedAtCtxTime) : deck.offsetSec;
    const genres = (deck.meta.genreTags || []).map((g) => g.toLowerCase()).join(' ');
    const isElectronicDance = genres.includes('dance') || genres.includes('club') || genres.includes('house') || genres.includes('edm') || genres.includes('techno');
    const isAcousticBallad = genres.includes('acoustic') || genres.includes('ballad') || genres.includes('ghazal') || genres.includes('sufi');
    const isPercussionHeavy = genres.includes('dhol') || genres.includes('bhangra') || genres.includes('punjabi') || genres.includes('garba');
    const avgVocal = deck.meta.sections && deck.meta.sections.length > 0
      ? deck.meta.sections.reduce((acc, s) => acc + (s.vocal_activity ?? 0.5), 0) / deck.meta.sections.length
      : isAcousticBallad ? 0.75 : isElectronicDance ? 0.35 : 0.50;

    return {
      trackId: deck.meta.submissionId,
      deckId: deck.deckId,
      title: deck.meta.title,
      artist: deck.meta.artist,
      durationSec: deck.buffer.duration,
      currentPosSec,
      bpm: deck.meta.bpm ?? null,
      camelotKey: deck.meta.camelotKey ?? null,
      overallEnergy: deck.meta.overallEnergy ?? null,
      genreTags: deck.meta.genreTags || [],
      sections: deck.meta.sections || [],
      timingProfile: deck.meta.timingProfile ?? null,
      vocalDensity: avgVocal,
      isElectronicDance,
      isAcousticBallad,
      isPercussionHeavy,
    };
  }

  /**
   * Plans ahead using the dedicated Transition Decision Engine (WHEN, WHERE, HOW, HOW LONG, BPM relationship)
   * without triggering audio until the scheduled musical boundary arrives on `AudioContext.currentTime`.
   */
  public schedulePlannedTransition(
    fromDeckId: number,
    toDeckId: number
  ): TransitionDecisionPlan | null {
    const fromDeck = this.decks[fromDeckId];
    const toDeck = this.decks[toDeckId];
    if (!fromDeck || !toDeck) return null;
    const outInput = this.buildDeckDecisionInput(fromDeck);
    const inInput = this.buildDeckDecisionInput(toDeck);
    if (!outInput || !inInput) return null;

    const plan = this.timelineScheduler.scheduleOrReplan(outInput, inInput, {
      djLevel: this.djLevel,
      moodId: this.activeMoodId,
      targetEnergy: this.targetEnergy,
      recentTechniques: this.recentTechniques,
      tempoPolicy: createTempoPolicyForLevel(this.djLevel),
    });

    if (plan.status === 'planned') {
      // Align incoming deck cue-in to the planner's chosen musical entry point!
      toDeck.cueInSec = plan.incoming_mix_in_sec;
      toDeck.offsetSec = plan.incoming_mix_in_sec;
    }
    return plan;
  }

  /**
   * Executes a pre-computed `TransitionDecisionPlan` cleanly through the existing 14-technique DSP graph.
   */
  public executePlannedTransition(plan: TransitionDecisionPlan): boolean {
    if (plan.status !== 'planned') return false;
    const toDeck = this.decks[plan.incoming_deck_id];
    if (!toDeck || !toDeck.buffer || !toDeck.meta) {
      this.timelineScheduler.cancelActivePlan('Incoming deck unavailable at execution time');
      return false;
    }
    toDeck.cueInSec = plan.incoming_mix_in_sec;
    toDeck.offsetSec = plan.incoming_mix_in_sec;

    const ok = this.triggerAutomatedTransition(
      plan.outgoing_deck_id,
      plan.incoming_deck_id,
      plan.duration_sec,
      plan.selected_technique,
      plan
    );
    if (ok) {
      this.recentTechniques.unshift(plan.selected_technique);
      if (this.recentTechniques.length > 12) this.recentTechniques.length = 12;
      this.timelineScheduler.markExecuted();
    }
    return ok;
  }

  public setTransitionStyle(style: TransitionStyle): void {
    this.activeTransitionStyle = normalizeTechniqueName(style);
  }

  public async ensureRunning(): Promise<void> {
    this.init();
    if (this.ctx && 'resume' in this.ctx && this.ctx.state === 'suspended') {
      await (this.ctx as AudioContext).resume();
    }
  }

  public getDeckCount(): number {
    return 4;
  }

  public hasActiveAudio(): boolean {
    return this.decks.some((d) => d.isPlaying && d.sourceNode !== null);
  }

  public findIdleDeck(maxActiveDecks: number = 4): number | null {
    const limit = Math.max(2, Math.min(4, maxActiveDecks));
    for (let i = 0; i < limit; i++) {
      const d = this.decks[i];
      if (d && !d.isPlaying && !d.buffer) return i;
    }
    for (let i = 0; i < limit; i++) {
      const d = this.decks[i];
      if (d && !d.isPlaying) return i;
    }
    return null;
  }

  public getActivePlayingDeck(): WebAudioDeckStatus | null {
    const snaps = this.getDeckSnapshots();
    return snaps.find((d) => d.isPlaying) || null;
  }

  public getPreparedIdleDeck(): WebAudioDeckStatus | null {
    const snaps = this.getDeckSnapshots();
    return snaps.find((d) => !d.isPlaying && d.isLoaded && d.submissionId !== null) || null;
  }

  public getPreparedIdleDecks(): WebAudioDeckStatus[] {
    return this.getDeckSnapshots().filter((d) => !d.isPlaying && d.isLoaded && d.submissionId !== null);
  }

  /**
   * Loads a pre-decoded AudioBuffer directly onto an idle deck (also used by deterministic DSP tests).
   */
  public loadDecodedDeckBuffer(
    deckId: number,
    decoded: AudioBuffer,
    meta: DeckTrackMetadata
  ): boolean {
    if (!this.ctx || deckId < 0 || deckId >= this.decks.length) return false;
    const deck = this.decks[deckId];
    if (deck.isPlaying) return false;
    if (!decoded || decoded.duration <= 0) {
      throw new Error('Decoded audio buffer has zero duration');
    }

    const plan = planMusicalPerformanceWindow(
      meta,
      decoded.duration,
      this.djLevel,
      this.targetEnergy,
      this.playedSectionSignatures
    );
    this.playedSectionSignatures.add(plan.sectionSignature);
    if (this.playedSectionSignatures.size > 120) {
      const oldest = this.playedSectionSignatures.values().next().value;
      if (oldest) this.playedSectionSignatures.delete(oldest);
    }

    deck.buffer = decoded;
    deck.rmsGainFactor = computeBufferRmsGain(decoded);
    deck.meta = meta;
    deck.cueInSec = plan.cueInSec;
    deck.cueOutSec = plan.cueOutSec;
    deck.offsetSec = plan.cueInSec;
    deck.sectionLabel = plan.sectionLabel;
    deck.sectionFallbackUsed = plan.fallbackUsed;
    deck.intraTrackJump = plan.intraTrackJump;
    deck.intraJumpExecuted = false;
    deck.preloadSignaled = false;
    deck.transitionTriggered = false;
    deck.isLayering = false;
    this.emitSnapshots();
    return true;
  }

  public async prepareAuthorizedDeckBuffer(
    deckId: number,
    audioBytes: ArrayBuffer,
    meta: DeckTrackMetadata
  ): Promise<boolean> {
    await this.ensureRunning();
    if (!this.ctx || deckId < 0 || deckId >= this.decks.length) return false;
    const deck = this.decks[deckId];
    if (deck.isPlaying) return false;

    const decoded = await this.ctx.decodeAudioData(audioBytes.slice(0));
    return this.loadDecodedDeckBuffer(deckId, decoded, meta);
  }

  private resetDeckDsp(deck: WebAudioDeckChain, now: number): void {
    deck.eqLow.gain.cancelScheduledValues(now);
    deck.eqMid.gain.cancelScheduledValues(now);
    deck.eqHigh.gain.cancelScheduledValues(now);
    deck.eqLow.gain.setValueAtTime(0, now);
    deck.eqMid.gain.setValueAtTime(0, now);
    deck.eqHigh.gain.setValueAtTime(0, now);

    deck.filter.type = 'allpass';
    deck.filter.Q.cancelScheduledValues(now);
    deck.filter.Q.setValueAtTime(1.8, now);
    deck.filter.frequency.cancelScheduledValues(now);
    deck.filter.frequency.setValueAtTime(1000, now);

    deck.gateGain.gain.cancelScheduledValues(now);
    deck.gateGain.gain.setValueAtTime(1.0, now);

    deck.echoSend.gain.cancelScheduledValues(now);
    deck.echoSend.gain.setValueAtTime(0.0, now);
    deck.echoFeedback.gain.cancelScheduledValues(now);
    deck.echoFeedback.gain.setValueAtTime(0.0, now);

    // Apply automatic RMS gain matching on Level 2+ (Level 1 uses unity 1.0 manual gain)
    const gainTarget = this.djLevel >= 2 ? deck.rmsGainFactor : 1.0;
    deck.channelGain.gain.cancelScheduledValues(now);
    deck.channelGain.gain.setValueAtTime(gainTarget, now);
  }

  public startDeck(
    deckId: number,
    playbackRate: number = 1.0,
    initialCrossfadeGain?: number,
    startOffsetSec?: number
  ): boolean {
    if (!this.ctx || deckId < 0 || deckId >= this.decks.length) return false;
    const deck = this.decks[deckId];
    if (!deck.buffer || !deck.meta) return false;

    if (deck.cleanupTimer) {
      clearTimeout(deck.cleanupTimer);
      deck.cleanupTimer = null;
    }

    if (deck.sourceNode) {
      try {
        deck.sourceNode.onended = null;
        deck.sourceNode.stop();
        deck.sourceNode.disconnect();
      } catch {}
    }

    const now = this.ctx.currentTime;
    this.resetDeckDsp(deck, now);

    const otherPlaying = this.decks.some((d) => d.deckId !== deckId && d.isPlaying);
    const startGain =
      initialCrossfadeGain !== undefined ? initialCrossfadeGain : otherPlaying ? 0.0 : 1.0;
    deck.crossfadeGain.gain.cancelScheduledValues(now);
    deck.crossfadeGain.gain.setValueAtTime(startGain, now);

    const effectiveOffset = Math.max(
      0,
      Math.min(
        deck.buffer.duration - 0.5,
        startOffsetSec !== undefined ? startOffsetSec : deck.offsetSec
      )
    );
    deck.offsetSec = effectiveOffset;

    const src = this.ctx.createBufferSource();
    src.buffer = deck.buffer;
    src.playbackRate.setValueAtTime(playbackRate, now);
    src.connect(deck.channelGain);

    const finishedSubmissionId = deck.meta.submissionId;
    const wasLayer = deck.isLayering;
    src.onended = () => {
      if (deck.sourceNode === src) {
        if (deck.cleanupTimer) {
          clearTimeout(deck.cleanupTimer);
          deck.cleanupTimer = null;
        }
        deck.isPlaying = false;
        deck.isLayering = false;
        deck.sourceNode = null;
        deck.buffer = null;
        deck.meta = null;
        deck.offsetSec = 0;
        deck.cueInSec = 0;
        deck.cueOutSec = 0;
        this.emitSnapshots();
        if (!wasLayer) {
          this.callbacks.onTrackEnded?.(deckId, finishedSubmissionId);
        }
        this.maybeStopClockMonitor();
      }
    };

    src.start(now, effectiveOffset);
    deck.sourceNode = src;
    deck.startedAtCtxTime = now - effectiveOffset;
    deck.isPlaying = true;
    deck.preloadSignaled = false;
    deck.transitionTriggered = false;
    this.ensureClockMonitor();
    this.emitSnapshots();
    return true;
  }

  public stopDeck(deckId: number, suppressCallback: boolean = true): void {
    if (deckId < 0 || deckId >= this.decks.length) return;
    const deck = this.decks[deckId];
    if (deck.cleanupTimer) {
      clearTimeout(deck.cleanupTimer);
      deck.cleanupTimer = null;
    }
    const prevSubId = deck.meta?.submissionId ?? null;
    const wasLayer = deck.isLayering;
    if (deck.sourceNode) {
      try {
        deck.sourceNode.onended = null;
        deck.sourceNode.stop();
        deck.sourceNode.disconnect();
      } catch {}
      deck.sourceNode = null;
    }
    if (this.ctx) {
      const now = this.ctx.currentTime;
      deck.crossfadeGain.gain.cancelScheduledValues(now);
      deck.crossfadeGain.gain.setValueAtTime(0.0, now);
      deck.echoSend.gain.cancelScheduledValues(now);
      deck.echoSend.gain.setValueAtTime(0.0, now);
      deck.echoFeedback.gain.cancelScheduledValues(now);
      deck.echoFeedback.gain.setValueAtTime(0.0, now);
    }
    deck.isPlaying = false;
    deck.isLayering = false;
    deck.buffer = null;
    deck.meta = null;
    deck.offsetSec = 0;
    deck.cueInSec = 0;
    deck.cueOutSec = 0;
    this.maybeStopClockMonitor();
    this.emitSnapshots();
    if (!suppressCallback && !wasLayer && prevSubId) {
      this.callbacks.onTrackEnded?.(deckId, prevSubId);
    }
  }

  /**
   * Executes a real, audibly distinct, sample-accurate DJ transition on `AudioContext.currentTime`
   * gated by the active DJ Level (1–4) and metadata confidence, and logs the exact technique performed.
   */
  public triggerAutomatedTransition(
    fromDeckId: number,
    toDeckId: number,
    durationSec: number,
    requestedStyle: TransitionStyle,
    plan?: TransitionDecisionPlan
  ): boolean {
    if (!this.ctx) return false;
    const fromDeck = this.decks[fromDeckId];
    const toDeck = this.decks[toDeckId];
    if (!fromDeck || !toDeck || !toDeck.buffer || !toDeck.meta) return false;
    // Overlapping transition guard: never corrupt an already-active outgoing fade
    if (fromDeck.transitionTriggered) return false;

    const resolved = resolveTechniqueForLevel(
      this.djLevel,
      fromDeck.meta,
      toDeck.meta,
      plan ? plan.selected_technique : requestedStyle
    );
    const technique = plan ? plan.selected_technique : resolved.technique;
    const fallbackUsed = plan ? plan.bpm_relationship === 'no_beatmatch' : resolved.fallbackUsed;
    const reason = plan ? plan.reasons.join(' | ') : resolved.reason;
    const tempoRatio = plan ? plan.incoming_playback_rate : resolved.tempoRatio;

    const now = this.ctx.currentTime;
    const refBpm = plan?.current_bpm || fromDeck.meta?.bpm || toDeck.meta?.bpm || 124;
    const beatSec = Math.max(0.3, Math.min(0.85, 60 / refBpm));

    // Duration tailored to each genuine DJ technique (honoring planned beat/bar duration when supplied!)
    const playableSpan = Math.max(2.0, (toDeck.cueOutSec || toDeck.buffer.duration) - toDeck.cueInSec);
    let dur = plan
      ? Math.max(0.3, Math.min(plan.duration_sec, playableSpan * 0.45))
      : Math.max(0.8, Math.min(durationSec, playableSpan * 0.35));
    if (!plan) {
      if (technique === 'safe_quick_cut') dur = 0.32;
      else if (technique === 'backspin_cut') dur = Math.min(1.1, beatSec * 2);
      else if (technique === 'phrase_long_blend') dur = Math.min(12.0, Math.max(dur, 8.0), playableSpan * 0.35);
      else if (technique === 'four_deck_layer' || technique === 'creative_mashup_layer') {
        dur = Math.min(14.0, Math.max(dur, 10.0), playableSpan * 0.4);
      }
    }

    const bassSwapFrac = plan?.eq_bass_handover_schedule?.bass_swap_at_frac ?? 0.5;
    const mid = now + dur * bassSwapFrac;

    // Atomically start incoming deck at gain 0.0 and its planned phrase-aligned cueInSec (zero click/pop!)
    if (!toDeck.isPlaying) {
      this.startDeck(toDeckId, tempoRatio, 0.0, toDeck.cueInSec);
    }
    fromDeck.transitionTriggered = true;

    // Anchor ALL outgoing AudioParams at `now` BEFORE ramping (prevents Web Audio ramping from t=0!)
    const anchorParam = (param: AudioParam, fallbackVal?: number) => {
      param.cancelScheduledValues(now);
      param.setValueAtTime(fallbackVal !== undefined ? fallbackVal : param.value, now);
    };

    anchorParam(fromDeck.crossfadeGain.gain, Math.max(0.01, fromDeck.crossfadeGain.gain.value || 1.0));
    anchorParam(fromDeck.eqLow.gain);
    anchorParam(fromDeck.eqMid.gain);
    anchorParam(fromDeck.eqHigh.gain);
    anchorParam(fromDeck.gateGain.gain, 1.0);
    anchorParam(fromDeck.echoSend.gain, 0.0);
    anchorParam(fromDeck.echoFeedback.gain, 0.0);

    anchorParam(toDeck.crossfadeGain.gain, 0.0);
    anchorParam(toDeck.eqLow.gain, 0.0);
    anchorParam(toDeck.eqMid.gain, 0.0);
    anchorParam(toDeck.eqHigh.gain, 0.0);
    anchorParam(toDeck.gateGain.gain, 1.0);

    // Optional genuine 3rd-deck rhythmic groove layer on Level 3 & Level 4 when another prepared deck exists
    let layeredDeck: WebAudioDeckChain | null = null;
    if (
      this.djLevel >= 3 &&
      (technique === 'four_deck_layer' || technique === 'creative_mashup_layer')
    ) {
      const candidateLayer = this.decks.find(
        (d) =>
          d.deckId !== fromDeckId &&
          d.deckId !== toDeckId &&
          !d.isPlaying &&
          d.buffer !== null &&
          d.meta !== null
      );
      if (candidateLayer && candidateLayer.meta) {
        const layerResolve = resolveTechniqueForLevel(
          this.djLevel,
          fromDeck.meta,
          candidateLayer.meta,
          technique
        );
        // Only layer a 3rd deck if tempo is genuinely syncable (never layer off-beat!)
        if (!layerResolve.fallbackUsed && Math.abs(1 - layerResolve.tempoRatio) <= 0.14) {
          candidateLayer.isLayering = true;
          if (this.startDeck(candidateLayer.deckId, layerResolve.tempoRatio, 0.0, candidateLayer.cueInSec)) {
            layeredDeck = candidateLayer;
            // Carve out sub/low frequencies on Deck 3 so 3-deck sum stays punchy and never muddies/clips
            layeredDeck.eqLow.gain.setValueAtTime(-30, now);
            layeredDeck.eqMid.gain.setValueAtTime(-4, now);
            layeredDeck.eqHigh.gain.setValueAtTime(1.0, now);
            layeredDeck.filter.type = 'highpass';
            layeredDeck.filter.Q.setValueAtTime(1.4, now);
            layeredDeck.filter.frequency.setValueAtTime(420, now);
            layeredDeck.filter.frequency.exponentialRampToValueAtTime(1800, now + dur * 0.9);
            // Headroom-balanced layer envelope (max 0.36 so 3 summed decks stay safely below limiter)
            layeredDeck.crossfadeGain.gain.setValueAtTime(0.0, now);
            layeredDeck.crossfadeGain.gain.linearRampToValueAtTime(0.36, now + dur * 0.25);
            layeredDeck.crossfadeGain.gain.setValueAtTime(0.32, now + dur * 0.72);
            layeredDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
          } else {
            candidateLayer.isLayering = false;
          }
        }
      }
    }

    // Track whether an echo tail lingers past `now + dur` so deck cleanup waits for clean decay
    let cleanupGraceSec = 0.2;

    // =========================================================================
    // AUDIBLE TECHNIQUE LIBRARY (14 DISTINCT DSP SCHEDULES ACROSS LEVELS 1–4)
    // =========================================================================
    switch (technique) {
      case 'safe_quick_cut': {
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur);
        break;
      }

      case 'eq3_handover': {
        // Level 1: progressive High -> Mid -> Low 3-band EQ handover
        const midCarve = plan?.eq_bass_handover_schedule?.mid_carve_db ?? -14;
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur * 0.6);

        toDeck.eqHigh.gain.setValueAtTime(-16, now);
        toDeck.eqHigh.gain.linearRampToValueAtTime(0, now + dur * 0.35);
        fromDeck.eqHigh.gain.linearRampToValueAtTime(-16, now + dur * 0.45);

        // Vocal interaction aware mid carve:
        if (plan?.vocal_handover_type === 'competing_lead_vocals') {
          // Deep mid carve on both decks during overlap to eliminate vocal clash
          toDeck.eqMid.gain.setValueAtTime(-18, now);
          toDeck.eqMid.gain.linearRampToValueAtTime(midCarve, now + dur * 0.4);
          toDeck.eqMid.gain.linearRampToValueAtTime(0, now + dur * 0.85);
          fromDeck.eqMid.gain.linearRampToValueAtTime(midCarve, now + dur * 0.35);
          fromDeck.eqMid.gain.linearRampToValueAtTime(-24, now + dur * 0.75);
        } else if (plan?.vocal_handover_type === 'vocal_over_instrumental') {
          // Outgoing vocal floats undisturbed while incoming instrumental builds underneath
          toDeck.eqMid.gain.setValueAtTime(-16, now);
          toDeck.eqMid.gain.linearRampToValueAtTime(-4, now + dur * 0.6);
          toDeck.eqMid.gain.linearRampToValueAtTime(0, now + dur * 0.9);
          fromDeck.eqMid.gain.linearRampToValueAtTime(-6, now + dur * 0.75);
        } else {
          toDeck.eqMid.gain.setValueAtTime(-14, now);
          toDeck.eqMid.gain.linearRampToValueAtTime(0, now + dur * 0.65);
          fromDeck.eqMid.gain.linearRampToValueAtTime(-14, now + dur * 0.75);
        }

        toDeck.eqLow.gain.setValueAtTime(-24, now);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, now + dur * 0.9);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-24, now + dur * 0.95);
        break;
      }

      case 'phrase_long_blend': {
        // Level 1: smooth equal-power long blend preserving clarity with gentle low trim
        const steps = 16;
        for (let i = 1; i <= steps; i++) {
          const frac = i / steps;
          const t = now + dur * frac;
          fromDeck.crossfadeGain.gain.linearRampToValueAtTime(Math.cos(frac * Math.PI * 0.5), t);
          toDeck.crossfadeGain.gain.linearRampToValueAtTime(Math.sin(frac * Math.PI * 0.5), t);
        }
        toDeck.eqLow.gain.setValueAtTime(-12, now);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, mid + dur * 0.2);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-14, now + dur);
        break;
      }

      case 'timed_bass_swap': {
        // Level 2: crisp beat-timed bass cut/boost at transition midpoint
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur * 0.65);

        const swapAt = mid;
        toDeck.eqLow.gain.setValueAtTime(-28, now);
        toDeck.eqLow.gain.setValueAtTime(-28, swapAt - 0.04);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, swapAt + 0.08);

        fromDeck.eqLow.gain.setValueAtTime(0, now);
        fromDeck.eqLow.gain.setValueAtTime(0, swapAt - 0.04);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-28, swapAt + 0.08);

        if (plan?.musical_purpose === 'dramatic_drop') {
          // Instantaneous 1/16th beat silence micro-gap right before the drop hits
          fromDeck.crossfadeGain.gain.setValueAtTime(1.0, swapAt - 0.06);
          fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.05, swapAt - 0.01);
          toDeck.crossfadeGain.gain.setValueAtTime(0.0, swapAt - 0.01);
          toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, swapAt + 0.02);
        }
        break;
      }

      case 'filter_sweep_drop':
      case 'filtered_buildup_drop': {
        // Level 2/3: resonant HPF tension sweep on outgoing -> full-bass drop on incoming
        const isExcitement = plan?.musical_purpose === 'build_excitement';
        const isBreathingRoom = plan?.musical_purpose === 'create_breathing_room';
        const peakQ = technique === 'filtered_buildup_drop' ? 6.2 : isExcitement ? 4.8 : 3.2;

        if (isBreathingRoom) {
          // Low-pass warm softening into peaceful/calm incoming track
          fromDeck.filter.type = 'lowpass';
          fromDeck.filter.Q.cancelScheduledValues(now);
          fromDeck.filter.Q.setValueAtTime(1.8, now);
          fromDeck.filter.frequency.cancelScheduledValues(now);
          fromDeck.filter.frequency.setValueAtTime(12000, now);
          fromDeck.filter.frequency.exponentialRampToValueAtTime(800, now + dur * 0.85);
        } else {
          fromDeck.filter.type = 'highpass';
          fromDeck.filter.Q.cancelScheduledValues(now);
          fromDeck.filter.Q.setValueAtTime(peakQ, now);
          fromDeck.filter.frequency.cancelScheduledValues(now);
          fromDeck.filter.frequency.setValueAtTime(75, now);
          fromDeck.filter.frequency.exponentialRampToValueAtTime(isExcitement ? 6400 : 5200, now + dur * 0.88);
        }

        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, now + dur * 0.85);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);

        toDeck.crossfadeGain.gain.linearRampToValueAtTime(0.35, now + dur * 0.78);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur * 0.9);

        toDeck.eqLow.gain.setValueAtTime(-26, now);
        toDeck.eqLow.gain.setValueAtTime(-26, now + dur * 0.84);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, now + dur * 0.92);
        break;
      }

      case 'echo_out_blend': {
        // Level 3: beat-synced 3/4-beat echo send ramp on outgoing + sharp fader drop while echo tail decays cleanly to 0
        cleanupGraceSec = 2.4;
        const delayTime = Math.max(0.18, Math.min(0.75, beatSec * 0.75));
        fromDeck.echoDelay.delayTime.setValueAtTime(delayTime, now);
        fromDeck.echoFeedback.gain.setValueAtTime(0.56, now);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.8, mid);
        fromDeck.echoSend.gain.setValueAtTime(0.8, mid + beatSec * 0.4);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.0, now + dur);
        fromDeck.echoFeedback.gain.setValueAtTime(0.56, now + dur * 0.8);
        fromDeck.echoFeedback.gain.linearRampToValueAtTime(0.0, now + dur + 2.2);

        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, mid);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, mid + 0.12);

        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, mid + 0.15);
        break;
      }

      case 'beat_roll_buildup': {
        // Level 3: click-free micro-ramped 1/2 -> 1/4 -> 1/8 beat stutter roll + rising HPF tension into drop
        fromDeck.filter.type = 'highpass';
        fromDeck.filter.Q.cancelScheduledValues(now);
        fromDeck.filter.Q.setValueAtTime(4.2, now);
        fromDeck.filter.frequency.cancelScheduledValues(now);
        fromDeck.filter.frequency.setValueAtTime(120, now);
        fromDeck.filter.frequency.exponentialRampToValueAtTime(3400, now + dur * 0.85);

        let cursor = now;
        const dropTime = now + dur * 0.88;
        let pulseLen = beatSec * 0.5;
        let pulsesAtRate = 0;
        const ramp = 0.005; // 5ms click-free micro-fade
        while (cursor < dropTime) {
          const offAt = cursor + pulseLen * 0.55;
          fromDeck.gateGain.gain.linearRampToValueAtTime(1.0, cursor + ramp);
          if (offAt < dropTime) {
            fromDeck.gateGain.gain.setValueAtTime(1.0, Math.max(cursor + ramp, offAt - ramp));
            fromDeck.gateGain.gain.linearRampToValueAtTime(0.12, offAt + ramp);
          }
          cursor += pulseLen;
          pulsesAtRate += 1;
          if (pulsesAtRate >= 4 && pulseLen > beatSec * 0.125) {
            pulseLen *= 0.5;
            pulsesAtRate = 0;
          }
        }
        fromDeck.gateGain.gain.linearRampToValueAtTime(1.0, dropTime + 0.04);
        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, dropTime - 0.04);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, dropTime + 0.06);

        toDeck.crossfadeGain.gain.setValueAtTime(0.0, dropTime - 0.04);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, dropTime + 0.06);
        break;
      }

      case 'four_deck_layer': {
        // Level 3: headroom-balanced extended layering (plus optional 3rd deck groove layer above)
        const outgoingHold = layeredDeck ? 0.45 : 0.55;
        const incomingTarget = layeredDeck ? 0.88 : 1.0;

        toDeck.eqLow.gain.setValueAtTime(-24, now);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, now + dur * 0.35);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-28, now + dur * 0.35);
        fromDeck.filter.type = 'highpass';
        fromDeck.filter.frequency.cancelScheduledValues(now);
        fromDeck.filter.frequency.setValueAtTime(320, now + dur * 0.35);
        fromDeck.filter.frequency.exponentialRampToValueAtTime(2200, now + dur);

        toDeck.crossfadeGain.gain.linearRampToValueAtTime(incomingTarget, now + dur * 0.3);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(outgoingHold, now + dur * 0.35);
        fromDeck.crossfadeGain.gain.setValueAtTime(outgoingHold * 0.82, now + dur * 0.8);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
        break;
      }

      case 'stutter_gate_drop': {
        // Level 4: crisp click-free 16th-note rhythmic trance/club gate chop on outgoing track into full drop
        const sixteenth = Math.max(0.06, beatSec * 0.25);
        const dropAt = now + dur * 0.82;
        const ramp = 0.004;
        let t = now;
        while (t < dropAt) {
          const closeAt = t + sixteenth * 0.48;
          fromDeck.gateGain.gain.linearRampToValueAtTime(1.0, t + ramp);
          if (closeAt < dropAt) {
            fromDeck.gateGain.gain.setValueAtTime(1.0, Math.max(t + ramp, closeAt - ramp));
            fromDeck.gateGain.gain.linearRampToValueAtTime(0.02, closeAt + ramp);
          }
          t += sixteenth;
        }
        fromDeck.gateGain.gain.linearRampToValueAtTime(1.0, dropAt + 0.04);
        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, dropAt - 0.03);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, dropAt + 0.05);

        toDeck.crossfadeGain.gain.setValueAtTime(0.0, dropAt - 0.03);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, dropAt + 0.05);
        break;
      }

      case 'backspin_cut': {
        // Level 4: vinyl brake/backspin pitch drop + HPF wash directly into incoming downbeat
        if (fromDeck.sourceNode) {
          fromDeck.sourceNode.playbackRate.cancelScheduledValues(now);
          fromDeck.sourceNode.playbackRate.setValueAtTime(1.0, now);
          fromDeck.sourceNode.playbackRate.exponentialRampToValueAtTime(0.22, now + dur * 0.88);
        }
        fromDeck.filter.type = 'highpass';
        fromDeck.filter.frequency.cancelScheduledValues(now);
        fromDeck.filter.frequency.setValueAtTime(150, now);
        fromDeck.filter.frequency.exponentialRampToValueAtTime(3600, now + dur * 0.88);

        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, now + dur * 0.82);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);

        toDeck.crossfadeGain.gain.setValueAtTime(0.0, now + dur * 0.85);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur);
        break;
      }

      case 'echo_freeze_drop': {
        // Level 4: high-feedback (clamped <= 0.75) 1/2-beat echo freeze wash decaying cleanly to 0 while incoming bass slams in
        cleanupGraceSec = 2.6;
        const halfBeat = Math.max(0.15, Math.min(0.5, beatSec * 0.5));
        fromDeck.echoDelay.delayTime.setValueAtTime(halfBeat, now);
        fromDeck.echoFeedback.gain.setValueAtTime(0.74, now);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.9, now + dur * 0.45);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.0, now + dur * 0.65);
        fromDeck.echoFeedback.gain.setValueAtTime(0.74, now + dur * 0.65);
        fromDeck.echoFeedback.gain.linearRampToValueAtTime(0.0, now + dur + 2.4);

        fromDeck.filter.type = 'highpass';
        fromDeck.filter.frequency.cancelScheduledValues(now);
        fromDeck.filter.frequency.setValueAtTime(250, now);
        fromDeck.filter.frequency.exponentialRampToValueAtTime(4800, now + dur * 0.65);

        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, now + dur * 0.5);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur * 0.62);

        toDeck.crossfadeGain.gain.setValueAtTime(0.0, now + dur * 0.48);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur * 0.62);
        break;
      }

      case 'tempo_ramp_bridge': {
        // Level 4: smooth continuous tempo ramp pulling outgoing & incoming tempos together
        if (fromDeck.sourceNode && toDeck.sourceNode && fromDeck.meta?.bpm && toDeck.meta?.bpm) {
          const outgoingTarget = Math.max(0.86, Math.min(1.14, toDeck.meta.bpm / fromDeck.meta.bpm));
          fromDeck.sourceNode.playbackRate.cancelScheduledValues(now);
          fromDeck.sourceNode.playbackRate.setValueAtTime(1.0, now);
          fromDeck.sourceNode.playbackRate.linearRampToValueAtTime(outgoingTarget, now + dur);
          toDeck.sourceNode.playbackRate.cancelScheduledValues(now);
          toDeck.sourceNode.playbackRate.setValueAtTime(tempoRatio, now);
          toDeck.sourceNode.playbackRate.linearRampToValueAtTime(1.0, now + dur);
        }
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur);
        toDeck.eqLow.gain.setValueAtTime(-24, now);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, mid + 0.1);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-24, mid + 0.1);
        break;
      }

      case 'creative_mashup_layer':
      default: {
        // Level 4: EQ-carved mashup layering (headroom-compensated when 3rd deck active) + midpoint bass swap + clean echo-out
        cleanupGraceSec = 1.8;
        const incMid = layeredDeck ? 0.82 : 0.92;
        const outMid = layeredDeck ? 0.72 : 0.82;

        toDeck.eqLow.gain.setValueAtTime(-30, now);
        toDeck.eqMid.gain.setValueAtTime(2.0, now);
        toDeck.eqHigh.gain.setValueAtTime(1.5, now);
        fromDeck.eqMid.gain.linearRampToValueAtTime(-8, now + dur * 0.25);

        toDeck.crossfadeGain.gain.linearRampToValueAtTime(incMid, now + dur * 0.25);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(outMid, now + dur * 0.25);

        // Midpoint bass swap
        toDeck.eqLow.gain.setValueAtTime(-30, mid - 0.05);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, mid + 0.08);
        toDeck.eqMid.gain.linearRampToValueAtTime(0, mid + 0.4);
        toDeck.eqHigh.gain.linearRampToValueAtTime(0, mid + 0.4);

        fromDeck.eqLow.gain.setValueAtTime(0, mid - 0.05);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-30, mid + 0.08);

        // Beat-synced echo-out tail on outgoing vocal/top layer (decays cleanly to 0)
        fromDeck.echoDelay.delayTime.setValueAtTime(Math.max(0.2, beatSec * 0.5), mid);
        fromDeck.echoSend.gain.setValueAtTime(0.0, mid);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.52, now + dur * 0.85);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.0, now + dur);
        fromDeck.echoFeedback.gain.setValueAtTime(0.44, mid);
        fromDeck.echoFeedback.gain.setValueAtTime(0.44, now + dur * 0.9);
        fromDeck.echoFeedback.gain.linearRampToValueAtTime(0.0, now + dur + 1.6);

        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur);
        break;
      }
    }

    // Restore natural 1.0 playback rate smoothly after transition (except tempo_ramp_bridge which handles it inline)
    if (technique !== 'tempo_ramp_bridge' && tempoRatio !== 1.0 && toDeck.sourceNode) {
      toDeck.sourceNode.playbackRate.setValueAtTime(tempoRatio, now + dur);
      toDeck.sourceNode.playbackRate.linearRampToValueAtTime(1.0, now + dur + 2.0);
    }

    // Schedule clean post-transition source stop/cleanup on real-time AudioContext so outgoing decks
    // do not stay silently busy after section exits (skip setTimeout on OfflineAudioContext!).
    const isRealtimeCtx = typeof AudioContext !== 'undefined' && this.ctx instanceof AudioContext;
    if (isRealtimeCtx) {
      const fromSrcRef = fromDeck.sourceNode;
      const fromCleanupMs = Math.round((dur + cleanupGraceSec) * 1000);
      fromDeck.cleanupTimer = setTimeout(() => {
        if (fromDeck.sourceNode === fromSrcRef) {
          this.stopDeck(fromDeckId, false);
        }
      }, fromCleanupMs);

      if (layeredDeck) {
        const layerDeckId = layeredDeck.deckId;
        const layerSrcRef = layeredDeck.sourceNode;
        layeredDeck.cleanupTimer = setTimeout(() => {
          if (this.decks[layerDeckId]?.sourceNode === layerSrcRef) {
            this.stopDeck(layerDeckId, true);
          }
        }, Math.round((dur + 0.15) * 1000));
      }
    }

    const gainMatchDb =
      this.djLevel >= 2 ? Number((20 * Math.log10(toDeck.rmsGainFactor)).toFixed(1)) : 0;

    const execution: DjTechniqueExecutionRecord = {
      id: `tx_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      timestamp: Date.now(),
      djLevel: this.djLevel,
      techniqueId: technique,
      techniqueLabel: TECHNIQUE_LABELS[technique] || technique,
      outgoingTitle: fromDeck.meta?.title || `Deck ${fromDeckId + 1}`,
      incomingTitle: toDeck.meta.title,
      fromDeckId,
      toDeckId,
      durationSec: Number(dur.toFixed(2)),
      tempoMatchRatio: Number(tempoRatio.toFixed(3)),
      gainMatchDb,
      fallbackUsed: fallbackUsed || toDeck.sectionFallbackUsed,
      reason: layeredDeck
        ? `${reason} + 3rd-deck carved groove layer on Deck ${String.fromCharCode(65 + layeredDeck.deckId)}.`
        : reason,
      sectionLabel: toDeck.sectionLabel,
      cueInSec: toDeck.cueInSec,
      cueOutSec: toDeck.cueOutSec,
      layeredDeckId: layeredDeck ? layeredDeck.deckId : null,
    };

    this.callbacks.onTechniqueExecuted?.(execution);

    if (fromDeck.meta) {
      this.callbacks.onTransitionStarted?.(
        fromDeckId,
        fromDeck.meta.submissionId,
        toDeckId,
        toDeck.meta.submissionId,
        execution
      );
    }

    return true;
  }

  /**
   * Executes a smooth, beat-aligned intra-track section jump (Level 3–4) using an idle deck
   * for a 120ms click-free equal-power crossfade without audio interruption.
   */
  private triggerIntraTrackSectionJump(deck: WebAudioDeckChain): boolean {
    if (!this.ctx || !deck.buffer || !deck.meta || !deck.intraTrackJump || deck.intraJumpExecuted) {
      return false;
    }
    const jump = deck.intraTrackJump;
    const now = this.ctx.currentTime;
    const oldSrc = deck.sourceNode;
    if (!oldSrc) return false;

    deck.intraJumpExecuted = true;

    // 90ms click-free micro-dip + immediate restart at beat-aligned target section boundary
    const newSrc = this.ctx.createBufferSource();
    newSrc.buffer = deck.buffer;
    newSrc.playbackRate.setValueAtTime(oldSrc.playbackRate.value || 1.0, now);
    newSrc.connect(deck.channelGain);

    deck.gateGain.gain.cancelScheduledValues(now);
    deck.gateGain.gain.setValueAtTime(1.0, now);
    deck.gateGain.gain.linearRampToValueAtTime(0.05, now + 0.045);
    deck.gateGain.gain.linearRampToValueAtTime(1.0, now + 0.095);

    const finishedSubmissionId = deck.meta.submissionId;
    newSrc.onended = () => {
      if (deck.sourceNode === newSrc) {
        deck.isPlaying = false;
        deck.sourceNode = null;
        deck.buffer = null;
        deck.meta = null;
        deck.offsetSec = 0;
        this.emitSnapshots();
        this.callbacks.onTrackEnded?.(deck.deckId, finishedSubmissionId);
        this.maybeStopClockMonitor();
      }
    };

    try {
      oldSrc.onended = null;
      oldSrc.stop(now + 0.045);
    } catch {}

    newSrc.start(now + 0.045, jump.toSec);
    deck.sourceNode = newSrc;
    deck.offsetSec = jump.toSec;
    deck.startedAtCtxTime = now + 0.045 - jump.toSec;

    this.callbacks.onTechniqueExecuted?.({
      id: `jmp_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      timestamp: Date.now(),
      djLevel: this.djLevel,
      techniqueId: 'phrase_long_blend',
      techniqueLabel: `Intra-Track Jump → ${jump.targetLabel}`,
      outgoingTitle: deck.meta.title,
      incomingTitle: deck.meta.title,
      fromDeckId: deck.deckId,
      toDeckId: deck.deckId,
      durationSec: 0.1,
      tempoMatchRatio: 1.0,
      gainMatchDb: 0,
      fallbackUsed: false,
      reason: `Skipped breakdown at ${jump.fromSec.toFixed(1)}s directly onto phrase-aligned ${jump.targetLabel} (${jump.toSec.toFixed(1)}s).`,
      sectionLabel: jump.targetLabel,
      cueInSec: jump.toSec,
      cueOutSec: deck.cueOutSec,
    });

    return true;
  }

  private ensureClockMonitor(): void {
    // Never start real-time setInterval polling on OfflineAudioContext or headless test contexts
    if (
      typeof window === 'undefined' ||
      typeof AudioContext === 'undefined' ||
      !(this.ctx instanceof AudioContext)
    ) {
      return;
    }
    if (this.clockMonitorId) return;
    this.clockMonitorId = setInterval(() => {
      if (!this.ctx) return;
      const now = this.ctx.currentTime;
      let anyPlaying = false;

      for (const deck of this.decks) {
        if (!deck.isPlaying || !deck.buffer || !deck.meta || deck.isLayering) continue;
        anyPlaying = true;
        const pos = Math.max(0, now - deck.startedAtCtxTime);
        const targetExit = deck.cueOutSec > deck.cueInSec + 6 ? deck.cueOutSec : deck.buffer.duration;
        const remaining = Math.max(0, targetExit - pos);

        // Execute planned Level 3–4 intra-track section jump at breakdown boundary
        if (
          this.djLevel >= 3 &&
          deck.intraTrackJump &&
          !deck.intraJumpExecuted &&
          !deck.transitionTriggered &&
          pos >= deck.intraTrackJump.fromSec
        ) {
          this.triggerIntraTrackSectionJump(deck);
          continue;
        }

        const preloadThreshold = Math.min(90, Math.max(30, (targetExit - deck.cueInSec) * 0.60));
        if (!deck.preloadSignaled && remaining <= preloadThreshold) {
          deck.preloadSignaled = true;
          this.callbacks.onPreloadWindow?.(deck.deckId, deck.meta.submissionId);
        }

        if (!deck.transitionTriggered) {
          const prepared = this.decks.find(
            (d) =>
              d.deckId !== deck.deckId &&
              !d.isPlaying &&
              !d.isLayering &&
              d.buffer !== null &&
              d.meta !== null
          );
          if (prepared) {
            // Musical Transition Timeline Scheduler: plan ahead, wait for exact beat/phrase boundary,
            // and replan cleanly if preparation finished late past a phrase boundary!
            let activePlan = this.timelineScheduler.getActivePlan();
            if (
              !activePlan ||
              activePlan.outgoing_deck_id !== deck.deckId ||
              activePlan.incoming_deck_id !== prepared.deckId
            ) {
              activePlan = this.schedulePlannedTransition(deck.deckId, prepared.deckId);
            }

            if (activePlan && activePlan.status === 'planned') {
              const tick = this.timelineScheduler.evaluateClockTick(pos, deck.buffer.duration);
              if (tick === 'replan_needed') {
                this.schedulePlannedTransition(deck.deckId, prepared.deckId);
              } else if (tick === 'execute') {
                this.executePlannedTransition(activePlan);
              }
            } else if (remaining <= 10.0 && remaining >= 1.0) {
              // Automatic mix-out controller:
              // Outgoing track is nearing end (<= 10s remaining) and no transition executed yet.
              // Execute an intentional musical handover rather than playing indefinitely into silence!
              const fallbackTechnique: DjTechniqueId =
                this.djLevel === 1 ? 'eq3_handover' : 'filter_sweep_drop';
              const handoverDur = Math.max(1.5, Math.min(4.5, remaining - 0.4));
              this.triggerAutomatedTransition(
                deck.deckId,
                prepared.deckId,
                handoverDur,
                fallbackTechnique
              );
            }
          } else if (remaining <= 2.2 && !deck.transitionTriggered) {
            // Clean fade-out safeguard when no incoming track is available, preventing click/hang
            deck.transitionTriggered = true;
            const t = this.ctx.currentTime;
            deck.crossfadeGain.gain.setValueAtTime(deck.crossfadeGain.gain.value, t);
            deck.crossfadeGain.gain.linearRampToValueAtTime(0.0, t + Math.max(0.5, remaining - 0.2));
          }
        }
      }

      if (anyPlaying) {
        this.emitSnapshots();
      } else {
        this.maybeStopClockMonitor();
      }
    }, 250);
  }

  private maybeStopClockMonitor(): void {
    const anyPlaying = this.decks.some((d) => d.isPlaying);
    if (!anyPlaying && this.clockMonitorId) {
      clearInterval(this.clockMonitorId);
      this.clockMonitorId = null;
    }
  }

  private emitSnapshots(): void {
    this.callbacks.onSnapshot?.(this.getDeckSnapshots());
  }

  public setMasterVolume(vol: number): void {
    if (!this.masterGain || !this.ctx) return;
    const clamped = Math.max(0, Math.min(1.2, vol));
    this.masterGain.gain.setTargetAtTime(clamped, this.ctx.currentTime, 0.03);
  }

  public setCrossfader(pos: number): void {
    this.crossfaderPos = Math.max(-1, Math.min(1, pos));
    if (!this.ctx || this.decks.length < 4) return;

    const norm = (this.crossfaderPos + 1) / 2;
    const gainLeft = Math.cos(norm * Math.PI * 0.5);
    const gainRight = Math.sin(norm * Math.PI * 0.5);
    const t = this.ctx.currentTime;

    this.decks[0].crossfadeGain.gain.setTargetAtTime(gainLeft, t, 0.02);
    this.decks[2].crossfadeGain.gain.setTargetAtTime(gainLeft, t, 0.02);
    this.decks[1].crossfadeGain.gain.setTargetAtTime(gainRight, t, 0.02);
    this.decks[3].crossfadeGain.gain.setTargetAtTime(gainRight, t, 0.02);
  }

  public setDeckEq(deckId: number, lowDb: number, midDb: number, highDb: number): void {
    if (!this.ctx || !this.decks[deckId]) return;
    const d = this.decks[deckId];
    const t = this.ctx.currentTime;
    d.eqLow.gain.setTargetAtTime(lowDb, t, 0.03);
    d.eqMid.gain.setTargetAtTime(midDb, t, 0.03);
    d.eqHigh.gain.setTargetAtTime(highDb, t, 0.03);
  }

  public setDeckGain(deckId: number, gain: number): void {
    if (!this.ctx || !this.decks[deckId]) return;
    this.decks[deckId].channelGain.gain.setTargetAtTime(gain, this.ctx.currentTime, 0.03);
  }

  public setDeckTempo(deckId: number, rate: number): void {
    if (!this.ctx || !this.decks[deckId]) return;
    const clamped = Math.max(0.5, Math.min(1.5, rate));
    const src = this.decks[deckId].sourceNode;
    if (src) {
      src.playbackRate.setTargetAtTime(clamped, this.ctx.currentTime, 0.03);
    }
  }

  public setDeckFilter(deckId: number, pos: number): void {
    if (!this.ctx || !this.decks[deckId]) return;
    const f = this.decks[deckId].filter;
    const t = this.ctx.currentTime;
    if (Math.abs(pos) < 0.05) {
      f.type = 'allpass';
    } else if (pos < 0) {
      f.type = 'lowpass';
      const freq = 20000 * Math.pow(200 / 20000, Math.abs(pos));
      f.frequency.setTargetAtTime(freq, t, 0.03);
    } else {
      f.type = 'highpass';
      const freq = 20 * Math.pow(8000 / 20, pos);
      f.frequency.setTargetAtTime(freq, t, 0.03);
    }
  }

  public async emergencyFadeAndStop(durationMs: number = 250): Promise<void> {
    if (!this.masterGain || !this.ctx) {
      this.decks.forEach((d) => this.stopDeck(d.deckId));
      return;
    }
    const t = this.ctx.currentTime;
    this.masterGain.gain.cancelScheduledValues(t);
    this.masterGain.gain.setValueAtTime(this.masterGain.gain.value, t);
    this.masterGain.gain.linearRampToValueAtTime(0.0001, t + durationMs / 1000);
    await new Promise((resolve) => setTimeout(resolve, durationMs + 40));
    this.decks.forEach((d) => this.stopDeck(d.deckId));
    if (this.masterGain && this.ctx) {
      this.masterGain.gain.setValueAtTime(0.85, this.ctx.currentTime);
    }
  }

  public emergencyFade(durationMs: number = 300): void {
    void this.emergencyFadeAndStop(durationMs);
  }

  public getWaveformData(targetArray: Uint8Array<ArrayBuffer>): void {
    if (!this.analyser || !this.hasActiveAudio()) {
      targetArray.fill(128);
      return;
    }
    this.analyser.getByteTimeDomainData(targetArray);
  }

  public getDeckSnapshots(): WebAudioDeckStatus[] {
    const now = this.ctx?.currentTime ?? 0;
    return this.decks.map((d) => ({
      deckId: d.deckId,
      submissionId: d.meta?.submissionId ?? null,
      title: d.meta?.title ?? null,
      artist: d.meta?.artist ?? null,
      isLoaded: d.buffer !== null,
      isPlaying: d.isPlaying,
      positionSec: d.isPlaying ? Math.max(0, now - d.startedAtCtxTime) : d.offsetSec,
      durationSec: d.buffer ? d.buffer.duration : 0,
      bpm: d.meta?.bpm ?? null,
      camelotKey: d.meta?.camelotKey ?? null,
      sectionLabel: d.buffer ? d.sectionLabel : null,
      cueInSec: d.cueInSec,
      cueOutSec: d.cueOutSec,
    }));
  }
}

export const webAudioEngine = new FourDeckWebAudioEngine();
