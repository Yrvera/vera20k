"""Graph invariants for prerequisite routing."""

from __future__ import annotations

from .model import Diagnostic


def validate_requires_cycles(
    edges: list[dict],
    known_systems: set[str],
    coupled_sets: list[set[str]],
    diagnostics: list[Diagnostic],
) -> None:
    planes = sorted(
        {
            edge.get("plane")
            for edge in edges
            if edge.get("kind") == "requires"
            and isinstance(edge.get("plane"), str)
        }
    )
    for plane in planes:
        adjacency = {system_id: set() for system_id in known_systems}
        for edge in edges:
            if edge.get("kind") != "requires" or edge.get("plane") != plane:
                continue
            source = edge.get("from")
            target = edge.get("to")
            if source in adjacency and target in adjacency:
                adjacency[source].add(target)
        for component in _strongly_connected_components(adjacency):
            member = next(iter(component))
            cyclic = len(component) > 1 or member in adjacency[member]
            acknowledged = (
                len(component) > 1
                and any(component <= coupled for coupled in coupled_sets)
            )
            if cyclic and not acknowledged:
                diagnostics.append(
                    Diagnostic(
                        "error",
                        "UNACKNOWLEDGED_REQUIRES_CYCLE",
                        f"{plane} requires cycle is not declared as a "
                        "coupled set: "
                        + ", ".join(sorted(component)),
                    )
                )


def _strongly_connected_components(
    adjacency: dict[str, set[str]]
) -> list[set[str]]:
    index = 0
    indices: dict[str, int] = {}
    lowlinks: dict[str, int] = {}
    stack: list[str] = []
    on_stack: set[str] = set()
    components: list[set[str]] = []

    def visit(node: str) -> None:
        nonlocal index
        indices[node] = index
        lowlinks[node] = index
        index += 1
        stack.append(node)
        on_stack.add(node)
        for target in sorted(adjacency[node]):
            if target not in indices:
                visit(target)
                lowlinks[node] = min(lowlinks[node], lowlinks[target])
            elif target in on_stack:
                lowlinks[node] = min(lowlinks[node], indices[target])
        if lowlinks[node] == indices[node]:
            component: set[str] = set()
            while True:
                item = stack.pop()
                on_stack.remove(item)
                component.add(item)
                if item == node:
                    break
            components.append(component)

    for node in sorted(adjacency):
        if node not in indices:
            visit(node)
    return components
