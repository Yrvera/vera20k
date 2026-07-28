"""Tests for strict and deterministic JSON/file primitives."""

from __future__ import annotations

import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from tools.parity_ledger.errors import LedgerError
from tools.parity_ledger.jsonio import (
    atomic_write_bytes,
    canonical_json_bytes,
    load_json_strict,
    sha256_file,
    validate_relative_path,
)


class JsonIoTests(unittest.TestCase):
    def test_canonical_utf8_and_keys(self) -> None:
        self.assertEqual(
            canonical_json_bytes({"z": 2, "a": "Yuri Ø"}),
            '{"a":"Yuri Ø","z":2}\n'.encode("utf-8"),
        )

    def test_rejects_duplicate_nonfinite_and_surrogate(self) -> None:
        for payload in ('{"a":1,"a":2}', '{"a":NaN}', '"\\ud800"'):
            with self.subTest(payload=payload), self.assertRaises(LedgerError):
                load_json_strict(payload)
        with self.assertRaises(LedgerError):
            canonical_json_bytes("\ud800")

    def test_safe_relative_paths(self) -> None:
        self.assertEqual(validate_relative_path("src/sim/tick.rs"), "src/sim/tick.rs")
        for value in (
            "",
            "/abs",
            "C:/root",
            "../up",
            "a\\b",
            "a//b",
            "NUL.txt",
            "CON .dat",
            "COM1 .log",
            "a. ",
        ):
            with self.subTest(value=value), self.assertRaises(LedgerError):
                validate_relative_path(value)

    def test_streaming_hash_and_atomic_replace(self) -> None:
        payload = (b"ledger\n" * 4096) + b"tail\n"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "nested" / "value.json"
            atomic_write_bytes(path, payload)
            self.assertEqual(path.read_bytes(), payload)
            self.assertEqual(sha256_file(path), hashlib.sha256(payload).hexdigest())

    def test_failed_replace_keeps_destination(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "value.json"
            path.write_bytes(b"old")
            with mock.patch("tools.parity_ledger.jsonio.os.replace", side_effect=OSError("blocked")):
                with self.assertRaises(LedgerError):
                    atomic_write_bytes(path, b"new")
            self.assertEqual(path.read_bytes(), b"old")
            self.assertEqual(list(path.parent.glob(".*.tmp")), [])


if __name__ == "__main__":
    unittest.main()
