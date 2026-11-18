"""
DHUN Audius Media Resolver Service (FastAPI)

Exposes an HTTP/HTTPS endpoint compatible with `dhun-local-helper`'s
`DHUN_AUDIO_RESOLVER_URL` contract. Searches the official public Audius REST
API (`https://api.audius.co/v1`) for a streamable track matching the submitted
song title/artist and returns either:
  - JSON (`direct_https_audio_url`) for `dhun-local-helper`'s hardened AudioDownloader, or
  - Direct audio bytes (`audio/mpeg`) when `RESOLVER_RESPONSE_MODE=binary`.
"""

from __future__ import annotations

import hmac
import os
import re
from typing import Any, Optional

import httpx
from fastapi import FastAPI, Header, HTTPException, Query
from fastapi.responses import JSONResponse, StreamingResponse
from pydantic import BaseModel

AUDIUS_API_BASE = os.getenv("AUDIUS_API_BASE", "https://api.audius.co/v1").rstrip("/")
AUDIUS_APP_NAME = os.getenv("AUDIUS_APP_NAME", "DHUN")
AUDIUS_API_KEY = os.getenv("AUDIUS_API_KEY", "").strip()
RESOLVER_API_KEY = os.getenv("RESOLVER_API_KEY", "").strip()
DEFAULT_RESPONSE_MODE = os.getenv("RESOLVER_RESPONSE_MODE", "json").strip().lower()

NOISE_PATTERN = re.compile(
    r"(\(|\[)\s*(official\s*(music\s*)?video|official\s*audio|lyric(s|al)?(\s*video)?|"
    r"full\s*song|4k|hd|audio|video\s*song|feat\.?[^)\]]*)\s*(\)|\])",
    re.IGNORECASE,
)

app = FastAPI(
    title="DHUN Audius Resolver Service",
    version="1.0.0",
    description="Resolves song metadata to streamable tracks on the official Audius catalog.",
)


class ResolverLookupPayload(BaseModel):
    source_url: str = ""
    provider_video_id: str = ""
    title: str
    artist: str = ""
    version_tag: str = "original"


class ResolverJsonResponse(BaseModel):
    direct_https_audio_url: str
    license_or_authorization_note: str
    expected_sha256: Optional[str] = None


def verify_bearer_token(authorization: Optional[str]) -> None:
    """Enforces optional Bearer authentication when RESOLVER_API_KEY is configured."""
    if not RESOLVER_API_KEY:
        return
    if not authorization or not authorization.lower().startswith("bearer "):
        raise HTTPException(status_code=401, detail="Missing Bearer token")
    supplied = authorization[7:].strip()
    if not hmac.compare_digest(supplied, RESOLVER_API_KEY):
        raise HTTPException(status_code=403, detail="Invalid Bearer token")


def clean_search_query(title: str, artist: str) -> str:
    """Strips noisy video suffixes so Audius catalog search matches music tracks accurately."""
    cleaned_title = NOISE_PATTERN.sub("", title or "").strip(" -_|")
    cleaned_artist = (artist or "").strip()
    if cleaned_artist.lower() in {"", "unknown artist", "youtube creator", "various artists"}:
        cleaned_artist = ""
    if cleaned_artist and cleaned_artist.lower() not in cleaned_title.lower():
        return f"{cleaned_title} {cleaned_artist}".strip()
    return cleaned_title or title.strip()


def pick_best_audius_track(
    tracks: list[dict[str, Any]], title: str, artist: str
) -> Optional[dict[str, Any]]:
    """Selects the highest-relevance publicly streamable, non-gated track from Audius search."""
    title_tokens = {t for t in re.findall(r"[a-z0-9]+", title.lower()) if len(t) > 1}
    artist_tokens = {t for t in re.findall(r"[a-z0-9]+", artist.lower()) if len(t) > 1}

    best_track: Optional[dict[str, Any]] = None
    best_score = -1.0

    for track in tracks:
        if not track.get("id"):
            continue
        if track.get("is_streamable") is False:
            continue
        if track.get("is_stream_gated") is True:
            continue

        t_title = str(track.get("title") or "").lower()
        t_user = track.get("user") or {}
        t_artist = f"{t_user.get('name', '')} {t_user.get('handle', '')}".lower()

        cand_title_tokens = set(re.findall(r"[a-z0-9]+", t_title))
        cand_artist_tokens = set(re.findall(r"[a-z0-9]+", t_artist))

        title_overlap = (
            len(title_tokens & cand_title_tokens) / max(len(title_tokens), 1)
            if title_tokens
            else 0.0
        )
        artist_overlap = (
            len(artist_tokens & (cand_artist_tokens | cand_title_tokens))
            / max(len(artist_tokens), 1)
            if artist_tokens
            else 0.0
        )
        play_bonus = min(float(track.get("play_count") or 0) / 500_000.0, 0.25)
        score = (title_overlap * 0.65) + (artist_overlap * 0.25) + play_bonus

        if score > best_score:
            best_score = score
            best_track = track

    return best_track


async def search_audius_track(query: str, title: str, artist: str) -> Optional[dict[str, Any]]:
    headers = {"Accept": "application/json"}
    if AUDIUS_API_KEY:
        headers["x-api-key"] = AUDIUS_API_KEY

    params = {"query": query, "app_name": AUDIUS_APP_NAME}
    async with httpx.AsyncClient(timeout=15.0, follow_redirects=True) as client:
        resp = await client.get(
            f"{AUDIUS_API_BASE}/tracks/search",
            params=params,
            headers=headers,
        )
        if resp.status_code != 200:
            raise HTTPException(
                status_code=502,
                detail=f"Audius search API returned HTTP {resp.status_code}",
            )
        body = resp.json()
        tracks = body.get("data") or []
        return pick_best_audius_track(tracks, title, artist)


@app.get("/health")
async def health() -> dict[str, Any]:
    return {
        "status": "ok",
        "service": "dhun-audius-resolver",
        "provider": "audius",
        "audius_api_base": AUDIUS_API_BASE,
        "auth_required": bool(RESOLVER_API_KEY),
        "default_response_mode": DEFAULT_RESPONSE_MODE,
    }


@app.post("/resolve")
async def resolve_audio(
    payload: ResolverLookupPayload,
    mode: Optional[str] = Query(default=None, pattern="^(json|binary)$"),
    authorization: Optional[str] = Header(default=None),
):
    verify_bearer_token(authorization)

    query = clean_search_query(payload.title, payload.artist)
    if not query:
        raise HTTPException(status_code=400, detail="Song title is required to search Audius")

    track = await search_audius_track(query, payload.title, payload.artist)
    if not track:
        raise HTTPException(
            status_code=404,
            detail=f"No streamable Audius track matched '{query}'",
        )

    track_id = str(track["id"])
    artist_handle = str((track.get("user") or {}).get("handle") or "unknown")
    track_title = str(track.get("title") or payload.title)
    stream_url = f"{AUDIUS_API_BASE}/tracks/{track_id}/stream?app_name={AUDIUS_APP_NAME}"
    license_note = (
        f"Audius Open Catalog Stream (track_id={track_id}, title='{track_title}', artist=@{artist_handle})"
    )

    response_mode = (mode or DEFAULT_RESPONSE_MODE).lower()
    if response_mode == "binary":
        headers = {}
        if AUDIUS_API_KEY:
            headers["x-api-key"] = AUDIUS_API_KEY

        client = httpx.AsyncClient(timeout=60.0, follow_redirects=True)
        req = client.build_request("GET", stream_url, headers=headers)
        upstream = await client.send(req, stream=True)
        if upstream.status_code != 200:
            await upstream.aclose()
            await client.aclose()
            raise HTTPException(
                status_code=502,
                detail=f"Audius stream endpoint returned HTTP {upstream.status_code}",
            )

        content_type = upstream.headers.get("content-type", "audio/mpeg")

        async def audio_iter():
            try:
                async for chunk in upstream.aiter_bytes(chunk_size=65536):
                    yield chunk
            finally:
                await upstream.aclose()
                await client.aclose()

        return StreamingResponse(
            audio_iter(),
            media_type=content_type,
            headers={"X-DHUN-License-Note": license_note},
        )

    return JSONResponse(
        content=ResolverJsonResponse(
            direct_https_audio_url=stream_url,
            license_or_authorization_note=license_note,
            expected_sha256=None,
        ).model_dump()
    )
