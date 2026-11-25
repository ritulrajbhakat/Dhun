import os
import sys
import json
import yt_dlp

# Set paths
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE_DIR = os.path.join(BASE_DIR, "resolver-service", "cache")
VENV_BIN = os.path.join(BASE_DIR, "resolver-service", ".venv", "Scripts")
os.makedirs(CACHE_DIR, exist_ok=True)

# Add venv to path so ffmpeg is discoverable
if VENV_BIN not in os.environ["PATH"]:
    os.environ["PATH"] = VENV_BIN + os.pathsep + os.environ["PATH"]

TRACKS = [
    {
        "id": "animals",
        "title": "Animals (Original Mix)",
        "artist": "Martin Garrix",
        "url": "https://www.youtube.com/watch?v=DuFUtL8zUAk",
        "expected_id": "DuFUtL8zUAk"
    },
    {
        "id": "titanium",
        "title": "Titanium (feat. Sia)",
        "artist": "David Guetta",
        "url": "https://www.youtube.com/watch?v=JRfuAukYTKg",
        "expected_id": "JRfuAukYTKg"
    },
    {
        "id": "dont_start_now",
        "title": "Don't Start Now",
        "artist": "Dua Lipa",
        "url": "https://www.youtube.com/watch?v=oygrmJFKYZY",
        "expected_id": "oygrmJFKYZY"
    },
    {
        "id": "ghungroo",
        "title": "Ghungroo (From \"War\")",
        "artist": "Vishal-Shekhar, Arijit Singh & Shilpa Rao",
        "url": "https://youtu.be/qFkNATtc3mc",
        "expected_id": "qFkNATtc3mc"
    },
    {
        "id": "zingaat",
        "title": "Zingaat",
        "artist": "Ajay-Atul",
        "url": "https://www.youtube.com/watch?v=8eong8XHDB4",
        "expected_id": "8eong8XHDB4"
    },
    {
        "id": "malhari",
        "title": "Malhari (From \"Bajirao Mastani\")",
        "artist": "Vishal Dadlani",
        "url": "https://www.youtube.com/watch?v=sRlA7JWTj04",
        "expected_id": "sRlA7JWTj04"
    },
    {
        "id": "big_dawgs",
        "title": "Big Dawgs",
        "artist": "Hanumankind & Kalmi",
        "url": "https://www.youtube.com/watch?v=hOHKltAiKXQ",
        "expected_id": "hOHKltAiKXQ"
    },
    {
        "id": "blinding_lights",
        "title": "Blinding Lights",
        "artist": "The Weeknd",
        "url": "https://www.youtube.com/watch?v=fHI8X4OXluQ",
        "expected_id": "fHI8X4OXluQ"
    },
    {
        "id": "chaiyya_chaiyya",
        "title": "Chaiyya Chaiyya (From \"Dil Se\")",
        "artist": "Sukhwinder Singh & Sapna Awasthi",
        "url": "https://www.youtube.com/watch?v=49oz7P9d8IA",
        "expected_id": "49oz7P9d8IA"
    },
    {
        "id": "perfect",
        "title": "Perfect",
        "artist": "Ed Sheeran",
        "url": "https://www.youtube.com/watch?v=1UQzJfsT2eo",
        "expected_id": "1UQzJfsT2eo"
    }
]

def acquire_tracks():
    results = []
    print(f"Starting acquisition of {len(TRACKS)} stress-test tracks...")
    
    ydl_opts = {
        'format': 'bestaudio/best',
        'postprocessors': [{
            'key': 'FFmpegExtractAudio',
            'preferredcodec': 'mp3',
            'preferredquality': '192',
        }],
        'outtmpl': os.path.join(CACHE_DIR, '%(id)s.%(ext)s'),
        'ffmpeg_location': VENV_BIN,
        'quiet': False,
        'no_warnings': True,
    }

    for item in TRACKS:
        print(f"\n--- Acquiring: {item['title']} - {item['artist']} ---")
        out_file = os.path.join(CACHE_DIR, f"{item['expected_id']}.mp3")
        
        if os.path.exists(out_file) and os.path.getsize(out_file) > 100000:
            print(f"Already cached: {out_file} ({os.path.getsize(out_file)} bytes)")
            results.append({
                "track": item,
                "file_path": out_file,
                "file_size": os.path.getsize(out_file),
                "status": "cached"
            })
            continue

        try:
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                info = ydl.extract_info(item['url'], download=True)
                video_id = info.get('id', item['expected_id'])
                final_path = os.path.join(CACHE_DIR, f"{video_id}.mp3")
                size = os.path.getsize(final_path) if os.path.exists(final_path) else 0
                duration = info.get('duration')
                print(f"Successfully downloaded: {final_path} (size={size}, duration={duration}s)")
                results.append({
                    "track": item,
                    "video_id": video_id,
                    "file_path": final_path,
                    "file_size": size,
                    "duration": duration,
                    "status": "success"
                })
        except Exception as e:
            print(f"Error acquiring {item['title']}: {e}")
            results.append({
                "track": item,
                "error": str(e),
                "status": "failed"
            })

    summary_path = os.path.join(BASE_DIR, "scripts", "acquisition_summary.json")
    with open(summary_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2)
    print(f"\nAcquisition complete! Summary saved to {summary_path}")

if __name__ == "__main__":
    acquire_tracks()
