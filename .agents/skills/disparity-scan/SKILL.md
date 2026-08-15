---
name: disparity-scan
description: >
  Find disparities between active Yuri's Revenge gamemd.exe evidence and the
  Rust implementation of a named system. Use research docs to discover
  candidate behaviors, verify Rust state directly, and selectively verify
  stale, ambiguous, conflicting, or load-bearing claims against the active
  binary. Produce a structured list that separates verified gaps from
  doc-derived candidates and fix priority from parity verdict. Trigger on
  parity audits, requests to find disparities, or questions about what
  gamemd has that Rust does not.
---

# Disparity Scan - Evidence vs Rust Implementation

Find behaviors, visuals, sounds, INI keys, and edge cases for **`$ARGUMENTS`**
that differ between active Yuri's Revenge evidence and the current Rust
implementation.

Use `docs/research/` for fast discovery and navigation. Research prose is not
the specification: it supplies candidate claims, addresses, terminology, and
prior interpretations. The active YR binary and observed retail behavior are
authoritative. A doc-only claim must remain a candidate until its active-YR
binding is verified.

Keep the scan proportional. Verify current Rust directly for every finding.
Escalate candidate gamemd claims when they conflict, are stale or ambiguous,
would drive a high-impact fix, or concern load-bearing state, ordering,
authority, RNG, determinism, bytes, or pixels. Leave lower-risk unverified
claims clearly labeled instead of pretending they are confirmed gaps.

Report exact truth separately from delivery priority. Rank verified gaps for
the retail-convincing stock-skirmish milestone by normal-play frequency,
player noticeability, loop breadth, compounding, outcome/determinism risk, and
unblock value. Bounded expert-only differences remain visible as
exactification residuals.

If `$ARGUMENTS` is empty, ask the user what system to scan.

---

## Hard Gate - Read-Only Analysis, Candidate Handoff

Modify no source or research input. The only ordinary write is the disparity
report requested by this workflow.

- Do not edit Rust, tests, INI data, or research docs.
- Use Ghidra read-only during analysis.
- Record research corrections and certainty-gated Ghidra metadata candidates
  in the report rather than applying them.
- Treat Ghidra metadata synchronization as a separate, explicitly authorized
  mutation under ENGINE.md, never as part of a nominally read-only scan.

By default, stop after reporting annotation candidates. Synchronize only when
the user directly requests it or passes `--sync-ghidra-labels`. Child workers
never mutate Ghidra.

If the user asks to implement a fix during the scan, explain that this skill
only identifies and verifies disparities. Hand implementation off as a
separate task.

---

## Research Index Preflight

Use the research-index MCP before manual searching. Prefer
`research_brief(query=<system-or-topic>, system=<system>)`; if unavailable, use:

```text
python tools/research_index/brief.py "<system-or-topic>" --limit 8
```

If the exact system is known, add `--system <system>` (for example `bridges`,
`miner`, `skirmish-ui`, or `chrono`). If it reports zero docs, retry without
`--system`, then with the inferred exact system. Use `validate.py`, `map.py`,
`handoff.py`, `graph.py implementation`, and `search.py` as needed to collect
verified docs, stale warnings, Rust-status chunks, and likely touched files.

The index is a locator, not proof. Read every cited source directly before
using it in a finding.

## Evidence Laws

```text
ACTIVE YR EVIDENCE IS AUTHORITATIVE.
DOCS ARE DISCOVERY POINTERS AND INTERPRETATIONS, NOT THE SPEC.
EVERY RUST-STATE CLAIM MUST COME FROM READING CURRENT CODE.
```

Apply this authority order:

1. Active `gamemd.exe` bytes and reproducible observed retail behavior.
2. Live Ghidra interpretation tied to the active binary.
3. Research docs whose relevant claims were verified against that binary and
   have no newer contradictory evidence.
4. Stale, unverified, or provenance-unclear docs, used only as candidate leads.

Prose alone never upgrades a claim to verified. If evidence conflicts, prefer
the higher-authority source and report the conflict. Do not choose the claim
that merely sounds plausible.

Use these evidence states consistently:

- **ACTIVE-YR VERIFIED** - supported by active-binary evidence, a reproducible
  retail observation, or a still-current binary-verified doc with precise
  provenance.
- **DOC-DERIVED CANDIDATE** - supported only by research prose or an uncertain
  active-YR binding. It may justify investigation, not implementation.
- **RUST-STATE UNKNOWN** - current code could not be resolved confidently.
- **INACTIVE / TS-LEGACY** - present in shared engine code but not active in
  standard YR under the relevant defaults.

```text
TIBERIAN SUN != YURI'S REVENGE.
```

Filter TS-only mechanisms from active-YR gaps only after verifying their gate.
Do not infer inactivity from a filename or an old doc label.

```text
EXACT VERDICT AND DELIVERY PRIORITY ARE SEPARATE.
```

For a verified active-YR difference, keep the exact verdict as DRIFT even when
delivery priority is low. A different mechanism with one sampled same-output
result remains DRIFT or UNCHECKED until exact equivalence is proven. Never turn
a scan into a parity percentage or completion certification.

---

## Procedure

### S1 - Resolve scope

Parse `$ARGUMENTS`:

- A system name such as `miner`, `garrison`, or `ore growth`: scan that system.
- A specific function or file: scan only that boundary.
- An ambiguous term: ask the user to narrow it.

State the scope before continuing. Do not silently broaden it.

### S2 - Inventory documented candidate behaviors

Read every relevant document under `docs/research/` top to bottom. For each
claim, record:

- behavior, state transition, formula, timing, or decision logic;
- visual, animation, opacity, particle, or sprite behavior;
- audio cue or voice line;
- INI key and claimed semantics;
- edge case, constant, threshold, or ordering constraint;
- doc path and exact section or line;
- verification status, date/provenance when present, and any stale warning;
- disagreement with other docs or known retail behavior.

Build a flat numbered inventory. This is a candidate set, not a baseline that
has already been proven and not yet a list of gaps.

### S3 - Read the current Rust implementation directly

For every candidate, locate the current implementation, including:

- `src/sim/<system>/` for simulation behavior;
- `src/render/` for visuals;
- `src/rules/` for INI parsing and authority;
- `src/sim/world/` and `app_*` for cross-cutting behavior.

For each Rust-state claim:

1. Open the current file.
2. Read the relevant control flow and callers, not just a matching symbol.
3. Record the exact file and line.
4. Classify Rust as present, partial, missing, different, or unknown relative
   to the candidate claim.

If subagents are used, spot-check at least five returned Rust-state claims. If
one is wrong, independently re-verify all claims from that worker.

Do not promote the comparison result until the gamemd side also has an evidence
state:

- **VERIFIED MATCH** - active-YR behavior and current Rust are proven equivalent
  at the audited boundary. Omit from the main gaps but retain in the appendix.
- **VERIFIED GAP** - active-YR behavior is verified and Rust is missing,
  partial, or different.
- **DOC-DERIVED CANDIDATE** - Rust differs from a doc claim that has not been
  sufficiently verified. Keep it out of confirmed gaps.
- **RUST-STATE UNKNOWN** - the implementation side could not be established.

### S4 - Verify gamemd claims proportionally

Use live active-YR evidence before promoting a candidate when any of these
conditions applies:

- docs conflict, are ambiguous, stale, superseded, or lack an active-YR tie;
- the finding would rank HIGH or materially drive an implementation decision;
- it concerns state ownership, ordering, RNG, determinism, lifecycle,
  irreversible transitions, byte layout, or pixel/result exactness;
- the cited address, caller, flag gate, or mechanism identity is uncertain;
- observed retail behavior contradicts the docs or Rust-side interpretation.

Prefer the smallest decisive check: inspect active bytes, decompile the named
function and callers, verify the gate, or reproduce the retail observation.
Record exact evidence. If live verification is unavailable or inconclusive,
keep the item as `DOC-DERIVED CANDIDATE` or `UNCHECKED`; do not guess.

Do not exhaustively decompile every low-risk behavior by default. Use
`re-investigate` for fresh mechanism research or `trace-action` for a concrete
end-to-end behavior when the user needs deeper verification.

When a doc is wrong, add it to **Doc errors discovered**. Do not edit it during
this workflow.

### S5 - Classify inactive and prerequisite-blocked items

- **TS-legacy / inactive in YR:** exclude from active-YR gaps only with evidence
  for the gate and relevant default. Record important filtered cases in the
  appendix.
- **Blocked by an unimplemented prerequisite:** retain the exact result. A
  verified missing gamemd behavior remains a `VERIFIED GAP - BLOCKED`; an
  unverified one remains a blocked candidate. Put it in the deferred section
  and do not recommend implementing it before its prerequisite.
- **Outside the stated scope:** record as a residual without expanding the scan.

Never call a missing behavior "correctly absent" merely because its dependency
has not been built yet.

### S6 - Categorize and rank

Group verified gaps by:

- Visual / VFX
- Behavior and state machines
- Audio
- INI integration
- Edge cases
- Determinism / multiplayer

Rank delivery priority independently of evidence confidence:

- **HIGH** - frequent, immediately player-visible, compounding, outcome-changing,
  determinism-critical, or a broad unblocker.
- **MEDIUM** - visible in a narrower ordinary-play situation.
- **LOW** - rare or boundary-condition visibility but still real drift.

Do not use code size as a priority proxy. Keep doc-derived candidates in their
own section; do not let severity wording make them sound verified.

### S7 - Write the report

Save to `docs/gap-scans/YYYY-MM-DD-disparity-scan-<system>.md`:

```markdown
---
title: Disparity Scan - <system>
date: YYYY-MM-DD
scope: <one-line scope statement>
methodology: docs-first discovery, direct Rust verification, selective active-YR verification
---

# Disparity Scan - <system>

## Scope and evidence basis

<What was scanned, excluded, which docs were read, and what live binary or
retail evidence was available.>

## Summary

- N documented candidate behaviors inventoried
- N active-YR claims verified
- N verified gaps
- N doc-derived candidates awaiting verification
- N verified matches / false positives
- N deferred or prerequisite-blocked gaps/candidates

This report is a dated disparity snapshot, not a parity percentage or
completion certificate.

## Verified gaps

### HIGH priority

**G<n>. <one-line description>**
- **Active-YR evidence:** <binary address/caller, retail observation, or precise
  verified-doc provenance>
- **Research pointer:** <doc path and section/line>
- **Rust state:** <missing/partial/different with file:line, or no equivalent found>
- **Exact verdict:** DRIFT
- **Priority rationale:** <frequency/player impact/compounding/unblock value>

### MEDIUM priority
<same structure>

### LOW priority / exactification residuals
<same structure>

## Doc-derived candidates needing verification

These are not confirmed gaps and must not be handed directly to implementation.

**C<n>. <one-line candidate>**
- **Doc claim:** <path and section/line>
- **Rust state:** <directly verified file:line, or unknown>
- **Missing proof:** <specific active-YR evidence needed>
- **Potential impact:** <why verification may or may not be worth doing>

## Deferred / blocked by prerequisites

- **<gap or candidate>** - <evidence state>; blocked by <prerequisite>. The
  disparity remains recorded, but implementation is not yet recommended.

## Doc errors discovered

- **<doc path and section>** - <incorrect claim and higher-authority evidence>

## Appendix - verified matches and false positives

| Preliminary claim | Evidence state | Actual Rust state |
|-------------------|----------------|-------------------|
| ... | ... | Implemented at <file:line> |

## Ghidra annotation candidates

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|----------------|------------------|-------------------|------|------------|--------|
| ... | ... | ... | label/comment/reference | ... | deferred/conflict |

Write `None` when no candidate passes ENGINE.md's certainty gate.

## Recommendations

<Which verified gaps cluster, what deserves a separate brainstorm, which
candidates need verification, and what should remain deferred.>
```

Lead with the summary and HIGH verified gaps. Every verified gap must cite:

- decisive active-YR evidence;
- the research pointer, when one exists;
- current Rust state with file and line or explicit `no equivalent found`;
- a player-impact rationale separate from the exact verdict.

Never fabricate paths, addresses, or verification status.

### S7.5 - Optional Ghidra metadata sync

Stop after reporting candidates unless explicitly authorized to synchronize.
If authorized, wait for all readers to stop and follow ENGINE.md's serial sync
protocol. `--no-sync-ghidra-labels` or a read-only request prohibits syncing.

### S8 - Hand off

Tell the user where the report was saved and summarize verified gaps separately
from unverified candidates. Offer appropriate next actions without choosing one:

- brainstorm one verified HIGH gap;
- run `/verify-doc` on a suspect document;
- run `/re-investigate <subsystem>` when the research basis is weak;
- run `/re-investigate <subsystem>` or `/trace-action <mechanic>` when deeper
  verification is needed;
- stay focused on the user's current task.

---

## Anti-patterns

- **Treating docs as the specification.** A doc is a pointer or interpretation
  until its active-YR claim is verified.
- **Trusting delegated Rust-state claims.** Read current code and callers.
- **Calling doc-vs-Rust disagreement a confirmed gap.** Without sufficient
  gamemd evidence it is only a candidate.
- **Skipping high-impact verification to stay fast.** Proportional does not mean
  avoiding the decisive check.
- **Decompiling every low-risk line by default.** Escalate based on uncertainty
  and impact; use a deeper skill when exhaustive proof is requested.
- **Restating docs instead of reporting differences.** Keep the inventory
  internal and report matches only in the appendix.
- **Treating blocked behavior as absent by design.** Preserve it as a blocked
  gap or candidate.
- **Treating TS legacy as active YR without checking its gate.**
- **Severity-ranking by code size.** Rank player impact, frequency, and risk.
- **Using a parity percentage.** A scoped scan cannot certify completion.
- **Implementing during the scan.** Hand verified findings to a separate
  brainstorm and implementation task.

---

## Distinction from sibling skills

- **`disparity-scan`** - focused docs-first candidate inventory plus direct Rust
  cross-check and proportional active-YR verification. Use for a faster bounded
  sweep without treating docs as authority.
- **`re-investigate <system>`** - fresh active-binary research with a durable
  implementation handoff when existing evidence is insufficient.
- **`verify-doc <doc>`** - verify one research document against the binary.
- **`trace-action <mechanic>`** - verify one player action end to end through
  simulation, rendering, and audio.

---

## Key principles

- Surface disparities; let the user choose priorities.
- Verify current Rust state for every reported comparison.
- Separate verified gaps from doc-derived candidates visibly and consistently.
- Preserve blocked and low-priority drift without promoting it prematurely.
- Treat the report as a dated evidence snapshot, not a permanent source of truth
  or a completion ledger.
- Keep verified matches and false positives so later scans do not rediscover
  them without checking whether the code or evidence has changed.
