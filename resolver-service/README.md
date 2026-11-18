# DHUN Audius Media Resolver Service (FastAPI)

Lightweight Python FastAPI microservice that connects `dhun-local-helper` (`DHUN_AUDIO_RESOLVER_URL`) to the official public **Audius REST API** (`https://api.audius.co/v1`).

## How It Works

```text
DHUN Host / Guest submits song link
        ↓
dhun-local-helper (Stages 1 & 2: validate + metadata lookup)
        ↓  HTTPS POST /resolve { source_url, provider_video_id, title, artist, version_tag }
DHUN Audius Resolver (FastAPI)
        ↓  GET https://api.audius.co/v1/tracks/search
        ↓  Returns JSON { direct_https_audio_url } (or streams audio/mpeg in binary mode)
dhun-local-helper (Stage 3: AudioDownloader HTTPS + anti-SSRF + Symphonia verify)
        ↓
Temporary Local Audio Cache (max 15 tracks, 60-min expiry, purged on party close)
        ↓
Symphonia DSP Analysis (BPM / half-double BPM / Camelot key / beat grid / energy)
        ↓
4-Deck Web Audio Engine → DJ Speakers
```

## Local Setup (Virtual Environment)

```powershell
cd resolver-service
python -m venv .venv
.\.venv\Scripts\pip install -r requirements.txt
```

Run locally for testing:

```powershell
$env:RESOLVER_API_KEY = "choose-a-strong-secret-token"
.\.venv\Scripts\uvicorn app:app --host 0.0.0.0 --port 8080
```

Test with `curl`:

```powershell
curl.exe http://127.0.0.1:8080/health
```

## Deploying Over HTTPS for `dhun-local-helper`

`dhun-local-helper` enforces strict anti-SSRF and HTTPS validation (`DHUN_AUDIO_RESOLVER_URL` must be a public `https://` URL — plain `http://` and `localhost` are rejected by design).

Deploy `resolver-service/` on any HTTPS platform (Render, Railway, Fly.io, Cloud Run, or behind Caddy/Nginx with TLS) with:

- **Start command:** `uvicorn app:app --host 0.0.0.0 --port $PORT`
- **Optional environment variables:**
  - `RESOLVER_API_KEY` — shared secret checked via `Authorization: Bearer <token>`
  - `AUDIUS_API_KEY` — optional Audius developer API key for higher rate limits
  - `AUDIUS_APP_NAME` — defaults to `DHUN`
  - `RESOLVER_RESPONSE_MODE` — `json` (default, returns `direct_https_audio_url`) or `binary` (streams `audio/mpeg` directly)

## Connecting `dhun-local-helper`

Before starting `dhun-local-helper`, set:

```powershell
$env:DHUN_AUDIO_RESOLVER_URL = "https://your-resolver-domain.example.com/resolve"
$env:DHUN_AUDIO_RESOLVER_KEY = "choose-a-strong-secret-token"
powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\DHUN\bin\start-dhun-helper.ps1"
```

Verify `audio_resolver_configured: true` at `http://127.0.0.1:47821/api/local/health`.
