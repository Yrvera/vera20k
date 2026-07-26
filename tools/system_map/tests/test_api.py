"""Public System Map API and conservative candidate-ranking tests."""

from __future__ import annotations

from pathlib import Path
import secrets
import unittest

from tools.system_map.api import (
    find_candidates,
    load_report,
    report_summary,
    require_loop,
    require_mechanism,
    require_system,
)
from tools.system_map.model import SystemMapError


class PublicApiTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.repo = Path(__file__).resolve().parents[3]
        cls.report = load_report(cls.repo, require_sources=True)

    def test_public_loader_returns_validated_live_report(self) -> None:
        summary = report_summary(self.report)

        self.assertEqual(summary["system_count"], 336)
        self.assertGreater(summary["loop_count"], 0)
        self.assertEqual(summary["mechanism_count"], 7)
        self.assertEqual(summary["error_count"], 0)
        self.assertEqual(
            sum(summary["mapping_freshness"].values()),
            summary["system_count"],
        )

    def test_power_query_prefers_power_owner_and_recovery_loop(self) -> None:
        result = find_candidates(
            self.report,
            "power outage recovery",
            limit=5,
        )

        self.assertTrue(result["matched"])
        self.assertEqual(
            result["system_candidates"][0]["id"],
            "GSI-09.07",
        )
        self.assertEqual(
            result["loop_candidates"][0]["id"],
            "LOOP-012-POWER-OUTAGE-RECOVERY",
        )
        self.assertTrue(
            result["system_candidates"][0]["candidate_only"]
        )

    def test_two_term_query_rejects_one_term_context_noise(self) -> None:
        result = find_candidates(self.report, "bridge collapse", limit=20)

        self.assertTrue(result["system_candidates"])
        self.assertTrue(
            all(
                set(candidate["matched_terms"]) == {"bridge", "collapse"}
                for candidate in result["system_candidates"]
            )
        )
        self.assertEqual(result["loop_candidates"], [])

    def test_exact_ids_resolve_case_insensitively(self) -> None:
        system = require_system(self.report, "gsi-07.15")
        loop = require_loop(
            self.report,
            "loop-004-harvest-credit",
        )
        mechanism = require_mechanism(
            self.report,
            "mblk-004-powered-radar-gate",
        )

        self.assertEqual(system["system"]["id"], "GSI-07.15")
        self.assertEqual(loop["id"], "LOOP-004-HARVEST-CREDIT")
        self.assertEqual(
            mechanism["mechanism"]["id"],
            "MBLK-004-POWERED-RADAR-GATE",
        )
        self.assertEqual(loop["ordered_systems"][0], "GSI-07.15")

    def test_mechanism_candidates_are_independent_and_bounded(self) -> None:
        result = find_candidates(
            self.report,
            "power radar handoff",
            limit=2,
        )

        self.assertEqual(len(result["mechanism_candidates"]), 2)
        self.assertEqual(
            result["mechanism_candidates"][0]["id"],
            "MBLK-004-POWERED-RADAR-GATE",
        )
        self.assertTrue(
            all(
                candidate["candidate_only"]
                for candidate in result["mechanism_candidates"]
            )
        )
        self.assertNotIn("steps", result["mechanism_candidates"][0])

    def test_mechanism_namespace_and_loop_residuals_stay_separate(self) -> None:
        self.assertTrue(
            all(edge["id"].startswith("EDGE-") for edge in self.report["edges"])
        )
        self.assertTrue(
            all(
                edge["id"].startswith("MBEDGE-")
                for edge in self.report["mechanism_edges"]
            )
        )
        loop = self.report["loops"]["LOOP-012-POWER-OUTAGE-RECOVERY"]
        self.assertEqual(
            loop["mechanisms"],
            [
                "MBLK-001-SELL-MISSION-AUTHORITY",
                "MBLK-002-SELL-REFUND-COMMIT",
                "MBLK-003-HOUSE-POWER-REASSESSMENT",
                "MBLK-004-POWERED-RADAR-GATE",
                "MBLK-006-POWER-BAR-PRESENTATION-HANDOFF",
                "MBLK-007-RADAR-PRESENTATION-HANDOFF",
                "MBLK-005-LOW-POWER-NOTIFICATION-GUARD",
            ],
        )
        self.assertEqual(
            loop["unmapped_mechanism_stage_orders"],
            list(range(10, 19)),
        )
        untouched = self.report["loops"]["LOOP-004-HARVEST-CREDIT"]
        self.assertEqual(untouched["mechanisms"], [])
        self.assertEqual(untouched["unmapped_mechanism_stage_orders"], [])

    def test_unknown_exact_id_raises_structured_error(self) -> None:
        with self.assertRaises(SystemMapError) as raised:
            require_system(self.report, "GSI-99.99")

        self.assertEqual(raised.exception.exit_code, 4)
        self.assertEqual(
            raised.exception.diagnostics[0].code,
            "UNKNOWN_SYSTEM",
        )

    def test_tooling_only_query_is_an_explicit_zero_match(self) -> None:
        result = find_candidates(
            self.report,
            f"research navigator {secrets.token_hex(8)}",
        )

        self.assertFalse(result["matched"])
        self.assertEqual(result["system_candidates"], [])
        self.assertEqual(result["loop_candidates"], [])


if __name__ == "__main__":
    unittest.main()
