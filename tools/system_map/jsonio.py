"""Strict JSON, portable paths, hashing, and atomic deterministic writes."""

from __future__ import annotations

import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import secrets

from .model import Diagnostic, SystemMapError


def _fail(code: str, message: str, *, field: str = "", path: str = "") -> None:
    raise SystemMapError(
        [Diagnostic("error", code, message, field=field, path=path)]
    )


def _pairs_no_duplicates(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            _fail("DUPLICATE_JSON_KEY", f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def _reject_constant(value: str) -> None:
    _fail("NON_FINITE_JSON", f"non-finite JSON number {value}")


def _validate_value(value: object, location: str = "$") -> None:
    if value is None or isinstance(value, (bool, int, str)):
        if isinstance(value, str) and any(
            0xD800 <= ord(char) <= 0xDFFF for char in value
        ):
            _fail("INVALID_UNICODE", f"unpaired surrogate at {location}")
        return
    if isinstance(value, float):
        if not math.isfinite(value):
            _fail("NON_FINITE_JSON", f"non-finite number at {location}")
        return
    if isinstance(value, list):
        for index, item in enumerate(value):
            _validate_value(item, f"{location}[{index}]")
        return
    if isinstance(value, dict):
        for key, item in value.items():
            if not isinstance(key, str):
                _fail("NON_STRING_JSON_KEY", f"non-string key at {location}")
            _validate_value(key, f"{location}.<key>")
            _validate_value(item, f"{location}.{key}")
        return
    _fail(
        "UNSUPPORTED_JSON_VALUE",
        f"unsupported {type(value).__name__} at {location}",
    )


def load_json_strict(path: Path) -> object:
    """Load exactly one UTF-8 JSON value, rejecting duplicate keys/extensions."""

    try:
        data = path.read_bytes().decode("utf-8", errors="strict")
    except FileNotFoundError as exc:
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "MISSING_CANONICAL_FILE",
                    f"required file does not exist: {path.as_posix()}",
                    path=path.as_posix(),
                )
            ]
        ) from exc
    except UnicodeDecodeError as exc:
        _fail(
            "INVALID_UTF8",
            f"invalid UTF-8: {exc}",
            path=path.as_posix(),
        )
    try:
        value = json.loads(
            data,
            object_pairs_hook=_pairs_no_duplicates,
            parse_constant=_reject_constant,
        )
    except SystemMapError:
        raise
    except (json.JSONDecodeError, RecursionError) as exc:
        _fail("INVALID_JSON", f"invalid JSON: {exc}", path=path.as_posix())
    _validate_value(value)
    return value


def canonical_json_bytes(value: object) -> bytes:
    """Return the deterministic persisted JSON representation."""

    _validate_value(value)
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            allow_nan=False,
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
        + b"\n"
    )


def pretty_json(value: object) -> str:
    """Return deterministic human-readable JSON."""

    _validate_value(value)
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            allow_nan=False,
            sort_keys=True,
            indent=2,
        )
        + "\n"
    )


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


_DEVICE_STEMS = {"CON", "PRN", "AUX", "NUL"}
_DEVICE_STEMS.update(f"COM{number}" for number in range(1, 10))
_DEVICE_STEMS.update(f"LPT{number}" for number in range(1, 10))


def validate_relative_path(value: object, *, field: str = "path") -> str | None:
    """Return a portable repo-relative path, or ``None`` when invalid."""

    if not isinstance(value, str) or not value:
        return None
    if (
        "\\" in value
        or PurePosixPath(value).is_absolute()
        or PureWindowsPath(value).is_absolute()
        or PureWindowsPath(value).drive
        or ":" in value
    ):
        return None
    for part in value.split("/"):
        if part in {"", ".", ".."}:
            return None
        if len(part) > 255 or part.endswith((" ", ".")):
            return None
        if any(ord(char) < 32 or ord(char) == 127 for char in part):
            return None
        if part.split(".", 1)[0].rstrip(" .").upper() in _DEVICE_STEMS:
            return None
    return value


def atomic_write_bytes(path: Path, payload: bytes) -> None:
    """Flush and atomically replace a destination using a sibling temp file."""

    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{secrets.token_hex(8)}.tmp")
    descriptor: int | None = None
    try:
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_BINARY", 0)
        descriptor = os.open(temporary, flags, 0o600)
        view = memoryview(payload)
        while view:
            written = os.write(descriptor, view)
            if written <= 0:
                raise OSError("short write")
            view = view[written:]
        os.fsync(descriptor)
        os.close(descriptor)
        descriptor = None
        os.replace(temporary, path)
    except OSError as exc:
        raise SystemMapError(
            [
                Diagnostic(
                    "error",
                    "OUTPUT_IO_FAILED",
                    str(exc),
                    path=path.as_posix(),
                )
            ],
            exit_code=3,
        ) from exc
    finally:
        if descriptor is not None:
            os.close(descriptor)
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
