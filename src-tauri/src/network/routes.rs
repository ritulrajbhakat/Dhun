use axum::extract::{ConnectInfo, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, Json};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::Database;
use crate::db::models::GuestRequest;
use crate::network::rate_limit::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub session_id: String,
    pub party_name: String,
    pub primary_mood: String,
    pub rate_limiter: RateLimiter,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AuthPayload {
    pub display_name: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub guest_id: String,
    pub token: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRequestPayload {
    pub token: Option<String>,
    pub guest_id: Option<String>,
    pub display_name: Option<String>,
    pub track_id: String,
    pub host_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetRequestsQuery {
    pub token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PartyInfoResponse {
    pub session_id: String,
    pub party_name: String,
    pub primary_mood: String,
    pub allow_requests: bool,
}

pub async fn get_party_info(State(state): State<Arc<AppState>>) -> Json<PartyInfoResponse> {
    Json(PartyInfoResponse {
        session_id: state.session_id.clone(),
        party_name: state.party_name.clone(),
        primary_mood: state.primary_mood.clone(),
        allow_requests: true,
    })
}

pub async fn guest_auth(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let clean_name = payload.display_name.trim();
    if clean_name.is_empty() || clean_name.len() > 50 {
        return Err((StatusCode::BAD_REQUEST, "Display name must be between 1 and 50 characters.".to_string()));
    }

    let client_ip = addr.ip().to_string();
    let auth_rate_key = format!("auth:{}", client_ip);
    if !state.rate_limiter.check_and_record(&auth_rate_key) {
        return Err((StatusCode::TOO_MANY_REQUESTS, "Authentication rate limit reached. Please wait before creating a new session.".to_string()));
    }

    let sanitized_name: String = clean_name.chars().filter(|c| !c.is_control()).take(50).collect();
    let token = format!("tok_{}", Uuid::new_v4().to_string().replace('-', ""));
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());
    let guest_id = format!("gst_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);

    state.db.register_guest(&guest_id, &state.session_id, &client_ip, &sanitized_name, &token_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to register guest session: {}", e)))?;

    Ok(Json(AuthResponse {
        guest_id,
        token,
        display_name: sanitized_name,
    }))
}

pub async fn search_catalog(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<crate::db::models::Track>>, (StatusCode, String)> {
    if let Some(ref q) = query.q {
        if q.len() > 100 {
            return Err((StatusCode::BAD_REQUEST, "Search query exceeds 100 character maximum limit.".to_string()));
        }
    }

    let all_tracks = state.db.get_all_tracks().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let search_str = query.q.unwrap_or_default().to_lowercase();
    let limit = query.limit.unwrap_or(30).min(100);

    let filtered: Vec<_> = all_tracks
        .into_iter()
        .filter(|t| {
            if search_str.is_empty() {
                true
            } else {
                t.title.to_lowercase().contains(&search_str)
                    || t.artist.to_lowercase().contains(&search_str)
                    || t.genre.to_lowercase().contains(&search_str)
            }
        })
        .take(limit)
        .collect();

    Ok(Json(filtered))
}

pub async fn submit_request(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<CreateRequestPayload>,
) -> Result<Json<GuestRequest>, (StatusCode, String)> {
    // 1. Mandatory guest authentication via cryptographic token (unauthenticated bypasses removed)
    let raw_token = payload.token.as_deref().ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "Authentication token is required to submit requests. Please authenticate via /api/auth first.".to_string())
    })?;
    let clean_token = raw_token.trim();
    if clean_token.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "Authentication token cannot be empty.".to_string()));
    }

    let mut hasher = Sha256::new();
    hasher.update(clean_token.as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());

    let guest = state.db.lookup_guest_by_token_hash(&state.session_id, &token_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid or expired guest session token. Please re-authenticate.".to_string()))?;

    if guest.is_blocked {
        return Err((StatusCode::FORBIDDEN, "This device has been blocked from submitting requests.".to_string()));
    }
    if guest.request_count >= 10 {
        return Err((StatusCode::TOO_MANY_REQUESTS, "Maximum song request limit reached (10 songs per guest).".to_string()));
    }

    let clean_guest_id = guest.id.clone();
    let sanitized_display_name = guest.display_name.clone();

    let clean_track_id = payload.track_id.trim();
    if clean_track_id.is_empty() || clean_track_id.len() > 100 {
        return Err((StatusCode::BAD_REQUEST, "Invalid track identifier length.".to_string()));
    }

    let sanitized_host_note = payload.host_note.map(|n| {
        n.trim().chars().filter(|c| !c.is_control()).take(200).collect::<String>()
    });

    // 2. Check rate limiter using composite key (IP + guest ID) to prevent local storage reset evasion
    let client_ip = addr.ip().to_string();
    let rate_key = format!("{}:{}", client_ip, clean_guest_id);
    if !state.rate_limiter.check_and_record(&rate_key) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Request limit reached. Please wait a few minutes before submitting another song.".to_string(),
        ));
    }

    // 3. Validate that track actually exists and is playable in the catalogue
    let candidates = state.db.get_playable_candidates()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let target = candidates
        .into_iter()
        .find(|c| c.track.id == clean_track_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "Requested track is not found or not available in the active catalogue.".to_string(),
            )
        })?;

    // 4. Prevent duplicate requests in active queue (pending, approved, or scheduled)
    let existing_requests = state.db.get_session_requests(&state.session_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let is_already_queued = existing_requests.iter().any(|r| {
        r.track_id == clean_track_id && (r.status == "pending" || r.status == "approved" || r.status == "scheduled")
    });

    if is_already_queued {
        return Err((
            StatusCode::CONFLICT,
            "This track has already been requested and is currently in the party queue!".to_string(),
        ));
    }

    let req = GuestRequest {
        id: format!("req_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string()),
        session_id: state.session_id.clone(),
        guest_id: clean_guest_id.clone(),
        guest_name: Some(sanitized_display_name),
        track_id: target.track.id.clone(),
        track_title: Some(target.track.title),
        track_artist: Some(target.track.artist),
        submitted_at: chrono::Utc::now().timestamp_millis(),
        status: "pending".to_string(),
        priority_order: 100, // standard guest priority
        host_note: sanitized_host_note,
        reviewed_at: None,
        scheduled_slot: None,
    };

    state.db.insert_guest_request(&req).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 5. Only increment guest request count AFTER validation and successful database insertion
    let _ = state.db.increment_guest_request_count(&clean_guest_id);

    Ok(Json(req))
}

pub async fn get_session_requests(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetRequestsQuery>,
) -> Result<Json<Vec<GuestRequest>>, (StatusCode, String)> {
    let raw_token = query.token.as_deref().ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "Guest session token required to view party request queue.".to_string())
    })?;
    let clean_token = raw_token.trim();
    if clean_token.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "Session token cannot be empty.".to_string()));
    }
    let mut hasher = Sha256::new();
    hasher.update(clean_token.as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());

    let caller_guest = state.db.lookup_guest_by_token_hash(&state.session_id, &token_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid or expired guest session token.".to_string()))?;

    let mut requests = state.db.get_session_requests(&state.session_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Strict privacy filtering: mask other guests' identities and host notes
    for r in requests.iter_mut() {
        let is_own = caller_guest.id == r.guest_id;
        if !is_own {
            r.guest_id = "anonymized".to_string();
            r.guest_name = Some("Party Guest".to_string());
            r.host_note = None;
        }
    }
    Ok(Json(requests))
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub async fn serve_guest_mobile_html(State(state): State<Arc<AppState>>) -> Html<String> {
    let clean_party = escape_html(&state.party_name);
    let clean_mood = escape_html(&state.primary_mood);

    // 100% self-contained, offline-first mobile web application with strict DOM XSS protection
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>DHUN — {clean_party}</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      background: #090a0f;
      color: #f1f5f9;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      padding: 16px;
      max-width: 480px;
      margin: 0 auto;
    }}
    header {{ text-align: center; padding: 16px 0; border-bottom: 1px solid #1e293b; }}
    .logo {{ font-size: 26px; font-weight: 900; color: #f43f5e; letter-spacing: 2px; }}
    .subtitle {{ font-size: 13px; color: #94a3b8; margin-top: 2px; }}
    .badge {{
      display: inline-block;
      margin-top: 8px;
      font-size: 11px;
      font-weight: 700;
      color: #fbbf24;
      background: rgba(245, 158, 11, 0.15);
      border: 1px solid rgba(245, 158, 11, 0.4);
      border-radius: 9999px;
      padding: 4px 12px;
      text-transform: uppercase;
      letter-spacing: 1px;
    }}
    .input-group {{ margin-top: 14px; position: relative; }}
    input[type="text"] {{
      width: 100%;
      background: #111827;
      border: 1px solid #374151;
      border-radius: 12px;
      padding: 12px 16px;
      color: #fff;
      font-size: 14px;
      outline: none;
      transition: border-color 0.2s;
    }}
    input[type="text"]:focus {{ border-color: #f43f5e; }}
    #resultsList {{
      margin-top: 14px;
      flex: 1;
      overflow-y: auto;
      max-height: 52vh;
      display: flex;
      flex-direction: column;
      gap: 8px;
    }}
    .track-card {{
      background: #111827;
      border: 1px solid #1f2937;
      border-radius: 12px;
      padding: 12px;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }}
    .track-title {{ font-size: 14px; font-weight: 700; color: #fff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
    .track-artist {{ font-size: 12px; color: #9ca3af; margin-top: 2px; }}
    .btn-req {{
      background: #e11d48;
      color: #fff;
      font-size: 12px;
      font-weight: 700;
      border: none;
      border-radius: 8px;
      padding: 8px 14px;
      cursor: pointer;
      flex-shrink: 0;
      margin-left: 10px;
    }}
    .btn-req:active {{ transform: scale(0.96); }}
    .section-title {{ font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 1px; color: #94a3b8; margin: 16px 0 8px; }}
    .req-item {{
      background: #0f172a;
      border: 1px solid #1e293b;
      border-radius: 8px;
      padding: 8px 12px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      font-size: 12px;
    }}
    .status-badge {{
      font-size: 10px;
      font-weight: 700;
      padding: 2px 8px;
      border-radius: 6px;
      text-transform: uppercase;
    }}
    .status-pending {{ background: rgba(148, 163, 184, 0.2); color: #cbd5e1; }}
    .status-approved {{ background: rgba(14, 165, 233, 0.2); color: #38bdf8; border: 1px solid rgba(14, 165, 233, 0.4); }}
    .status-playing {{ background: rgba(16, 185, 129, 0.2); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.4); }}
    footer {{ text-align: center; font-size: 11px; color: #475569; margin-top: auto; padding: 14px 0 4px; }}
  </style>
</head>
<body>
  <header>
    <div class="logo">DHUN</div>
    <div class="subtitle">Your Party. Your DJ.</div>
    <div class="badge">{clean_party} &bull; {clean_mood}</div>
  </header>

  <div class="input-group">
    <input id="guestName" type="text" placeholder="Your name (e.g. Aman)" />
  </div>

  <div class="input-group">
    <input id="searchInput" type="text" placeholder="Search track, artist, genre..." />
  </div>

  <div id="resultsList">
    <div style="text-align:center; padding: 30px; color:#64748b;">Loading available music...</div>
  </div>

  <div>
    <div class="section-title">My Song Requests</div>
    <div id="myRequestsList" style="display:flex; flex-direction:column; gap:6px;">
      <div style="color:#64748b; font-style:italic; font-size:12px;">No requests submitted yet.</div>
    </div>
  </div>

  <footer>
    Connected to Local Laptop Host &bull; Offline Wi-Fi Ready
  </footer>

  <script>
    let guestToken = localStorage.getItem('dhun_guest_token');
    let guestId = localStorage.getItem('dhun_guest_id') || ('gst_' + Math.random().toString(36).substring(2, 10));
    localStorage.setItem('dhun_guest_id', guestId);
    let nameInput = document.getElementById('guestName');
    nameInput.value = localStorage.getItem('dhun_guest_name') || '';
    nameInput.addEventListener('input', (e) => localStorage.setItem('dhun_guest_name', e.target.value));

    async function ensureAuth() {{
      if (guestToken) return guestToken;
      let name = nameInput.value.trim() || 'Party Guest';
      try {{
        let res = await fetch('/api/guest/auth', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{ display_name: name }})
        }});
        if (res.ok) {{
          let data = await res.json();
          guestToken = data.token;
          guestId = data.guest_id;
          localStorage.setItem('dhun_guest_token', guestToken);
          localStorage.setItem('dhun_guest_id', guestId);
          return guestToken;
        }}
      }} catch (err) {{
        console.error('Handshake error:', err);
      }}
      return null;
    }}

    async function loadTracks(query = '') {{
      try {{
        let res = await fetch('/api/guest/search?q=' + encodeURIComponent(query));
        let tracks = await res.json();
        let container = document.getElementById('resultsList');
        container.innerHTML = '';
        if (!tracks || tracks.length === 0) {{
          let msg = document.createElement('div');
          msg.style.cssText = 'text-align:center; padding:30px; color:#64748b;';
          msg.textContent = 'No tracks found in catalogue';
          container.appendChild(msg);
          return;
        }}
        tracks.forEach(t => {{
          let card = document.createElement('div');
          card.className = 'track-card';

          let info = document.createElement('div');
          info.style.cssText = 'min-width:0; flex:1;';

          let title = document.createElement('div');
          title.className = 'track-title';
          title.textContent = t.title;

          let artist = document.createElement('div');
          artist.className = 'track-artist';
          artist.textContent = `${{t.artist}} • ${{t.genre}}`;

          info.appendChild(title);
          info.appendChild(artist);

          let btn = document.createElement('button');
          btn.className = 'btn-req';
          btn.textContent = 'Request';
          btn.addEventListener('click', () => requestSong(t.id, t.title));

          card.appendChild(info);
          card.appendChild(btn);
          container.appendChild(card);
        }});
      }} catch (err) {{
        console.error(err);
      }}
    }}

    async function requestSong(trackId, title) {{
      let token = await ensureAuth();
      let name = nameInput.value.trim() || 'Party Guest';
      try {{
        let payload = {{
          token: token,
          guest_id: guestId,
          display_name: name,
          track_id: trackId
        }};
        let res = await fetch('/api/guest/request', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify(payload)
        }});
        if (res.ok) {{
          alert('Request submitted for "' + title + '"! Host notified.');
          loadMyRequests();
        }} else {{
          let err = await res.text();
          alert('Could not submit request: ' + err);
        }}
      }} catch (err) {{
        alert('Error communicating with host server');
      }}
    }}

    async function loadMyRequests() {{
      try {{
        let url = '/api/guest/requests';
        if (guestToken) {{
          url += '?token=' + encodeURIComponent(guestToken);
        }}
        let res = await fetch(url);
        let requests = await res.json();
        let my = requests.filter(r => r.guest_id === guestId || r.guest_id !== 'anonymized');
        let container = document.getElementById('myRequestsList');
        container.innerHTML = '';
        if (!my || my.length === 0) {{
          let msg = document.createElement('div');
          msg.style.cssText = 'color:#64748b; font-style:italic; font-size:12px;';
          msg.textContent = 'No requests submitted yet.';
          container.appendChild(msg);
          return;
        }}
        my.forEach(r => {{
          let item = document.createElement('div');
          item.className = 'req-item';

          let title = document.createElement('span');
          title.style.cssText = 'font-weight:600; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; margin-right:8px;';
          title.textContent = r.track_title || 'Track';

          let badge = document.createElement('span');
          badge.className = 'status-badge status-' + r.status;
          badge.textContent = r.status;

          item.appendChild(title);
          item.appendChild(badge);
          container.appendChild(item);
        }});
      }} catch (err) {{
        console.error(err);
      }}
    }}

    document.getElementById('searchInput').addEventListener('input', (e) => loadTracks(e.target.value));
    loadTracks();
    loadMyRequests();
    setInterval(loadMyRequests, 5000);
  </script>
</body>
</html>"#,
        clean_party = clean_party,
        clean_mood = clean_mood
    );

    Html(html)
}
