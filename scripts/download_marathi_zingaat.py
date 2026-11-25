import os
import sys
import json
import yt_dlp

BASE_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun"
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
VENV_BIN = os.path.join(BASE_DIR, "resolver-service", ".venv", "Scripts")

os.environ["PATH"] = VENV_BIN + os.pathsep + os.environ["PATH"]

target_id = "8eong8XHDB4"
url = f"https://www.youtube.com/watch?v={target_id}"

ydl_opts = {
    'format': 'bestaudio/best',
    'postprocessors': [{
        'key': 'FFmpegExtractAudio',
        'preferredcodec': 'mp3',
        'preferredquality': '320',
    }],
    'outtmpl': os.path.join(CACHE_DIR, '%(id)s.%(ext)s'),
    'ffmpeg_location': VENV_BIN,
    'quiet': False,
    'no_warnings': True,
}

print(f"Downloading Marathi Sairat Zingaat ({target_id})...")
with yt_dlp.YoutubeDL(ydl_opts) as ydl:
    info = ydl.extract_info(url, download=True)
    out_file = os.path.join(CACHE_DIR, f"{target_id}.mp3")
    print(f"Downloaded: {out_file}, Size: {os.path.getsize(out_file)} bytes, Title: {info['title']}")
