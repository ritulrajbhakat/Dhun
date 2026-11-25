import asyncio
import os
import random
import sys

# Automatically prepend the .venv\Scripts folder (where python, ffmpeg, and ffprobe live) 
# to PATH so yt-dlp's subprocesses find ffmpeg immediately without manual venv activation.
venv_bin_path = os.path.dirname(os.path.abspath(sys.executable))
if venv_bin_path not in os.environ["PATH"]:
    os.environ["PATH"] = venv_bin_path + os.pathsep + os.environ["PATH"]

from fastapi import FastAPI, HTTPException
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel
import yt_dlp

app = FastAPI(title="DHUN Local Audio Resolver")

@app.on_event("startup")
async def start_parent_watchdog():
    parent_pid_str = os.environ.get("DHUN_PARENT_PID")
    if not parent_pid_str or not parent_pid_str.isdigit():
        return
    parent_pid = int(parent_pid_str)

    def is_parent_alive():
        if os.name == 'nt':
            import ctypes
            SYNCHRONIZE = 0x00100000
            PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
            handle = ctypes.windll.kernel32.OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, False, parent_pid)
            if not handle:
                return False
            res = ctypes.windll.kernel32.WaitForSingleObject(handle, 0)
            ctypes.windll.kernel32.CloseHandle(handle)
            return res == 0x00000102
        else:
            try:
                os.kill(parent_pid, 0)
                return True
            except OSError:
                return False

    async def watchdog_loop():
        while True:
            await asyncio.sleep(1.0)
            if not is_parent_alive():
                print(f"[Watchdog] Parent DHUN process ({parent_pid}) terminated. Shutting down resolver.")
                os._exit(0)

    asyncio.create_task(watchdog_loop())

# Create local cache directory for resolved MP3 files
CACHE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cache")
os.makedirs(CACHE_DIR, exist_ok=True)

# Mount cache folder so local MP3s are instantly servable over loopback HTTP
app.mount("/cache", StaticFiles(directory=CACHE_DIR), name="cache")

class ResolveRequest(BaseModel):
    source_url: str
    provider_video_id: str = ""
    title: str = ""
    artist: str = ""
    version_tag: str = "original"

@app.post("/resolve")
async def resolve_audio(req: ResolveRequest):
    try:
        # Random 15–20 second non-blocking delay before hitting yt-dlp
        delay = random.uniform(15, 20)
        await asyncio.sleep(delay)

        # Extract video info first to get the clean video ID
        with yt_dlp.YoutubeDL({'quiet': True, 'no_warnings': True}) as ydl:
            info = ydl.extract_info(req.source_url, download=False)
            if not info:
                raise HTTPException(status_code=400, detail="Failed to extract info from URL")

            entries = info.get('entries')
            if entries:
                entry_list = [e for e in entries if e is not None]
                video_info = entry_list[0] if entry_list else info
            else:
                video_info = info

            video_id = video_info.get('id', 'audio_track')
            expected_mp3_path = os.path.join(CACHE_DIR, f"{video_id}.mp3")
            local_audio_url = f"http://127.0.0.1:8000/cache/{video_id}.mp3"

            # If already downloaded and cached, return immediately
            if os.path.exists(expected_mp3_path):
                return {
                    "direct_https_audio_url": local_audio_url,
                    "license_or_authorization_note": "Cached locally via yt-dlp + ffmpeg",
                    "expected_sha256": None,
                    "status": "success",
                    "title": video_info.get('title', req.title),
                    "duration": video_info.get('duration')
                }

        # Otherwise, download and convert to MP3 using local ffmpeg
        ydl_opts = {
            'format': 'bestaudio/best',
            'postprocessors': [{
                'key': 'FFmpegExtractAudio',
                'preferredcodec': 'mp3',
                'preferredquality': '192',
            }],
            'outtmpl': os.path.join(CACHE_DIR, '%(id)s.%(ext)s'),
            'ffmpeg_location': venv_bin_path,
            'quiet': True,
            'no_warnings': True,
        }

        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            ydl.download([req.source_url])

        return {
            "direct_https_audio_url": local_audio_url,
            "license_or_authorization_note": "Resolved and converted locally via yt-dlp + ffmpeg",
            "expected_sha256": None,
            "status": "success",
            "title": video_info.get('title', req.title),
            "duration": video_info.get('duration')
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"yt-dlp extraction/conversion failed: {str(e)}")

@app.get("/api/local/health")
async def health_check():
    return {"status": "healthy", "version": "2.1.0"}