# DHUN Local Connection, Browser Security, and Source Acquisition Architecture

## 1. System Topology (Strict Separation of Concerns)

DHUN consists of three strictly separated components:

1. **Public Web Frontend (Vercel — HTTPS)**
   - Renders the Indian party-stage UI, animated DJ character, 15 Canvas visual FX, Devanagari `धुन डीजे` branding, 4-deck Web Audio mixer controls, and Host Console (`M`).
   - Communicates **only** with the host laptop's **Local DHUN Helper Service** over loopback (`http://127.0.0.1:47821`).
   - Holds **no** Global Metadata Catalogue credentials and exposes **no** global catalogue browsing, search, or editing UI.

2. **Local DHUN Helper Service (`dhun-local-helper` — Host Laptop)**
   - Runs locally on the host's computer while the party is active.
   - Manages the private party room (`local_party_rooms`), LAN guest admission (`local_room_guests`), unified host & guest song submissions (`local_room_submissions`, zero approval queue), 10–15 track temporary local audio cache (`local_temp_cache`), local Symphonia audio decoding & DSP analysis, and the non-interrupting automated DJ candidate scorer.
   - Rejects any non-LAN external IP (`403 Forbidden`) and rejects DNS-rebinding `Host` headers (`403 Forbidden`).
   - Closes the room and purges temporary cached audio **only** when the host explicitly closes the party (with launch-time expiry cleanup as a crash fallback).

3. **Global Shared Song Metadata Catalogue (`dhun-server` — Cloud Service)**
   - Completely isolated database (`global_song_catalogue`, `global_song_source_urls`) with **zero** party rooms, playlists, guest requests, or audio files.
   - Accessible **only** by the automated DJ engine inside `dhun-local-helper` using NIST-standard HMAC-SHA256 request signing (`X-Dhun-Signature`, `X-Dhun-Timestamp`, ±300s replay window).
   - Rejects any request containing raw audio bytes or party/guest fields (`400 Bad Request`).

---

## 2. Browser Security Strategy (HTTPS Public Website ↔ Local Helper & LAN Guests)

Serving the public DHUN website over HTTPS (Vercel) while operating a local host service over loopback/LAN introduces two well-known browser security constraints: **Mixed Content** and **Chrome Private Network Access (PNA)**. DHUN resolves both cleanly without faking connections or routing private parties through the cloud:

### A. Host Laptop (Vercel HTTPS → `http://127.0.0.1:47821`)
- **Potentially Trustworthy Origin**: Modern Chromium (Chrome, Edge, Brave), Firefox, and Safari exempt `http://127.0.0.1` and `http://localhost` from HTTPS mixed-content blocking because loopback traffic never leaves the host machine.
- **Private Network Access (PNA)**: Chromium sends a preflight `OPTIONS` request containing `Access-Control-Request-Private-Network: true`. `dhun-local-helper` explicitly validates the `Origin` header (allowing configured Vercel origins and localhost) and responds with `Access-Control-Allow-Private-Network: true`.
- **Known Browser Caveat**: Safari on macOS/iOS applies stricter mixed-content rules when an HTTPS page opens an insecure `ws://127.0.0.1` WebSocket in certain versions, and strict Brave Shields may require allowing local network access for the DHUN site. If WebSocket upgrade is blocked by a host browser policy, `dhun-local-helper` remains reachable over loopback HTTP and falls back cleanly on REST refresh; Prompt 3 will evaluate optional local loopback TLS certificates (`mkcert`/localhost cert helper) or local helper UI launch for zero-friction Safari support.

### B. Guest Smartphones (Direct Same-Wi-Fi LAN Portal)
- **Why guests do NOT hit the Vercel HTTPS site to talk to a private LAN IP**: An HTTPS webpage on a smartphone **cannot** make `fetch()` calls to `http://192.168.x.x:47821` — every mobile browser (iOS Safari and Android Chrome) strictly blocks HTTPS-to-private-IP HTTP mixed content!
- **DHUN's Workable Solution**: The host's QR code encodes the **direct same-origin LAN URL** served by `dhun-local-helper` itself:
  ```
  http://<REAL_DETECTED_LAN_IP>:47821/join/<room_id>?access=<room_access_token>
  ```
  - Because the guest smartphone loads the mobile request page directly over HTTP from the host's LAN IP, all guest API calls (`/guests/join`, `/submissions`, `/my-submissions`) are **same-origin HTTP on the local Wi-Fi** — avoiding mixed-content blocks on iOS Safari and Android Chrome while guaranteeing guests outside the host's Wi-Fi/LAN cannot reach the room.
  - If the host machine has no active RFC1918 Wi-Fi/LAN interface, `dhun-local-helper` honestly reports `lan_reachable: false` and refuses to fabricate a placeholder IP or QR code.

---

## 3. Local Acquisition & Platform/Legal Limitations (Honest Disclosure)

DHUN enforces a strict 6-stage local processing pipeline (`src-tauri/src/library/pipeline.rs`):
1. URL validation & source identification (`youtube.com/watch`, `youtu.be`, `music.youtube.com/watch` only).
2. Existing reusable metadata lookup (local cache → authenticated Global Catalogue → official YouTube oEmbed).
3. **Authorised Audio Acquisition (When Permitted)**:
   - Validating a YouTube / YouTube Music URL or fetching official oEmbed metadata does **not** grant permission to download, decrypt, rip, or cache YouTube audio streams.
   - DHUN does **not** embed yt-dlp/stream-rippers, does **not** circumvent DRM/platform restrictions, and does **not** silently substitute an embedded YouTube iframe player for the local audio engine.
   - Standard YouTube / YouTube Music submissions are honestly recorded with `acquisition_status = "restricted_by_provider"`, while keeping full metadata lookup, DJ candidate pool scoring, and metadata-only catalogue sync functional.
   - Local temporary caching (10–15 tracks) and local Symphonia DSP analysis run when the host supplies an explicitly authorized HTTPS audio stream mirror (`AuthorizedStreamProof`) for content they hold rights/license to process.
4. Temporary local caching (max 15 tracks, priority-locked for playing/upcoming decks, purged on explicit party close + 24h launch-time expiry sweep).
5. Local audio analysis (BPM, half/double-time BPM, Camelot key, beat grid, phrase markers, energy, danceability).
6. Metadata-only sync to the Global Shared Song Metadata Catalogue (queued in SQLite `sync_outbox` when offline and retried automatically when online; zero audio bytes ever uploaded).
