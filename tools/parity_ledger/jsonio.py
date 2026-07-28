"""Strict JSON, canonical bytes, safe paths, hashing, and atomic writes."""

from __future__ import annotations

import hashlib
import json
import math
import os
from dataclasses import dataclass
from pathlib import Path, PurePosixPath, PureWindowsPath
import secrets
from typing import NoReturn

from .errors import Diagnostic, ExitCode, FailureCode, LedgerError


def _validation_error(code: FailureCode, message: str, *, field: str = "") -> NoReturn:
    raise LedgerError(
        ExitCode.VALIDATION_FAILED,
        [Diagnostic(code.value, field=field, message=message, fatal=True)],
    )


def _pairs_no_duplicates(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            _validation_error(FailureCode.SCHEMA_INVALID, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(value: str) -> NoReturn:
    _validation_error(FailureCode.SCHEMA_INVALID, f"non-finite JSON number: {value}")


def _validate_json_value(value: object, path: str = "$" ) -> None:
    if value is None or isinstance(value, (bool, int)):
        return
    if isinstance(value, float):
        if not math.isfinite(value):
            _validation_error(FailureCode.SCHEMA_INVALID, f"non-finite number at {path}")
        return
    if isinstance(value, str):
        if any(0xD800 <= ord(char) <= 0xDFFF for char in value):
            _validation_error(FailureCode.SCHEMA_INVALID, f"unpaired surrogate at {path}")
        return
    if isinstance(value, list):
        for index, item in enumerate(value):
            _validate_json_value(item, f"{path}[{index}]")
        return
    if isinstance(value, dict):
        for key, item in value.items():
            if not isinstance(key, str):
                _validation_error(FailureCode.SCHEMA_INVALID, f"non-string key at {path}")
            _validate_json_value(key, f"{path}.<key>")
            _validate_json_value(item, f"{path}.{key}")
        return
    _validation_error(
        FailureCode.SCHEMA_INVALID,
        f"unsupported JSON value {type(value).__name__} at {path}",
    )


def load_json_strict(data: str | bytes) -> object:
    """Decode one strict JSON value while rejecting duplicates and extensions."""

    if isinstance(data, bytes):
        try:
            data = data.decode("utf-8", errors="strict")
        except UnicodeDecodeError as exc:
            _validation_error(FailureCode.SCHEMA_INVALID, f"invalid UTF-8: {exc}")
    try:
        value = json.loads(
            data,
            object_pairs_hook=_pairs_no_duplicates,
            parse_constant=_reject_constant,
        )
    except LedgerError:
        raise
    except (json.JSONDecodeError, RecursionError) as exc:
        _validation_error(FailureCode.SCHEMA_INVALID, f"invalid JSON: {exc}")
    _validate_json_value(value)
    return value


def canonical_json_bytes(value: object) -> bytes:
    """Return the sole accepted UTF-8 representation for a JSON value."""

    _validate_json_value(value)
    try:
        encoded = json.dumps(
            value,
            ensure_ascii=False,
            allow_nan=False,
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8")
    except (UnicodeEncodeError, ValueError, TypeError) as exc:
        _validation_error(FailureCode.SCHEMA_INVALID, f"cannot encode canonical JSON: {exc}")
    return encoded + b"\n"


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


def validate_relative_path(value: str, *, field: str = "path") -> str:
    """Validate and return a portable repository-relative path."""

    if not isinstance(value, str) or not value:
        _validation_error(FailureCode.UNSAFE_PATH, "path must be a non-empty string", field=field)
    if "\\" in value or PurePosixPath(value).is_absolute() or PureWindowsPath(value).is_absolute():
        _validation_error(FailureCode.UNSAFE_PATH, f"path is not portable and relative: {value!r}", field=field)
    if PureWindowsPath(value).drive or ":" in value:
        _validation_error(FailureCode.UNSAFE_PATH, f"path contains a drive or stream: {value!r}", field=field)
    parts = value.split("/")
    for part in parts:
        if part in {"", ".", ".."}:
            _validation_error(FailureCode.UNSAFE_PATH, f"unsafe path segment in {value!r}", field=field)
        if len(part) > 255 or part.endswith((" ", ".")):
            _validation_error(FailureCode.UNSAFE_PATH, f"unsafe path segment {part!r}", field=field)
        if any(ord(char) < 32 or ord(char) == 127 for char in part):
            _validation_error(FailureCode.UNSAFE_PATH, f"control character in path {value!r}", field=field)
        stem = part.split(".", 1)[0].rstrip(" .").upper()
        if stem in _DEVICE_STEMS:
            _validation_error(FailureCode.UNSAFE_PATH, f"reserved device path {value!r}", field=field)
    return value


@dataclass
class StagedWrite:
    """A fully flushed temporary sibling awaiting one atomic replacement."""

    destination: Path
    temporary: Path
    committed: bool = False

    def commit(self) -> None:
        try:
            os.replace(self.temporary, self.destination)
            self.committed = True
        except OSError as exc:
            raise LedgerError(
                ExitCode.WORKSPACE_FAILED,
                [
                    Diagnostic(
                        FailureCode.OUTPUT_IO_FAILED.value,
                        source_path=self.destination.as_posix(),
                        message=str(exc),
                        fatal=True,
                    )
                ],
            ) from exc

    def cleanup(self) -> None:
        if self.committed:
            return
        try:
            self.temporary.unlink()
        except FileNotFoundError:
            pass


def stage_atomic_bytes(path: Path, payload: bytes) -> StagedWrite:
    """Flush one temporary sibling without replacing its destination yet."""

    temp_path: Path | None = None
    descriptor: int | None = None
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        for _attempt in range(100):
            candidate = path.with_name(f".{path.name}.{secrets.token_hex(8)}.tmp")
            try:
                flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_BINARY", 0)
                descriptor = os.open(candidate, flags, 0o600)
                temp_path = candidate
                break
            except FileExistsError:
                continue
        if descriptor is None or temp_path is None:
            raise OSError("could not allocate a unique temporary file")
        view = memoryview(payload)
        while view:
            written = os.write(descriptor, view)
            if written <= 0:
                raise OSError("short write while creating atomic output")
            view = view[written:]
        os.fsync(descriptor)
        os.close(descriptor)
        descriptor = None
        staged = StagedWrite(path, temp_path)
        temp_path = None
        return staged
    except OSError as exc:
        raise LedgerError(
            ExitCode.WORKSPACE_FAILED,
            [Diagnostic(FailureCode.OUTPUT_IO_FAILED.value, source_path=path.as_posix(), message=str(exc), fatal=True)],
        ) from exc
    finally:
        if descriptor is not None:
            os.close(descriptor)
        if temp_path is not None:
            try:
                temp_path.unlink()
            except FileNotFoundError:
                pass


def atomic_write_bytes(path: Path, payload: bytes) -> None:
    """Durably replace one file from an exclusive temporary sibling."""

    staged = stage_atomic_bytes(path, payload)
    try:
        staged.commit()
    finally:
        staged.cleanup()
