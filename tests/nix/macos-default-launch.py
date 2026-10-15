"""Temporarily route HTTP test URLs to an isolated App; restore defaults on every exit."""
import http.server
import json
from pathlib import Path
import subprocess
import sys
import time

helper, app = sys.argv[1:3]
root = Path('/private/tmp/nexo-macos-launch-test')
root.mkdir(exist_ok=True)
report = root / 'result.json'
original = {scheme: subprocess.check_output([helper, 'resolve', scheme + '://example.com']).decode().strip() for scheme in ['http']}
state = {'original': original, 'status': 'starting', 'requested_url': None, 'restored': False}
def save():
    report.write_text(json.dumps(state, indent=2))
class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path.startswith('/default-browser-launch'):
            state['requested_url'] = self.path
            save()
        data = b'<!doctype html><title>Nexo default-browser launch passed</title><h1>Nexo default-browser launch passed</h1><p>Local isolated launch test.</p>'
        self.send_response(200)
        self.send_header('Content-Type', 'text/html')
        self.end_headers()
        self.wfile.write(data)
    def log_message(self, *args):
        pass
server = http.server.HTTPServer(('127.0.0.1', 18765), Handler)
server.timeout = 0.25
try:
    subprocess.run([helper, 'register', app], check=True)
    for scheme in ['http']:
        subprocess.run([helper, 'set-path', scheme, app], check=True)
        resolved = subprocess.check_output([helper, 'resolve', scheme + '://127.0.0.1:18765/']).decode().strip()
        if resolved != app:
            raise RuntimeError(f'{scheme} did not select isolated test App: {resolved}')
    state['status'] = 'ready'
    save()
    print('Temporary test handlers ready; automatic restoration in 90 seconds', flush=True)
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        server.handle_request()
        if state['requested_url']:
            state['status'] = 'url_delivered'
            break
finally:
    restore_errors = []
    for scheme, path in original.items():
        result = subprocess.run([helper, 'set-path', scheme, path], capture_output=True)
        if result.returncode:
            restore_errors.append({'scheme': scheme, 'error': result.stderr.decode().strip()})
    restored = {scheme: subprocess.check_output([helper, 'resolve', scheme + '://example.com']).decode().strip() for scheme in original}
    state['restored'] = restored == original
    state['restore_errors'] = restore_errors
    save()
    server.server_close()
    print(json.dumps(state), flush=True)

if state['status'] != 'url_delivered' or not state['restored'] or state['restore_errors']:
    raise SystemExit('Launch or handler restoration did not pass')
