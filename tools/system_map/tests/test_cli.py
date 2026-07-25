"""Production-shaped CLI smoke tests."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class CliSmokeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.repo = Path(__file__).resolve().parents[3]
        (cls.repo / "target").mkdir(exist_ok=True)
        required = (
            cls.repo / "system_map/registry.v2.json",
            cls.repo / "system_map/source-lock.v2.json",
            cls.repo / "system_map/topology.v2.json",
        )
        if not all(path.exists() for path in required):
            raise unittest.SkipTest("canonical System Map v2 data is unavailable")

    def test_check_emits_actionable_warning_documents(self) -> None:
        completed = self._run("check", "--ci")
        result = json.loads(completed.stdout)

        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(result["systems"], 336)
        self.assertEqual(result["warnings"], len(result["diagnostics"]))
        self.assertTrue(
            all(item["severity"] == "warning" for item in result["diagnostics"])
        )

    def test_unicode_loop_route_is_utf8_safe(self) -> None:
        completed = self._run("loop", "LOOP-004-HARVEST-CREDIT")

        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("GSI-07.15 → GSI-09.05", completed.stdout)
        self.assertIn("stages:", completed.stdout)
        self.assertIn("Rust touchpoints:", completed.stdout)

    def test_legacy_alias_resolves_to_canonical_system(self) -> None:
        completed = self._run("show", "GSI-04.03A")

        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("historical pseudo-GSI alias", completed.stdout)
        self.assertIn("GSI-04.09", completed.stdout)

    def test_unknown_system_returns_structured_error(self) -> None:
        completed = self._run("show", "GSI-99.99")
        result = json.loads(completed.stderr)

        self.assertEqual(completed.returncode, 4)
        self.assertEqual(
            result["diagnostics"][0]["code"], "UNKNOWN_SYSTEM"
        )

    def test_render_records_exact_input_provenance_and_checks_fresh_output(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(
            prefix="system-map-cli-", dir=self.repo / "target"
        ) as temporary:
            output = Path(temporary)
            output_arg = output.relative_to(self.repo).as_posix()

            rendered = self._run("render", "--output", output_arg)
            self.assertEqual(rendered.returncode, 0, rendered.stderr)

            json_path = output / "system-map.v2.json"
            markdown_path = output / "SYSTEM_MAP_V2.md"
            report = json.loads(json_path.read_text(encoding="utf-8"))
            provenance = report["provenance"]
            self.assertEqual(
                provenance["generator"],
                {"id": "vera20k-system-map", "version": "1.0.0"},
            )
            canonical_inputs = {
                "registry": "system_map/registry.v2.json",
                "source_lock": "system_map/source-lock.v2.json",
                "topology": "system_map/topology.v2.json",
            }
            for name, relative_path in canonical_inputs.items():
                source = self.repo / relative_path
                expected_sha = hashlib.sha256(source.read_bytes()).hexdigest()
                self.assertEqual(
                    provenance["inputs"][name],
                    {"path": relative_path, "sha256": expected_sha},
                )

            markdown = markdown_path.read_text(encoding="utf-8")
            self.assertIn("`vera20k-system-map` `1.0.0`", markdown)
            for item in provenance["inputs"].values():
                self.assertIn(f"`{item['sha256']}`", markdown)

            before = {
                path: (path.read_bytes(), path.stat().st_mtime_ns)
                for path in (json_path, markdown_path)
            }
            checked = self._run(
                "render", "--check", "--output", output_arg
            )
            self.assertEqual(checked.returncode, 0, checked.stderr)
            result = json.loads(checked.stdout)
            self.assertTrue(result["checked"])
            for path, (payload, modified_ns) in before.items():
                self.assertEqual(path.read_bytes(), payload)
                self.assertEqual(path.stat().st_mtime_ns, modified_ns)

    def test_render_check_reports_missing_outputs_without_writing(self) -> None:
        with tempfile.TemporaryDirectory(
            prefix="system-map-cli-", dir=self.repo / "target"
        ) as temporary:
            output = Path(temporary)
            output_arg = output.relative_to(self.repo).as_posix()

            completed = self._run(
                "render", "--check", "--output", output_arg
            )
            result = json.loads(completed.stderr)

            self.assertEqual(completed.returncode, 1)
            self.assertEqual(
                {
                    (item["code"], item["record_id"])
                    for item in result["diagnostics"]
                },
                {
                    ("MISSING_RENDER_OUTPUT", "json"),
                    ("MISSING_RENDER_OUTPUT", "markdown"),
                },
            )
            self.assertEqual(list(output.iterdir()), [])

    def test_render_check_reports_stale_output_without_repairing_it(self) -> None:
        with tempfile.TemporaryDirectory(
            prefix="system-map-cli-", dir=self.repo / "target"
        ) as temporary:
            output = Path(temporary)
            output_arg = output.relative_to(self.repo).as_posix()
            rendered = self._run("render", "--output", output_arg)
            self.assertEqual(rendered.returncode, 0, rendered.stderr)

            json_path = output / "system-map.v2.json"
            markdown_path = output / "SYSTEM_MAP_V2.md"
            json_path.write_bytes(json_path.read_bytes() + b"stale\n")
            before_json = json_path.read_bytes()
            before_markdown = markdown_path.read_bytes()

            completed = self._run(
                "render", "--check", "--output", output_arg
            )
            result = json.loads(completed.stderr)

            self.assertEqual(completed.returncode, 1)
            self.assertEqual(len(result["diagnostics"]), 1)
            self.assertEqual(
                result["diagnostics"][0]["code"], "STALE_RENDER_OUTPUT"
            )
            self.assertEqual(
                result["diagnostics"][0]["record_id"], "json"
            )
            self.assertIn(
                "expected sha256=", result["diagnostics"][0]["message"]
            )
            self.assertIn(
                "actual sha256=", result["diagnostics"][0]["message"]
            )
            self.assertEqual(json_path.read_bytes(), before_json)
            self.assertEqual(markdown_path.read_bytes(), before_markdown)

    def _run(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, "-m", "tools.system_map", *args],
            cwd=self.repo,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="strict",
        )


if __name__ == "__main__":
    unittest.main()
