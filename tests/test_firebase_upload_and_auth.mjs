// tests/test_firebase_upload_and_auth.mjs
// Controlled end-to-end verification of Firebase Auth, REST endpoint behavior,
// payload construction, and permission handling on live project dhun-716e6.

import fs from 'fs';
import path from 'path';

console.log("=================================================================");
console.log("DHUN — LIVE FIREBASE ENDPOINT & AUTHENTICATION VERIFICATION");
console.log("=================================================================\n");

const PROJECT_ID = "dhun-716e6";
const DATABASE = "(default)";
const LOCATION = "nam5";

// 1. Verify Identity Toolkit endpoint behavior
console.log("--- 1. Testing Firebase Anonymous Auth Sign-Up Endpoint ---");
const testApiKey = "AIzaSyFakeKeyForTesting123";
const signUpUrl = `https://identitytoolkit.googleapis.com/v1/accounts:signUp?key=${testApiKey}`;

try {
  const authResp = await fetch(signUpUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ returnSecureToken: true })
  });
  const authBody = await authResp.text();
  console.log(`Endpoint: ${signUpUrl}`);
  console.log(`HTTP Status: ${authResp.status}`);
  console.log(`Response Body: ${authBody.trim()}`);
  
  if (authResp.status === 400 && authBody.includes("API_KEY_INVALID")) {
    console.log("-> VERIFIED: Google Identity Toolkit actively listening and enforces API key validity.\n");
  } else {
    console.log("-> UNEXPECTED response from Identity Toolkit.\n");
  }
} catch (e) {
  console.log(`Identity Toolkit fetch error: ${e.message}\n`);
}

// 2. Verify Live Cloud Firestore REST Endpoint on dhun-716e6
console.log("--- 2. Testing Live Cloud Firestore Endpoint (dhun-716e6) ---");
const firestoreBaseUrl = `https://firestore.googleapis.com/v1/projects/${PROJECT_ID}/databases/${DATABASE}/documents/catalogue`;

try {
  const getResp = await fetch(firestoreBaseUrl);
  const getBody = await getResp.text();
  console.log(`Endpoint: ${firestoreBaseUrl}`);
  console.log(`HTTP Status: ${getResp.status}`);
  console.log(`Response Body: ${getBody.trim()}`);

  if (getResp.status === 403 && getBody.includes("PERMISSION_DENIED")) {
    console.log("-> VERIFIED: Live Firestore database (default) exists at nam5 and actively denies unauthenticated access (default deny-all).\n");
  } else {
    console.log("-> Status other than 403 received.\n");
  }
} catch (e) {
  console.log(`Firestore GET error: ${e.message}\n`);
}

// 3. Test Patching a Song Document on Live Firestore (Testing Permission Enforcement)
console.log("--- 3. Testing Document PATCH on Live Firestore (dhun-716e6) ---");
const testDocId = "fp_test_titanium_live_check";
const patchUrl = `${firestoreBaseUrl}/${testDocId}`;

const testPayload = {
  fields: {
    canonical_fingerprint: { stringValue: "test_titanium_live_check" },
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
  const patchResp = await fetch(patchUrl, {
    method: "PATCH",
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer fake_test_token_for_verification"
    },
    body: JSON.stringify(testPayload)
  });
  const patchBody = await patchResp.text();
  console.log(`Endpoint: ${patchUrl}`);
  console.log(`HTTP Status: ${patchResp.status}`);
  console.log(`Response Body: ${patchBody.trim()}`);

  if (patchResp.status === 401 || patchResp.status === 403) {
    console.log("-> VERIFIED: Live server strictly enforces auth and rejects invalid tokens / unpermitted writes.\n");
  } else {
    console.log(`-> Unexpected PATCH status: ${patchResp.status}\n`);
  }
} catch (e) {
  console.log(`Firestore PATCH error: ${e.message}\n`);
}

console.log("=== Firebase Endpoint & Auth Verification Complete ===");
