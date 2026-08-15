---
name: audit
description: "Verify and correct one existing VERA20k research document against active Yuri's Revenge gamemd.exe evidence. Use only when the user explicitly asks to audit and fix/correct a research doc; for read-only review use verify-doc or verify-doc-swarm."
---

# Audit And Correct A Research Document

Use this skill to correct one named document under
`<main-checkout>/docs/research/`. The result is a smaller,
more trustworthy document, not a fresh parallel report.

Read `AGENTS.md` first. Active `gamemd.exe` behavior is the specification. Local
labels, prior prose, inferred names, YRpp, and agreement between documents are
navigation aids rather than proof.

## Authorization And Scope

- Require explicit user intent to edit or correct the document. A request to
  inspect, review, or diagnose is read-only and belongs to `verify-doc`.
- Own exactly one research document unless the user names more.
- Do not edit Rust, INI, assets, other documents, or git history.
- Keep evidence gathering read-only and report Ghidra annotation candidates separately.
  This skill does not synchronize them by default. A root/standalone run may do so only
  with `--sync-ghidra-labels` or a direct user request, following ENGINE.md. Workers
  remain read-only.
- Use `apply_patch` for document edits and preserve unrelated correct material.

## Preflight

1. Resolve the requested path and read the entire document.
2. Check its modification time and nearby claim/lock files. If another session
   changed or claimed it recently, stop rather than racing the moving target.
3. Use the research-index MCP first (`research_validate`, `research_map`,
   `research_brief`, or `research_graph` as appropriate). Use the repo CLI only
   when the corresponding MCP capability is unavailable.
4. Record the exact active-YR binary/project being queried. Do not silently audit
   a different executable or a dormant TS path.

## Two-Pass Audit

### Pass 1: Enumerate Claims

Build a private checklist of every load-bearing claim, including:

- function identity, address, owner, caller, and active-YR reachability;
- field offsets, widths, signedness, constants, and sentinel values;
- branch conditions, formulas, rounding, clamps, iteration order, and timing;
- vtable ownership and slot identity;
- INI keys, asset names, and Rust-facing implementation conclusions.

Do not begin rewriting after finding the first error. First establish the claim
surface so one correction does not leave dependent prose contradictory.

### Pass 2: Verify And Classify

Classify each claim as `VERIFIED`, `WRONG`, `STALE`, `MISLEADING`, or `UNCHECKED`.
Use the function body, assembly/raw bytes, callers, receiver flow, data references,
and active-YR reachability as needed. When evidence conflicts, report the conflict
instead of forcing a clean answer.

For vtable claims, use the complete protocol:

1. Resolve the table's Complete Object Locator.
2. Resolve the COL TypeDescriptor to prove the owning class/subobject.
3. Compute the entry at `slot_index * 4` for this 32-bit binary.
4. Follow any adjustor thunk and verify the destination body and callsites.

A plausible local label or a decompiler-assigned name does not complete this
proof.

Record any Ghidra annotation candidate separately with its address/source, current
metadata, proposed label/comment/reference, and the exact live proof required by
ENGINE.md. Otherwise record no candidate.

## Optional Ghidra Metadata Sync

By default, stop after reporting the candidate queue. If `--sync-ghidra-labels` was
provided or the user directly requested synchronization, the root/standalone agent
waits for every reader to stop and follows ENGINE.md's serial sync protocol. A read-only
request or `--no-sync-ghidra-labels` disables synchronization.

## Correct The Document

- Correct only claims supported by the evidence gathered in this audit.
- Never silently delete a load-bearing claim that cannot be re-established. Move
  or mark it as `UNKNOWN`/`UNCHECKED`, preserve enough original wording for
  provenance, and record the verification calls/evidence paths attempted. Never
  invent a replacement address, field, slot, or label.
- Keep verified findings separate from inference and implementation suggestions.
- Add a compact inline citation beside every corrected binary claim. The citation
  must identify the address/symbol plus the exact MCP method and material
  arguments (or equivalent artifact) used, so another investigator can reproduce
  it.
- Update dependent summaries, tables, and implementation handoffs when their
  premise changed.
- Do not inflate the document with raw decompiler dumps. Preserve the evidence
  chain and the exact behavior contract.

## Final Response

Lead with the verdict. Name the corrected document, summarize the material
corrections, list remaining `UNCHECKED` blockers, and report Ghidra annotation counts
as applied/deferred/none. Link the edited file with its absolute path.
