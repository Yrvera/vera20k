"""Cross-process serialization for research-index generation publication."""

from __future__ import annotations

from contextlib import contextmanager
import os
from pathlib import Path
import time


DEFAULT_LOCK_TIMEOUT_SECONDS = 120.0


class IndexLockTimeout(RuntimeError):
    """The index publication lock could not be acquired in time."""


def lock_path(db_path: Path) -> Path:
    return db_path.with_suffix(db_path.suffix + ".lock")


@contextmanager
def index_lock(
    db_path: Path,
    timeout_seconds: float = DEFAULT_LOCK_TIMEOUT_SECONDS,
):
    """Hold one cross-process advisory lock for DB/manifest publication."""
    path = lock_path(db_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    handle = path.open("a+b")
    try:
        if handle.tell() == 0:
            handle.write(b"\0")
            handle.flush()
        deadline = time.monotonic() + timeout_seconds
        while True:
            try:
                _lock_handle(handle)
                break
            except OSError as exc:
                if time.monotonic() >= deadline:
                    raise IndexLockTimeout(
                        f"timed out waiting for research-index lock: {path}"
                    ) from exc
                time.sleep(0.1)
        try:
            yield
        finally:
            _unlock_handle(handle)
    finally:
        handle.close()


def _lock_handle(handle) -> None:
    handle.seek(0)
    if os.name == "nt":
        import msvcrt

        msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
        return

    import fcntl

    fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)


def _unlock_handle(handle) -> None:
    handle.seek(0)
    if os.name == "nt":
        import msvcrt

        msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
        return

    import fcntl

    fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
