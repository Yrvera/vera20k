"""Chunk markdown and INI files into cited line ranges."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re


HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*$")
MAX_PLAIN_CHUNK_LINES = 120

# `INIClass__LoadFromStraw @ 0x00525A60` trims the raw line of every character
# at or below 0x20 (`strtrim @ 0x00727CF0`, called at 0x00525DB8) before any
# header test. Mirrors `trim_ascii_controls` in `src/rules/ini_parser.rs`.
_NATIVE_TRIM_CHARS = "".join(chr(code) for code in range(0x21))


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


def ini_section_name(line: str) -> str | None:
    """Return the section name this line opens, or None if it opens none.

    Retail rule, from `INIClass__LoadFromStraw @ 0x00525A60`: after the
    whole-line trim, a header is a line whose first character is `[` (`CMP
    byte ptr [ESP + 0x78], 0x5b` @ 0x00525DC1) *and* that contains a `]`
    anywhere (`strchr(line, ']')` via `PUSH 0x5d` @ 0x00525DCC calling
    `CRT__strchr @ 0x007CAF30`). The loader then writes a NUL over that first
    `]` (`MOV byte ptr [EAX],0x0` @ 0x00525DDB), so the name is the text
    between `[` and the *first* `]` and everything after it is discarded.
    There is no `;`-comment handling on the header path at all — a trailing
    comment, or a second `[...]`, is simply thrown away with the rest of the
    line. All five `[`-detection sites in the function (0x00525B2B,
    0x00525C0B, 0x00525C95, 0x00525DC1, 0x00525EA5) pair the `[` test with the
    same `strchr` for `]`, so `[Foo` with no bracket is never a header.

    Retail INIs rely on this: 61 of rulesmd.ini's 1478 headers and 154 of
    artmd.ini's 1582 carry trailing text (e.g. `[GAFWLL];temp wall for
    yuri[YAWALL]`). A stricter test drops those headers and misfiles their
    keys into the preceding section.

    Kept rule-for-rule identical to VERA's own reader,
    `IniFile::parse_line` in `src/rules/ini_parser.rs`.
    """
    trimmed = line.strip(_NATIVE_TRIM_CHARS)
    if not trimmed.startswith("["):
        return None
    end = trimmed.find("]")
    if end < 0:
        return None
    return trimmed[1:end]


def chunk_ini(lines: list[str]) -> list[Chunk]:
    chunks: list[Chunk] = []
    start = 1
    heading = "INI"

    for idx, line in enumerate(lines, start=1):
        name = ini_section_name(line)
        if name is None:
            continue

        if idx > start:
            _append_chunk(chunks, heading, start, idx - 1, lines[start - 1 : idx - 1])

        heading = f"INI [{name}]"
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
