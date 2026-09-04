---
name: re-investigate
description: "Research a gamemd.exe mechanism before implementation. Produces a sourced research document with current Rust implications and acceptance scenarios; does not implement code."
---

# Investigate a mechanism

Research the requested system under [ENGINE.md](../../../ENGINE.md) and the
[binary workflow](../../../docs/research/ghidra-workflow.md). Deliver the behavior
contract an implementer needs, including the details that distinguish exact YR
behavior from a plausible approximation.

Resolve a concrete scope from the request. A broad request may need a coverage map;
an explicitly exhaustive request requires complete coverage of its declared scope.
Do not silently replace that request with sampling or call unresolved required
behavior complete. Choose the investigation method to fit the question.

Use the research index and existing reports to locate evidence and avoid duplicate
work. Read current Rust in the task checkout and retail INIs in the main checkout. A recent
report is useful context, not a reason to refuse an authorized investigation.

Start from the actual behavior owner and prove active-YR reachability. Follow
callers, callees, concrete virtual dispatch, initialization, and consumers as needed
to establish the scoped mechanism. Record precise conditions, constants, field
widths, signedness, rounding, state transitions, ordering, RNG consumption, and
lifecycle effects. Unread helpers or uncertain labels cannot support a conclusion.
Resolve contradictory observations from primary evidence.

For visual or audio work, follow the full production composition or event path;
identify the actual assets, frames, palette, positioning, and timing. Loading an
asset does not prove it is drawn or visible. Keep unknowns and omitted variants
explicit without expanding into unrelated systems.

Write or update a task-owned report under `docs/research/`. Organize it for the
mechanism rather than filling a fixed template. Include:

- Scope, evidence basis, active-YR conditions, and remaining coverage.
- Exact behavior with reproducible inline binary citations; inference kept distinct.
- Current Rust surfaces and concrete differences, or explicitly unchecked Rust state.
- Implementation implications, acceptance scenarios, prerequisites, and risks of an
  attractive but incorrect translation. Preserve Rust-native ownership choices.
- Relevant stale-document corrections and Ghidra annotation candidates.

This skill writes research, not Rust or gameplay patches. Report candidates by
default; only an authorized root/sole agent may synchronize Ghidra metadata under
the binary workflow after all readers stop. `--sync-ghidra-labels` opts in;
`--no-sync-ghidra-labels` or a read-only request opts out.

Finish with the decisive findings, report link, and what remains unresolved.
