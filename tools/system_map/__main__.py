"""Reference checker for docs/system-map.

    python -m tools.system_map check

Reads registry.v2.json, topology.v2.json and mechanisms.v1.json, then reports
every repository path they cite that no longer exists and every GSI/loop/block
id they reference that is not defined. Exit code 2 on any error. There is no
importer, freshness model, renderer or source lock any more; the JSON files are
hand-maintained data and this is the only tooling around them.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAP = ROOT / "docs" / "system-map"
FILES = ["registry.v2.json", "topology.v2.json", "mechanisms.v1.json"]
PATH_RE = re.compile(r"^(src|docs|tools|tests|assets)/[^\s:]+")
ID_RE = re.compile(r"^(GSI-\d\d\.\d\d|LOOP-\d{3}-[A-Z0-9-]+|MBLK-\d{3}-[A-Z0-9-]+)$")


def strings(obj, where=""):
    if isinstance(obj, dict):
        for k, v in obj.items():
            yield from strings(v, f"{where}.{k}" if where else k)
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            yield from strings(v, f"{where}[{i}]")
    elif isinstance(obj, str):
        yield where, obj


def main(argv: list[str]) -> int:
    if not argv or argv[0] != "check":
        print(__doc__)
        return 0
    data = {}
    for name in FILES:
        try:
            data[name] = json.loads((MAP / name).read_text(encoding="utf-8"))
        except (OSError, ValueError) as exc:
            print(f"error: {name}: {exc}")
            return 2
    defined = set(data["registry.v2.json"].get("systems", {}))
    topo = data["topology.v2.json"]
    defined |= set(topo.get("systems", {})) | set(topo.get("loops", {}))
    blocks = data["mechanisms.v1.json"].get("blocks", {})
    defined |= set(blocks) if isinstance(blocks, dict) else {x.get("id") for x in blocks if isinstance(x, dict)}
    errors = 0
    for name, doc in data.items():
        for where, value in strings(doc):
            m = PATH_RE.match(value)
            if m and not (ROOT / m.group(0)).exists():
                print(f"error: {name}: {where}: missing path {m.group(0)}")
                errors += 1
            elif ID_RE.match(value) and value not in defined:
                print(f"error: {name}: {where}: undefined id {value}")
                errors += 1
    print(f"{'FAIL' if errors else 'OK'}: {errors} error(s) across {len(FILES)} files")
    return 2 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
