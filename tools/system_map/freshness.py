"""Live Git freshness for mapped Rust production surfaces."""

from __future__ import annotations

from pathlib import Path
import re
import subprocess

from .model import COMMIT_RE


def repository_state(repo: Path) -> dict:
    """Return deterministic read-only Git state for report provenance."""

    head = _git(repo, "rev-parse", "--verify", "HEAD")
    branch = _git(repo, "branch", "--show-current")
    status = _git(repo, "status", "--porcelain=v1", "--untracked-files=all")
    return {
        "branch": branch.strip() or "(detached)",
        "dirty_paths": sorted(
            _porcelain_paths(status) if status is not None else []
        ),
        "head": head.strip().lower() if head else "UNAVAILABLE",
    }


def collect_system_surfaces(topology: dict) -> dict[str, list[dict]]:
    """Collect explicit Rust paths without guessing ownership from prose."""

    result: dict[str, list[dict]] = {}
    systems = topology.get("systems", {})
    if isinstance(systems, dict):
        for system_id, annotation in systems.items():
            if not isinstance(annotation, dict):
                continue
            default_coverage = annotation.get(
                "rust_surface_coverage", "representative"
            )
            _add_surfaces(
                result,
                system_id,
                annotation.get("rust_surfaces", []),
                default_coverage=default_coverage,
            )

    loops = topology.get("loops", {})
    if isinstance(loops, dict):
        for loop in loops.values():
            if not isinstance(loop, dict):
                continue
            owner = loop.get("owner")
            _add_surfaces(
                result,
                owner,
                loop.get("rust_touchpoints", []),
                default_coverage="representative",
            )
            stages = loop.get("ordered_stages", loop.get("stages", []))
            if not isinstance(stages, list):
                continue
            for stage in stages:
                if not isinstance(stage, dict):
                    continue
                system_id = stage.get(
                    "system", stage.get("system_id", stage.get("id"))
                )
                _add_surfaces(
                    result,
                    system_id,
                    stage.get(
                        "rust_surfaces", stage.get("rust_entrypoints", [])
                    ),
                    default_coverage=stage.get(
                        "rust_surface_coverage", "representative"
                    ),
                )

    edges = topology.get("edges", [])
    if isinstance(edges, list):
        for edge in edges:
            if not isinstance(edge, dict) or edge.get("plane") != "rust":
                continue
            observed = edge.get("observed_at_commit")
            surfaces = []
            evidence = edge.get("evidence", [])
            if isinstance(evidence, list):
                for item in evidence:
                    path = item.get("path") if isinstance(item, dict) else item
                    normalized = _rust_evidence_path(path)
                    if normalized is not None:
                        surfaces.append(
                            {
                                "coverage": "representative",
                                "observed_at_commit": observed,
                                "path": normalized,
                            }
                        )
            for system_id in {edge.get("from"), edge.get("to")}:
                _add_surfaces(
                    result,
                    system_id,
                    surfaces,
                    default_coverage="representative",
                )

    for system_id, surfaces in result.items():
        unique: dict[tuple[str, str, str], dict] = {}
        for surface in surfaces:
            key = (
                surface["path"],
                surface.get("observed_at_commit", ""),
                surface.get("coverage", "representative"),
            )
            unique[key] = surface
        result[system_id] = [
            unique[key] for key in sorted(unique, key=lambda item: item)
        ]
    return result


def build_freshness(
    repo: Path,
    registry: dict,
    topology: dict,
) -> dict[str, dict]:
    """Compare each mapped system against topology and matrix observations."""

    surfaces_by_system = collect_system_surfaces(topology)
    baseline_commit = registry.get("baseline_rust_snapshot")
    inspector = _GitInspector(repo)
    result: dict[str, dict] = {}
    systems = registry.get("systems", {})
    for system_id in sorted(systems):
        surfaces = surfaces_by_system.get(system_id, [])
        result[system_id] = {
            "baseline_status_freshness": compare_surfaces(
                repo,
                surfaces,
                baseline_commit,
                inspector=inspector,
                prefer_surface_commits=False,
            ),
            "rust_mapping_freshness": compare_surfaces(
                repo,
                surfaces,
                None,
                inspector=inspector,
                prefer_surface_commits=True,
            ),
        }
    return result


def compare_surfaces(
    repo: Path,
    surfaces: list[dict],
    default_commit: object,
    *,
    inspector: _GitInspector | None = None,
    prefer_surface_commits: bool = True,
) -> dict:
    """Classify one mapping without treating representative paths as exhaustive."""

    if not surfaces:
        return {
            "changed_paths": [],
            "dirty_paths": [],
            "missing_paths": [],
            "observed_at_commits": [],
            "paths": [],
            "reasons": ["no explicit Rust production surface is mapped"],
            "state": "UNMAPPED",
        }

    active_inspector = inspector or _GitInspector(repo)
    paths = sorted(
        {
            surface["path"]
            for surface in surfaces
            if isinstance(surface.get("path"), str)
        }
    )
    missing = sorted(path for path in paths if not (repo / path).exists())
    commits = sorted(
        {
            str(
                (
                    surface.get("observed_at_commit", default_commit)
                    if prefer_surface_commits
                    else default_commit
                )
                or ""
            ).lower()
            for surface in surfaces
        }
    )
    invalid_commits = [
        commit
        for commit in commits
        if not COMMIT_RE.fullmatch(commit)
        or not active_inspector.commit_exists(commit)
    ]
    dirty = active_inspector.dirty_paths(paths)
    changed: set[str] = set()
    divergent: list[str] = []
    if not invalid_commits:
        by_commit: dict[str, list[str]] = {}
        for surface in surfaces:
            commit = str(
                (
                    surface.get("observed_at_commit", default_commit)
                    if prefer_surface_commits
                    else default_commit
                )
                or ""
            ).lower()
            by_commit.setdefault(commit, []).append(surface["path"])
        for commit, commit_paths in sorted(by_commit.items()):
            if not active_inspector.is_ancestor(commit):
                divergent.append(commit)
                continue
            changed.update(
                active_inspector.changed_paths(commit, commit_paths)
            )

    reasons: list[str] = []
    if missing:
        state = "MISSING"
        reasons.append("one or more mapped paths do not exist")
    elif dirty:
        state = "STALE"
        reasons.append("mapped paths have uncommitted or untracked changes")
    elif invalid_commits:
        state = "UNRESOLVED"
        reasons.append("one or more observation commits are unavailable")
    elif divergent:
        state = "DIVERGED"
        reasons.append("observation commit is not an ancestor of HEAD")
    elif changed:
        state = "STALE"
        reasons.append("mapped paths changed after the observation commit")
    elif all(
        surface.get("coverage", "representative") == "exhaustive"
        for surface in surfaces
    ):
        state = "FRESH"
        reasons.append("all exhaustively mapped paths are unchanged")
    else:
        state = "UNRESOLVED"
        reasons.append(
            "representative paths are unchanged, but coverage is not exhaustive"
        )

    return {
        "changed_paths": sorted(changed),
        "dirty_paths": sorted(dirty),
        "missing_paths": missing,
        "observed_at_commits": commits,
        "paths": paths,
        "reasons": reasons,
        "state": state,
    }


def _add_surfaces(
    result: dict[str, list[dict]],
    system_id: object,
    surfaces: object,
    *,
    default_coverage: object,
) -> None:
    if not isinstance(system_id, str) or not isinstance(surfaces, list):
        return
    for surface in surfaces:
        if isinstance(surface, str):
            normalized = {
                "coverage": (
                    default_coverage
                    if default_coverage in {"representative", "exhaustive"}
                    else "representative"
                ),
                "path": surface,
            }
        elif isinstance(surface, dict) and isinstance(
            surface.get("path"), str
        ):
            normalized = {
                "coverage": surface.get(
                    "coverage",
                    default_coverage
                    if default_coverage in {"representative", "exhaustive"}
                    else "representative",
                ),
                "path": surface["path"],
            }
            if isinstance(surface.get("observed_at_commit"), str):
                normalized["observed_at_commit"] = surface[
                    "observed_at_commit"
                ]
        else:
            continue
        result.setdefault(system_id, []).append(normalized)


def _git(repo: Path, *args: str) -> str | None:
    try:
        completed = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
    except OSError:
        return None
    return completed.stdout if completed.returncode == 0 else None


class _GitInspector:
    """Cache repo-wide Git facts and intersect them with mapped surfaces."""

    def __init__(self, repo: Path):
        self.repo = repo
        status = _git(
            repo,
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--",
            "src",
            "tests",
        )
        self._dirty = _porcelain_paths(status or "")
        self._exists: dict[str, bool] = {}
        self._ancestor: dict[str, bool] = {}
        self._changed: dict[str, set[str]] = {}

    def commit_exists(self, commit: str) -> bool:
        if commit not in self._exists:
            try:
                completed = subprocess.run(
                    ["git", "cat-file", "-e", f"{commit}^{{commit}}"],
                    cwd=self.repo,
                    check=False,
                    capture_output=True,
                )
                self._exists[commit] = completed.returncode == 0
            except OSError:
                self._exists[commit] = False
        return self._exists[commit]

    def is_ancestor(self, commit: str) -> bool:
        if commit not in self._ancestor:
            try:
                completed = subprocess.run(
                    ["git", "merge-base", "--is-ancestor", commit, "HEAD"],
                    cwd=self.repo,
                    check=False,
                    capture_output=True,
                )
                self._ancestor[commit] = completed.returncode == 0
            except OSError:
                self._ancestor[commit] = False
        return self._ancestor[commit]

    def changed_paths(self, commit: str, paths: list[str]) -> list[str]:
        if commit not in self._changed:
            output = _git(
                self.repo,
                "diff",
                "--name-only",
                f"{commit}..HEAD",
                "--",
                "src",
                "tests",
            )
            self._changed[commit] = {
                line.strip().replace("\\", "/")
                for line in (output or "").splitlines()
                if line.strip()
            }
        return sorted(_matching_paths(paths, self._changed[commit]))

    def dirty_paths(self, paths: list[str]) -> list[str]:
        return sorted(_matching_paths(paths, self._dirty))


def _matching_paths(
    mapped_paths: list[str], changed_paths: set[str]
) -> set[str]:
    matches: set[str] = set()
    for changed in changed_paths:
        for mapped in mapped_paths:
            normalized = mapped.rstrip("/")
            if changed == normalized or changed.startswith(normalized + "/"):
                matches.add(changed)
                break
    return matches


def _rust_evidence_path(value: object) -> str | None:
    if not isinstance(value, str):
        return None
    candidate = value.strip().replace("\\", "/")
    match = re.fullmatch(r"(.+?\.rs)(?::\d+(?:-\d+)?)?", candidate)
    if match is None:
        return None
    path = match.group(1)
    if not path.startswith(("src/", "tests/")):
        return None
    return path


def _porcelain_paths(output: str) -> set[str]:
    paths: set[str] = set()
    for line in output.splitlines():
        if len(line) < 4:
            continue
        value = line[3:].strip()
        if " -> " in value:
            value = value.split(" -> ", 1)[1]
        if value.startswith('"') and value.endswith('"'):
            value = value[1:-1]
        paths.add(value.replace("\\", "/"))
    return paths
