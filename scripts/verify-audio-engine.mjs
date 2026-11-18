// Deterministic Audio-Graph, DSP Technique, Gain-Staging, Feedback-Stability &
// Musical-Section Performance Verification Harness for DHUN WebAudioEngine.
//
// Runs directly against the real `FourDeckWebAudioEngine`, `resolveTechniqueForLevel`,
// and `planMusicalPerformanceWindow` compiled from `src/audio/WebAudioEngine.ts`.

import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..');
const esbuild = require(path.join(rootDir, 'node_modules', 'esbuild'));

const built = esbuild.buildSync({
  entryPoints: [path.join(rootDir, 'src', 'audio', 'WebAudioEngine.ts')],
  bundle: true,
  platform: 'node',
  format: 'cjs',
  write: false,
});

const mod = { exports: {} };
new Function('exports', 'require', 'module', built.outputFiles[0].text)(mod.exports, require, mod);

const {
  FourDeckWebAudioEngine,
  resolveTechniqueForLevel,
  planMusicalPerformanceWindow,
} = mod.exports;

// ============================================================================
// Sample-Accurate Instrumented Web Audio Graph Harness
// Captures every AudioParam event, validates anchoring/stability/headroom,
// and verifies real scheduled automation across all 4 DJ levels & 14 techniques.
// ============================================================================
class InstrumentedAudioParam {
  constructor(name, initial) {
    this.name = name;
    this.value = initial;
    this.events = [];
  }
  cancelScheduledValues(t) {
    this.events.push({ op: 'cancel', t });
    return this;
  }
  setValueAtTime(v, t) {
    this.value = v;
    this.events.push({ op: 'set', v, t });
    return this;
  }
  linearRampToValueAtTime(v, t) {
    this.events.push({ op: 'linear', v, t });
    this.value = v;
    return this;
  }
  exponentialRampToValueAtTime(v, t) {
    assert(v > 0, `${this.name}: exponentialRamp target must be > 0 (got ${v})`);
    this.events.push({ op: 'exp', v, t });
    this.value = v;
    return this;
  }
  setTargetAtTime(v, t, tc) {
    this.events.push({ op: 'target', v, t, tc });
    this.value = v;
    return this;
  }
  valueAt(time) {
    let cur = this.events.length ? this.events[0].v ?? this.value : this.value;
    let lastT = 0;
    for (const ev of this.events) {
      if (ev.op === 'cancel') continue;
      if (ev.op === 'set') {
        if (ev.t <= time) {
          cur = ev.v;
          lastT = ev.t;
        }
      } else if (ev.op === 'linear' || ev.op === 'exp') {
        if (ev.t <= time) {
          cur = ev.v;
          lastT = ev.t;
        } else if (ev.t > lastT && time > lastT) {
          const frac = (time - lastT) / (ev.t - lastT);
          cur = cur + (ev.v - cur) * frac;
          return cur;
        }
      }
    }
    return cur;
  }
}

class InstrumentedNode {
  constructor(kind) {
    this.kind = kind;
    this.connections = [];
  }
  connect(dest) {
    this.connections.push(dest);
    return dest;
  }
  disconnect() {
    this.connections = [];
  }
}

class InstrumentedAudioContext {
  constructor(sampleRate = 44100) {
    this.sampleRate = sampleRate;
    this.currentTime = 10.0; // Non-zero start time explicitly catches missing setValueAtTime(val, now) anchor bugs!
    this.state = 'running';
    this.destination = new InstrumentedNode('destination');
    this.createdSources = [];
    this.nodes = [];
  }
  createGain() {
    const n = new InstrumentedNode('gain');
    n.gain = new InstrumentedAudioParam('gain', 1.0);
    this.nodes.push(n);
    return n;
  }
  createBiquadFilter() {
    const n = new InstrumentedNode('biquad');
    n.type = 'allpass';
    n.frequency = new InstrumentedAudioParam('frequency', 350);
    n.Q = new InstrumentedAudioParam('Q', 1.0);
    n.gain = new InstrumentedAudioParam('gain', 0.0);
    this.nodes.push(n);
    return n;
  }
  createDelay(maxDelayTime = 1.0) {
    const n = new InstrumentedNode('delay');
    n.maxDelayTime = maxDelayTime;
    n.delayTime = new InstrumentedAudioParam('delayTime', 0.0);
    this.nodes.push(n);
    return n;
  }
  createDynamicsCompressor() {
    const n = new InstrumentedNode('compressor');
    n.threshold = new InstrumentedAudioParam('threshold', -24);
    n.knee = new InstrumentedAudioParam('knee', 30);
    n.ratio = new InstrumentedAudioParam('ratio', 12);
    n.attack = new InstrumentedAudioParam('attack', 0.003);
    n.release = new InstrumentedAudioParam('release', 0.25);
    this.nodes.push(n);
    return n;
  }
  createAnalyser() {
    const n = new InstrumentedNode('analyser');
    n.fftSize = 256;
    n.smoothingTimeConstant = 0.8;
    n.getByteTimeDomainData = (arr) => arr.fill(128);
    this.nodes.push(n);
    return n;
  }
  createBufferSource() {
    const n = new InstrumentedNode('bufferSource');
    n.buffer = null;
    n.playbackRate = new InstrumentedAudioParam('playbackRate', 1.0);
    n.onended = null;
    n.startedArgs = null;
    n.stoppedAt = null;
    n.start = (when = 0, offset = 0) => {
      n.startedArgs = { when, offset };
    };
    n.stop = (when = 0) => {
      n.stoppedAt = when;
    };
    this.createdSources.push(n);
    return n;
  }
}

function makeFakeBuffer(durationSec, rmsAmp = 0.2, sampleRate = 44100) {
  const len = Math.floor(durationSec * sampleRate);
  const data = new Float32Array(len);
  const peak = rmsAmp * Math.SQRT2;
  for (let i = 0; i < len; i++) {
    data[i] = peak * Math.sin((2 * Math.PI * 110 * i) / sampleRate);
  }
  return {
    duration: durationSec,
    sampleRate,
    length: len,
    numberOfChannels: 2,
    getChannelData: () => data,
  };
}

// ============================================================================
// TEST 1: All 14 DSP Techniques Execute Cleanly on the Audio Graph + Stability
// ============================================================================
const ALL_14_TECHNIQUES = [
  { level: 1, id: 'safe_quick_cut', check: (d0, d1) => d0.crossfadeGain.gain.events.some((e) => e.op === 'linear' && e.v === 0) && d1.crossfadeGain.gain.events.some((e) => e.op === 'linear' && e.v === 1) },
  { level: 1, id: 'eq3_handover', check: (d0, d1) => d1.eqHigh.gain.events.some((e) => e.v === -16) && d1.eqMid.gain.events.some((e) => e.v === -14) && d1.eqLow.gain.events.some((e) => e.v === -24) },
  { level: 1, id: 'phrase_long_blend', check: (d0, d1) => d0.crossfadeGain.gain.events.filter((e) => e.op === 'linear').length >= 16 },
  { level: 2, id: 'timed_bass_swap', check: (d0, d1) => d1.eqLow.gain.events.some((e) => e.v === -28) && d0.eqLow.gain.events.some((e) => e.v === -28) },
  { level: 2, id: 'filter_sweep_drop', check: (d0) => d0.filter.type === 'highpass' && d0.filter.frequency.events.some((e) => e.op === 'exp' && e.v >= 5000) },
  { level: 3, id: 'echo_out_blend', check: (d0) => d0.echoSend.gain.events.some((e) => e.v >= 0.75) && d0.echoFeedback.gain.events.at(-1).v === 0 },
  { level: 3, id: 'beat_roll_buildup', check: (d0) => d0.gateGain.gain.events.filter((e) => e.op === 'linear').length >= 8 && d0.filter.type === 'highpass' },
  { level: 3, id: 'filtered_buildup_drop', check: (d0) => d0.filter.type === 'highpass' && d0.filter.Q.events.some((e) => e.v >= 6.0) },
  { level: 3, id: 'four_deck_layer', check: (d0, d1, d2) => d2 && d2.isPlaying && d2.filter.type === 'highpass' && d2.crossfadeGain.gain.events.some((e) => e.v > 0.25 && e.v <= 0.4) },
  { level: 4, id: 'stutter_gate_drop', check: (d0) => d0.gateGain.gain.events.filter((e) => e.op === 'linear' && e.v <= 0.05).length >= 8 },
  { level: 4, id: 'backspin_cut', check: (d0) => d0.sourceNode.playbackRate.events.some((e) => e.op === 'exp' && e.v <= 0.25) },
  { level: 4, id: 'echo_freeze_drop', check: (d0) => d0.echoFeedback.gain.events.some((e) => e.v >= 0.7 && e.v <= 0.78) && d0.echoFeedback.gain.events.at(-1).v === 0 },
  { level: 4, id: 'tempo_ramp_bridge', check: (d0, d1) => d0.sourceNode.playbackRate.events.some((e) => e.op === 'linear') && d1.sourceNode.playbackRate.events.some((e) => e.op === 'linear' && e.v === 1.0) },
  { level: 4, id: 'creative_mashup_layer', check: (d0, d1, d2) => d1.eqLow.gain.events.some((e) => e.v === -30) && d0.echoSend.gain.events.some((e) => e.v >= 0.5) && d2 && d2.isPlaying },
];

for (const tc of ALL_14_TECHNIQUES) {
  const ctx = new InstrumentedAudioContext();
  const engine = new FourDeckWebAudioEngine();
  engine.init(ctx);
  engine.setDjLevel(tc.level);

  const logs = [];
  engine.setCallbacks({ onTechniqueExecuted: (ex) => logs.push(ex) });

  // Quiet outgoing buffer vs loud incoming buffer to also verify L2+ RMS gain matching
  const bufA = makeFakeBuffer(60, 0.12);
  const bufB = makeFakeBuffer(60, 0.32);
  const bufC = makeFakeBuffer(60, 0.20);

  engine.loadDecodedDeckBuffer(0, bufA, { submissionId: 'sA', cacheId: 'cA', title: 'Track A', artist: 'Art A', bpm: 124, camelotKey: '8A' });
  engine.loadDecodedDeckBuffer(1, bufB, { submissionId: 'sB', cacheId: 'cB', title: 'Track B', artist: 'Art B', bpm: 126, camelotKey: '9A' });
  if (tc.level >= 3) {
    engine.loadDecodedDeckBuffer(2, bufC, { submissionId: 'sC', cacheId: 'cC', title: 'Groove C', artist: 'Art C', bpm: 125, camelotKey: '8A' });
  }

  assert.equal(engine.startDeck(0, 1.0), true);
  const ok = engine.triggerAutomatedTransition(0, 1, 8.0, tc.id);
  assert.equal(ok, true, `${tc.id} failed to execute`);
  assert.equal(logs.length, 1);
  assert.equal(logs[0].techniqueId, tc.id, `Expected ${tc.id}, got ${logs[0].techniqueId}`);

  // Overlapping transition guard: second immediate call on Deck 0 MUST be rejected!
  assert.equal(engine.triggerAutomatedTransition(0, 1, 8.0, tc.id), false, `${tc.id}: overlapping transition was not guarded`);

  // Verify real DSP nodes received expected automation
  const decks = engine.decks;
  assert.equal(tc.check(decks[0], decks[1], decks[2]), true, `DSP assertion failed for ${tc.id}`);

  // Verify incoming deck started atomically at crossfadeGain = 0.0 (zero click/pop burst!)
  const firstSetTo = decks[1].crossfadeGain.gain.events.find((e) => e.op === 'set');
  assert.equal(firstSetTo.v, 0.0, `${tc.id}: incoming deck did not start at crossfadeGain 0.0`);

  // Verify echo feedback is strictly stable (<= 0.78) on ALL decks!
  for (const d of decks) {
    for (const ev of d.echoFeedback.gain.events) {
      if (ev.v !== undefined) {
        assert(ev.v >= 0 && ev.v <= 0.78, `${tc.id}: unstable echoFeedback ${ev.v}`);
      }
    }
  }

  // Verify RMS gain matching behavior: L1 keeps unity 1.0; L2+ normalizes loud track B downward
  if (tc.level === 1) {
    assert.equal(logs[0].gainMatchDb, 0, 'Level 1 must not apply auto gain match');
  } else {
    assert(logs[0].gainMatchDb < -1.0, `Level ${tc.level} should attenuate loud Track B (got ${logs[0].gainMatchDb} dB)`);
  }

  // Verify every ramp on outgoing EQ/crossfade was anchored at >= ctx.currentTime (no t=0 ramp bugs!)
  for (const param of [decks[0].crossfadeGain.gain, decks[0].eqLow.gain, decks[0].eqMid.gain, decks[0].eqHigh.gain]) {
    const firstRampIdx = param.events.findIndex((e) => e.op === 'linear' || e.op === 'exp');
    if (firstRampIdx !== -1) {
      const priorSet = param.events.slice(0, firstRampIdx).reverse().find((e) => e.op === 'set');
      assert(priorSet && priorSet.t >= ctx.currentTime, `${tc.id}: unanchored ramp on ${param.name}`);
    }
  }
}

// ============================================================================
// TEST 2: Strict Inheritance + Distinct Behavior Across L1, L2, L3, L4
// ============================================================================
const fromMeta = { submissionId: 's1', cacheId: 'c1', title: 'A', artist: 'X', bpm: 124, camelotKey: '8A' };
const toMeta = { submissionId: 's2', cacheId: 'c2', title: 'B', artist: 'Y', bpm: 126, camelotKey: '9A' };
const levelChoices = [1, 2, 3, 4].map((lvl) => resolveTechniqueForLevel(lvl, fromMeta, toMeta, 'creative_mashup_layer'));
assert.equal(new Set(levelChoices.map((c) => c.technique)).size, 4, 'L1-L4 must resolve to 4 distinct techniques for same pair');

// Level 4 strictly inherits L1-L3 techniques
for (const inherited of ['safe_quick_cut', 'eq3_handover', 'phrase_long_blend', 'timed_bass_swap', 'echo_out_blend', 'beat_roll_buildup']) {
  const r = resolveTechniqueForLevel(4, fromMeta, toMeta, inherited);
  assert.equal(r.technique, inherited, `Level 4 failed to inherit ${inherited}`);
  assert.equal(r.fallbackUsed, false);
}

// Level 2 half/double-time sync (128 -> 64 BPM => 1.0 ratio)
const hd = resolveTechniqueForLevel(2, { ...fromMeta, bpm: 128 }, { ...toMeta, bpm: 64 }, 'timed_bass_swap');
assert(Math.abs(hd.tempoRatio - 1.0) < 0.001, `Expected half/double-time ratio 1.0, got ${hd.tempoRatio}`);

// Missing BPM triggers safe fallback (never fakes beat alignment)
const missingL1 = resolveTechniqueForLevel(1, { ...fromMeta, bpm: null }, { ...toMeta, bpm: null }, 'timed_bass_swap');
assert.equal(missingL1.fallbackUsed, true);
assert.equal(missingL1.technique, 'safe_quick_cut');
const missingL4 = resolveTechniqueForLevel(4, { ...fromMeta, bpm: null }, { ...toMeta, bpm: null }, 'creative_mashup_layer');
assert.equal(missingL4.fallbackUsed, true);
assert.equal(missingL4.technique, 'echo_out_blend');

// ============================================================================
// TEST 3: Musical Section-Based Performance Planner & Anti-Repetition
// ============================================================================
const richSections = [
  { section_type: 'intro', start_ms: 0, end_ms: 16000, energy_level: 0.30, bass_energy: 0.15, confidence: 0.85 },
  { section_type: 'groove', start_ms: 16000, end_ms: 48000, energy_level: 0.72, bass_energy: 0.68, confidence: 0.82 },
  { section_type: 'breakdown', start_ms: 48000, end_ms: 64000, energy_level: 0.35, bass_energy: 0.18, confidence: 0.84 },
  { section_type: 'build', start_ms: 64000, end_ms: 80000, energy_level: 0.65, bass_energy: 0.45, confidence: 0.80 },
  { section_type: 'drop', start_ms: 80000, end_ms: 112000, energy_level: 0.94, bass_energy: 0.92, confidence: 0.91 },
  { section_type: 'outro', start_ms: 112000, end_ms: 128000, energy_level: 0.28, bass_energy: 0.14, confidence: 0.86 },
];
const metaWithSections = { ...fromMeta, bpm: 120, sections: richSections };
const played = new Set();

// First selection on L3 at high energy (0.85) should skip intro/outro and pick high-impact moment aligned to phrase boundaries
const win1 = planMusicalPerformanceWindow(metaWithSections, 128, 3, 0.85, played);
assert.equal(win1.fallbackUsed, false);
assert(win1.cueInSec >= 16, `Expected phrase-aligned skip past intro, got cueInSec=${win1.cueInSec}`);
assert(win1.cueOutSec <= 112, `Expected phrase-aligned exit before outro, got cueOutSec=${win1.cueOutSec}`);
assert.equal(win1.cueInSec % 16, 0, 'cueInSec must snap strictly to analyzed phrase boundary');
played.add(win1.sectionSignature);

// Second selection of same track avoids repeating the exact same section!
const win2 = planMusicalPerformanceWindow(metaWithSections, 128, 3, 0.85, played);
assert.notEqual(win2.sectionSignature, win1.sectionSignature, 'Section planner must avoid repeating already-played section');

// Intra-track breakdown skip: when starting at groove (16s) on L3 with high energy (0.85), should plan jump over breakdown (48s -> 64s)!
const playedDropAndBuild = new Set(['s1:80000-112000', 's1:64000-80000']);
const winJump = planMusicalPerformanceWindow(metaWithSections, 128, 3, 0.85, playedDropAndBuild);
assert(winJump.intraTrackJump !== null, 'Expected L3 intra-track jump skipping breakdown into build/drop');
assert.equal(winJump.intraTrackJump.fromSec, 48);
assert.equal(winJump.intraTrackJump.toSec, 64);

// Low-confidence / flat sections MUST fall back honestly to full track
const flatSections = [
  { section_type: 'intro', start_ms: 0, end_ms: 16000, energy_level: 0.5, confidence: 0.4 },
  { section_type: 'phrase', start_ms: 16000, end_ms: 32000, energy_level: 0.5, confidence: 0.35 },
  { section_type: 'outro', start_ms: 32000, end_ms: 48000, energy_level: 0.5, confidence: 0.4 },
];
const winFallback = planMusicalPerformanceWindow({ ...fromMeta, sections: flatSections }, 48, 4, 0.85, new Set());
assert.equal(winFallback.fallbackUsed, true);
assert.equal(winFallback.cueInSec, 0);
assert.equal(winFallback.cueOutSec, 48);

console.log('PASSED: All 14 Web Audio DSP techniques, L1-L4 inheritance, headroom/feedback stability, 3rd-deck layering, and musical section performance checks.');
