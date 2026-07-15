"""Conservative declaration generation and closed evidence-check evaluation."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re

from .errors import Diagnostic, ExitCode, FailureCode, LedgerError
from .jsonio import sha256_bytes, sha256_file
from .model import (
    ArtifactHashCheck,
    BridgeTraceCheck,
    EvidenceDeclaration,
    EvidenceKind,
    GitAncestorCheck,
    ImplementationState,
    Obligation,
    OracleState,
    PathExistsCheck,
    RegressionState,
    TestDeclaredCheck,
)
from .workspace import (
    current_test_declared,
    declared_test_names,
    is_ancestor,
    resolve_repo_path,
    run_git,
)


_BARE_ID_RE = re.compile(
    r"(?<![A-Za-z0-9_])([GHMLS][1-9]\d*)(?![A-Za-z0-9_])"
)
@dataclass(frozen=True)
class EvaluatedEvidence:
    implementation: dict[str, ImplementationState]
    regression: dict[str, RegressionState]
    oracle: dict[str, OracleState]
    diagnostics: tuple[Diagnostic, ...]


def _evidence_id(kind: str, obligation_id: str, *parts: str) -> str:
    suffix = sha256_bytes("\0".join((kind, obligation_id, *parts)).encode("utf-8"))[:20]
    return f"evidence:{kind}:{suffix}"


def _changed_paths(repo: Path, commit: str) -> tuple[str, ...]:
    output = run_git(
        repo,
        (
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "--diff-filter=ACMRTUXB",
            "-r",
            commit,
        ),
    )
    return tuple(sorted({line.strip().replace("\\", "/") for line in output.splitlines() if line.strip()}))


def _added_tests(repo: Path, commit: str) -> tuple[tuple[str, str], ...]:
    patch = run_git(repo, ("show", "--format=", "--unified=0", commit, "--", "*.rs"))
    current_path: str | None = None
    added: dict[str, list[str]] = {}
    for line in patch.splitlines():
        if line.startswith("+++ b/"):
            current_path = line[6:]
            added.setdefault(current_path, [])
        elif current_path is not None and line.startswith("+") and not line.startswith("+++"):
            added[current_path].append(line[1:])
    result: set[tuple[str, str]] = set()
    for path, lines in added.items():
        candidates = set(declared_test_names("\n".join(lines)))
        if not candidates:
            continue
        commit_text = run_git(repo, ("show", f"{commit}:{path}"))
        active_at_commit = set(declared_test_names(commit_text))
        result.update((path, test_name) for test_name in candidates & active_at_commit)
    return tuple(sorted(result))


def derive_path_evidence(
    obligations: tuple[Obligation, ...],
) -> tuple[EvidenceDeclaration, ...]:
    """Generate the complete canonical declaration set for cited Rust anchors."""

    declarations: dict[str, EvidenceDeclaration] = {}
    for obligation in obligations:
        for anchor in obligation.rust_anchors:
            evidence = EvidenceDeclaration(
                _evidence_id("path", obligation.id, anchor),
                (obligation.id,),
                EvidenceKind.IMPLEMENTATION_ANCHOR,
                None,
                None,
                None,
                PathExistsCheck(anchor),
            )
            declarations[evidence.id] = evidence
    return tuple(sorted(declarations.values(), key=lambda item: item.id))


def derive_evidence(
    repo: Path,
    obligations: tuple[Obligation, ...],
) -> tuple[tuple[EvidenceDeclaration, ...], tuple[Diagnostic, ...]]:
    """Generate path and explicitly scoped Git/test declarations from repository facts."""

    declarations = {item.id: item for item in derive_path_evidence(obligations)}
    diagnostics: list[Diagnostic] = []
    by_local: dict[str, list[Obligation]] = {}
    for obligation in obligations:
        local_id = obligation.id.split(":", 1)[1]
        if re.fullmatch(r"[GHMLS][1-9]\d*", local_id):
            by_local.setdefault(local_id, []).append(obligation)

    log = run_git(repo, ("log", "--format=%H%x09%s", "HEAD"))
    linked_commits: dict[tuple[str, str], EvidenceKind] = {}
    for line in log.splitlines():
        if "\t" not in line:
            continue
        commit, subject = line.split("\t", 1)
        tokens = sorted(set(_BARE_ID_RE.findall(subject)))
        if not tokens:
            continue
        changed = set(_changed_paths(repo, commit))
        for token in tokens:
            candidates = [
                item
                for item in by_local.get(token, ())
                if set(item.rust_anchors) & changed
                and any(resolve_repo_path(repo, anchor).is_file() for anchor in item.rust_anchors)
            ]
            scoped = [
                item
                for item in candidates
                if re.search(
                    rf"(?i)(?<![A-Za-z0-9_]){re.escape(item.system)}\s*:\s*"
                    rf"{re.escape(token)}(?![A-Za-z0-9_])",
                    subject,
                )
            ]
            if len(scoped) != 1:
                if len(scoped) > 1:
                    diagnostics.append(
                        Diagnostic(
                            FailureCode.EVIDENCE_INVALID.value,
                            record_id=token,
                            message=f"ambiguous commit {commit} subject {subject!r}",
                            fatal=False,
                        )
                    )
                continue
            obligation = scoped[0]
            linked_commits[(commit, obligation.id)] = EvidenceKind.GIT_SCOPED

    tests_by_commit: dict[str, tuple[tuple[str, str], ...]] = {}
    for (commit, obligation_id), kind in sorted(linked_commits.items()):
        evidence = EvidenceDeclaration(
            _evidence_id("git", obligation_id, commit),
            (obligation_id,),
            kind,
            None,
            None,
            None,
            GitAncestorCheck(commit),
        )
        declarations[evidence.id] = evidence
        if kind is not EvidenceKind.GIT_SCOPED:
            continue
        if commit not in tests_by_commit:
            tests_by_commit[commit] = _added_tests(repo, commit)
        for path, test_name in tests_by_commit[commit]:
            if not current_test_declared(repo, path, test_name):
                continue
            test_evidence = EvidenceDeclaration(
                _evidence_id("test", obligation_id, commit, path, test_name),
                (obligation_id,),
                EvidenceKind.REGRESSION_DECLARATION,
                None,
                None,
                None,
                TestDeclaredCheck(path, test_name, commit),
            )
            declarations[test_evidence.id] = test_evidence
    return tuple(sorted(declarations.values(), key=lambda item: item.id)), tuple(sorted(diagnostics))


_IMPLEMENTATION_RANK = {
    ImplementationState.NONE: 0,
    ImplementationState.CANDIDATE: 1,
    ImplementationState.LANDED: 2,
    ImplementationState.STALE_MAPPING: 3,
}


def _raise_implementation(
    facts: dict[str, ImplementationState],
    obligation_id: str,
    state: ImplementationState,
) -> None:
    if _IMPLEMENTATION_RANK[state] > _IMPLEMENTATION_RANK[facts[obligation_id]]:
        facts[obligation_id] = state


def _artifact_matches(repo: Path, path: str, expected: str) -> bool:
    candidate = resolve_repo_path(repo, path)
    return candidate.is_file() and sha256_file(candidate) == expected


def _current_tracked_anchors(repo: Path, anchors: set[str]) -> set[str]:
    current_files = {
        anchor for anchor in anchors if resolve_repo_path(repo, anchor).is_file()
    }
    if not current_files:
        return set()
    output = run_git(
        repo,
        ("ls-tree", "-r", "--name-only", "HEAD", "--", *sorted(current_files)),
    )
    tracked = {
        line.strip().replace("\\", "/")
        for line in output.splitlines()
        if line.strip()
    }
    return current_files & tracked


def _validate_git_association(
    repo: Path,
    declaration: EvidenceDeclaration,
    obligation: Obligation,
    check: GitAncestorCheck,
) -> bool:
    local_id = obligation.id.split(":", 1)[1]
    changed_anchors: set[str] = set()
    if re.fullmatch(r"[GHMLS][1-9]\d*", local_id) is None:
        valid = False
    else:
        subject = run_git(repo, ("show", "-s", "--format=%s", check.commit)).strip()
        changed = set(_changed_paths(repo, check.commit))
        changed_anchors = set(obligation.rust_anchors) & changed
        anchor_match = bool(changed_anchors)
        scoped_match = re.search(
            rf"(?i)(?<![A-Za-z0-9_]){re.escape(obligation.system)}\s*:\s*"
            rf"{re.escape(local_id)}(?![A-Za-z0-9_])",
            subject,
        ) is not None
        valid = (
            anchor_match
            and scoped_match
            and declaration.kind is EvidenceKind.GIT_SCOPED
        )
    if not valid:
        raise LedgerError(
            ExitCode.VALIDATION_FAILED,
            [
                Diagnostic(
                    FailureCode.EVIDENCE_INVALID.value,
                    record_id=declaration.id,
                    field=obligation.id,
                    message="Git declaration fails subject-scope or changed-anchor revalidation",
                    fatal=True,
                )
            ],
        )
    return bool(_current_tracked_anchors(repo, changed_anchors))


def evaluate_evidence(
    repo: Path,
    declarations: tuple[EvidenceDeclaration, ...],
    obligations: tuple[Obligation, ...],
) -> EvaluatedEvidence:
    identifiers = {item.id for item in obligations}
    obligations_by_id = {item.id: item for item in obligations}
    implementation = {identifier: ImplementationState.NONE for identifier in identifiers}
    regression = {identifier: RegressionState.NONE for identifier in identifiers}
    oracle = {identifier: OracleState.NONE for identifier in identifiers}
    diagnostics: list[Diagnostic] = []
    scoped_commits = {
        (obligation_id, declaration.check.commit)
        for declaration in declarations
        if declaration.kind is EvidenceKind.GIT_SCOPED
        and isinstance(declaration.check, GitAncestorCheck)
        for obligation_id in declaration.obligations
    }
    added_tests_by_commit: dict[str, tuple[tuple[str, str], ...]] = {}
    ancestry_by_commit: dict[str, bool] = {}
    landed_scoped: set[tuple[str, str]] = set()
    for declaration in declarations:
        check = declaration.check
        if not isinstance(check, GitAncestorCheck):
            continue
        for obligation_id in declaration.obligations:
            current_anchor = _validate_git_association(
                repo,
                declaration,
                obligations_by_id[obligation_id],
                check,
            )
            if not current_anchor:
                _raise_implementation(
                    implementation,
                    obligation_id,
                    ImplementationState.STALE_MAPPING,
                )
                diagnostics.append(
                    Diagnostic(
                        FailureCode.CURRENT_ANCHOR_MISSING.value,
                        record_id=obligation_id,
                        field=declaration.id,
                        message=(
                            "Git declaration's changed Rust anchor is not a current tracked file"
                        ),
                        fatal=False,
                    )
                )
                continue
            if check.commit not in ancestry_by_commit:
                ancestry_by_commit[check.commit] = is_ancestor(repo, check.commit)
            if ancestry_by_commit[check.commit]:
                _raise_implementation(
                    implementation,
                    obligation_id,
                    ImplementationState.LANDED,
                )
                landed_scoped.add((obligation_id, check.commit))

    for declaration in declarations:
        for obligation_id in declaration.obligations:
            check = declaration.check
            if isinstance(check, PathExistsCheck):
                state = (
                    ImplementationState.CANDIDATE
                    if resolve_repo_path(repo, check.path).is_file()
                    else ImplementationState.STALE_MAPPING
                )
                _raise_implementation(implementation, obligation_id, state)
                if state is ImplementationState.STALE_MAPPING:
                    diagnostics.append(
                        Diagnostic(
                            FailureCode.CURRENT_ANCHOR_MISSING.value,
                            source_path=check.path,
                            record_id=obligation_id,
                            message="declared path is missing",
                            fatal=False,
                        )
                    )
            elif isinstance(check, GitAncestorCheck):
                continue
            elif isinstance(check, TestDeclaredCheck):
                if (obligation_id, check.commit) not in scoped_commits:
                    raise LedgerError(
                        ExitCode.VALIDATION_FAILED,
                        [
                            Diagnostic(
                                FailureCode.EVIDENCE_INVALID.value,
                                record_id=declaration.id,
                                field=obligation_id,
                                message=(
                                    "regression declaration lacks a matching scoped Git commit"
                                ),
                                fatal=True,
                            )
                        ],
                    )
                if (obligation_id, check.commit) not in landed_scoped:
                    continue
                if check.commit not in added_tests_by_commit:
                    added_tests_by_commit[check.commit] = _added_tests(repo, check.commit)
                if (check.path, check.test_name) not in added_tests_by_commit[check.commit]:
                    raise LedgerError(
                        ExitCode.VALIDATION_FAILED,
                        [
                            Diagnostic(
                                FailureCode.EVIDENCE_INVALID.value,
                                record_id=declaration.id,
                                field=obligation_id,
                                message="test was not added by its declared scoped Git commit",
                                fatal=True,
                            )
                        ],
                    )
                if current_test_declared(repo, check.path, check.test_name):
                    regression[obligation_id] = RegressionState.DECLARED
                else:
                    _raise_implementation(
                        implementation,
                        obligation_id,
                        ImplementationState.STALE_MAPPING,
                    )
                    diagnostics.append(
                        Diagnostic(
                            FailureCode.CURRENT_ANCHOR_MISSING.value,
                            source_path=check.path,
                            record_id=obligation_id,
                            field=check.test_name,
                            message="declared test is missing",
                            fatal=False,
                        )
                    )
            elif isinstance(check, ArtifactHashCheck):
                oracle[obligation_id] = OracleState.INCOMPLETE
                matches = declaration.artifact is not None and _artifact_matches(
                    repo,
                    declaration.artifact.path,
                    declaration.artifact.sha256,
                )
                if not matches:
                    artifact_path = declaration.artifact.path if declaration.artifact is not None else ""
                    diagnostics.append(
                        Diagnostic(
                            FailureCode.CURRENT_ANCHOR_MISSING.value,
                            source_path=artifact_path,
                            record_id=obligation_id,
                            field=declaration.id,
                            message="declared oracle artifact is missing or hash-mismatched",
                            fatal=False,
                        )
                    )
            elif isinstance(check, BridgeTraceCheck):
                oracle[obligation_id] = OracleState.INCOMPLETE
                matches = _artifact_matches(
                    repo,
                    check.left_trace.path,
                    check.left_trace.sha256,
                ) and _artifact_matches(
                    repo,
                    check.right_trace.path,
                    check.right_trace.sha256,
                )
                if not matches:
                    diagnostics.append(
                        Diagnostic(
                            FailureCode.CURRENT_ANCHOR_MISSING.value,
                            source_path=check.left_trace.path,
                            record_id=obligation_id,
                            field=declaration.id,
                            message="declared bridge traces are missing or hash-mismatched",
                            fatal=False,
                        )
                    )
    return EvaluatedEvidence(implementation, regression, oracle, tuple(sorted(diagnostics)))
