---
name: re-swarm
description: "Project-scoped for VERA20k/RA2 Yuri's Revenge only. Use when the user invokes '/re-swarm', asks to orchestrate parallel reverse-engineering investigations, wants detail-level gamemd.exe research gaps scanned, wants parity-blocker research, or wants multiple /re-investigate reports reconciled into implementation handoffs. Dispatches at most 3 read-only subagents per wave, each investigating one narrow target, then checks reports for cross-doc contradictions and Rust-facing handoff quality. Unless the user opts out, the parent serially syncs only certainty-gated Ghidra metadata after all workers stop. Never auto-applies doc or code patches."
---

# Re-Swarm

Coordinate up to 3 parallel reverse-engineering investigations per wave for detail-level
`gamemd.exe` gaps, then reconcile their reports. This skill is detection and
research only: do not edit Rust, INI files, tracked/published docs outside
`docs/research/`, or Ghidra state while any worker/reader is active. The final
parent-only serial metadata sync is the sole Ghidra exception. Reports must include
implementation handoffs, but the swarm never implements them.

Research truth bar: record active-gamemd mechanism, state, pixels/results, and
uncertainty precisely; no approximation may be labelled exact parity. This does
not make every finding current implementation priority. Reconciliation must
also classify retail-skirmish impact and separate exactification residuals.

Native-to-Rust translation rule: do not recommend literal C++ architecture ports
unless the user explicitly asks. Research must identify the verified gamemd
behavior contract, then hand it off as Rust-native ownership boundaries: storage
can remain in `EntityStore`, scheduler/order can live in a Rust `LogicScheduler`
or equivalent owner, lifecycle effects can be helper APIs, and systems can stay
plain functions. What must match is the native semantics: ordering, membership,
state reads/writes, RNG use, frame visibility, same-tick consequences, and
downstream bytes/pixels. Avoid both bad shortcuts: copying raw pointer/vtable
architecture verbatim, or proposing clean Rust that changes behavior.

Label-adversarial Ghidra rule: local Ghidra names, labels, comments, xref
labels, and decompiler-assigned symbol names are navigation hints only. This
project may contain polluted or stale labels from earlier scripts. Handoff-critical
identity claims must be verified from function body, assembly/decompile, callsites,
receiver/`this` pointer, argument flow, vtable slot bytes, data references, and
active-YR reachability. Prefer address plus verified role over label name, and
record label drift explicitly.

## Hard Gates

- Dispatch at most 3 subagents at once, or fewer when collaboration slots are
  unavailable. Larger queues run as sequential waves.
- Use subagents only after the user explicitly invokes `/re-swarm` or otherwise
  asks for parallel/delegated reverse-engineering work.
- Each subagent must use Ghidra MCP read-only. Use the full mutating-tool blacklist in
  the worker prompt below; do not duplicate it here.
- Each subagent must return Ghidra annotation candidates separately: address,
  current name/reference, proposed label/comment/reference, and exact live-binary
  proof. A worker never applies a candidate.
- Each subagent may write exactly one research report under
  `<main-checkout>/docs/research/` plus the shared claims file.
  Nothing else.
- The parent may write only the coverage index and claims file. Reconciliation
  never applies code or docs fixes. After all readers stop, the parent may apply
  only ENGINE.md certainty-gated Ghidra metadata serially.
- If more than half of a wave fails, stop before reconciliation and ask the user
  whether to retry failed slots, review successes only, or abort.
- Every returned finding must distinguish verified binary evidence from inference
  and must label whether the path is active in standard YR.
- Handoff-critical claims require stronger evidence than ordinary report facts:
  cite decompile plus assembly/disassembly address range, decompile plus
  xref/caller evidence, or INI/default source plus the binary reader address.
  Do not mark a handoff COMPLETE from decompiler prose alone when the claim
  depends on inclusive/exclusive bounds, signedness, argument order, struct
  offsets, default values, path liveness, or TS-vs-YR activity.

## Invocation Modes

- `/re-swarm`: scan for detail-level research gaps, propose up to 3 targets, wait for
  confirmation.
- `/re-swarm <t1>, <t2>, ...`: validate explicit targets, wait for confirmation.
- `/re-swarm --area <area>`: scan only one area, propose up to 3 targets, wait for
  confirmation.
- `/re-swarm --parity-blocker <area>`: propose targets ranked by what blocks
  player-visible parity or screenshot parity for the named area.
- `/re-swarm --dry-run`: scan/validate and print the dispatch plan, then stop.
- `/re-swarm --refresh-index`: rebuild the coverage index before ranking.
- `/re-swarm --handoff-plan`: after normal reconciliation, add a non-editing
  implementation bridge plan: patch order, files likely touched, tests to
  add/update, risk ordering, and facts that must remain unchanged.
- `/re-swarm --no-sync-ghidra-labels`: keep the complete run read-only in Ghidra
  and report annotation candidates only.

If an explicit list has one entry, use a single investigator unless parallelism
adds a concrete independent check. If it has more than 3 entries, split it into
sequential waves. Do not dispatch broad targets like `BuildingClass` or `combat`;
narrow them to one function, field, INI key, vtable slot, or behavior.

## Research Index Preflight

Before rebuilding the swarm coverage index or proposing slots, use the
research-index MCP first. Prefer `research_brief`, `research_map`, and
`research_validate`; use the repo-local CLI only for a missing MCP capability:

```text
python tools/research_index/brief.py "<topic>" --limit 8
python tools/research_index/map.py "<topic>" --limit 12
```

If the exact system is known, add `--system <system>` (examples: `bridges`,
`miner`, `skirmish-ui`, `chrono`). If the result reports zero docs, rerun without
`--system`, then rerun with the inferred exact system. Use `validate.py` for stale
or missing docs and `graph.py evidence` / `graph.py implementation` for candidate
anchors and Rust-facing handoff clues. The index informs target selection; it does
not replace read-only Ghidra investigation.

## Step 1: Build Or Read The Coverage Index

For auto, scoped, and refresh modes, use
`<main-checkout>/docs/research/.swarm-coverage-index.md`.

- If absent, rebuild it.
- If present and modified within 7 days, use it and tell the user the cache date.
- If older than 7 days or `--refresh-index` was passed, rebuild it and tell the
  user.

When rebuilding, walk every `*_GHIDRA_REPORT.md` in
`<main-checkout>/docs/research/` and collect detail-level candidates:

- `[DEFERRED]` entries from open-question sections.
- "Phase 2/3 deferred", "future work", or equivalent markers.
- Function addresses mentioned in passing but not analyzed in detail.
- Struct fields whose purpose is unknown or only typed at an offset.
- Vtable slots marked unknown, unused, or omitted from a documented table.
- INI keys referenced by docs, then inverse coverage against `ini/rulesmd.ini` and
  `ini/artmd.ini` for undocumented keys.
- Existing reports missing an `Implementation Handoff` section, or handoffs marked
  `current Rust delta: unchecked` for player-visible behavior.
- Stale-doc markers found during recent implementation work: reports whose claims
  contradict current docs, current Rust, or sibling reports.
- Optional vtable coverage checks against Ghidra only if they require no more than
  50 read-only Ghidra calls.

Write one flat Markdown bullet per gap:

```markdown
- DEFERRED | RADIO_PROTOCOL | Q-12 | "Is FUN_0050AB10 the resend handler?" | category: bounded-cost-too-high
- HINTED_FN | FOOT_MOVEMENT | FUN_005A1C30 mentioned 3x | no dedicated investigation
- UNDOC_FIELD | BUILDING_CLASS | offset 0x6A4 | type uint32 | purpose unknown
- VTABLE_SLOT | AIRCRAFT_CLASS | slot 0x48 | unknown per doc; binary has FUN_004C8800
- UNDOC_INI | rulesmd.ini | [General] AllowableVehicleHijackerCounts= | no doc citation
- MISSING_HANDOFF | BUILDING_BRACKETS | exact Z-buffer predicate known | no Rust-facing acceptance scenario
```

## Step 1.5: Summarize Recent Work And Known Settled Facts

Before ranking targets, write a short current-state note for the dispatch prompt.
Use only cheap local evidence: recent conversation context, relevant report
headings, `git diff --stat`, and focused `rg` over `src/` and docs. Do not run a
mini-investigation here.

The note must contain:

- `Already settled`: facts agents must not rediscover unless resolving a conflict.
- `Current Rust shape`: files/functions already touched or clearly involved.
- `Known remaining gaps`: specific questions that still block parity.
- `Stale docs suspected`: exact doc paths or claims, if known.

For `--parity-blocker`, also include one player-visible target scenario such as
"selected building partly behind cliff under gap re-shroud".

## Step 2: Rank And Propose Targets

Filter by area if requested, then rank candidates:

- HIGH: referenced from 3+ docs, controls player-visible thresholds/formulas, is
  on an active YR tick path, or blocks a known player-visible/screenshot parity
  scenario.
- MEDIUM: referenced from 1-2 docs, fills a deferred item from a high-confidence
  report, covers an INI key in a default-enabled section, or turns an existing
  high-confidence report into an implementation-ready handoff.
- LOW: isolated curiosity, no downstream consumer, or likely TS-only/disabled code.

Break ties by cross-reference count, then smallest scope, then subsystem diversity.
For `--parity-blocker`, break ties by highest player visibility, then smallest
scope. Print a dispatch table with slot, target, one-line scope, source, tier, and
expected handoff. Wait for explicit confirmation such as `go`. Prior confirmation
in the same thread does not count for a new swarm. For `--dry-run`, stop after
printing the plan.

Example:

```markdown
Proposed /re-swarm dispatch:

| Slot | Target | Scope | Source | Tier | Expected handoff |
|---|---|---|---|---|---|
| 1 | FUN_0050AB10 radio resend handler | Decompile state and confirm YR live path | RADIO_PROTOCOL Q-12 | HIGH | resend timing acceptance scenario |
| 2 | BuildingClass+0x6A4 field | Identify type, accessor, and behavior | BUILDING_POWER UNDOC_FIELD | MEDIUM | Rust field mapping or no-op proof |
| 3 | [General] AllowableVehicleHijackerCounts= | Find reader, struct offset, default behavior | rulesmd.ini UNDOC_INI | MEDIUM | rules parser delta + test |
```

## Step 3: Write Claims

Path: `<main-checkout>/docs/research/.swarm-claims.md`

Append one block per swarm invocation. Do not overwrite old rows.

```markdown
## Swarm 2026-05-18T11:42

- 2026-05-18T11:42 - slot-1 - FUN_0050AB10_RadioResendHandler - claimed
- 2026-05-18T11:42 - slot-2 - BuildingClass_field_0x6A4 - claimed
```

Statuses are `claimed`, `done`, and `failed`. A `claimed` row older than 4 hours is
stale and may be re-claimed by a future swarm, but do not delete it.

## Citation discipline

Every material worker claim must cite the exact verification call or local source.
Match evidence to claim type:

| Claim | Minimum evidence |
|---|---|
| Function behavior | `decompile_function` at the entry |
| Caller/callee relation | callers/callees query |
| Vtable owner | COL→TypeDescriptor mangled-name walk |
| Vtable slot | `read_memory vtable+N*4` plus decompile of the result |
| Struct offset/type | assembly context showing displacement and access width |
| Constant/default | decompile/assembly showing the literal |
| INI reader/default | string xref, reader decompile, and default literal |
| YR activity/TS legacy | caller trace plus gate and stock default |

Bare or wrong-kind citations do not enter reconciliation. Put unresolved claims
in a YELLOW `Unverified` section instead of mixing them into verified findings.

## Step 4: Dispatch Subagents

Use parallel `spawn_agent` calls where available. Do not set an unavailable model
override; inherit the active model unless the user explicitly requested a supported
model. Each subagent receives one narrow target and a self-contained prompt.

Do not assume subagents will automatically load this skill. Include the relevant
`/re-investigate` constraints directly in each prompt:

```text
You are subagent slot {{SLOT}} of a /re-swarm batch. Your single target is:

{{TARGET}}

Scope, do not expand: {{SCOPE}}

Recent work context from the parent:
{{RECENT_WORK_CONTEXT}}

Use the VERA20k /re-investigate workflow for this target, with these added
constraints:

Hard constraints:
- Ghidra MCP is read-only. Do not call rename_function,
  rename_function_by_address, rename_data, rename_variable, create_label,
  create_function, add_struct_field, modify_struct_field,
  set_decompiler_comment, set_plate_comment, set_disassembly_comment,
  add_memory_reference, remove_reference, save_program, or any mutating tool.
- In swarm mode, `/re-investigate` guidance to create missing functions is
  overridden by this read-only rule. If Ghidra missed a function boundary, do not
  create it; inspect bytes/reachable callers read-only where possible, or record
  the missing boundary as Remaining Uncertainty.
- You may write exactly one research report:
  <main-checkout>/docs/research/<TARGET_SLUG>_GHIDRA_REPORT.md
- You may update only the shared claims file in addition to that report.
- Do not modify Rust files, INI files, tracked/published docs outside
  `docs/research/`, or any other location.
- Follow AGENTS.md project rules: gamemd.exe is the behavior spec, verified binary
  findings must be separated from inference, offsets and addresses must not be
  invented, and TS legacy paths must be checked before being treated as standard YR.
- Every material finding must say: Active in YR: Yes / No / Conditional, with
  evidence.
- Before investigating, write these four lines in your working notes and satisfy
  them in the report: `Target question`, `Non-goals`, `Evidence needed to mark
  COMPLETE`, and `Stop conditions`.
- Handoff-critical claims need stronger evidence than normal facts. Cite either
  decompile plus assembly/disassembly address range, decompile plus xref/caller
  evidence, or INI/default source plus the binary reader address. This is required
  for claims involving inclusive/exclusive bounds, signedness, argument order,
  struct offsets, default values, path liveness, or TS-vs-YR activity.
- Do not rediscover facts listed under `Already settled` unless you find a direct
  contradiction. Spend the slot on the unknowns and the implementation handoff.
- Keep the scope narrow. If related gaps appear, list them as open questions but do
  not investigate them in this slot.
- Your report must include `/re-investigate`'s `Implementation Handoff` section.
  COMPLETE requires at least one handoff item unless the target proves there is no
  Rust-facing or doc-facing implication.
- Include a `Negative Facts / Do Not Do` subsection when the investigation rules
  out an attractive but wrong implementation shortcut. Example:
  `Do not use AddOccupy/RemoveOccupy for real foundation occupancy`.
- Each implementation handoff acceptance scenario must include one concrete Rust
  test-name proposal, e.g. `test_garefn_addoccupy_does_not_block_real_foundation`.
- Include a `Remaining Uncertainty` subsection for any unresolved condition,
  missing caller, inactive-path doubt, or partial Rust-facing implication.
- Include a `Ghidra Annotation Candidates` subsection. For each candidate give
  address, current metadata, proposed label/comment/reference, mutation kind,
  and the exact live-binary calls proving that it passes ENGINE.md's certainty
  gate. Write `None` when no candidate is certain enough. Do not apply it.

Return only:
1. A Markdown summary no longer than 150 lines.
2. The exact report path written.
3. Up to 5 load-bearing verified facts, one line each, with evidence such as Ghidra
   address, offset, INI key path, or file:line.
4. Up to 3 implementation handoff bullets in the form:
   `verified behavior -> Rust delta -> affected surface -> acceptance scenario -> proposed test name -> risk`.
5. Up to 5 negative facts / do-not-do notes, each with evidence.
6. Remaining uncertainty bullets, or `None`.
7. Any stale-doc replacement wording found, with exact doc path.
8. Ghidra annotation candidates, or `None`.
9. Status: COMPLETE, PARTIAL (reason), or FAILED (reason).

Do not return raw decompilation, assembly listings, raw INI dumps, full struct
tables, or the full open questions log.
```

After dispatching, wait for all returns before reconciliation unless a tool-level
timeout makes a slot failed.

## Step 5: Collect Returns

For each returned summary:

1. Verify the report path exists.
2. Update that slot in `.swarm-claims.md` to `done` or `failed`.
3. Treat malformed returns as PARTIAL: over 150 lines, missing report path, missing
   facts list, missing implementation handoff, or no status.
4. Open each report and enforce the citation discipline above. Bare claims or
   wrong-kind evidence make the slot PARTIAL and are excluded from reconciliation.
   If more than ~20% of load-bearing claims fail this gate, mark the slot FAILED.
5. If 0-2 slots failed, continue and flag failures clearly.
6. If 3+ slots failed, stop and ask the user how to proceed.

A slot may only be marked COMPLETE if it returns at least one item in this chain:
`verified behavior -> evidence -> Rust implication -> acceptance scenario -> proposed test name`.
If the slot proves no implementation is needed, that proof must be explicit and
evidenced. Missing negative facts are acceptable only if the slot explicitly says
`Negative Facts / Do Not Do: None`.
If a slot's handoff depends on bounds, signedness, argument order, struct offsets,
defaults, path liveness, or TS-vs-YR activity, COMPLETE also requires the stronger
evidence form described above. Otherwise mark it PARTIAL and name the missing
verification.

## Step 6: Reconcile In Parent

For each successful or partial slot:

1. Read the written report.
2. Extract the 5-10 most load-bearing claims: offsets, addresses, formulas, INI
   mappings, call relationships, and YR-activity labels.
3. Cross-reference each claim against sibling docs in
   `<main-checkout>/docs/research/`.
   - If the sibling matches, record it as corroborated.
   - If it conflicts, record both claims and both evidence citations.
   - If it appears nowhere else, record it as novel.
4. Spot-check at least 2 load-bearing claims by re-reading the cited Ghidra address
   or cited source material.
   - Spot-check every claim that directly gates an implementation handoff when it
     involves bounds, signedness, argument order, struct offsets, defaults, path
     liveness, or TS-vs-YR activity.
5. Check YR-activity claims against AGENTS.md, existing TS-legacy notes, and sibling
   docs. Flag any suspicious "Active in YR: Yes" claim.
6. Reconcile implementation handoffs:
   - Merge duplicate handoffs that point at the same Rust surface.
   - Flag contradictions between sibling handoffs.
   - For each handoff, run a mandatory focused Rust scan before ranking:
     `rg` likely target symbols/INI keys/type names in `src/`, collect file
     references, existing tests, and likely ownership/module boundary.
   - Record the Rust scan as: `target symbols -> file refs -> existing tests ->
     likely ownership`. This is reconnaissance only; do not edit Rust.
   - Rank handoffs by player visibility, certainty, and smallest implementation
     surface.
   - Preserve `do-not-do` risks; these are often the most useful output.
7. Consolidate negative facts / do-not-do notes:
   - Keep evidenced notes even if they are not direct implementation tasks.
   - Prefer short imperative wording: `Do not ... because ...`.
   - Attach the evidence source and affected Rust surface when known.
8. Extract remaining uncertainty:
   - Include unresolved caller questions, conditional YR activity, missing Rust
     threading, unverified stock/mod split, or partial handoff risks.
   - Do not bury uncertainty inside prose; make it a distinct final-report list.
9. Extract stale-doc fixes separately from implementation handoffs. Include exact
   replacement wording, but do not apply it unless the user asks.

## Step 6.5: Serial Ghidra Annotation Sync

This skill's description advertises parent synchronization, so run it after every
worker stops unless `--no-sync-ghidra-labels` or a read-only request disables it.
Collect and deduplicate candidates, then follow ENGINE.md's serial certainty, save,
and readback protocol. If write tools are unavailable, report the queue and finish
the research normally.

If `--handoff-plan` was requested, add a non-editing bridge plan after the normal
reconciliation:

1. Group handoffs by likely Rust ownership/module boundary.
2. Produce a patch order that starts with data model/parser/test fixtures, then
   shared helpers, then gameplay call sites, then UI/render consumers if any.
3. List files likely touched and tests to add/update, using exact paths when the
   Rust scan found them.
4. Rank risks by player visibility, regression surface, determinism risk, and
   stock-frequency.
5. List `must remain unchanged` facts, especially negative facts and known
   gamemd-compatible exceptions.
6. Stop at the plan. Do not implement, stage, commit, or edit docs.

Do not edit docs during reconciliation.

## Step 7: Report To User

Keep the final report concise and structured:

```markdown
## /re-swarm Results - 2026-05-18T11:42

Status: 4 of 5 complete, 4 reports written, 1 contradiction found.

Ghidra annotations: 6 candidates, 3 applied serially, 2 uncertain, 1 conflicted.

### Reports Written
1. slot-1 - RADIO_RESEND_HANDLER_GHIDRA_REPORT.md - COMPLETE - one-line finding
2. slot-2 - BUILDING_FIELD_6A4_GHIDRA_REPORT.md - PARTIAL - reason

### Cross-Doc Contradictions
1. slot-2 vs BUILDING_POWER_GHIDRA_REPORT.md:
   - slot-2 claim and evidence
   - sibling claim and evidence
   - recommended next check

### Novel Findings
- slot-1: finding with evidence.

### Implementation Handoffs
1. slot-2: verified behavior -> affected Rust surface -> acceptance scenario -> proposed test name -> risk.

### Focused Rust Scan
- slot-2: target symbols -> file refs -> existing tests -> likely ownership.

### Negative Facts / Do Not Do
- slot-2: do-not-do note with evidence and affected Rust surface.

### Remaining Uncertainty
- slot-3: unresolved caller, inactive-path doubt, missing Rust threading, or `None`.

### Handoff Plan
Only include when `--handoff-plan` was requested.
- Patch order: data/parser -> helper -> call sites -> consumers.
- Files likely touched: exact paths from focused Rust scan.
- Tests to add/update: proposed test names and target files.
- Risk ordering: highest player-visible/frequent risk first.
- Must remain unchanged: negative facts and verified exceptions.

### Stale Doc Fixes Suggested
- `path/to/report.md`: replace "<old claim>" with "<new claim>".

### YR-Activity Flags
- slot-3: conditional or suspicious active-path claim.

### Contract Violations
- slot-4 returned 187 lines, treated as PARTIAL.

### Failed Slots
- slot-5 failed because Ghidra MCP timed out. Suggested single-target retry:
  /re-investigate "target".
```

End after reporting. Do not start a follow-up swarm, run `/verify-doc`, or edit docs
unless the user explicitly asks.

## Stop Conditions

- More than 2 slots fail.
- A subagent writes outside its allowed locations.
- A target expands into a whole system instead of one detail.
- Ghidra MCP read-only access is not available.
- The user has not confirmed dispatch after seeing the proposed slot list.
