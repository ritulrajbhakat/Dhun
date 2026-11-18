"""
DHUN yt-dlp Media Resolver Service (FastAPI)

Exposes an HTTP/HTTPS endpoint compatible with `dhun-local-helper`'s
`DHUN_AUDIO_RESOLVER_URL` contract. Resolves the submitted URL, video ID,
or song title/artist via `yt-dlp`, converts to MP3 (deleting the original
downloaded video), and returns either:
- Direct audio bytes (`audio/mpeg`) when `RESOLVER_RESPONSE_MODE=binary`, or
- JSON (`direct_https_audio_url` + `expected_sha256`) pointing to a temporary
  token-gated `/files/{file_id}.mp3` endpoint on this service.
"""

from __future__ import annotations

import asyncio
import hashlib
import hmac
import os
import re
import tempfile
import time
import uuid
from pathlib import Path
from typing import Any, Optional

import yt_dlp
from fastapi import BackgroundTasks, FastAPI, Header, HTTPException, Query, Request
from fastapi.responses import FileResponse, JSONResponse
from pydantic import BaseModel

RESOLVER_API_KEY = os.getenv("RESOLVER_API_KEY", "").strip()
DEFAULT_RESPONSE_MODE = os.getenv("RESOLVER_RESPONSE_MODE", "binary").strip().lower()
PUBLIC_BASE_URL = os.getenv("PUBLIC_BASE_URL", "").rstrip("/")
DEFAULT_TEMP_DIR = Path(tempfile.gettempdir()) / "dhun_resolver_audio"
TEMP_AUDIO_DIR = Path(os.getenv("TEMP_AUDIO_DIR", str(DEFAULT_TEMP_DIR)))
TEMP_AUDIO_DIR.mkdir(parents=True, exist_ok=True)
FILE_TTL_SECONDS = int(os.getenv("FILE_TTL_SECONDS", "600"))

NOISE_PATTERN = re.compile(
    r"(\(|\[)\s*(official\s*(music\s*)?video|official\s*audio|lyric(s|al)?(\s*video)?|"
    r"full\s*song|4k|hd|audio|video\s*song|feat\.?[^)\]]*)\s*(\)|\])",
    re.IGNORECASE,
)

app = FastAPI(
    title="DHUN yt-dlp Resolver Service",
    version="2.0.0",
    description="Downloads and converts media to MP3 via yt-dlp while deleting source video files.",
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


def build_ytdlp_target(payload: ResolverLookupPayload) -> str:
    """Determines the best target URL or search query for yt-dlp."""
    raw_url = (payload.source_url or "").strip()
    if raw_url.startswith(("http://", "https://")):
        return raw_url

    video_id = (payload.provider_video_id or "").strip()
    if re.fullmatch(r"[A-Za-z0-9_-]{11}", video_id):
        return f"https://www.youtube.com/watch?v={video_id}"

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
    return f"ytsearch1:{query}"


def cleanup_file(path: Path) -> None:
    """Safely removes temporary audio/video files from disk."""
    try:
        if path.exists():
            path.unlink()
    except OSError:
        pass


def purge_expired_files() -> None:
    """Cleans up any leftover temporary files older than FILE_TTL_SECONDS."""
    now = time.time()
    for item in TEMP_AUDIO_DIR.glob("*"):
        try:
            if item.is_file() and (now - item.stat().st_mtime) > FILE_TTL_SECONDS:
                item.unlink()
        except OSError:
            pass


def compute_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download_and_convert_to_mp3(target: str, job_id: str) -> tuple[Path, dict[str, Any]]:
    """Downloads via yt-dlp, converts to MP3 with FFmpeg, and deletes the original video."""
    output_template = str(TEMP_AUDIO_DIR / f"{job_id}.%(ext)s")
    expected_mp3 = TEMP_AUDIO_DIR / f"{job_id}.mp3"

    with yt_dlp.YoutubeDL(
        {
            "format": "bestaudio/best",
            "outtmpl": output_template,
            "keepvideo": False,  # Deletes original video/stream immediately after MP3 extraction
            "noplaylist": True,
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
    ) as ydl:
        raw_info = ydl.extract_info(target, download=True)
        info: dict[str, Any] = dict(raw_info) if raw_info else {}
        entries = info.get("entries")
        if entries is not None:
            first_entry = next(iter(entries), None)
            info = dict(first_entry) if first_entry else {}

    # Ensure no leftover non-mp3 source files remain for this job_id
    for leftover in TEMP_AUDIO_DIR.glob(f"{job_id}.*"):
        if leftover.suffix.lower() != ".mp3":
            cleanup_file(leftover)

    if not expected_mp3.exists():
        raise RuntimeError("MP3 conversion failed (verify FFmpeg is installed and in PATH)")

    return expected_mp3, info


@app.get("/health")
async def health() -> dict[str, Any]:
    return {
        "status": "ok",
        "service": "dhun-ytdlp-resolver",
        "provider": "yt-dlp",
        "auth_required": bool(RESOLVER_API_KEY),
        "default_response_mode": DEFAULT_RESPONSE_MODE,
    }


@app.get("/files/{file_id}.mp3")
async def serve_temp_mp3(
    file_id: str,
    background_tasks: BackgroundTasks,
    token: Optional[str] = Query(default=None),
    authorization: Optional[str] = Header(default=None),
):
    """Serves the temporary converted MP3 when operating in JSON response mode."""
    if RESOLVER_API_KEY:
        supplied = (token or "").strip()
        if not supplied and authorization and authorization.lower().startswith("bearer "):
            supplied = authorization[7:].strip()
        if not supplied or not hmac.compare_digest(supplied, RESOLVER_API_KEY):
            raise HTTPException(status_code=403, detail="Invalid or missing file access token")

    if not re.fullmatch(r"[0-9a-f-]{36}", file_id):
        raise HTTPException(status_code=400, detail="Invalid file identifier")

    mp3_path = TEMP_AUDIO_DIR / f"{file_id}.mp3"
    if not mp3_path.exists():
        raise HTTPException(status_code=404, detail="Audio file expired or not found")

    background_tasks.add_task(cleanup_file, mp3_path)
    return FileResponse(path=mp3_path, media_type="audio/mpeg", filename=f"{file_id}.mp3")


@app.post("/resolve")
async def resolve_audio(
    payload: ResolverLookupPayload,
    request: Request,
    background_tasks: BackgroundTasks,
    mode: Optional[str] = Query(default=None, pattern="^(json|binary)$"),
    authorization: Optional[str] = Header(default=None),
):
    verify_bearer_token(authorization)
    purge_expired_files()

    target = build_ytdlp_target(payload)
    job_id = str(uuid.uuid4())

    try:
        mp3_path, info = await asyncio.to_thread(download_and_convert_to_mp3, target, job_id)
    except Exception as exc:
        raise HTTPException(status_code=502, detail=f"yt-dlp extraction failed: {exc}")

    resolved_title = str(info.get("title") or payload.title)
    resolved_id = str(info.get("id") or payload.provider_video_id or job_id)
    license_note = f"yt-dlp Extracted Audio (id={resolved_id}, title='{resolved_title}')"

    response_mode = (mode or DEFAULT_RESPONSE_MODE).lower()
    if response_mode == "binary":
        # Delete the converted MP3 immediately after streaming it to dhun-local-helper
        background_tasks.add_task(cleanup_file, mp3_path)
        return FileResponse(
            path=mp3_path,
            media_type="audio/mpeg",
            filename=f"{resolved_id}.mp3",
            headers={"X-DHUN-License-Note": license_note},
        )

    sha256_hex = await asyncio.to_thread(compute_sha256, mp3_path)
    base_url = PUBLIC_BASE_URL or str(request.base_url).rstrip("/")
    token_suffix = f"?token={RESOLVER_API_KEY}" if RESOLVER_API_KEY else ""
    direct_url = f"{base_url}/files/{job_id}.mp3{token_suffix}"

    return JSONResponse(
        content=ResolverJsonResponse(
            direct_https_audio_url=direct_url,
            license_or_authorization_note=license_note,
            expected_sha256=sha256_hex,
        ).model_dump()
    