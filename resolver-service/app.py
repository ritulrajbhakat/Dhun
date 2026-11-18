import os
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import yt_dlp

app = FastAPI(title="DHUN Local Audio Resolver")

VENV_SCRIPTS_PATH = r"C:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service\.venv\Scripts"

class ResolveRequest(BaseModel):
    url: str
    title: str

@app.post("/resolve")
async def resolve_audio(req: ResolveRequest):
    ydl_opts = {
        'format': 'bestaudio/best',
        'postprocessors': [{
            'key': 'FFmpegExtractAudio',
            'preferredcodec': 'mp3',
            'preferredquality': '192',
        }],
        'ffmpeg_location': VENV_SCRIPTS_PATH,
        'quiet': True,
        'no_warnings': True,
    }

    try:
        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            info = ydl.extract_info(req.url, download=False)
            if not info:
                raise HTTPException(status_code=400, detail="Failed to extract info from URL")

            entries = info.get('entries')
            if entries:
                entry_list = [e for e in entries if e is not None]
                video_info = entry_list[0] if entry_list else info
            else:
                video_info = info

            audio_url = video_info.get('url')
            if not audio_url:
                formats = video_info.get('formats') or []
                audio_formats = [f for f in formats if f.get('acodec') != 'none' and f.get('vcodec') == 'none']
                if audio_formats:
                    audio_url = audio_formats[0].get('url')
                else:
                    audio_url = video_info.get('webpage_url')

            return {
                "status": "success",
                "title": video_info.get('title', req.title),
                "duration": video_info.get('duration'),
                "direct_https_audio_url": audio_url
            }
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"yt-dlp extraction failed: {str(e)}")

@app.get("/api/local/health")
async def health_check():
    return {"status": "healthy", "version": "2.1.0"}