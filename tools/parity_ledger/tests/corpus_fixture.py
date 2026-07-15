"""Tracked-only synthetic repository builder for corpus and CLI tests."""

from pathlib import Path

from tools.parity_ledger.source_sets import BOOTSTRAP_SOURCES
from tools.parity_ledger.tests.test_import_miner import _roadmap as miner_roadmap
from tools.parity_ledger.tests.test_import_miner import _scan as miner_scan
from tools.parity_ledger.tests.test_import_shell import _roadmap as shell_roadmap
from tools.parity_ledger.tests.test_import_shell import _scan as shell_scan


def make_repo(root: Path) -> Path:
    (root / ".git").mkdir()
    (root / "Cargo.toml").write_text('[package]\nname="fixture"\nversion="0.0.0"\n', encoding="utf-8")
    fixture_dir = Path(__file__).parent / "fixtures"
    payloads = {
        "core-todo": (fixture_dir / "core-checklist.md").read_bytes(),
        "scheduler-roadmap": (fixture_dir / "scheduler-checklist.md").read_bytes(),
        "miner-scan": miner_scan(),
        "miner-roadmap": miner_roadmap(),
        "shell-scan": shell_scan(),
        "shell-roadmap": shell_roadmap(),
    }
    for config in BOOTSTRAP_SOURCES:
        path = root / Path(*config.path.split("/"))
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payloads[config.source_id])
    return root
