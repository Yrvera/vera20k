---
name: re-swarm
description: "Coordinate parallel gamemd.exe investigations and reconcile their Rust-facing handoffs. Workers keep Ghidra read-only; the parent synchronizes certain, low-risk metadata after all readers stop unless opted out. Does not implement gameplay or patch existing research inputs."
---

# Coordinate mechanism research

Use [shared coordination](../_shared/swarm.md),
[re-investigate](../re-investigate/SKILL.md), and the
[binary workflow](../../../docs/research/ghidra-workflow.md).

Resolve the requested targets into independent mechanism questions. For an area
or an unspecified target, inspect current code, research, and known unknowns, then
select a bounded wave that would most improve implementation decisions. Explain
the selection and proceed within the user's scope. Prefer ordinary-skirmish
blockers when ranking; preserve exactness for every finding.

Give each worker its target, known evidence, current Rust context, unresolved
question, and a unique report path under task-owned `docs/research/`. Workers may
write their assigned reports; existing research inputs, gameplay files, and Ghidra
state remain unchanged. Do not create shared claims logs or hand-maintained indexes.

Reconcile the reports against their citations and current Rust. Resolve conflicting
identities, offsets, formulas, active-YR conditions, and ownership assumptions from
primary evidence. Independently inspect conclusions that drive implementation,
especially bounds, ordering, dispatch, and lifecycle claims. A worker's confidence
does not upgrade its evidence.

Consolidate duplicate handoffs and identify the smallest coherent prerequisites.
Each useful handoff connects verified behavior to an existing Rust surface or
missing capability, an acceptance scenario, and remaining risk. Report stale-doc
corrections without applying them. Keep unresolved questions visible; a partial
investigation cannot certify complete coverage.

After every reader stops, deduplicate annotation candidates and apply only certain,
low-risk metadata serially in the parent, following the binary workflow's save and
readback protocol. This skill authorizes that sync by default. A read-only request
or `--no-sync-ghidra-labels` disables it. Conflicts remain unapplied; unavailable
write tools leave a candidate report and do not invalidate completed research.

Supported modifiers:

- `--area <area>` bounds selection; `--parity-blocker <area>` prioritizes a known gap.
- `--dry-run` reports the proposed work without dispatch or mutation.
- `--refresh-index` refreshes candidate discovery from current sources.
- `--handoff-plan` adds a non-editing dependency and validation outline.

Return report links, decisive findings, reconciled handoffs, remaining uncertainty,
and annotation outcomes. Stop at the requested research scope.
