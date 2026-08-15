---
name: implementation-contract
description: >
  Project-scoped for VERA20k/RA2 Yuri's Revenge only. Use after research,
  synthesis, disparity scans, traces, or audits when the user wants to prove
  exactly what Rust must implement or change to close a specific exact-mechanism
  parity gap. Distills verified gamemd.exe behavior plus current Rust evidence
  into a sourced `*_IMPLEMENTATION_CONTRACT.md` with required deltas, blockers,
  and acceptance tests. Never writes Rust code.
---

# Implementation Contract

Create an implementation contract for: **$ARGUMENTS**

This skill is a distillation step between research/synthesis and
brainstorming, planning, or implementation. It does not discover an entire
system from scratch. It proves which Rust changes are actually required to close
a bounded parity gap.

For the current retail-convincing skirmish milestone, exact truth and delivery
priority are separate columns. A proven exact delta remains `REQUIRED_FIX` as a
truth statement, but may be an `EXACTIFICATION-RESIDUAL` rather than required
now. Never soften the evidence verdict; never make a bounded expert-only delta
block common-path implementation merely because it exists.

Core proof:

```text
verified gamemd.exe behavior
+ verified current Rust behavior
+ mechanism, byte, pixel, timing, or downstream-result mismatch
= required Rust delta
```

And the inverse:

```text
verified gamemd.exe behavior
+ verified current Rust behavior
+ proven byte-identical state and pixel/result-identical output over the relevant input space
= test-only or documentation-only delta
```

Native-to-Rust translation rule:

```text
verified gamemd.exe mechanism
+ Rust-native ownership boundary that preserves the same semantics
= acceptable implementation shape
```

Do not require literal C++ architecture ports. Do not recreate raw pointer
vectors, global singleton mutation, COM/vtable plumbing, or the full native
inheritance tree unless the evidence proves that no Rust-native structure can
preserve the semantics. The contract must state the behavior to preserve and the
Rust-native owner likely responsible for it: storage (`EntityStore`), active
order (`LogicScheduler` or equivalent), lifecycle helpers, subsystem functions,
or app/render consumers. The required delta is exact gamemd semantics in clean
Rust, not C++ body shape in Rust syntax.

Label-adversarial Ghidra rule: local Ghidra names, labels, comments, xref
labels, and decompiler-assigned symbol names are navigation hints only. This
project may contain polluted or stale labels from earlier scripts. Any contract
claim that depends on function identity, caller identity, vtable slot identity,
field role, or active-YR reachability must be verified from body/callsites/bytes
or marked `BLOCKED`. Prefer address plus verified role over label name.

---

## Hard Gates

- Do not write Rust, INI, asset, or research-doc patches.
- The only filesystem artifact this skill may create or modify is its own contract under
  `docs/contracts/`. Report Ghidra annotation candidates; synchronization requires
  `--sync-ghidra-labels` or a direct user request.
- Do not call something wrong unless both sides are proven:
  - original gamemd behavior is sourced;
  - current Rust behavior is verified by reading code;
  - the difference affects mechanism, consumed bytes, pixels, timing, audio, UI,
    or downstream results, or exact equivalence is unproven.
- Do not treat synthesis docs as primary evidence. Use them as maps to the
  underlying report, trace, Ghidra address, INI key, asset, or Rust surface.
- Treat contradictions as primary evidence. If a screenshot, runtime result, test
  failure, trace, asset dump, or user observation disagrees with the current
  explanation, mark the row `BLOCKED` or `UNKNOWN` until the contradiction is
  resolved with primary evidence.
- Trace from the behavior owner, not a helper. UI contracts start from the
  paint/message/render owner; gameplay contracts start from the tick/action
  owner; parsing contracts start from the actual load/apply path; audio
  contracts start from the event trigger path.
- Prove activation in the target scenario before classifying a row as
  `REQUIRED_FIX` or `TEST_ONLY`. Existing code in gamemd.exe is not
  enough; record the target mode/dialog/map/unit/setting and the flag/key/case
  that enables the behavior.
- Inspect the relevant variant set before generalizing: SHP frames, YR `*md`
  overrides, base INI fallback, dialog IDs, game-mode/difficulty branches,
  house/faction overrides, map overrides, and optional flags.
- Do not run broad reverse engineering. If evidence is missing, mark the item
  `BLOCKED` and recommend `/re-investigate` or `/re-swarm`.
- Use bounded live Ghidra spot-checks only when resolving a handoff-critical
  contradiction or confirming a cited address.
- A live spot-check may report a Ghidra annotation candidate only with its
  address/source, current metadata, proposed label/comment/reference, and exact proof.
  Workers and parallel readers remain read-only.
- Do not spawn subagents unless the user explicitly requested delegated or
  parallel work.

---

## When To Use

Use this skill when the user asks for:

- an implementation contract;
- proof of what Rust needs to change;
- a contract to close a parity gap;
- turning a synthesis doc into concrete Rust deltas and tests;
- proving whether suspected gaps are real before implementation.

Do not use this skill for:

- broad "what should we work on next" or backlog scans: keep outside this skill;
- docs-vs-Rust disparity discovery across a whole system: use `/disparity-scan`;
- new binary research: use `/re-investigate` or `/re-swarm`;
- architecture option design: use `/brainstorm`;
- task-by-task coding plans: use `/write-plan`.

---

## Classification Labels

Every contract row must use one of these labels:

- `REQUIRED_FIX` - Proven active-gamemd mechanism, byte, pixel, timing, audio,
  UI, or downstream-result mismatch. Rust must change.
- `TEST_ONLY` - Rust appears to match, but regression coverage is missing or
  stale.
- `DOC_ONLY` - Code behavior is acceptable; comments, tests, plans, or research
  prose are stale or misleading.
- `BLOCKED` - gamemd behavior is not verified enough for implementation.
- `UNKNOWN` - current Rust behavior or source mapping could not be proven in the
  time/scope available.

Only `REQUIRED_FIX`, `TEST_ONLY`, and `DOC_ONLY` may become implementation work.
`BLOCKED` and `UNKNOWN` require more investigation. There is no internal-only
`NO_DELTA` classification for active YR behavior; if exact equivalence is not
proven, classify the row as `REQUIRED_FIX`, `BLOCKED`, or `UNKNOWN`.

Every row also receives one delivery class:

- `MILESTONE-BLOCKING`
- `COMPOUNDING`
- `EXACTIFICATION-RESIDUAL`
- `UNKNOWN-RISK`

`BLOCKED` or `UNKNOWN` evidence blocks current implementation only when its
delivery class is unknown-risk for the selected common scenario or it could
affect deterministic/shared architecture.

---

## Step 0: Scope The Gap

Parse `$ARGUMENTS` into one bounded parity target.

State:

- system name;
- exact active-gamemd mechanism, byte, pixel/result, or suspected gap;
- included surfaces;
- explicit non-scope;
- whether this is based on an existing synthesis doc, disparity scan, trace, or
  direct user claim.

If the scope is broad, narrow it before continuing. A good contract target is
"CABHUT bridge collapse footprint", not "bridges".

## Research Index Preflight

Before manual doc gathering, use the research-index MCP first. Prefer
`research_brief` and `research_handoff`; if unavailable, use the repo-local CLI
fallback:

```text
python tools/research_index/brief.py "<topic>" --limit 8
```

If the exact system is known, add `--system <system>` (examples: `bridges`,
`miner`, `skirmish-ui`, `chrono`). If the result reports zero docs, rerun without
`--system`, then rerun with the inferred exact system. Use `validate.py`,
`map.py`, `handoff.py`, `graph.py implementation`, and `search.py` for focused
follow-up. Treat index output as navigation and evidence discovery; verified
Ghidra docs, live binary checks, INI/assets, current Rust reads, and tests still
decide parity.

---

## Step 1: Gather Evidence Baseline

Search in this order:

1. `<main-checkout>/docs/research/` for relevant
   `*_SYSTEM_MODEL_SYNTHESIS.md`.
2. Source reports cited by the synthesis:
   - `*_GHIDRA_REPORT.md`
   - trace docs
   - verify-doc audit notes
   - re-swarm / re-investigate outputs
3. INI and art data:
   - `ini/rulesmd.ini`, `ini/artmd.ini`
   - base fallback `ini/rules.ini`, `ini/art.ini`
4. Current Rust:
   - use `rg` and direct file reads for symbol navigation;
   - verify current behavior by reading actual lines.
5. Existing plans, disparity scans, and gap scans as derivative context only.

For every source, classify its role:

- `PRIMARY`: direct binary, trace, INI, asset, or current Rust evidence.
- `SYNTHESIS`: map/reconciliation source.
- `DERIVATIVE`: plan, queue, scan, or old overview.
- `STALE_OR_CONFLICTED`: useful only as a warning.

---

## Step 2: Establish The gamemd Baseline

Extract only behavior that is safe enough to implement:

- active in standard YR or explicitly scoped to a conditional path;
- not TS legacy unless the user intentionally asked for that path;
- backed by primary evidence or a synthesis claim pointing to primary evidence;
- concrete enough to affect active function semantics, consumed bytes,
  downstream values, pixels, audio, UI, or timing.

For each behavior, capture:

- exact mechanism and result;
- timing/order within the tick or state flow;
- constants, bounds, clamps, signedness, and rounding;
- field reads/writes, state bytes, iteration order, and call order;
- INI/asset source and fallback rule;
- animation/audio/RNG order;
- edge cases and stop conditions;
- source citation.

If a behavior matters but lacks evidence, mark it `BLOCKED`; do not infer it.

For UI, menu, shell, sidebar, viewport, sprite, or other visual contracts, also
require a visual asset role table before declaring any row implementation-safe:

```markdown
| Asset | Exact frame | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|
```

Rules for visual baselines:

- `REQUIRED_FIX` must cite complete paint order and target-mode/dialog flag state,
  not just a loaded asset, a filename, or one helper function.
- Multi-frame SHP evidence must name the exact frame. Frame 0 is not assumed.
- Optional draw helpers must have their target-mode flag setters verified before
  becoming implementation-safe.
- Negative claims must be role-scoped. "Not used as preview backing" cannot be
  converted into "not visible" unless the paint path proves that broader claim.
- Stale negative docs become `DOC_ONLY` or `BLOCKED` until primary evidence proves
  the specific visual role.

---

## Step 3: Verify Current Rust

For each gamemd behavior candidate, find the current Rust owner or absence.

Requirements:

- Read the actual Rust files before claiming `missing`, `partial`, or `matches`.
- Cite file paths and line numbers in notes.
- Distinguish behavior from comments. A stale comment is not a behavior bug.
- Treat internal implementation differences as DRIFT unless exact byte/pixel
  equivalence is proven for the relevant input space.
- For sim changes, check deterministic ownership and architecture boundaries.

Do not trust filenames, TODOs, previous agent summaries, or synthesis Rust
handoffs without reading the code.

---

## Step 4: Build The Parity Delta Table

The table is the contract's core. Every row must be evidence-backed.

Use this format:

```markdown
| Evidence class | Delivery class | Mechanism/result | gamemd.exe behavior | Current Rust behavior | Required Rust delta | Evidence | Acceptance test |
|---|---|---|---|---|---|---|---|
```

Rules:

- `REQUIRED_FIX` rows must have a specific required Rust delta and a test.
- `TEST_ONLY` rows must say why behavior appears correct and what regression test
  preserves it.
- `DOC_ONLY` rows must name the stale prose/test wording and the correction.
- `BLOCKED` rows must name the missing evidence and the recommended investigation.
- `UNKNOWN` rows must name what Rust or source mapping could not be proven.
- Visual/UI rows must include the relevant asset role, exact frame, paint-order
  position, and target-mode flag state in either the row or its evidence note.

If a row cannot be classified, the scope is not ready for an implementation
contract.

---

## Step 5: Required Rust Changes

Summarize current implementation work from rows classified
`MILESTONE-BLOCKING` or `COMPOUNDING`. List exactification residuals separately
without turning them into current tasks.

For each required change, state:

- owning module/function/type;
- exact behavior to add, preserve, or correct;
- data source to use instead of hardcoding;
- architecture boundary constraints;
- determinism/state-hash considerations for `sim/`;
- risk area and likely dependent tests.

This section is not a task-by-task implementation plan. Leave task breakdown to
`/write-plan`.

---

## Step 6: Acceptance Tests

Every `REQUIRED_FIX` must have at least one proof mechanism:

- unit test for pure logic;
- integration test or fixture for world behavior;
- INI parse/default test;
- trace/fidelity scenario for rendering, audio, UI, or gameplay timing;
- explicit manual comparison only when automated verification is not currently
  possible.
- screenshot/pixel/manual parity scenario for UI/visual contracts, plus unit
  coverage for rect/frame/order where the renderer exposes testable surfaces.

Each test must say:

- setup;
- action;
- expected mechanism, state bytes, downstream values, pixels, audio, UI, or
  timing result;
- which contract row it proves.

Do not write "add tests" without concrete scenarios.

---

## Step 7: Known Non-Requirements

List tempting changes that must not be implemented now:

- stale-doc behavior contradicted by newer evidence;
- TS legacy or inactive YR paths;
- changes outside active-gamemd exact-mechanism parity;
- behavior with proven byte-identical and pixel/result-identical equivalence;
- broad refactors outside the gap.

This section prevents over-implementation.

---

## Step 8: Blockers And Follow-Ups

List all `BLOCKED` and `UNKNOWN` items with the next appropriate skill:

- `/re-investigate` for one narrow binary question;
- `/re-swarm` for several independent binary questions;
- `/disparity-scan` when current Rust comparison is still too broad;
- `/brainstorm` when behavior is proven but architecture placement is unclear;
- `/write-plan` when the contract is approved and implementation tasks are next.

---

## Step 9: Write The Contract

Save to:

```text
docs/contracts/YYYY-MM-DD-<topic>-implementation-contract.md
```

Use this template:

```markdown
# [System] Implementation Contract

Date: YYYY-MM-DD
Scope: <bounded scope>
Status: <READY_FOR_PLAN | PARTIAL_READY | BLOCKED>

## Gap Being Closed
One sentence describing the mechanism, byte, pixel, timing, or downstream-result mismatch or suspected mismatch.

## Scope
Included:
- ...

Excluded:
- ...

## Evidence Baseline
| Source | Role | Use |
|---|---|---|

## Parity Delta Table
| Class | Mechanism/result | gamemd.exe behavior | Current Rust behavior | Required Rust delta | Evidence | Acceptance test |
|---|---|---|---|---|---|---|

## Required Rust Changes
- ...

## Acceptance Tests
- ...

## Known Non-Requirements
- ...

## Blockers And Follow-Ups
- ...

## Source Ledger
- ...

## Ghidra Annotation Candidates
- None | <address/source, current metadata, proposed mutation, proof, status>
```

Set status:

- `READY_FOR_PLAN` when all required behavior is proven and deltas/tests are
  concrete.
- `PARTIAL_READY` when some rows are ready but bounded blockers remain.
- `BLOCKED` when the core gap cannot be proven yet.

## Step 9.5: Optional Ghidra Annotation Sync

By default, stop after reporting candidates in the contract. If
`--sync-ghidra-labels` was provided or the user directly requested synchronization, the
root/standalone run waits for every reader to stop and follows ENGINE.md's serial sync
protocol. A read-only request or `--no-sync-ghidra-labels` disables synchronization.

---

## Quality Checklist

Before finishing, verify:

- Every `REQUIRED_FIX` row proves gamemd behavior, Rust behavior, mechanism or
  byte/pixel/result mismatch, required delta, and test.
- No row cites only a synthesis doc when primary evidence is available.
- Stale docs are treated as warnings and source maps, not blockers. If primary
  evidence contradicts an old report, the contract follows primary evidence and
  records the stale claim as `DOC_ONLY` or a follow-up correction.
- No `NEEDS_REINVESTIGATE` claim is treated as implementation-safe.
- No TS-legacy or inactive path is listed as required for standard YR.
- No active-gamemd mechanism difference is dismissed as internal-only unless
  exact byte/pixel equivalence is proven.
- Every test maps to at least one table row.
- The contract names what *not* to implement.
- The handoff clearly says whether to run `/brainstorm`, `/write-plan`, or more
  research next.
- Ghidra annotation candidates are listed with applied/deferred/conflicted status, or
  explicitly `None`; no uncertainty was converted into metadata.
- Every visual/UI `REQUIRED_FIX` row has exact asset role, frame, paint-order
  position, and target-mode flag proof.
- No visual/UI row generalizes a scoped negative claim into a broader negative.
- Any optional visual helper without verified flag setter evidence is `BLOCKED`.
