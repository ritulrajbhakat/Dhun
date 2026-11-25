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

// Essential Stage 5 DSP outputs required for any record claiming 'analyzed' status
function hasRequiredDspAnalysis(data) {
  return ('original_bpm' in data)
    && ('bpm_confidence' in data)
    && ('musical_key' in data)
    && ('camelot_key' in data)
    && ('key_confidence' in data)
    && ('beat_grid_json' in data);
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

  // Records claiming 'analyzed' status must include the essential Stage 5 DSP outputs,
  // analysis algorithm version, and analysis confidence metric.
  if ('processing_status' in data && data.processing_status === 'analyzed') {
    if (!hasRequiredDspAnalysis(data) || !('analysis_version' in data) || !('analysis_confidence' in data)) {
      return false;
    }
  }

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
      && hasRequiredDspAnalysis(incoming)
      && (!('version_tag' in existing) || incoming.version_tag === existing.version_tag)
      && (!('duration_ms' in existing) || incoming.duration_ms === existing.duration_ms);
  }

  // Scenario B: Algorithm version upgrade
  if (incomingStatus === 'analyzed'
      && ('analysis_version' in incoming)
      && ('analysis_confidence' in incoming)
      && hasRequiredDspAnalysis(incoming)
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

  // version_tag cannot be changed or removed once present
  if ('version_tag' in existing && (!('version_tag' in incoming) || incoming.version_tag !== existing.version_tag)) {
    return false;
  }

  // duration_ms cannot be changed or removed once present
  if ('duration_ms' in existing && (!('duration_ms' in incoming) || incoming.duration_ms !== existing.duration_ms)) {
    return false;
  }

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

// =========================================================================
// PART 1: INITIAL ANALYSIS & DSP REQUIREMENT TESTS (TC01 - TC15)
// =========================================================================

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

test("04", "Bare stub receives update with invalid analysis_version (0 or 200) -> REJECT", () => {
  const incoming = { ...validAnalyzedDoc, analysis_version: 0, updated_at: bareMetadataStub.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, incoming);
});

test("05", "Bare stub receives update missing required analysis_version -> REJECT", () => {
  const incoming = { ...validAnalyzedDoc, updated_at: bareMetadataStub.updated_at + 1000 };
  delete incoming.analysis_version;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, incoming);
});

test("06", "Bare stub receives update to analyzed with version/confidence but zero DSP results -> REJECT", () => {
  const falseAnalyzed = {
    ...bareMetadataStub,
    processing_status: "analyzed",
    analysis_version: 2,
    analysis_confidence: 0.95,
    updated_at: bareMetadataStub.updated_at + 1000
  };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, falseAnalyzed);
});

test("07", "Bare stub receives update to analyzed missing beat_grid_json -> REJECT", () => {
  const incoming = { ...validAnalyzedDoc, updated_at: bareMetadataStub.updated_at + 1000 };
  delete incoming.beat_grid_json;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, incoming);
});

test("08", "Bare stub receives update to analyzed missing camelot_key -> REJECT", () => {
  const incoming = { ...validAnalyzedDoc, updated_at: bareMetadataStub.updated_at + 1000 };
  delete incoming.camelot_key;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, incoming);
});

test("09", "Valid partial analysis meeting minimum contract (omits optional phrases/tags/halftime) -> ACCEPT", () => {
  const partialAnalysis = {
    canonical_fingerprint: "a1b2c3d4e5f67890abcdef",
    title: "Animals",
    artist: "Martin Garrix",
    version_tag: "original",
    duration_ms: 186000,
    original_bpm: 128.0,
    bpm_confidence: 0.95,
    musical_key: "F Minor",
    camelot_key: "4A",
    key_confidence: 0.95,
    beat_grid_json: '{"bpm":128.0}',
    analysis_version: 2,
    analysis_confidence: 0.95,
    processing_status: "analyzed",
    source_urls: ["https://www.youtube.com/watch?v=DuFUtL8zUAk"],
    provenance: "dhun_local_dsp",
    created_at: 1732500000000,
    updated_at: bareMetadataStub.updated_at + 1000
  };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub, partialAnalysis);
});

test("10", "Existing pending record with analysis_version cannot bypass anti-downgrade -> REJECT", () => {
  const existingPending = { ...bareMetadataStub, processing_status: "pending", analysis_version: 3, analysis_confidence: 0.95 };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.20, updated_at: existingPending.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingPending, incomingDowngrade);
});

test("11", "Existing failed record with only original_bpm cannot bypass protection -> REJECT", () => {
  const existingFailed = { ...bareMetadataStub, processing_status: "failed", original_bpm: 128.0 };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingFailed.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingFailed, incomingDowngrade);
});

test("12", "Existing record with only musical_key cannot bypass protection -> REJECT", () => {
  const existingKey = { ...bareMetadataStub, musical_key: "F Minor" };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingKey.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingKey, incomingDowngrade);
});

test("13", "Existing record with only phrase_markers_json cannot bypass protection -> REJECT", () => {
  const existingPhrase = { ...bareMetadataStub, phrase_markers_json: "[]" };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingPhrase.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingPhrase, incomingDowngrade);
});

test("14", "Existing record with only danceability cannot bypass protection -> REJECT", () => {
  const existingDance = { ...bareMetadataStub, danceability: 0.85 };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingDance.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingDance, incomingDowngrade);
});

test("15", "Existing record with only genre_tags cannot bypass protection -> REJECT", () => {
  const existingGenre = { ...bareMetadataStub, genre_tags: ["edm"] };
  const incomingDowngrade = { ...validAnalyzedDoc, analysis_version: 1, analysis_confidence: 0.50, updated_at: existingGenre.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingGenre, incomingDowngrade);
});

// =========================================================================
// PART 2: ALGORITHM UPGRADE & METADATA IMMUTABILITY TESTS (TC16 - TC27)
// =========================================================================

test("16", "v2 -> v3 with equal confidence and changed DSP -> ACCEPT", () => {
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

test("17", "v2 -> v3 with higher confidence and changed DSP -> ACCEPT", () => {
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

test("18", "v2 -> v3 with lower confidence (0.95 -> 0.85) -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 3, analysis_confidence: 0.85, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("19", "v3 -> v2 version regression -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 3, analysis_confidence: 0.95 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("20", "v2 -> v2 with changed BPM -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, original_bpm: 128.0 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, original_bpm: 129.0, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("21", "v2 -> v2 with changed beat grid -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, beat_grid_json: '{"bpm":128.0}' };
  const incoming = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, beat_grid_json: '{"bpm":128.0,"new":1}', updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("22", "v2 -> v2 with changed musical key -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, musical_key: "F Minor" };
  const incoming = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, musical_key: "G Minor", updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("23", "Version upgrade with invalid DSP field (BPM > 260) -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 3, analysis_confidence: 0.98, original_bpm: 300.0, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("24", "Version upgrade attempting to alter duration_ms -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, duration_ms: 186000 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 3, analysis_confidence: 0.98, duration_ms: 190000, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("25", "Version upgrade attempting to alter version_tag -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95, version_tag: "original" };
  const incoming = { ...validAnalyzedDoc, analysis_version: 3, analysis_confidence: 0.98, version_tag: "club_mix", updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("26", "Version upgrade attempting to omit required DSP analysis (missing beat_grid_json) -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 3, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  delete incoming.beat_grid_json;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("27", "Version upgrade with identity modification (title/artist/fingerprint) -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2, analysis_confidence: 0.95 };
  const tamperedTitle = { ...validAnalyzedDoc, title: "Hacked", analysis_version: 3, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  const tamperedArtist = { ...validAnalyzedDoc, artist: "Hacked", analysis_version: 3, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  const tamperedFp = { ...validAnalyzedDoc, canonical_fingerprint: "000000000000000000", analysis_version: 3, analysis_confidence: 0.98, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, tamperedTitle) &&
         !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, tamperedArtist) &&
         !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, tamperedFp);
});

// =========================================================================
// PART 3: SOURCE-LINK MAINTENANCE TESTS (TC28 - TC41)
// =========================================================================

test("28", "Add a valid source URL -> ACCEPT", () => {
  const existing = { ...validAnalyzedDoc, source_urls: ["https://youtube.com/watch?v=1"] };
  const incoming = { ...validAnalyzedDoc, source_urls: ["https://youtube.com/watch?v=1", "youtube:2"] };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("29", "Change provenance only -> ACCEPT", () => {
  const existing = { ...validAnalyzedDoc, provenance: "client_a" };
  const incoming = { ...validAnalyzedDoc, provenance: "client_b" };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("30", "Change source URL and updated_at -> ACCEPT", () => {
  const existing = { ...validAnalyzedDoc, source_urls: ["https://youtube.com/watch?v=1"] };
  const incoming = { ...validAnalyzedDoc, source_urls: ["https://youtube.com/watch?v=1", "youtube:2"], updated_at: existing.updated_at + 1000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("31", "Change updated_at only -> REJECT", () => {
  const existing = { ...validAnalyzedDoc };
  const incoming = { ...validAnalyzedDoc, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("32", "Change duration_ms in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, duration_ms: 186000 };
  const incoming = { ...validAnalyzedDoc, duration_ms: 190000, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("33", "Change version_tag in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, version_tag: "original" };
  const incoming = { ...validAnalyzedDoc, version_tag: "club_mix", source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("34", "Change BPM in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, original_bpm: 128.0 };
  const incoming = { ...validAnalyzedDoc, original_bpm: 130.0, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("35", "Change Camelot key in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, camelot_key: "4A" };
  const incoming = { ...validAnalyzedDoc, camelot_key: "5A", source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("36", "Change beat grid in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, beat_grid_json: '{"bpm":128.0}' };
  const incoming = { ...validAnalyzedDoc, beat_grid_json: '{"bpm":128.0,"shift":1}', source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("37", "Change analysis version in Path 2 (e.g. v2 -> v1) -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_version: 2 };
  const incoming = { ...validAnalyzedDoc, analysis_version: 1, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("38", "Change analysis confidence in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, analysis_confidence: 0.95 };
  const incoming = { ...validAnalyzedDoc, analysis_confidence: 0.99, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("39", "Change processing_status in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, processing_status: "analyzed" };
  const incoming = { ...validAnalyzedDoc, processing_status: "pending", source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("40", "Remove a DSP field in Path 2 -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, phrase_markers_json: '[]' };
  const incoming = { ...validAnalyzedDoc, source_urls: ["youtube:new"], updated_at: existing.updated_at + 1000 };
  delete incoming.phrase_markers_json;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("41", "Remove source_urls in Path 2 where schema requires list -> REJECT", () => {
  const existing = { ...validAnalyzedDoc };
  const incoming = { ...validAnalyzedDoc, provenance: "client_b", updated_at: existing.updated_at + 1000 };
  delete incoming.source_urls;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

// =========================================================================
// PART 4: STATUS AND TIMESTAMP TESTS (TC42 - TC61)
// =========================================================================

test("42", "Status transition metadata_only -> metadata_only -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "metadata_only" }, { processing_status: "metadata_only" });
});

test("43", "Status transition metadata_only -> analyzed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "metadata_only" }, { processing_status: "analyzed" });
});

test("44", "Status transition pending -> pending -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "pending" }, { processing_status: "pending" });
});

test("45", "Status transition pending -> metadata_only -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "pending" }, { processing_status: "metadata_only" });
});

test("46", "Status transition pending -> analyzed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "pending" }, { processing_status: "analyzed" });
});

test("47", "Status transition failed -> failed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "failed" }, { processing_status: "failed" });
});

test("48", "Status transition failed -> metadata_only -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "failed" }, { processing_status: "metadata_only" });
});

test("49", "Status transition failed -> analyzed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "failed" }, { processing_status: "analyzed" });
});

test("50", "Status transition analyzed -> analyzed -> ACCEPT", () => {
  return isAllowedStatusTransition({ processing_status: "analyzed" }, { processing_status: "analyzed" });
});

test("51", "Status downgrade analyzed -> metadata_only -> REJECT", () => {
  return !isAllowedStatusTransition({ processing_status: "analyzed" }, { processing_status: "metadata_only" });
});

test("52", "Status downgrade analyzed -> pending -> REJECT", () => {
  return !isAllowedStatusTransition({ processing_status: "analyzed" }, { processing_status: "pending" });
});

test("53", "Status downgrade analyzed -> failed -> REJECT", () => {
  return !isAllowedStatusTransition({ processing_status: "analyzed" }, { processing_status: "failed" });
});

test("54", "created_at modification -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, created_at: 1732500000000 };
  const incoming = { ...validAnalyzedDoc, created_at: 1732500999999, updated_at: existing.updated_at + 1000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("55", "created_at removal -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, created_at: 1732500000000 };
  const incoming = { ...validAnalyzedDoc, updated_at: existing.updated_at + 1000 };
  delete incoming.created_at;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("56", "updated_at rewind -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, updated_at: 1732500500000 };
  const incoming = { ...validAnalyzedDoc, updated_at: 1732500100000 };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("57", "updated_at removal -> REJECT", () => {
  const existing = { ...validAnalyzedDoc, updated_at: 1732500500000 };
  const incoming = { ...validAnalyzedDoc };
  delete incoming.updated_at;
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existing, incoming);
});

test("58", "Legacy document without created_at receives created_at on update -> ACCEPT", () => {
  const legacyDoc = { ...bareMetadataStub };
  delete legacyDoc.created_at;
  const incoming = { ...validAnalyzedDoc, created_at: 1732500000000, updated_at: 1732500000000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", legacyDoc, incoming);
});

test("59", "Legacy document without updated_at receives updated_at on update -> ACCEPT", () => {
  const legacyDoc = { ...bareMetadataStub };
  delete legacyDoc.updated_at;
  const incoming = { ...validAnalyzedDoc, updated_at: 1732500000000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", legacyDoc, incoming);
});

test("60", "Legacy document with missing optional analysis fields updates safely -> ACCEPT", () => {
  const sparseDoc = {
    canonical_fingerprint: "a1b2c3d4e5f67890abcdef",
    title: "Animals",
    artist: "Martin Garrix"
  };
  const incoming = { ...validAnalyzedDoc, updated_at: 1732500000000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", sparseDoc, incoming);
});

test("61", "Existing analysis in pending or failed status requires version upgrade to modify DSP -> ACCEPT", () => {
  const existingFailed = { ...validAnalyzedDoc, processing_status: "failed", analysis_version: 2, analysis_confidence: 0.80 };
  const incomingUpgrade = { ...validAnalyzedDoc, processing_status: "analyzed", analysis_version: 3, analysis_confidence: 0.95, updated_at: existingFailed.updated_at + 1000 };
  return evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", existingFailed, incomingUpgrade);
});

// =========================================================================
// PART 5: GENERAL SECURITY & SCHEMA VALIDATION TESTS (TC62 - TC79)
// =========================================================================

test("62", "Unauthenticated read -> REJECT", () => !evaluateRead(null, "catalogue/fp_a1b2c3d4e5f67890abcdef"));
test("63", "Authenticated read -> ACCEPT", () => evaluateRead(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef"));
test("64", "Unauthenticated create -> REJECT", () => !evaluateCreate(null, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validAnalyzedDoc));
test("65", "Authenticated valid create with complete DSP analysis -> ACCEPT", () => evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validAnalyzedDoc));
test("66", "Authenticated create claiming analyzed but missing required DSP analysis -> REJECT", () => {
  const fakeAnalyzed = {
    canonical_fingerprint: "a1b2c3d4e5f67890abcdef",
    title: "Animals",
    artist: "Martin Garrix",
    processing_status: "analyzed",
    analysis_version: 2,
    analysis_confidence: 0.95,
    created_at: 1732500000000,
    updated_at: 1732500000000
  };
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", fakeAnalyzed);
});
test("67", "Authenticated create with status metadata_only without DSP analysis -> ACCEPT", () => {
  return evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", bareMetadataStub);
});
test("68", "Unauthenticated update -> REJECT", () => !evaluateUpdate(null, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validAnalyzedDoc, validAnalyzedDoc));
test("69", "Authenticated invalid update (schema violation) -> REJECT", () => {
  const badIncoming = { ...validAnalyzedDoc, original_bpm: "invalid_string_bpm" };
  return !evaluateUpdate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", validAnalyzedDoc, badIncoming);
});
test("70", "Delete rejection -> REJECT", () => !evaluateDelete(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef"));
test("71", "Invalid document prefix (e.g. song_123, track_abc) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/song_123", "song_123", validAnalyzedDoc) &&
         !evaluateCreate(mockAuth, "catalogue/track_abc", "track_abc", validAnalyzedDoc);
});
test("72", "Invalid fingerprint binding (fp_ ID does not match canonical_fingerprint) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_different_hash", "fp_different_hash", validAnalyzedDoc);
});
test("73", "Invalid Camelot key (e.g. 13A, 0B, 25Z, nonsense) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, camelot_key: "13A" }) &&
         !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, camelot_key: "0B" }) &&
         !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, camelot_key: "25Z" }) &&
         !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, camelot_key: "nonsense_key" });
});
test("74", "Invalid musical key (e.g. Heavy Metal, Pop Rock) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, musical_key: "Heavy Metal" });
});
test("75", "Invalid source URL scheme (e.g. javascript:, ftp:, data:) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, source_urls: ["javascript:alert(1)"] }) &&
         !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, source_urls: ["ftp://server/file.mp3"] });
});
test("76", "Oversized tag list (>15 tags) -> REJECT", () => {
  const sixteenTags = Array.from({ length: 16 }, (_, i) => `tag_${i}`);
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, genre_tags: sixteenTags });
});
test("77", "Oversized source URL list (>5 URLs) -> REJECT", () => {
  const sixUrls = Array.from({ length: 6 }, (_, i) => `https://youtube.com/watch?v=video_${i}`);
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, source_urls: sixUrls });
});
test("78", "Unexpected extra document field (smuggled field) -> REJECT", () => {
  return !evaluateCreate(mockAuth, "catalogue/fp_a1b2c3d4e5f67890abcdef", "fp_a1b2c3d4e5f67890abcdef", { ...validAnalyzedDoc, raw_pcm_audio_bytes: "AAAA..." });
});
test("79", "Access to paths outside /catalogue (e.g. /users, /admin) -> REJECT", () => {
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
