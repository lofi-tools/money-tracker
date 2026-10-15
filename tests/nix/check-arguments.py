"""Exercise the actual Nix-generated launcher against a harmless argv recorder."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

with tempfile.TemporaryDirectory(prefix='brave args ') as temp:
    capture = Path(temp) / 'arguments.json'
    env = dict(os.environ, NEXO_ARG_CAPTURE=str(capture), NEXO_ENV_MARKER='preserved')
    arguments = ['', 'path with spaces/file.html', '--profile-directory=Profile 2',
                 'https://example.invalid/a%20b?x=1&y=(two)', 'line\nbreak', '--', '-psn_0_12345']
    process = subprocess.Popen([sys.argv[1], *arguments], env=env, cwd=temp)
    assert process.wait() == 0
    result = json.loads(capture.read_text())
    assert result['args'] == [sys.argv[2], *arguments], result['args']
    assert result['cwd'] == temp
    assert result['marker'] == 'preserved'
    assert result['pid'] == process.pid, 'The launcher must exec the browser'
    process = subprocess.run([sys.argv[1], '--exit=37'], env=env, cwd=temp)
    assert process.returncode == 37
print('Argument boundaries, empty/newline/URL/macOS arguments, environment, cwd, exec PID and exit status preserved')
