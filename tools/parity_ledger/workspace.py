"""Read-only repository, path, source-fingerprint, and Git inspection helpers."""

from __future__ import annotations

from pathlib import Path
import re
import subprocess
from typing import Callable

from .errors import Diagnostic, ExitCode, FailureCode, LedgerError
from .jsonio import sha256_file, validate_relative_path
from .model import SourceLockDocument, SourceState, Tracking


Runner = Callable[..., subprocess.CompletedProcess[str]]

_ATTRIBUTE_PATTERN = r"#\s*\[(?:[^\[\]]|\[[^\[\]]*\])*\]"
_TEST_DECL_RE = re.compile(
    rf"(?m)^[ \t]*(?P<attrs>(?:{_ATTRIBUTE_PATTERN}[ \t\r\n]*)+)"
    r"(?:pub(?:\s*\([^\r\n)]*\))?\s+)?(?:async\s+)?"
    r"fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b"
)
_TEST_ATTRIBUTE_RE = re.compile(
    r"#\s*\[\s*(?:[A-Za-z_][A-Za-z0-9_]*::)?test\s*\]"
)
_CFG_ATTRIBUTE_RE = re.compile(r"#\s*\[\s*cfg(?:_attr)?\b")
_MACRO_DEFINITION_RE = re.compile(
    r"\bmacro_rules!\s*[A-Za-z_][A-Za-z0-9_]*\s*(?P<delimiter>[\(\[\{])"
)
_MACRO_ITEM_RE = re.compile(
    r"\bmacro\s+[A-Za-z_][A-Za-z0-9_]*\s*(?P<delimiter>[\(\[\{])"
)
_MACRO_INVOCATION_RE = re.compile(
    r"\b[A-Za-z_][A-Za-z0-9_]*!\s*(?P<delimiter>[\(\[\{])"
)


def _blank_non_newlines(chars: list[str], start: int, end: int) -> None:
    for index in range(start, end):
        if chars[index] not in {"\r", "\n"}:
            chars[index] = " "


def _rust_code_view(text: str) -> str:
    """Blank comments and string literals while preserving declaration line structure."""

    chars = list(text)
    index = 0
    while index < len(text):
        if text.startswith("//", index):
            end = text.find("\n", index + 2)
            end = len(text) if end < 0 else end
            _blank_non_newlines(chars, index, end)
            index = end
            continue
        if text.startswith("/*", index):
            depth = 1
            end = index + 2
            while end < len(text) and depth:
                if text.startswith("/*", end):
                    depth += 1
                    end += 2
                elif text.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            _blank_non_newlines(chars, index, end)
            index = end
            continue

        raw_prefix_end: int | None = None
        for prefix in ("br", "cr", "r"):
            if text.startswith(prefix, index) and (
                index == 0 or not (text[index - 1].isalnum() or text[index - 1] == "_")
            ):
                candidate = index + len(prefix)
                while candidate < len(text) and text[candidate] == "#":
                    candidate += 1
                if candidate < len(text) and text[candidate] == '"':
                    raw_prefix_end = candidate
                    break
        if raw_prefix_end is not None:
            hashes = text[index:raw_prefix_end].count("#")
            terminator = '"' + ("#" * hashes)
            end = text.find(terminator, raw_prefix_end + 1)
            end = len(text) if end < 0 else end + len(terminator)
            _blank_non_newlines(chars, index, end)
            index = end
            continue

        string_start = index
        if text[index] in {"b", "c"} and index + 1 < len(text) and text[index + 1] == '"':
            quote = index + 1
        elif text[index] == '"':
            quote = index
        else:
            index += 1
            continue
        end = quote + 1
        while end < len(text):
            if text[end] == "\\":
                end = min(end + 2, len(text))
            elif text[end] == '"':
                end += 1
                break
            else:
                end += 1
        _blank_non_newlines(chars, string_start, end)
        index = end
    return "".join(chars)


def _balanced_token_range(code: str, start: int) -> tuple[int, int] | None:
    closing = {"(": ")", "[": "]", "{": "}"}
    stack = [closing[code[start]]]
    index = start + 1
    while index < len(code):
        char = code[index]
        if char in closing:
            stack.append(closing[char])
        elif char in closing.values():
            if not stack or char != stack[-1]:
                return None
            stack.pop()
            if not stack:
                return start, index + 1
        index += 1
    return None


def _macro_token_ranges(code: str) -> tuple[tuple[int, int], ...]:
    ranges: set[tuple[int, int]] = set()
    for pattern in (_MACRO_DEFINITION_RE, _MACRO_ITEM_RE, _MACRO_INVOCATION_RE):
        for match in pattern.finditer(code):
            token_range = _balanced_token_range(code, match.start("delimiter"))
            if token_range is not None:
                ranges.add(token_range)
    return tuple(sorted(ranges))


def find_repo_root(start: Path) -> Path:
    current = start.resolve()
    if current.is_file():
        current = current.parent
    for candidate in (current, *current.parents):
        if (candidate / "Cargo.toml").is_file() and (candidate / ".git").exists():
            return candidate
    raise LedgerError(
        ExitCode.WORKSPACE_FAILED,
        [Diagnostic(FailureCode.GIT_FAILED.value, message="repository root not found", fatal=True)],
    )


def resolve_repo_path(repo: Path, relative: str) -> Path:
    validate_relative_path(relative)
    root = repo.resolve()
    candidate = (root / Path(*relative.split("/"))).resolve(strict=False)
    try:
        candidate.relative_to(root)
    except ValueError:
        raise LedgerError(
            ExitCode.WORKSPACE_FAILED,
            [
                Diagnostic(
                    FailureCode.UNSAFE_PATH.value,
                    source_path=relative,
                    message="resolved path escapes repository",
                    fatal=True,
                )
            ],
        )
    return candidate


def _invoke_git(repo: Path, args: tuple[str, ...], runner: Runner) -> subprocess.CompletedProcess[str]:
    try:
        return runner(
            ("git", "-C", str(repo), *args),
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="strict",
        )
    except (OSError, UnicodeError) as exc:
        raise LedgerError(
            ExitCode.WORKSPACE_FAILED,
            [Diagnostic(FailureCode.GIT_FAILED.value, message=str(exc), fatal=True)],
        ) from exc


def run_git(repo: Path, args: tuple[str, ...], *, runner: Runner = subprocess.run) -> str:
    completed = _invoke_git(repo, args, runner)
    if completed.returncode != 0:
        raise LedgerError(
            ExitCode.WORKSPACE_FAILED,
            [
                Diagnostic(
                    FailureCode.GIT_FAILED.value,
                    message=f"git {' '.join(args)} failed ({completed.returncode}): {completed.stderr.strip()}",
                    fatal=True,
                )
            ],
        )
    return completed.stdout


def is_ancestor(
    repo: Path,
    commit: str,
    *,
    head: str = "HEAD",
    runner: Runner = subprocess.run,
) -> bool:
    """Return Git's true/non-ancestor states; reserve errors for return codes >=2."""

    completed = _invoke_git(repo, ("merge-base", "--is-ancestor", commit, head), runner)
    if completed.returncode == 0:
        return True
    if completed.returncode == 1:
        return False
    raise LedgerError(
        ExitCode.WORKSPACE_FAILED,
        [
            Diagnostic(
                FailureCode.GIT_FAILED.value,
                message=f"git ancestry check failed ({completed.returncode}): {completed.stderr.strip()}",
                fatal=True,
            )
        ],
    )


def source_states(
    repo: Path,
    document: SourceLockDocument,
    *,
    mode: str = "default",
) -> dict[str, SourceState]:
    if mode not in {"default", "ci", "require"}:
        raise ValueError(f"unknown source mode {mode!r}")
    result: dict[str, SourceState] = {}
    for source in document.sources:
        if mode == "ci" and source.tracking is Tracking.IGNORED_LOCAL:
            result[source.source_id] = SourceState.UNAVAILABLE
            continue
        path = resolve_repo_path(repo, source.path)
        if not path.is_file():
            state = SourceState.UNAVAILABLE
        elif sha256_file(path) != source.sha256:
            state = SourceState.STALE
        else:
            state = SourceState.CURRENT
        result[source.source_id] = state
    if mode == "require":
        failures = [
            Diagnostic(
                FailureCode.SOURCE_UNAVAILABLE.value
                if state is SourceState.UNAVAILABLE
                else FailureCode.SOURCE_STALE.value,
                source_path=next(item.path for item in document.sources if item.source_id == source_id),
                record_id=source_id,
                message=f"required source is {state.value}",
                fatal=True,
            )
            for source_id, state in result.items()
            if state is not SourceState.CURRENT
        ]
        if failures:
            raise LedgerError(ExitCode.REQUIRED_SOURCE_FAILED, failures)
    return result


def declared_test_names(text: str) -> tuple[str, ...]:
    """Extract direct, unconditional Rust test functions outside macro token trees."""

    code = _rust_code_view(text)
    macro_ranges = _macro_token_ranges(code)
    names = {
        match.group("name")
        for match in _TEST_DECL_RE.finditer(code)
        if _TEST_ATTRIBUTE_RE.search(match.group("attrs"))
        and not _CFG_ATTRIBUTE_RE.search(match.group("attrs"))
        and not any(start <= match.start() < end for start, end in macro_ranges)
    }
    return tuple(sorted(names))


def current_test_declared(repo: Path, path: str, test_name: str) -> bool:

    candidate = resolve_repo_path(repo, path)
    if not candidate.is_file():
        return False
    try:
        text = candidate.read_text(encoding="utf-8", errors="strict")
    except (OSError, UnicodeError):
        return False
    return test_name in declared_test_names(text)
