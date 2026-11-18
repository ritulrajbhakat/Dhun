// DHUN Dedicated DJ Transition Decision Engine & Timeline Scheduler
//
// Strictly separates musical transition PLANNING (WHEN, WHERE, HOW, HOW LONG, WHICH BPM relationship)
// from DSP EXECUTION (`WebAudioEngine.ts`). Zero AI dependencies — 100% deterministic musical rules.

import {
  BpmRelationshipType,
  DjLevel,
  DjTechniqueId,
  EqBassHandoverSchedule,
  RejectedTechniqueAlternative,
  TrackSectionMarker,
  TrackTimingProfile,
  TransitionDecisionPlan,
} from '../types';

export interface TempoQualityPolicy {
  maxDirectStretchPct: number;
  maxHalfDoubleStretchPct: number;
  maxGradualRampPct: number;
  minBeatgridConfidence: number;
  maxVariableDriftBpm: number;
  allowThreeQuarterBridge: boolean;
}

export function createTempoPolicyForLevel(level: DjLevel): TempoQualityPolicy {
  switch (level) {
    case 1:
      return {
        maxDirectStretchPct: 4.0,
        maxHalfDoubleStretchPct: 0.0,
        maxGradualRampPct: 0.0,
        minBeatgridConfidence: 0.55,
        maxVariableDriftBpm: 3.0,
        allowThreeQuarterBridge: false,
      };
    case 2:
      return {
        maxDirectStretchPct: 8.0,
        maxHalfDoubleStretchPct: 6.0,
        maxGradualRampPct: 0.0,
        minBeatgridConfidence: 0.50,
        maxVariableDriftBpm: 3.5,
        allowThreeQuarterBridge: false,
      };
    case 3:
      return {
        maxDirectStretchPct: 10.0,
        maxHalfDoubleStretchPct: 8.0,
        maxGradualRampPct: 11.0,
        minBeatgridConfidence: 0.48,
        maxVariableDriftBpm: 4.0,
        allowThreeQuarterBridge: true,
      };
    case 4:
    default:
      return {
        maxDirectStretchPct: 12.0,
        maxHalfDoubleStretchPct: 10.0,
        maxGradualRampPct: 16.0,
        minBeatgridConfidence: 0.45,
        maxVariableDriftBpm: 4.5,
        allowThreeQuarterBridge: true,
      };
  }
}

export interface TrackDecisionInput {
  trackId: string;
  deckId: number;
  title: string;
  artist: string;
  durationSec: number;
  currentPosSec: number;
  bpm?: number | null;
  camelotKey?: string | null;
  overallEnergy?: number | null;
  genreTags?: string[];
  sections?: TrackSectionMarker[];
  timingProfile?: TrackTimingProfile | null;
}

export interface PartyDecisionContext {
  djLevel: DjLevel;
  moodId: string;
  targetEnergy: number;
  recentTechniques: string[];
  tempoPolicy?: TempoQualityPolicy;
}

export interface BpmRelationshipDecision {
  relationship: BpmRelationshipType;
  targetBpm: number | null;
  incomingPlaybackRate: number;
  outgoingTargetRate: number;
  adjustmentPct: number;
  direction: 'up' | 'down' | 'none';
  reason: string;
}

export function calculateCamelotCompatibility(keyA?: string | null, keyB?: string | null): number {
  if (!keyA || !keyB) return 0.65;
  const parse = (k: string): [number, string] | null => {
    const m = k.trim().toUpperCase().match(/^(\d{1,2})([AB])$/);
    if (!m) return null;
    const num = Number(m[1]);
    if (num < 1 || num > 12) return null;
    return [num, m[2]];
  };
  const a = parse(keyA);
  const b = parse(keyB);
  if (!a || !b) return 0.65;
  if (a[0] === b[0] && a[1] === b[1]) return 1.0;
  if (a[0] === b[0] && a[1] !== b[1]) return 0.90;
  const dist = Math.min(Math.abs(a[0] - b[0]), 12 - Math.abs(a[0] - b[0]));
  if (a[1] === b[1] && dist === 1) return 0.90;
  if (a[1] === b[1] && dist === 2) return 0.75;
  return 0.35;
}

export function localBpmAtSec(track: TrackDecisionInput, positionSec: number): number | null {
  const tp = track.timingProfile;
  if (tp) {
    const posMs = Math.round(positionSec * 1000);
    if (tp.tempo_map && tp.tempo_map.length > 0) {
      const seg =
        tp.tempo_map.find((s) => posMs >= s.start_ms && posMs <= s.end_ms) ||
        tp.tempo_map[tp.tempo_map.length - 1];
      if (seg && seg.bpm > 40) return seg.bpm;
    }
    if (tp.average_bpm > 40) return tp.average_bpm;
  }
  return track.bpm && track.bpm > 40 ? track.bpm : null;
}

export function evaluateBpmRelationship(
  outBpm: number | null,
  inBpm: number | null,
  beatgridConf: number,
  meterCompatible: boolean,
  outVariable: boolean,
  inVariable: boolean,
  policy: TempoQualityPolicy
): BpmRelationshipDecision {
  if (!outBpm || !inBpm) {
    return {
      relationship: 'no_beatmatch',
      targetBpm: null,
      incomingPlaybackRate: 1.0,
      outgoingTargetRate: 1.0,
      adjustmentPct: 0,
      direction: 'none',
      reason: 'BPM unavailable on one or both tracks — refusing forced beatmatch.',
    };
  }

  if (beatgridConf < policy.minBeatgridConfidence) {
    return {
      relationship: 'no_beatmatch',
      targetBpm: inBpm,
      incomingPlaybackRate: 1.0,
      outgoingTargetRate: 1.0,
      adjustmentPct: 0,
      direction: 'none',
      reason: `Beatgrid confidence (${Math.round(beatgridConf * 100)}%) below threshold (${Math.round(policy.minBeatgridConfidence * 100)}%) — no beatmatch.`,
    };
  }

  if (!meterCompatible) {
    return {
      relationship: 'no_beatmatch',
      targetBpm: inBpm,
      incomingPlaybackRate: 1.0,
      outgoingTargetRate: 1.0,
      adjustmentPct: 0,
      direction: 'none',
      reason: 'Incompatible rhythmic meters (e.g., 3/4 or 6/8 vs 4/4) — preserving natural rhythm without beatmatch.',
    };
  }

  const dir = (r: number): 'up' | 'down' | 'none' =>
    r > 1.001 ? 'up' : r < 0.999 ? 'down' : 'none';

  if (outVariable || inVariable) {
    const directPct = (Math.abs(outBpm - inBpm) / outBpm) * 100;
    if (directPct <= 3.0) {
      const rate = Math.max(0.97, Math.min(1.03, outBpm / inBpm));
      return {
        relationship: 'direct',
        targetBpm: outBpm,
        incomingPlaybackRate: Number(rate.toFixed(3)),
        outgoingTargetRate: 1.0,
        adjustmentPct: Number((Math.abs(rate - 1) * 100).toFixed(1)),
        direction: dir(rate),
        reason: 'Variable-tempo tracks within 3% local BPM — light direct beat-assist.',
      };
    }
    return {
      relationship: 'no_beatmatch',
      targetBpm: inBpm,
      incomingPlaybackRate: 1.0,
      outgoingTargetRate: 1.0,
      adjustmentPct: 0,
      direction: 'none',
      reason: 'Variable percussion tempo drift detected — avoiding pitch stretch to protect live rhythm.',
    };
  }

  // 1. Direct match
  const directRate = outBpm / inBpm;
  const directPct = Math.abs(directRate - 1) * 100;
  if (directPct <= policy.maxDirectStretchPct) {
    return {
      relationship: 'direct',
      targetBpm: outBpm,
      incomingPlaybackRate: Number(directRate.toFixed(3)),
      outgoingTargetRate: 1.0,
      adjustmentPct: Number(directPct.toFixed(1)),
      direction: dir(directRate),
      reason: `Direct BPM match (${inBpm.toFixed(1)} → ${outBpm.toFixed(1)} BPM, ${directPct.toFixed(1)}% shift).`,
    };
  }

  // 2. Half-time & 3. Double-time
  if (policy.maxHalfDoubleStretchPct > 0) {
    const halfRate = outBpm / (inBpm * 2);
    const halfPct = Math.abs(halfRate - 1) * 100;
    if (halfPct <= policy.maxHalfDoubleStretchPct) {
      return {
        relationship: 'half_time',
        targetBpm: Number((outBpm * 0.5).toFixed(1)),
        incomingPlaybackRate: Number(halfRate.toFixed(3)),
        outgoingTargetRate: 1.0,
        adjustmentPct: Number(halfPct.toFixed(1)),
        direction: dir(halfRate),
        reason: `Half-time rhythmic lock (${inBpm.toFixed(1)} BPM aligned 1:2 with ${outBpm.toFixed(1)} BPM).`,
      };
    }

    const doubleRate = outBpm / (inBpm * 0.5);
    const doublePct = Math.abs(doubleRate - 1) * 100;
    if (doublePct <= policy.maxHalfDoubleStretchPct) {
      return {
        relationship: 'double_time',
        targetBpm: Number((outBpm * 2).toFixed(1)),
        incomingPlaybackRate: Number(doubleRate.toFixed(3)),
        outgoingTargetRate: 1.0,
        adjustmentPct: Number(doublePct.toFixed(1)),
        direction: dir(doubleRate),
        reason: `Double-time rhythmic lock (${inBpm.toFixed(1)} BPM aligned 2:1 with ${outBpm.toFixed(1)} BPM).`,
      };
    }
  }

  // 4. Gradual tempo ramp
  if (policy.maxGradualRampPct > 0 && directPct <= policy.maxGradualRampPct) {
    const outTarget = Math.max(0.84, Math.min(1.16, inBpm / outBpm));
    const inStart = Math.max(0.84, Math.min(1.16, outBpm / inBpm));
    return {
      relationship: 'gradual_tempo_ramp',
      targetBpm: inBpm,
      incomingPlaybackRate: Number(inStart.toFixed(3)),
      outgoingTargetRate: Number(outTarget.toFixed(3)),
      adjustmentPct: Number(directPct.toFixed(1)),
      direction: inBpm > outBpm ? 'up' : 'down',
      reason: `Gradual tempo ramp bridging ${outBpm.toFixed(1)} → ${inBpm.toFixed(1)} BPM (${directPct.toFixed(1)}% shift).`,
    };
  }

  // 5. 3:4 / 4:3 rhythmic bridge
  if (policy.allowThreeQuarterBridge) {
    const r34 = (outBpm * 0.75) / inBpm;
    const r43 = (outBpm * (4 / 3)) / inBpm;
    const p34 = Math.abs(r34 - 1) * 100;
    const p43 = Math.abs(r43 - 1) * 100;
    if (p34 <= 4.0 || p43 <= 4.0) {
      const bestR = p34 <= p43 ? r34 : r43;
      const bestP = Math.min(p34, p43);
      return {
        relationship: 'three_quarter_bridge',
        targetBpm: inBpm,
        incomingPlaybackRate: Number(bestR.toFixed(3)),
        outgoingTargetRate: 1.0,
        adjustmentPct: Number(bestP.toFixed(1)),
        direction: dir(bestR),
        reason: `3:4 rhythmic tempo bridge (${outBpm.toFixed(1)} → ${inBpm.toFixed(1)} BPM).`,
      };
    }
  }

  return {
    relationship: 'no_beatmatch',
    targetBpm: inBpm,
    incomingPlaybackRate: 1.0,
    outgoingTargetRate: 1.0,
    adjustmentPct: Number(directPct.toFixed(1)),
    direction: 'none',
    reason: `BPM gap (${outBpm.toFixed(1)} → ${inBpm.toFixed(1)} BPM, ${directPct.toFixed(1)}%) exceeds musical quality limit — refusing beatmatch.`,
  };
}

function classifyMoodRhythm(moodId: string, outGenres: string[], inGenres: string[]) {
  const m = (moodId || '').toLowerCase();
  const g = [...outGenres, ...inGenres].join(' ').toLowerCase();
  const isCyclicFolkGarba =
    m.includes('garba') || m.includes('dandiya') || g.includes('garba') || g.includes('dandiya');
  const isPercussionHeavyDhol =
    m.includes('baraat') || g.includes('dhol') || g.includes('bhangra');
  const isIndianVocalCentric =
    m.includes('shaadi') ||
    m.includes('bollywood') ||
    m.includes('mehendi') ||
    m.includes('haldi') ||
    m.includes('sangeet') ||
    m.includes('retro_bollywood') ||
    m.includes('punjabi') ||
    g.includes('bollywood') ||
    g.includes('punjabi') ||
    g.includes('sufi');
  const isElectronicClub =
    m.includes('desi_club') ||
    m.includes('house') ||
    m.includes('club') ||
    g.includes('edm') ||
    g.includes('house') ||
    g.includes('techno') ||
    g.includes('remix');
  return {
    isCyclicFolkGarba,
    isPercussionHeavyDhol,
    isIndianVocalCentric,
    isElectronicClub,
    preferShortCleanMix: isPercussionHeavyDhol || isCyclicFolkGarba,
  };
}

/**
 * Core deterministic DJ Transition Decision Engine.
 */
export function planDjTransition(
  outgoing: TrackDecisionInput,
  incoming: TrackDecisionInput,
  party: PartyDecisionContext
): TransitionDecisionPlan {
  const policy = party.tempoPolicy || createTempoPolicyForLevel(party.djLevel);
  const remainingSec = Math.max(0, outgoing.durationSec - outgoing.currentPosSec);
  const reasons: string[] = [];
  const rejectedAlternatives: RejectedTechniqueAlternative[] = [];

  const outConf = outgoing.timingProfile?.beatgrid_confidence ?? (outgoing.bpm ? 0.65 : 0.25);
  const inConf = incoming.timingProfile?.beatgrid_confidence ?? (incoming.bpm ? 0.65 : 0.25);
  const beatgridConfidence = Number(Math.min(outConf, inConf).toFixed(2));

  const outMeter = outgoing.timingProfile?.meter || { beats_per_bar: 4, meter_label: '4/4', confidence: 0.6 };
  const inMeter = incoming.timingProfile?.meter || { beats_per_bar: 4, meter_label: '4/4', confidence: 0.6 };
  const meterUncertain = outMeter.meter_label === 'uncertain' || inMeter.meter_label === 'uncertain';
  const meterCompatible = !meterUncertain && outMeter.beats_per_bar === inMeter.beats_per_bar;
  const meterSummary = `${outMeter.meter_label} → ${inMeter.meter_label}`;
  const beatsPerBar = meterCompatible ? Math.max(3, outMeter.beats_per_bar) : 4;

  const outVariable = Boolean(
    outgoing.timingProfile &&
      (outgoing.timingProfile.is_variable_tempo ||
        outgoing.timingProfile.tempo_drift_bpm >= policy.maxVariableDriftBpm)
  );
  const inVariable = Boolean(
    incoming.timingProfile &&
      (incoming.timingProfile.is_variable_tempo ||
        incoming.timingProfile.tempo_drift_bpm >= policy.maxVariableDriftBpm)
  );

  // Reject transition cleanly when remaining playback time is insufficient (< 1.2s)
  if (remainingSec < 1.2) {
    return {
      status: 'rejected',
      outgoing_track_id: outgoing.trackId,
      outgoing_deck_id: outgoing.deckId,
      incoming_track_id: incoming.trackId,
      incoming_deck_id: incoming.deckId,
      outgoing_mix_out_sec: outgoing.durationSec,
      outgoing_phrase_boundary_sec: outgoing.durationSec,
      incoming_mix_in_sec: 0,
      incoming_phrase_boundary_sec: 0,
      outgoing_section_type: 'end',
      incoming_section_type: 'intro',
      current_bpm: localBpmAtSec(outgoing, outgoing.currentPosSec),
      incoming_bpm: localBpmAtSec(incoming, 0),
      target_bpm: localBpmAtSec(incoming, 0),
      bpm_relationship: 'no_beatmatch',
      tempo_adjustment_pct: 0,
      tempo_direction: 'none',
      incoming_playback_rate: 1.0,
      outgoing_target_rate: 1.0,
      beat_alignment_offset_sec: 0,
      meter_compatible: meterCompatible,
      meter_summary: meterSummary,
      beatgrid_confidence: beatgridConfidence,
      duration_beats: 1,
      duration_bars: 0,
      duration_sec: 0.25,
      vocal_overlap_policy: 'clean_cut_handover',
      energy_progression: 'reset',
      eq_bass_handover_schedule: {
        incoming_low_cut_until_frac: 0,
        bass_swap_at_frac: 0.5,
        outgoing_low_cut_at_frac: 0.5,
        mid_carve_db: 0,
        high_handover_frac: 0.5,
      },
      selected_technique: 'safe_quick_cut',
      scheduled_execution_sec: outgoing.currentPosSec,
      confidence: 0.2,
      reasons: ['Insufficient remaining playback time (< 1.2s) for a musical transition.'],
      rejected_alternatives: [],
      rejection_reason: 'Insufficient remaining playback time (< 1.2s).',
    };
  }

  const outGenres = outgoing.genreTags || [];
  const inGenres = incoming.genreTags || [];
  const moodRhythm = classifyMoodRhythm(party.moodId, outGenres, inGenres);
  const genresConflict =
    outGenres.length > 0 &&
    inGenres.length > 0 &&
    !outGenres.some((og) => inGenres.some((ig) => og.toLowerCase() === ig.toLowerCase()));

  // Select outgoing exit phrase boundary (protecting build-ups & hooks!)
  const outSections = outgoing.sections || [];
  const upcomingOut = outSections.filter((s) => s.end_ms / 1000 > outgoing.currentPosSec + 2.0);
  const activeOut = outSections.find(
    (s) => outgoing.currentPosSec >= s.start_ms / 1000 && outgoing.currentPosSec < s.end_ms / 1000
  );
  const protectingBuild = Boolean(activeOut && activeOut.section_type === 'build' && (activeOut.confidence ?? 0) >= 0.55);

  let outMixOutSec = Math.max(outgoing.currentPosSec + 1.0, outgoing.durationSec - 8.0);
  let outSectionType = 'phrase';
  let outVocalAct = 0.45;
  let outEnergy = outgoing.overallEnergy ?? 0.7;

  if (upcomingOut.length > 0) {
    let best: { sec: TrackSectionMarker; exitPt: number; sc: number } | null = null;
    for (let i = 0; i < upcomingOut.length; i++) {
      const sec = upcomingOut[i];
      const st = Math.max(outgoing.currentPosSec + 1.0, sec.start_ms / 1000);
      const en = Math.min(outgoing.durationSec, sec.end_ms / 1000);
      if (sec.section_type === 'build') {
        const nextSec = upcomingOut[i + 1];
        if (nextSec && ['drop', 'peak', 'chorus'].includes(nextSec.section_type)) {
          continue; // Never cut a build-up right before its drop/payoff!
        }
      }
      let sc = 0;
      let exitPt = st;
      switch (sec.section_type) {
        case 'outro':
          sc += 5.0;
          exitPt = st;
          break;
        case 'breakdown':
          sc += 4.2;
          exitPt = st;
          break;
        case 'instrumental':
        case 'groove':
          sc += 3.8;
          exitPt = st;
          break;
        case 'chorus':
        case 'peak':
          sc += 3.4;
          exitPt = Math.max(st, en - 8.0);
          break;
        case 'drop':
          sc += 2.6;
          exitPt = Math.max(st, en - 4.0);
          break;
        case 'verse':
          sc += 2.2;
          exitPt = Math.max(st, Math.min(en, outgoing.durationSec - 4.0));
          break;
        default:
          sc += 1.5;
          exitPt = st;
      }
      if (outgoing.durationSec - exitPt < 2.0) sc -= 2.5;
      if ((sec.vocal_activity ?? 0.45) < 0.40) sc += 0.8;
      if (!best || sc > best.sc) best = { sec, exitPt, sc };
    }
    if (best) {
      if (protectingBuild) {
        reasons.push('Protected active build-up so its drop/payoff completes before mix-out.');
      }
      outMixOutSec = best.exitPt;
      outSectionType = best.sec.section_type;
      outVocalAct = best.sec.vocal_activity ?? 0.45;
      outEnergy = best.sec.energy_level;
    }
  }

  // Select incoming entry point (avoiding vocal clash & competing drops!)
  const inSections = incoming.sections || [];
  let inMixInSec = 0;
  let inSectionType = 'intro';
  let inVocalAct = 0.25;
  let inEnergy = incoming.overallEnergy ?? 0.7;

  if (inSections.length > 0) {
    let bestIn: { sec: TrackSectionMarker; sc: number } | null = null;
    for (const sec of inSections) {
      const st = Math.max(0, sec.start_ms / 1000);
      if (incoming.durationSec - st < 12.0 || sec.section_type === 'outro') continue;
      let sc = 0;
      const t = sec.section_type;
      if (t === 'intro' && ['chorus', 'peak', 'outro', 'verse'].includes(outSectionType)) sc += 4.5;
      else if ((t === 'instrumental' || t === 'groove') && outVocalAct >= 0.50) sc += 4.8;
      else if (t === 'breakdown' && ['outro', 'peak', 'chorus'].includes(outSectionType)) sc += 4.0;
      else if (t === 'build' && ['breakdown', 'groove', 'outro'].includes(outSectionType)) sc += 4.3;
      else if (['drop', 'chorus', 'peak'].includes(t) && outSectionType === 'breakdown') sc += 4.6;
      else if (t === 'intro') sc += 3.2;
      else if (t === 'groove' || t === 'instrumental') sc += 3.5;
      else if (t === 'chorus') sc += 2.4;

      if (outVocalAct >= 0.55 && (sec.vocal_activity ?? 0.25) >= 0.55) sc -= 4.5;
      if (outSectionType === 'drop' && t === 'drop') sc -= 5.0;
      sc -= Math.abs(sec.energy_level - party.targetEnergy) * 1.2;

      if (!bestIn || sc > bestIn.sc) bestIn = { sec, sc };
    }
    if (bestIn) {
      inMixInSec = Math.max(0, bestIn.sec.start_ms / 1000);
      inSectionType = bestIn.sec.section_type;
      inVocalAct = bestIn.sec.vocal_activity ?? 0.25;
      inEnergy = bestIn.sec.energy_level;
    }
  }

  // Evaluate BPM relationship at ACTUAL mix-out & mix-in local tempos
  const outLocalBpm = localBpmAtSec(outgoing, outMixOutSec);
  const inLocalBpm = localBpmAtSec(incoming, inMixInSec);
  const bpmDecision = evaluateBpmRelationship(
    outLocalBpm,
    inLocalBpm,
    beatgridConfidence,
    meterCompatible,
    outVariable,
    inVariable,
    policy
  );
  reasons.push(bpmDecision.reason);

  const vocalClash = outVocalAct >= 0.55 && inVocalAct >= 0.55;
  const vocalOverlapPolicy = vocalClash
    ? 'avoid_vocal_clash'
    : outVocalAct >= 0.50 && inVocalAct <= 0.35
    ? 'instrumental_under_vocal'
    : bpmDecision.relationship === 'no_beatmatch'
    ? 'clean_cut_handover'
    : 'vocal_clear_blend';

  if (vocalClash) {
    reasons.push('High vocal activity on both decks — enforcing vocal-clash avoidance.');
  } else if (vocalOverlapPolicy === 'instrumental_under_vocal') {
    reasons.push('Incoming instrumental/intro cleanly supports outgoing vocal phrase.');
  }

  const keyCompat = calculateCamelotCompatibility(outgoing.camelotKey, incoming.camelotKey);
  const energyDelta = inEnergy - outEnergy;
  const energyProgression =
    energyDelta >= 0.12
      ? 'build'
      : energyDelta <= -0.14
      ? 'drop_relief'
      : outEnergy >= 0.8 && inEnergy >= 0.8
      ? 'peak_hold'
      : 'sustain';

  const recentUsed = (name: string) => party.recentTechniques.slice(0, 2).includes(name);

  let selectedTechnique: DjTechniqueId;
  if (bpmDecision.relationship === 'no_beatmatch') {
    rejectedAlternatives.push(
      { technique: 'phrase_long_blend', reason: 'Rejected: no reliable beatmatch/meter alignment for multi-bar blend.' },
      { technique: 'timed_bass_swap', reason: 'Rejected: requires beat-locked tempo match.' }
    );
    if (party.djLevel >= 4 && genresConflict && !recentUsed('backspin_cut')) {
      selectedTechnique = 'backspin_cut';
    } else if (party.djLevel >= 3 && !moodRhythm.isPercussionHeavyDhol) {
      selectedTechnique = 'echo_out_blend';
    } else {
      selectedTechnique = 'safe_quick_cut';
    }
  } else if (bpmDecision.relationship === 'gradual_tempo_ramp') {
    rejectedAlternatives.push({
      technique: 'timed_bass_swap',
      reason: 'Rejected: BPM difference exceeds static pitch-lock limit; gradual ramp required.',
    });
    selectedTechnique = party.djLevel >= 4 ? 'tempo_ramp_bridge' : 'echo_out_blend';
  } else if (bpmDecision.relationship === 'three_quarter_bridge') {
    rejectedAlternatives.push({
      technique: 'phrase_long_blend',
      reason: 'Rejected: 3:4 bridge works best via echo-out/cut rather than sustained overlap.',
    });
    selectedTechnique = party.djLevel >= 3 ? 'echo_out_blend' : 'safe_quick_cut';
  } else if (vocalClash || moodRhythm.preferShortCleanMix) {
    rejectedAlternatives.push({
      technique: 'phrase_long_blend',
      reason: vocalClash
        ? 'Rejected: long overlap would cause vocal-vs-vocal clash.'
        : 'Rejected: Indian dhol/garba percussion clarity degrades under long overlapping blends.',
    });
    if (party.djLevel === 1) selectedTechnique = 'eq3_handover';
    else if (party.djLevel === 2) {
      selectedTechnique = moodRhythm.isPercussionHeavyDhol && genresConflict ? 'safe_quick_cut' : 'timed_bass_swap';
    } else if (party.djLevel === 3) selectedTechnique = 'echo_out_blend';
    else selectedTechnique = outSectionType === 'breakdown' || energyDelta >= 0.1 ? 'echo_freeze_drop' : 'echo_out_blend';
  } else {
    if (party.djLevel === 1) {
      rejectedAlternatives.push({
        technique: 'timed_bass_swap',
        reason: 'Rejected: Level 1 preserves classic 2-deck EQ/phrase mixing.',
      });
      selectedTechnique = bpmDecision.adjustmentPct <= 3.0 && keyCompat >= 0.75 ? 'phrase_long_blend' : 'eq3_handover';
    } else if (party.djLevel === 2) {
      rejectedAlternatives.push({
        technique: 'beat_roll_buildup',
        reason: 'Rejected: performance FX unlocked at Level 3+.',
      });
      selectedTechnique = keyCompat >= 0.80 && !recentUsed('timed_bass_swap') ? 'timed_bass_swap' : 'filter_sweep_drop';
    } else if (party.djLevel === 3) {
      if ((outSectionType === 'breakdown' || energyDelta >= 0.08) && !recentUsed('beat_roll_buildup')) {
        selectedTechnique = 'beat_roll_buildup';
      } else if (moodRhythm.isElectronicClub && keyCompat >= 0.85 && !recentUsed('four_deck_layer')) {
        selectedTechnique = 'four_deck_layer';
      } else if (energyDelta <= -0.10) {
        selectedTechnique = 'echo_out_blend';
      } else {
        selectedTechnique = 'filtered_buildup_drop';
      }
    } else {
      if (keyCompat >= 0.88 && inVocalAct <= 0.45 && !recentUsed('creative_mashup_layer')) {
        selectedTechnique = 'creative_mashup_layer';
      } else if ((outSectionType === 'breakdown' || energyDelta >= 0.08) && !recentUsed('stutter_gate_drop')) {
        selectedTechnique = 'stutter_gate_drop';
      } else if (!recentUsed('echo_freeze_drop')) {
        selectedTechnique = 'echo_freeze_drop';
      } else {
        selectedTechnique = 'four_deck_layer';
      }
    }
  }

  // Adaptive transition length in beats (4 | 8 | 16 | 32 | 64) -> seconds from real tempo!
  const refBpm = Math.max(60, Math.min(200, outLocalBpm || inLocalBpm || 124));
  const beatSec = 60 / refBpm;
  let rawBeats = 16;
  if (selectedTechnique === 'safe_quick_cut' || selectedTechnique === 'backspin_cut') rawBeats = 4;
  else if (
    selectedTechnique === 'stutter_gate_drop' ||
    selectedTechnique === 'beat_roll_buildup' ||
    selectedTechnique === 'echo_freeze_drop'
  ) {
    rawBeats = 8;
  } else if (selectedTechnique === 'echo_out_blend') {
    rawBeats = vocalClash || moodRhythm.preferShortCleanMix ? 8 : 16;
  } else if (
    selectedTechnique === 'eq3_handover' ||
    selectedTechnique === 'timed_bass_swap' ||
    selectedTechnique === 'filter_sweep_drop' ||
    selectedTechnique === 'filtered_buildup_drop' ||
    selectedTechnique === 'tempo_ramp_bridge'
  ) {
    rawBeats =
      vocalClash || moodRhythm.isIndianVocalCentric || genresConflict
        ? 8
        : moodRhythm.isElectronicClub && inVocalAct < 0.35
        ? 32
        : 16;
  } else if (selectedTechnique === 'phrase_long_blend') {
    rawBeats =
      moodRhythm.isElectronicClub && outVocalAct < 0.3 && inVocalAct < 0.3 && keyCompat >= 0.85
        ? 64
        : vocalClash || moodRhythm.isIndianVocalCentric
        ? 16
        : 32;
  } else if (selectedTechnique === 'four_deck_layer' || selectedTechnique === 'creative_mashup_layer') {
    rawBeats = moodRhythm.isElectronicClub && outVocalAct < 0.35 && inVocalAct < 0.35 ? 64 : 32;
  }

  const maxFitSec = Math.max(
    0.5,
    Math.min(
      Math.max(outgoing.durationSec - outMixOutSec, remainingSec * 0.5),
      remainingSec - 0.3,
      (incoming.durationSec - inMixInSec) * 0.45
    )
  );
  const allowedLengths = [64, 32, 16, 8, 4];
  const durationBeats = allowedLengths.find((b) => b <= rawBeats && b * beatSec <= maxFitSec) ?? 4;
  const durationSec =
    selectedTechnique === 'safe_quick_cut'
      ? Math.max(0.25, Math.min(0.55, beatSec * 0.75))
      : selectedTechnique === 'backspin_cut'
      ? Math.max(0.65, Math.min(1.35, beatSec * 2.0))
      : Math.max(0.5, Math.min(maxFitSec, durationBeats * beatSec));
  const durationBars = Math.max(1, Math.floor(durationBeats / Math.max(3, beatsPerBar)));

  // Snap scheduled execution time to nearest outgoing downbeat when beatgrid is confident
  let scheduledExecutionSec = Math.min(
    Math.max(outgoing.currentPosSec, outgoing.durationSec - durationSec),
    Math.max(outgoing.currentPosSec, outMixOutSec)
  );
  let beatAlignmentOffsetSec = 0;

  if (beatgridConfidence >= 0.50 && outgoing.timingProfile?.downbeat_timestamps_ms?.length) {
    const targetMs = Math.round(scheduledExecutionSec * 1000);
    const futureDownbeats = outgoing.timingProfile.downbeat_timestamps_ms.filter(
      (ms) => ms / 1000 >= outgoing.currentPosSec
    );
    if (futureDownbeats.length > 0) {
      let nearest = futureDownbeats[0];
      for (const ms of futureDownbeats) {
        if (Math.abs(ms - targetMs) < Math.abs(nearest - targetMs)) nearest = ms;
      }
      const snapped = nearest / 1000;
      if (Math.abs(snapped - scheduledExecutionSec) <= beatSec * 4.0 && snapped + durationSec <= outgoing.durationSec) {
        beatAlignmentOffsetSec = Number((snapped - scheduledExecutionSec).toFixed(3));
        scheduledExecutionSec = snapped;
      }
    }
  }

  const eqBassHandoverSchedule: EqBassHandoverSchedule = {
    incoming_low_cut_until_frac: selectedTechnique === 'safe_quick_cut' ? 0 : 0.48,
    bass_swap_at_frac: ['filter_sweep_drop', 'filtered_buildup_drop', 'beat_roll_buildup', 'stutter_gate_drop'].includes(
      selectedTechnique
    )
      ? 0.86
      : selectedTechnique === 'echo_freeze_drop'
      ? 0.56
      : 0.50,
    outgoing_low_cut_at_frac: vocalClash ? 0.40 : 0.50,
    mid_carve_db: vocalClash ? -10 : outVocalAct >= 0.50 ? -6 : -3,
    high_handover_frac: 0.35,
  };

  reasons.push(
    `Exit ${outgoing.title} at ${scheduledExecutionSec.toFixed(1)}s (${outSectionType}) → Enter ${incoming.title} at ${inMixInSec.toFixed(1)}s (${inSectionType}) over ${durationBeats} beats (${durationBars} bars, ${durationSec.toFixed(1)}s) via ${selectedTechnique}.`
  );

  const confidence = Number(
    Math.max(
      0.2,
      Math.min(
        0.98,
        beatgridConfidence * 0.45 +
          (meterCompatible ? 0.25 : 0.05) +
          (vocalClash ? 0.05 : 0.15) +
          keyCompat * 0.15
      )
    ).toFixed(2)
  );

  return {
    status: 'planned',
    outgoing_track_id: outgoing.trackId,
    outgoing_deck_id: outgoing.deckId,
    incoming_track_id: incoming.trackId,
    incoming_deck_id: incoming.deckId,
    outgoing_mix_out_sec: Number(scheduledExecutionSec.toFixed(2)),
    outgoing_phrase_boundary_sec: Number(outMixOutSec.toFixed(2)),
    incoming_mix_in_sec: Number(inMixInSec.toFixed(2)),
    incoming_phrase_boundary_sec: Number(inMixInSec.toFixed(2)),
    outgoing_section_type: outSectionType,
    incoming_section_type: inSectionType,
    current_bpm: outLocalBpm,
    incoming_bpm: inLocalBpm,
    target_bpm: bpmDecision.targetBpm,
    bpm_relationship: bpmDecision.relationship,
    tempo_adjustment_pct: bpmDecision.adjustmentPct,
    tempo_direction: bpmDecision.direction,
    incoming_playback_rate: bpmDecision.incomingPlaybackRate,
    outgoing_target_rate: bpmDecision.outgoingTargetRate,
    beat_alignment_offset_sec: beatAlignmentOffsetSec,
    meter_compatible: meterCompatible,
    meter_summary: meterSummary,
    beatgrid_confidence: beatgridConfidence,
    duration_beats: durationBeats,
    duration_bars: durationBars,
    duration_sec: Number(durationSec.toFixed(2)),
    vocal_overlap_policy: vocalOverlapPolicy,
    energy_progression: energyProgression,
    eq_bass_handover_schedule: eqBassHandoverSchedule,
    selected_technique: selectedTechnique,
    scheduled_execution_sec: Number(scheduledExecutionSec.toFixed(2)),
    confidence,
    reasons,
    rejected_alternatives: rejectedAlternatives,
    rejection_reason: null,
  };
}

/**
 * Musical Transition Timeline Scheduler (Section 7):
 * Plans ahead, waits for the exact outgoing musical boundary on `AudioContext.currentTime`,
 * replans cleanly when preparation is delayed past a beat boundary, and cancels safely if
 * the incoming deck fails or becomes unavailable.
 */
export class TransitionTimelineScheduler {
  private activePlan: TransitionDecisionPlan | null = null;
  private diagnosticHistory: TransitionDecisionPlan[] = [];

  public getActivePlan(): TransitionDecisionPlan | null {
    return this.activePlan;
  }

  public getDiagnosticLog(): TransitionDecisionPlan[] {
    return [...this.diagnosticHistory];
  }

  private recordDiagnostic(plan: TransitionDecisionPlan): void {
    this.diagnosticHistory.unshift({ ...plan });
    if (this.diagnosticHistory.length > 40) {
      this.diagnosticHistory.length = 40;
    }
  }

  public scheduleOrReplan(
    outgoing: TrackDecisionInput,
    incoming: TrackDecisionInput,
    party: PartyDecisionContext
  ): TransitionDecisionPlan {
    const plan = planDjTransition(outgoing, incoming, party);
    this.recordDiagnostic(plan);
    if (plan.status === 'planned') {
      this.activePlan = plan;
    } else {
      this.activePlan = null;
    }
    return plan;
  }

  /**
   * Handles incoming track preparation delay: if the originally scheduled boundary was missed
   * by more than 1.5 seconds, replans to the next valid outgoing musical boundary instead of
   * forcing an off-phrase cut!
   */
  public evaluateClockTick(
    outgoingPosSec: number,
    outgoingDurationSec: number
  ): 'none' | 'wait' | 'execute' | 'replan_needed' {
    if (!this.activePlan || this.activePlan.status !== 'planned') return 'none';
    const target = this.activePlan.scheduled_execution_sec;
    const remaining = Math.max(0, outgoingDurationSec - outgoingPosSec);

    // If preparation finished late and we already passed target by > 1.5s while plenty of track remains, replan!
    if (outgoingPosSec > target + 1.5 && remaining > this.activePlan.duration_sec + 4.0) {
      return 'replan_needed';
    }

    // Trigger cleanly right at the scheduled beat/phrase boundary (within 0.2s lookahead)
    if (outgoingPosSec >= target - 0.15) {
      return 'execute';
    }

    return 'wait';
  }

  public markExecuted(): TransitionDecisionPlan | null {
    if (!this.activePlan) return null;
    const executed = { ...this.activePlan, status: 'executed' };
    this.recordDiagnostic(executed);
    this.activePlan = null;
    return executed;
  }

  public cancelActivePlan(reason: string): TransitionDecisionPlan | null {
    if (!this.activePlan) return null;
    const cancelled: TransitionDecisionPlan = {
      ...this.activePlan,
      status: 'cancelled',
      rejection_reason: reason,
      reasons: [...this.activePlan.reasons, `Cancelled: ${reason}`],
    };
    this.recordDiagnostic(cancelled);
    this.activePlan = null;
    return cancelled;
  }

  public shortenActivePlan(maxDurationSec: number, reason: string): TransitionDecisionPlan | null {
    if (!this.activePlan || this.activePlan.status !== 'planned') return null;
    if (this.activePlan.duration_sec <= maxDurationSec) return this.activePlan;
    const refBpm = this.activePlan.current_bpm || this.activePlan.incoming_bpm || 124;
    const beatSec = 60 / Math.max(60, refBpm);
    const shorterBeats = [32, 16, 8, 4].find((b) => b * beatSec <= maxDurationSec) ?? 4;
    const newDurSec = Number(Math.max(0.4, Math.min(maxDurationSec, shorterBeats * beatSec)).toFixed(2));
    this.activePlan = {
      ...this.activePlan,
      duration_beats: shorterBeats,
      duration_bars: Math.max(1, Math.floor(shorterBeats / 4)),
      duration_sec: newDurSec,
      reasons: [...this.activePlan.reasons, `Shortened transition to ${shorterBeats} beats (${newDurSec}s): ${reason}`],
    };
    this.recordDiagnostic(this.activePlan);
    return this.activePlan;
  }
}
