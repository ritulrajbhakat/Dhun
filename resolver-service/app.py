"""
DHUN yt-dlp Media Resolver Service (FastAPI)

Exposes an HTTP/HTTPS endpoint compatible with `dhun-local-helper`'s
`DHUN_AUDIO_RESOLVER_URL` contract. Resolves the submitted URL, video ID,
or song title/artist via `yt-dlp` in guest mode, converts to MP3 (deleting
the original downloaded video), and returns either:
- Direct audio bytes (`audio/mpeg`) when `RESOLVER_RESPONSE_MODE=binary`, or
- JSON (`direct_https_audio_url` + `expected_sha256`) pointing to a
  token-gated `/files/{file_id}.mp3` endpoint on this service.
"""

from __future__ import annotations

import asyncio
import hashlib
import hmac
import json
import os
import re
import shutil
import tempfile
import time
import uuid
from pathlib import Path
from typing import TYPE_CHECKING, Any, Optional

import yt_dlp
from fastapi import FastAPI, Header, HTTPException, Query, Request
from fastapi.responses import FileResponse, JSONResponse
from pydantic import BaseModel

if TYPE_CHECKING:
    from yt_dlp import _YoutubeDLOptions

RESOLVER_API_KEY = os.getenv("RESOLVER_API_KEY", "").strip()
DEFAULT_RESPONSE_MODE = os.getenv("RESOLVER_RESPONSE_MODE", "binary").strip().lower()
PUBLIC_BASE_URL = os.getenv("PUBLIC_BASE_URL", "").rstrip("/")

DEFAULT_TEMP_DIR = Path(tempfile.gettempdir()) / "dhun_resolver_audio"
TEMP_AUDIO_DIR = Path(os.getenv("TEMP_AUDIO_DIR", str(DEFAULT_TEMP_DIR)))
TEMP_AUDIO_DIR.mkdir(parents=True, exist_ok=True)

FILE_TTL_SECONDS = int(os.getenv("FILE_TTL_SECONDS", "86400"))
CACHE_MAX_FILES = int(os.getenv("CACHE_MAX_FILES", "150"))

# Serialize yt-dlp requests so concurrent calls never burst YouTube at once
YTDLP_SEMAPHORE = asyncio.Semaphore(1)

NOISE_PATTERN = re.compile(
    r"(\(|\[)\s*(official\s*(music\s*)?video|official\s*audio|lyric(s|al)?(\s*video)?|"
    r"full\s*song|4k|hd|audio|video\s*song|feat\.?[^)\]]*)\s*(\)|\])",
    re.IGNORECASE,
)

app = FastAPI(
    title="DHUN yt-dlp Resolver Service",
    version="2.1.0",
    description="Downloads and converts media to MP3 via yt-dlp in guest mode while deleting source video files.",
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


def build_ytdlp_target(payload: ResolverLookupPayload) -> tuple[str, str]:
    """Determines the best target URL or search query for yt-dlp and its cache key."""
    video_id = (payload.provider_video_id or "").strip()
    if re.fullmatch(r"[A-Za-z0-9_-]{11}", video_id):
        return f"https://www.youtube.com/watch?v={video_id}", f"yt_{video_id}"

    raw_url = (payload.source_url or "").strip()
    if raw_url.startswith(("http://", "https://")):
        url_hash = hashlib.sha256(raw_url.encode("utf-8")).hexdigest()[:16]
        return raw_url, f"url_{url_hash}"

    cleaned_title = NOISE_PATTERN.sub("", payload.title or "").strip(" -_|")
    cleaned_artist = (payload.artist or "").strip()
    if cleaned_artist.lower() in {"", "unknown artist", "youtube creator", "various artists"}:
        cleaned_artist = ""

    query = (
        f"{cleaned_title} {cleaned_artist}".strip()
        if cleaned_artist and cleaned_artist.lower() not in cleaned_title.lower()
        else (cleaned_title or payload.title.strip())
    )
    if not query:
        raise HTTPException(status_code=400, detail="A valid source_url, video ID, or title is required")

    query_hash = hashlib.sha256(query.lower().encode("utf-8")).hexdigest()[:16]
    return f"ytsearch1:{query}", f"q_{query_hash}"


def cleanup_file(path: Path) -> None:
    """Safely removes temporary audio/video files from disk."""
    try:
        if path.exists():
            path.unlink()
    except OSError:
        pass


def purge_expired_files() -> None:
    """Cleans up expired MP3s and enforces CACHE_MAX_FILES LRU limit."""
    now = time.time()
    cached_mp3s: list[tuple[float, Path]] = []

    for item in TEMP_AUDIO_DIR.glob("*.mp3"):
        try:
            if not item.is_file():
                continue
            mtime = item.stat().st_mtime
            if (now - mtime) > FILE_TTL_SECONDS:
                cleanup_file(item)
                cleanup_file(item.with_suffix(".json"))
            else:
                cached_mp3s.append((mtime, item))
        except OSError:
            pass

    if len(cached_mp3s) > CACHE_MAX_FILES:
        cached_mp3s.sort(key=lambda pair: pair[0])
        for _, old_mp3 in cached_mp3s[: len(cached_mp3s) - CACHE_MAX_FILES]:
            cleanup_file(old_mp3)
            cleanup_file(old_mp3.with_suffix(".json"))


def compute_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def get_cached_mp3(cache_key: str) -> Optional[tuple[Path, dict[str, Any]]]:
    """Returns cached MP3 path and saved metadata if already downloaded."""
    mp3_path = TEMP_AUDIO_DIR / f"{cache_key}.mp3"
    meta_path = TEMP_AUDIO_DIR / f"{cache_key}.json"
    if mp3_path.exists() and mp3_path.stat().st_size > 0:
        os.utime(mp3_path, None)
        meta: dict[str, Any] = {}
        if meta_path.exists():
            try:
                meta = json.loads(meta_path.read_text(encoding="utf-8"))
            except Exception:
                meta = {}
        return mp3_path, meta
    return None


def download_and_convert_to_mp3(target: str, cache_key: str) -> tuple[Path, dict[str, Any]]:
    """Downloads via yt-dlp in guest mode, converts to MP3 with FFmpeg, and deletes the original video."""
    cached = get_cached_mp3(cache_key)
    if cached is not None:
        return cached

    job_id = str(uuid.uuid4())
    output_template = str(TEMP_AUDIO_DIR / f"tmp_{job_id}.%(ext)s")
    temp_mp3 = TEMP_AUDIO_DIR / f"tmp_{job_id}.mp3"
    final_mp3 = TEMP_AUDIO_DIR / f"{cache_key}.mp3"
    final_meta = TEMP_AUDIO_DIR / f"{cache_key}.json"

    ydl_opts: _YoutubeDLOptions = {
        "format": "bestaudio/best",
        "outtmpl": output_template,
        "noplaylist": True,
        "sleep_interval": 2,
        "max_sleep_interval": 6,
        "js_runtimes": {"deno": {}, "node": {}},
        "quiet": True,
        "no_warnings": True,
        "postprocessors": [
            {
                "key": "FFmpegExtractAudio",
                "preferredcodec": "mp3",
                "preferredquality": "192",
            }
        ],
    }

    try:
        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            raw_info = ydl.extract_info(target, download=True)
            info: dict[str, Any] = dict(raw_info) if raw_info else {}
            entries = info.get("entries")
            if entries is not None:
                first_entry = next(iter(entries), None)
                info = dict(first_entry) if first_entry else {}
    finally:
        # Ensure no leftover non-mp3 source files remain for this job_id
        for leftover in TEMP_AUDIO_DIR.glob(f"tmp_{job_id}.*"):
            if leftover.suffix.lower() != ".mp3":
                cleanup_file(leftover)

    if not temp_mp3.exists():
        raise RuntimeError("MP3 conversion failed (verify FFmpeg is installed and in PATH)")

    shutil.move(str(temp_mp3), str(final_mp3))
    meta_summary = {
        "id": str(info.get("id") or ""),
        "title": str(info.get("title") or ""),
    }
    try:
        final_meta.write_text(json.dumps(meta_summary), encoding="utf-8")
    except OSError:
        pass

    return final_mp3, meta_summary


@app.get("/health")
async def health() -> dict[str, Any]:
    return {
        "status": "ok",
        "service": "dhun-ytdlp-resolver",
        "provider": "yt-dlp",
        "auth_required": bool(RESOLVER_API_KEY),
        "default_response_mode": DEFAULT_RESPONSE_MODE,
        "cached_tracks": len(list(TEMP_AUDIO_DIR.glob("*.mp3"))),
    }


@app.get("/files/{file_id}.mp3")
async def serve_temp_mp3(
    file_id: str,
    token: Optional[str] = Query(default=None),
    authorization: Optional[str] = Header(default=None),
):
    """Serves the converted MP3 when operating in JSON response mode."""
    if RESOLVER_API_KEY:
        supplied = (token or "").strip()
        if not supplied and authorization and authorization.lower().startswith("bearer "):
            supplied = authorization[7:].strip()
        if not supplied or not hmac.compare_digest(supplied, RESOLVER_API_KEY):
            raise HTTPException(status_code=403, detail="Invalid or missing file access token")

    if not re.fullmatch(r"(yt_[A-Za-z0-9_-]{11}|(url|q)_[0-9a-f]{16})", file_id):
        raise HTTPException(status_code=400, detail="Invalid file identifier")

    mp3_path = TEMP_AUDIO_DIR / f"{file_id}.mp3"
    if not mp3_path.exists():
        raise HTTPException(status_code=404, detail="Audio file expired or not found")

    os.utime(mp3_path, None)
    return FileResponse(path=mp3_path, media_type="audio/mpeg", filename=f"{file_id}.mp3")


@app.post("/resolve")
async def resolve_audio(
    payload: ResolverLookupPayload,
    request: Request,
    mode: Optional[str] = Query(default=None, pattern="^(json|binary)$"),
    authorization: Optional[str] = Header(default=None),
):
    verify_bearer_token(authorization)
    purge_expired_files()

    target, cache_key = build_ytdlp_target(payload)

    cached = get_cached_mp3(cache_key)
    if cached is not None:
        mp3_path, info = cached
    else:
        async with YTDLP_SEMAPHORE:
            try:
                mp3_path, info = await asyncio.to_thread(
                    download_and_convert_to_mp3, target, cache_key
                )
            except Exception as exc:
                raise HTTPException(status_code=502, detail=f"yt-dlp extraction failed: {exc}")

    resolved_title = str(info.get("title") or payload.title)
    resolved_id = str(info.get("id") or payload.provider_video_id or cache_key)
    license_note = f"yt-dlp Guest Audio (id={resolved_id}, title='{resolved_title}')"

    response_mode = (mode or DEFAULT_RESPONSE_MODE).lower()
    if response_mode == "binary":
        return FileResponse(
            path=mp3_path,
            media_type="audio/mpeg",
            filename=f"{resolved_id}.mp3",
            headers={"X-DHUN-License-Note": license_note},
        )

    sha256_hex = await asyncio.to_thread(compute_sha256, mp3_path)
    base_url = PUBLIC_BASE_URL or str(request.base_url).rstrip("/")
    token_suffix = f"?token={RESOLVER_API_KEY}" if RESOLVER_API_KEY else ""
    direct_url = f"{base_url}/files/{cache_key}.mp3{token_suffix}"

    return JSONResponse(
        content=ResolverJsonResponse(
            direct_https_audio_url=direct_url,
            license_or_authorization_note=license_note,
            expected_sha256=sha256_hex,
        ).model_dump()
    )