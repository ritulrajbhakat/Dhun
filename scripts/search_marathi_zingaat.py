import yt_dlp
import json

ydl_opts = {'quiet': True, 'no_warnings': True}
with yt_dlp.YoutubeDL(ydl_opts) as ydl:
    res = ydl.extract_info('ytsearch5:Zingaat Marathi Sairat Ajay Atul Official', download=False)
    for entry in res['entries']:
        print(f"{entry['id']} ::: {entry['title']} ::: {entry['duration']}s ::: {entry.get('channel')}")
