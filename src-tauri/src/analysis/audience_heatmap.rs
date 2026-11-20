use anyhow::{anyhow, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::{
    HeatmapMarker, HookCandidate, PhraseMarker, ReplayRegion,
    TimelineAlignmentStatus, TrackAudienceHeatmap, TrackTimingProfile,
};

/// Parser version constant for audit and reproducibility
pub const HEATMAP_PARSER_VERSION: i32 = 1;

/// Parsed heatmap data containing raw markers and explicitly labelled regions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedHeatmapData {
    pub markers: Vec<HeatmapMarker>,
    pub labelled_regions: Vec<ReplayRegion>,
    pub parser_version: i32,
    #[serde(default = "default_source_object")]
    pub source_object: String,
}

fn default_source_object() -> String {
    "unknown".to_string()
}

pub struct AudienceHeatmapAdapter;

impl AudienceHeatmapAdapter {
    /// Strictly extracts and validates an 11-character YouTube video ID from supported URL patterns.
    /// Rejects non-YouTube domains, missing video IDs, malformed IDs, and arbitrary non-URLs.
    /// YouTube URLs are used exclusively as metadata references: DHUN NEVER plays videos or embeds players.
    pub fn extract_video_id_from_url(url_str: &str) -> Result<String> {
        let trimmed = url_str.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Empty URL provided"));
        }

        let parsed = Url::parse(trimmed).map_err(|e| anyhow!("Invalid URL format: {}", e))?;

        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow!("URL missing host"))?
            .to_lowercase();

        let is_youtube = host == "youtube.com"
            || host == "www.youtube.com"
            || host == "m.youtube.com"
            || host == "music.youtube.com"
            || host == "youtu.be";

        if !is_youtube {
            return Err(anyhow!(
                "Unsupported domain '{}': only youtube.com and youtu.be are supported for audience heatmap metadata",
                host
            ));
        }

        let raw_id = if host == "youtu.be" {
            let path = parsed.path().trim_start_matches('/');
            path.split('/').next().unwrap_or("").to_string()
        } else {
            let path = parsed.path();
            if path == "/watch" {
                parsed
                    .query_pairs()
                    .find(|(k, _)| k == "v")
                    .map(|(_, v)| v.to_string())
                    .ok_or_else(|| anyhow!("YouTube watch URL missing required '?v=' video ID parameter"))?
            } else if path.starts_with("/embed/") {
                path.trim_start_matches("/embed/").split('/').next().unwrap_or("").to_string()
            } else if path.starts_with("/v/") {
                path.trim_start_matches("/v/").split('/').next().unwrap_or("").to_string()
            } else {
                return Err(anyhow!("Unsupported YouTube URL path '{}'", path));
            }
        };

        let clean_id = raw_id.split('?').next().unwrap_or(&raw_id);
        let clean_id = clean_id.split('&').next().unwrap_or(clean_id);

        if !Self::is_valid_video_id(clean_id) {
            return Err(anyhow!(
                "Invalid YouTube video ID '{}': must be exactly 11 characters matching [a-zA-Z0-9_-]",
                clean_id
            ));
        }

        Ok(clean_id.to_string())
    }

    /// Verifies if a string is a valid YouTube video ID (exactly 11 characters matching [a-zA-Z0-9_-])
    pub fn is_valid_video_id(id: &str) -> bool {
        if id.len() != 11 {
            return false;
        }
        id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }

    /// Safely extracts a balanced JSON object from HTML starting after a prefix needle.
    /// Tracks string escapes and brace nesting levels to avoid fragile substring splits.
    pub fn extract_balanced_json(html: &str, needle: &str) -> Option<serde_json::Value> {
        let pos = html.find(needle)?;
        let start_pos = pos + needle.len();
        let bytes = html.as_bytes();
        let len = bytes.len();

        let mut idx = start_pos;
        while idx < len && (bytes[idx] == b' ' || bytes[idx] == b'\t' || bytes[idx] == b'\r' || bytes[idx] == b'\n' || bytes[idx] == b'=') {
            idx += 1;
        }

        if idx >= len || (bytes[idx] != b'{' && bytes[idx] != b'[') {
            return None;
        }

        let start_idx = idx;
        let mut depth: usize = 0;
        let mut in_string = false;
        let mut is_escaped = false;

        while idx < len {
            let b = bytes[idx];

            if in_string {
                if is_escaped {
                    is_escaped = false;
                } else if b == b'\\' {
                    is_escaped = true;
                } else if b == b'"' {
                    in_string = false;
                }
            } else {
                match b {
                    b'"' => in_string = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        if depth > 0 {
                            depth -= 1;
                            if depth == 0 {
                                let json_str = &html[start_idx..=idx];
                                return serde_json::from_str(json_str).ok();
                            }
                        }
                    }
                    _ => {}
                }
            }
            idx += 1;
        }

        None
    }

    /// Extracts audience heatmap markers from watch page HTML.
    /// Inspects both `ytInitialData` and `ytInitialPlayerResponse` without preferring either blindly.
    /// Selects whichever object actually contains valid heatmap markers and reports the source.
    /// If neither contains heatmap markers, returns a typed Err with an honest diagnostic reason.
    pub fn extract_heatmap_from_watch_html(html: &str) -> Result<ParsedHeatmapData> {
        let mut yt_initial_data = None;
        let mut yt_player_response = None;

        // Try extracting ytInitialData (supports variable declarations and window properties)
        for needle in &["var ytInitialData = ", "ytInitialData = ", "window[\"ytInitialData\"] = "] {
            if let Some(val) = Self::extract_balanced_json(html, needle) {
                yt_initial_data = Some(val);
                break;
            }
        }

        // Try extracting ytInitialPlayerResponse
        for needle in &["var ytInitialPlayerResponse = ", "ytInitialPlayerResponse = ", "window[\"ytInitialPlayerResponse\"] = "] {
            if let Some(val) = Self::extract_balanced_json(html, needle) {
                yt_player_response = Some(val);
                break;
            }
        }

        // Attempt 1: Check ytInitialData (standard desktop SSR location for macroMarkersListEntity)
        if let Some(ref data) = yt_initial_data {
            if let Ok(mut parsed) = Self::parse_player_response(data) {
                if !parsed.markers.is_empty() {
                    parsed.source_object = "ytInitialData".to_string();
                    return Ok(parsed);
                }
            }
        }

        // Attempt 2: Check ytInitialPlayerResponse (historical or mobile SSR location)
        if let Some(ref resp) = yt_player_response {
            if let Ok(mut parsed) = Self::parse_player_response(resp) {
                if !parsed.markers.is_empty() {
                    parsed.source_object = "ytInitialPlayerResponse".to_string();
                    return Ok(parsed);
                }
            }
        }

        // If neither produced valid markers, report diagnostic reason honestly
        if yt_initial_data.is_some() || yt_player_response.is_some() {
            Err(anyhow!(
                "Audience heatmap unavailable: Watch page JSON was parsed successfully, but neither ytInitialData nor ytInitialPlayerResponse contained heatmap markers (this video may lack sufficient audience replay data or markers are ungenerated)"
            ))
        } else {
            Err(anyhow!(
                "Audience heatmap unavailable: Neither ytInitialData nor ytInitialPlayerResponse was found in watch HTML (page may be bot-gated, consent-blocked, or uses an unsupported markup structure)"
            ))
        }
    }

    /// Fetches public YouTube watch page and attempts to extract embedded player response with heatmap markers.
    /// Strictly metadata-only: NEVER plays video, loads video streams, or creates video players.
    /// Honestly reports errors if YouTube rate limits (HTTP 429), blocks, or gates access.
    pub async fn fetch_public_player_heatmap(
        video_id: &str,
        client: &reqwest::Client,
    ) -> Result<ParsedHeatmapData> {
        if !Self::is_valid_video_id(video_id) {
            return Err(anyhow!("Invalid video ID for heatmap fetch: {}", video_id));
        }

        let url = format!("https://www.youtube.com/watch?v={}&bpctr=9999999999&has_verified=1", video_id);

        let resp = client
            .get(&url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .header("Accept-Language", "en-US,en;q=0.9")
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| anyhow!("Network request to YouTube watch page failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(anyhow!("YouTube returned non-success HTTP status: {}", status));
        }

        let html = resp
            .text()
            .await
            .map_err(|e| anyhow!("Failed to read YouTube response body: {}", e))?;

        Self::extract_heatmap_from_watch_html(&html)
    }

    /// Extracts raw heatmap markers and labelled regions from arbitrary YouTube player response JSON.
    /// Supports multiple historical and modern response variants without leaking structure details.
    pub fn parse_player_response(json_val: &serde_json::Value) -> Result<ParsedHeatmapData> {
        let mut markers = Vec::new();
        let mut labelled_regions = Vec::new();

        // 1. Direct array fixture support: [ { "start_ms": ..., "duration_ms": ..., "intensity": ... } ]
        if let Some(arr) = json_val.as_array() {
            for item in arr {
                if let Some(m) = Self::parse_single_marker(item) {
                    markers.push(m);
                }
            }
            if !markers.is_empty() {
                markers.sort_by_key(|m| m.start_ms);
                return Ok(ParsedHeatmapData {
                    markers,
                    labelled_regions,
                    parser_version: HEATMAP_PARSER_VERSION,
                    source_object: "json_payload".to_string(),
                });
            }
        }

        // 2. Format A: `frameworkUpdates.entityBatchUpdate.mutations`
        if let Some(mutations) = json_val
            .pointer("/frameworkUpdates/entityBatchUpdate/mutations")
            .and_then(|v| v.as_array())
        {
            for mut_item in mutations {
                // Check for heatMarkerRenderer or markers in payload
                if let Some(marker_arr) = mut_item
                    .pointer("/payload/macroMarkersListEntity/markersList/markers")
                    .or_else(|| mut_item.pointer("/payload/heatMarkerRenderer/markers"))
                    .and_then(|v| v.as_array())
                {
                    for item in marker_arr {
                        if let Some(m) = Self::parse_single_marker(item) {
                            markers.push(m);
                        }
                    }
                }

                // Check for single marker renderer inside mutation
                if let Some(m) = Self::parse_single_marker(mut_item) {
                    markers.push(m);
                }

                // Check for labelled ranges (e.g. most replayed highlighted segment)
                if let Some(path_renderer) = mut_item.pointer("/payload/heatMarkersPathRenderer") {
                    if let Some(reg) = Self::parse_labelled_region(path_renderer) {
                        labelled_regions.push(reg);
                    }
                }
            }
        }

        // 3. Format B: `playerOverlays.playerOverlayRenderer.decoratedPlayerBarRenderer.playerBar.multiMarkersPlayerBarRenderer.markersMap`
        if let Some(markers_map) = json_val
            .pointer("/playerOverlays/playerOverlayRenderer/decoratedPlayerBarRenderer/playerBar/multiMarkersPlayerBarRenderer/markersMap")
            .and_then(|v| v.as_array())
        {
            for entry in markers_map {
                let key = entry.get("key").and_then(|k| k.as_str()).unwrap_or("");
                if key.contains("HEATMAP") || key.is_empty() {
                    if let Some(heat_markers) = entry
                        .pointer("/value/heatmap/heatMarkers")
                        .or_else(|| entry.pointer("/value/heatMarkers"))
                        .and_then(|v| v.as_array())
                    {
                        for item in heat_markers {
                            if let Some(renderer) = item.get("heatMarkerRenderer").or(Some(item)) {
                                if let Some(m) = Self::parse_single_marker(renderer) {
                                    markers.push(m);
                                }
                            }
                        }
                    }

                    if let Some(path_renderer) = entry.pointer("/value/heatmap/heatMarkersPathRenderer") {
                        if let Some(reg) = Self::parse_labelled_region(path_renderer) {
                            labelled_regions.push(reg);
                        }
                    }
                }
            }
        }

        // 4. Fallback search: top-level `heatMarkers` or `markers`
        if markers.is_empty() {
            if let Some(arr) = json_val.get("heatMarkers").or_else(|| json_val.get("markers")).and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(m) = Self::parse_single_marker(item) {
                        markers.push(m);
                    }
                }
            }
        }

        if markers.is_empty() && labelled_regions.is_empty() {
            return Err(anyhow!("No valid heatmap markers found in player response"));
        }

        markers.sort_by_key(|m| m.start_ms);
        Ok(ParsedHeatmapData {
            markers,
            labelled_regions,
            parser_version: HEATMAP_PARSER_VERSION,
            source_object: "json_payload".to_string(),
        })
    }

    /// Parses an individual marker object accommodating both naming conventions
    fn parse_single_marker(val: &serde_json::Value) -> Option<HeatmapMarker> {
        let node = val.get("heatMarkerRenderer").unwrap_or(val);

        // start_ms: `startMillis` (Format A) OR `timeRangeStartMillis` (Format B) OR `start_ms`
        let start_ms = node
            .get("startMillis")
            .or_else(|| node.get("timeRangeStartMillis"))
            .or_else(|| node.get("start_ms"))
            .and_then(|v| {
                if let Some(n) = v.as_i64() {
                    Some(n)
                } else if let Some(s) = v.as_str() {
                    s.parse::<i64>().ok()
                } else {
                    None
                }
            })?;

        // Negative start times are malformed
        if start_ms < 0 {
            return None;
        }

        // duration_ms: `durationMillis` (Format A) OR `markerDurationMillis` (Format B) OR `duration_ms`
        let duration_ms = node
            .get("durationMillis")
            .or_else(|| node.get("markerDurationMillis"))
            .or_else(|| node.get("duration_ms"))
            .and_then(|v| {
                if let Some(n) = v.as_i64() {
                    Some(n)
                } else if let Some(s) = v.as_str() {
                    s.parse::<i64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(2000); // Default 2s duration if omitted

        // Zero or negative durations are malformed
        if duration_ms <= 0 {
            return None;
        }

        // intensity: `intensityScoreNormalized` (Format A) OR `heatMarkerIntensityScoreNormalized` (Format B) OR `intensity`
        let intensity = node
            .get("intensityScoreNormalized")
            .or_else(|| node.get("heatMarkerIntensityScoreNormalized"))
            .or_else(|| node.get("intensity"))
            .and_then(|v| {
                if let Some(f) = v.as_f64() {
                    Some(f)
                } else if let Some(s) = v.as_str() {
                    s.parse::<f64>().ok()
                } else {
                    None
                }
            })?;

        let clamped_intensity = if intensity.is_finite() {
            intensity.clamp(0.0, 1.0)
        } else {
            0.0
        };

        Some(HeatmapMarker {
            start_ms,
            duration_ms: duration_ms.max(100),
            intensity: clamped_intensity,
        })
    }

    /// Parses YouTube's explicitly labelled highlight range
    fn parse_labelled_region(val: &serde_json::Value) -> Option<ReplayRegion> {
        let start_ms = val
            .get("visibleTimeRangeStartMillis")
            .or_else(|| val.get("start_ms"))
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))?;
        let end_ms = val
            .get("visibleTimeRangeEndMillis")
            .or_else(|| val.get("end_ms"))
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))?;

        if end_ms <= start_ms {
            return None;
        }

        let label = val
            .get("heatMarkerType")
            .or_else(|| val.get("label"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some("Most Replayed".to_string()));

        Some(ReplayRegion {
            id: Uuid::new_v4().to_string(),
            start_ms,
            end_ms,
            peak_ms: start_ms + (end_ms - start_ms) / 2,
            peak_intensity: 1.0,
            avg_intensity: 0.85,
            is_youtube_labelled: true,
            label,
        })
    }

    /// Extracts continuous high-engagement replay regions by:
    /// 1. Smoothing small fluctuations without destroying real peaks.
    /// 2. Detecting local maxima.
    /// 3. Grouping adjacent high-interest markers.
    /// 4. Identifying region start and end.
    /// 5. Merging small gaps (<= 3000 ms) when intensity remains elevated.
    /// 6. Separating distinct peaks when a meaningful valley exists.
    /// 7. Calculating maximum intensity, average intensity, and duration.
    /// 8. Retaining YouTube-labelled regions separately.
    pub fn extract_replay_regions(
        markers: &[HeatmapMarker],
        labelled_regions: &[ReplayRegion],
    ) -> Vec<ReplayRegion> {
        if markers.is_empty() {
            return labelled_regions.to_vec();
        }

        let n = markers.len();
        if n == 1 {
            let m = &markers[0];
            return vec![ReplayRegion {
                id: Uuid::new_v4().to_string(),
                start_ms: m.start_ms,
                end_ms: m.start_ms + m.duration_ms,
                peak_ms: m.start_ms,
                peak_intensity: m.intensity,
                avg_intensity: m.intensity,
                is_youtube_labelled: false,
                label: None,
            }];
        }

        // 1. 3-point Gaussian-weighted smoothing over normalized intensity values:
        //    y_0 = 0.70*y_0 + 0.30*y_1
        //    y_i = 0.25*y_{i-1} + 0.50*y_i + 0.25*y_{i+1}
        //    y_{N-1} = 0.30*y_{N-2} + 0.70*y_{N-1}
        let mut smoothed: Vec<f64> = Vec::with_capacity(n);
        smoothed.push((0.70 * markers[0].intensity + 0.30 * markers[1].intensity).clamp(0.0, 1.0));
        for i in 1..n - 1 {
            let s = 0.25 * markers[i - 1].intensity + 0.50 * markers[i].intensity + 0.25 * markers[i + 1].intensity;
            smoothed.push(s.clamp(0.0, 1.0));
        }
        smoothed.push((0.30 * markers[n - 2].intensity + 0.70 * markers[n - 1].intensity).clamp(0.0, 1.0));

        // Find global stats
        let mut max_val = 0.0f64;
        let mut sum_val = 0.0f64;
        for &val in &smoothed {
            if val > max_val {
                max_val = val;
            }
            sum_val += val;
        }
        let mean_val = sum_val / n as f64;

        // Adaptive threshold: interest must be at least 0.35 AND at least 0.50 * peak
        let min_interest = (0.35f64).max(max_val * 0.50).min(0.70);

        // 2 & 3. Group contiguous segments where smoothed intensity >= min_interest
        let mut raw_regions: Vec<ReplayRegion> = Vec::new();
        let mut in_region = false;
        let mut seg_start_idx = 0usize;

        for i in 0..n {
            let is_high = smoothed[i] >= min_interest;
            if is_high && !in_region {
                in_region = true;
                seg_start_idx = i;
            } else if !is_high && in_region {
                in_region = false;
                let seg_end_idx = i - 1;
                Self::build_region_from_slice(
                    markers,
                    &smoothed,
                    seg_start_idx,
                    seg_end_idx,
                    &mut raw_regions,
                );
            }
        }
        if in_region {
            Self::build_region_from_slice(
                markers,
                &smoothed,
                seg_start_idx,
                n - 1,
                &mut raw_regions,
            );
        }

        // 4. Merge small gaps (<= 3000 ms) only within configurable limits
        let mut merged_regions: Vec<ReplayRegion> = Vec::new();
        for reg in raw_regions {
            if let Some(last) = merged_regions.last_mut() {
                let gap_ms = reg.start_ms - last.end_ms;
                if gap_ms >= 0 && gap_ms <= 3000 {
                    // Merge adjacent regions
                    last.end_ms = reg.end_ms;
                    if reg.peak_intensity > last.peak_intensity {
                        last.peak_intensity = reg.peak_intensity;
                        last.peak_ms = reg.peak_ms;
                    }
                    last.avg_intensity = (last.avg_intensity + reg.avg_intensity) / 2.0;
                    continue;
                }
            }
            merged_regions.push(reg);
        }

        // 5. Preserve YouTube-labelled regions separately:
        // If an extracted region overlaps with a YouTube-labelled region, attach the label.
        // If a labelled region is completely outside any extracted region, retain it as a distinct region.
        for labelled in labelled_regions {
            if let Some(matching) = merged_regions.iter_mut().find(|r| {
                r.start_ms <= labelled.end_ms && r.end_ms >= labelled.start_ms
            }) {
                matching.is_youtube_labelled = true;
                if matching.label.is_none() {
                    matching.label = labelled.label.clone();
                }
            } else {
                merged_regions.push(labelled.clone());
            }
        }

        merged_regions.sort_by_key(|r| r.start_ms);
        merged_regions
    }

    /// Internal helper: builds one or more regions from a high-intensity slice,
    /// splitting distinct peaks when a meaningful valley exists (> 20% intensity drop).
    fn build_region_from_slice(
        markers: &[HeatmapMarker],
        smoothed: &[f64],
        start_idx: usize,
        end_idx: usize,
        out: &mut Vec<ReplayRegion>,
    ) {
        if start_idx > end_idx {
            return;
        }

        // Find local maxima and local minima (valleys) within this slice
        let mut peaks = Vec::new();
        let mut valleys = Vec::new();

        for i in start_idx..=end_idx {
            let v = smoothed[i];
            let prev = if i > start_idx { smoothed[i - 1] } else { v };
            let next = if i < end_idx { smoothed[i + 1] } else { v };

            if v >= prev && v >= next && v > 0.30 {
                peaks.push(i);
            } else if v <= prev && v <= next {
                valleys.push(i);
            }
        }

        // Check if there is a meaningful valley between consecutive peaks
        // A valley is meaningful if its value drops by more than 20% below both adjacent peaks
        let mut split_points = Vec::new();
        if peaks.len() >= 2 {
            for w in peaks.windows(2) {
                let p1 = w[0];
                let p2 = w[1];
                let val1 = smoothed[p1];
                let val2 = smoothed[p2];
                let min_peak = val1.min(val2);

                if let Some(&valley_idx) = valleys.iter().find(|&&idx| idx > p1 && idx < p2) {
                    let valley_val = smoothed[valley_idx];
                    if (min_peak - valley_val) >= 0.20 && valley_val < min_peak * 0.75 {
                        split_points.push(valley_idx);
                    }
                }
            }
        }

        if split_points.is_empty() {
            // Single continuous region
            let start_ms = markers[start_idx].start_ms;
            let end_ms = markers[end_idx].start_ms + markers[end_idx].duration_ms;

            let mut peak_val = 0.0f64;
            let mut peak_ms = start_ms;
            let mut sum = 0.0f64;
            let count = (end_idx - start_idx + 1) as f64;

            for i in start_idx..=end_idx {
                let val = smoothed[i];
                sum += val;
                if val > peak_val {
                    peak_val = val;
                    peak_ms = markers[i].start_ms;
                }
            }

            out.push(ReplayRegion {
                id: Uuid::new_v4().to_string(),
                start_ms,
                end_ms,
                peak_ms,
                peak_intensity: (peak_val * 1000.0).round() / 1000.0,
                avg_intensity: ((sum / count) * 1000.0).round() / 1000.0,
                is_youtube_labelled: false,
                label: None,
            });
        } else {
            // Split at valleys into multiple sub-regions
            let mut curr_start = start_idx;
            for &split_idx in &split_points {
                if split_idx > curr_start {
                    Self::build_region_from_slice(markers, smoothed, curr_start, split_idx - 1, out);
                }
                curr_start = split_idx;
            }
            if curr_start <= end_idx {
                Self::build_region_from_slice(markers, smoothed, curr_start, end_idx, out);
            }
        }
    }

    /// Exact locked scoring formula:
    /// CombinedHookScore = (AudienceReplayScore * 0.65) + (DHUNMusicalScore * 0.35)
    /// Both scores normalized to 0.0 - 1.0 before calculation.
    ///
    /// Fallback rule:
    /// When audience evidence is unavailable, missing, or blocked:
    /// CombinedHookScore = DHUNMusicalScore * 1.0
    /// (100% own musical analysis, strictly zero penalty or artificial zero score).
    pub fn compute_combined_hook_score(
        audience_score: Option<f64>,
        dhun_musical_score: f64,
    ) -> f64 {
        let musical_norm = dhun_musical_score.clamp(0.0, 1.0);
        match audience_score {
            Some(aud) => {
                let aud_norm = aud.clamp(0.0, 1.0);
                let combined = (aud_norm * 0.65) + (musical_norm * 0.35);
                (combined * 10000.0).round() / 10000.0
            }
            None => {
                // Honest fallback: 100% DHUN musical score
                (musical_norm * 10000.0).round() / 10000.0
            }
        }
    }

    /// Evaluates DHUN's Stage 1 musical analysis for a given phrase.
    /// Returns an independent, unpenalized musical score between 0.0 and 1.0.
    pub fn evaluate_dhun_musical_score(
        phrase: &PhraseMarker,
        timing_profile: &TrackTimingProfile,
        duration_ms: u64,
    ) -> f64 {
        let base_score = match phrase.section_type.as_str() {
            "chorus" => 0.88,
            "peak" => 0.85,
            "drop" => 0.82,
            "build" => 0.65,
            "groove" => 0.60,
            "verse" => 0.50,
            "bridge" => 0.55,
            "pre_chorus" => 0.62,
            "breakdown" => 0.45,
            "intro" => 0.30,
            "outro" => 0.25,
            _ => 0.40,
        };

        let energy_boost = (phrase.energy_level * 0.15).clamp(0.0, 0.15);
        let vocal_boost = (phrase.vocal_activity * 0.15).clamp(0.0, 0.15);
        let conf_boost = (phrase.confidence * 0.05).clamp(0.0, 0.05);

        // First hook or main drop alignment bonus
        let mut landmark_bonus = 0.0f64;
        if timing_profile.first_hook_ms > 0
            && phrase.start_ms <= timing_profile.first_hook_ms
            && phrase.end_ms >= timing_profile.first_hook_ms
        {
            landmark_bonus += 0.08;
        }
        if timing_profile.main_drop_ms > 0
            && phrase.start_ms <= timing_profile.main_drop_ms
            && phrase.end_ms >= timing_profile.main_drop_ms
        {
            landmark_bonus += 0.06;
        }

        let total = base_score + energy_boost + vocal_boost + conf_boost + landmark_bonus;
        (total.clamp(0.05, 0.99) * 1000.0).round() / 1000.0
    }

    /// Extracts initial Stage 1 musical hook candidates from DHUN's phrase analysis.
    /// Works 100% offline without requiring any YouTube network access or audience data.
    pub fn extract_musical_hook_candidates(
        track_id: &str,
        phrases: &[PhraseMarker],
        timing_profile: &TrackTimingProfile,
        duration_ms: u64,
    ) -> Vec<HookCandidate> {
        let mut candidates = Vec::new();

        for (i, p) in phrases.iter().enumerate() {
            let musical_score = Self::evaluate_dhun_musical_score(p, timing_profile, duration_ms);
            let is_musical_hook = musical_score >= 0.70
                || matches!(p.section_type.as_str(), "chorus" | "peak" | "drop");

            // Candidate start/end snapped to bar/phrase
            let start_ms = p.start_ms;
            let end_ms = p.end_ms;

            let candidate_type = if p.section_type == "chorus" {
                "chorus".to_string()
            } else if p.section_type == "drop" || (start_ms - timing_profile.main_drop_ms).abs() < 1500 {
                "main_drop".to_string()
            } else if p.section_type == "peak" {
                "peak".to_string()
            } else if p.vocal_activity >= 0.50 {
                "vocal_hook".to_string()
            } else {
                p.section_type.clone()
            };

            let combined_score = Self::compute_combined_hook_score(None, musical_score);

            candidates.push(HookCandidate {
                id: format!("{}-hook-{}", track_id, i + 1),
                track_id: track_id.to_string(),
                video_id: None,
                video_start_ms: None,
                video_end_ms: None,
                audio_start_ms: start_ms,
                audio_end_ms: end_ms,
                dhun_musical_score: musical_score,
                audience_replay_score: None,
                combined_hook_score: combined_score,
                alignment_status: TimelineAlignmentStatus::Unavailable,
                alignment_offset_ms: 0,
                dj_usability_score: 0.0,
                is_audience_popular: false,
                is_musical_hook,
                is_aligned_hook: false,
                is_usable_entry: false,
                is_usable_exit: false,
                candidate_type,
                entry_cue_ms: start_ms,
                exit_cue_ms: end_ms,
                usability_reasons: vec!["Stage 1 musical candidate initialized from authorized audio DSP.".to_string()],
                rejection_reasons: vec![],
            });
        }

        candidates.sort_by(|a, b| b.combined_hook_score.partial_cmp(&a.combined_hook_score).unwrap());
        candidates
    }
}
