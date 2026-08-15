---
name: verify-doc-swarm
description: "Project-scoped for VERA20k/RA2 Yuri's Revenge only. Use when the user invokes '/verify-doc-swarm', asks to orchestrate parallel research-doc audits, wants the highest-leverage stale or never-audited docs checked against gamemd.exe, or asks to reconcile multiple /verify-doc GREEN/YELLOW/RED results. Dispatches at most 3 read-only subagents per wave, each running /verify-doc on one research doc, then ranks staleness, WRONG/MISLEADING findings, cross-doc contradictions, and correction-fact bundles for a later patch pass. Never auto-applies doc or code patches."
---

# Verify Doc Swarm

Coordinate up to 3 parallel `/verify-doc` audits per wave for research documents in `<main-checkout>/docs/research/`, then reconcile the bounded GREEN/YELLOW/RED summaries into a ranked staleness, contradiction, and correction-fact report. This skill is detection-only: it may make the next doc-patch pass easy, but it never edits audited docs itself and does not write replacement prose for audited docs.

Documentation truth bar: verified mismatches in fields, offsets, operators,
ordering, bytes, or pixels remain wrong and must not be rewritten as exact
equivalence. This audit verdict is separate from current skirmish fix priority.

Label-adversarial Ghidra rule: local Ghidra names, labels, comments, xref
labels, and decompiler-assigned symbol names are navigation hints only. This
project may contain polluted or stale labels from earlier scripts. Dispatch and
reconciliation must treat address plus verified role as stronger than label name,
and must surface label drift clusters as a root-cause pattern.

## Hard Gates

- Dispatch at most 3 subagents at once, or fewer when collaboration slots are
  unavailable. Split larger queues into sequential waves.
- Use subagents only after the user explicitly invokes `/verify-doc-swarm` or otherwise asks for parallel/delegated doc audits.
- Each subagent must use Ghidra MCP read-only. Forbid `rename_function`, `rename_function_by_address`, `rename_data`, `rename_variable`, `create_label`, `create_function`, `add_struct_field`, `modify_struct_field`, `set_decompiler_comment`, `set_plate_comment`, `set_disassembly_comment`, `save_program`, and any mutating operation.
- Each subagent may write exactly one file: append one line to `<main-checkout>/docs/research/AUDIT_LOG.md`, matching `/verify-doc` rules.
- Subagents must not modify the audited doc, create corrected copies, write `.rs` files, touch INI files, or modify any other location.
- The parent may write only `<main-checkout>/docs/research/.audit-swarm-claims.md` and `<main-checkout>/docs/research/.audit-coverage-index.md`.
- Reconciliation never applies doc rewrites or code fixes.
- If more than half of a wave fails, stop before reconciliation and ask the user whether to retry failed slots, review successes only, or abort.
- Treat GREEN on a large/high-risk doc with suspicion until spot-checked.
- Every returned finding must distinguish verified binary evidence from inference, and every YR-activity assertion must say `Active in YR: Yes / No / Conditional` with evidence or default-gate context.
- If an audit discovers that the target is structurally wrong enough to require rebuilding the analysis rather than patching isolated claims, recommend a bounded `/re-investigate` target. Do not let `/verify-doc-swarm` silently become a re-investigation.
- Before and after dispatch, check that only allowed files changed: `AUDIT_LOG.md`, `.audit-swarm-claims.md`, and `.audit-coverage-index.md`. Any audited-doc edit, Rust edit, INI edit, or Ghidra mutation is a contract violation and must be reported before continuing.

## Status Rubric

Use the `/verify-doc` status meanings consistently across all slots:

- `GREEN`: zero WRONG, zero MISLEADING, and less than 5% UNVERIFIABLE among audited load-bearing claims. Safe to rely on for the audited scope.
- `YELLOW`: isolated wrong/stale/unverifiable claims, or a partial audit, but the doc remains usable with the listed caveats.
- `RED`: structural errors such as wrong primary function, wrong struct base/layout family, TS-legacy framed as standard YR, or broad pseudocode that would mislead implementation work. Recommend targeted rewrite or `/re-investigate`.

Tiny mismatches are never downgraded: off-by-one, `<` vs `<=`, signedness, wrong offset, wrong clamp, wrong default value, or wrong order-of-operations is `WRONG`.

## Invocation Modes

- `/verify-doc-swarm`: scan candidate docs, propose up to 3 targets, wait for confirmation.
- `/verify-doc-swarm <d1>, <d2>, ...`: validate explicit docs, then wait for confirmation.
- `/verify-doc-swarm --area <area>`: scan only one area, propose up to 3 targets, wait for confirmation.
- `/verify-doc-swarm --dry-run`: scan/validate and print the dispatch plan, then stop.
- `/verify-doc-swarm --refresh-index`: rebuild the audit coverage index before ranking.
- `/verify-doc-swarm --patch-plan`: run the normal audit swarm, then group findings into correction-fact bundles for a later patch pass. This still does not edit docs or generate replacement prose.

Valid areas include `combat`, `movement`, `locomotion`, `pathfinding`, `vision`, `shroud`, `power`, `production`, `placement`, `harvester`, `miner`, `building`, `infantry`, `unit`, `aircraft`, `sidebar`, `shell`, `audio`, `animation`, `weapon`, `superweapon`, `terrain`, `map`, `ai`, `radio`, `timer`, `core-engine`, and `ini-parsing`. Treat area names as aliases: for example `harvester` and `miner` both include `miner/` docs and refinery-dock reports.

If an explicit list has more than 3 docs, split it into sequential waves. If the
user supplies a system name like `garrison`, resolve it to concrete research docs
before dispatching.

## Step 0: Preflight

Before scanning or proposing slots:

1. Confirm `<main-checkout>/docs/research/` exists.
2. Confirm `AUDIT_LOG.md` exists or can be created by `/verify-doc` rules.
3. Confirm Ghidra MCP read-only access is available. If unavailable, stop and report the blocker.
4. Confirm a subagent tool is available. If unavailable, offer to run serial `/verify-doc` audits instead of pretending to swarm.
5. Read `.audit-swarm-claims.md` if present and identify active `claimed` rows less than 4 hours old. Do not propose the same doc while a fresh claim exists unless the user explicitly overrides.
6. Capture a cheap write-integrity baseline for `docs/research/`, repo `src/`, repo `ini/`, and the Ghidra mutating-tools ban. A simple `git diff --stat` plus doc mtimes is enough; this is to catch unexpected writes after dispatch, not to police unrelated pre-existing changes.
7. Use the research-index MCP as the first candidate/source map. Prefer
   `research_validate`, `research_map`, and `research_related`; use the repo-local
   CLI only for a missing MCP capability:

   ```text
   python tools/research_index/validate.py --system <system> "<topic>" --limit 12
   python tools/research_index/map.py --system <system> "<topic>" --limit 12
   ```

   If the exact system is unclear or returns zero docs, rerun without
   `--system`, then use the inferred exact system (for example `skirmish-ui`,
   `bridges`, `miner`, `chrono`). Use stale/unknown docs, missing links, and
   contradiction signals as candidate-ranking inputs. The index does not replace
   `/verify-doc`; it only chooses better audit targets.

## Step 1: Build Or Read The Audit Index

For auto, scoped, and refresh modes, use `<main-checkout>/docs/research/.audit-coverage-index.md`.

- If absent, rebuild it.
- If present and modified within 7 days, use it and tell the user the cache date.
- If older than 7 days or `--refresh-index` was passed, rebuild it and tell the user.

When rebuilding, recursively walk every `*GHIDRA_REPORT.md` and other RE/research docs in `docs/research/`, including subdirectories such as `miner/`, `units/`, and `traces/`. Exclude plans, design specs, generated correction-only files such as `*_VERIFY_DOC_AMENDMENTS.md`, and patch-cluster notes unless explicitly requested. For each doc, extract:

- Last audit: most recent `AUDIT_LOG.md` entry for the filename, with date and GREEN/YELLOW/RED status. No entry means never audited.
- Patch status: most recent PATCHED/PATCHED-to-GREEN/PATCHED-to-YELLOW line for the filename, if any. Treat a doc patched after its last YELLOW/RED audit as `POST_PATCH_NEEDS_VERIFY` unless a later GREEN audit exists.
- Doc mtime: if the doc was edited after its last audit, it is post-audit drift.
- Cross-reference count: grep the rest of `docs/research/` for the filename, and grep repo `src/` for citations across `sim`, `rules`, `map`, `assets`, `render`, `sidebar`, `ui`, and `audio`.
- Surface-area proxy: count address citations like `0x00XXXXXX`, `+0x` offsets, `FUN_` references, function-name references, vtable-slot mentions, and INI key mappings.
- TS-legacy density: count callouts like `inherited from Tiberian Sun`, `SpecialFlags`, `FogOfWar`, `Tunnel`, `Subterranean`, and `Veins`.
- Implementation-load proxy: count references to `src/`, `Implementation Handoff`, `Current Rust`, `Required implementation effect`, and proposed test names.
- Recent edit signal: edited in the last 14 days and last audit predates the edit.

Write one flat Markdown bullet per doc:

```markdown
- NEVER_AUDITED | RADIO_PROTOCOL_GHIDRA_REPORT.md | mtime=2026-04-12 | xref=6 | offsets=38 | ts-density=2
- POST_AUDIT_DRIFT | BUILDING_POWER_GHIDRA_REPORT.md | last=2026-04-20 GREEN | mtime=2026-05-15 | xref=4 | offsets=29
- POST_PATCH_NEEDS_VERIFY | POWER_SYSTEM_GHIDRA_REPORT.md | last=2026-05-20 YELLOW | patched=2026-05-20 | xref=5 | offsets=52
```

Categories, highest leverage first:

- `STALE_RED`: last audit was RED.
- `POST_PATCH_NEEDS_VERIFY`: patched after a non-GREEN audit and not yet re-audited GREEN.
- `POST_AUDIT_DRIFT`: doc edited after its last audit.
- `NEVER_AUDITED`: no `AUDIT_LOG.md` entry.
- `STALE_YELLOW`: last audit YELLOW and older than 30 days.
- `STALE_GREEN_90D`: last audit GREEN and older than 90 days.
- `FRESH_GREEN`: audited GREEN within 30 days.

Rank candidates:

- HIGH: `STALE_RED`, `POST_PATCH_NEEDS_VERIFY`, `POST_AUDIT_DRIFT` with xref >= 3, `NEVER_AUDITED` with xref >= 5, implementation-load >= 3, or ts-density >= 3.
- MEDIUM: `NEVER_AUDITED` with xref 1-4, `STALE_YELLOW`, or `STALE_GREEN_90D` with xref >= 5.
- LOW: isolated `STALE_GREEN_90D` or xref 0-1 docs.

Break ties by highest xref count, highest implementation-load, highest offset/address count, then newest mtime when last audit predates it. Spread picks across 3-5 subsystems unless `--area` was passed. Prefer canonical report docs over amendment-only docs; prefer docs with implementation handoffs when the user is preparing implementation work.

## Step 1.5: Validate Explicit Targets

For each explicit target:

1. Resolve it to a concrete file under `docs/research/`. If ambiguous, list candidates and ask.
2. Confirm it is a research/RE doc, not a plan or design spec. Ask whether to drop questionable slots.
3. Grep `AUDIT_LOG.md` for an entry from today. If found, ask whether to skip, re-audit, or scope to a subsection.
4. If the doc was edited in the last 6 hours, confirm before auditing a moving target.
5. If the target is an amendment, patch-cluster note, or audit-index file, ask whether the user meant the canonical source report instead.

## Step 2: Propose And Confirm

Print a table with slot, doc, category, last audit, patch status, xref, offsets, implementation-load, and tier. Wait for explicit confirmation such as `go`. A previous confirmation in the same thread does not count for a new swarm. For `--dry-run`, stop after printing the plan.

```markdown
Proposed audit-swarm dispatch (5 slots):

| Slot | Doc | Category | Last Audit | Patch | xref | offsets | impl | Tier |
|------|-----|----------|------------|-------|------|---------|------|------|
| 1 | MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md | STALE_RED | 2025-12-03 RED | none | 11 | 51 | 4 | HIGH |
```

## Step 3: Append Work Claims

Append to `<main-checkout>/docs/research/.audit-swarm-claims.md`. Create it if missing with:

```markdown
# Verify-Doc-Swarm Work Claims

Active and historical claims from /verify-doc-swarm invocations. Prevents parallel
sessions from duplicating audit work. Append-only; stale claims (>4h `claimed`)
may be re-claimed but rows are not deleted.
```

Append one block per swarm:

```markdown
## Audit-Swarm 2026-05-20T14:22 (session <short-id>)

- 2026-05-20T14:22 - slot-1 - MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md - claimed
```

Statuses are append-only events: `claimed`, `done`, and `failed`. Do not edit or delete old rows. A `claimed` row older than 4 hours is stale and may be re-claimed by appending a new `claimed` row. Completion is recorded by appending a new row, not by rewriting the old row:

```markdown
- 2026-05-20T14:22 - slot-1 - MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md - claimed
- 2026-05-20T15:03 - slot-1 - MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md - done (YELLOW, 2 WRONG)
```

## Step 4: Dispatch Subagents

Use parallel `spawn_agent` calls where available. Do not set an unavailable model override; inherit the active model unless the user explicitly requested a supported model. Each subagent receives one concrete doc path and a self-contained prompt:

```text
You are subagent slot {{SLOT}} of a /verify-doc-swarm batch. Your single target is:

  {{DOC_PATH}}

Run /verify-doc on this document with these added constraints:

HARD CONSTRAINTS:
- Ghidra MCP is READ-ONLY. Do NOT call rename_function, rename_function_by_address,
  rename_data, rename_variable, create_label, create_function, add_struct_field,
  modify_struct_field, set_decompiler_comment, set_plate_comment,
  set_disassembly_comment, save_program, or any other mutating tool.
- You may write EXACTLY ONE file: append one line to
  <main-checkout>/docs/research/AUDIT_LOG.md.
  Do NOT modify the audited research doc. Do NOT create a corrected copy. Do NOT
  write .rs files. Do NOT touch INI files. Do NOT modify any other location.
- Tiny mismatches count as WRONG. A < vs <= mismatch is WRONG. An off-by-one is
  WRONG. An offset or clamp mismatch is WRONG.
- Read literally: compare operators character-by-character, constants digit-by-digit,
  offsets byte-by-byte.
- Apply the TS-vs-YR filter: every Active in YR assertion must be re-verified
  against default gating flags and live YR caller paths.
- Do not audit sibling docs. Note contradictions for the parent; do not chase them.
- Use `/verify-doc` claim-selection rules: audit roughly 15-25 load-bearing claims
  unless the doc is smaller, and name any deliberately scoped-out sections.
- If the doc is structurally broken enough that isolated corrections would be
  misleading, mark it RED and recommend a bounded `/re-investigate` target instead
  of rebuilding the doc inside this audit.

RETURN VALUE CONTRACT:
1. A YAML front matter block:
   ```yaml
   slot: {{SLOT}}
   doc: {{DOC_FILENAME}}
   status: GREEN | YELLOW | RED
   result: COMPLETE | PARTIAL | FAILED
   audit_log_appended: true | false
   load_bearing_audited: N
   confirmed: X
   wrong: Y
   stale: Z
   unverifiable: W
   misleading: V
   needs_reinvestigate: true | false
   ```
2. Exact AUDIT_LOG.md line appended, verbatim.
3. Bounded Markdown summary, no more than 150 lines:
   - Doc filename and status.
   - Load-bearing claim count: N audited (CONFIRMED: X, WRONG: Y, STALE: Z,
     UNVERIFIABLE: W, MISLEADING: V).
   - Top 5 most-impactful WRONG/MISLEADING findings with evidence: Ghidra address,
     offset, exact quoted doc text, exact binary value.
   - For each WRONG/MISLEADING finding, classify: verified binary fact / inference,
     player-visible impact, likely downstream docs, and whether it is patchable or
     needs `/re-investigate`.
   - Correction facts when safe: section or quote anchor, exact verified binary
     fact, and evidence. Do not write replacement prose and do not edit the doc.
   - Sibling-doc contradictions noticed in passing.
   - Subsections deliberately scoped out.
4. One-line status: COMPLETE | PARTIAL (reason) | FAILED (reason).

Do not include full claim tables, raw decompilation, raw INI dumps, or narrative
doc recaps. If blocked, append a FAILED AUDIT_LOG line and return FAILED.
```

Wait for all returns, unless a timeout produces a failed slot. Do not reconcile until every slot is complete, partial, or failed.

## Step 5: Intake Returns

For each slot:

- Verify the returned `AUDIT_LOG.md` line was actually appended.
- Append a new `.audit-swarm-claims.md` row marking the matching slot `done` or `failed`; do not rewrite the original `claimed` row.
- Tally success/failure counts and GREEN/YELLOW/RED status counts.
- Treat malformed returns as PARTIAL: over 150 lines, missing YAML front matter, missing audit-log line, missing status, missing findings list, or missing scoped-out section list.
- Flag GREEN-on-fat-target when xref >= 5, offset count >= 30, or ts-density >= 2 and there are zero WRONG/MISLEADING findings.
- Flag RED-needs-reinvestigate when `needs_reinvestigate: true`, when the primary function/address family is wrong, or when a struct-table sample invalidates the whole table.
- Run the post-dispatch write-integrity check. If any audited doc, Rust file, INI file, or disallowed docs file changed, stop and report the contract violation before reconciliation.

If 3 or more slots failed, stop and ask the user how to proceed.

## Step 6: Reconcile

For each successful or partial audit:

1. Read the bounded summary carefully.
2. Cross-reference WRONG/MISLEADING findings against sibling docs by grepping `docs/research/` for the same address, offset, INI key, or function.
3. Pattern-match across slots: TS-legacy mislabels, vtable slot shifts, offsets off by a constant, `param_1` `int` vs `int *` confusion, same-address collisions, function-entry vs interior-address confusion, stale Ghidra labels, and repeated Rust-status drift.
4. Spot-check one GREEN slot by reading its `AUDIT_LOG.md` line and independently verifying one load-bearing function or claim against Ghidra/docs. If it disagrees, demote to flagged YELLOW in the report.
5. Check `AUDIT_LOG.md` history for oscillating docs such as RED -> GREEN -> RED or YELLOW -> PATCHED -> YELLOW.
6. Build a correction-fact queue from safe findings only:
   - Include exact doc quote or section anchor.
   - Include exact verified binary fact.
   - Include binary evidence and whether the parent spot-checked it.
   - Group related fixes by target doc.
   - Mark `needs_reinvestigate` instead of patch-ready when the correction requires rebuilding a model, tracing a new caller chain, or resolving a broad YR-activity question.
7. If `--patch-plan` was requested, add an ordered non-editing patch plan: docs to patch first, findings that need parent spot-check before patching, sibling docs to verify afterward, and single-doc `/verify-doc` commands to rerun. Do not include replacement prose.

Do not edit audited docs during reconciliation.

## Step 7: Report

Return a concise report with:

- Status: complete count, failed count, GREEN/YELLOW/RED mix, contradiction count.
- Audits completed: one line per slot with doc, status, and key finding.
- Highest-impact WRONG/MISLEADING findings ranked by player visibility times downstream load.
- Cross-doc contradictions, without resolving them unless the evidence was directly checked.
- Correction-fact queue grouped by doc, with evidence and spot-check state.
- RED docs that should go to `/re-investigate` instead of isolated patching.
- Cross-slot patterns worth investigating before isolated doc edits.
- Suspicion flags such as GREEN-on-fat-target and whether the spot-check passed.
- Subagent contract violations.
- Failed slots and proposed single-doc `/verify-doc` retry commands.
- Verbatim `AUDIT_LOG.md` lines appended.
- Next steps for the user to choose from.

Stop after the report. Do not start another swarm, run `/verify-doc` on contradictions, or edit docs unless the user asks.

## Fit Notes For This Project

- Prefer verifying canonical `*_GHIDRA_REPORT.md` files before derivative amendment files. Amendment docs are useful evidence, but they can hide whether the source report is now safe.
- Many local docs are patch-cluster workflows: YELLOW audit -> PATCHED-to-GREEN -> disparity scan. The swarm should preserve that cadence by outputting correction facts and anchors, not by editing docs itself.
- Re-investigation reports include `Coverage Ledger`, `Open Questions`, and `Implementation Handoff`. When auditing those docs, treat all three as load-bearing: a stale handoff or open-question state can mislead implementation just as much as a wrong offset.
- Typical high-risk local failure patterns are `param_1` pointer-index offsets, function-entry vs interior-address citations, TS legacy treated as live YR, same field given different names across sibling docs, Ghidra label drift, and Rust status sections drifting after implementation work. Bias candidate ranking and reconciliation toward those patterns.
- For implementation-facing docs, a GREEN audit is not enough if the Rust-status section was not checked. Either spot-check the cited Rust surface or mark Rust status as deliberately out of scope.
