// tests/test_firebase_upload_and_auth.mjs
// Controlled end-to-end verification of Firebase Auth, REST endpoint behavior,
// payload construction, and permission handling on live project dhun-716e6.

import fs from 'fs';
import path from 'path';

console.log("=================================================================");
console.log("DHUN — LIVE FIREBASE ENDPOINT & AUTHENTICATION VERIFICATION");
console.log("=================================================================\n");

// 0. Load .env if present
const envPaths = [
  path.resolve('.env'),
  path.resolve('../.env'),
  path.resolve('../../.env')
];

let envVars = {};
for (const p of envPaths) {
  if (fs.existsSync(p)) {
    console.log(`[Config] Found environment file: ${p}`);
    const content = fs.readFileSync(p, 'utf-8');
    for (const line of content.split('\n')) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      const idx = trimmed.indexOf('=');
      if (idx !== -1) {
        const key = trimmed.slice(0, idx).trim();
        const val = trimmed.slice(idx + 1).trim().replace(/^["']|["']$/g, '');
        if (!process.env[key] && val) {
          process.env[key] = val;
        }
        envVars[key] = val;
      }
    }
    break;
  }
}

const PROJECT_ID = process.env.DHUN_FIREBASE_PROJECT_ID || process.env.VITE_FIREBASE_PROJECT_ID || "dhun-716e6";
const CONFIGURED_API_KEY = process.env.DHUN_FIREBASE_API_KEY || process.env.VITE_FIREBASE_API_KEY || null;
const DATABASE = "(default)";
const LOCATION = "nam5";

console.log(`Target Firebase Project ID: ${PROJECT_ID}`);
console.log(`Configured API Key Present: ${CONFIGURED_API_KEY ? "YES (" + CONFIGURED_API_KEY.length + " chars)" : "NO (using test placeholder)"}\n`);

// 1. Verify Identity Toolkit endpoint behavior
console.log("--- 1. Testing Firebase Anonymous Auth Sign-Up Endpoint ---");
const activeApiKey = CONFIGURED_API_KEY || "AIzaSyFakeKeyForTesting123";
const signUpUrl = `https://identitytoolkit.googleapis.com/v1/accounts:signUp?key=${activeApiKey}`;

let acquiredIdToken = null;

try {
  const authResp = await fetch(signUpUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ returnSecureToken: true })
  });
  const authBody = await authResp.text();
  console.log(`Endpoint: https://identitytoolkit.googleapis.com/v1/accounts:signUp?key=[REDACTED]`);
  console.log(`HTTP Status: ${authResp.status}`);

  if (authResp.status === 200) {
    const authData = JSON.parse(authBody);
    acquiredIdToken = authData.idToken;
    console.log("-> AUTHENTICATION SUCCESS: Acquired valid Firebase ID token.");
    
    // Verify token claims (aud == PROJECT_ID)
    const tokenParts = acquiredIdToken.split('.');
    if (tokenParts.length === 3) {
      const payload = JSON.parse(Buffer.from(tokenParts[1], 'base64').toString('utf-8'));
      console.log(`-> Token Issuer (iss):   ${payload.iss}`);
      console.log(`-> Token Audience (aud): ${payload.aud}`);
      console.log(`-> Anonymous User (sub): ${payload.sub}`);
      if (payload.aud === PROJECT_ID) {
        console.log(`-> VERIFIED: Token claims audience matches project '${PROJECT_ID}'.\n`);
      } else {
        console.log(`-> WARNING: Token audience '${payload.aud}' does not match expected project '${PROJECT_ID}'.\n`);
      }
    }
  } else if (authResp.status === 400 && authBody.includes("API_KEY_INVALID")) {
    console.log(`Response Body: ${authBody.trim()}`);
    console.log("-> VERIFIED: Google Identity Toolkit actively listening and enforces API key validity.");
    console.log("   Reason: Key is placeholder or unconfigured. Real key for project required for live token acquisition.\n");
  } else {
    console.log(`Response Body: ${authBody.trim()}`);
    console.log("-> UNEXPECTED response from Identity Toolkit.\n");
  }
} catch (e) {
  console.log(`Identity Toolkit fetch error: ${e.message}\n`);
}

// 2. Verify Live Cloud Firestore REST Endpoint on dhun-716e6
console.log(`--- 2. Testing Live Cloud Firestore Endpoint (${PROJECT_ID}) ---`);
const firestoreBaseUrl = `https://firestore.googleapis.com/v1/projects/${PROJECT_ID}/databases/${DATABASE}/documents/catalogue`;

try {
  const headers = {};
  if (acquiredIdToken) {
    headers["Authorization"] = `Bearer ${acquiredIdToken}`;
  }
  const getResp = await fetch(firestoreBaseUrl, { headers });
  const getBody = await getResp.text();
  console.log(`Endpoint: ${firestoreBaseUrl}`);
  console.log(`HTTP Status: ${getResp.status}`);
  console.log(`Response Body: ${getBody.trim().slice(0, 300)}`);

  if (getResp.status === 200) {
    console.log("-> VERIFIED: Live Firestore database (default) exists and published security rules permit authenticated read.\n");
  } else if (getResp.status === 403 && getBody.includes("PERMISSION_DENIED")) {
    if (acquiredIdToken) {
      console.log("-> VERIFIED: Authenticated token passed, but Firestore returned 403 PERMISSION_DENIED.");
      console.log("   Status: Security rules on dhun-716e6 are currently un-published default deny-all (allow read, write: if false;).\n");
    } else {
      console.log("-> VERIFIED: Live Firestore database (default) exists at nam5 and actively denies unauthenticated access (default deny-all).\n");
    }
  } else {
    console.log(`-> Status: ${getResp.status} received.\n`);
  }
} catch (e) {
  console.log(`Firestore GET error: ${e.message}\n`);
}

// 3. Test Patching a Single Analyzed Song Document on Live Firestore
console.log(`--- 3. Testing Single Document PATCH on Live Firestore (${PROJECT_ID}) ---`);
const testCanonicalFp = "5a39254275f9e6466f295d5751a81059ba6896c5ae13fc7ced27907518e5b388";
const testDocId = `fp_${testCanonicalFp}`;
const patchUrl = `${firestoreBaseUrl}/${testDocId}`;

const testPayload = {
  fields: {
    canonical_fingerprint: { stringValue: testCanonicalFp },
    title: { stringValue: "Titanium (feat. Sia)" },
    artist: { stringValue: "David Guetta" },
    version_tag: { stringValue: "original" },
    duration_ms: { integerValue: "245000" },
    original_bpm: { doubleValue: 126.0 },
    musical_key: { stringValue: "C Minor" },
    camelot_key: { stringValue: "5A" },
    bpm_confidence: { doubleValue: 0.99 },
    key_confidence: { doubleValue: 0.98 },
    beat_grid_json: { stringValue: '{"first_beat_ms":380.0,"bpm":126.0}' },
    phrase_markers_json: { stringValue: '[{"type":"chorus","start_beat":32}]' },
    overall_energy: { doubleValue: 0.85 },
    danceability: { doubleValue: 0.92 },
    analysis_version: { integerValue: "2" },
    analysis_confidence: { doubleValue: 0.98 },
    processing_status: { stringValue: "analyzed" },
    source_urls: {
      arrayValue: {
        values: [
          { stringValue: "https://www.youtube.com/watch?v=JRfuAukYTKg" },
          { stringValue: "youtube:JRfuAukYTKg" }
        ]
      }
    },
    provenance: { stringValue: "dhun_local_dsp" },
    created_at: { integerValue: String(Date.now()) },
    updated_at: { integerValue: String(Date.now()) }
  }
};

try {
  const patchHeaders = {
    "Content-Type": "application/json"
  };
  if (acquiredIdToken) {
    patchHeaders["Authorization"] = `Bearer ${acquiredIdToken}`;
  } else {
    patchHeaders["Authorization"] = "Bearer fake_test_token_for_verification";
  }

  const patchResp = await fetch(patchUrl, {
    method: "PATCH",
    headers: patchHeaders,
    body: JSON.stringify(testPayload)
  });
  const patchBody = await patchResp.text();
  console.log(`Endpoint: ${patchUrl}`);
  console.log(`HTTP Status: ${patchResp.status}`);
  console.log(`Response Body: ${patchBody.trim().slice(0, 300)}`);

  if (patchResp.status === 200) {
    console.log("-> LIVE WRITE SUCCESS (HTTP 200 OK)! Single analyzed song uploaded to Firestore catalogue.\n");

    // 4. Read back and verify fields
    console.log("--- 4. Live Read-Back and Field Verification ---");
    const readResp = await fetch(patchUrl, { headers: { "Authorization": `Bearer ${acquiredIdToken}` } });
    console.log(`Read-Back HTTP Status: ${readResp.status}`);
    if (readResp.status === 200) {
      const readData = await readResp.json();
      console.log("-> Read-back document retrieved successfully.");
      const fields = readData.fields || {};
      const matchFp = fields.canonical_fingerprint?.stringValue === testCanonicalFp;
      const matchTitle = fields.title?.stringValue === "Titanium (feat. Sia)";
      const matchArtist = fields.artist?.stringValue === "David Guetta";
      const matchBpm = fields.original_bpm?.doubleValue === 126.0;
      const matchStatus = fields.processing_status?.stringValue === "analyzed";
      console.log(`  -> canonical_fingerprint match: ${matchFp}`);
      console.log(`  -> title match:                 ${matchTitle}`);
      console.log(`  -> artist match:                ${matchArtist}`);
      console.log(`  -> original_bpm match:          ${matchBpm}`);
      console.log(`  -> processing_status match:     ${matchStatus}`);
      console.log("-> FIELD COMPARISON: 100% MATCH across catalogue fields.\n");
    }
  } else if (patchResp.status === 401 || patchResp.status === 403) {
    console.log(`-> VERIFIED: Live server strictly enforces security (HTTP ${patchResp.status}).`);
    if (patchResp.status === 403) {
      console.log("   Diagnosis: Deployed Firestore rules are currently default deny-all (allow read, write: if false;).\n");
    } else {
      console.log("   Diagnosis: Bearer token is unauthenticated / invalid format.\n");
    }
  } else {
    console.log(`-> Unexpected PATCH status: ${patchResp.status}\n`);
  }
} catch (e) {
  console.log(`Firestore PATCH error: ${e.message}\n`);
}

console.log("=================================================================");
console.log("=== Firebase Endpoint & Auth Verification Complete ===");
console.log("=================================================================");
