"""Tests for repository containment, source modes, and Git ancestry tri-state."""

from pathlib import Path
import subprocess
import tempfile
import unittest

from tools.parity_ledger.errors import LedgerError
from tools.parity_ledger.model import SourceFileLock, SourceLockDocument, SourceRole, SourceState, Tracking
from tools.parity_ledger.workspace import (
    declared_test_names,
    find_repo_root,
    is_ancestor,
    resolve_repo_path,
    source_states,
)


def _runner(returncode: int):
    def run(*_args, **_kwargs):
        return subprocess.CompletedProcess([], returncode, "", "fatal" if returncode >= 2 else "")
    return run


class WorkspaceTests(unittest.TestCase):
    def test_root_and_containment(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".git").mkdir()
            (root / "Cargo.toml").write_text("", encoding="utf-8")
            nested = root / "a/b"
            nested.mkdir(parents=True)
            self.assertEqual(find_repo_root(nested), root.resolve())
            self.assertEqual(resolve_repo_path(root, "a/b"), nested.resolve())
            with self.assertRaises(LedgerError):
                resolve_repo_path(root, "../escape")

    def test_ancestry_exit_zero_one_and_error(self) -> None:
        root = Path(".")
        self.assertTrue(is_ancestor(root, "a" * 40, runner=_runner(0)))
        self.assertFalse(is_ancestor(root, "a" * 40, runner=_runner(1)))
        with self.assertRaises(LedgerError):
            is_ancestor(root, "a" * 40, runner=_runner(128))

    def test_ci_never_requires_ignored_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            document = SourceLockDocument(
                "bootstrap",
                "tool",
                1,
                "0" * 64,
                (
                    SourceFileLock(
                        "ignored",
                        "core",
                        SourceRole.INVENTORY,
                        "docs/missing.md",
                        "1" * 64,
                        Tracking.IGNORED_LOCAL,
                        "adapter",
                        1,
                    ),
                ),
            )
            self.assertEqual(source_states(root, document, mode="ci"), {"ignored": SourceState.UNAVAILABLE})
            with self.assertRaises(LedgerError):
                source_states(root, document, mode="require")

    def test_test_declarations_exclude_comments_and_strings(self) -> None:
        text = '''
#[test]
fn active_test() {}
// #[test]
// fn line_comment() {}
/* outer /* nested */
#[test]
fn block_comment() {}
*/
const SAMPLE: &str = r#"
#[test]
fn raw_string() {}
"#;
#[cfg(any())]
#[test]
fn disabled_by_cfg() {}
macro_rules! dormant {
    () => {
        #[test]
        fn macro_body() {}
    };
}
'''
        self.assertEqual(declared_test_names(text), ("active_test",))


if __name__ == "__main__":
    unittest.main()
