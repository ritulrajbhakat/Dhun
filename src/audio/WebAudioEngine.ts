// DHUN 4-Deck Web Audio Real-Time Mixing Engine
// Architected for 4-deck capability across all 4 DJ levels (Levels 1-2 expose 2 decks; Levels 3-4 expose 4 decks).
// Real-time audio processing is strictly isolated from React rendering, network I/O, and database operations.

export type TransitionStyle = 'bass_swap' | 'quick_cut' | 'phrase_sync' | 'filter_sweep';

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
  trackId: string | null;
  bpm: number;
  keyName: string;
  isPlaying: boolean;
  startedAtCtxTime: number;
  offsetSec: number;
}

export interface WebAudioDeckStatus {
  deckId: number;
  trackId: string | null;
  isLoaded: boolean;
  isPlaying: boolean;
  positionSec: number;
  durationSec: number;
  bpm: number;
  keyName: string;
}

class FourDeckWebAudioEngine {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private limiter: DynamicsCompressorNode | null = null;
  private analyser: AnalyserNode | null = null;
  private decks: WebAudioDeckChain[] = [];
  private crossfaderPos = -1.0; // -1.0 = Left group (Decks 0,2), +1.0 = Right group (Decks 1,3)

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
      // Default Deck 0 active on left side
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
        trackId: null,
        bpm: 124,
        keyName: 'C Major',
        isPlaying: false,
        startedAtCtxTime: 0,
        offsetSec: 0,
      };
    });
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

  /// Prepares an authorized cached audio buffer onto an idle deck without interrupting
  /// any currently playing deck.
  public async prepareAuthorizedDeckBuffer(
    deckId: number,
    audioBytes: ArrayBuffer,
    meta: { trackId: string; bpm?: number; keyName?: string }
  ): Promise<boolean> {
    await this.ensureRunning();
    if (!this.ctx || deckId < 0 || deckId >= this.decks.length) return false;
    const deck = this.decks[deckId];
    if (deck.isPlaying) {
      // Never interrupt a currently playing deck during preparation
      return false;
    }

    const decoded = await this.ctx.decodeAudioData(audioBytes.slice(0));
    deck.buffer = decoded;
    deck.trackId = meta.trackId;
    deck.bpm = meta.bpm || 124;
    deck.keyName = meta.keyName || 'C Major';
    deck.offsetSec = 0;
    return true;
  }

  public startDeck(deckId: number, playbackRate: number = 1.0): boolean {
    if (!this.ctx || deckId < 0 || deckId >= this.decks.length) return false;
    const deck = this.decks[deckId];
    if (!deck.buffer) return false;

    if (deck.sourceNode) {
      try {
        deck.sourceNode.stop();
        deck.sourceNode.disconnect();
      } catch {}
    }

    const src = this.ctx.createBufferSource();
    src.buffer = deck.buffer;
    src.playbackRate.setValueAtTime(playbackRate, this.ctx.currentTime);
    src.connect(deck.channelGain);
    src.onended = () => {
      if (deck.sourceNode === src) {
        deck.isPlaying = false;
        deck.sourceNode = null;
      }
    };

    src.start(0, deck.offsetSec);
    deck.sourceNode = src;
    deck.startedAtCtxTime = this.ctx.currentTime - deck.offsetSec;
    deck.isPlaying = true;
    return true;
  }

  public stopDeck(deckId: number): void {
    if (deckId < 0 || deckId >= this.decks.length) return;
    const deck = this.decks[deckId];
    if (deck.sourceNode) {
      try {
        deck.sourceNode.stop();
        deck.sourceNode.disconnect();
      } catch {}
      deck.sourceNode = null;
    }
    deck.isPlaying = false;
    deck.offsetSec = 0;
  }

  /// Sample-accurate Web Audio automated transition (equal-power / bass-swap / quick-cut / filter-sweep)
  /// executed completely inside the Web Audio native parameter timeline.
  public triggerAutomatedTransition(
    fromDeckId: number,
    toDeckId: number,
    durationSec: number,
    style: TransitionStyle
  ): boolean {
    if (!this.ctx) return false;
    const fromDeck = this.decks[fromDeckId];
    const toDeck = this.decks[toDeckId];
    if (!fromDeck || !toDeck || !toDeck.buffer) return false;

    const now = this.ctx.currentTime;
    const dur = style === 'quick_cut' ? 0.25 : Math.max(0.5, durationSec);
    const mid = now + dur * 0.5;

    if (!toDeck.isPlaying) {
      this.startDeck(toDeckId, 1.0);
    }

    // Crossfade gain ramps
    fromDeck.crossfadeGain.gain.cancelScheduledValues(now);
    toDeck.crossfadeGain.gain.cancelScheduledValues(now);
    fromDeck.crossfadeGain.gain.setValueAtTime(fromDeck.crossfadeGain.gain.value, now);
    toDeck.crossfadeGain.gain.setValueAtTime(0.0, now);

    fromDeck.crossfadeGain.gain.linearRampToValueAtTime(0.0, now + dur);
    toDeck.crossfadeGain.gain.linearRampToValueAtTime(1.0, now + dur);

    // Style-specific DSP automation
    if (style === 'bass_swap') {
      toDeck.eqLow.gain.setValueAtTime(-24, now);
      toDeck.eqLow.gain.setValueAtTime(-24, mid - 0.05);
      toDeck.eqLow.gain.linearRampToValueAtTime(0, mid + 0.1);
      fromDeck.eqLow.gain.setValueAtTime(0, now);
      fromDeck.eqLow.gain.setValueAtTime(0, mid - 0.05);
      fromDeck.eqLow.gain.linearRampToValueAtTime(-24, mid + 0.1);
    } else if (style === 'filter_sweep') {
      fromDeck.filter.type = 'highpass';
      fromDeck.filter.frequency.setValueAtTime(80, now);
      fromDeck.filter.frequency.exponentialRampToValueAtTime(4200, now + dur);
    }

    return true;
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

  public emergencyFade(durationMs: number = 300): void {
    if (!this.masterGain || !this.ctx) return;
    const t = this.ctx.currentTime;
    this.masterGain.gain.cancelScheduledValues(t);
    this.masterGain.gain.setValueAtTime(this.masterGain.gain.value, t);
    this.masterGain.gain.linearRampToValueAtTime(0.0001, t + durationMs / 1000);
    setTimeout(() => {
      this.decks.forEach((d) => this.stopDeck(d.deckId));
    }, durationMs + 40);
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
      trackId: d.trackId,
      isLoaded: d.buffer !== null,
      isPlaying: d.isPlaying,
      positionSec: d.isPlaying ? Math.max(0, now - d.startedAtCtxTime) : d.offsetSec,
      durationSec: d.buffer ? d.buffer.duration : 0,
      bpm: d.bpm,
      keyName: d.keyName,
    }));
  }
}

export const webAudioEngine = new FourDeckWebAudioEngine();
