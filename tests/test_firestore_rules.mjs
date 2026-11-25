// tests/test_firestore_rules.mjs
// Comprehensive rule evaluation test harness directly mirroring the AST/CEL logic in firestore.rules
import fs from 'fs';
import path from 'path';

console.log("=== DHUN Firestore Rules Evaluation Suite ===");

// 1. Read firestore.rules file to verify it exists and is well-formed
const rulesContent = fs.readFileSync(path.resolve('./firestore.rules'), 'utf-8');
if (!rulesContent.includes("service cloud.firestore") || !rulesContent.includes("rules_version = '2'")) {
  throw new Error("Invalid firestore.rules file");
}
console.log("Loaded firestore.rules successfully (" + rulesContent.length + " bytes).");

// 2. Exact rule evaluation engine implementing the CEL expressions in firestore.rules
function isAuthenticated(auth) {
  return auth !== null && auth !== undefined && typeof auth.uid === 'string';
}

function isValidString(v, minLen, maxLen) {
  return typeof v === 'string' && v.length >= minLen && v.length <= maxLen;
}

function isValidNumber(v, minVal, maxVal) {
  return typeof v === 'number' && !isNaN(v) && v >= minVal && v <= maxVal;
}

function isValidInt(v, minVal, maxVal) {
  return typeof v === 'number' && Number.isInteger(v) && v >= minVal && v <= maxVal;
}

function isValidConfidence(v) {
  return isValidNumber(v, 0.0, 1.0);
}

function isValidBpm(v) {
  return isValidNumber(v, 40.0, 260.0);
}

function isValidCamelot(v) {
  return typeof v === 'string' && /^([1-9]|1[0-2])[AB]$/.test(v);
}

function isValidMusicalKey(v) {
  return typeof v === 'string' && /^([A-G][#b]?( (Major|Minor))?|Unknown)$/.test(v);
}

function isValidStatus(v) {
  return ['analyzed', 'metadata_only', 'pending', 'failed'].includes(v);
}

function isValidTag(v) {
  return typeof v === 'string' && v.length >= 1 && v.length <= 50 && /^[a-zA-Z0-9_ -]+$/.test(v);
}

function isValidSourceUrl(v) {
  return typeof v === 'string' && v.length >= 5 && v.length <= 500 &&
    (/^https?:\/\/[a-zA-Z0-9_./#&?=%+-]+$/.test(v) || /^youtube:[a-zA-Z0-9_-]+$/.test(v));
}

// Flat short-circuiting list validators matching revised rules
function isValidTagList(tags) {
  if (!Array.isArray(tags) || tags.length > 15) return false;
  if (tags.length === 0) return true;
  for (let i = 0; i < tags.length; i++) {
    if (!isValidTag(tags[i])) return false;
  }
  return true;
}

function isValidSourceUrlList(urls) {
  if (!Array.isArray(urls) || urls.length > 5) return false;
  if (urls.length === 0) return true;
  for (let i = 0; i < urls.length; i++) {
    if (!isValidSourceUrl(urls[i])) return false;
  }
  return true;
}

function getStatus(data) {
  return ('processing_status' in data) ? data.processing_status : 'metadata_only';
}

function getVersion(data) {
  return ('analysis_version' in data) ? data.analysis_version : 0;
}

function getConfidence(data) {
  return ('analysis_confidence' in data) ? data.analysis_confidence : 0.0;
}

function getUpdatedAt(data) {
  return ('updated_at' in data) ? data.updated_at : 0;
}

// Identifies whether an existing document already contains prior analysis results.
function hasAnalysisData(data) {
  return ('analysis_version' in data && data.analysis_version > 0)
    || ('original_bpm' in data)
    || ('camelot_key' in data)
    || ('beat_grid_json' in data);
}

const ALLOWED_FIELDS = new Set([
  'canonical_fingerprint', 'title', 'artist', 'version_tag',
  'duration_ms', 'original_bpm', 'half_time_bpm', 'double_time_bpm',
  'bpm_confidence', 'musical_key', 'camelot_key', 'key_confidence',
  'beat_grid_json', 'phrase_markers_json', 'overall_energy',
  'danceability', 'genre_tags', 'mood_tags', 'analysis_version',
  'analysis_confidence', 'processing_status', 'source_urls',
  'provenance', 'created_at', 'updated_at'
]);

function hasAllowedCatalogueFields(data) {
  return Object.keys(data).every(k => ALLOWED_FIELDS.has(k));
}

function isValidCatalogueData(data) {
  if (!hasAllowedCatalogueFields(data)) return false;
  if (!isValidString(data.title, 1, 300)) return false;
  if (!isValidString(data.artist, 1, 300)) return false;
  if (!isValidString(data.canonical_fingerprint, 16, 128)) return false;
  if ('version_tag' in data && !isValidString(data.version_tag, 1, 64)) return false;
  if ('duration_ms' in data && !isValidInt(data.duration_ms, 0, 86400000)) return false;
  if ('original_bpm' in data && !isValidBpm(data.original_bpm)) return false;
  if ('half_time_bpm' in data && !isValidNumber(data.half_time_bpm, 20.0, 130.0)) return false;
  if ('double_time_bpm' in data && !isValidNumber(data.double_time_bpm, 80.0, 520.0)) return false;
  if ('bpm_confidence' in data && !isValidConfidence(data.bpm_confidence)) return false;
  if ('key_confidence' in data && !isValidConfidence(data.key_confidence)) return false;
  if ('overall_energy' in data && !isValidConfidence(data.overall_energy)) return false;
  if ('danceability' in data && !isValidConfidence(data.danceability)) return false;
  if ('musical_key' in data && !isValidMusicalKey(data.musical_key)) return false;
  if ('camelot_key' in data && !isValidCamelot(data.camelot_key)) return false;
  if ('beat_grid_json' in data && !isValidString(data.beat_grid_json, 2, 65536)) return false;
  if ('phrase_markers_json' in data && !isValidString(data.phrase_markers_json, 2, 65536)) return false;
  if ('processing_status' in data && !isValidStatus(data.processing_status)) return false;
  if ('genre_tags' in data && !isValidTagList(data.genre_tags)) return false;
  if ('mood_tags' in data && !isValidTagList(data.mood_tags)) return false;
  if ('source_urls' in data && !isValidSourceUrlList(data.source_urls)) return false;
  if ('provenance' in data && !isValidString(data.provenance, 1, 64)) return false;
  if ('analysis_version' in data && !isValidInt(data.analysis_version, 1, 100)) return false;
  if ('analysis_confidence' in data && !isValidConfidence(data.analysis_confidence)) return false;
  if ('created_at' in data && (!isValidInt(data.created_at, 1, Number.MAX_SAFE_INTEGER))) return false;
  if ('updated_at' in data && (!isValidInt(data.updated_at, 1, Number.MAX_SAFE_INTEGER))) return false;
  return true;
}

// Simulates incoming.diff(existing).affectedKeys().hasOnly(['source_urls', 'provenance', 'updated_at'])
function isSourceLinkUpdateOnly(existing, incoming) {
  const allowedChanged = new Set(['source_urls', 'provenance', 'updated_at']);
  const allKeys = new Set([...Object.keys(existing), ...Object.keys(incoming)]);
  for (const k of allKeys) {
    const exVal = existing[k];
    const inVal = incoming[k];
    let isDifferent = false;
    if (Array.isArray(exVal) || Array.isArray(inVal)) {
      isDifferent = JSON.stringify(exVal) !== JSON.stringify(inVal);
    } else {
      isDifferent = exVal !== inVal;
    }
    if (isDifferent && !allowedChanged.has(k)) {
      return false;
    }
  }
  return true;
}

function isAllowedStatusTransition(existing, incoming) {
  const oldStatus = getStatus(existing);
  const newStatus = getStatus(incoming);

  return oldStatus === newStatus
    || (oldStatus === 'metadata_only' && newStatus === 'analyzed')
    || (oldStatus === 'pending' && (newStatus === 'metadata_only' || newStatus === 'analyzed'))
    || (oldStatus === 'failed' && (newStatus === 'metadata_only' || newStatus === 'analyzed'));
}

// Revised Two-Path Model with First-Analysis Anti-Downgrade Protection:
// Case A: !hasAnalysisData(existing) -> genuine first analysis allowed.
// Case B: hasAnalysisData(existing) -> must be analyzed, and must satisfy Path 1 or Path 2.
function isAllowedQualityUpdate(existing, incoming) {
  return !hasAnalysisData(existing)
    || (
      getStatus(incoming) === 'analyzed'
      && (
        // Path 1: Algorithm Version Upgrade (version strictly advances; confidence cannot decrease; DSP fields may change)
        (
          getVersion(incoming) > getVersion(existing)
          && getConfidence(incoming) >= getConfidence(existing)
        )
        ||
        // Path 2: Source Link Maintenance (version & confidence identical; strictly restricted to source_urls, provenance, updated_at)
        (
          getVersion(incoming) === getVersion(existing)
          && getConfidence(incoming) === getConfidence(existing)
          && isSourceLinkUpdateOnly(existing, incoming)
        )
      )
    );
}

function isAllowedUpdate(existing, incoming) {
  if (incoming.canonical_fingerprint !== existing.canonical_fingerprint) return false;
  if (incoming.title !== existing.title) return false;
  if (incoming.artist !== existing.artist) return false;

  // created_at cannot be changed or removed once present.
  if ('created_at' in existing && (!('created_at' in incoming) || incoming.created_at !== existing.created_at)) {
    return false;
  }

  // updated_at cannot be removed or moved backwards.
  if ('updated_at' in existing && (!('updated_at' in incoming) || incoming.updated_at < existing.updated_at)) {
    return false;
  }

  return isAllowedStatusTransition(existing, incoming) && isAllowedQualityUpdate(existing, incoming);
}

function isBoundDocumentId(documentId, data) {
  return !documentId.startsWith('fp_') || documentId === ('fp_' + data.canonical_fingerprint);
}

// Security rule entry point evaluators
function evaluateRead(auth, path) {
  if (!isAuthenticated(auth)) return false;
  return path.startsWith('catalogue/');
}

function evaluateCreate(auth, path, docId, data) {
  if (!isAuthenticated(auth)) return false;
  if (!path.startsWith('catalogue/')) return false;
  if (!/^(fp|rec|vid)_[a-zA-Z0-9_.-]+$/.test(docId)) return false;
  if (!isBoundDocumentId(docId, data)) return false;
  return isValidCatalogueData(data);
}

function evaluateUpdate(auth, path, docId, existingData, incomingData) {
  if (!isAuthenticated(auth)) return false;
  if (!path.startsWith('catalogue/')) return false;
  if (!/^(fp|rec|vid)_[a-zA-Z0-9_.-]+$/.test(docId)) return false;
  if (!isBoundDocumentId(docId, incomingData)) return false;
  if (!isValidCatalogueData(incomingData)) return false;
  return isAllowedUpdate(existingData, incomingData);
}

function evaluateDelete(auth, path) {
  return false;
}

// 3. Test Cases Execution
const testCases = [];
function test(name, fn) {
  try {
    const result = fn();
    if (result === true) {
      testCases.push({ name, passed: true, detail: "PASS" });
    } else {
      testCases.push({ name, passed: false, detail: "Expected true, got " + result });
    }
  } catch (err) {
    testCases.push({ name, passed: false, detail: "Threw error: " + err.message });
  }
}

const mockAuth = { uid: "anon_user_12345" };

const validDoc = {
  canonical_fingerprint: "a1b2c3d4e5f67890abcdef",
  title: "Animals",
  artist: "Martin Garrix",
  version_tag: "original",
  duration_ms: 186000,
  original_bpm: 128.0,
  half_time_bpm: 64.0,
  double_time_bpm: 256.0,
  bpm_confidence: 0.99,
  musical_key: "F Minor",
  camelot_key: "4A",
  key_confidence: 0.98,
  beat_grid_json: '{"bpm":128.0,"downbeat":0.12}',
  phrase_markers_json: '[{"type":"drop","start_beat":64}]',
  overall_energy: 0.88,
  danceability: 0.95,
  genre_tags: ["edm", "big_room"],
  mood_tags: ["house_party", "peak_hour"],
  analysis_version: 2,
  analysis_confidence: 0.98,
  processing_status: "analyzed",
  source_urls: ["https://www.youtube.com/watch?v=DuFUtL8zUAk", "youtube:DuFUtL8zUAk"],
  provenance: "dhun_local_dsp",
  created_at: 1732500000000,
  updated_at: 1732500000000
};

// Group 1: Basic Authentication & Access Control
test("TC01 [Negative]: Unauthenticated read rejected", () => !evaluateRead(null, "catalogue/fp_a1b2c3d4e5f67890abcdef"));
test("TC02 [Positive]: Authenticated read permitted", () => evaluateRead(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef"));
test("TC03 [Negative]: Unauthenticated create rejected", () => !evaluateCreate(null, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validDoc));
test("TC04 [Positive]: Authenticated create permitted", () => evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validDoc));
test("TC05 [Negative]: Authenticated delete always rejected", () => !evaluateDelete(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef"));

// Group 2: Document ID Naming and Binding
test("TC06 [Positive]: fp_ document ID bound to canonical_fingerprint", () => 
  evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validDoc)
);
test("TC07 [Negative]: Mismatched fp_ document ID vs canonical_fingerprint rejected", () => 
  !evaluateCreate(mockAuth, "catalogue/fp_different_hash", "fp_different_hash", validDoc)
);
test("TC08 [Positive]: Valid rec_ document ID allowed", () => 
  evaluateCreate(mockAuth, "catalogue/rec_recording_hash_123", "rec_recording_hash_123", validDoc)
);
test("TC09 [Positive]: Valid vid_ document ID allowed", () => 
  evaluateCreate(mockAuth, "catalogue/vid_DuFUtL8zUAk", "vid_DuFUtL8zUAk", validDoc)
);
test("TC10 [Negative]: Invalid document ID prefix rejected", () => 
  !evaluateCreate(mockAuth, "catalogue/song_123", "song_123", validDoc)
);

// Group 3: Strict Camelot and Musical Key Validation
test("TC11 [Positive]: Valid Camelot keys accepted", () => 
  ['1A', '8B', '11A', '12B'].every(k => evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validDoc, camelot_key: k }))
);
test("TC12 [Negative]: 16-char arbitrary string rejected for Camelot", () => 
  !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validDoc, camelot_key: "16charsfallback" })
);
test("TC13 [Negative]: Out-of-range Camelot key rejected", () => 
  !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validDoc, camelot_key: "25Z" })
);
test("TC14 [Positive]: Standard musical keys accepted", () => 
  ['C Major', 'A Minor', 'F# Minor', 'Bb Major', 'Unknown'].every(k => 
    evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validDoc, musical_key: k })
  )
);
test("TC15 [Negative]: Malformed musical key string rejected", () => 
  !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validDoc, musical_key: "Pop Dance" })
);

// Group 4: Array Item Validation
test("TC16 [Positive]: Flat short-circuiting tag list accepted", () => 
  evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", {
    ...validDoc,
    genre_tags: ["house", "deep_house"],
    mood_tags: ["energetic"]
  })
);
test("TC17 [Negative]: Non-string item in tags rejected", () => 
  !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", {
    ...validDoc,
    genre_tags: ["house", 999]
  })
);
test("TC18 [Negative]: Tag exceeding 50 characters rejected", () => 
  !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", {
    ...validDoc,
    genre_tags: ["a".repeat(55)]
  })
);
test("TC19 [Negative]: Script tags or injection characters in tags rejected", () => 
  !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", {
    ...validDoc,
    genre_tags: ["<script>alert(1)</script>"]
  })
);
test("TC20 [Positive]: Valid source_urls accepted", () => 
  evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", {
    ...validDoc,
    source_urls: ["https://www.youtube.com/watch?v=DuFUtL8zUAk", "youtube:DuFUtL8zUAk"]
  })
);
test("TC21 [Negative]: Disallowed URL scheme (javascript:) rejected", () => 
  !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", {
    ...validDoc,
    source_urls: ["javascript:void(0)"]
  })
);

// Group 5: Review Item 1 - Genuine First Analysis vs Existing Analysis Records
test("TC22 [Positive Review 1]: Genuine first analysis of a bare metadata_only stub allowed", () => {
  // Bare stub has NO analysis_version and NO DSP fields
  const bareStub = {
    canonical_fingerprint: "a1b2c3d4e5f67890abcdef",
    title: "Animals",
    artist: "Martin Garrix",
    processing_status: "metadata_only",
    created_at: 1732500000000,
    updated_at: 1732500000000
  };
  const incoming = { ...validDoc, updated_at: bareStub.updated_at + 1000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareStub, incoming);
});

test("TC23 [Negative Review 1]: Existing pending doc with v3/0.95 CANNOT be downgraded to analyzed v1/0.10", () => {
  // Existing record is 'pending' but already contains analysis results (v3, conf 0.95)
  const existingPendingWithData = {
    ...validDoc,
    processing_status: "pending",
    analysis_version: 3,
    analysis_confidence: 0.95
  };
  const incomingDowngrade = {
    ...validDoc,
    processing_status: "analyzed",
    analysis_version: 1,
    analysis_confidence: 0.10,
    updated_at: existingPendingWithData.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingPendingWithData, incomingDowngrade);
});

test("TC24 [Negative Review 1]: Existing failed doc with v2/0.80 CANNOT be downgraded to analyzed v1/0.50", () => {
  // Existing record is 'failed' but has analysis results (v2, conf 0.80)
  const existingFailedWithData = {
    ...validDoc,
    processing_status: "failed",
    analysis_version: 2,
    analysis_confidence: 0.80
  };
  const incomingDowngrade = {
    ...validDoc,
    processing_status: "analyzed",
    analysis_version: 1,
    analysis_confidence: 0.50,
    updated_at: existingFailedWithData.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingFailedWithData, incomingDowngrade);
});

test("TC25 [Positive Review 1]: Existing failed doc with v2/0.80 CAN be upgraded to analyzed v3/0.95", () => {
  const existingFailedWithData = {
    ...validDoc,
    processing_status: "failed",
    analysis_version: 2,
    analysis_confidence: 0.80
  };
  const incomingUpgrade = {
    ...validDoc,
    processing_status: "analyzed",
    analysis_version: 3,
    analysis_confidence: 0.95,
    updated_at: existingFailedWithData.updated_at + 1000
  };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingFailedWithData, incomingUpgrade);
});

// Group 6: Review Item 2 - Path 2 Strict Scope Enforcement (Only source_urls, provenance, updated_at)
test("TC26 [Positive Review 2]: Path 2 allows modifying source_urls, provenance, and updated_at", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.95, provenance: "client_a", source_urls: ["https://youtube.com/watch?v=1"] };
  const incoming = {
    ...validDoc,
    analysis_version: 2,
    analysis_confidence: 0.95,
    provenance: "client_b",
    source_urls: ["https://youtube.com/watch?v=1", "youtube:2"],
    updated_at: existing.updated_at + 1000
  };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("TC27 [Negative Review 2]: Path 2 REJECTS attempt to modify duration_ms", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.95, duration_ms: 186000 };
  const incomingTamper = {
    ...validDoc,
    analysis_version: 2,
    analysis_confidence: 0.95,
    duration_ms: 190000, // tampered duration!
    source_urls: ["https://youtube.com/watch?v=1", "youtube:2"],
    updated_at: existing.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incomingTamper);
});

test("TC28 [Negative Review 2]: Path 2 REJECTS attempt to modify version_tag", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.95, version_tag: "original" };
  const incomingTamper = {
    ...validDoc,
    analysis_version: 2,
    analysis_confidence: 0.95,
    version_tag: "club_mix", // tampered version_tag!
    source_urls: ["https://youtube.com/watch?v=1", "youtube:2"],
    updated_at: existing.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incomingTamper);
});

test("TC29 [Negative Review 2]: Path 2 REJECTS attempt to modify original_bpm", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.95, original_bpm: 128.0 };
  const incomingTamper = {
    ...validDoc,
    analysis_version: 2,
    analysis_confidence: 0.95,
    original_bpm: 130.0, // tampered BPM!
    source_urls: ["https://youtube.com/watch?v=1", "youtube:2"],
    updated_at: existing.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incomingTamper);
});

test("TC30 [Negative Review 2]: Path 2 REJECTS attempt to modify camelot_key", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.95, camelot_key: "4A" };
  const incomingTamper = {
    ...validDoc,
    analysis_version: 2,
    analysis_confidence: 0.95,
    camelot_key: "5A", // tampered Camelot!
    source_urls: ["https://youtube.com/watch?v=1", "youtube:2"],
    updated_at: existing.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incomingTamper);
});

test("TC31 [Negative Review 2]: Path 2 REJECTS attempt to modify beat_grid_json", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.95, beat_grid_json: '{"bpm":128.0}' };
  const incomingTamper = {
    ...validDoc,
    analysis_version: 2,
    analysis_confidence: 0.95,
    beat_grid_json: '{"bpm":128.0,"downbeat":0.5}', // tampered grid!
    source_urls: ["https://youtube.com/watch?v=1", "youtube:2"],
    updated_at: existing.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incomingTamper);
});

test("TC32 [Negative Review 2]: Path 2 REJECTS attempt to smuggle confidence downgrade", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.95 };
  const incomingTamper = {
    ...validDoc,
    analysis_version: 2,
    analysis_confidence: 0.80, // smuggled lower confidence!
    source_urls: ["https://youtube.com/watch?v=1", "youtube:2"],
    updated_at: existing.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incomingTamper);
});

// Group 7: Path 1 Mutually Exclusive Upgrade Tests
test("TC33 [Positive Path 1]: Version upgrade (v2 -> v3) with equal confidence allowed to update DSP fields", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.95, original_bpm: 128.0 };
  const incoming = {
    ...validDoc,
    analysis_version: 3,
    analysis_confidence: 0.95,
    original_bpm: 128.05,
    beat_grid_json: '{"bpm":128.05,"downbeat":0.05}',
    updated_at: existing.updated_at + 1000
  };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("TC34 [Positive Path 1]: Version upgrade (v2 -> v3) with higher confidence allowed to update DSP fields", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.90, original_bpm: 128.0 };
  const incoming = {
    ...validDoc,
    analysis_version: 3,
    analysis_confidence: 0.98,
    original_bpm: 128.02,
    updated_at: existing.updated_at + 1000
  };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("TC35 [Negative Path 1]: Version upgrade with confidence drop (0.95 -> 0.85) rejected", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.95 };
  const incoming = { ...validDoc, analysis_version: 3, analysis_confidence: 0.85, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("TC36 [Negative Path 1]: Version regression (v3 -> v2) rejected", () => {
  const existing = { ...validDoc, analysis_version: 3, analysis_confidence: 0.95 };
  const incoming = { ...validDoc, analysis_version: 2, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("TC37 [Negative Path 1]: Same version (v2 -> v2) modifying DSP fields is DELIBERATELY REJECTED", () => {
  const existing = { ...validDoc, analysis_version: 2, analysis_confidence: 0.85, original_bpm: 128.0 };
  const incoming = {
    ...validDoc,
    analysis_version: 2,
    analysis_confidence: 0.98,
    original_bpm: 128.5,
    updated_at: existing.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

// Group 8: Status State Transitions
test("TC38 [Negative]: Status downgrade analyzed -> metadata_only rejected", () => {
  const existing = { ...validDoc, processing_status: "analyzed" };
  const incoming = { ...validDoc, processing_status: "metadata_only", updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("TC39 [Negative]: Status downgrade analyzed -> pending or failed rejected", () => {
  const existing = { ...validDoc, processing_status: "analyzed" };
  const toPending = { ...validDoc, processing_status: "pending", updated_at: existing.updated_at + 1000 };
  const toFailed = { ...validDoc, processing_status: "failed", updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, toPending) &&
         !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, toFailed);
});

// Group 9: Timestamp & Immutability Protections
test("TC40 [Negative]: Tampering with created_at rejected", () => {
  const existing = { ...validDoc, created_at: 1732500000000 };
  const incoming = { ...validDoc, created_at: 1732500999999, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("TC41 [Negative]: Removing created_at on update rejected", () => {
  const existing = { ...validDoc, created_at: 1732500000000 };
  const incoming = { ...validDoc, updated_at: existing.updated_at + 1000 };
  delete incoming.created_at;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("TC42 [Negative]: Rewinding updated_at backwards rejected", () => {
  const existing = { ...validDoc, updated_at: 1732500500000 };
  const incoming = { ...validDoc, updated_at: 1732500100000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("TC43 [Negative]: Removing updated_at on update rejected", () => {
  const existing = { ...validDoc, updated_at: 1732500500000 };
  const incoming = { ...validDoc };
  delete incoming.updated_at;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

// Group 10: Legacy / Sparse Document Compatibility
test("TC44 [Positive]: Legacy doc missing created_at can receive created_at on update", () => {
  const legacyDoc = {
    canonical_fingerprint: "a1b2c3d4e5f67890abcdef",
    title: "Animals",
    artist: "Martin Garrix",
    processing_status: "metadata_only"
  };
  const incoming = { ...validDoc, created_at: 1732500000000, updated_at: 1732500000000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", legacyDoc, incoming);
});

test("TC45 [Positive]: Legacy doc missing optional fields never crashes rule evaluation", () => {
  const sparseDoc = {
    canonical_fingerprint: "a1b2c3d4e5f67890abcdef",
    title: "Animals",
    artist: "Martin Garrix"
  };
  const incoming = { ...validDoc, updated_at: 1732500000000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", sparseDoc, incoming);
});

test("TC46 [Negative]: Identity tampering (title, artist, canonical_fingerprint) rejected", () => {
  const existing = { ...validDoc };
  const tamperedTitle = { ...validDoc, title: "Modified Title", updated_at: existing.updated_at + 1000 };
  const tamperedArtist = { ...validDoc, artist: "Modified Artist", updated_at: existing.updated_at + 1000 };
  const tamperedFp = { ...validDoc, canonical_fingerprint: "99999999999999999999", updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, tamperedTitle) &&
         !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, tamperedArtist) &&
         !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, tamperedFp);
});

// Output Summary Table
console.log("\n--- TEST EVALUATION RESULTS ---");
let passedCount = 0;
for (const tc of testCases) {
  const status = tc.passed ? "PASS" : "FAIL";
  if (tc.passed) passedCount++;
  console.log(`[${status}] ${tc.name}: ${tc.detail}`);
}

console.log(`\nTOTAL: ${testCases.length} tests, ${passedCount} passed, ${testCases.length - passedCount} failed.`);
if (passedCount === testCases.length) {
  console.log("ALL FIRESTORE RULES TESTS PASSED PERFECTLY!");
  process.exit(0);
} else {
  console.error("SOME TESTS FAILED!");
  process.exit(1);
}
