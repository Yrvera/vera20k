from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from tools.tactical_certification.core import (
    OutputExistsError,
    ValidationError,
    assert_snapshot_unchanged,
    contains_forbidden_verdict,
    parse_json_bytes,
    reject_reparse_ancestors,
    require_regular_file,
    write_bytes_exclusive,
    write_json_exclusive,
)


class CoreTests(unittest.TestCase):
    def test_json_rejects_duplicate_nonfinite_and_nonobject(self) -> None:
        with self.assertRaisesRegex(ValidationError, "duplicate"):
            parse_json_bytes(b'{"a":1,"a":2}', "test")
        with self.assertRaisesRegex(ValidationError, "non-finite"):
            parse_json_bytes(b'{"value":NaN}', "test")
        with self.assertRaisesRegex(ValidationError, "non-finite"):
            parse_json_bytes(b'{"value":1e400}', "test")
        with self.assertRaisesRegex(ValidationError, "root"):
            parse_json_bytes(b"[]", "test")

    def test_regular_snapshot_detects_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary).absolute() / "input.bin"
            path.write_bytes(b"first")
            before = require_regular_file(path, "input")
            path.write_bytes(b"second")
            with self.assertRaisesRegex(ValidationError, "changed"):
                assert_snapshot_unchanged(before, "input")

    def test_exclusive_outputs_fsync_and_never_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).absolute()
            binary = directory / "value.bin"
            report = directory / "value.json"
            write_bytes_exclusive(binary, b"immutable")
            write_json_exclusive(report, {"status": "VALID"})
            self.assertEqual(binary.read_bytes(), b"immutable")
            self.assertIn('"VALID"', report.read_text(encoding="utf-8"))
            with self.assertRaises(OutputExistsError):
                write_bytes_exclusive(binary, b"replacement")

    def test_relative_path_and_available_symlink_are_rejected(self) -> None:
        with self.assertRaisesRegex(ValidationError, "absolute"):
            reject_reparse_ancestors(Path("relative"), "test")
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary).absolute()
            target = directory / "target"
            target.mkdir()
            link = directory / "link"
            try:
                link.symlink_to(target, target_is_directory=True)
            except OSError:
                self.skipTest("this Windows account cannot create symbolic links")
            with self.assertRaisesRegex(ValidationError, "reparse|link|junction"):
                reject_reparse_ancestors(link / "child", "test")

    def test_native_result_labels_are_forbidden_recursively(self) -> None:
        self.assertTrue(contains_forbidden_verdict({"result": "MATCH"}))
        self.assertTrue(contains_forbidden_verdict({"nested": ["DRIFT"]}))
        self.assertFalse(
            contains_forbidden_verdict(
                {"status": "VALID", "native_comparator": "NONE"}
            )
        )
