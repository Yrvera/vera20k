---
name: trace-swarm
description: "Project-scoped for VERA20k/RA2 Yuri's Revenge only. Use when the user invokes '/trace-swarm', asks to orchestrate parallel end-to-end parity traces, wants a bounded multi-mechanic audit across mature movement/combat/vision/power/production/placement/sidebar/audio/render-order systems, or asks to reconcile multiple /trace-action reports. Dispatches at most 3 read-only subagents per wave, each running /trace-action on one concrete player-visible scenario, then ranks FAIL/UNCHECKED results by player visibility and frequency. Do not use first for broad, under-implemented UI/menu/setup surfaces. Never auto-applies fixes."
---

# Trace Swarm

Coordinate up to 3 parallel `/trace-action` investigations per wave for concrete RA2/YR mechanics, then reconcile their reports into a ranked disparity list. This skill is detection-only: do not edit Rust, INI, tracked/published docs outside `docs/research/`, or Ghidra state while running a swarm.

Exact trace verdict bar: active-gamemd mechanism, byte state, and
pixel/result output. Keep DRIFT honest, then assign the separate milestone
delivery class.

For the current retail-convincing milestone, default candidate selection to
concrete ordinary-skirmish interactions with high frequency, player
noticeability, loop breadth, compounding, or unblock value. Keep the exact trace
verdict, but reconcile bounded expert-only findings under exactification
residuals unless the user explicitly requests an exact audit wave.

## Hard Gates

- Run the Suitability Gate below before proposing or dispatching any swarm, even when the user explicitly asks for trace-swarm.
- Dispatch at most 3 subagents at once. Split larger queues into sequential waves.
- Use subagents only after the user explicitly invokes `/trace-swarm` or otherwise asks for parallel/delegated tracing.
- Each subagent must use Ghidra MCP read-only. The prompt must forbid `rename_function`, `rename_function_by_address`, `rename_data`, `rename_variable`, `create_label`, `create_function`, `add_struct_field`, `modify_struct_field`, `set_decompiler_comment`, `set_plate_comment`, `set_disassembly_comment`, `save_program`, and any other mutating operation.
- Each subagent may write exactly one trace report under `<main-checkout>/docs/research/traces/` plus the shared claims file. Nothing else.
- The parent may write only the claims file. Reconciliation never applies code or docs fixes.
- If 2 or more subagents in a wave fail, stop before reconciliation and ask the user whether to retry failed slots, review successes only, or abort.
- If a subagent returns all-PASS on a non-trivial trace, treat it as suspicious until spot-checked. PASS requires literal computed equality, not intuition.

## Suitability Gate

Trace-swarm is for parallel confirmation of several independent, concrete, player-visible mechanics. It is the wrong first tool for broad feature surfaces, especially UI/menu/setup screens where many slots will only rediscover the same missing parent system.

Before Step 1, do a quick read-only triage:

1. Restate the user's target as either:
   - **Concrete trace set**: 2-3 independent actions with implemented Rust surfaces and gamemd evidence to compare.
   - **Broad feature surface**: a screen/system area such as "skirmish UI", "main menu", "sidebar", "lobby", "save/load", or "options" without concrete interactions.
2. Inspect recent code/docs enough to answer:
   - Is the feature on the normal player path, or dev-gated/stubbed?
   - Are the likely slots independent, or do they share one parent missing system?
   - Would at least 3 slots probably return mostly `NOT-IMPLEMENTED` for the same root cause?
3. If the answer is "broad feature surface" and either dev-gated/stubbed or dominated by one parent missing system, stop before dispatch. Report that trace-swarm is not ideal, summarize the root reason in 3-6 bullets, and recommend one of:
   - `/re-investigate <surface>` when active-YR behavior itself is not established.
   - `/brainstorm <surface>` then `/write-plan` when the next useful step is implementation design.
   - One focused `/trace-action <concrete action>` when a single interaction needs binary confirmation.
4. Only continue with trace-swarm if the user explicitly confirms after this warning with wording like `trace it anyway`, or if the triage finds 2-3 concrete, independent, implemented interactions.

For UI/menu/setup surfaces, default to **one of the alternatives** unless the target is narrow and mature. Good trace-swarm UI targets: "sidebar Repair/Sell button flash cadence", "power bar low-power blink", "main-menu owner-draw button pressed offset". Poor trace-swarm UI targets: "skirmish UI", "main menu screen", "options dialog", "lobby UI".

During this triage, use the research-index MCP first. Prefer `research_brief`; if
unavailable, use the repo-local CLI fallback:

```text
python tools/research_index/brief.py "<target>" --limit 8
```

If the exact system is known, add `--system <system>` (examples: `bridges`,
`miner`, `skirmish-ui`, `chrono`). If the result reports zero docs, rerun without
`--system`, then rerun with the inferred exact system. Use `map.py`, `search.py`,
and `graph.py implementation` to decide whether the target is a mature concrete
interaction or a broad surface better handled by another skill.

## Invocation Modes

- `/trace-swarm`: auto-scan candidate mechanics, propose up to 3 targets, wait for confirmation.
- `/trace-swarm <m1>, <m2>, ...`: validate explicit mechanics, restate each as a concrete scenario, wait for confirmation.
- `/trace-swarm --area <area>`: scan only one area, propose up to 3 targets, wait for confirmation.
- `/trace-swarm --dry-run`: scan/validate and print the dispatch plan, then stop.
- `/trace-swarm --refresh-index`: rebuild the trace coverage index before ranking.

Valid areas include `combat`, `movement`, `vision`, `power`, `production`, `placement`, `ui`, `sidebar`, `audio`, and `render-order`.

`ui` is valid only for mature, concrete interactions. Broad setup/menu screen requests must pass the Suitability Gate first and usually route to re-investigate/planning instead.

If an explicit list has one entry, use a single `/trace-action` unless a second slot adds an independent check. If it has more than 3 entries, split it into sequential waves of 3. Do not trace abstract targets like `movement` or broad surfaces like `skirmish UI`; ask for a concrete scenario such as `Conscript walks from (50,50) to (55,55) around a wall` or `click Choose Map on dialog 0x102 at 800x600`.

## Step 1: Build Or Read The Candidate Index

For auto, scoped, and refresh modes, use `<main-checkout>/docs/research/traces/.trace-coverage-index.md`.

- If absent, rebuild it.
- If present and modified within 7 days, use it and tell the user the cache date.
- If older than 7 days or `--refresh-index` was passed, rebuild it and tell the user.

When rebuilding, collect candidate player-visible mechanics from:

- Canonical mechanics: combat damage/range/splash/retaliation; ground/air/harvester/chrono/transport/scatter movement; shroud, gap, spy, minimap vision; power supply/demand and low-power transitions; queue/placement/sidebar production feedback; placement terrain/adjacency/foundation; sidebar/ui hit testing and flash cadence; EVA/sound cue ordering and panning; render order, brackets, health bars, cliff redraw.
- Recent code changes: `git log --since="14 days ago" --name-only -- src/sim src/render src/sidebar src/ui src/audio`, mapped back to player-visible mechanics.
- Existing trace reports: `*_TRACE.md` files, prioritizing prior FAIL/UNCHECKED reports not clearly re-traced and deprioritizing recently traced or NOT IMPLEMENTED reports.
- Open follow-ups: grep `<main-checkout>/docs/research/` and `docs/plans/` for `TODO: re-trace`, `deferred from trace report`, and similar markers.

Write one flat Markdown bullet per candidate:

```markdown
- CANONICAL | combat | "Grizzly vs Rhino at edge of range" | last traced: never | recent code: combat_aoe.rs (2d ago)
- PRIOR_FAIL | power | "power goes offline" | last traced: 2026-02-15 (UNCHECKED on radar disable timing) | recent code: none
```

Rank candidates by player visibility times frequency:

- HIGH: visible/audible/behavioral result, fires in most matches, or has unresolved prior FAIL.
- MEDIUM: highly visible but conditional, or subtle but frequent.
- LOW: niche edge cases, intact recent traces, or no relevant code drift.

Break ties by most recent touched code, oldest prior trace, then subsystem diversity.

## Step 2: Propose And Confirm

Before dispatching, print a table with slot, mechanic, concrete scenario, source, and tier. Wait for an explicit confirmation such as `go`. A previous confirmation in the same thread does not count for a new swarm. For `--dry-run`, stop after printing the plan.

Example:

```markdown
Proposed trace swarm dispatch (3 slots):

| Slot | Mechanic | Scenario | Source | Tier |
|------|----------|----------|--------|------|
| 1 | Combat - point blank | Grizzly vs Rhino at 1 cell, no cover | CANONICAL | HIGH |
| 2 | Vision - shroud reveal | Conscript walks 5 cells through unexplored shroud | CANONICAL | HIGH |
```

## Step 3: Append Work Claims

Append to `<main-checkout>/docs/research/traces/.trace-swarm-claims.md`. Create it if missing with:

```markdown
# Trace-Swarm Work Claims

Active and historical claims from /trace-swarm invocations. Prevents parallel
sessions from duplicating work. Append-only; stale claims (>4h `claimed`)
may be re-claimed but rows are not deleted.
```

Append one block per swarm:

```markdown
## Trace Swarm 2026-05-20T11:42 (session <short-id>)

- 2026-05-20T11:42 - slot-1 - combat_grizzly_vs_rhino_pointblank - claimed
```

Statuses are `claimed`, `done`, and `failed`. Do not delete stale claims; a `claimed` row older than 4 hours may be re-claimed by appending a new row.

## Step 4: Dispatch Subagents

Use parallel Agent tool calls with `subagent_type: general-purpose` - Explore agents cannot write the report file. Agents run in the background and report back on completion; dispatch all slots before waiting. Do not set an unavailable model override; inherit the active model unless the user explicitly requested a supported model. Each subagent receives one concrete scenario and a self-contained prompt:

```text
You are subagent slot {{SLOT}} of a /trace-swarm batch. Your single mechanic is:

  {{MECHANIC}}

Concrete scenario (do not generalize, do not expand): {{SCENARIO}}

Run /trace-action on this exact scenario with these added constraints:

HARD CONSTRAINTS:
- Ghidra MCP is READ-ONLY. Do NOT call rename_function, rename_function_by_address,
  rename_data, rename_variable, create_label, create_function, add_struct_field,
  modify_struct_field, set_decompiler_comment, set_plate_comment,
  set_disassembly_comment, save_program, or any other mutating tool.
- You may write EXACTLY ONE file: {{REPORT_PATH}}. Do NOT modify .rs files, INI
  files, tracked/published docs outside `docs/research/`, or any other location.
- PASS at any stage requires literal numerical equality between our output and
  gamemd's. If you did not compute both, mark UNCHECKED, not PASS.
- Confirm every gamemd reference is active in standard YR, not dormant TS legacy.
- Ghidra display names/labels are NAVIGATION HINTS ONLY and may be stale or polluted.
  Before citing a function as gamemd evidence, confirm its identity from the body /
  callers / vtable (COL→TypeDescriptor read for any vtable claim). A wrong label
  yields a false FAIL or false PASS. If you cannot confirm identity, mark UNCHECKED
  and cite the address, not the name.
- Trace one mechanic and one concrete scenario only. Put adjacent findings in the
  report's Adjacent Findings section; do not trace them this run.

RETURN VALUE CONTRACT:
1. Markdown summary, no more than 150 lines.
2. Exact report file path.
3. Verdict tally: PASS: <n> | FAIL: <n> | UNCHECKED: <n> | NOT-IMPLEMENTED: <n>
4. Top 5 most player-visible FAIL or NOT-IMPLEMENTED findings, one line each,
   with stage, player-visible difference, our file:line, and gamemd evidence.
5. One-line status: COMPLETE | PARTIAL (reason) | FAILED (reason).

Do not include the full stage table, raw decompilation, raw INI dumps, or adjacent
findings in the return. If blocked, write what you have and return PARTIAL or FAILED.
```

Wait for all returns, unless a timeout produces a failed slot. Do not start reconciliation until every slot is complete, partial, or failed.

## Step 5: Intake Returns

For each slot:

- Verify the report exists.
- Update the matching claims row to `done` or `failed`.
- Treat malformed returns as PARTIAL: over 150 lines, no report path, missing tally, or missing status.
- Flag all-PASS/non-trivial traces for spot-checking.

If 2 or more slots failed, stop and ask the user how to proceed.

## Step 6: Reconcile

For each successful or partial report:

1. Read the report.
2. Tally PASS, FAIL, UNCHECKED, and NOT IMPLEMENTED stages.
3. Spot-check at least one PASS by rereading cited Rust/INI/research/Ghidra evidence and confirming both values were computed.
4. Spot-check at least one FAIL where present and confirm the gamemd value is cited, not invented.
5. Re-rank all failures by player visibility times frequency, not by count.
6. Detect cross-trace patterns: facing convention drift, shared render-stage UNCHECKED gaps, adjacent NOT IMPLEMENTED systems, or common parser/rules data errors.

Do not edit code or docs during reconciliation.

## Step 7: Report

Return a concise report with:

- Status: complete/partial/failed counts, reports written, disparity count, high-priority count.
- Reports written: one line per slot with path, status, and tally.
- Disparities ranked HIGH, MEDIUM, LOW by visibility times frequency, with evidence and trigger frequency.
- Cross-trace patterns worth investigating before isolated fixes.
- Pad-PASS audits and what was spot-checked.
- Subagent contract violations.
- Failed slots and proposed single-slot retry commands.
- Next steps for the user to choose from.

Stop after the report. Do not start another swarm, re-trace a failed slot, or patch code unless the user asks.
