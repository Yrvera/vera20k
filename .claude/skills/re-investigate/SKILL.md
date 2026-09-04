---
name: re-investigate
description: "Research native gamemd.exe behavior, establishing exact active-YR semantics, sources, and implementation implications."
---

# Investigate native behavior

Follow [ENGINE.md](../../../ENGINE.md) and the
[binary reference](../../../docs/research/ghidra-workflow.md). Use existing research
to locate evidence, current Rust for relevant implications, and main-checkout retail INIs.

Establish the requested mechanism from its active-YR owner, initialization, dispatch
and consumers. Capture exact predicates, numeric semantics, ordering, RNG and lifecycle
effects. For visuals/audio, establish the complete composition or event path.

Answer with the behavior and reproducible evidence, active conditions, unresolved
coverage, and relevant Rust implications or acceptance scenarios. Distinguish established
facts from inference. A broad investigation may produce a coverage map; an exhaustive
request remains incomplete while required behavior is unresolved.

Choose the research method, delegation and presentation to fit the task. Save a
task-owned `docs/research/` report when requested or useful for preserving substantive
research; do not require one for a short question.

Research alone does not authorize implementation or corrections to existing research documents.
Ghidra mutation authority and persistence follow the binary reference.
