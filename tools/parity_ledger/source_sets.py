"""Immutable source-set declarations for the bootstrap ledger corpus."""

from dataclasses import dataclass

from .model import SourceRole, Tracking


@dataclass(frozen=True)
class SourceConfig:
    source_id: str
    system: str
    role: SourceRole
    path: str
    adapter: str
    tracking: Tracking
    declared_count: int | None


BOOTSTRAP_SOURCES = (
    SourceConfig(
        "core-todo",
        "core",
        SourceRole.INVENTORY,
        "docs/plans/2026-05-29-core-engine-substrate-todo.md",
        "core-checklist",
        Tracking.IGNORED_LOCAL,
        32,
    ),
    SourceConfig(
        "scheduler-roadmap",
        "scheduler",
        SourceRole.INVENTORY,
        "docs/plans/2026-05-28-foundational-scheduler-roadmap-todo.md",
        "scheduler-checklist",
        Tracking.IGNORED_LOCAL,
        17,
    ),
    SourceConfig(
        "miner-scan",
        "miner",
        SourceRole.INVENTORY,
        "docs/gap-scans/2026-07-02-disparity-scan-miner.md",
        "miner-scan",
        Tracking.IGNORED_LOCAL,
        139,
    ),
    SourceConfig(
        "miner-roadmap",
        "miner",
        SourceRole.ASSIGNMENT,
        "docs/plans/2026-07-02-miner-parity-roadmap.md",
        "miner-roadmap",
        Tracking.IGNORED_LOCAL,
        None,
    ),
    SourceConfig(
        "shell-scan",
        "shell",
        SourceRole.INVENTORY,
        "docs/gap-scans/2026-07-06-disparity-scan-shell-ui.md",
        "shell-scan",
        Tracking.IGNORED_LOCAL,
        89,
    ),
    SourceConfig(
        "shell-roadmap",
        "shell",
        SourceRole.ASSIGNMENT,
        "docs/plans/2026-07-06-shell-ui-parity-roadmap.md",
        "shell-roadmap",
        Tracking.IGNORED_LOCAL,
        None,
    ),
)

SOURCE_SETS = {"bootstrap": BOOTSTRAP_SOURCES}
