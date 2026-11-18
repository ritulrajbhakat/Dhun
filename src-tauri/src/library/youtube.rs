use anyhow::{anyhow, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Supported online music source providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicSourceProvider {
    YouTube,
    YouTubeMusic,
    CustomAuthorized,
}

impl MusicSourceProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::YouTube => "youtube",
            Self::YouTubeMusic => "youtube_music",
            Self::CustomAuthorized => "custom_authorized",
        }
    }
}

/// Explicit playability status reflecting legal and technical realities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourcePlayabilityStatus {
    Registered,
    MetadataAvailable,
    PlaybackUnverified,
    Playable,
    Unavailable,
    Restricted,
    Failed,
}

impl SourcePlayabilityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::MetadataAvailable => "metadata_available",
            Self::PlaybackUnverified => "playback_unverified",
            Self::Playable => "playable",
            Self::Unavailable => "unavailable",
            Self::Restricted => "restricted",
            Self::Failed => "failed",
        }
    }
}

/// Parsed and validated YouTube/YouTube Music URL components
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedYouTubeUrl {
    pub provider: MusicSourceProvider,
    pub video_id: String,
    pub canonical_url: String,
}

/// Validates that a video ID conforms to YouTube's standard 11-character base64-URL identifier
pub fn is_valid_youtube_video_id(id: &str) -> bool {
    id.len() == 11 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Keyword and word-match pre-validation for YouTube / YouTube Music links.
/// Rejects embedded credentials (`@` host spoofing), control characters, traversal tokens,
/// script/file scheme keywords, and URLs lacking required YouTube host keywords.
pub fn validate_youtube_link_keywords(raw_url: &str) -> Result<()> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() || trimmed.len() > 2048 {
        return Err(anyhow!("YouTube URL is empty or exceeds maximum length"));
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(anyhow!("YouTube URL contains illegal whitespace or control characters"));
    }

    let lower = trimmed.to_lowercase();
    for forbidden_keyword in [
        "javascript:",
        "vbscript:",
        "data:",
        "file://",
        "ftp://",
        "../",
        "..\\",
        "%00",
        "169.254.169.254",
    ] {
        if lower.contains(forbidden_keyword) {
            return Err(anyhow!(
                "Rejected URL containing forbidden keyword '{}'",
                forbidden_keyword
            ));
        }
    }

    if !(lower.contains("youtube.com") || lower.contains("youtu.be")) {
        return Err(anyhow!(
            "URL does not match required YouTube domain keywords ('youtube.com' or 'youtu.be')"
        ));
    }

    Ok(())
}

/// Parses and strictly validates a YouTube or YouTube Music link, extracting provider and video ID.
/// Rejects unauthorized domains, insecure schemes, userinfo spoofing, and malformed identifiers.
pub fn parse_youtube_url(url_str: &str) -> Result<ParsedYouTubeUrl> {
    validate_youtube_link_keywords(url_str)?;

    let parsed = Url::parse(url_str.trim()).map_err(|e| anyhow!("Malformed URL: {}", e))?;

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(anyhow!(
            "Security violation: URLs with embedded credentials/userinfo ('@') are rejected"
        ));
    }

    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err(anyhow!(
            "Invalid protocol '{}': only HTTP/HTTPS URLs are supported",
            parsed.scheme()
        ));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("URL missing host"))?
        .to_lowercase();

    // 1. YouTube Music: music.youtube.com/watch?v=...
    if host == "music.youtube.com" {
        let video_id = parsed
            .query_pairs()
            .find(|(k, _)| k == "v")
            .map(|(_, v)| v.to_string())
            .ok_or_else(|| anyhow!("YouTube Music URL missing '?v=' video ID parameter"))?;

        if !is_valid_youtube_video_id(&video_id) {
            return Err(anyhow!(
                "Invalid YouTube video ID '{}': must be exactly 11 characters (alphanumeric, -, _)",
                video_id
            ));
        }

        return Ok(ParsedYouTubeUrl {
            provider: MusicSourceProvider::YouTubeMusic,
            video_id: video_id.clone(),
            canonical_url: format!("https://music.youtube.com/watch?v={}", video_id),
        });
    }

    // 2. Standard YouTube: youtube.com, www.youtube.com, m.youtube.com
    if host == "youtube.com" || host == "www.youtube.com" || host == "m.youtube.com" {
        let path = parsed.path();
        if path == "/watch" {
            let video_id = parsed
                .query_pairs()
                .find(|(k, _)| k == "v")
                .map(|(_, v)| v.to_string())
                .ok_or_else(|| anyhow!("YouTube URL missing '?v=' video ID parameter"))?;

            if !is_valid_youtube_video_id(&video_id) {
                return Err(anyhow!(
                    "Invalid YouTube video ID '{}': must be exactly 11 characters",
                    video_id
                ));
            }

            return Ok(ParsedYouTubeUrl {
                provider: MusicSourceProvider::YouTube,
                video_id: video_id.clone(),
                canonical_url: format!("https://www.youtube.com/watch?v={}", video_id),
            });
        } else if path.starts_with("/embed/") {
            let video_id = path.trim_start_matches("/embed/");
            let clean_id = video_id.split('?').next().unwrap_or(video_id);
            if !is_valid_youtube_video_id(clean_id) {
                return Err(anyhow!("Invalid YouTube embed video ID '{}'", clean_id));
            }
            return Ok(ParsedYouTubeUrl {
                provider: MusicSourceProvider::YouTube,
                video_id: clean_id.to_string(),
                canonical_url: format!("https://www.youtube.com/watch?v={}", clean_id),
            });
        } else {
            return Err(anyhow!(
                "Unsupported YouTube URL path '{}': only /watch?v=... is supported",
                path
            ));
        }
    }

    // 3. Short links: youtu.be/ID
    if host == "youtu.be" {
        let video_id = parsed.path().trim_start_matches('/');
        let clean_id = video_id.split('?').next().unwrap_or(video_id);
        if !is_valid_youtube_video_id(clean_id) {
            return Err(anyhow!(
                "Invalid youtu.be video ID '{}': must be exactly 11 characters",
                clean_id
            ));
        }

        return Ok(ParsedYouTubeUrl {
            provider: MusicSourceProvider::YouTube,
            video_id: clean_id.to_string(),
            canonical_url: format!("https://www.youtube.com/watch?v={}", clean_id),
        });
    }

    Err(anyhow!(
        "Unauthorized music provider domain '{}': only youtube.com, youtu.be, and music.youtube.com are permitted",
        host
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeOEmbedResponse {
    pub title: Option<String>,
    pub author_name: Option<String>,
    pub thumbnail_url: Option<String>,
    pub html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedSourceMetadata {
    pub title: String,
    pub artist: String,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<u32>,
    pub metadata_status: String,
    pub playability_status: SourcePlayabilityStatus,
    pub playback_method: String,
    pub error_reason: Option<String>,
}

/// Fetches permitted metadata using YouTube's official public oEmbed endpoint.
/// If network is unreachable (e.g. offline sandbox or offline party), provides clean fallback metadata.
pub async fn fetch_youtube_metadata(
    parsed: &ParsedYouTubeUrl,
    client: &reqwest::Client,
) -> FetchedSourceMetadata {
    let query_params: [(&str, &str); 2] = [
        ("url", parsed.canonical_url.as_str()),
        ("format", "json"),
    ];

    match client
        .get("https://www.youtube.com/oembed")
        .query(&query_params)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(oembed) = resp.json::<YouTubeOEmbedResponse>().await {
                let title = oembed
                    .title
                    .unwrap_or_else(|| format!("YouTube Track ({})", parsed.video_id));
                let artist = oembed
                    .author_name
                    .unwrap_or_else(|| "YouTube Creator".to_string());
                let thumbnail = oembed.thumbnail_url.or_else(|| {
                    Some(format!(
                        "https://i.ytimg.com/vi/{}/hqdefault.jpg",
                        parsed.video_id
                    ))
                });

                info!(
                    "Fetched official YouTube metadata for '{}': {} - {}",
                    parsed.video_id, title, artist
                );

                return FetchedSourceMetadata {
                    title,
                    artist,
                    thumbnail_url: thumbnail,
                    duration_seconds: None,
                    metadata_status: "fetched".to_string(),
                    playability_status: SourcePlayabilityStatus::PlaybackUnverified,
                    playback_method: "youtube_iframe_embed".to_string(),
                    error_reason: Some(
                        "Direct audio stream extraction is restricted by provider. Track is queued for official YouTube Player iframe integration in Phase 2."
                            .to_string(),
                    ),
                };
            }
        }
        _ => {}
    }

    // Offline / fallback metadata
    FetchedSourceMetadata {
        title: format!("YouTube Track ({})", parsed.video_id),
        artist: "YouTube Video".to_string(),
        thumbnail_url: Some(format!(
            "https://i.ytimg.com/vi/{}/hqdefault.jpg",
            parsed.video_id
        )),
        duration_seconds: None,
        metadata_status: "partial".to_string(),
        playability_status: SourcePlayabilityStatus::PlaybackUnverified,
        playback_method: "youtube_iframe_embed".to_string(),
        error_reason: Some(
            "Registered via official reference. Direct audio stream extraction is restricted by provider; scheduled for official YouTube embedded player in Phase 2."
                .to_string(),
        ),
    }
}
