"""Runner failure contracts using synthetic x86; these are not retail evidence.

python -m unittest tools.test_native_oracle -v
"""

from contextlib import redirect_stdout
import io
import os
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX

from tools import native_oracle as oracle


CODE = 0x1000


def fixture(code):
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    uc.mem_map(CODE, 0x1000)
    uc.mem_write(CODE, code)
    return uc


class ExecutionTests(unittest.TestCase):
    def test_return_boundary_and_required_address(self):
        uc = fixture(b"\xb8\x2a\x00\x00\x00\x90")
        self.assertEqual(oracle.run_checked(uc, CODE, CODE + 5,
                                           required_addresses=[CODE]), CODE + 5)
        self.assertEqual(uc.reg_read(UC_X86_REG_EAX), 42)

    def test_instruction_limit_rejects_silent_stop_with_trace(self):
        with self.assertRaisesRegex(oracle.OracleError, "Incomplete.*trace:.*0x00001000"):
            oracle.run_checked(fixture(b"\xeb\xfe"), CODE, CODE + 2, count=4)

    def test_time_limit_rejects_silent_stop(self):
        with self.assertRaisesRegex(oracle.OracleError, "Incomplete"):
            oracle.run_checked(fixture(b"\xeb\xfe"), CODE, CODE + 2,
                               count=1_000_000_000, timeout_us=1000)

    def test_fault_is_not_a_result(self):
        with self.assertRaisesRegex(oracle.OracleError, "faulted.*trace:"):
            oracle.run_checked(fixture(b"\xa1\x00\x00\x00\x70"), CODE, CODE + 5)

    def test_external_early_stop_is_not_a_result(self):
        uc = fixture(b"\x90\x90")
        uc.hook_add(UC_HOOK_CODE, lambda machine, *_: machine.emu_stop())
        with self.assertRaisesRegex(oracle.OracleError, "Incomplete"):
            oracle.run_checked(uc, CODE, CODE + 2)

    def test_missing_required_path_is_rejected(self):
        with self.assertRaisesRegex(oracle.OracleError, "not reached"):
            oracle.run_checked(fixture(b"\xeb\x01\x90\x90"), CODE, CODE + 3,
                               required_addresses=[CODE + 2])

    def test_secondary_boundary_and_old_exit_configuration(self):
        uc = fixture(b"\x90\x90\x90")
        uc.ctl_exits_enabled(True)
        uc.ctl_set_exits([CODE + 1])
        self.assertEqual(oracle.run_checked(uc, CODE, (CODE + 3, CODE + 2)), CODE + 2)

    def test_stop_boundary_cannot_claim_instruction_coverage(self):
        with self.assertRaises(ValueError):
            oracle.run_checked(fixture(b"\x90"), CODE, CODE + 1,
                               required_addresses=[CODE + 1])

    def synthetic_call(self, code, **kwargs):
        def load(uc):
            uc.mem_map(oracle.IMAGE_BASE, 0x1000)
            uc.mem_write(oracle.IMAGE_BASE, code)
        with patch.object(oracle, "load_image", load), \
                patch.object(oracle, "image_bytes", return_value=b""), \
                patch.object(oracle, "_sections", return_value=[(0, 0, len(code), len(code), 0x20000000)]):
            return oracle.call(oracle.IMAGE_BASE, **kwargs)

    def test_call_rejects_synthetic_entry_and_native_code_replacement(self):
        sections = [(0, 0, 2, 2, 0x20000000)]
        with patch.object(oracle, "image_bytes", return_value=b""), \
                patch.object(oracle, "_sections", return_value=sections):
            with self.assertRaisesRegex(oracle.OracleError, "original native"):
                oracle.call(oracle.SCRATCH)
        with self.assertRaisesRegex(oracle.OracleError, "cannot replace native"):
            self.synthetic_call(b"\x90\xc3", writes={oracle.IMAGE_BASE: b"\xc3"})

    def test_each_call_starts_with_fresh_global_data(self):
        addr = struct.pack("<I", oracle.IMAGE_BASE + 0x100)
        code = b"\xff\x05" + addr + b"\xa1" + addr + b"\xc3"
        self.assertEqual([self.synthetic_call(code)["eax"] for _ in range(2)], [1, 1])

    def test_fstp_observation_must_finish_before_result(self):
        code = b"\xd9\xe8\xc3"  # FLD1; RET. Third instruction is observation FSTP.
        with self.assertRaisesRegex(oracle.OracleError, "Incomplete"):
            self.synthetic_call(code, capture_st0=True, timeout_instr=2)
        self.assertEqual(self.synthetic_call(code, capture_st0=True)["st0"], 1.0)

    def test_jump_past_fstp_observation_cannot_return_unwritten_zero(self):
        code = b"\x68" + struct.pack("<I", oracle.RET_MAGIC + 6) + b"\xc3"
        with self.assertRaisesRegex(oracle.OracleError, "not reached"):
            self.synthetic_call(code, capture_st0=True)


class IdentityAndReferenceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.target = Path(self.directory.name) / "vectors.json"
        self.metadata = {"native_sha256": "synthetic-test-only", "unicorn_core": [2, 1, 0]}

    def finish(self, data, *args, **kwargs):
        with redirect_stdout(io.StringIO()):
            oracle.finish_vectors(data, self.target, provenance=kwargs.get("provenance", self.metadata),
                                  argv=list(args))

    def test_wrong_executable_is_rejected_explicitly(self):
        self.target.write_bytes(b"not the original executable")
        with self.assertRaisesRegex(oracle.OracleError, "Unsupported gamemd.exe SHA-256"):
            oracle._verified_image(self.target)

    def test_explicit_missing_path_does_not_fall_back(self):
        (self.target.parent / "gamemd.exe").write_bytes(b"fallback must not be selected")
        with patch.dict(os.environ, {"VERA20K_GAMEMD_EXE": str(self.target),
                                     "RA2_DIR": self.directory.name}):
            with self.assertRaisesRegex(oracle.OracleError, "Missing original executable"):
                oracle.configured_gamemd()

    def test_default_missing_reference_does_not_create_it(self):
        with self.assertRaisesRegex(oracle.OracleError, "Reference missing"):
            self.finish({"value": 42})
        self.assertEqual(list(Path(self.directory.name).iterdir()), [])

    def test_write_then_default_check_never_changes_files(self):
        data = {"cases": [(42, 17)]}
        self.finish(data, "--write")
        before = {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in self.target.parent.iterdir()}
        self.finish(data)
        self.assertEqual(before, {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in before})
        with self.assertRaisesRegex(oracle.OracleError, r"\$\.cases\[0\]\[1\]: expected 17, got 18"):
            self.finish({"cases": [(42, 18)]})
        self.assertEqual(before, {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in before})

    def test_provenance_change_rejects_matching_payload(self):
        self.finish({"value": 42}, "--write")
        with self.assertRaisesRegex(oracle.OracleError, "Provenance mismatch"):
            self.finish({"value": 42}, provenance=dict(self.metadata, unicorn_core=[2, 2, 0]))

    def test_bad_metadata_cannot_partially_replace_reference(self):
        self.finish({"value": 42}, "--write")
        before = self.target.read_bytes()
        with self.assertRaises(ValueError):
            self.finish({"value": 43}, "--write", provenance={"bad": float("nan")})
        self.assertEqual(self.target.read_bytes(), before)

    def test_help_does_not_evaluate_native_callables(self):
        def forbidden():
            self.fail("--help evaluated native work")
        with self.assertRaises(SystemExit) as stopped:
            self.finish(forbidden, "--help", provenance=forbidden)
        self.assertEqual(stopped.exception.code, 0)

    def test_legacy_comparison_preserves_signed_zero(self):
        self.target.write_text('{"value": -0.0}', encoding="utf-8")
        with self.assertRaisesRegex(oracle.OracleError, "binary64 differs"):
            self.finish({"value": 0.0})

    def test_long_dump_difference_reports_offset_and_context(self):
        difference = oracle.first_difference("a" * 200 + "b", "a" * 200 + "c")
        self.assertIn("character 200", difference)
        self.assertIn("ab'", difference)
        self.assertIn("ac'", difference)


if __name__ == "__main__":
    unittest.main()
