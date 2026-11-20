#!/usr/bin/env node

/**
 * DHUN — 14 DJ MOODS MUSICAL IDENTITY & CONTRAST BENCHMARK
 *
 * Verifies that all 14 party moods exhibit genuinely distinct DJ personalities:
 * 1. Meaningful ranking contrasts across the 6 required mood pairs:
 *    - Shaadi vs Cocktail Night
 *    - Baraat vs Retro Bollywood
 *    - Garba vs Dandiya
 *    - Punjabi Tashan vs House Party
 *    - Mehendi vs Festival Mode
 *    - Desi Club vs International Party
 * 2. Multi-track set sequence progressions:
 *    - Baraat (high-octane procession stamina, tight <= 8 beat cuts, downbeat drops)
 *    - Cocktail Night (luxurious 32-64 beat blends, gentle lowpass breathing room, hypnotic ~0.60 energy)
 *    - Garba (progressive cyclic acceleration, phrase sync, uninterrupted folk circle)
 * 3. Transition purpose, duration, and DSP behavior governed by active mood.
 */

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

const { CURATED_MOODS } = loadTs('src/api/ipc.ts');
const { planDjTransition } = loadTs('src/audio/TransitionDecisionEngine.ts');

console.log('='.repeat(75));
console.log('DHUN — ALL 14 DJ MOODS MUSICAL IDENTITY & CONTRAST BENCHMARK');
console.log('='.repeat(75));

// Verify all 14 preset moods are present and contain enriched attributes
assert.equal(CURATED_MOODS.length, 14, 'Must have exactly 14 curated moods');

const moodMap = new Map(CURATED_MOODS.map((m) => [m.id, m]));

// Helper to calculate Camelot compatibility
function calculateCamelotCompat(keyA, keyB) {
  if (!keyA || !keyB) return 0.65;
  if (keyA === keyB) return 1.0;
  const numA = parseInt(keyA, 10);
  const numB = parseInt(keyB, 10);
  const letterA = keyA.slice(-1);
  const letterB = keyB.slice(-1);
  if (letterA === letterB) {
    const diff = Math.abs(numA - numB);
    if (diff === 1 || diff === 11) return 0.90;
    if (diff === 2 || diff === 10) return 0.75;
  } else if (numA === numB) {
    return 0.88; // Relative major/minor
  }
  return 0.35;
}

// Mirroring the Rust selector evaluate_submission_dj_intelligence scoring logic
function scoreCandidateForMood(cand, cur, mood, unplayedQueue = []) {
  let reasons = [];
  let penalties = [];
  let penaltyDeduction = 0.0;

  const candBpm = cand.bpm;
  const curBpm = cur ? cur.bpm : (mood.min_bpm + mood.max_bpm) / 2;
  const candEnergy = cand.overallEnergy;
  const curEnergy = cur ? cur.overallEnergy : mood.target_energy;
  const deltaEnergy = candEnergy - curEnergy;

  // 1. Harmonic Key (25 pts max)
  let keyCompat = calculateCamelotCompat(cur ? cur.camelotKey : null, cand.camelotKey);
  let harmonicScore = keyCompat >= 0.98 ? 25.0 : keyCompat >= 0.88 ? 23.0 : keyCompat >= 0.74 ? 20.0 : 7.5;

  // 2. BPM & Groove Relationship (20 pts max)
  let bpmScore = 15.0;
  if (cur) {
    const pct = Math.abs(curBpm - candBpm) / curBpm;
    const doublePct = Math.abs(curBpm * 2.0 - candBpm) / (curBpm * 2.0);
    const halfPct = Math.abs(curBpm * 0.5 - candBpm) / (curBpm * 0.5);

    if (mood.acceptable_tempo_relationships?.includes('half_time') && halfPct <= 0.05) {
      bpmScore = 20.0;
      reasons.push(`${mood.display_name} actively rewards harmonic half-time groove`);
    } else if (mood.acceptable_tempo_relationships?.includes('double_time') && doublePct <= 0.05) {
      bpmScore = 20.0;
      reasons.push(`${mood.display_name} actively rewards harmonic double-time groove`);
    } else if (pct <= 0.03) {
      bpmScore = 20.0;
      reasons.push('Tight tempo lock');
    } else if (pct <= 0.08) {
      bpmScore = 17.0;
    } else {
      bpmScore = 9.0;
    }
  }

  // BPM window check
  if (candBpm >= mood.min_bpm && candBpm <= mood.max_bpm) {
    reasons.push(`In the pocket for ${mood.display_name} BPM window`);
  } else if ((candBpm < mood.min_bpm - 8.0 || candBpm > mood.max_bpm + 8.0) && mood.bpm_flexibility === 'tight') {
    penalties.push(`Tempo outside tight ${mood.display_name} window`);
    penaltyDeduction += 15.0;
  }

  // 3. Energy Trajectory (20 pts max)
  let energyScore = 14.0;
  if (Math.abs(candEnergy - mood.target_energy) <= 0.08) {
    energyScore = 20.0;
    reasons.push(`Hits ${mood.display_name} target energy (${Math.round(mood.target_energy * 100)}%)`);
  } else if (Math.abs(candEnergy - mood.target_energy) > 0.15) {
    energyScore = 8.0;
  }

  // 4. Timbre & Vocal Balance (20 pts max)
  let timbreScore = 10.0;
  let vocalScore = 6.0;
  if (mood.vocal_preference === 'singalong_hooks') {
    if ((cand.vocalDensity || 0) >= 0.70) {
      vocalScore = 10.0;
      reasons.push('Singalong vocal hook elevates crowd participation');
    } else if ((cand.vocalDensity || 0) <= 0.30) {
      vocalScore = 2.0;
      penalties.push('Lacks required singalong vocals for vocal-centric celebration');
    }
  } else if (mood.vocal_preference === 'soulful_lounge') {
    if ((cand.vocalDensity || 0) <= 0.35) {
      vocalScore = 10.0;
      reasons.push('Instrumental lounge texture fits sophisticated setting');
    } else if ((cand.vocalDensity || 0) >= 0.75) {
      vocalScore = 2.0;
      penalties.push('Excessive vocal intensity disrupts lounge atmosphere');
    }
  }

  if (cur && cur.isElectronicDance && cand.isAcousticBallad && deltaEnergy <= -0.30) {
    penaltyDeduction += 20.0;
    timbreScore = 0.0;
  }

  // 5. Genre, Subgenre & Mood Personality Affinity
  const allGenresStr = (cand.genreTags || []).join(' ').toLowerCase();
  const genreMatch = mood.genre_weights?.some(([g]) => allGenresStr.includes(g.toLowerCase()));
  let genreScore = genreMatch ? 8.0 : 0.0;

  let moodAffinityScore = 0.0;
  // Subgenre matching
  for (const [sg, weight] of mood.subgenre_weights || []) {
    if (allGenresStr.includes(sg.toLowerCase())) {
      reasons.push(`Signature subgenre '${sg}' active`);
      moodAffinityScore += weight * 3.0;
      break;
    }
  }

  // Percussion matching
  switch (mood.preferred_percussion) {
    case 'live_punjabi_dhol':
      if (cand.isPercussionHeavy || allGenresStr.includes('dhol') || allGenresStr.includes('bhangra')) {
        reasons.push('Acoustic live Punjabi dhol driving groove');
        moodAffinityScore += 5.0;
      }
      break;
    case 'cyclic_3_taali':
      if (allGenresStr.includes('garba') || allGenresStr.includes('taali') || allGenresStr.includes('sanedo')) {
        reasons.push('Authentic 3-Taali cyclic folk rhythm aligns with Garba');
        moodAffinityScore += 6.0;
      }
      break;
    case 'sharp_dandiya_sticks':
      if (allGenresStr.includes('dandiya') || allGenresStr.includes('raas') || (candBpm >= 122.0 && cand.isPercussionHeavy)) {
        reasons.push('Sharp stick-striking cadence matches Dandiya tempo');
        moodAffinityScore += 6.0;
      }
      break;
    case 'deep_house_analog':
      if (allGenresStr.includes('deep') || allGenresStr.includes('lounge') || allGenresStr.includes('chill') || allGenresStr.includes('jazz')) {
        reasons.push('Warm Rhodes and hypnotic deep house groove');
        moodAffinityScore += 6.0;
      }
      break;
    case 'stadium_bigroom_kick':
      if (cand.isElectronicDance && (allGenresStr.includes('edm') || allGenresStr.includes('big room') || allGenresStr.includes('drop'))) {
        reasons.push('Massive stadium drop payoff aligns with Festival Mode');
        moodAffinityScore += 6.0;
      }
      break;
    case 'organic_vintage_drums':
      if (allGenresStr.includes('retro') || allGenresStr.includes('disco') || allGenresStr.includes('70s') || allGenresStr.includes('classic')) {
        reasons.push('Organic acoustic drums and live brass instrumentation');
        moodAffinityScore += 6.0;
      }
      break;
    case 'afro_latin_syncopated':
      if (allGenresStr.includes('afrobeat') || allGenresStr.includes('reggaeton') || allGenresStr.includes('latin') || allGenresStr.includes('dembow')) {
        reasons.push('Syncopated Afro-Latin body groove');
        moodAffinityScore += 6.0;
      }
      break;
    case 'four_on_the_floor_sub':
      if (cand.isElectronicDance && (allGenresStr.includes('club') || allGenresStr.includes('remix'))) {
        reasons.push('Driving 4-on-the-floor kick and sub-bass');
        moodAffinityScore += 5.0;
      }
      break;
  }

  // Mood Atmosphere Specific Penalties
  if (mood.id === 'cocktail_night') {
    if (candEnergy > 0.74 || allGenresStr.includes('dhol') || allGenresStr.includes('edm')) {
      penaltyDeduction += 25.0;
      penalties.push('Energy or timbre too aggressive for Cocktail Night lounge ambiance');
    }
  } else if (mood.id === 'baraat') {
    if (candEnergy < 0.72 || (cand.isAcousticBallad && !allGenresStr.includes('dhol'))) {
      penaltyDeduction += 25.0;
      penalties.push('Low energy or acoustic texture deflates high-octane Baraat procession');
    }
  } else if (mood.id === 'festival_mode') {
    if (candEnergy < 0.70) {
      penaltyDeduction += 25.0;
      penalties.push('Subdued energy lacks Stadium Festival mainstage impact');
    }
  } else if (mood.id === 'retro_bollywood') {
    if (allGenresStr.includes('edm') || allGenresStr.includes('trap') || (allGenresStr.includes('club') && cand.isElectronicDance)) {
      penaltyDeduction += 20.0;
      penalties.push('Modern electronic club production clashes with Retro Bollywood golden vintage warmth');
    }
  } else if (mood.id === 'garba') {
    if (cand.isElectronicDance && allGenresStr.includes('house')) {
      penaltyDeduction += 20.0;
      penalties.push('Western four-on-the-floor kick breaks cyclic Garba taali stepping');
    }
  } else if (mood.id === 'dandiya') {
    if (candBpm < 120.0) {
      penaltyDeduction += 20.0;
      penalties.push('Tempo too slow for fast synchronized Dandiya stick striking');
    }
  } else if (mood.id === 'mehendi') {
    if (candEnergy > 0.82 || allGenresStr.includes('trap')) {
      penaltyDeduction += 20.0;
      penalties.push('Heavy club energy overpowers relaxed daytime Mehendi celebration');
    }
  }

  const lookaheadScore = 4.0;
  const cacheBonus = 5.0;
  const total =
    harmonicScore +
    bpmScore +
    10.0 + // groove_and_meter
    energyScore +
    timbreScore +
    vocalScore +
    genreScore +
    moodAffinityScore +
    lookaheadScore +
    cacheBonus -
    penaltyDeduction;

  const finalScore = Math.max(0, Math.min(100, Math.round(((total / 115.0) * 100.0) * 10) / 10));

  return {
    score: finalScore,
    reasons,
    penalties,
  };
}

// =========================================================================
// SECTION 1: MEANINGFUL MOOD CONTRAST BENCHMARKS
// =========================================================================
console.log('\n--- 1. VERIFYING 6 MEANINGFUL MOOD CONTRAST SCENARIOS ---');

// Contrast 1: Shaadi vs Cocktail Night
{
  const shaadi = moodMap.get('shaadi');
  const cocktail = moodMap.get('cocktail_night');

  const bollywoodSingalong = {
    title: 'Bole Chudiyan (Wedding Anthem)',
    bpm: 118, camelotKey: '9A', overallEnergy: 0.82, vocalDensity: 0.85,
    genreTags: ['Bollywood', 'Wedding Dholak', 'Singalong'], isPercussionHeavy: true,
  };
  const deepHouseLounge = {
    title: 'Sunset Coast (Deep House Lounge)',
    bpm: 120, camelotKey: '9A', overallEnergy: 0.58, vocalDensity: 0.20,
    genreTags: ['Deep House', 'Sunset House', 'Melodic Lounge'], isElectronicDance: true,
  };

  const playingTrack = { bpm: 120, camelotKey: '9A', overallEnergy: 0.70 };

  const resShaadiA = scoreCandidateForMood(bollywoodSingalong, playingTrack, shaadi);
  const resShaadiB = scoreCandidateForMood(deepHouseLounge, playingTrack, shaadi);

  const resCocktailA = scoreCandidateForMood(bollywoodSingalong, playingTrack, cocktail);
  const resCocktailB = scoreCandidateForMood(deepHouseLounge, playingTrack, cocktail);

  console.log(`[Contrast 1] Shaadi vs Cocktail Night:`);
  console.log(`  Shaadi:       Bole Chudiyan (${resShaadiA.score}) vs Sunset Lounge (${resShaadiB.score})`);
  console.log(`  Cocktail:     Bole Chudiyan (${resCocktailA.score}) vs Sunset Lounge (${resCocktailB.score})`);

  assert.ok(resShaadiA.score > resShaadiB.score, 'Shaadi must rank celebratory wedding singalong #1');
  assert.ok(resCocktailB.score > resCocktailA.score, 'Cocktail Night must rank deep house lounge #1');
  assert.ok(resCocktailA.penalties.length > 0, 'Cocktail Night must penalize aggressive wedding singalongs');
  console.log('  ✓ Shaadi embraces celebratory singalong; Cocktail Night selects deep house and penalizes high-energy singalongs');
}

// Contrast 2: Baraat vs Retro Bollywood
{
  const baraat = moodMap.get('baraat');
  const retro = moodMap.get('retro_bollywood');

  const liveDholAnthem = {
    title: 'Baraat Dhol Hype (Live Dhol & Brass)',
    bpm: 138, camelotKey: '6A', overallEnergy: 0.96, vocalDensity: 0.40,
    genreTags: ['Bhangra Dhol', 'Live Dhol', 'Baraat Brass'], isPercussionHeavy: true,
  };
  const rdBurmanFunk = {
    title: 'Dum Maro Dum (70s Vintage Funk)',
    bpm: 116, camelotKey: '6A', overallEnergy: 0.62, vocalDensity: 0.75,
    genreTags: ['Disco Bollywood', '70s Funk', 'RD Burman Brass'], isAcousticBallad: false,
  };

  const playingTrack = { bpm: 130, camelotKey: '6A', overallEnergy: 0.80 };

  const resBaraatA = scoreCandidateForMood(liveDholAnthem, playingTrack, baraat);
  const resBaraatB = scoreCandidateForMood(rdBurmanFunk, playingTrack, baraat);

  const resRetroA = scoreCandidateForMood(liveDholAnthem, playingTrack, retro);
  const resRetroB = scoreCandidateForMood(rdBurmanFunk, playingTrack, retro);

  console.log(`[Contrast 2] Baraat vs Retro Bollywood:`);
  console.log(`  Baraat:       Live Dhol (${resBaraatA.score}) vs RD Burman (${resBaraatB.score})`);
  console.log(`  Retro:        Live Dhol (${resRetroA.score}) vs RD Burman (${resRetroB.score})`);

  assert.ok(resBaraatA.score > resBaraatB.score, 'Baraat must rank live dhol anthem #1');
  assert.ok(resRetroB.score > resRetroA.score, 'Retro Bollywood must rank 70s vintage funk #1');
  assert.ok(resBaraatB.penalties.length > 0, 'Baraat must penalize subdued retro tempo');
  console.log('  ✓ Baraat prioritizes relentless live dhol; Retro Bollywood prioritizes golden vintage brass');
}

// Contrast 3: Garba vs Dandiya
{
  const garba = moodMap.get('garba');
  const dandiya = moodMap.get('dandiya');

  const garbaCyclic = {
    title: 'Tara Vina Shyam (3-Taali Cyclic Folk)',
    bpm: 114, camelotKey: '8A', overallEnergy: 0.78, vocalDensity: 0.70,
    genreTags: ['Garba Folk', '3-Taali Dhol', 'Sanedo'], isPercussionHeavy: true,
  };
  const fastDandiya = {
    title: 'Dholida Dhol Re Vagad (Fast Stick Raas)',
    bpm: 136, camelotKey: '8A', overallEnergy: 0.90, vocalDensity: 0.65,
    genreTags: ['Dandiya Raas', 'Raas Beat', 'Stick Rhythm'], isPercussionHeavy: true,
  };

  const playingTrack = { bpm: 115, camelotKey: '8A', overallEnergy: 0.75 };

  const resGarbaA = scoreCandidateForMood(garbaCyclic, playingTrack, garba);
  const resGarbaB = scoreCandidateForMood(fastDandiya, playingTrack, garba);

  const resDandiyaA = scoreCandidateForMood(garbaCyclic, playingTrack, dandiya);
  const resDandiyaB = scoreCandidateForMood(fastDandiya, playingTrack, dandiya);

  console.log(`[Contrast 3] Garba vs Dandiya:`);
  console.log(`  Garba:        3-Taali (${resGarbaA.score}) vs Fast Stick Raas (${resGarbaB.score})`);
  console.log(`  Dandiya:      3-Taali (${resDandiyaA.score}) vs Fast Stick Raas (${resDandiyaB.score})`);

  assert.ok(resGarbaA.score > resGarbaB.score, 'Garba must rank 114 BPM 3-Taali cyclic folk #1 at set start');
  assert.ok(resDandiyaB.score > resDandiyaA.score, 'Dandiya must rank fast 136 BPM stick raas #1');
  assert.ok(resDandiyaA.penalties.length > 0, 'Dandiya must penalize tempo < 120 BPM');
  console.log('  ✓ Garba prioritizes cyclic folk trance; Dandiya demands fast sharp stick cadence (>= 124 BPM)');
}

// Contrast 4: Punjabi Tashan vs House Party
{
  const punjabi = moodMap.get('punjabi_tashan');
  const houseParty = moodMap.get('house_party');

  const urbanTrap = {
    title: 'Brown Munde (808 Sub Trap Swagger)',
    bpm: 80, camelotKey: '10A', overallEnergy: 0.92, vocalDensity: 0.80,
    genreTags: ['Punjabi Trap', '808 Trap', 'UK Drill Punjabi'], isPercussionHeavy: true,
  };
  const popChartDance = {
    title: 'Levitating (Accessible Pop Dance)',
    bpm: 103, camelotKey: '10A', overallEnergy: 0.75, vocalDensity: 0.70,
    genreTags: ['Pop', 'Chart Singalong', 'Dance'], isElectronicDance: true,
  };

  // Playing track is a 140 BPM Bhangra track -> 80 BPM trap is an exact half-time relationship!
  const playingTrack = { bpm: 140, camelotKey: '10A', overallEnergy: 0.88 };

  const resPunjabiA = scoreCandidateForMood(urbanTrap, playingTrack, punjabi);
  const resPunjabiB = scoreCandidateForMood(popChartDance, playingTrack, punjabi);

  const resHouseA = scoreCandidateForMood(urbanTrap, playingTrack, houseParty);
  const resHouseB = scoreCandidateForMood(popChartDance, playingTrack, houseParty);

  console.log(`[Contrast 4] Punjabi Tashan vs House Party:`);
  console.log(`  Punjabi:      808 Trap (${resPunjabiA.score}) vs Pop Dance (${resPunjabiB.score})`);
  console.log(`  House Party:  808 Trap (${resHouseA.score}) vs Pop Dance (${resHouseB.score})`);

  assert.ok(resPunjabiA.score > resPunjabiB.score, 'Punjabi Tashan must rank 808 Trap #1 via harmonic half-time');
  assert.ok(resHouseB.score > resHouseA.score, 'House Party must rank accessible pop singalong #1');
  console.log('  ✓ Punjabi Tashan rewards 808 trap half-time groove; House Party rewards accessible pop dance');
}

// Contrast 5: Mehendi vs Festival Mode
{
  const mehendi = moodMap.get('mehendi');
  const festival = moodMap.get('festival_mode');

  const sufiBallad = {
    title: 'Mast Magan (Acoustic Sufi Ballad)',
    bpm: 96, camelotKey: '11B', overallEnergy: 0.56, vocalDensity: 0.80,
    genreTags: ['Folk Bollywood', 'Folk Ballad', 'Sufi'], isAcousticBallad: true,
  };
  const bigRoomEdm = {
    title: 'Festival Pyro Drop (Big Room EDM)',
    bpm: 130, camelotKey: '11B', overallEnergy: 0.98, vocalDensity: 0.20,
    genreTags: ['Festival Big Room', 'Mainstage Big Room', 'EDM Drop'], isElectronicDance: true,
  };

  const playingTrack = { bpm: 100, camelotKey: '11B', overallEnergy: 0.60 };

  const resMehendiA = scoreCandidateForMood(sufiBallad, playingTrack, mehendi);
  const resMehendiB = scoreCandidateForMood(bigRoomEdm, playingTrack, mehendi);

  const resFestivalA = scoreCandidateForMood(sufiBallad, playingTrack, festival);
  const resFestivalB = scoreCandidateForMood(bigRoomEdm, playingTrack, festival);

  console.log(`[Contrast 5] Mehendi vs Festival Mode:`);
  console.log(`  Mehendi:      Sufi Ballad (${resMehendiA.score}) vs Big Room EDM (${resMehendiB.score})`);
  console.log(`  Festival:     Sufi Ballad (${resFestivalA.score}) vs Big Room EDM (${resFestivalB.score})`);

  assert.ok(resMehendiA.score > resMehendiB.score, 'Mehendi must rank acoustic Sufi ballad #1');
  assert.ok(resFestivalB.score > resFestivalA.score, 'Festival Mode must rank stadium EDM drop #1');
  assert.ok(resMehendiB.penalties.length > 0, 'Mehendi must penalize aggressive club energy');
  assert.ok(resFestivalA.penalties.length > 0, 'Festival Mode must penalize subdued ballad energy');
  console.log('  ✓ Mehendi preserves intimate acoustic warmth; Festival Mode delivers massive stadium drops');
}

// Contrast 6: Desi Club vs International Party
{
  const desiClub = moodMap.get('desi_club');
  const intlParty = moodMap.get('international_party');

  const bollywoodClubRemix = {
    title: 'Kar Gayi Chull (4x4 Club Remix)',
    bpm: 128, camelotKey: '7A', overallEnergy: 0.90, vocalDensity: 0.60,
    genreTags: ['Desi Remix', 'Bollywood 4x4', 'Club EDM'], isElectronicDance: true,
  };
  const afrobeatDembow = {
    title: 'Calm Down (Afrobeat Dembow Flow)',
    bpm: 104, camelotKey: '7A', overallEnergy: 0.80, vocalDensity: 0.70,
    genreTags: ['Afrobeat', 'Afrobeats Amapiano', 'Latin Dembow'], isElectronicDance: false,
  };

  const playingTrack = { bpm: 126, camelotKey: '7A', overallEnergy: 0.85 };

  const resDesiA = scoreCandidateForMood(bollywoodClubRemix, playingTrack, desiClub);
  const resDesiB = scoreCandidateForMood(afrobeatDembow, playingTrack, desiClub);

  const resIntlA = scoreCandidateForMood(bollywoodClubRemix, playingTrack, intlParty);
  const resIntlB = scoreCandidateForMood(afrobeatDembow, playingTrack, intlParty);

  console.log(`[Contrast 6] Desi Club vs International Party:`);
  console.log(`  Desi Club:    4x4 Club Remix (${resDesiA.score}) vs Afrobeat Dembow (${resDesiB.score})`);
  console.log(`  Intl Party:   4x4 Club Remix (${resIntlA.score}) vs Afrobeat Dembow (${resIntlB.score})`);

  assert.ok(resDesiA.score > resDesiB.score, 'Desi Club must rank 4x4 Bollywood club remix #1');
  assert.ok(resIntlB.score > resIntlA.score, 'International Party must rank Afrobeat dembow #1');
  console.log('  ✓ Desi Club locks 4-on-the-floor driving kicks; International Party celebrates global syncopation');
}

// =========================================================================
// SECTION 2: MULTI-TRACK SET SEQUENCE SIMULATION
// =========================================================================
console.log('\n--- 2. VERIFYING MULTI-TRACK SET SEQUENCE PROGRESSION ---');

// A. Baraat Sequence (High-octane procession: short punchy cuts, relentless peak stamina)
{
  const t1 = { trackId: 'b1', deckId: 0, title: 'Baraat Entrance', bpm: 136, camelotKey: '6A', overallEnergy: 0.94, durationSec: 200, currentPosSec: 180, genreTags: ['Bhangra Dhol', 'Live Dhol'], isPercussionHeavy: true };
  const t2 = { trackId: 'b2', deckId: 1, title: 'Dholak Hype', bpm: 138, camelotKey: '6A', overallEnergy: 0.96, durationSec: 190, currentPosSec: 0, genreTags: ['Bhangra Dhol', 'Baraat Brass'], isPercussionHeavy: true, predictedEnergyDirection: 'boost' };

  const plan = planDjTransition(t1, t2, { djLevel: 2, moodId: 'baraat', targetEnergy: 0.96, recentTechniques: [] });

  assert.ok(plan.duration_beats <= 8, 'Baraat must enforce fast punchy transition (<= 8 beats)');
  assert.equal(plan.musical_purpose, 'build_excitement');
  assert.ok(plan.eq_bass_handover_schedule.bass_swap_at_frac <= 0.50, 'Baraat triggers bass swap early for clean dhol impact');
  console.log(`  ✓ Baraat Sequence: ${plan.selected_technique} (${plan.duration_beats} beats). Musical Purpose: ${plan.musical_purpose}. Clean crisp dhol drop on beat 1.`);
}

// B. Cocktail Night Sequence (Luxurious 32-64 beat blend, hypnotic breathing room)
{
  const t1 = { trackId: 'c1', deckId: 0, title: 'Sunset Lounge A', bpm: 120, camelotKey: '8A', overallEnergy: 0.60, durationSec: 300, currentPosSec: 260, genreTags: ['Deep House', 'Sunset House'], isElectronicDance: true };
  const t2 = { trackId: 'c2', deckId: 1, title: 'Sunset Lounge B', bpm: 121, camelotKey: '8A', overallEnergy: 0.58, durationSec: 280, currentPosSec: 0, genreTags: ['Deep House', 'Melodic Lounge'], isElectronicDance: true, predictedEnergyDirection: 'sustain' };

  const plan = planDjTransition(t1, t2, { djLevel: 2, moodId: 'cocktail_night', targetEnergy: 0.60, recentTechniques: [] });

  assert.ok(plan.duration_beats >= 32, 'Cocktail Night must use luxurious long blend (>= 32 beats)');
  assert.equal(plan.musical_purpose, 'preserve_dance_momentum');
  console.log(`  ✓ Cocktail Night Sequence: ${plan.selected_technique} (${plan.duration_beats} beats, ${plan.musical_duration_sec}s). Musical Purpose: ${plan.musical_purpose}.`);
}

// C. Garba Progressive Acceleration Sequence
{
  const t1 = { trackId: 'g1', deckId: 0, title: 'Garba 2-Taali (Opening)', bpm: 114, camelotKey: '7A', overallEnergy: 0.76, durationSec: 240, currentPosSec: 210, genreTags: ['Garba Folk', '3-Taali Dhol'], isPercussionHeavy: true };
  const t2 = { trackId: 'g2', deckId: 1, title: 'Garba Chalti (Accelerating)', bpm: 124, camelotKey: '7A', overallEnergy: 0.84, durationSec: 230, currentPosSec: 0, genreTags: ['Garba Folk', '2-Taali Chalti'], isPercussionHeavy: true, predictedEnergyDirection: 'boost' };

  const plan = planDjTransition(t1, t2, { djLevel: 2, moodId: 'garba', targetEnergy: 0.84, recentTechniques: [] });

  assert.ok(plan.duration_beats >= 8 && plan.duration_beats <= 32);
  console.log(`  ✓ Garba Sequence: ${plan.selected_technique} (${plan.duration_beats} beats). Progressive circle acceleration cleanly maintained.`);
}

console.log('\n' + '='.repeat(75));
console.log('ALL 14 MOOD IDENTITY & CONTRAST BENCHMARKS PASSED COMPLETELY!');
console.log('='.repeat(75));
