---
name: disparity-scan
description: "Compare a system's current Rust with active YR evidence. Separate demonstrated gaps from doc-derived candidates and rank impact independently."
---

# Find disparities

Use [ENGINE.md](../../../ENGINE.md), research and current code to discover
differences within the requested system/scenario. Inspect production Rust and
consumers for every comparison; missing symbols or old reports do not prove absence.

Classify before ranking:

- **Verified gap:** native behavior is established and Rust differs; cite both sides.
- **Doc-derived candidate:** Rust differs from unverified research; name missing proof.
- **Rust state unknown:** the production equivalent remains unresolved.
- **Inactive / TS legacy:** exclusion requires gate/default evidence.
- **Match:** state the checked boundary and evidence strength.

Use the [binary workflow](../../../docs/research/ghidra-workflow.md) for decisive
checks of stale, conflicting, uncertain or implementation-consequential claims.
Include relevant visuals, audio, INI behavior and boundaries. Name omissions;
exhaustive requests require complete declared coverage.

Group by mechanism/prerequisite and rank frequency, player impact, determinism
and unblock value. A missing prerequisite leaves dependent gaps blocked, not
correctly absent.

Save a dated task-owned `docs/research/` report with evidence, Rust locations,
gaps/candidates, prerequisites and unresolved checks; include useful false positives
and stale-doc corrections. Keep source and research inputs unchanged. Ghidra sync is
opt-in under the binary workflow; workers report candidates only. Already-authorized
implementation may continue in the parent task.
