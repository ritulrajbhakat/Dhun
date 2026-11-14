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
    // Check rate limiter
    if !state.rate_limiter.check_and_record(&payload.guest_id) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Request limit reached. Please wait a few minutes before submitting another song.".to_string(),
        ));
    }

    let req = GuestRequest {
        id: format!("req_{}", Uuid::new_v4().to_string().replace('-', "")[..12].to_string()),
        session_id: state.session_id.clone(),
        guest_id: payload.guest_id,
        guest_name: Some(payload.display_name),
        track_id: payload.track_id,
        track_title: None,
        track_artist: None,
        submitted_at: chrono::Utc::now().timestamp_millis(),
        status: "pending".to_string(),
        priority_order: 100, // standard guest priority
        host_note: payload.host_note,
        reviewed_at: None,
        scheduled_slot: None,
    };

    state
        .db
        .insert_guest_request(&req)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(req))
}

pub async fn get_session_requests(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<GuestRequest>>, (StatusCode, String)> {
    let requests = state
        .db
        .get_session_requests(&state.session_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(requests))
}

pub async fn serve_guest_mobile_html(State(state): State<Arc<AppState>>) -> Html<String> {
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>DHUN Guest Request — {party_name}</title>
  <script src="https://cdn.tailwindcss.com"></script>
  <style>
    body {{ background: #0a0b0e; color: #f1f5f9; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }}
    .dhun-gradient {{ background: linear-gradient(135deg, #e11d48, #f43f5e); }}
  </style>
</head>
<body class="min-h-screen flex flex-col p-4 max-w-md mx-auto">
  <header class="py-4 text-center border-b border-gray-800">
    <div class="inline-flex items-center space-x-2">
      <span class="w-3 h-3 rounded-full bg-rose-500 animate-pulse"></span>
      <h1 class="text-2xl font-black tracking-wider text-rose-500">DHUN</h1>
    </div>
    <p class="text-sm text-gray-400 mt-1">Your Party. Your DJ.</p>
    <div class="mt-2 text-xs uppercase tracking-widest text-amber-400 bg-amber-950/40 border border-amber-900/60 rounded-full px-3 py-1 inline-block">
      {party_name} &bull; {primary_mood}
    </div>
  </header>

  <main class="flex-1 mt-6 flex flex-col space-y-4">
    <div>
      <input id="guestName" type="text" placeholder="Your name (e.g. Aman)" class="w-full bg-gray-900 border border-gray-700 rounded-xl px-4 py-3 text-sm focus:outline-none focus:border-rose-500 placeholder-gray-500" />
    </div>

    <div class="relative">
      <input id="searchInput" type="text" placeholder="Search track, artist, genre..." class="w-full bg-gray-900 border border-gray-700 rounded-xl px-4 py-3 text-sm pl-10 focus:outline-none focus:border-rose-500 placeholder-gray-500" />
      <span class="absolute left-3 top-3 text-gray-500">🔍</span>
    </div>

    <div id="resultsList" class="flex-1 overflow-y-auto space-y-2 max-h-96">
      <div class="text-center text-gray-500 text-sm py-8">Loading available music...</div>
    </div>

    <div id="myRequestsSection" class="border-t border-gray-800 pt-4 mt-6">
      <h2 class="text-xs uppercase tracking-wider text-gray-400 font-semibold mb-2">My Requests</h2>
      <div id="myRequestsList" class="space-y-2 text-xs"></div>
    </div>
  </main>

  <footer class="py-4 text-center text-xs text-gray-600">
    Connected to Local Laptop Host &bull; No App Install Required
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
        if (tracks.length === 0) {{
          container.innerHTML = '<div class="text-center text-gray-500 text-sm py-8">No tracks found</div>';
          return;
        }}
        container.innerHTML = tracks.map(t => `
          <div class="bg-gray-900/80 border border-gray-800 rounded-xl p-3 flex items-center justify-between">
            <div class="flex-1 mr-3 min-w-0">
              <div class="font-semibold text-sm truncate text-white">${{t.title}}</div>
              <div class="text-xs text-gray-400 truncate">${{t.artist}} &bull; ${{t.genre}}</div>
            </div>
            <button onclick="requestSong('${{t.id}}', '${{t.title.replace(/'/g, "\\'")}}')" class="px-3 py-1.5 bg-rose-600 hover:bg-rose-500 active:scale-95 text-white font-medium text-xs rounded-lg transition-all shadow-sm">Request</button>
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
          alert('Request submitted for "' + title + '"! Waiting for host review.');
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
        if (my.length === 0) {{
          container.innerHTML = '<div class="text-gray-600 italic">No requests submitted yet.</div>';
          return;
        }}
        container.innerHTML = my.map(r => `
          <div class="bg-gray-900 border border-gray-800/80 rounded-lg p-2.5 flex items-center justify-between">
            <span class="truncate mr-2 font-medium">${{r.track_title || 'Track'}}</span>
            <span class="px-2 py-0.5 rounded text-[10px] font-semibold uppercase ${{r.status === 'playing' ? 'bg-emerald-950 text-emerald-400 border border-emerald-800' : r.status === 'approved' ? 'bg-cyan-950 text-cyan-400 border border-cyan-800' : 'bg-gray-800 text-gray-400'}}">${{r.status}}</span>
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

