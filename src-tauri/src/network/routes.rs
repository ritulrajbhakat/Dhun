use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, Json};
use serde::{Deserialize, Serialize};
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
pub struct CreateRequestPayload {
    pub guest_id: String,
    pub display_name: String,
    pub track_id: String,
    pub host_note: Option<String>,
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

pub async fn search_catalog(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<crate::db::models::Track>>, (StatusCode, String)> {
    let all_tracks = state.db.get_all_tracks().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let search_str = query.q.unwrap_or_default().to_lowercase();
    let limit = query.limit.unwrap_or(30);

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
    Json(payload): Json<CreateRequestPayload>,
) -> Result<Json<GuestRequest>, (StatusCode, String)> {
    // 1. Check rate limiter
    if !state.rate_limiter.check_and_record(&payload.guest_id) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Request limit reached. Please wait a few minutes before submitting another song.".to_string(),
        ));
    }

    // 2. Validate that track actually exists and is playable in the catalogue
    let candidates = state.db.get_playable_candidates()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let target = candidates
        .into_iter()
        .find(|c| c.track.id == payload.track_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "Requested track is not found or not available in the active catalogue.".to_string(),
            )
        })?;

    let req = GuestRequest {
        id: format!("req_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string()),
        session_id: state.session_id.clone(),
        guest_id: payload.guest_id,
        guest_name: Some(payload.display_name),
        track_id: target.track.id.clone(),
        track_title: Some(target.track.title),
        track_artist: Some(target.track.artist),
        submitted_at: chrono::Utc::now().timestamp_millis(),
        status: "pending".to_string(),
        priority_order: 100, // standard guest priority
        host_note: payload.host_note,
        reviewed_at: None,
        scheduled_slot: None,
    };

    state.db.insert_guest_request(&req).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(req))
}

pub async fn get_session_requests(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<GuestRequest>>, (StatusCode, String)> {
    let requests = state.db.get_session_requests(&state.session_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(requests))
}

pub async fn serve_guest_mobile_html(State(state): State<Arc<AppState>>) -> Html<String> {
    // 100% self-contained, offline-first mobile web application (no external CDN dependencies)
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>DHUN — {party_name}</title>
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
    <div class="badge">{party_name} &bull; {primary_mood}</div>
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
    let guestId = localStorage.getItem('dhun_guest_id') || 'gst_' + Math.random().toString(36).substring(2, 10);
    localStorage.setItem('dhun_guest_id', guestId);
    let nameInput = document.getElementById('guestName');
    nameInput.value = localStorage.getItem('dhun_guest_name') || '';
    nameInput.addEventListener('input', (e) => localStorage.setItem('dhun_guest_name', e.target.value));

    async function loadTracks(query = '') {{
      try {{
        let res = await fetch('/api/guest/search?q=' + encodeURIComponent(query));
        let tracks = await res.json();
        let container = document.getElementById('resultsList');
        if (!tracks || tracks.length === 0) {{
          container.innerHTML = '<div style="text-align:center; padding:30px; color:#64748b;">No tracks found</div>';
          return;
        }}
        container.innerHTML = tracks.map(t => `
          <div class="track-card">
            <div style="min-width:0; flex:1;">
              <div class="track-title">${{t.title}}</div>
              <div class="track-artist">${{t.artist}} &bull; ${{t.genre}}</div>
            </div>
            <button onclick="requestSong('${{t.id}}', '${{t.title.replace(/'/g, "\\'")}}')" class="btn-req">Request</button>
          </div>
        `).join('');
      }} catch (err) {{
        console.error(err);
      }}
    }}

    async function requestSong(trackId, title) {{
      let name = nameInput.value.trim() || 'Guest';
      try {{
        let res = await fetch('/api/guest/request', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{ guest_id: guestId, display_name: name, track_id: trackId }})
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
        let res = await fetch('/api/guest/requests');
        let requests = await res.json();
        let my = requests.filter(r => r.guest_id === guestId);
        let container = document.getElementById('myRequestsList');
        if (!my || my.length === 0) {{
          container.innerHTML = '<div style="color:#64748b; font-style:italic; font-size:12px;">No requests submitted yet.</div>';
          return;
        }}
        container.innerHTML = my.map(r => `
          <div class="req-item">
            <span style="font-weight:600; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; margin-right:8px;">${{r.track_title || 'Track'}}</span>
            <span class="status-badge status-${{r.status}}">${{r.status}}</span>
          </div>
        `).join('');
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
        party_name = state.party_name,
        primary_mood = state.primary_mood
    );

    Html(html)
}
