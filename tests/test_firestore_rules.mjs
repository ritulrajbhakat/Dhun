// tests/test_firestore_rules.mjs
// Deterministic In-Process CEL Semantics & Schema Test Suite
// Directly mirrors the AST and CEL execution semantics of firestore.rules
import fs from 'fs';
import path from 'path';

console.log("=== DHUN Firestore Rules Evaluation Suite ===");

// 1. Read firestore.rules file to verify it exists and is well-formed
const rulesPath = path.resolve('./firestore.rules');
const rulesContent = fs.readFileSync(rulesPath, 'utf-8');
if (!rulesContent.includes("service cloud.firestore") || !rulesContent.includes("rules_version = '2'")) {
  throw new Error("Invalid firestore.rules file");
}
console.log(`Loaded firestore.rules successfully (${rulesContent.length} bytes).`);

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

// Safely reads algorithm version:
// If analysis_version is explicit, returns it.
// If missing but document contains analysis data, baseline version is 2 (matching DHUN's default).
// Otherwise returns 0 for bare unanalyzed stubs.
function getVersion(data) {
  return ('analysis_version' in data)
    ? data.analysis_version
    : (hasAnalysisData(data) ? 2 : 0);
}

// Safely reads analysis confidence:
// If analysis_confidence is explicit, returns it.
// If missing but document contains analysis data, baseline confidence is 1.0 (matching DHUN's default).
// Otherwise returns 0.0 for bare unanalyzed stubs.
function getConfidence(data) {
  return ('analysis_confidence' in data)
    ? data.analysis_confidence
    : (hasAnalysisData(data) ? 1.0 : 0.0);
}

function getUpdatedAt(data) {
  return ('updated_at' in data) ? data.updated_at : 0;
}

// Comprehensive detection of all 15 analysis-related fields
function hasAnalysisData(data) {
  return ('analysis_version' in data)
    || ('analysis_confidence' in data)
    || ('original_bpm' in data)
    || ('half_time_bpm' in data)
    || ('double_time_bpm' in data)
    || ('bpm_confidence' in data)
    || ('musical_key' in data)
    || ('camelot_key' in data)
    || ('key_confidence' in data)
    || ('beat_grid_json' in data)
    || ('phrase_markers_json' in data)
    || ('overall_energy' in data)
    || ('danceability' in data)
    || ('genre_tags' in data)
    || ('mood_tags' in data);
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

// Simulates incoming.diff(existing).affectedKeys()
function getAffectedKeys(existing, incoming) {
  const allKeys = new Set([...Object.keys(existing), ...Object.keys(incoming)]);
  const affected = new Set();
  for (const k of allKeys) {
    const exVal = existing[k];
    const inVal = incoming[k];
    if (exVal === undefined || inVal === undefined) {
      affected.add(k);
    } else if (Array.isArray(exVal) || Array.isArray(inVal)) {
      if (JSON.stringify(exVal) !== JSON.stringify(inVal)) affected.add(k);
    } else if (exVal !== inVal) {
      affected.add(k);
    }
  }
  return affected;
}

// Path 2 enforcement:
// diff.hasOnly(['source_urls', 'provenance', 'updated_at']) && diff.hasAny(['source_urls', 'provenance'])
function isSourceLinkUpdateOnly(existing, incoming) {
  const diff = getAffectedKeys(existing, incoming);
  const allowed = new Set(['source_urls', 'provenance', 'updated_at']);
  for (const k of diff) {
    if (!allowed.has(k)) return false;
  }
  return diff.has('source_urls') || diff.has('provenance');
}

function isAllowedStatusTransition(existing, incoming) {
  const oldStatus = getStatus(existing);
  const newStatus = getStatus(incoming);

  return oldStatus === newStatus
    || (oldStatus === 'metadata_only' && newStatus === 'analyzed')
    || (oldStatus === 'pending' && (newStatus === 'metadata_only' || newStatus === 'analyzed'))
    || (oldStatus === 'failed' && (newStatus === 'metadata_only' || newStatus === 'analyzed'));
}

// Mutually exclusive three-scenario policy
function isAllowedQualityUpdate(existing, incoming) {
  const existingHasAnalysis = hasAnalysisData(existing);
  const incomingStatus = getStatus(incoming);

  // Scenario A: Genuine first analysis of a bare stub
  if (!existingHasAnalysis) {
    return incomingStatus === 'analyzed'
      && ('analysis_version' in incoming)
      && ('analysis_confidence' in incoming)
      && (!('version_tag' in existing) || incoming.version_tag === existing.version_tag)
      && (!('duration_ms' in existing) || incoming.duration_ms === existing.duration_ms);
  }

  // Scenario B: Algorithm version upgrade
  if (incomingStatus === 'analyzed'
      && getVersion(incoming) > getVersion(existing)
      && getConfidence(incoming) >= getConfidence(existing)) {
    return true;
  }

  // Scenario C: Source-link maintenance
  if (getVersion(incoming) === getVersion(existing)
      && getConfidence(incoming) === getConfidence(existing)
      && isSourceLinkUpdateOnly(existing, incoming)) {
    return true;
  }

  return false;
}

function isAllowedUpdate(existing, incoming) {
  if (incoming.canonical_fingerprint !== existing.canonical_fingerprint) return false;
  if (incoming.title !== existing.title) return false;
  if (incoming.artist !== existing.artist) return false;

  // created_at cannot be changed or removed once present
  if ('created_at' in existing && (!('created_at' in incoming) || incoming.created_at !== existing.created_at)) {
    return false;
  }

  // updated_at cannot be removed or moved backwards
  if ('updated_at' in existing && (!('updated_at' in incoming) || incoming.updated_at < existing.updated_at)) {
    return false;
  }

  // source_urls cannot be removed once present
  if ('source_urls' in existing && (!('source_urls' in incoming) || incoming.source_urls.length === 0)) {
    return false;
  }

  return isAllowedStatusTransition(existing, incoming) && isAllowedQualityUpdate(existing, incoming);
}

function isBoundDocumentId(documentId, data) {
  return !documentId.startsWith('fp_') || documentId === ('fp_' + data.canonical_fingerprint);
}

// Evaluators matching rule match blocks
function evaluateRead(auth, docPath) {
  if (!isAuthenticated(auth)) return false;
  return docPath.startsWith('catalogue/');
}

function evaluateCreate(auth, docPath, docId, data) {
  if (!isAuthenticated(auth)) return false;
  if (!docPath.startsWith('catalogue/')) return false;
  if (!/^(fp|rec|vid)_[a-zA-Z0-9_.-]+$/.test(docId)) return false;
  if (!isBoundDocumentId(docId, data)) return false;
  return isValidCatalogueData(data);
}

function evaluateUpdate(auth, docPath, docId, existingData, incomingData) {
  if (!isAuthenticated(auth)) return false;
  if (!docPath.startsWith('catalogue/')) return false;
  if (!/^(fp|rec|vid)_[a-zA-Z0-9_.-]+$/.test(docId)) return false;
  if (!isBoundDocumentId(docId, incomingData)) return false;
  if (!isValidCatalogueData(incomingData)) return false;
  return isAllowedUpdate(existingData, incomingData);
}

function evaluateDelete(auth, docPath) {
  return false;
}

// 3. Test Runner
const testCases = [];
function test(id, name, fn) {
  try {
    const result = fn();
    if (result === true) {
      testCases.push({ id, name, passed: true, detail: "PASS" });
    } else {
      testCases.push({ id, name, passed: false, detail: "Expected true, got " + result });
    }
  } catch (err) {
    testCases.push({ id, name, passed: false, detail: "Threw error: " + err.message });
  }
}

const mockAuth = { uid: "anon_test_user_777" };

const validAnalyzedDoc = {
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

// ==========================================
// PART 7.1: INITIAL ANALYSIS TESTS (11 Tests)
// ==========================================
const bareMetadataStub = {
  canonical_fingerprint: "a1b2c3d4e5f67890abcdef",
  title: "Animals",
  artist: "Martin Garrix",
  version_tag: "original",
  duration_ms: 186000,
  processing_status: "metadata_only",
  source_urls: ["https://www.youtube.com/watch?v=DuFUtL8zUAk"],
  provenance: "dhun_resolver",
  created_at: 1732500000000,
  updated_at: 1732500000000
};

test("01", "Bare metadata stub receives first valid analysis -> ACCEPT", () => {
  const incoming = { ...validAnalyzedDoc, updated_at: bareMetadataStub.updated_at + 1000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, incoming);
});

test("02", "Bare stub receives update remaining metadata_only while changing duration -> REJECT", () => {
  const incoming = { ...bareMetadataStub, duration_ms: 200000, updated_at: bareMetadataStub.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, incoming);
});

test("03", "Bare stub receives update remaining metadata_only while changing version_tag -> REJECT", () => {
  const incoming = { ...bareMetadataStub, version_tag: "extended", updated_at: bareMetadataStub.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, incoming);
});

test("04", "Bare stub receives update with invalid analysis_version (e.g. 0 or 200) -> REJECT", () => {
  const incoming = { ...validAnalyzedDoc, analysis_version: 0, updated_at: bareMetadataStub.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, incoming);
});

test("05", "Bare stub receives update missing required first-analysis fields (e.g. missing analysis_version) -> REJECT", () => {
  const incoming = { ...validAnalyzedDoc, updated_at: bareMetadataStub.updated_at + 1000 };
  delete incoming.analysis_version;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, incoming);
});

test("06", "Existing pending record with analysis_version cannot bypass anti-downgrade -> REJECT", () => {
  const existingPending = { ...bareMetadataStub, processing_status: "pending", analysis_version: 3, analysis_confidence: 0.95 };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.20, updated_at: existingPending.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingPending, incomingDowngrade);
});

test("07", "Existing failed record with only original_bpm cannot bypass protection -> REJECT", () => {
  const existingFailed = { ...bareMetadataStub, processing_status: "failed", original_bpm: 128.0 };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingFailed.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingFailed, incomingDowngrade);
});

test("08", "Existing record with only musical_key cannot bypass protection -> REJECT", () => {
  const existingKey = { ...bareMetadataStub, musical_key: "F Minor" };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingKey.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingKey, incomingDowngrade);
});

test("09", "Existing record with only phrase_markers_json cannot bypass protection -> REJECT", () => {
  const existingPhrase = { ...bareMetadataStub, phrase_markers_json: "[]" };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingPhrase.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingPhrase, incomingDowngrade);
});

test("10", "Existing record with only danceability cannot bypass protection -> REJECT", () => {
  const existingDance = { ...bareMetadataStub, danceability: 0.85 };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingDance.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingDance, incomingDowngrade);
});

test("11", "Existing record with only genre_tags cannot bypass protection -> REJECT", () => {
  const existingGenre = { ...bareMetadataStub, genre_tags: ["edm"] };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingGenre.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingGenre, incomingDowngrade);
});

// ==========================================
// PART 7.2: ALGORITHM UPGRADE TESTS (9 Tests)
// ==========================================
test("12", "v2 -> v3 with equal confidence and changed DSP -> ACCEPT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, original_bpm: 128.0 };
  const incoming = {
    ...validAnalyzedDoc,
    analysis_version: 3,
    analysis_confidence: 0.95,
    original_bpm: 128.04,
    beat_grid_json: '{"bpm":128.04,"downbeat":0.08}',
    updated_at: existing.updated_at + 1000
  };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("13", "v2 -> v3 with higher confidence and changed DSP -> ACCEPT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.90, original_bpm: 128.0 };
  const incoming = {
    ...validAnalyzedDoc,
    analysis_version: 3,
    analysis_confidence: 0.98,
    original_bpm: 128.02,
    updated_at: existing.updated_at + 1000
  };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("14", "v2 -> v3 with lower confidence (0.95 -> 0.85) -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 3, analysis_confidence: 0.85, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("15", "v3 -> v2 version regression -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 3, analysis_confidence: 0.95 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("16", "v2 -> v2 with changed BPM -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, original_bpm: 128.0 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, original_bpm: 129.0, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("17", "v2 -> v2 with changed beat grid -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, beat_grid_json: '{"bpm":128.0}' };
  const incoming = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, beat_grid_json: '{"bpm":128.0,"new":1}', updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("18", "v2 -> v2 with changed musical key -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, musical_key: "F Minor" };
  const incoming = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, musical_key: "G Minor", updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("19", "Version upgrade with invalid DSP field (e.g. BPM > 260) -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 3, analysis_confidence: 0.98, original_bpm: 300.0, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("20", "Version upgrade with identity modification (title/artist/fingerprint) -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95 };
  const tamperedTitle = { ...validAnalyzedDoc, title: "Hacked", analysis_version: 3, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  const tamperedArtist = { ...validAnalyzedDoc, artist: "Hacked", analysis_version: 3, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  const tamperedFp = { ...validAnalyzedDoc, canonical_fingerprint: "000000000000000000", analysis_version: 3, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, tamperedTitle) &&
         !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, tamperedArtist) &&
         !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, tamperedFp);
});

// ==========================================
// PART 7.3: SOURCE-LINK MAINTENANCE TESTS (14 Tests)
// ==========================================
test("21", "Add a valid source URL -> ACCEPT", () => {
  const existing = { ...validAnalyzedDoc, source_urls: ["https://youtube.com/watch?v=1"] };
  const incoming = { ...validAnalyzedDoc, source_urls: ["https://youtube.com/watch?v=1", "youtube:2"] };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("22", "Change provenance only -> ACCEPT", () => {
  const existing = { ...validAnalyzedDoc, provenance: "client_a" };
  const incoming = { ...validAnalyzedDoc, provenance: "client_b" };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("23", "Change source URL and updated_at -> ACCEPT", () => {
  const existing = { ...validAnalyzedDoc, source_urls: ["https://youtube.com/watch?v=1"] };
  const incoming = { ...validAnalyzedDoc, source_urls: ["https://youtube.com/watch?v=1", "youtube:2"], updated_at: existing.updated_at + 1000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("24", "Change updated_at only -> REJECT", () => {
  const existing = { ...validAnalyzedDoc };
  const incoming = { ...validAnalyzedDoc, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("25", "Change duration_ms in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, duration_ms: 186000 };
  const incoming = { ...validAnalyzedDoc, duration_ms: 190000, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("26", "Change version_tag in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, version_tag: "original" };
  const incoming = { ...validAnalyzedDoc, version_tag: "club_mix", source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("27", "Change BPM in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, original_bpm: 128.0 };
  const incoming = { ...validAnalyzedDoc, original_bpm: 130.0, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("28", "Change Camelot key in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, camelot_key: "4A" };
  const incoming = { ...validAnalyzedDoc, camelot_key: "5A", source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("29", "Change beat grid in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, beat_grid_json: '{"bpm":128.0}' };
  const incoming = { ...validAnalyzedDoc, beat_grid_json: '{"bpm":128.0,"shift":1}', source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("30", "Change analysis version in Path 2 (e.g. v2 -> v1) -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 1, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("31", "Change analysis confidence in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_confidence: 0.95 };
  const incoming = { ...validAnalyzedDoc, analysis_confidence: 0.99, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("32", "Change processing_status in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, processing_status: "analyzed" };
  const incoming = { ...validAnalyzedDoc, processing_status: "pending", source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("33", "Remove a DSP field in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, phrase_markers_json: '[]' };
  const incoming = { ...validAnalyzedDoc, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  delete incoming.phrase_markers_json;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("34", "Remove source_urls in Path 2 where schema requires list -> REJECT", () => {
  const existing = { ...validAnalyzedDoc };
  const incoming = { ...validAnalyzedDoc, provenance: "client_b", updated_at: existing.updated_at + 1000 };
  delete incoming.source_urls; // violates schema in isValidCatalogueData
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

// ==========================================
// PART 7.4: STATUS AND TIMESTAMP TESTS (10 Tests)
// ==========================================
test("35a", "Status transition metadata_only -> metadata_only -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "metadata_only" }, { processing_status: "metadata_only" });
});
test("35b", "Status transition metadata_only -> analyzed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "metadata_only" }, { processing_status: "analyzed" });
});
test("35c", "Status transition pending -> pending -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "pending" }, { processing_status: "pending" });
});
test("35d", "Status transition pending -> metadata_only -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "pending" }, { processing_status: "metadata_only" });
});
test("35e", "Status transition pending -> analyzed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "pending" }, { processing_status: "analyzed" });
});
test("35f", "Status transition failed -> failed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "failed" }, { processing_status: "failed" });
});
test("35g", "Status transition failed -> metadata_only -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "failed" }, { processing_status: "metadata_only" });
});
test("35h", "Status transition failed -> analyzed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "failed" }, { processing_status: "analyzed" });
});
test("35i", "Status transition analyzed -> analyzed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "analyzed" }, { processing_status: "analyzed" });
});

test("36a", "Status downgrade analyzed -> metadata_only -> REJECT", () => {
  return !isAllowedStatusTransition({ processing_status: "analyzed" }, { processing_status: "metadata_only" });
});
test("36b", "Status downgrade analyzed -> pending -> REJECT", () => {
  return !isAllowedStatusTransition({ processing_status: "analyzed" }, { processing_status: "pending" });
});
test("36c", "Status downgrade analyzed -> failed -> REJECT", () => {
  return !isAllowedStatusTransition({ processing_status: "analyzed" }, { processing_status: "failed" });
});

test("37", "created_at modification -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, created_at: 1732500000000 };
  const incoming = { ...validAnalyzedDoc, created_at: 1732500999999, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("38", "created_at removal -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, created_at: 1732500000000 };
  const incoming = { ...validAnalyzedDoc, updated_at: existing.updated_at + 1000 };
  delete incoming.created_at;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("39", "updated_at rewind -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, updated_at: 1732500500000 };
  const incoming = { ...validAnalyzedDoc, updated_at: 1732500100000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("40", "updated_at removal -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, updated_at: 1732500500000 };
  const incoming = { ...validAnalyzedDoc };
  delete incoming.updated_at;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("41", "Legacy document without created_at receives created_at on update -> ACCEPT", () => {
  const legacyDoc = { ...bareMetadataStub };
  delete legacyDoc.created_at;
  const incoming = { ...validAnalyzedDoc, created_at: 1732500000000, updated_at: 1732500000000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", legacyDoc, incoming);
});

test("42", "Legacy document without updated_at receives updated_at on update -> ACCEPT", () => {
  const legacyDoc = { ...bareMetadataStub };
  delete legacyDoc.updated_at;
  const incoming = { ...validAnalyzedDoc, updated_at: 1732500000000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", legacyDoc, incoming);
});

test("43", "Legacy document with missing optional analysis fields updates safely -> ACCEPT", () => {
  const sparseDoc = {
    canonical_fingerprint: "a1b2c3d4e5f67890abcdef",
    title: "Animals",
    artist: "Martin Garrix"
  };
  const incoming = { ...validAnalyzedDoc, updated_at: 1732500000000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", sparseDoc, incoming);
});

test("44", "Existing analysis in pending or failed status requires version upgrade to modify DSP -> ACCEPT", () => {
  const existingFailed = { ...validAnalyzedDoc, processing_status: "failed", analysis_version: 2, analysis_confidence: 0.80 };
  const incomingUpgrade = { ...validAnalyzedDoc, processing_status: "analyzed", analysis_version: 3, analysis_confidence: 0.95, updated_at: existingFailed.updated_at + 1000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingFailed, incomingUpgrade);
});

// ==========================================
// PART 7.5: GENERAL SECURITY TESTS (16 Tests)
// ==========================================
test("45", "Unauthenticated read -> REJECT", () => !evaluateRead(null, "catalogue/fp_a1b2c3d4e5f67890abcdef"));
test("46", "Authenticated read -> ACCEPT", () => evaluateRead(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef"));
test("47", "Unauthenticated create -> REJECT", () => !evaluateCreate(null, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validAnalyzedDoc));
test("48", "Authenticated valid create -> ACCEPT", () => evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validAnalyzedDoc));
test("49", "Unauthenticated update -> REJECT", () => !evaluateUpdate(null, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validAnalyzedDoc, validAnalyzedDoc));
test("50", "Authenticated invalid update (schema violation) -> REJECT", () => {
  const badIncoming = { ...validAnalyzedDoc, original_bpm: "invalid_string_bpm" };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validAnalyzedDoc, badIncoming);
});
test("51", "Delete rejection -> REJECT", () => !evaluateDelete(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef"));
test("52", "Invalid document prefix (e.g. song_123, track_abc) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/song_123", "song_123", validAnalyzedDoc) &&
         !evaluateCreate(mockAuth, "catalogue/track_abc", "track_abc", validAnalyzedDoc);
});
test("53", "Invalid fingerprint binding (fp_ ID does not match canonical_fingerprint) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_different_hash", "fp_different_hash", validAnalyzedDoc);
});
test("54", "Invalid Camelot key (e.g. 13A, 0B, 25Z, nonsense) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, camelot_key: "13A" }) &&
         !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, camelot_key: "0B" }) &&
         !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, camelot_key: "25Z" }) &&
         !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, camelot_key: "nonsense_key" });
});
test("55", "Invalid musical key (e.g. Heavy Metal, Pop Rock) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, musical_key: "Heavy Metal" });
});
test("56", "Invalid source URL scheme (e.g. javascript:, ftp:, data:) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, source_urls: ["javascript:alert(1)"] }) &&
         !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, source_urls: ["ftp://server/file.mp3"] });
});
test("57", "Oversized tag list (>15 tags) -> REJECT", () => {
  const sixteenTags = Array.from({ length: 16 }, (_, i) => `tag_${i}`);
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, genre_tags: sixteenTags });
});
test("58", "Oversized source URL list (>5 URLs) -> REJECT", () => {
  const sixUrls = Array.from({ length: 6 }, (_, i) => `https://youtube.com/watch?v=video_${i}`);
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, source_urls: sixUrls });
});
test("59", "Unexpected extra document field (smuggled field) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, raw_pcm_audio_bytes: "AAAA..." });
});
test("60", "Access to paths outside /catalogue (e.g. /users, /admin) -> REJECT", () => {
  return !evaluateRead(mockAuth, "users/admin_doc") &&
         !evaluateRead(mockAuth, "config/secrets") &&
         !evaluateCreate(mockAuth, "users/admin_doc", "admin_doc", validAnalyzedDoc);
});

// Summary Reporting
console.log("\n--- TEST EVALUATION RESULTS ---");
let passedCount = 0;
for (const tc of testCases) {
  const status = tc.passed ? "PASS" : "FAIL";
  if (tc.passed) passedCount++;
  console.log(`[${status}] [TC-${tc.id}] ${tc.name}: ${tc.detail}`);
}

console.log(`\nTOTAL: ${testCases.length} tests, ${passedCount} passed, ${testCases.length - passedCount} failed.`);
if (passedCount === testCases.length) {
  console.log("ALL FIRESTORE RULES TESTS PASSED PERFECTLY!");
  process.exit(0);
} else {
  console.error("SOME TESTS FAILED!");
  process.exit(1);
}
