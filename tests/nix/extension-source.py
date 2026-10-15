"""Reject package boundary changes that need an explicit deployment review."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('extension_check', ROOT / 'nix/check-extension.py')
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)
IDENTITY = 'agmnljgahcijdoahbejifmdcdpamijin'


def copy_runtime(source):
    for name in checker.FILES:
        target = source / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / 'apps/nexo-capture-extension' / name, target)


class SourceBoundary(unittest.TestCase):
    def test_runtime(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory)
            copy_runtime(source)
            self.assertEqual(checker.check(source, IDENTITY), IDENTITY)

    def test_changed_boundaries(self):
        mutations = {
            'identity': lambda m: m.update(key='YWJj'),
            'api permissions': lambda m: m['permissions'].append('cookies'),
            'site access': lambda m: m.update(optional_host_permissions=['https://*/*']),
            'automatic access': lambda m: m.update(host_permissions=['https://platform.nexo.com/*']),
            'entry point': lambda m: m.update(options_page='private.json'),
        }
        for label, change in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as directory:
                source = Path(directory) / 'runtime'
                copy_runtime(source)
                path = source / 'manifest.json'
                manifest = json.loads(path.read_text())
                change(manifest)
                path.write_text(json.dumps(manifest))
                with self.assertRaises(ValueError):
                    checker.check(source, IDENTITY)

    def test_extra_file(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'runtime'
            copy_runtime(source)
            (source / 'private.json').write_text('{}')
            with self.assertRaisesRegex(ValueError, 'file inventory'):
                checker.check(source, IDENTITY)


if __name__ == '__main__':
    unittest.main()
