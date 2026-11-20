import os
import sys
import json
import urllib.parse
from http.server import HTTPServer, SimpleHTTPRequestHandler

PORT = 8888
RECORDINGS_DIR = r"C:\Users\ritul\.gemini\antigravity\brain\54150f37-588d-4187-8e67-1cb7efa91897\recordings"
os.makedirs(RECORDINGS_DIR, exist_ok=True)

SCRATCH_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE_DIR = os.path.join(SCRATCH_DIR, "resolver-service", "cache")

class AudioEvalHandler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "*")
        super().end_headers()

    def do_OPTIONS(self):
        self.send_response(200)
        self.end_headers()

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path

        if path == "/health":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"status":"ok"}')
            return

        if path.startswith("/audio/"):
            fname = path[len("/audio/"):]
            fpath = os.path.join(CACHE_DIR, fname)
            if os.path.exists(fpath):
                self.send_response(200)
                self.send_header("Content-Type", "audio/mpeg")
                self.send_header("Content-Length", str(os.path.getsize(fpath)))
                self.send_header("Accept-Ranges", "bytes")
                self.end_headers()
                with open(fpath, "rb") as f:
                    self.wfile.write(f.read())
                return
            else:
                self.send_error(404, f"Audio file not found: {fname}")
                return

        if path == "/" or path == "/render.html":
            html_path = os.path.join(os.path.dirname(__file__), "render_audio.html")
            if os.path.exists(html_path):
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(os.path.getsize(html_path)))
                self.end_headers()
                with open(html_path, "rb") as f:
                    self.wfile.write(f.read())
                return

        super().do_GET()

    def do_POST(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path

        if path == "/api/save_audio":
            query = urllib.parse.parse_qs(parsed.query)
            filename = query.get("filename", ["output.wav"])[0]
            # Sanitize filename
            filename = os.path.basename(filename)
            content_len = int(self.headers.get("Content-Length", 0))
            audio_bytes = self.rfile.read(content_len)

            dest_path = os.path.join(RECORDINGS_DIR, filename)
            with open(dest_path, "wb") as f:
                f.write(audio_bytes)

            print(f"[SERVER] Successfully saved: {filename} ({len(audio_bytes)} bytes)")
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"status": "saved", "path": dest_path, "bytes": len(audio_bytes)}).encode("utf-8"))
            return

        self.send_error(404, "Endpoint not found")

if __name__ == "__main__":
    server = HTTPServer(("127.0.0.1", PORT), AudioEvalHandler)
    print(f"Audio Eval Server listening on http://127.0.0.1:{PORT}")
    server.serve_forever()
