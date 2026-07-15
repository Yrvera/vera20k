"""Tests for conservative Git association and declaration-only evaluation."""

from pathlib import Path
import tempfile
import unittest
from unittest import mock

from tools.parity_ledger.errors import LedgerError
from tools.parity_ledger.evidence import derive_evidence, evaluate_evidence
from tools.parity_ledger.jsonio import sha256_bytes
from tools.parity_ledger.model import (
    ArtifactHashCheck,
    ArtifactRef,
    Assignment,
    BridgeTraceCheck,
    Coverage,
    CoverageMode,
    EvidenceDeclaration,
    EvidenceKind,
    GitAncestorCheck,
    ImplementationState,
    Obligation,
    ObligationKind,
    OracleState,
    PathExistsCheck,
    RegressionState,
    SourceClaims,
    SourceRef,
    TestDeclaredCheck,
    Tracking,
)


def gap(anchor: str = "src/miner.rs", local_id: str = "G5") -> Obligation:
    return Obligation(
        f"miner:{local_id}",
        "miner",
        ObligationKind.PARITY_GAP,
        "gap",
        SourceRef(
            "docs/a.md",
            local_id,
            "source",
            "0" * 64,
            Tracking.TRACKED,
            "adapter",
            1,
        ),
        SourceClaims(),
        Assignment(None),
        rust_anchors=(anchor,),
    )


class EvidenceTests(unittest.TestCase):
    def test_unscoped_unrelated_commit_is_not_linked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text("", encoding="utf-8")

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[0] == "log":
                    return "a" * 40 + "\tui: G5 event flag\n"
                if args[0] == "diff-tree":
                    return "src/ui.rs\n"
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                declarations, _ = derive_evidence(repo, (gap(),))
            self.assertEqual([item.kind for item in declarations], [EvidenceKind.IMPLEMENTATION_ANCHOR])

    def test_scoped_commit_requires_anchor_intersection(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text("", encoding="utf-8")

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[0] == "log":
                    return "b" * 40 + "\tminer: G5 exact fix\n"
                if args[0] == "diff-tree":
                    return "src/miner.rs\n"
                if args[0] == "show":
                    return ""
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                declarations, _ = derive_evidence(repo, (gap(),))
            self.assertIn(EvidenceKind.GIT_SCOPED, {item.kind for item in declarations})

    def test_unscoped_matching_commit_is_not_linked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text("", encoding="utf-8")

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[0] == "log":
                    return "c" * 40 + "\tmaintenance G5 cleanup\n"
                if args[0] == "diff-tree":
                    return "src/miner.rs\n"
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                declarations, _ = derive_evidence(repo, (gap(),))
            self.assertEqual(
                [item.kind for item in declarations],
                [EvidenceKind.IMPLEMENTATION_ANCHOR],
            )

    def test_deletion_only_commit_is_not_linked(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text("recreated later", encoding="utf-8")

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[0] == "log":
                    return "d" * 40 + "\tminer: G5 remove old path\n"
                if args[0] == "diff-tree":
                    self.assertIn("--diff-filter=ACMRTUXB", args)
                    return ""
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                declarations, _ = derive_evidence(repo, (gap(),))
            self.assertEqual(
                [item.kind for item in declarations],
                [EvidenceKind.IMPLEMENTATION_ANCHOR],
            )

    def test_id_prefixes_with_lowercase_suffixes_are_not_tokens(self) -> None:
        for local_id, subject in (("S4", "miner: S4b slice"), ("G5", "miner: G5foo slice")):
            with self.subTest(subject=subject), tempfile.TemporaryDirectory() as directory:
                repo = Path(directory)
                (repo / "src").mkdir()
                (repo / "src/miner.rs").write_text("", encoding="utf-8")

                def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                    if args[0] == "log":
                        return "d" * 40 + f"\t{subject}\n"
                    raise AssertionError(args)

                with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                    declarations, _ = derive_evidence(repo, (gap(local_id=local_id),))
                self.assertEqual(
                    [item.kind for item in declarations],
                    [EvidenceKind.IMPLEMENTATION_ANCHOR],
                )

    def test_checks_never_claim_results_or_verified_oracle(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text("#[test]\nfn catches_gap() {}\n", encoding="utf-8")
            artifact_path = repo / "trace.bin"
            artifact_path.write_bytes(b"trace")
            artifact = ArtifactRef("trace.bin", sha256_bytes(b"trace"))
            declarations = (
                EvidenceDeclaration(
                    "evidence:path",
                    ("miner:G5",),
                    EvidenceKind.IMPLEMENTATION_ANCHOR,
                    None,
                    None,
                    None,
                    PathExistsCheck("src/miner.rs"),
                ),
                EvidenceDeclaration(
                    "evidence:git",
                    ("miner:G5",),
                    EvidenceKind.GIT_SCOPED,
                    None,
                    None,
                    None,
                    GitAncestorCheck("a" * 40),
                ),
                EvidenceDeclaration(
                    "evidence:test",
                    ("miner:G5",),
                    EvidenceKind.REGRESSION_DECLARATION,
                    None,
                    None,
                    None,
                    TestDeclaredCheck("src/miner.rs", "catches_gap", "a" * 40),
                ),
                EvidenceDeclaration(
                    "evidence:artifact",
                    ("miner:G5",),
                    EvidenceKind.GAMEMD_VECTOR,
                    artifact,
                    None,
                    Coverage(CoverageMode.SAMPLED, "one case"),
                    ArtifactHashCheck(),
                ),
                EvidenceDeclaration(
                    "evidence:bridge",
                    ("miner:G5",),
                    EvidenceKind.BRIDGE_TRACE,
                    None,
                    None,
                    Coverage(CoverageMode.EXHAUSTIVE, "declared only"),
                    BridgeTraceCheck(artifact, artifact),
                ),
            )

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[:3] == ("show", "-s", "--format=%s"):
                    return "miner: G5 exact fix\n"
                if args[0] == "diff-tree":
                    return "src/miner.rs\n"
                if args[0] == "ls-tree":
                    return "src/miner.rs\n"
                if args[0] == "show" and len(args) == 2:
                    return "#[test]\nfn catches_gap() {}\n"
                if args[0] == "show":
                    return "+++ b/src/miner.rs\n+#[test]\n+fn catches_gap() {}\n"
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                with mock.patch("tools.parity_ledger.evidence.is_ancestor", return_value=True):
                    facts = evaluate_evidence(repo, declarations, (gap(),))
            self.assertEqual(facts.implementation["miner:G5"], ImplementationState.LANDED)
            self.assertEqual(facts.regression["miner:G5"], RegressionState.DECLARED)
            self.assertEqual(facts.oracle["miner:G5"], OracleState.INCOMPLETE)
            self.assertNotIn(RegressionState.PASS, facts.regression.values())
            self.assertNotIn(RegressionState.FAIL, facts.regression.values())
            self.assertNotIn(OracleState.EXHAUSTIVE, facts.oracle.values())

    def test_forged_scoped_declaration_cannot_become_landed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            declaration = EvidenceDeclaration(
                "evidence:forged",
                ("miner:G5",),
                EvidenceKind.GIT_SCOPED,
                None,
                None,
                None,
                GitAncestorCheck("a" * 40),
            )

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[:3] == ("show", "-s", "--format=%s"):
                    return "unrelated maintenance\n"
                if args[0] == "diff-tree":
                    return "src/other.rs\n"
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                with mock.patch("tools.parity_ledger.evidence.is_ancestor", return_value=True):
                    with self.assertRaises(LedgerError):
                        evaluate_evidence(repo, (declaration,), (gap(),))

    def test_missing_oracle_artifacts_do_not_erase_landed_implementation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text("", encoding="utf-8")
            missing = ArtifactRef("missing.bin", "f" * 64)
            declarations = (
                EvidenceDeclaration(
                    "evidence:git",
                    ("miner:G5",),
                    EvidenceKind.GIT_SCOPED,
                    None,
                    None,
                    None,
                    GitAncestorCheck("a" * 40),
                ),
                EvidenceDeclaration(
                    "evidence:artifact",
                    ("miner:G5",),
                    EvidenceKind.GAMEMD_VECTOR,
                    missing,
                    None,
                    Coverage(CoverageMode.SAMPLED, "one case"),
                    ArtifactHashCheck(),
                ),
                EvidenceDeclaration(
                    "evidence:bridge",
                    ("miner:G5",),
                    EvidenceKind.BRIDGE_TRACE,
                    None,
                    None,
                    Coverage(CoverageMode.SAMPLED, "one case"),
                    BridgeTraceCheck(missing, missing),
                ),
            )

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[:3] == ("show", "-s", "--format=%s"):
                    return "miner: G5 exact fix\n"
                if args[0] == "diff-tree":
                    return "src/miner.rs\n"
                if args[0] == "ls-tree":
                    return "src/miner.rs\n"
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                with mock.patch("tools.parity_ledger.evidence.is_ancestor", return_value=True):
                    facts = evaluate_evidence(repo, declarations, (gap(),))
            self.assertEqual(facts.implementation["miner:G5"], ImplementationState.LANDED)
            self.assertEqual(facts.oracle["miner:G5"], OracleState.INCOMPLETE)
            self.assertEqual(len(facts.diagnostics), 2)

    def test_deleted_changed_anchor_cannot_become_landed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            declaration = EvidenceDeclaration(
                "evidence:git",
                ("miner:G5",),
                EvidenceKind.GIT_SCOPED,
                None,
                None,
                None,
                GitAncestorCheck("a" * 40),
            )

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[:3] == ("show", "-s", "--format=%s"):
                    return "miner: G5 exact fix\n"
                if args[0] == "diff-tree":
                    return "src/miner.rs\n"
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                with mock.patch("tools.parity_ledger.evidence.is_ancestor", return_value=True):
                    facts = evaluate_evidence(repo, (declaration,), (gap(),))
            self.assertEqual(
                facts.implementation["miner:G5"],
                ImplementationState.STALE_MAPPING,
            )
            self.assertEqual(len(facts.diagnostics), 1)

    def test_commented_out_test_is_not_declared(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text(
                "// #[test]\n// fn fake_regression() {}\n",
                encoding="utf-8",
            )

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[0] == "log":
                    return "e" * 40 + "\tminer: G5 exact fix\n"
                if args[0] == "diff-tree":
                    return "src/miner.rs\n"
                if args[0] == "show":
                    return (
                        "+++ b/src/miner.rs\n"
                        "+// #[test]\n"
                        "+// fn fake_regression() {}\n"
                    )
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                declarations, _ = derive_evidence(repo, (gap(),))
            self.assertNotIn(
                EvidenceKind.REGRESSION_DECLARATION,
                {item.kind for item in declarations},
            )

    def test_block_commented_test_is_not_declared(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text(
                "/*\n#[test]\nfn fake_regression() {}\n*/\n",
                encoding="utf-8",
            )

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[0] == "log":
                    return "f" * 40 + "\tminer: G5 exact fix\n"
                if args[0] == "diff-tree":
                    return "src/miner.rs\n"
                if args[0] == "show" and len(args) == 2:
                    return "/*\n#[test]\nfn fake_regression() {}\n*/\n"
                if args[0] == "show":
                    return (
                        "+++ b/src/miner.rs\n"
                        "+#[test]\n"
                        "+fn fake_regression() {}\n"
                    )
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                declarations, _ = derive_evidence(repo, (gap(),))
            self.assertNotIn(
                EvidenceKind.REGRESSION_DECLARATION,
                {item.kind for item in declarations},
            )

    def test_nonancestor_scoped_commit_cannot_declare_regression(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text(
                "#[test]\nfn catches_gap() {}\n",
                encoding="utf-8",
            )
            commit = "a" * 40
            declarations = (
                EvidenceDeclaration(
                    "evidence:git",
                    ("miner:G5",),
                    EvidenceKind.GIT_SCOPED,
                    None,
                    None,
                    None,
                    GitAncestorCheck(commit),
                ),
                EvidenceDeclaration(
                    "evidence:test",
                    ("miner:G5",),
                    EvidenceKind.REGRESSION_DECLARATION,
                    None,
                    None,
                    None,
                    TestDeclaredCheck("src/miner.rs", "catches_gap", commit),
                ),
            )

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[:3] == ("show", "-s", "--format=%s"):
                    return "miner: G5 exact fix\n"
                if args[0] == "diff-tree":
                    return "src/miner.rs\n"
                if args[0] == "ls-tree":
                    return "src/miner.rs\n"
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                with mock.patch("tools.parity_ledger.evidence.is_ancestor", return_value=False):
                    facts = evaluate_evidence(repo, declarations, (gap(),))
            self.assertEqual(facts.implementation["miner:G5"], ImplementationState.NONE)
            self.assertEqual(facts.regression["miner:G5"], RegressionState.NONE)

    def test_recreated_untracked_anchor_cannot_become_landed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "src").mkdir()
            (repo / "src/miner.rs").write_text("recreated", encoding="utf-8")
            declaration = EvidenceDeclaration(
                "evidence:git",
                ("miner:G5",),
                EvidenceKind.GIT_SCOPED,
                None,
                None,
                None,
                GitAncestorCheck("a" * 40),
            )

            def fake_git(_repo: Path, args: tuple[str, ...]) -> str:
                if args[:3] == ("show", "-s", "--format=%s"):
                    return "miner: G5 remove old path\n"
                if args[0] == "diff-tree":
                    return "src/miner.rs\n"
                if args[0] == "ls-tree":
                    return ""
                raise AssertionError(args)

            with mock.patch("tools.parity_ledger.evidence.run_git", side_effect=fake_git):
                with mock.patch("tools.parity_ledger.evidence.is_ancestor", return_value=True):
                    facts = evaluate_evidence(repo, (declaration,), (gap(),))
            self.assertEqual(
                facts.implementation["miner:G5"],
                ImplementationState.STALE_MAPPING,
            )


if __name__ == "__main__":
    unittest.main()
