// DHUN 4-Deck Web Audio Real-Time Mixing & Automated Advancement Engine
// Architected for 4-deck capability across all 4 DJ levels (Levels 1-2 expose 2 decks; Levels 3-4 expose 4 decks).
// Real-time audio processing is strictly isolated from React rendering, network I/O, and database operations.
// Progress and transitions are driven exclusively by actual AudioContext.currentTime & decoded AudioBuffers —
// never simulated with fabricated progress timers.

export type TransitionStyle = 'bass_swap' | 'quick_cut' | 'phrase_sync' | 'filter_sweep';

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
  crossfadeGain: GainNode;
  buffer: AudioBuffer | null;
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
    toSubmissionId: string
  ) => void;
  onTrackEnded?: (deckId: number, submissionId: string) => void;
  onSnapshot?: (snapshots: WebAudioDeckStatus[]) => void;
}

class FourDeckWebAudioEngine {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private limiter: DynamicsCompressorNode | null = null;
  private analyser: AnalyserNode | null = null;
  private decks: WebAudioDeckChain[] = [];
  private crossfaderPos = -1.0;
  private activeTransitionStyle: TransitionStyle = 'bass_swap';
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

    // Build all 4 independent deck DSP chains (Decks 0, 1, 2, 3)
    this.decks = [0, 1, 2, 3].map((deckId) => {
      const ctx = this.ctx!;
      const channelGain = ctx.createGain();
      channelGain.gain.setValueAtTime(1.0, ctx.currentTime);

      const eqLow = ctx.createBiquadFilter();
      eqLow.type = 'lowshelf';
      eqLow.frequency.value = 250;

      const eqMid = ctx.createBiquadFilter();
      eqMid.type = 'peaking';
      eqMid.frequency.value = 1000;
      eqMid.Q.value = 1.0;

      const eqHigh = ctx.createBiquadFilter();
      eqHigh.type = 'highshelf';
      eqHigh.frequency.value = 4000;

      const filter = ctx.createBiquadFilter();
      filter.type = 'allpass';

      const crossfadeGain = ctx.createGain();
      crossfadeGain.gain.setValueAtTime(deckId === 0 ? 1.0 : 0.0, ctx.currentTime);

      channelGain.connect(eqLow);
      eqLow.connect(eqMid);
      eqMid.connect(eqHigh);
      eqHigh.connect(filter);
      filter.connect(crossfadeGain);
      crossfadeGain.connect(this.masterGain!);

      return {
        deckId,
        channelGain,
        eqLow,
        eqMid,
        eqHigh,
        filter,
        crossfadeGain,
        buffer: null,
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

  public setTransitionStyle(style: TransitionStyle): void {
    this.activeTransitionStyle = style;
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

  public findIdleDeck(maxActiveDecks: number = 4): number | null {
    const limit = Math.max(2, Math.min(4, maxActiveDecks));
    // Prefer completely unloaded idle deck first
    for (let i = 0; i < limit; i++) {
      const d = this.decks[i];
      if (d && !d.isPlaying && !d.buffer) return i;
    }
    // Otherwise reuse an idle non-playing deck
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

  /**
   * Decodes actual host-cached authorized audio bytes onto an idle deck off the real-time
   * audio callback. Strictly refuses to overwrite an actively playing deck.
   */
  public async prepareAuthorizedDeckBuffer(
    deckId: number,
    audioBytes: ArrayBuffer,
    meta: DeckTrackMetadata
  ): Promise<boolean> {
    await this.ensureRunning();
    if (!this.ctx || deckId < 0 || deckId >= this.decks.length) return false;
    const deck = this.decks[deckId];
    if (deck.isPlaying) {
      return false;
    }

    // Decode outside the real-time callback; throws if audio is corrupt so caller recovers cleanly
    const decoded = await this.ctx.decodeAudioData(audioBytes.slice(0));
    if (!decoded || decoded.duration <= 0) {
      throw new Error('Decoded audio buffer has zero duration');
    }

    deck.buffer = decoded;
    deck.meta = meta;
    deck.offsetSec = 0;
    deck.preloadSignaled = false;
    deck.transitionTriggered = false;
    this.emitSnapshots();
    return true;
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
    // Reset EQ & filter cleanly for incoming deck start
    deck.eqLow.gain.setValueAtTime(0, now);
    deck.eqMid.gain.setValueAtTime(0, now);
    deck.eqHigh.gain.setValueAtTime(0, now);
    deck.filter.type = 'allpass';

    // If no other deck is playing, ensure this deck has full crossfade gain
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
    deck.isPlaying = false;
    deck.buffer = null;
    deck.meta = null;
    deck.offsetSec = 0;
    this.maybeStopClockMonitor();
    this.emitSnapshots();
  }

  /**
   * Sample-accurate Web Audio automated transition executed on AudioContext.currentTime.
   * Aligns tempo gently if both tracks have valid BPM metadata within ±6%.
   */
  public triggerAutomatedTransition(
    fromDeckId: number,
    toDeckId: number,
    durationSec: number,
    style: TransitionStyle
  ): boolean {
    if (!this.ctx) return false;
    const fromDeck = this.decks[fromDeckId];
    const toDeck = this.decks[toDeckId];
    if (!fromDeck || !toDeck || !toDeck.buffer || !toDeck.meta) return false;

    const now = this.ctx.currentTime;
    const dur = style === 'quick_cut' ? 0.35 : Math.max(0.8, Math.min(durationSec, toDeck.buffer.duration * 0.35));
    const mid = now + dur * 0.5;

    // Gentle BPM tempo matching if metadata supports it (within ±6%)
    let playbackRate = 1.0;
    if (fromDeck.meta?.bpm && toDeck.meta?.bpm && toDeck.meta.bpm > 0) {
      const ratio = fromDeck.meta.bpm / toDeck.meta.bpm;
      if (ratio >= 0.94 && ratio <= 1.06) {
        playbackRate = ratio;
      }
    }

    if (!toDeck.isPlaying) {
      this.startDeck(toDeckId, playbackRate);
    }

    fromDeck.transitionTriggered = true;

    fromDeck.crossfadeGain.gain.cancelScheduledValues(now);
    toDeck.crossfadeGain.gain.cancelScheduledValues(now);
    fromDeck.crossfadeGain.gain.setValueAtTime(fromDeck.crossfadeGain.gain.value, now);
    toDeck.crossfadeGain.gain.setValueAtTime(0.0, now);

    fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
    toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur);

    if (style === 'bass_swap') {
      toDeck.eqLow.gain.setValueAtTime(-24, now);
      toDeck.eqLow.gain.setValueAtTime(-24, mid - 0.05);
      toDeck.eqLow.gain.linearRampToValueAtTime(0, mid + 0.12);
      fromDeck.eqLow.gain.setValueAtTime(0, now);
      fromDeck.eqLow.gain.setValueAtTime(0, mid - 0.05);
      fromDeck.eqLow.gain.linearRampToValueAtTime(-24, mid + 0.12);
    } else if (style === 'filter_sweep') {
      fromDeck.filter.type = 'highpass';
      fromDeck.filter.frequency.setValueAtTime(80, now);
      fromDeck.filter.frequency.exponentialRampToValueAtTime(4200, now + dur);
    }

    // Gradually restore incoming deck tempo back to natural 1.0 after transition
    if (playbackRate !== 1.0 && toDeck.sourceNode) {
      toDeck.sourceNode.playbackRate.linearRampToValueAtTime(1.0, now + dur + 2.0);
    }

    if (fromDeck.meta) {
      this.callbacks.onTransitionStarted?.(
        fromDeckId,
        fromDeck.meta.submissionId,
        toDeckId,
        toDeck.meta.submissionId
      );
    }

    return true;
  }

  /**
   * Monitors actual AudioContext.currentTime exclusively while real decoded audio is playing.
   * Triggers advance preloading and automated transitions without React render timers.
   */
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

        // Preload window: 18s before end (or 35% remaining on short clips)
        const preloadThreshold = Math.min(18, dur * 0.35);
        if (!deck.preloadSignaled && remaining <= preloadThreshold) {
          deck.preloadSignaled = true;
          this.callbacks.onPreloadWindow?.(deck.deckId, deck.meta.submissionId);
        }

        // Automated transition window: 6s before end (or 20% remaining on short clips)
        const transDuration = Math.min(6, dur * 0.2);
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
    if (!this.analyser) {
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
