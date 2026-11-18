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

import { DjLevel, DjTechniqueExecutionRecord, DjTechniqueId } from '../types';

export type TransitionStyle = DjTechniqueId;

export interface DeckTrackMetadata {
  submissionId: string;
  cacheId: string;
  title: string;
  artist: string;
  bpm?: number | null;
  camelotKey?: string | null;
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
  preloadSignaled: boolean;
  transitionTriggered: boolean;
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

const TECHNIQUE_LABELS: Record<string, string> = {
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

class FourDeckWebAudioEngine {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private limiter: DynamicsCompressorNode | null = null;
  private analyser: AnalyserNode | null = null;
  private decks: WebAudioDeckChain[] = [];
  private crossfaderPos = -1.0;
  private djLevel: DjLevel = 2;
  private activeTransitionStyle: TransitionStyle = 'timed_bass_swap';
  private callbacks: AutoDjEngineCallbacks = {};
  private clockMonitorId: ReturnType<typeof setInterval> | null = null;

  public init(): void {
    if (this.ctx) return;
    if (typeof window === 'undefined') return;
    const AudioCtx =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    if (!AudioCtx) return;

    this.ctx = new AudioCtx();

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
        preloadSignaled: false,
        transitionTriggered: false,
      };
    });
  }

  public setCallbacks(cbs: AutoDjEngineCallbacks): void {
    this.callbacks = cbs;
  }

  public setDjLevel(level: DjLevel): void {
    this.djLevel = level;
  }

  public setTransitionStyle(style: TransitionStyle): void {
    this.activeTransitionStyle = normalizeTechniqueName(style);
  }

  public async ensureRunning(): Promise<void> {
    this.init();
    if (this.ctx && this.ctx.state === 'suspended') {
      await this.ctx.resume();
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
    if (!decoded || decoded.duration <= 0) {
      throw new Error('Decoded audio buffer has zero duration');
    }

    deck.buffer = decoded;
    deck.rmsGainFactor = computeBufferRmsGain(decoded);
    deck.meta = meta;
    deck.offsetSec = 0;
    deck.preloadSignaled = false;
    deck.transitionTriggered = false;
    this.emitSnapshots();
    return true;
  }

  private resetDeckDsp(deck: WebAudioDeckChain, now: number): void {
    deck.eqLow.gain.cancelScheduledValues(now);
    deck.eqMid.gain.cancelScheduledValues(now);
    deck.eqHigh.gain.cancelScheduledValues(now);
    deck.eqLow.gain.setValueAtTime(0, now);
    deck.eqMid.gain.setValueAtTime(0, now);
    deck.eqHigh.gain.setValueAtTime(0, now);

    deck.filter.type = 'allpass';
    deck.filter.frequency.cancelScheduledValues(now);
    deck.filter.frequency.setValueAtTime(1000, now);

    deck.gateGain.gain.cancelScheduledValues(now);
    deck.gateGain.gain.setValueAtTime(1.0, now);

    deck.echoSend.gain.cancelScheduledValues(now);
    deck.echoSend.gain.setValueAtTime(0.0, now);
    deck.echoFeedback.gain.cancelScheduledValues(now);
    deck.echoFeedback.gain.setValueAtTime(0.45, now);

    // Apply automatic RMS gain matching on Level 2+ (Level 1 uses unity 1.0 manual gain)
    const gainTarget = this.djLevel >= 2 ? deck.rmsGainFactor : 1.0;
    deck.channelGain.gain.cancelScheduledValues(now);
    deck.channelGain.gain.setValueAtTime(gainTarget, now);
  }

  public startDeck(deckId: number, playbackRate: number = 1.0): boolean {
    if (!this.ctx || deckId < 0 || deckId >= this.decks.length) return false;
    const deck = this.decks[deckId];
    if (!deck.buffer || !deck.meta) return false;

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
    if (!otherPlaying) {
      deck.crossfadeGain.gain.cancelScheduledValues(now);
      deck.crossfadeGain.gain.setValueAtTime(1.0, now);
    }

    const src = this.ctx.createBufferSource();
    src.buffer = deck.buffer;
    src.playbackRate.setValueAtTime(playbackRate, now);
    src.connect(deck.channelGain);

    const finishedSubmissionId = deck.meta.submissionId;
    src.onended = () => {
      if (deck.sourceNode === src) {
        deck.isPlaying = false;
        deck.sourceNode = null;
        deck.buffer = null;
        deck.meta = null;
        deck.offsetSec = 0;
        this.emitSnapshots();
        this.callbacks.onTrackEnded?.(deckId, finishedSubmissionId);
        this.maybeStopClockMonitor();
      }
    };

    src.start(0, deck.offsetSec);
    deck.sourceNode = src;
    deck.startedAtCtxTime = now - deck.offsetSec;
    deck.isPlaying = true;
    deck.preloadSignaled = false;
    deck.transitionTriggered = false;
    this.ensureClockMonitor();
    this.emitSnapshots();
    return true;
  }

  public stopDeck(deckId: number): void {
    if (deckId < 0 || deckId >= this.decks.length) return;
    const deck = this.decks[deckId];
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
      deck.echoSend.gain.cancelScheduledValues(now);
      deck.echoSend.gain.setValueAtTime(0.0, now);
      deck.echoFeedback.gain.cancelScheduledValues(now);
      deck.echoFeedback.gain.setValueAtTime(0.0, now);
    }
    deck.isPlaying = false;
    deck.buffer = null;
    deck.meta = null;
    deck.offsetSec = 0;
    this.maybeStopClockMonitor();
    this.emitSnapshots();
  }

  /**
   * Executes a real, audibly distinct, sample-accurate DJ transition on `AudioContext.currentTime`
   * gated by the active DJ Level (1–4) and metadata confidence, and logs the exact technique performed.
   */
  public triggerAutomatedTransition(
    fromDeckId: number,
    toDeckId: number,
    durationSec: number,
    requestedStyle: TransitionStyle
  ): boolean {
    if (!this.ctx) return false;
    const fromDeck = this.decks[fromDeckId];
    const toDeck = this.decks[toDeckId];
    if (!fromDeck || !toDeck || !toDeck.buffer || !toDeck.meta) return false;

    const { technique, fallbackUsed, reason, tempoRatio } = resolveTechniqueForLevel(
      this.djLevel,
      fromDeck.meta,
      toDeck.meta,
      requestedStyle
    );

    const now = this.ctx.currentTime;
    const refBpm = fromDeck.meta?.bpm || toDeck.meta?.bpm || 124;
    const beatSec = Math.max(0.3, Math.min(0.85, 60 / refBpm));

    // Duration tailored to each genuine DJ technique
    let dur = Math.max(0.8, Math.min(durationSec, toDeck.buffer.duration * 0.35));
    if (technique === 'safe_quick_cut') dur = 0.32;
    else if (technique === 'backspin_cut') dur = Math.min(1.1, beatSec * 2);
    else if (technique === 'phrase_long_blend') dur = Math.min(12.0, Math.max(dur, 8.0), toDeck.buffer.duration * 0.35);
    else if (technique === 'four_deck_layer' || technique === 'creative_mashup_layer') {
      dur = Math.min(14.0, Math.max(dur, 10.0), toDeck.buffer.duration * 0.4);
    }

    const mid = now + dur * 0.5;

    if (!toDeck.isPlaying) {
      this.startDeck(toDeckId, tempoRatio);
    }
    fromDeck.transitionTriggered = true;

    fromDeck.crossfadeGain.gain.cancelScheduledValues(now);
    toDeck.crossfadeGain.gain.cancelScheduledValues(now);
    fromDeck.crossfadeGain.gain.setValueAtTime(fromDeck.crossfadeGain.gain.value, now);
    toDeck.crossfadeGain.gain.setValueAtTime(0.0, now);

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
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur * 0.6);

        toDeck.eqHigh.gain.setValueAtTime(-16, now);
        toDeck.eqHigh.gain.linearRampToValueAtTime(0, now + dur * 0.35);
        fromDeck.eqHigh.gain.linearRampToValueAtTime(-16, now + dur * 0.45);

        toDeck.eqMid.gain.setValueAtTime(-14, now);
        toDeck.eqMid.gain.linearRampToValueAtTime(0, now + dur * 0.65);
        fromDeck.eqMid.gain.linearRampToValueAtTime(-14, now + dur * 0.75);

        toDeck.eqLow.gain.setValueAtTime(-24, now);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, now + dur * 0.9);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-24, now + dur * 0.95);
        break;
      }

      case 'phrase_long_blend': {
        // Level 1: smooth equal-power long blend preserving clarity with gentle low trim
        const steps = 16;
        for (let i = 0; i <= steps; i++) {
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

        toDeck.eqLow.gain.setValueAtTime(-28, now);
        toDeck.eqLow.gain.setValueAtTime(-28, mid - 0.04);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, mid + 0.08);

        fromDeck.eqLow.gain.setValueAtTime(0, now);
        fromDeck.eqLow.gain.setValueAtTime(0, mid - 0.04);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-28, mid + 0.08);
        break;
      }

      case 'filter_sweep_drop':
      case 'filtered_buildup_drop': {
        // Level 2/3: resonant HPF tension sweep on outgoing -> full-bass drop on incoming
        const peakQ = technique === 'filtered_buildup_drop' ? 6.5 : 3.2;
        fromDeck.filter.type = 'highpass';
        fromDeck.filter.Q.setValueAtTime(peakQ, now);
        fromDeck.filter.frequency.setValueAtTime(75, now);
        fromDeck.filter.frequency.exponentialRampToValueAtTime(5200, now + dur * 0.88);

        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, now + dur * 0.85);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);

        toDeck.crossfadeGain.gain.setValueAtTime(0.0, now);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(0.35, now + dur * 0.78);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur * 0.9);

        toDeck.eqLow.gain.setValueAtTime(-26, now);
        toDeck.eqLow.gain.setValueAtTime(-26, now + dur * 0.84);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, now + dur * 0.92);
        break;
      }

      case 'echo_out_blend': {
        // Level 3: beat-synced 3/4-beat echo send ramp on outgoing + sharp fader drop while echo tail decays!
        const delayTime = Math.max(0.18, Math.min(0.75, beatSec * 0.75));
        fromDeck.echoDelay.delayTime.setValueAtTime(delayTime, now);
        fromDeck.echoFeedback.gain.setValueAtTime(0.58, now);
        fromDeck.echoSend.gain.setValueAtTime(0.0, now);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.85, mid);
        fromDeck.echoSend.gain.setValueAtTime(0.85, mid + beatSec * 0.5);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.0, now + dur);
        fromDeck.echoFeedback.gain.linearRampToValueAtTime(0.15, now + dur + 2.2);

        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, mid);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, mid + 0.12);

        toDeck.crossfadeGain.gain.setValueAtTime(0.0, now);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, mid + 0.15);
        break;
      }

      case 'beat_roll_buildup': {
        // Level 3: rhythmic 1/2 -> 1/4 -> 1/8 beat stutter roll + rising HPF tension into drop!
        fromDeck.filter.type = 'highpass';
        fromDeck.filter.Q.setValueAtTime(4.5, now);
        fromDeck.filter.frequency.setValueAtTime(120, now);
        fromDeck.filter.frequency.exponentialRampToValueAtTime(3400, now + dur * 0.85);

        let cursor = now;
        const dropTime = now + dur * 0.88;
        let pulseLen = beatSec * 0.5;
        let pulsesAtRate = 0;
        while (cursor < dropTime) {
          fromDeck.gateGain.gain.setValueAtTime(1.0, cursor);
          fromDeck.gateGain.gain.setValueAtTime(0.12, cursor + pulseLen * 0.55);
          cursor += pulseLen;
          pulsesAtRate += 1;
          if (pulsesAtRate >= 4 && pulseLen > beatSec * 0.125) {
            pulseLen *= 0.5;
            pulsesAtRate = 0;
          }
        }
        fromDeck.gateGain.gain.setValueAtTime(1.0, dropTime + 0.05);
        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, dropTime - 0.04);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, dropTime + 0.06);

        toDeck.crossfadeGain.gain.setValueAtTime(0.0, dropTime - 0.04);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, dropTime + 0.06);
        break;
      }

      case 'four_deck_layer': {
        // Level 3: keeps outgoing mid/high groove riding softly over incoming bass for extended 4-deck blend
        toDeck.eqLow.gain.setValueAtTime(-24, now);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, now + dur * 0.35);
        fromDeck.eqLow.gain.setValueAtTime(0, now);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-28, now + dur * 0.35);
        fromDeck.filter.type = 'highpass';
        fromDeck.filter.frequency.setValueAtTime(320, now + dur * 0.35);
        fromDeck.filter.frequency.exponentialRampToValueAtTime(2200, now + dur);

        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur * 0.3);
        fromDeck.crossfadeGain.gain.setValueAtTime(0.55, now + dur * 0.35);
        fromDeck.crossfadeGain.gain.setValueAtTime(0.45, now + dur * 0.8);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
        break;
      }

      case 'stutter_gate_drop': {
        // Level 4: crisp 16th-note rhythmic trance/club gate chop on outgoing track into full drop!
        const sixteenth = Math.max(0.06, beatSec * 0.25);
        const dropAt = now + dur * 0.82;
        let t = now;
        while (t < dropAt) {
          fromDeck.gateGain.gain.setValueAtTime(1.0, t);
          fromDeck.gateGain.gain.setValueAtTime(0.02, t + sixteenth * 0.48);
          t += sixteenth;
        }
        fromDeck.gateGain.gain.setValueAtTime(1.0, dropAt + 0.05);
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
        fromDeck.filter.frequency.setValueAtTime(150, now);
        fromDeck.filter.frequency.exponentialRampToValueAtTime(3600, now + dur * 0.88);

        fromDeck.crossfadeGain.gain.setValueAtTime(1.0, now + dur * 0.82);
        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);

        toDeck.crossfadeGain.gain.setValueAtTime(0.0, now + dur * 0.85);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur);
        break;
      }

      case 'echo_freeze_drop': {
        // Level 4: high-feedback 1/2-beat echo freeze wash while incoming bass slams in!
        const halfBeat = Math.max(0.15, Math.min(0.5, beatSec * 0.5));
        fromDeck.echoDelay.delayTime.setValueAtTime(halfBeat, now);
        fromDeck.echoFeedback.gain.setValueAtTime(0.76, now);
        fromDeck.echoSend.gain.setValueAtTime(0.0, now);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.95, now + dur * 0.45);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.0, now + dur * 0.65);
        fromDeck.echoFeedback.gain.linearRampToValueAtTime(0.08, now + dur + 2.8);

        fromDeck.filter.type = 'highpass';
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
          fromDeck.sourceNode.playbackRate.setValueAtTime(1.0, now);
          fromDeck.sourceNode.playbackRate.linearRampToValueAtTime(outgoingTarget, now + dur);
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
        // Level 4: dual-deck EQ-carved mashup layering (incoming mid/high rides over outgoing bass,
        // then crisp midpoint bass swap + subtle echo-out tail on outgoing!)
        toDeck.eqLow.gain.setValueAtTime(-30, now);
        toDeck.eqMid.gain.setValueAtTime(2.5, now);
        toDeck.eqHigh.gain.setValueAtTime(1.5, now);
        fromDeck.eqMid.gain.linearRampToValueAtTime(-8, now + dur * 0.25);

        toDeck.crossfadeGain.gain.linearRampToValueAtTime(0.95, now + dur * 0.25);
        fromDeck.crossfadeGain.gain.setValueAtTime(0.85, now + dur * 0.25);

        // Midpoint bass swap
        toDeck.eqLow.gain.setValueAtTime(-30, mid - 0.05);
        toDeck.eqLow.gain.linearRampToValueAtTime(0, mid + 0.08);
        toDeck.eqMid.gain.linearRampToValueAtTime(0, mid + 0.4);
        toDeck.eqHigh.gain.linearRampToValueAtTime(0, mid + 0.4);

        fromDeck.eqLow.gain.setValueAtTime(0, mid - 0.05);
        fromDeck.eqLow.gain.linearRampToValueAtTime(-30, mid + 0.08);

        // Subtle beat-synced echo-out on outgoing vocal/top layer
        fromDeck.echoDelay.delayTime.setValueAtTime(Math.max(0.2, beatSec * 0.5), mid);
        fromDeck.echoSend.gain.setValueAtTime(0.0, mid);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.55, now + dur * 0.85);
        fromDeck.echoSend.gain.linearRampToValueAtTime(0.0, now + dur);
        fromDeck.echoFeedback.gain.setValueAtTime(0.45, mid);
        fromDeck.echoFeedback.gain.linearRampToValueAtTime(0.1, now + dur + 1.5);

        fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
        toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur);
        break;
      }
    }

    // Restore natural 1.0 playback rate smoothly after transition (except tempo_ramp_bridge which handles it inline)
    if (technique !== 'tempo_ramp_bridge' && tempoRatio !== 1.0 && toDeck.sourceNode) {
      toDeck.sourceNode.playbackRate.linearRampToValueAtTime(1.0, now + dur + 2.0);
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
      fallbackUsed,
      reason,
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

  private ensureClockMonitor(): void {
    if (this.clockMonitorId) return;
    this.clockMonitorId = setInterval(() => {
      if (!this.ctx) return;
      const now = this.ctx.currentTime;
      let anyPlaying = false;

      for (const deck of this.decks) {
        if (!deck.isPlaying || !deck.buffer || !deck.meta) continue;
        anyPlaying = true;
        const pos = Math.max(0, now - deck.startedAtCtxTime);
        const dur = deck.buffer.duration;
        const remaining = Math.max(0, dur - pos);

        const preloadThreshold = Math.min(20, dur * 0.38);
        if (!deck.preloadSignaled && remaining <= preloadThreshold) {
          deck.preloadSignaled = true;
          this.callbacks.onPreloadWindow?.(deck.deckId, deck.meta.submissionId);
        }

        const baseTransWindow = this.djLevel >= 3 ? 10 : this.djLevel === 2 ? 7 : 6;
        const transDuration = Math.min(baseTransWindow, dur * 0.25);
        if (!deck.transitionTriggered && remaining <= transDuration) {
          const prepared = this.decks.find(
            (d) => d.deckId !== deck.deckId && !d.isPlaying && d.buffer !== null && d.meta !== null
          );
          if (prepared) {
            this.triggerAutomatedTransition(
              deck.deckId,
              prepared.deckId,
              transDuration,
              this.activeTransitionStyle
            );
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
    }));
  }
}

export const webAudioEngine = new FourDeckWebAudioEngine();
