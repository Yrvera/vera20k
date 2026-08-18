"""Chunk markdown and INI files into cited line ranges."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re


HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*$")
INI_SECTION_RE = re.compile(r"^\s*\[([^\]]+)\]\s*$")
MAX_PLAIN_CHUNK_LINES = 120


@dataclass(frozen=True)
class Chunk:
    heading_path: str
    start_line: int
    end_line: int
    text: str


def chunk_file(path: Path) -> list[Chunk]:
    suffix = path.suffix.lower()
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    if not lines:
        return []

    if suffix == ".md":
        return chunk_markdown(lines)
    if suffix == ".ini":
        return chunk_ini(lines)
    return chunk_plain(lines)


def chunk_markdown(lines: list[str]) -> list[Chunk]:
    chunks: list[Chunk] = []
    headings: list[str] = []
    start = 1
    current_heading = "Document"

    for idx, line in enumerate(lines, start=1):
        match = HEADING_RE.match(line)
        if not match:
            continue

        if idx > start:
            _append_chunk(chunks, current_heading, start, idx - 1, lines[start - 1 : idx - 1])

        level = len(match.group(1))
        title = match.group(2).strip()
        headings = headings[: level - 1]
        headings.append(title)
        current_heading = " > ".join(headings)
        start = idx

    if start <= len(lines):
        _append_chunk(chunks, current_heading, start, len(lines), lines[start - 1 :])

    return chunks


def chunk_ini(lines: list[str]) -> list[Chunk]:
    chunks: list[Chunk] = []
    start = 1
    heading = "INI"

    for idx, line in enumerate(lines, start=1):
        match = INI_SECTION_RE.match(line)
        if not match:
            continue

        if idx > start:
            _append_chunk(chunks, heading, start, idx - 1, lines[start - 1 : idx - 1])

        heading = f"INI [{match.group(1).strip()}]"
        start = idx

    if start <= len(lines):
        _append_chunk(chunks, heading, start, len(lines), lines[start - 1 :])

    return chunks


def chunk_plain(lines: list[str]) -> list[Chunk]:
    chunks: list[Chunk] = []
    for offset in range(0, len(lines), MAX_PLAIN_CHUNK_LINES):
        start = offset + 1
        end = min(offset + MAX_PLAIN_CHUNK_LINES, len(lines))
        _append_chunk(chunks, "Document", start, end, lines[offset:end])
    return chunks


def _append_chunk(chunks: list[Chunk], heading: str, start: int, end: int, chunk_lines: list[str]) -> None:
    text = "\n".join(chunk_lines).strip()
    if text:
        chunks.append(Chunk(heading, start, end, text))
