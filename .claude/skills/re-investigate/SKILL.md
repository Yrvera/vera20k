---
name: re-investigate
description: "Research a gamemd.exe mechanism and write its behavior contract, Rust implications, and acceptance scenarios. Research only; no implementation."
---

# Investigate a mechanism

Follow [ENGINE.md](../../../ENGINE.md) and the
[binary workflow](../../../docs/research/ghidra-workflow.md). Use existing research
to locate evidence, current Rust to establish implications, and main-checkout retail INIs.

A broad request may produce a coverage map. An explicitly exhaustive request requires
the complete declared scope; unresolved required behavior remains incomplete.

Trace from the active-YR owner through initialization, relevant dispatch and consumers.
Capture exact predicates, numeric semantics, ordering, RNG and lifecycle effects.
For visuals/audio, establish the complete composition or event path, not merely a helper.

Write or update a task-owned `docs/research/` report containing:

- Scope, active conditions, reproducible binary citations, and unresolved coverage.
- Exact mechanism, with inference distinguished from established behavior.
- Current Rust differences or unchecked surfaces.
- Implementation implications, acceptance scenarios, prerequisites and translation risks.
- Stale-document corrections and Ghidra annotation candidates.

Only the research artifact changes. Ghidra synchronization is opt-in under the binary
workflow (`--sync-ghidra-labels`); `--no-sync-ghidra-labels` or read-only requests
disable it. Workers report candidates only. Finish with findings and the report link.
