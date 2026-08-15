from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "collect_candidates.py"
SPEC = importlib.util.spec_from_file_location("rust_scan_collect_candidates", SCRIPT_PATH)
assert SPEC is not None and SPEC.loader is not None
COLLECTOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COLLECTOR
SPEC.loader.exec_module(COLLECTOR)


class CandidateCollectorTests(unittest.TestCase):
    def test_sanitizer_ignores_comments_and_string_bodies(self) -> None:
        lines = [
            "// HashMap f64",
            'let label = "HashSet f32";',
            "let live: f64 = 1.0;",
            "/* unsafe { */ let index = value as usize;",
        ]
        sanitized = COLLECTOR.sanitize_lines(lines)
        self.assertNotIn("HashMap", sanitized[0])
        self.assertNotIn("HashSet", sanitized[1])
        self.assertIn("f64", sanitized[2])
        self.assertNotIn("unsafe", sanitized[3])
        self.assertIn("as usize", sanitized[3])

    def test_inline_test_scope_is_tagged(self) -> None:
        lines = [
            "fn production() {}",
            "#[cfg(test)]",
            "mod tests {",
            "    fn helper() {",
            "        let value: f64 = 1.0;",
            "    }",
            "}",
            "fn production_again() {}",
        ]
        sanitized = COLLECTOR.sanitize_lines(lines)
        flags = COLLECTOR.probable_test_flags(
            "src/sim/example.rs", lines, sanitized
        )
        self.assertFalse(flags[0])
        self.assertTrue(all(flags[1:7]))
        self.assertFalse(flags[7])

    def test_grouped_architecture_path_and_cast_are_candidates(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            root = Path(raw_root)
            source = root / "src" / "sim" / "example.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                "\n".join(
                    (
                        "//! Example.",
                        "use crate::{",
                        "    map::Map,",
                        "    render::Thing,",
                        "};",
                        "fn index(value: i32) -> usize { value as usize }",
                        "fn bucket(value: u32) -> u8 { (value as u8) % 10 }",
                        "#[cfg(test)]",
                        "mod tests {",
                        "    fn oracle() { let _: f64 = 1.0; }",
                        "}",
                    )
                ),
                encoding="utf-8",
            )
            rules = tuple(
                rule
                for rule in COLLECTOR.RULES
                if rule.rule_id in {"ARCH-001", "DET-001", "DET-004"}
            )
            candidates = COLLECTOR.collect_regex_candidates(
                root, [source], rules, "auto"
            )
            by_rule = {}
            for candidate in candidates:
                by_rule.setdefault(candidate.rule_id, []).append(candidate)

            self.assertEqual(by_rule["ARCH-001"][0].line, 2)
            self.assertEqual(
                [item.line for item in by_rule["DET-004"]],
                [6, 7],
            )
            self.assertIn(
                "probable-test-context",
                by_rule["DET-001"][0].hints,
            )
            self.assertNotIn("severity", by_rule["DET-004"][0].as_dict())

    def test_cfg_rule_ignores_plain_test_gate_but_finds_feature_gate(self) -> None:
        rule = next(rule for rule in COLLECTOR.RULES if rule.rule_id == "DET-005")
        self.assertIsNone(rule.regex.search("#[cfg(test)]"))
        self.assertIsNotNone(rule.regex.search('#[cfg(feature = "parallel")]'))
        self.assertIsNotNone(
            rule.regex.search('#[cfg(target_pointer_width = "32")]')
        )

    def test_parallel_hash_and_unsafe_signals_are_candidates(self) -> None:
        rules = {rule.rule_id: rule for rule in COLLECTOR.RULES}
        self.assertIsNotNone(
            rules["DET-002"].regex.search("DefaultHasher::new()")
        )
        self.assertIsNotNone(
            rules["DET-006"].regex.search("values.par_bridge().find_any(predicate)")
        )
        self.assertIsNotNone(
            rules["DET-006"].regex.search("values.into_par_iter().reduce_with(merge)")
        )
        self.assertIsNotNone(
            rules["SAFE-001"].regex.search("unsafe { values.set_len(count); }")
        )

    def test_api_ownership_rules_are_opt_in(self) -> None:
        default_rules = COLLECTOR.selected_rules(
            (
                "determinism",
                "architecture",
                "state",
                "safety",
                "performance",
            ),
            (),
        )
        default_ids = {rule.rule_id for rule in default_rules}
        self.assertNotIn("OWN-001", default_ids)
        self.assertNotIn("OWN-002", default_ids)
        self.assertIn("OWN-003", default_ids)

        ownership_rules = COLLECTOR.selected_rules(("ownership",), ())
        self.assertEqual(
            {rule.rule_id for rule in ownership_rules},
            {"OWN-001", "OWN-002"},
        )

    def test_float_literal_and_bare_extern_block_are_candidates(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            root = Path(raw_root)
            source = root / "src" / "sim" / "ffi_math.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                "//! Example.\n"
                "fn ratio() { let value = 1.25; }\n"
                'extern "C" { fn native(); }\n',
                encoding="utf-8",
            )
            rules = tuple(
                rule
                for rule in COLLECTOR.RULES
                if rule.rule_id in {"DET-001", "SAFE-001"}
            )
            candidates = COLLECTOR.collect_regex_candidates(
                root, [source], rules, "auto"
            )
            self.assertEqual(
                {(candidate.rule_id, candidate.line) for candidate in candidates},
                {("DET-001", 2), ("SAFE-001", 3)},
            )

    def test_candidate_order_is_stable_and_deduplicated(self) -> None:
        candidate_a = COLLECTOR.Candidate(
            "DET-004", "determinism", "src/sim/z.rs", 9, "x as u8", ()
        )
        candidate_b = COLLECTOR.Candidate(
            "ARCH-001",
            "architecture",
            "src/sim/a.rs",
            2,
            "crate::render::Thing",
            (),
        )
        ordered = COLLECTOR.deduplicate_and_sort(
            [candidate_a, candidate_b, candidate_a]
        )
        self.assertEqual([item.path for item in ordered], ["src/sim/a.rs", "src/sim/z.rs"])

    def test_crlf_input_keeps_posix_path_and_line_number(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            root = Path(raw_root)
            source = root / "src" / "sim" / "nested" / "crlf.rs"
            source.parent.mkdir(parents=True)
            source.write_bytes(b"//! Example.\r\nfn value() -> f64 { 1.0 }\r\n")
            rule = next(rule for rule in COLLECTOR.RULES if rule.rule_id == "DET-001")
            candidates = COLLECTOR.collect_regex_candidates(
                root, [source], (rule,), "auto"
            )
            self.assertEqual(len(candidates), 1)
            self.assertEqual(candidates[0].path, "src/sim/nested/crlf.rs")
            self.assertEqual(candidates[0].line, 2)

    def test_named_test_files_are_detected(self) -> None:
        self.assertTrue(COLLECTOR.is_named_test_file("src/sim/world/world_tests.rs"))
        self.assertTrue(COLLECTOR.is_named_test_file("tests/replay.rs"))
        self.assertFalse(COLLECTOR.is_named_test_file("src/sim/world/mod.rs"))

    def test_probable_inline_test_candidates_are_opt_in(self) -> None:
        production = COLLECTOR.Candidate(
            "DET-004", "determinism", "src/sim/a.rs", 1, "x as u8", ()
        )
        test_only = COLLECTOR.Candidate(
            "DET-004",
            "determinism",
            "src/sim/a.rs",
            9,
            "x as u8",
            ("probable-test-context",),
        )
        self.assertEqual(
            COLLECTOR.filter_test_candidates([production, test_only], False),
            [production],
        )
        self.assertEqual(
            COLLECTOR.filter_test_candidates([production, test_only], True),
            [production, test_only],
        )

    def test_file_size_cue_excludes_probable_inline_test_scope(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            root = Path(raw_root)
            source = root / "src" / "sim" / "mostly_inline.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                "\n".join(
                    [
                        "//! Example.",
                        "fn production() {}",
                        "#[cfg(test)]",
                        "mod tests {",
                        *(f"fn case_{index}() {{}}" for index in range(700)),
                        "}",
                    ]
                ),
                encoding="utf-8",
            )
            candidates = COLLECTOR.collect_structure_candidates(root, [source])
            self.assertNotIn(
                "STRUCT-002",
                {candidate.rule_id for candidate in candidates},
            )


if __name__ == "__main__":
    unittest.main()
