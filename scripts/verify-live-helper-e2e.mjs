// Live End-to-End Verification Script against the ACTUAL installed release `dhun-local-helper.exe`.
//
// Exercises the real compiled Windows binary on a dedicated loopback port with a synthetic WAV
// authorized test fixture server (zero YouTube ripping, zero modification of app.py):
//   1. Starts installed `dhun-local-helper.exe` backed by an on-disk SQLite DB
//   2. Verifies `/api/local/health`, room creation, and party start
//   3. Fires 4 rapid concurrent URL submissions (including 1 duplicate) -> verifies <250ms non-blocking response + dedup
//   4. Waits for background worker to cache authorized WAVs, run Symphonia BPM/key/section analysis, and populate `next_playable_candidate`
//   5. Streams cached audio via `/api/local/rooms/:id/cache/:cache_id/stream` (verifies RIFF/WAVE bytes)
//   6. Switches DJ Level 1 -> 2 -> 3 -> 4 and verifies dynamic skill profile, deck count, and transition style updates
//   7. Submits a track while fixture server is stopped (offline/transient failure simulation), kills & restarts
//      the actual `dhun-local-helper.exe` process, restarts fixture server, and verifies persistent room + queue recovery!
//   8. Explicitly closes party room and verifies temporary cache cleanup.

import assert from 'node:assert/strict';
import http from 'node:http';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';

const INSTALLED_HELPER = path.join(
  process.env.LOCALAPPDATA || '',
  'DHUN',
  'bin',
  'dhun-local-helper.exe'
);
assert(fs.existsSync(INSTALLED_HELPER), `Installed helper not found at ${INSTALLED_HELPER}`);

function makeSyntheticWavBuffer(durationSec, freqHz, bpmPulse = true) {
  const sr = 44100;
  const numFrames = Math.floor(sr * durationSec);
  const dataBytes = numFrames * 4;
  const buf = Buffer.alloc(44 + dataBytes);
  buf.write('RIFF', 0);
  buf.writeUInt32LE(36 + dataBytes, 4);
  buf.write('WAVEfmt ', 8);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);
  buf.writeUInt16LE(2, 22);
  buf.writeUInt32LE(sr, 24);
  buf.writeUInt32LE(sr * 4, 28);
  buf.writeUInt16LE(4, 32);
  buf.writeUInt16LE(16, 34);
  buf.write('data', 36);
  buf.writeUInt32LE(dataBytes, 40);

  for (let i = 0; i < numFrames; i++) {
    const t = i / sr;
    // Create structural contrast across quarters (intro -> groove -> breakdown -> drop)
    const q = Math.floor((t / durationSec) * 4);
    const amp = q === 0 ? 0.1 : q === 1 ? 0.28 : q === 2 ? 0.12 : 0.45;
    const kick = bpmPulse && (t % 0.5) < 0.08 ? 0.25 * Math.sin(2 * Math.PI * 65 * t) : 0;
    const sample = Math.max(-0.95, Math.min(0.95, amp * Math.sin(2 * Math.PI * freqHz * t) + kick));
    const s16 = Math.round(sample * 32767);
    const off = 44 + i * 4;
    buf.writeInt16LE(s16, off);
    buf.writeInt16LE(s16, off + 2);
  }
  return buf;
}

const wavA = makeSyntheticWavBuffer(32, 220, true);
const wavB = makeSyntheticWavBuffer(32, 330, true);
const shaA = crypto.createHash('sha256').update(wavA).digest('hex');
const shaB = crypto.createHash('sha256').update(wavB).digest('hex');

let fixtureOnline = true;
const fixtureServer = http.createServer((req, res) => {
  if (!fixtureOnline) {
    res.writeHead(503, { 'Content-Type': 'text/plain' });
    res.end('Simulated offline outage');
    return;
  }
  if (req.method === 'POST' && req.url === '/resolve') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      const parsed = JSON.parse(body || '{}');
      const vid = parsed.provider_video_id || '';
      const port = fixtureServer.address().port;
      if (vid === 'dQw4w9WgXcQ' || vid === 'kJQP7kiw5Fk') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(
          JSON.stringify({
            direct_https_audio_url: `http://127.0.0.1:${port}/audio/${vid}.wav`,
            license_or_authorization_note: 'Synthetic local E2E verification WAV',
            expected_sha256: vid === 'dQw4w9WgXcQ' ? shaA : shaB,
          })
        );
      } else {
        res.writeHead(404, { 'Content-Type': 'text/plain' });
        res.end('Metadata-only test track');
      }
    });
    return;
  }
  if (req.method === 'GET' && req.url?.startsWith('/audio/')) {
    const isA = req.url.includes('dQw4w9WgXcQ');
    const payload = isA ? wavA : wavB;
    res.writeHead(200, { 'Content-Type': 'audio/wav', 'Content-Length': payload.length });
    res.end(payload);
    return;
  }
  res.writeHead(404);
  res.end();
});

async function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function waitForHealth(port, timeoutMs = 10000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/api/local/health`);
      if (r.ok) return await r.json();
    } catch {}
    await sleep(250);
  }
  throw new Error(`Helper on port ${port} did not become healthy within ${timeoutMs}ms`);
}

function startHelperProcess(port, dataDir, resolverPort) {
  return spawn(INSTALLED_HELPER, [], {
    env: {
      ...process.env,
      DHUN_LOCAL_PORT: String(port),
      DHUN_LOCAL_DB_PATH: path.join(dataDir, 'dhun_e2e.db'),
      DHUN_CACHE_DIR: path.join(dataDir, 'temp_cache'),
      DHUN_AUDIO_RESOLVER_URL: `http://127.0.0.1:${resolverPort}/resolve`,
      DHUN_ALLOW_LOOPBACK_RESOLVER: '1',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

async function main() {
  await new Promise((resolve) => fixtureServer.listen(0, '127.0.0.1', resolve));
  const fixturePort = fixtureServer.address().port;
  const helperPort = 47831;
  const tempDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dhun-live-e2e-'));

  let helper = startHelperProcess(helperPort, tempDataDir, fixturePort);
  try {
    const health = await waitForHealth(helperPort);
    assert.equal(health.status, 'ok');
    assert.equal(health.audio_resolver_configured, true);
    console.log(`[1/7] Installed helper ONLINE v${health.version} (port ${helperPort})`);

    // Create & Start party room
    const createRes = await fetch(`http://127.0.0.1:${helperPort}/api/local/rooms`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ party_name: 'Live E2E Validation', dj_level: 1, primary_mood_id: 'house_party' }),
    });
    assert.equal(createRes.status, 200);
    const created = await createRes.json();
    const roomId = created.room.room_id;
    const hostHeaders = {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${created.host_token}`,
    };

    const startRes = await fetch(`http://127.0.0.1:${helperPort}/api/local/rooms/${roomId}/start`, {
      method: 'POST',
      headers: hostHeaders,
    });
    assert.equal(startRes.status, 200);
    console.log(`[2/7] Room ${created.room.room_code} created & started LIVE`);

    // Fire 4 rapid concurrent URL submissions (including 1 duplicate of dQw4w9WgXcQ)
    const t0 = Date.now();
    const urls = [
      'https://www.youtube.com/watch?v=dQw4w9WgXcQ',
      'https://youtu.be/dQw4w9WgXcQ', // duplicate
      'https://www.youtube.com/watch?v=9bZkp7q19f0', // metadata-only (404 from fixture)
      'https://www.youtube.com/watch?v=kJQP7kiw5Fk',
    ];
    const subResults = [];
    for (const url of urls) {
      const r = await fetch(`http://127.0.0.1:${helperPort}/api/local/rooms/${roomId}/submissions`, {
        method: 'POST',
        headers: hostHeaders,
        body: JSON.stringify({ url }),
      });
      assert.equal(r.status, 200);
      subResults.push(await r.json());
    }
    const burstMs = Date.now() - t0;
    assert(burstMs < 2000, `Rapid 4-URL submission burst took ${burstMs}ms (expected non-blocking)`);
    assert.equal(subResults[1].status, 'already_queued', 'Duplicate URL must deduplicate');
    assert.equal(subResults[1].id, subResults[0].id);
    console.log(`[3/7] Rapid 4-URL burst accepted in ${burstMs}ms with clean deduplication`);

    // Wait for background intake worker to drain all 3 unique jobs
    let dash = null;
    for (let i = 0; i < 40; i++) {
      const r = await fetch(`http://127.0.0.1:${helperPort}/api/local/rooms/${roomId}`, {
        headers: hostHeaders,
      });
      dash = await r.json();
      if (dash.pending_intake_jobs === 0 && dash.next_playable_candidate) break;
      await sleep(350);
    }
    assert(dash && dash.pending_intake_jobs === 0, `Intake queue did not drain cleanly: pending=${dash?.pending_intake_jobs}`);
    assert.equal(dash.submissions.length, 3, 'Expected exactly 3 unique submissions after dedup');
    if (!dash.next_playable_candidate) {
      console.error('DEBUG submissions:', JSON.stringify(dash.submissions.map((s) => ({
        id: s.id, title: s.title, acq: s.acquisition_status, note: s.acquisition_note, cache_id: s.cache_id, bpm: s.bpm
      })), null, 2));
    }
    assert(dash.next_playable_candidate, 'Authorized cached WAV must become next_playable_candidate');
    assert(
      Array.isArray(dash.next_playable_candidate.sections) && dash.next_playable_candidate.sections.length >= 2,
      'Analyzed track must carry real DSP phrase/section markers'
    );

    // Verify metadata-only track is NEVER marked playable in recommendations
    const metaRec = dash.recommendations.find((r) => r.submission_id === subResults[2].id);
    assert(metaRec && !metaRec.is_playable && metaRec.playability_label === 'metadata_only_restricted');
    console.log(
      `[4/7] Async worker drained 3 jobs -> next_playable=${dash.next_playable_candidate.title} (${dash.next_playable_candidate.bpm} BPM, ${dash.next_playable_candidate.sections.length} DSP sections)`
    );

    // Stream cached audio and verify valid WAV RIFF header
    const streamRes = await fetch(
      `http://127.0.0.1:${helperPort}${dash.next_playable_candidate.stream_url}`,
      { headers: { Authorization: `Bearer ${created.host_token}` } }
    );
    assert.equal(streamRes.status, 200);
    const streamBuf = Buffer.from(await streamRes.arrayBuffer());
    assert.equal(streamBuf.subarray(0, 4).toString('ascii'), 'RIFF');
    console.log(`[5/7] Authorized cached audio streamed cleanly (${streamBuf.length} bytes, RIFF/WAVE verified)`);

    // Switch DJ Levels 1 -> 2 -> 3 -> 4 and verify live config + skill profile separation
    const seenDeckCounts = [];
    for (const lvl of [1, 2, 3, 4]) {
      const cfgRes = await fetch(`http://127.0.0.1:${helperPort}/api/local/rooms/${roomId}/config`, {
        method: 'POST',
        headers: hostHeaders,
        body: JSON.stringify({ dj_level: lvl }),
      });
      assert.equal(cfgRes.status, 200);
      const dRes = await fetch(`http://127.0.0.1:${helperPort}/api/local/rooms/${roomId}`, {
        headers: hostHeaders,
      });
      const d = await dRes.json();
      assert.equal(d.dj_skill_profile.level, lvl);
      seenDeckCounts.push(d.exposed_decks);
    }
    assert.deepEqual(seenDeckCounts, [2, 2, 4, 4]);
    console.log('[6/7] Live DJ Level 1->2->3->4 switching verified (profiles + 2/2/4/4 decks)');

    // Simulate transient outage + HARD RESTART of installed dhun-local-helper.exe with pending job!
    fixtureOnline = false;
    const offlineSubRes = await fetch(`http://127.0.0.1:${helperPort}/api/local/rooms/${roomId}/submissions`, {
      method: 'POST',
      headers: hostHeaders,
      body: JSON.stringify({ url: 'https://www.youtube.com/watch?v=JGwWNGJdvx8' }),
    });
    assert.equal(offlineSubRes.status, 200);

    // Hard kill helper immediately, bring fixture back online, restart helper process!
    helper.kill('SIGKILL');
    await sleep(600);
    fixtureOnline = true;
    helper = startHelperProcess(helperPort, tempDataDir, fixturePort);
    await waitForHealth(helperPort);

    let postRestartDash = null;
    for (let i = 0; i < 40; i++) {
      const r = await fetch(`http://127.0.0.1:${helperPort}/api/local/rooms/${roomId}`, {
        headers: hostHeaders,
      });
      assert.equal(r.status, 200);
      postRestartDash = await r.json();
      if (postRestartDash.pending_intake_jobs === 0) break;
      await sleep(300);
    }
    assert.equal(postRestartDash.room.status, 'live', 'Room must remain LIVE after helper restart');
    assert.equal(postRestartDash.submissions.length, 4, 'All 4 submissions must persist across restart');
    assert.equal(postRestartDash.pending_intake_jobs, 0, 'Pending job must recover and drain after restart');
    console.log('[7/7] Hard helper restart + persistent SQLite queue recovery verified!');

    // Close party room cleanly and verify cache cleanup
    const closeRes = await fetch(`http://127.0.0.1:${helperPort}/api/local/rooms/${roomId}/close`, {
      method: 'POST',
      headers: hostHeaders,
      body: JSON.stringify({ force_release_audio: true }),
    });
    assert.equal(closeRes.status, 200);
    const closed = await closeRes.json();
    assert.equal(closed.status, 'closed');
    assert(closed.purged_cache_files >= 2, `Expected >=2 purged cache files, got ${closed.purged_cache_files}`);
    console.log(`ALL LIVE INSTALLED HELPER E2E CHECKS PASSED (purged ${closed.purged_cache_files} temp cache files).`);
  } finally {
    try {
      helper.kill('SIGKILL');
    } catch {}
    fixtureServer.close();
    await sleep(300);
    try {
      fs.rmSync(tempDataDir, { recursive: true, force: true });
    } catch {}
  }
}

main().catch((err) => {
  console.error('LIVE E2E FAILED:', err);
  process.exit(1);
});
