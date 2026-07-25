"""Metadata and evidence-term extraction for research files."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import hashlib
import os
import re


ADDRESS_RE = re.compile(r"\b0x[0-9A-Fa-f]{4,}\b")
RUST_PATH_RE = re.compile(r"\bsrc/[A-Za-z0-9_./-]+\.rs\b")
MARKDOWN_LINK_RE = re.compile(r"\[[^\]]+\]\(((?:https?://|mailto:)[^)]+|[^)#]+\.md)(?:#[^)]+)?\)")
INI_ASSIGNMENT_RE = re.compile(r"^\s*([A-Za-z][A-Za-z0-9_.$-]{1,64})\s*=", re.MULTILINE)
BACKTICK_KEY_RE = re.compile(r"`([A-Za-z][A-Za-z0-9_.$-]{2,64})=`")
SYMBOL_RE = re.compile(
    r"\b(?:[A-Z][A-Za-z0-9]+Class__|[A-Z][A-Za-z0-9]+__|[A-Z][A-Za-z0-9]+::)"
    r"[A-Za-z0-9_~]+|\b[A-Za-z][A-Za-z0-9]+_[A-Za-z0-9_]{3,}\b"
)
REPORT_SYMBOL_RE = re.compile(r".*_(?:GHIDRA_REPORT|TRACE|SYSTEM_MODEL_SYNTHESIS|VERIFY_DOC_AMENDMENTS|FOLLOWUP|INVESTIGATION|PLAN)$")
PSEUDO_SYMBOLS = {"ADDRESS_MAP", "AUDIT_LOG", "LABEL_AUDIT_LOG"}

INDEX_EXTENSIONS = {".md", ".ini", ".yaml", ".yml", ".csv"}
SKIP_DIRS = {".git", "target", ".cache", "vendor-eval"}


@dataclass(frozen=True)
class DocumentMetadata:
    title: str
    system: str
    subsystem: str
    source_kind: str
    status: str
    modified_time: float
    checksum: str


@dataclass(frozen=True)
class EvidenceTerms:
    addresses: tuple[str, ...]
    symbols: tuple[str, ...]
    ini_keys: tuple[str, ...]
    rust_paths: tuple[str, ...]
    links: tuple[str, ...]


def iter_indexable_files(roots: list[Path]) -> list[Path]:
    files: list[Path] = []
    for root in roots:
        if not root.exists():
            continue
        if root.is_file():
            if root.suffix.lower() in INDEX_EXTENSIONS:
                files.append(root)
            continue
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = [name for name in dirnames if name not in SKIP_DIRS]
            for filename in filenames:
                path = Path(dirpath) / filename
                if path.suffix.lower() in INDEX_EXTENSIONS:
                    files.append(path)
    return sorted(files)


def document_metadata(path: Path, workspace: Path) -> DocumentMetadata:
    rel = path.relative_to(workspace).as_posix()
    text = path.read_text(encoding="utf-8", errors="replace")
    title = extract_title(path, text)
    system, subsystem = infer_system(rel)
    source_kind = infer_source_kind(path.name)
    status = infer_status(path.name, source_kind)
    stat = path.stat()
    return DocumentMetadata(title, system, subsystem, source_kind, status, stat.st_mtime, checksum(text))


def extract_terms(text: str, suffix: str = "") -> EvidenceTerms:
    addresses = tuple(sorted(set(match.group(0).lower() for match in ADDRESS_RE.finditer(text))))
    rust_paths = tuple(sorted(set(match.group(0).replace("\\", "/") for match in RUST_PATH_RE.finditer(text))))

    ini_keys = set(match.group(1) for match in INI_ASSIGNMENT_RE.finditer(text)) if suffix.lower() == ".ini" else set()
    ini_keys.update(match.group(1) for match in BACKTICK_KEY_RE.finditer(text))

    symbols = {symbol for symbol in (match.group(0) for match in SYMBOL_RE.finditer(text)) if is_real_symbol(symbol)}
    links = tuple(sorted(set(match.group(1).replace("\\", "/") for match in MARKDOWN_LINK_RE.finditer(text))))

    return EvidenceTerms(
        addresses=addresses,
        symbols=tuple(sorted(symbols)),
        ini_keys=tuple(sorted(ini_keys)),
        rust_paths=rust_paths,
        links=links,
    )


def is_real_symbol(symbol: str) -> bool:
    if symbol in PSEUDO_SYMBOLS:
        return False
    if REPORT_SYMBOL_RE.fullmatch(symbol):
        return False
    return True


def extract_title(path: Path, text: str) -> str:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("# "):
            return stripped[2:].strip()
    return path.stem


def infer_system(relpath: str) -> tuple[str, str]:
    parts = relpath.split("/")
    if parts[:2] == ["docs", "research"]:
        if len(parts) >= 4 and parts[2] == "bridges":
            return "bridges", parts[3]
        if len(parts) >= 4:
            return parts[2], parts[3]
        if len(parts) >= 3:
            return infer_root_system(parts[2]), "root"
    if parts[:2] == ["docs", "plans"]:
        return "plans", "plans"
    if parts and parts[0] == "ini":
        return "ini", "rules"
    return "unknown", "unknown"


def infer_root_system(filename: str) -> str:
    upper = filename.upper()
    if "BRIDGE" in upper or "CABHUT" in upper:
        return "bridges"
    if "PATH" in upper or "ZONE" in upper:
        return "pathfinding"
    if "MINER" in upper or "HARV" in upper or "REFINERY" in upper:
        return "miner"
    if "SKIRMISH" in upper:
        return "skirmish-ui"
    if "COMBAT" in upper or "WEAPON" in upper or "WARHEAD" in upper:
        return "combat"
    if "RENDER" in upper or "DRAW" in upper or "PIXEL" in upper:
        return "rendering"
    return "root"


def infer_source_kind(filename: str) -> str:
    upper = filename.upper()
    if upper.endswith(".INI"):
        return "ini"
    if "GHIDRA_REPORT" in upper:
        return "ghidra"
    if "TRACE" in upper:
        return "trace"
    if "IMPLEMENTATION_CONTRACT" in upper:
        return "contract"
    if "SYSTEM_MODEL_SYNTHESIS" in upper or upper.endswith("_SYSTEM.MD"):
        return "synthesis"
    if "PLAN" in upper:
        return "plan"
    if "AUDIT" in upper:
        return "audit"
    return "unknown"


def infer_status(filename: str, source_kind: str) -> str:
    upper = filename.upper()
    if "SUPERSEDED" in upper or "STALE" in upper:
        return "stale"
    if "VERIFY" in upper or "VERIFICATION" in upper or source_kind in {"ghidra", "trace", "contract", "ini"}:
        return "verified"
    if source_kind == "synthesis":
        return "synthesis"
    if source_kind == "plan":
        return "plan"
    return "unknown"


def checksum(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8", errors="replace")).hexdigest()
