#!/usr/bin/env python3
"""Format cargo-modules output into the generated sections of docs/module-map.md."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import os
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parents[1]
DOCUMENT = ROOT / "docs/module-map.md"
TARGET = "x86_64-pc-windows-msvc"
COMMON = ["--lib", "-p", "vera20k", "--target", TARGET]
NODE = re.compile(r'^    "([^"\\]+)" \[.*// "([^"\\]+)" node$', re.MULTILINE)
EDGE = re.compile(r'^    "([^"\\]+)" -> "([^"\\]+)" .*// "uses" edge$', re.MULTILINE)


def capture(args: list[str]) -> str:
    return subprocess.check_output(
        args, cwd=ROOT, encoding="utf-8", env={**os.environ, "NO_COLOR": "1"}
    ).replace("\r\n", "\n").strip("\n") + "\n"


def dependencies(dot: str) -> dict[str, set[str]]:
    """Keep uses edges, map item paths to their nearest module, and deduplicate."""
    if not dot.startswith("digraph {") or not dot.rstrip().endswith("}"):
        raise ValueError("Incomplete cargo-modules DOT output")
    nodes = dict(NODE.findall(dot))
    modules = {name for name, kind in nodes.items() if kind in {"mod", "crate"}}
    if "vera20k" not in modules:
        raise ValueError("Expected the vera20k crate in cargo-modules output")
    if any(not (name == "vera20k" or name.startswith("vera20k::")) for name in nodes):
        raise ValueError("Generate the DOT with --no-externs --no-sysroot")

    def containing_module(item: str) -> str:
        if item not in nodes:
            raise ValueError(f"Dependency endpoint has no node: {item}")
        while item not in modules:
            item, separator, _ = item.rpartition("::")
            if not separator:
                raise ValueError("Dependency endpoint has no containing module")
        return item.removeprefix("vera20k::")

    edges = EDGE.findall(dot)
    if len(edges) != dot.count('// "uses" edge'):
        raise ValueError("Unrecognized cargo-modules dependency syntax")
    result = {module.removeprefix("vera20k::"): set() for module in modules}
    for source, target in edges:
        source, target = containing_module(source), containing_module(target)
        if source != target:
            result[source].add(target)
    return result


def tree_modules(tree: str) -> set[str]:
    if not tree.startswith("crate vera20k\n"):
        raise ValueError("Expected the vera20k module structure")
    modules = {"vera20k"}
    parents: list[str] = []
    for line in tree.splitlines()[1:]:
        match = re.fullmatch(r"([│ ]*)[├└]── mod (\w+): .+", line)
        if not match or len(match[1]) % 4:
            raise ValueError(f"Unrecognized module tree line: {line!r}")
        depth = len(match[1]) // 4
        if depth > len(parents):
            raise ValueError("Skipped a level in the module tree")
        parents = parents[:depth] + [match[2]]
        name = "::".join(parents)
        if name in modules:
            raise ValueError(f"Duplicate module in tree: {name}")
        modules.add(name)
    return modules


def replace_section(document: str, section: str, value: str) -> str:
    begin = f"<!-- module-map:{section}:begin -->"
    end = f"<!-- module-map:{section}:end -->"
    if document.count(begin) != 1 or document.count(end) != 1:
        raise ValueError(f"Missing or repeated section markers: {section}")
    before, rest = document.split(begin)
    _, after = rest.split(end)
    return f"{before}{begin}\n{value.rstrip()}\n{end}{after}"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Regenerate and compare without editing the document")
    args = parser.parse_args()

    # A source commit must describe the inputs actually analyzed. Documentation
    # and tooling edits are allowed; source/build-input edits must be committed.
    inputs = ["src", "Cargo.toml", "Cargo.lock", ".cargo", "rust-toolchain", "rust-toolchain.toml"]
    if capture(["git", "status", "--porcelain", "--untracked-files=all", "--", *inputs]).strip():
        raise SystemExit("Commit source/build-input changes before refreshing the map")
    revision = capture(["git", "rev-parse", "HEAD"]).strip()
    version = capture(["cargo", "modules", "--version"]).strip()
    if not re.search(r"\b0\.26\.0\b", version):
        raise SystemExit("This formatter targets cargo-modules 0.26.0; review its output before upgrading")

    tree = capture(["cargo", "modules", "structure", *COMMON, "--no-fns", "--no-traits", "--no-types"])
    dot = capture(["cargo", "modules", "dependencies", *COMMON, "--no-externs", "--no-sysroot"])
    graph = dependencies(dot)
    if tree_modules(tree) != set(graph):
        raise SystemExit("The module tree and dependency graph have different module sets")
    if capture(["git", "rev-parse", "HEAD"]).strip() != revision or capture(
        ["git", "status", "--porcelain", "--untracked-files=all", "--", *inputs]
    ).strip():
        raise SystemExit("Source/build inputs changed during generation; document was not updated")

    document = DOCUMENT.read_text(encoding="utf-8")
    updated = replace_section(document, "dependencies", dependency_text(graph))
    updated = replace_section(updated, "tree", f"```text\n{tree.rstrip()}\n```")
    if args.check:
        # Documentation-only commits need not change the snapshot's provenance.
        if updated != document:
            raise SystemExit("Module map differs from the current generated structure/dependencies")
        print("Module structure and dependencies match the current checkout")
        return
    updated = replace_section(updated, "provenance", provenance(revision, graph))
    DOCUMENT.write_text(updated, encoding="utf-8", newline="\n")
    print(f"Updated {DOCUMENT.relative_to(ROOT)}: {len(graph)-1} modules, {sum(map(len, graph.values()))} dependency edges")


def dependency_text(graph: dict[str, set[str]]) -> str:
    rows = [f"{name} -> {'; '.join(sorted(graph[name])) or '-'}" for name in sorted(graph)]
    return "```text\n" + "\n".join(rows) + "\n```"


def provenance(revision: str, graph: dict[str, set[str]]) -> str:
    day = datetime.now(timezone.utc).date().isoformat()
    return (
        f"Generated snapshot: `{revision}` ({day}), cargo-modules 0.26.0.\n\n"
        f"Scope: `vera20k` library, default features, `{TARGET}`, no depth limit.\n"
        "Test-only, binary-specific and inactive conditional modules are excluded.\n"
        "External crates and the sysroot are excluded from the dependency graph.\n"
        f"Contains **{len(graph)-1} modules plus the crate root**, and **{sum(map(len, graph.values()))} distinct\n"
        "cross-module dependency edges**. Check this source commit against your checkout."
    )


if __name__ == "__main__":
    main()
