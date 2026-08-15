#!/usr/bin/env python3
"""Collect lexical Rust review candidates without assigning verdicts.

The collector is intentionally conservative: it discovers stable, sorted
candidates and leaves type, call-path, authority, and severity decisions to the
reviewing agent.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence


FOCUS_NAMES = (
    "determinism",
    "architecture",
    "state",
    "safety",
    "performance",
    "ownership",
    "structure",
)


@dataclass(frozen=True)
class Rule:
    rule_id: str
    category: str
    focus: str
    scope: str
    pattern: str
    description: str

    @property
    def regex(self) -> re.Pattern[str]:
        return re.compile(self.pattern)


RULES: tuple[Rule, ...] = (
    Rule(
        "DET-001",
        "determinism",
        "determinism",
        "sim",
        r"\b(?:f32|f64)\b|(?<![\w.])\d+\.\d+(?:[eE][+-]?\d+)?(?:f32|f64)?\b",
        "Host floating-point type or conversion in simulation scope",
    ),
    Rule(
        "DET-002",
        "determinism",
        "determinism",
        "sim",
        r"\b(?:HashMap|HashSet|DefaultHasher)\b",
        "Unordered collection or unstable standard hash; verify observable use",
    ),
    Rule(
        "DET-003",
        "determinism",
        "determinism",
        "sim",
        (
            r"\b(?:thread_rng|OsRng|getrandom|from_entropy)\b"
            r"|rand\s*::\s*(?:random|rng)\b"
            r"|(?:SystemTime|Instant|Utc|Local)\s*::\s*now\b"
            r"|std\s*::\s*(?:env|fs\s*::\s*read_dir)\b"
        ),
        "External entropy, clock, environment, or filesystem-order source",
    ),
    Rule(
        "DET-004",
        "determinism",
        "determinism",
        "sim",
        (
            r"\bas\s+(?:u8|u16|u32|u64|u128|usize"
            r"|i8|i16|i32|i64|i128|isize)\b"
        ),
        "Integer cast requiring source-type and range verification",
    ),
    Rule(
        "DET-005",
        "determinism",
        "determinism",
        "sim",
        (
            r"#\s*\[\s*cfg(?:_attr)?\s*\([^\]]*"
            r"\b(?:target_arch|target_os|target_env|target_pointer_width"
            r"|target_endian|debug_assertions|feature)\b"
            r"|\bcfg!\s*\([^\)]*\b(?:target_arch|target_os|target_env"
            r"|target_pointer_width|target_endian|debug_assertions|feature)\b"
        ),
        "Production-relevant conditional behavior in simulation scope",
    ),
    Rule(
        "DET-006",
        "determinism",
        "determinism",
        "sim",
        (
            r"(?:std\s*::\s*)?thread\s*::\s*spawn\b"
            r"|\b(?:rayon|par_iter|par_iter_mut|into_par_iter|par_bridge"
            r"|find_any|reduce_with|tokio\s*::\s*spawn)\b"
            r"|\b(?:AtomicBool|AtomicI(?:8|16|32|64|128|size)"
            r"|AtomicU(?:8|16|32|64|128|size)|Ordering\s*::\s*Relaxed)\b"
            r"|\b(?:MaybeUninit|assume_init(?:_read|_drop|_mut|_ref)?)\b"
        ),
        "Concurrency, atomic, or initialization primitive needing semantic review",
    ),
    Rule(
        "DET-007",
        "determinism",
        "determinism",
        "sim",
        r"\bBinaryHeap\b|\.sort(?:_unstable)?(?:_by|_by_key)?\s*\(",
        "Ordering operation requiring tie-break and downstream-order review",
    ),
    Rule(
        "DET-008",
        "determinism",
        "state",
        "sim",
        r"\b(?:SimRng|main_rng|scenario_rng|mapgen_rng)\b",
        "Simulation RNG ownership or draw-order touchpoint",
    ),
    Rule(
        "STATE-001",
        "state",
        "state",
        "sim",
        r"#\s*\[\s*serde\s*\([^\]]*\bskip\b|SNAPSHOT_VERSION\b",
        "Serialization/hash/snapshot coverage touchpoint",
    ),
    Rule(
        "STATE-002",
        "state",
        "state",
        "sim",
        (
            r"\b(?:LogicVector|logic_vector|pending_delete|unlimbo|limbo"
            r"|reveal|conceal)\b"
            r"|(?:entities|occupancy)\s*\.\s*(?:insert|remove)\s*\("
        ),
        "Lifecycle, scheduler, entity-store, or occupancy authority touchpoint",
    ),
    Rule(
        "STATE-003",
        "state",
        "state",
        "sim",
        r"\b(?:advance_tick|advance_master_frame|commit_frame|flush_pending)\b",
        "Tick-spine or phase-boundary touchpoint",
    ),
    Rule(
        "ARCH-001",
        "architecture",
        "architecture",
        "sim",
        (
            r"crate\s*::\s*(?:(?:render|ui|sidebar|audio|net)\b"
            r"|\{[^\}]*\b(?:render|ui|sidebar|audio|net)\b)"
            r"|(?:super\s*::\s*)+(?:render|ui|sidebar|audio|net)\b"
            r"|(?:super\s*::\s*)+\{[^\}]*\b(?:render|ui|sidebar|audio|net)\b"
        ),
        "Reference to a layer forbidden beneath simulation",
    ),
    Rule(
        "ARCH-002",
        "architecture",
        "architecture",
        "sim",
        (
            r"\b(?:screen_[xy]|pixel_[xy]|shp_name|wgpu|egui|glam"
            r"|Texture|Sprite|AudioSink)\b"
        ),
        "Presentation-oriented state or type under simulation",
    ),
    Rule(
        "COORD-001",
        "determinism",
        "determinism",
        "sim",
        (
            r"\b(?:lepton|facing_byte|screen_[xy]|pixel_[xy])\b"
            r"|(?:<<|>>)\s*8\b|(?:\*|/)\s*256\b|\b256\s*(?:\*|/)"
        ),
        "Coordinate, unit, sign, shift, or rounding boundary",
    ),
    Rule(
        "SAFE-001",
        "safety",
        "safety",
        "all",
        (
            r"\bunsafe\s*(?:\{|fn\b|impl\b|trait\b|extern\b)"
            r"|#\s*\[\s*unsafe\s*\("
            r"|#\s*\[\s*(?:no_mangle|link_section|export_name)\b"
            r"|\bextern\s+\{"
            r"|\b(?:transmute|from_raw_parts|get_unchecked|set_len"
            r"|assume_init(?:_read|_drop|_mut|_ref)?)\b"
        ),
        "Unsafe operation or contract requiring invariant verification",
    ),
    Rule(
        "SAFE-002",
        "safety",
        "safety",
        "all",
        r"\b(?:panic|todo|unimplemented|unreachable)!\s*\(",
        "Production panic or placeholder candidate",
    ),
    Rule(
        "ERR-001",
        "error-handling",
        "safety",
        "all",
        r"\.(?:unwrap|expect)\s*\(",
        "Fallible value requiring input-boundary verification",
    ),
    Rule(
        "ERR-002",
        "error-handling",
        "safety",
        "all",
        r"\banyhow\s*::|use\s+anyhow\b",
        "Anyhow use requiring application-versus-library boundary review",
    ),
    Rule(
        "OWN-001",
        "ownership",
        "ownership",
        "all",
        r"&\s*(?:String|Vec\s*<)",
        "Borrowed concrete container; verify whether a view type is sufficient",
    ),
    Rule(
        "OWN-002",
        "ownership",
        "ownership",
        "all",
        r"^\s*(?:pub(?:\s*\([^\)]*\))?\s+)?[A-Za-z_]\w*\s*:\s*String\b",
        "Owned string field; inspect identity, mutability, and clone frequency",
    ),
    Rule(
        "OWN-003",
        "ownership",
        "state",
        "sim",
        r"\bRc\s*<\s*RefCell\b|\bRc\s*::\s*new\s*\(\s*RefCell\b",
        "Shared interior mutability; inspect ownership and reentrancy",
    ),
    Rule(
        "PERF-001",
        "performance",
        "performance",
        "all",
        (
            r"\bVec\s*::\s*with_capacity\s*\(|\bvec!\s*\["
            r"|\.collect\s*::\s*<\s*Vec\b|\.to_vec\s*\("
            r"|\.to_(?:string|owned)\s*\(|\bString\s*::\s*from\s*\("
            r"|\bformat!\s*\(|\bBox\s*::\s*new\s*\("
        ),
        "Possible allocation; prove hot-path reachability and multiplicity",
    ),
    Rule(
        "PERF-002",
        "performance",
        "performance",
        "all",
        r"\.clone\s*\(",
        "Clone candidate; resolve cloned type and call frequency",
    ),
    Rule(
        "MODERN-001",
        "structure",
        "structure",
        "all",
        r"\b(?:lazy_static|once_cell)\b",
        "Legacy lazy/once initialization dependency or macro",
    ),
)

SPECIAL_RULE_DESCRIPTIONS = {
    "STRUCT-001": "Module does not open with an inner doc comment",
    "STRUCT-002": "File exceeds the cohesion review cue",
}


@dataclass(frozen=True)
class Candidate:
    rule_id: str
    category: str
    path: str
    line: int
    text: str
    hints: tuple[str, ...]

    def as_dict(self) -> dict[str, object]:
        return {
            "kind": "candidate",
            "rule_id": self.rule_id,
            "category": self.category,
            "path": self.path,
            "line": self.line,
            "text": self.text,
            "hints": list(self.hints),
        }


def run_checked(
    args: Sequence[str], cwd: Path, *, allow_failure: bool = False
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        args,
        cwd=cwd,
        text=True,
        encoding="utf-8",
        errors="replace",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0 and not allow_failure:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"{' '.join(args)} failed ({result.returncode}): {detail}")
    return result


def find_repo_root(start: Path) -> Path:
    result = run_checked(("git", "rev-parse", "--show-toplevel"), start)
    return Path(result.stdout.strip()).resolve()


def is_within(path: Path, parent: Path) -> bool:
    try:
        path.relative_to(parent)
        return True
    except ValueError:
        return False


def resolve_target(root: Path, raw_target: str) -> Path:
    candidate = Path(raw_target)
    if not candidate.is_absolute():
        candidate = root / candidate
    candidate = candidate.resolve()
    if not is_within(candidate, root):
        raise ValueError(f"target escapes repository root: {raw_target}")
    if not candidate.exists():
        raise ValueError(f"target does not exist: {raw_target}")
    if candidate.is_file() and candidate.suffix != ".rs":
        raise ValueError(f"target file is not Rust: {raw_target}")
    return candidate


def rel_posix(path: Path, root: Path) -> str:
    return path.resolve().relative_to(root).as_posix()


def discover_rust_files(root: Path, target: Path) -> list[Path]:
    if shutil.which("rg") is None:
        raise RuntimeError("ripgrep (rg) is required for Rust file discovery")
    if target.is_file():
        return [target]

    result = run_checked(
        ("rg", "--files", "-g", "*.rs", str(target)),
        root,
        allow_failure=True,
    )
    if result.returncode == 1 and not result.stderr.strip():
        return []
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"rg file discovery failed ({result.returncode}): {detail}")
    paths: list[Path] = []
    for raw in result.stdout.splitlines():
        path = Path(raw)
        if not path.is_absolute():
            path = root / path
        path = path.resolve()
        if path.is_file() and path.suffix == ".rs" and is_within(path, target):
            paths.append(path)
    return sorted(set(paths), key=lambda item: rel_posix(item, root).casefold())


def changed_rust_paths(root: Path, base: str) -> set[str]:
    diff = run_checked(
        ("git", "diff", "--name-only", "--diff-filter=ACMR", base, "--"),
        root,
    )
    untracked = run_checked(
        ("git", "ls-files", "--others", "--exclude-standard", "--", "*.rs"),
        root,
    )
    changed: set[str] = set()
    for raw in (*diff.stdout.splitlines(), *untracked.stdout.splitlines()):
        normalized = raw.strip().replace("\\", "/")
        if normalized.endswith(".rs"):
            changed.add(normalized)
    return changed


def is_named_test_file(relative: str) -> bool:
    path = Path(relative)
    lowered = [part.casefold() for part in path.parts]
    stem = path.stem.casefold()
    return (
        "tests" in lowered
        or stem in {"test", "tests"}
        or stem.endswith("_test")
        or stem.endswith("_tests")
    )


def sanitize_lines(lines: Sequence[str]) -> list[str]:
    """Remove ordinary comments and string bodies while retaining line shape."""

    sanitized: list[str] = []
    block_depth = 0
    for line in lines:
        output: list[str] = []
        index = 0
        in_string = False
        escaped = False
        while index < len(line):
            char = line[index]
            nxt = line[index + 1] if index + 1 < len(line) else ""

            if block_depth:
                if char == "/" and nxt == "*":
                    block_depth += 1
                    output.extend((" ", " "))
                    index += 2
                elif char == "*" and nxt == "/":
                    block_depth -= 1
                    output.extend((" ", " "))
                    index += 2
                else:
                    output.append(" ")
                    index += 1
                continue

            if in_string:
                output.append(" ")
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    in_string = False
                index += 1
                continue

            if char == "/" and nxt == "/":
                output.extend(" " * (len(line) - index))
                break
            if char == "/" and nxt == "*":
                block_depth = 1
                output.extend((" ", " "))
                index += 2
                continue
            if char == '"':
                in_string = True
                output.append(" ")
                index += 1
                continue

            output.append(char)
            index += 1
        sanitized.append("".join(output))
    return sanitized


TEST_ATTRIBUTE_RE = re.compile(r"#\s*\[\s*(?:cfg\s*\(\s*test\s*\)|test)\s*\]")


def probable_test_flags(
    relative: str, raw_lines: Sequence[str], code_lines: Sequence[str]
) -> list[bool]:
    if is_named_test_file(relative):
        return [True] * len(raw_lines)

    flags = [False] * len(raw_lines)
    depth = 0
    test_scope_bases: list[int] = []
    pending_test_item = False

    for index, code in enumerate(code_lines):
        active_before = bool(test_scope_bases)
        if TEST_ATTRIBUTE_RE.search(code):
            pending_test_item = True
            flags[index] = True

        if active_before or pending_test_item:
            flags[index] = True

        opens = code.count("{")
        closes = code.count("}")
        if pending_test_item and opens:
            test_scope_bases.append(depth)
            pending_test_item = False
        elif pending_test_item and ";" in code:
            pending_test_item = False

        depth += opens - closes
        while test_scope_bases and depth <= test_scope_bases[-1]:
            test_scope_bases.pop()

    return flags


def rule_applies(rule: Rule, relative: str, profile: str) -> bool:
    if rule.scope == "all":
        return True
    if profile == "sim":
        return True
    if profile == "general":
        return False
    return relative.startswith("src/sim/")


def collect_regex_candidates(
    root: Path,
    files: Iterable[Path],
    rules: Sequence[Rule],
    profile: str,
) -> list[Candidate]:
    candidates: list[Candidate] = []
    compiled = [(rule, rule.regex) for rule in rules]

    for path in files:
        relative = rel_posix(path, root)
        raw_lines = path.read_text(encoding="utf-8-sig", errors="replace").splitlines()
        code_lines = sanitize_lines(raw_lines)
        test_flags = probable_test_flags(relative, raw_lines, code_lines)
        multiline_rules = [
            (rule, regex)
            for rule, regex in compiled
            if rule.rule_id == "ARCH-001"
            and rule_applies(rule, relative, profile)
        ]
        line_rules = [
            (rule, regex)
            for rule, regex in compiled
            if rule.rule_id != "ARCH-001"
        ]

        for line_number, (raw, code) in enumerate(
            zip(raw_lines, code_lines), start=1
        ):
            if not code.strip():
                continue
            for rule, regex in line_rules:
                if not rule_applies(rule, relative, profile):
                    continue
                if regex.search(code):
                    hints: list[str] = []
                    if test_flags[line_number - 1]:
                        hints.append("probable-test-context")
                    candidates.append(
                        Candidate(
                            rule.rule_id,
                            rule.category,
                            relative,
                            line_number,
                            raw.strip()[:500],
                            tuple(hints),
                        )
                    )

        joined_code = "\n".join(code_lines)
        for rule, regex in multiline_rules:
            for match in regex.finditer(joined_code):
                start_line = joined_code.count("\n", 0, match.start()) + 1
                end_line = joined_code.count("\n", 0, match.end()) + 1
                hints: list[str] = []
                if any(test_flags[start_line - 1 : end_line]):
                    hints.append("probable-test-context")
                excerpt = " ".join(
                    line.strip()
                    for line in raw_lines[start_line - 1 : end_line]
                    if line.strip()
                )
                candidates.append(
                    Candidate(
                        rule.rule_id,
                        rule.category,
                        relative,
                        start_line,
                        excerpt[:500],
                        tuple(hints),
                    )
                )

    return candidates


def collect_structure_candidates(
    root: Path, files: Iterable[Path]
) -> list[Candidate]:
    candidates: list[Candidate] = []
    for path in files:
        relative = rel_posix(path, root)
        lines = path.read_text(encoding="utf-8-sig", errors="replace").splitlines()
        code_lines = sanitize_lines(lines)
        test_flags = probable_test_flags(relative, lines, code_lines)
        first_line = 1
        first_text = ""
        for index, line in enumerate(lines, start=1):
            if line.strip():
                first_line = index
                first_text = line.strip()
                break
        if first_text and not first_text.startswith("//!"):
            candidates.append(
                Candidate(
                    "STRUCT-001",
                    "structure",
                    relative,
                    first_line,
                    first_text[:500],
                    ("verify-generated-or-special-module",),
                )
            )
        production_scope_lines = sum(
            1 for is_probable_test in test_flags if not is_probable_test
        )
        if production_scope_lines > 600:
            candidates.append(
                Candidate(
                    "STRUCT-002",
                    "structure",
                    relative,
                    1,
                    (
                        f"{production_scope_lines} probable production-scope lines "
                        f"({len(lines)} total); review growth and cohesion, not size alone"
                    ),
                    (
                        "size-is-review-cue",
                        "inline-test-scope-detection-is-approximate",
                    ),
                )
            )
    return candidates


def deduplicate_and_sort(candidates: Iterable[Candidate]) -> list[Candidate]:
    unique = {
        (item.rule_id, item.path, item.line, item.text, item.hints): item
        for item in candidates
    }
    return sorted(
        unique.values(),
        key=lambda item: (
            item.path.casefold(),
            item.line,
            item.rule_id,
            item.text,
        ),
    )


def filter_test_candidates(
    candidates: Iterable[Candidate], include_tests: bool
) -> list[Candidate]:
    if include_tests:
        return list(candidates)
    return [
        item
        for item in candidates
        if "probable-test-context" not in item.hints
    ]


def repo_snapshot(root: Path) -> tuple[str, bool]:
    head = run_checked(("git", "rev-parse", "--short=12", "HEAD"), root)
    status = run_checked(("git", "status", "--porcelain"), root)
    return head.stdout.strip(), bool(status.stdout.strip())


def selected_rules(
    focuses: Sequence[str], requested_ids: Sequence[str]
) -> tuple[Rule, ...]:
    requested = set(requested_ids)
    if requested:
        known = {rule.rule_id for rule in RULES} | set(SPECIAL_RULE_DESCRIPTIONS)
        unknown = sorted(requested - known)
        if unknown:
            raise ValueError(f"unknown rule id(s): {', '.join(unknown)}")
        return tuple(rule for rule in RULES if rule.rule_id in requested)
    focus_set = set(focuses)
    return tuple(rule for rule in RULES if rule.focus in focus_set)


def make_summary(
    *,
    root: Path,
    target: Path,
    head: str,
    dirty: bool,
    profile: str,
    focuses: Sequence[str],
    changed_only: bool,
    base: str,
    include_tests: bool,
    files: Sequence[Path],
    selected_rule_ids: Sequence[str],
    candidates: Sequence[Candidate],
) -> dict[str, object]:
    by_rule: dict[str, list[Candidate]] = {}
    for candidate in candidates:
        by_rule.setdefault(candidate.rule_id, []).append(candidate)

    rule_rows: list[dict[str, object]] = []
    descriptions = {rule.rule_id: rule.description for rule in RULES}
    descriptions.update(SPECIAL_RULE_DESCRIPTIONS)

    for rule_id in sorted(set(selected_rule_ids)):
        matches = by_rule.get(rule_id, [])
        rule_rows.append(
            {
                "rule_id": rule_id,
                "description": descriptions.get(rule_id, ""),
                "candidate_count": len(matches),
                "examples": [
                    {
                        "path": item.path,
                        "line": item.line,
                        "hints": list(item.hints),
                    }
                    for item in matches[:3]
                ],
            }
        )

    target_display = (
        "." if target == root else target.resolve().relative_to(root).as_posix()
    )
    return {
        "kind": "summary",
        "repository": str(root),
        "head": head,
        "dirty": dirty,
        "target": target_display,
        "profile": profile,
        "focus": list(focuses),
        "changed_only": changed_only,
        "base": base if changed_only else None,
        "include_tests": include_tests,
        "files_scanned": len(files),
        "candidate_total": len(candidates),
        "rules": rule_rows,
        "note": (
            "Counts are lexical candidates, not findings. "
            "Inline test-context detection is intentionally approximate."
        ),
    }


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Collect sorted lexical candidates for the VERA20k rust-scan skill."
    )
    parser.add_argument("--target", default="src/sim")
    parser.add_argument(
        "--profile", choices=("auto", "sim", "general"), default="auto"
    )
    parser.add_argument("--focus", action="append", choices=FOCUS_NAMES)
    parser.add_argument("--rule", action="append", default=[])
    parser.add_argument("--changed", action="store_true")
    parser.add_argument("--base", default="main")
    parser.add_argument("--include-tests", action="store_true")
    parser.add_argument("--format", choices=("summary", "jsonl"), default="summary")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv if argv is not None else sys.argv[1:])
    try:
        root = find_repo_root(Path.cwd())
        target = resolve_target(root, args.target)
        files = discover_rust_files(root, target)
        if args.changed:
            changed = changed_rust_paths(root, args.base)
            files = [path for path in files if rel_posix(path, root) in changed]
        if not args.include_tests:
            files = [
                path
                for path in files
                if not is_named_test_file(rel_posix(path, root))
            ]

        if args.focus:
            focuses = tuple(dict.fromkeys(args.focus))
        elif is_within(target, root / "src" / "sim") or any(
            rel_posix(path, root).startswith("src/sim/") for path in files
        ):
            focuses = (
                "determinism",
                "architecture",
                "state",
                "safety",
                "performance",
            )
        else:
            focuses = ("safety", "structure")

        rules = selected_rules(focuses, args.rule)
        candidates = collect_regex_candidates(root, files, rules, args.profile)
        candidates = filter_test_candidates(candidates, args.include_tests)
        requested_special = set(args.rule) & set(SPECIAL_RULE_DESCRIPTIONS)
        if requested_special:
            candidates.extend(
                item
                for item in collect_structure_candidates(root, files)
                if item.rule_id in requested_special
            )
        elif "structure" in focuses and not args.rule:
            candidates.extend(collect_structure_candidates(root, files))
        candidates = deduplicate_and_sort(candidates)
        selected_rule_ids = [rule.rule_id for rule in rules]
        if requested_special:
            selected_rule_ids.extend(sorted(requested_special))
        elif "structure" in focuses and not args.rule:
            selected_rule_ids.extend(sorted(SPECIAL_RULE_DESCRIPTIONS))
        head, dirty = repo_snapshot(root)
        summary = make_summary(
            root=root,
            target=target,
            head=head,
            dirty=dirty,
            profile=args.profile,
            focuses=focuses,
            changed_only=args.changed,
            base=args.base,
            include_tests=args.include_tests,
            files=files,
            selected_rule_ids=selected_rule_ids,
            candidates=candidates,
        )

        if args.format == "summary":
            print(json.dumps(summary, indent=2, ensure_ascii=False))
        else:
            print(
                json.dumps(
                    {
                        "kind": "metadata",
                        "repository": str(root),
                        "head": head,
                        "dirty": dirty,
                        "files_scanned": len(files),
                        "focus": list(focuses),
                    },
                    ensure_ascii=False,
                )
            )
            for candidate in candidates:
                print(json.dumps(candidate.as_dict(), ensure_ascii=False))
            print(json.dumps(summary, ensure_ascii=False))
        return 0
    except (OSError, RuntimeError, ValueError) as error:
        print(f"rust-scan candidate collection failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
