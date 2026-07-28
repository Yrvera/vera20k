"""Strict Markdown boundary, identity, ID, and path extraction primitives."""

from __future__ import annotations

import re
from typing import Iterable

from ..errors import Diagnostic, ExitCode, FailureCode, LedgerError
from ..jsonio import sha256_bytes


H2_RE = re.compile(r"^## (?!#)(?P<title>.+?)\s*$", re.MULTILINE)
H3_RE = re.compile(r"^### (?!#)(?P<title>.+?)\s*$", re.MULTILINE)
CHECK_RE = re.compile(r"^- \[(?P<mark>[ xX])\] (?P<lead>.+)$")
ID_RE = re.compile(
    r"(?<![A-Za-z0-9_])(?P<family>[GHMLS])(?P<num>[1-9]\d*)(?![A-Za-z0-9_])"
)
RANGE_RE = re.compile(
    r"(?<![A-Za-z0-9_])([GHMLS])([1-9]\d*)\s*(?:-|–|—|\.\.)\s*\1([1-9]\d*)"
    r"(?![A-Za-z0-9_])"
)
RUST_PATH_RE = re.compile(r"\b(?:src|tests)/[A-Za-z0-9_./-]+\.rs\b")
DONE_SUFFIX_RE = re.compile(r"\s+(?:—\s*)?\*\*done\b", re.IGNORECASE)
_FENCE_RE = re.compile(r"^\s*(```|~~~)")
_NESTED_LIST_RE = re.compile(r"^\s+(?:[-*+] |\d+[.)] )")


def malformed(message: str, *, source_path: str = "", record_id: str = "") -> None:
    raise LedgerError(
        ExitCode.VALIDATION_FAILED,
        [
            Diagnostic(
                FailureCode.SOURCE_MALFORMED.value,
                source_path=source_path,
                record_id=record_id,
                message=message,
                fatal=True,
            )
        ],
    )


def strict_text(raw: bytes, *, source_path: str = "") -> tuple[str, str]:
    digest = sha256_bytes(raw)
    try:
        return raw.decode("utf-8", errors="strict"), digest
    except UnicodeDecodeError as exc:
        malformed(f"invalid UTF-8: {exc}", source_path=source_path)


def without_fenced_code(text: str) -> str:
    """Blank fenced content while preserving physical line positions."""

    output: list[str] = []
    fence: str | None = None
    for line in text.splitlines(keepends=True):
        match = _FENCE_RE.match(line)
        if match:
            marker = match.group(1)
            if fence is None:
                fence = marker[0]
            elif marker[0] == fence:
                fence = None
            output.append("\n" if line.endswith("\n") else "")
        elif fence is not None:
            output.append("\n" if line.endswith("\n") else "")
        else:
            output.append(line)
    if fence is not None:
        malformed("unclosed Markdown code fence")
    return "".join(output)


def bounded_section(text: str, start_heading: str, end_heading: str | None) -> str:
    clean = without_fenced_code(text)
    headings = list(H2_RE.finditer(clean))
    start_matches = [match for match in headings if match.group("title") == start_heading]
    if len(start_matches) != 1:
        malformed(f"expected exactly one H2 heading {start_heading!r}")
    start = start_matches[0].end()
    if end_heading is None:
        return clean[start:]
    end_matches = [match for match in headings if match.group("title") == end_heading]
    if len(end_matches) != 1:
        malformed(f"expected exactly one H2 heading {end_heading!r}")
    end = end_matches[0].start()
    if end <= start:
        malformed(f"heading {end_heading!r} does not follow {start_heading!r}")
    return clean[start:end]


def fold_markdown(text: str) -> str:
    return re.sub(r"[ \t\r\n]+", " ", text).strip()


def slugify(value: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", value.casefold()).strip("-")
    return slug or "item"


def extract_rust_paths(text: str) -> tuple[str, ...]:
    return tuple(sorted(set(RUST_PATH_RE.findall(without_fenced_code(text)))))


def expand_ids(text: str) -> tuple[str, ...]:
    """Expand same-family ranges, then extract remaining scalar IDs."""

    found: set[str] = set()
    masked = list(text)
    for match in RANGE_RE.finditer(text):
        family, start_text, end_text = match.groups()
        start = int(start_text)
        end = int(end_text)
        if end < start or end - start > 10000:
            malformed(f"invalid or excessive ID range {match.group(0)!r}")
        found.update(f"{family}{number}" for number in range(start, end + 1))
        masked[match.start():match.end()] = " " * (match.end() - match.start())
    for match in ID_RE.finditer("".join(masked)):
        found.add(f"{match.group('family')}{match.group('num')}")
    return tuple(sorted(found, key=id_sort_key))


def id_sort_key(value: str) -> tuple[str, int]:
    match = re.fullmatch(r"([A-Z]+)([1-9]\d*)", value)
    if not match:
        return (value, 0)
    return (match.group(1), int(match.group(2)))


def checklist_records(section: str) -> list[tuple[tuple[str, ...], str]]:
    """Return heading path and status-free title for column-zero checkboxes."""

    heading_path: list[str] = []
    records: list[tuple[tuple[str, ...], str]] = []
    lines = section.splitlines()
    index = 0
    while index < len(lines):
        line = lines[index]
        h3 = re.match(r"^### (?!#)(.+?)\s*$", line)
        if h3:
            heading_path = [fold_markdown(h3.group(1))]
            index += 1
            continue
        check = CHECK_RE.match(line)
        if not check:
            index += 1
            continue
        parts = [check.group("lead")]
        next_index = index + 1
        while next_index < len(lines):
            continuation = lines[next_index]
            if not continuation.strip() or continuation.startswith(("## ", "### ", "- [")):
                break
            if _NESTED_LIST_RE.match(continuation):
                break
            if continuation[:1].isspace():
                parts.append(continuation.strip())
                next_index += 1
                continue
            break
        title = fold_markdown(" ".join(parts))
        done = DONE_SUFFIX_RE.search(title)
        if done:
            title = title[: done.start()].rstrip()
        if not title:
            malformed("checklist record has an empty title")
        records.append((tuple(heading_path), title))
        index = max(index + 1, next_index)
    return records


def exact_id_set(family: str, start: int, end: int) -> set[str]:
    return {f"{family}{number}" for number in range(start, end + 1)}


def ensure_exact_ids(actual: Iterable[str], expected: set[str], *, label: str) -> None:
    actual_set = set(actual)
    if actual_set != expected:
        malformed(
            f"{label} IDs differ; missing={sorted(expected - actual_set, key=id_sort_key)}, "
            f"extra={sorted(actual_set - expected, key=id_sort_key)}"
        )
