// Comprehensive Deterministic Verification of DHUN's DJ Transition Decision Engine,
// Musical Timeline Scheduler, Timing Maps, Indian Mood Intelligence, Prep-Delay Replanning,
// Failed Incoming Deck Cancellation, and Actual Web Audio DSP Graph Execution.

import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..');
const esbuild = require(path.join(rootDir, 'node_modules', 'esbuild'));

function loadTs(relPath) {
  const out = esbuild.buildSync({
    entryPoints: [path.join(rootDir, relPath)],
    bundle: true,
    platform: 'node',
    format: 'cjs',
    write: false,
  });
  const mod = { exports: {} };
  new Function('exports', 'require', 'module', out.outputFiles[0].text)(mod.exports, require, mod);
  return mod.exports;
}

const {
  planDjTransition,
  createTempoPolicyForLevel,
  TransitionTimelineScheduler,
} = loadTs('src/audio/TransitionDecisionEngine.ts');
const { FourDeckWebAudioEngine } = loadTs('src/audio/WebAudioEngine.ts');

function makeTiming(bpm, conf = 0.9, beatsPerBar = 4, meterLabel = '4/4', isVar = false, tempoMap = null) {
  return {
    average_bpm: bpm,
    beatgrid_confidence: conf,
    downbeat_offset_ms: 0,
    is_variable_tempo: isVar,
    tempo_drift_bpm: isVar ? 10.0 : 0.3,
    meter: { beats_per_bar: beatsPerBar, meter_label: meterLabel, confidence: conf },
    tempo_map: tempoMap || [{ start_ms: 0, end_ms: 180000, bpm, confidence: conf }],
    beat_timestamps_ms: Array.from({ length: 360 }, (_, i) => Math.round((i * 60000) / bpm)),
    downbeat_timestamps_ms: Array.from({ length: 90 }, (_, i) =>
      Math.round((i * beatsPerBar * 60000) / bpm)
    ),
  };
}

function makeSec(type, sMs, eMs, energy, vocal) {
  return {
    section_type: type,
    start_ms: sMs,
    end_ms: eMs,
    energy_level: energy,
    bass_energy: energy * 0.85,
    vocal_activity: vocal,
    confidence: 0.88,
    bar_index: 0,
  };
}

// 1. Same-BPM Desi Club instrumental -> 64-beat phrase-aligned blend
const clubOut = {
  trackId: 't1',
  deckId: 0,
  title: 'Club A',
  artist: 'DJ A',
  durationSec: 160,
  currentPosSec: 50,
  bpm: 128,
  camelotKey: '8A',
  overallEnergy: 0.84,
  genreTags: ['Tech House'],
  sections: [
    makeSec('groove', 0, 96000, 0.84, 0.12),
    makeSec('outro', 96000, 160000, 0.60, 0.08),
  ],
  timingProfile: makeTiming(128, 0.92, 4, '4/4', false),
};
const clubIn = {
  trackId: 't2',
  deckId: 1,
  title: 'Club B',
  artist: 'DJ B',
  durationSec: 160,
  currentPosSec: 0,
  bpm: 128,
  camelotKey: '8A',
  overallEnergy: 0.86,
  genreTags: ['Tech House'],
  sections: [
    makeSec('intro', 0, 48000, 0.65, 0.08),
    makeSec('drop', 48000, 128000, 0.94, 0.15),
  ],
  timingProfile: makeTiming(128, 0.92, 4, '4/4', false),
};
const pSame = planDjTransition(clubOut, clubIn, {
  djLevel: 3,
  moodId: 'desi_club',
  targetEnergy: 0.86,
  recentTechniques: [],
});
assert.equal(pSame.status, 'planned');
assert.equal(pSame.bpm_relationship, 'direct');
assert.equal(pSame.duration_beats, 64);

// 2. Half-time (128 -> 64) & Double-time (70 -> 140)
const pHalf = planDjTransition(
  clubOut,
  { ...clubIn, bpm: 64, timingProfile: makeTiming(64, 0.9, 4, '4/4') },
  { djLevel: 2, moodId: 'house_party', targetEnergy: 0.8, recentTechniques: [] }
);
assert.equal(pHalf.bpm_relationship, 'half_time');

const pDouble = planDjTransition(
  { ...clubOut, bpm: 70, timingProfile: makeTiming(70, 0.9, 4, '4/4') },
  { ...clubIn, bpm: 140, timingProfile: makeTiming(140, 0.9, 4, '4/4') },
  { djLevel: 2, moodId: 'house_party', targetEnergy: 0.8, recentTechniques: [] }
);
assert.equal(pDouble.bpm_relationship, 'double_time');

// 3. Gradual tempo ramp (112 -> 128 BPM) on Level 4
const pRamp = planDjTransition(
  { ...clubOut, bpm: 112, timingProfile: makeTiming(112, 0.9, 4, '4/4') },
  clubIn,
  { djLevel: 4, moodId: 'house_party', targetEnergy: 0.85, recentTechniques: [] }
);
assert.equal(pRamp.bpm_relationship, 'gradual_tempo_ramp');
assert.equal(pRamp.selected_technique, 'tempo_ramp_bridge');

// 4. Changing BPM within track (tempo map local BPM at outro is 138 BPM while intro was 105 BPM!)
const varTempoMap = [
  { start_ms: 0, end_ms: 80000, bpm: 105, confidence: 0.88 },
  { start_ms: 80000, end_ms: 160000, bpm: 138, confidence: 0.88 },
];
const pChanging = planDjTransition(
  { ...clubOut, bpm: 120, timingProfile: makeTiming(120, 0.88, 4, '4/4', true, varTempoMap) },
  { ...clubIn, bpm: 138, timingProfile: makeTiming(138, 0.88, 4, '4/4', false) },
  { djLevel: 2, moodId: 'garba', targetEnergy: 0.85, recentTechniques: [] }
);
assert.equal(pChanging.current_bpm, 138, 'Must use local tempo map BPM at exit boundary (138), not average (120)');
assert.equal(pChanging.bpm_relationship, 'direct', 'Local 138->138 BPM aligns cleanly even on accelerating track');

// 5. Low-confidence beatgrid -> no_beatmatch & safe cut
const pLowConf = planDjTransition(
  clubOut,
  { ...clubIn, timingProfile: makeTiming(128, 0.28, 4, '4/4') },
  { djLevel: 2, moodId: 'house_party', targetEnergy: 0.8, recentTechniques: [] }
);
assert.equal(pLowConf.bpm_relationship, 'no_beatmatch');
assert.equal(pLowConf.selected_technique, 'safe_quick_cut');
assert(pLowConf.rejected_alternatives.length > 0);

// 6. Different meters (6/8 Garba vs 4/4 Club) -> refuses beatmatch
const pMeter = planDjTransition(
  clubOut,
  { ...clubIn, timingProfile: makeTiming(128, 0.9, 6, '6/8') },
  { djLevel: 3, moodId: 'garba', targetEnergy: 0.8, recentTechniques: [] }
);
assert.equal(pMeter.meter_compatible, false);
assert.equal(pMeter.bpm_relationship, 'no_beatmatch');

// 7. Vocal overlap conflict -> avoid_vocal_clash + rejects phrase_long_blend
const pVocal = planDjTransition(
  { ...clubOut, genreTags: ['Bollywood'], sections: [makeSec('chorus', 40000, 120000, 0.85, 0.84)] },
  { ...clubIn, genreTags: ['Bollywood'], sections: [makeSec('chorus', 0, 90000, 0.85, 0.86)] },
  { djLevel: 2, moodId: 'shaadi', targetEnergy: 0.8, recentTechniques: [] }
);
assert.equal(pVocal.vocal_overlap_policy, 'avoid_vocal_clash');
assert(pVocal.duration_beats <= 16);
assert(pVocal.rejected_alternatives.some((r) => r.technique === 'phrase_long_blend'));

// 8. Intro-over-chorus pairing
const pIntroChorus = planDjTransition(
  { ...clubOut, sections: [makeSec('chorus', 40000, 120000, 0.86, 0.72)] },
  {
    ...clubIn,
    sections: [
      makeSec('intro', 0, 32000, 0.55, 0.10),
      makeSec('chorus', 32000, 100000, 0.88, 0.82),
    ],
  },
  { djLevel: 2, moodId: 'shaadi', targetEnergy: 0.82, recentTechniques: [] }
);
assert.equal(pIntroChorus.incoming_section_type, 'intro');
assert.equal(pIntroChorus.vocal_overlap_policy, 'instrumental_under_vocal');

// 9. Breakdown-to-drop + Build-up payoff protection
const pBuildDrop = planDjTransition(
  {
    ...clubOut,
    currentPosSec: 52,
    sections: [
      makeSec('build', 48000, 64000, 0.72, 0.15),
      makeSec('drop', 64000, 96000, 0.96, 0.15),
      makeSec('breakdown', 96000, 128000, 0.38, 0.10),
    ],
  },
  clubIn,
  { djLevel: 3, moodId: 'desi_club', targetEnergy: 0.85, recentTechniques: [] }
);
assert(pBuildDrop.outgoing_mix_out_sec >= 90, 'Must never cut an active build-up before its drop payoff');
assert.equal(pBuildDrop.outgoing_section_type, 'breakdown');
assert.equal(pBuildDrop.incoming_section_type, 'drop');

// 10. Genre switch on Level 4 with incompatible tempo -> backspin_cut
const pGenre = planDjTransition(
  { ...clubOut, bpm: 95, genreTags: ['Sufi'], timingProfile: makeTiming(95, 0.85, 4, '4/4') },
  { ...clubIn, bpm: 138, genreTags: ['Bhangra Dhol'], timingProfile: makeTiming(138, 0.85, 4, '4/4') },
  { djLevel: 4, moodId: 'baraat', targetEnergy: 0.92, recentTechniques: [] }
);
assert.equal(pGenre.bpm_relationship, 'no_beatmatch');
assert.equal(pGenre.selected_technique, 'backspin_cut');

// 11. Insufficient remaining playback time -> rejected cleanly
const pShort = planDjTransition(
  { ...clubOut, currentPosSec: 159.2 },
  clubIn,
  { djLevel: 2, moodId: 'house_party', targetEnergy: 0.8, recentTechniques: [] }
);
assert.equal(pShort.status, 'rejected');

// 12. Transition Timeline Scheduler: Prep delay -> replan_needed -> replan -> cancel -> diagnostic log!
const scheduler = new TransitionTimelineScheduler();
const initialPlan = scheduler.scheduleOrReplan(clubOut, clubIn, {
  djLevel: 2,
  moodId: 'house_party',
  targetEnergy: 0.8,
  recentTechniques: [],
  tempoPolicy: createTempoPolicyForLevel(2),
});
assert.equal(initialPlan.status, 'planned');
assert.equal(scheduler.evaluateClockTick(initialPlan.scheduled_execution_sec - 5, 160), 'wait');
assert.equal(scheduler.evaluateClockTick(initialPlan.scheduled_execution_sec, 160), 'execute');
// Simulate preparation delay that missed scheduled boundary by 3 seconds while runway remains
assert.equal(scheduler.evaluateClockTick(initialPlan.scheduled_execution_sec + 3.0, 160), 'replan_needed');

// Shorten active plan dynamically
const shortened = scheduler.shortenActivePlan(4.0, 'Remaining phrase shortened');
assert(shortened && shortened.duration_sec <= 4.0);

// Cancel plan when incoming deck fails -> must record cancellation in diagnostics
const cancelled = scheduler.cancelActivePlan('Incoming deck decode failed');
assert.equal(cancelled.status, 'cancelled');
assert.equal(scheduler.getActivePlan(), null);
assert(scheduler.getDiagnosticLog().length >= 3);

// 13. Live Audio-Graph Execution of Planned Transition (including failed incoming deck guard!)
class MiniParam {
  constructor(v) { this.value = v; this.ev = []; }
  cancelScheduledValues(t) { this.ev.push(['cancel', t]); }
  setValueAtTime(v, t) { this.value = v; this.ev.push(['set', v, t]); }
  linearRampToValueAtTime(v, t) { this.value = v; this.ev.push(['lin', v, t]); }
  exponentialRampToValueAtTime(v, t) { this.value = v; this.ev.push(['exp', v, t]); }
  setTargetAtTime(v, t) { this.value = v; this.ev.push(['tgt', v, t]); }
}
const mkNode = () => ({ connect() {}, disconnect() {} });
const ctx = {
  sampleRate: 44100,
  currentTime: 25.0,
  destination: mkNode(),
  createGain: () => ({ ...mkNode(), gain: new MiniParam(1) }),
  createBiquadFilter: () => ({ ...mkNode(), type: 'allpass', frequency: new MiniParam(1000), Q: new MiniParam(1), gain: new MiniParam(0) }),
  createDelay: () => ({ ...mkNode(), delayTime: new MiniParam(0.3) }),
  createDynamicsCompressor: () => ({ ...mkNode(), threshold: new MiniParam(-1), knee: new MiniParam(0), ratio: new MiniParam(20), attack: new MiniParam(0.002), release: new MiniParam(0.05) }),
  createAnalyser: () => ({ ...mkNode(), fftSize: 256, smoothingTimeConstant: 0.8, getByteTimeDomainData(a) { a.fill(128); } }),
  createBufferSource: () => ({ ...mkNode(), buffer: null, playbackRate: new MiniParam(1), onended: null, start(w, o) { this.started = { w, o }; }, stop() {} }),
};
const buf = { duration: 160, sampleRate: 44100, length: 44100 * 160, numberOfChannels: 2, getChannelData: () => new Float32Array(44100).fill(0.18) };

const engine = new FourDeckWebAudioEngine();
engine.init(ctx);
engine.setDjLevel(3);
engine.setMoodId('desi_club');
engine.loadDecodedDeckBuffer(0, buf, {
  submissionId: 't1', cacheId: 'c1', title: 'Club A', artist: 'DJ A', bpm: 128, camelotKey: '8A',
  genreTags: ['Tech House'], sections: clubOut.sections, timingProfile: clubOut.timingProfile,
});
engine.loadDecodedDeckBuffer(1, buf, {
  submissionId: 't2', cacheId: 'c2', title: 'Club B', artist: 'DJ B', bpm: 128, camelotKey: '8A',
  genreTags: ['Tech House'], sections: clubIn.sections, timingProfile: clubIn.timingProfile,
});
engine.startDeck(0, 1.0);
const planned = engine.schedulePlannedTransition(0, 1);
assert.equal(planned.status, 'planned');
assert.equal(engine.executePlannedTransition(planned), true);
assert.equal(engine.decks[1].sourceNode.started.o, planned.incoming_mix_in_sec);
assert(engine.getTransitionDiagnostics().some((d) => d.status === 'executed'));

// Failed incoming deck test: Deck 0 stays playing uninterrupted when Deck 2 is empty/failed!
engine.stopDeck(2);
const badExec = engine.executePlannedTransition({ ...planned, incoming_deck_id: 2 });
assert.equal(badExec, false);
assert.equal(engine.decks[0].isPlaying, true, 'Failed incoming deck must NEVER interrupt outgoing deck');

console.log('PASSED: All 17 Transition Intelligence, Timing Map, Meter, Vocal, Indian Mood & Timeline Scheduler scenarios verified.');
