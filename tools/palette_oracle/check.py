"""Validate tracked native goldens, optionally against a fresh oracle run."""
import argparse
import hashlib
import json
from pathlib import Path
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--generated', type=Path)
args = parser.parse_args()
fixtures = Path(__file__).parent / 'fixtures'
manifest = json.loads((fixtures / 'provenance.json').read_text())
for name, expected in manifest['fixtures'].items():
    assert hashlib.sha256((fixtures / name).read_bytes()).hexdigest() == expected, name
    if args.generated:
        assert hashlib.sha256((args.generated / name).read_bytes()).hexdigest() == expected, name
print(f"{len(manifest['fixtures'])} native fixture hashes match" + (' fresh original-byte execution' if args.generated else ' provenance'))
