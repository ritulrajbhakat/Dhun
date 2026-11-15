// WebAudio Real-Time DJ Synthesizer & Audio Engine for Browser Mode
// Provides real audio playback in browser tabs (Brave, Chrome, Edge, Firefox)

class BrowserAudioEngine {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private analyser: AnalyserNode | null = null;
  private currentKeyName: string = 'D Major';
  
  // Deck A nodes
  private deckAGain: GainNode | null = null;
  private deckALow: BiquadFilterNode | null = null;
  private deckAMid: BiquadFilterNode | null = null;
  private deckAHigh: BiquadFilterNode | null = null;
  private deckAFilter: BiquadFilterNode | null = null;
  private crossfaderAGain: GainNode | null = null;

  // Deck B nodes
  private deckBGain: GainNode | null = null;
  private deckBLow: BiquadFilterNode | null = null;
  private deckBMid: BiquadFilterNode | null = null;
  private deckBHigh: BiquadFilterNode | null = null;
  private deckBFilter: BiquadFilterNode | null = null;
  private crossfaderBGain: GainNode | null = null;

  // Sequencer state
  private isPlaying = false;
  private activeDeck = 0; // 0 = Deck A, 1 = Deck B
  private currentBpm = 128;
  private currentRootFreq = 146.83; // D3 for Gallan Goodiyaan (D Major)
  private stepIndex = 0;
  private nextStepTime = 0;
  private timerId: number | null = null;

  // Custom Audio Buffer Sources
  private deckABufferSource: AudioBufferSourceNode | null = null;
  private deckBBufferSource: AudioBufferSourceNode | null = null;

  public init() {
    if (this.ctx) return;
    const AudioCtx = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    if (!AudioCtx) return;
    
    this.ctx = new AudioCtx();

    // Master bus & Analyser
    this.masterGain = this.ctx.createGain();
    this.masterGain.gain.setValueAtTime(0.8, this.ctx.currentTime);

    this.analyser = this.ctx.createAnalyser();
    this.analyser.fftSize = 256;
    this.analyser.smoothingTimeConstant = 0.8;

    this.masterGain.connect(this.analyser);
    this.analyser.connect(this.ctx.destination);

    // Deck A Chain
    this.deckAGain = this.ctx.createGain();
    this.deckALow = this.ctx.createBiquadFilter();
    this.deckALow.type = 'lowshelf';
    this.deckALow.frequency.value = 250;

    this.deckAMid = this.ctx.createBiquadFilter();
    this.deckAMid.type = 'peaking';
    this.deckAMid.frequency.value = 1000;
    this.deckAMid.Q.value = 1.0;

    this.deckAHigh = this.ctx.createBiquadFilter();
    this.deckAHigh.type = 'highshelf';
    this.deckAHigh.frequency.value = 4000;

    this.deckAFilter = this.ctx.createBiquadFilter();
    this.deckAFilter.type = 'allpass'; // neutral by default

    this.crossfaderAGain = this.ctx.createGain();
    this.crossfaderAGain.gain.setValueAtTime(1.0, this.ctx.currentTime);

    // Connect Deck A
    this.deckAGain.connect(this.deckALow);
    this.deckALow.connect(this.deckAMid);
    this.deckAMid.connect(this.deckAHigh);
    this.deckAHigh.connect(this.deckAFilter);
    this.deckAFilter.connect(this.crossfaderAGain);
    this.crossfaderAGain.connect(this.masterGain);

    // Deck B Chain
    this.deckBGain = this.ctx.createGain();
    this.deckBLow = this.ctx.createBiquadFilter();
    this.deckBLow.type = 'lowshelf';
    this.deckBLow.frequency.value = 250;

    this.deckBMid = this.ctx.createBiquadFilter();
    this.deckBMid.type = 'peaking';
    this.deckBMid.frequency.value = 1000;
    this.deckBMid.Q.value = 1.0;

    this.deckBHigh = this.ctx.createBiquadFilter();
    this.deckBHigh.type = 'highshelf';
    this.deckBHigh.frequency.value = 4000;

    this.deckBFilter = this.ctx.createBiquadFilter();
    this.deckBFilter.type = 'allpass';

    this.crossfaderBGain = this.ctx.createGain();
    this.crossfaderBGain.gain.setValueAtTime(0.0, this.ctx.currentTime);

    // Connect Deck B
    this.deckBGain.connect(this.deckBLow);
    this.deckBLow.connect(this.deckBMid);
    this.deckBMid.connect(this.deckBHigh);
    this.deckBHigh.connect(this.deckBFilter);
    this.deckBFilter.connect(this.crossfaderBGain);
    this.crossfaderBGain.connect(this.masterGain);
  }

  public async ensureRunning() {
    this.init();
    if (this.ctx && this.ctx.state === 'suspended') {
      await this.ctx.resume();
    }
  }

  public startPlayback(bpm = 128, keyName = 'D Major') {
    this.ensureRunning();
    this.currentBpm = bpm;
    this.currentKeyName = keyName;
    this.currentRootFreq = this.parseKeyFrequency(keyName);
    this.isPlaying = true;

    if (!this.timerId && this.ctx) {
      this.nextStepTime = this.ctx.currentTime + 0.05;
      this.stepIndex = 0;
      this.scheduler();
    }
  }

  public getWaveformData(outputArray?: Uint8Array): Uint8Array {
    if (!this.analyser) {
      return outputArray || new Uint8Array(128).fill(128);
    }
    const arr = outputArray || new Uint8Array(this.analyser.frequencyBinCount);
    this.analyser.getByteTimeDomainData(arr as any);
    return arr;
  }

  public stopPlayback() {
    this.isPlaying = false;
    if (this.timerId) {
      window.clearTimeout(this.timerId);
      this.timerId = null;
    }
    if (this.deckABufferSource) {
      try { this.deckABufferSource.stop(); } catch {}
      this.deckABufferSource = null;
    }
    if (this.deckBBufferSource) {
      try { this.deckBBufferSource.stop(); } catch {}
      this.deckBBufferSource = null;
    }
  }

  public emergencyFade(durationMs = 300) {
    if (!this.ctx || !this.masterGain) return;
    const now = this.ctx.currentTime;
    this.masterGain.gain.cancelScheduledValues(now);
    this.masterGain.gain.setValueAtTime(this.masterGain.gain.value, now);
    this.masterGain.gain.linearRampToValueAtTime(0.0001, now + durationMs / 1000);
    setTimeout(() => {
      this.stopPlayback();
      if (this.masterGain && this.ctx) {
        this.masterGain.gain.setValueAtTime(0.8, this.ctx.currentTime);
      }
    }, durationMs + 50);
  }

  public setMasterVolume(vol: number) {
    if (!this.ctx || !this.masterGain) return;
    this.masterGain.gain.setTargetAtTime(Math.max(0, Math.min(1.5, vol)), this.ctx.currentTime, 0.02);
  }

  public setCrossfader(pos: number) { // -1.0 (Deck A) to +1.0 (Deck B)
    if (!this.ctx || !this.crossfaderAGain || !this.crossfaderBGain) return;
    const clamped = Math.max(-1, Math.min(1, pos));
    const normalized = (clamped + 1) / 2; // 0.0 to 1.0

    // Equal power curves
    const gainA = Math.cos(normalized * 0.5 * Math.PI);
    const gainB = Math.sin(normalized * 0.5 * Math.PI);

    this.crossfaderAGain.gain.setTargetAtTime(gainA, this.ctx.currentTime, 0.02);
    this.crossfaderBGain.gain.setTargetAtTime(gainB, this.ctx.currentTime, 0.02);
  }

  public setDeckEq(deckId: number, lowDb: number, midDb: number, highDb: number) {
    if (!this.ctx) return;
    const lowNode = deckId === 0 ? this.deckALow : this.deckBLow;
    const midNode = deckId === 0 ? this.deckAMid : this.deckBMid;
    const highNode = deckId === 0 ? this.deckAHigh : this.deckBHigh;

    if (lowNode) lowNode.gain.setTargetAtTime(lowDb, this.ctx.currentTime, 0.02);
    if (midNode) midNode.gain.setTargetAtTime(midDb, this.ctx.currentTime, 0.02);
    if (highNode) highNode.gain.setTargetAtTime(highDb, this.ctx.currentTime, 0.02);
  }

  public setDeckFilter(deckId: number, position: number) { // -1.0 (LPF) to +1.0 (HPF)
    if (!this.ctx) return;
    const filterNode = deckId === 0 ? this.deckAFilter : this.deckBFilter;
    if (!filterNode) return;

    if (Math.abs(position) < 0.05) {
      filterNode.type = 'allpass';
      filterNode.frequency.setTargetAtTime(1000, this.ctx.currentTime, 0.02);
    } else if (position > 0) {
      // HPF sweep from 20Hz up to 10kHz
      filterNode.type = 'highpass';
      const freq = 20 + Math.pow(position, 2) * 10000;
      filterNode.frequency.setTargetAtTime(freq, this.ctx.currentTime, 0.02);
      filterNode.Q.value = 2.0;
    } else {
      // LPF sweep from 20kHz down to 100Hz
      filterNode.type = 'lowpass';
      const freq = 20000 - Math.pow(-position, 2) * 19900;
      filterNode.frequency.setTargetAtTime(freq, this.ctx.currentTime, 0.02);
      filterNode.Q.value = 2.0;
    }
  }

  public setDeckGain(deckId: number, gain: number) {
    if (!this.ctx) return;
    const gainNode = deckId === 0 ? this.deckAGain : this.deckBGain;
    if (gainNode) gainNode.gain.setTargetAtTime(gain, this.ctx.currentTime, 0.02);
  }

  // Load and play a real local audio file in the browser
  public async loadUserAudioFile(deckId: number, file: File) {
    this.ensureRunning();
    if (!this.ctx) return;

    const arrayBuffer = await file.arrayBuffer();
    const audioBuffer = await this.ctx.decodeAudioData(arrayBuffer);

    const source = this.ctx.createBufferSource();
    source.buffer = audioBuffer;
    source.loop = true;

    const targetGain = deckId === 0 ? this.deckAGain : this.deckBGain;
    if (targetGain) {
      source.connect(targetGain);
    }

    source.start(0);

    if (deckId === 0) {
      if (this.deckABufferSource) {
        try { this.deckABufferSource.stop(); } catch {}
      }
      this.deckABufferSource = source;
    } else {
      if (this.deckBBufferSource) {
        try { this.deckBBufferSource.stop(); } catch {}
      }
      this.deckBBufferSource = source;
    }
  }

  // High-precision WebAudio Beat Scheduler
  private scheduler = () => {
    if (!this.isPlaying || !this.ctx) return;

    const scheduleAheadTime = 0.1; // 100ms lookahead
    const stepDuration = 60 / this.currentBpm / 4; // 16th note duration

    while (this.nextStepTime < this.ctx.currentTime + scheduleAheadTime) {
      this.playStep(this.stepIndex, this.nextStepTime);
      this.nextStepTime += stepDuration;
      this.stepIndex = (this.stepIndex + 1) % 16;
    }

    this.timerId = window.setTimeout(this.scheduler, 25);
  };

  private playStep(step: number, time: number) {
    if (!this.ctx) return;
    const targetGain = this.activeDeck === 0 ? this.deckAGain : this.deckBGain;
    if (!targetGain) return;

    // 1. Kick Drum: 4-on-the-floor (steps 0, 4, 8, 12)
    if (step % 4 === 0) {
      this.triggerKick(time, targetGain);
    }

    // 2. Off-beat Open/Closed Hi-hats (steps 2, 6, 10, 14 and 16th accents)
    if (step % 2 === 0) {
      const isOffbeat = step % 4 === 2;
      this.triggerHat(time, isOffbeat ? 0.08 : 0.03, targetGain);
    }

    // 3. Indian Dholak / Snare Tap (steps 4 and 12, plus syncopated step 14)
    if (step === 4 || step === 12 || step === 14) {
      this.triggerSnareDhol(time, step === 14 ? 0.6 : 1.0, targetGain);
    }

    // 4. Punchy Punjabi Synth Bassline (root notes and energetic octaves)
    if (step % 2 === 0 || step === 7 || step === 15) {
      const freqMultiplier = step === 0 || step === 8 ? 1.0 : (step === 4 || step === 12 ? 1.334 : 1.5);
      const freq = this.currentRootFreq * 0.5 * freqMultiplier;
      this.triggerBassNote(time, freq, targetGain);
    }

    // 5. Festive Chord Stab (on bar downbeat and phrase transitions)
    if (step === 0 || step === 6 || step === 10) {
      this.triggerChordStab(time, this.currentRootFreq, targetGain);
    }
  }

  private triggerKick(time: number, target: GainNode) {
    if (!this.ctx) return;
    const osc = this.ctx.createOscillator();
    const gain = this.ctx.createGain();

    osc.frequency.setValueAtTime(150, time);
    osc.frequency.exponentialRampToValueAtTime(38, time + 0.12);

    gain.gain.setValueAtTime(1.0, time);
    gain.gain.exponentialRampToValueAtTime(0.001, time + 0.22);

    osc.connect(gain);
    gain.connect(target);

    osc.start(time);
    osc.stop(time + 0.25);
  }

  private triggerHat(time: number, duration: number, target: GainNode) {
    if (!this.ctx) return;
    const bufferSize = this.ctx.sampleRate * duration;
    const buffer = this.ctx.createBuffer(1, bufferSize, this.ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < bufferSize; i++) {
      data[i] = (Math.random() * 2 - 1) * 0.6;
    }

    const noise = this.ctx.createBufferSource();
    noise.buffer = buffer;

    const filter = this.ctx.createBiquadFilter();
    filter.type = 'highpass';
    filter.frequency.setValueAtTime(7000, time);

    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(0.35, time);
    gain.gain.exponentialRampToValueAtTime(0.001, time + duration);

    noise.connect(filter);
    filter.connect(gain);
    gain.connect(target);

    noise.start(time);
    noise.stop(time + duration);
  }

  private triggerSnareDhol(time: number, volume: number, target: GainNode) {
    if (!this.ctx) return;
    
    // Skin 1: Resonant high rim-slap (tilli/dayan snap)
    const highOsc = this.ctx.createOscillator();
    const highGain = this.ctx.createGain();
    highOsc.type = 'triangle';
    highOsc.frequency.setValueAtTime(460, time);
    highOsc.frequency.exponentialRampToValueAtTime(180, time + 0.08);
    highGain.gain.setValueAtTime(0.55 * volume, time);
    highGain.gain.exponentialRampToValueAtTime(0.001, time + 0.10);
    highOsc.connect(highGain);
    highGain.connect(target);
    highOsc.start(time);
    highOsc.stop(time + 0.11);

    // Skin 2: Low dholak bass resonance (dagga boom)
    const lowOsc = this.ctx.createOscillator();
    const lowGain = this.ctx.createGain();
    lowOsc.type = 'sine';
    lowOsc.frequency.setValueAtTime(140, time);
    lowOsc.frequency.exponentialRampToValueAtTime(65, time + 0.12);
    lowGain.gain.setValueAtTime(0.75 * volume, time);
    lowGain.gain.exponentialRampToValueAtTime(0.001, time + 0.15);
    lowOsc.connect(lowGain);
    lowGain.connect(target);
    lowOsc.start(time);
    lowOsc.stop(time + 0.16);
  }

  private triggerBassNote(time: number, freq: number, target: GainNode) {
    if (!this.ctx) return;
    const osc = this.ctx.createOscillator();
    const filter = this.ctx.createBiquadFilter();
    const gain = this.ctx.createGain();

    osc.type = 'sawtooth';
    osc.frequency.setValueAtTime(freq, time);

    filter.type = 'lowpass';
    filter.frequency.setValueAtTime(750, time);
    filter.frequency.exponentialRampToValueAtTime(180, time + 0.16);

    gain.gain.setValueAtTime(0.55, time);
    gain.gain.exponentialRampToValueAtTime(0.001, time + 0.18);

    osc.connect(filter);
    filter.connect(gain);
    gain.connect(target);

    osc.start(time);
    osc.stop(time + 0.2);
  }

  private triggerChordStab(time: number, rootFreq: number, target: GainNode) {
    if (!this.ctx) return;
    const isMinor = this.currentKeyName.toLowerCase().includes('minor') || 
                    (this.currentKeyName.toLowerCase().includes('m') && !this.currentKeyName.toLowerCase().includes('major'));
    // Minor Triad: root, minor 3rd (1.189), 5th (1.498); Major Triad: root, major 3rd (1.259), 5th (1.498)
    const thirdMult = isMinor ? 1.1892 : 1.2599;
    const freqs = [rootFreq, rootFreq * thirdMult, rootFreq * 1.4983];
    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(0.25, time);
    gain.gain.exponentialRampToValueAtTime(0.001, time + 0.28);

    const filter = this.ctx.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.setValueAtTime(2600, time);
    filter.frequency.exponentialRampToValueAtTime(500, time + 0.28);

    freqs.forEach((f) => {
      if (!this.ctx) return;
      const osc = this.ctx.createOscillator();
      osc.type = 'sawtooth';
      osc.frequency.setValueAtTime(f, time);
      osc.connect(filter);
      osc.start(time);
      osc.stop(time + 0.30);
    });

    filter.connect(gain);
    gain.connect(target);
  }

  private parseKeyFrequency(keyName: string): number {
    if (keyName.includes('C#') || keyName.includes('Db')) return 138.59;
    if (keyName.includes('D')) return 146.83;
    if (keyName.includes('Eb') || keyName.includes('D#')) return 155.56;
    if (keyName.includes('E')) return 164.81;
    if (keyName.includes('F#')) return 185.00;
    if (keyName.includes('F')) return 174.61;
    if (keyName.includes('Ab') || keyName.includes('G#')) return 207.65;
    if (keyName.includes('A')) return 220.00;
    if (keyName.includes('Bb') || keyName.includes('A#')) return 233.08;
    if (keyName.includes('B')) return 246.94;
    if (keyName.includes('G')) return 196.00;
    return 130.81; // C3
  }
}

export const webAudioEngine = new BrowserAudioEngine();

