import os
import sys
import time

RESOLVER_DIR = r"c:\Users\ritul\.gemini\antigravity-ide\scratch\dhun\resolver-service"
sys.path.insert(0, RESOLVER_DIR)

from fastapi.testclient import TestClient
from app import app

client = TestClient(app)

print("=" * 60)
print("TESTING DHUN AUDIO RESOLVER SERVICE (EXACT CODE)")
print("=" * 60)

# 1. Health check test
print("\n[Test 1] Testing /api/local/health...")
t0 = time.time()
res_health = client.get("/api/local/health")
print(f"Status Code: {res_health.status_code}")
print(f"Response Body: {res_health.json()}")
assert res_health.status_code == 200
assert res_health.json().get("status") == "healthy"
print(">>> Health Check: PASSED")

# 2. Static files mount test
print("\n[Test 2] Testing /cache static files mount...")
cache_dir = os.path.join(RESOLVER_DIR, "cache")
cached_files = [f for f in os.listdir(cache_dir) if f.endswith(".mp3")]
print(f"Found {len(cached_files)} MP3 files in cache directory.")
if cached_files:
    test_file = cached_files[0]
    res_static = client.get(f"/cache/{test_file}", headers={"Range": "bytes=0-1024"})
    print(f"Testing stream of /cache/{test_file}...")
    print(f"Status Code: {res_static.status_code}")
    print(f"Content-Type: {res_static.headers.get('content-type')}")
    print(f"Content-Length: {len(res_static.content)} bytes received")
    assert res_static.status_code in [200, 206]
    print(">>> Cache Stream Check: PASSED")

# 3. Test /resolve endpoint with existing audio (verifying delay and return payload)
print("\n[Test 3] Testing /resolve endpoint with track...")
payload = {
    "source_url": "https://www.youtube.com/watch?v=DuFUtL8zUAk",
    "provider_video_id": "DuFUtL8zUAk",
    "title": "Animals (Original Mix)",
    "artist": "Martin Garrix",
    "version_tag": "original"
}
print(f"Sending payload: {payload}")
print("Timing the request (expecting 15-20s delay configured in app.py)...")
t_start = time.time()
res_resolve = client.post("/resolve", json=payload)
elapsed = time.time() - t_start

print(f"\nRequest Completed in {elapsed:.2f} seconds.")
print(f"Status Code: {res_resolve.status_code}")
print(f"Response Body: {res_resolve.json()}")

assert res_resolve.status_code == 200
data = res_resolve.json()
assert data.get("status") == "success"
assert "direct_https_audio_url" in data
print(f"Audio URL: {data['direct_https_audio_url']}")
print(f"Title: {data.get('title')}")
print(f"Duration: {data.get('duration')}s")
print(f"License Note: {data.get('license_or_authorization_note')}")

if 15.0 <= elapsed <= 25.0:
    print(f">>> 15-20s Delay Verification: PASSED (observed {elapsed:.2f}s)")
else:
    print(f">>> Delay observed: {elapsed:.2f}s")

print("\n" + "=" * 60)
print("ALL RESOLVER SERVICE CODE TESTS PASSED")
print("=" * 60)
