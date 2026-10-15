"""Validate the exact public source-only runtime passed to the hermetic build."""
import base64
import hashlib
import json
from pathlib import Path
import sys

FILES = {'manifest.json', 'options.html', 'options.css', 'src/options.js', 'src/background.js', 'src/page.js'}


def check(source, expected_id):
    manifest = json.loads((source / 'manifest.json').read_text())
    digest = hashlib.sha256(base64.b64decode(manifest['key'], validate=True)).hexdigest()[:32]
    identity = ''.join(chr(97 + int(c, 16)) for c in digest)
    if identity != expected_id:
        raise ValueError('Extension identity changed; review native pairing before deployment')
    if manifest['manifest_version'] != 3:
        raise ValueError('Expected Manifest V3')
    if set(manifest['permissions']) != {'scripting', 'storage', 'nativeMessaging'}:
        raise ValueError('Review changed API permissions')
    if manifest['optional_host_permissions'] != ['https://platform.nexo.com/*']:
        raise ValueError('Review changed optional site access')
    if manifest.get('host_permissions') or manifest.get('externally_connectable'):
        raise ValueError('Unexpected automatic host or external access')
    for name in [manifest['background']['service_worker'], manifest['options_page'], manifest['action']['default_popup']]:
        if name not in FILES or not (source / name).is_file():
            raise ValueError('Unpackaged extension entry point')
    actual = {str(path.relative_to(source)) for path in source.rglob('*') if path.is_file()}
    if actual != FILES:
        raise ValueError('Unexpected runtime file inventory')
    return identity


if __name__ == '__main__':
    print('Validated source-only extension:', check(Path(sys.argv[1]), sys.argv[2]))
