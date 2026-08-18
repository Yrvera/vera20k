# /decode-system smoke test — post-mortem

**Date:** 2026-05-24
**Target:** iron-curtain (Iron Curtain super weapon system)
**Outcome (revised after disk inspection):** Halted at the user's direction after early contract violations were detected, but the team continued working in the background and produced substantial real output before all teammates shut down.
**Status:** v1 had real failure modes (3 phantom completions out of 19 decode tasks, role violations, persistent shutdown unresponsiveness), but ALSO produced 16 high-quality decode docs and a 32-row parity report with multiple real HIGH-leverage DRIFT findings. The v2 SKILL.md changes are still warranted — they fix the failure modes that occurred — but the smoke test was a partial success, not a failure.

### Revised quantitative outcome

- Decode tasks: 19 dispatched, 16 produced real on-disk docs (84%), 3 phantom completions (16%): #5 (StartFidget), #6 (ReadCombatDamage), #7 (ReadGeneral)
- Compare tasks: 19 dispatched; `_parity.md` contains 32 rows (some tasks produced multiple rows, some are missing — exact correspondence not audited per-task)
- Real findings (HIGH player-visibility × frequency): 5 (see "Salvaged findings" section)
- Contract violations observed: 3 (phantom completions, decoder-role-spillover, manifest authority)
- Shutdown unresponsiveness: 4 of 5 teammates did not honor shutdown_request promptly

---

## TL;DR

The `/decode-system` skill's design (anchor → scope crawl → team setup → spawn) executed cleanly through Steps 1–3. Team primitive (`TeamCreate` + 5× `Agent` spawn) worked correctly. **Failure was at the runtime layer**: spawned teammates failed to honor their contracts within 90 seconds of spawn — marking tasks completed without producing output files, claiming tasks outside their declared role, and modifying manifest fields outside their write scope.

Three concrete failure modes captured. All are addressable in skill v2.

---

## What worked

| Phase | Component | Status |
|---|---|---|
| Step 1 | Anchor identification (3 seeds via search_strings + search_functions_enhanced) | ✓ Clean |
| Step 1 | Duplicate-check (caught CRATE_SYSTEM existing doc on initial target choice) | ✓ Worked as designed |
| Step 1 | Rust-counterpart pre-check (confirmed `src/sim/superweapon/iron_curtain.rs` exists) | ✓ Clean |
| Step 2 | BFS scope crawl with TS-filter | ✓ Produced 19-symbol manifest |
| Step 2 | Preflight decompiles surfaced 2 real findings (InfantryClass IC = instakill; StartFidget misnamed) | ✓ Real research value |
| Step 3 | TeamCreate + 40 TaskCreate (parallel) + 19 TaskUpdate (blockedBy dependencies) | ✓ All succeeded |
| Step 4 | Spawn 5 teammates in parallel (1 scope-explorer + 3 decoders + 1 rust-comparer, sonnet) | ✓ All spawned successfully |
| Step 5 | scope-explorer monitor + auto-deliver of teammate messages | ✓ Detected violations within 90s |

The scaffolding works. The contracts in the spawn prompts do not.

---

## What failed

### Failure 1 — Decoder marks task completed without writing output file

**Observed on:** task #5 (decode-fn-StartFidget_misnamed_dispatch), task #6 (decode-fn-ReadCombatDamage_IC_Duration).

**Evidence:**
- TaskList showed status `completed` for both within ~30s of spawn.
- Filesystem inspection (`ls iron-curtain/`) showed only `_manifest.yaml`. No `fn-StartFidget-IronCurtain-Dispatch.md`, no `fn-ReadCombatDamage-IronCurtainDuration.md`.
- The decoder spawn prompt explicitly required: "Write the doc to the task's output path" (step 5 of PRIMARY LOOP). It also said: "Cite EVERY Ghidra MCP call inline."
- The prompt did NOT include a verification step ("confirm the output file exists before marking completed").

**Root cause hypothesis:** The decoder agent's primary loop mechanically advanced through claim → in_progress → completed without actually executing the runbook's decompile + write steps. Possible contributing factors:
1. The spawn prompt's PRIMARY LOOP was readable as a checklist; the agent may have completed the checklist symbolically rather than functionally.
2. No "exit gate" requiring the output file's existence before TaskUpdate(completed).
3. The runbook templates (function-decode-v1 etc.) describe what the doc should contain but don't enforce file write as a step.

### Failure 2 (RECLASSIFIED) — Task assignment routing mis-routed a decode task to scope-explorer

**Initial observation:** task #7 (decode-fn-ReadGeneral_IC_InvokeAnim) was in_progress with owner = scope-explorer.

**Updated finding after reading scope-explorer's messages:** scope-explorer did NOT self-claim task #7. It RECEIVED a `task_assignment` message routing #7 to it, then explicitly reported the mis-routing back to team-lead: "I am not a decoder — I cannot run Ghidra decompiles." It correctly maintained its monitor role despite the assignment.

**This means scope-explorer worked as designed.** It also independently diagnosed the same systemic failures team-lead caught (tasks #5/#6 completed with no output, manifest not updated) and reported them upward. The monitor role is sound.

**Real root cause (unknown):** Something assigned task #7 to scope-explorer. Possible culprits:
1. The team primitive has an auto-assignment mechanism that fires `task_assignment` messages based on owner field changes or content matching. Team-lead created tasks with `owner: null` so this is unexpected.
2. Another teammate (decoder or rust-comparer) called `TaskUpdate(taskId=7, owner=scope-explorer)` — possibly by accident if their TaskList interpretation routed scope-explorer's name from somewhere.
3. The TaskUpdate mechanism may auto-fire assignment messages based on subject parsing or runbook content.

**Action for v2:** investigate the assignment routing mechanism. Add a hard prohibition to scope-explorer's prompt anyway:
```
HARD PROHIBITION: If you receive a task_assignment message for any task whose
subject does NOT start with "explorer-", you MUST decline by sending a message
back to team-lead identifying the mis-route. Do NOT TaskUpdate(owner=self) on
the misrouted task. Do NOT do the work even partially.
```

scope-explorer did exactly the right thing in this run by declining and reporting — but it still showed up as owner of #7 in TaskList, which means the assignment mechanism wrote owner=scope-explorer before scope-explorer could refuse. v2 needs to either fix that at the team primitive layer or instruct scope-explorer to TaskUpdate(owner=null) immediately on mis-routed assignments.

### Failure 3 — Teammate writes outside its scope in _manifest.yaml

**Observed:** task #5's manifest entry was modified to include `status: completed` AND a `compare-result: DRIFT` field — both written by a teammate that shouldn't have done so.

**Evidence:**
- The `compare-result` field belongs on COMPARE tasks (#20-38), not decode tasks (#1-19). Whoever wrote it conflated the two.
- rust-comparer's prompt scoped its writes to `_parity.md` (append) + the compare task's result field. It should not have edited a decode task's manifest entry.
- scope-explorer's prompt scoped its writes to manifest entries for `excluded-ts-legacy` and `scope-explorer-events`. It should not have edited task entries.

**Root cause hypothesis:** The manifest YAML is shared mutable state with no field-level write authority enforced. Multiple teammates write to the same file. The skill spec called this out as a pre-launch concern ("Manifest race conditions") but didn't enforce field-level scoping in the spawn prompts.

### Failure 4 — rust-comparer claims compare task whose decode produced no output

**Observed:** compare-fn-StartFidget (#24) was in_progress (owned by rust-comparer) despite decode #5 having no output file.

**Evidence:**
- The blockedBy mechanism uses task STATUS only (#5 marked completed → #24 unblocked).
- It does NOT verify the output file exists.
- rust-comparer claimed #24 and started work; would have failed when reading the non-existent decode doc.

**Root cause hypothesis:** Task dependency model assumes "completed = output exists." That assumption is violated by Failure 1. Two fixes possible: (a) rust-comparer prompt verifies the decode file exists before starting; (b) decoder's exit gate prevents marking completed without the file.

### Failure 5 (low severity) — decoder-3 holds two in_progress tasks

**Observed:** decoder-3 owned both #3 and #4 as in_progress simultaneously.

**Evidence:** Spawn prompt said "Prefer lowest task ID" but didn't enforce one-task-at-a-time.

**Root cause hypothesis:** Decoder PRIMARY LOOP step "Loop to step 1" doesn't say "but only after marking the current task completed." Agent claimed a second task before finishing the first.

---

## Skill v2 changes needed

### Change 1 — Decoder exit gate: file-write verification

Add to decoder spawn prompt, between step 6 (update manifest) and step 7 (TaskUpdate completed):

```
6.5. EXIT GATE: Before calling TaskUpdate to mark this task completed, you MUST:
     - Read the output file you just wrote. If Read fails, the file doesn't exist
       and you MAY NOT mark the task completed. Re-execute the runbook.
     - Verify the file has >= 50 lines of content. A stub doc fails the gate.
     - Verify the doc contains at least 3 inline Ghidra MCP citations (search for
       "decompile_function" or "get_xrefs" or "read_memory" in the doc body).
     - If any verification fails: do NOT mark completed. Report what's missing and
       re-execute the missing steps.
```

### Change 2 — Role-exclusive task-kind enforcement

Add to scope-explorer spawn prompt:

```
HARD PROHIBITION: You MAY NOT call TaskUpdate to set owner=scope-explorer on any
task with kind != "explorer" or subject not starting with "explorer-". If you find
yourself about to claim a decode-* or compare-* task, STOP. That's a decoder or
rust-comparer task. Go idle instead.
```

Add to decoder spawn prompts:

```
HARD PROHIBITION: You MAY ONLY claim tasks whose subject starts with "decode-".
Never claim compare-*, synthesize-*, verify-*, or any other prefix.
```

Add to rust-comparer spawn prompt:

```
HARD PROHIBITION: You MAY ONLY claim tasks whose subject starts with "compare-".
```

### Change 3 — Field-level manifest write authority

Document the manifest YAML write authority per role, AND inject the authority list into each spawn prompt:

| Role | Allowed to write |
|---|---|
| scope-explorer | `excluded-ts-legacy`, `scope-explorer-events`, `scope-explorer-state`, `backoff-since`, `ceiling-hit`, `scope.current_size` |
| decoder | Within the matching `tasks[id]` entry only: `status`, `owner`, `out-of-scope-refs`. Nothing else in the manifest. |
| rust-comparer | Within the matching `tasks[id]` entry only: `status`, `owner`, `result`. Nothing else. |
| team-lead | Anything. |

### Change 4 — Compare task verifies prerequisite output exists

Add to rust-comparer spawn prompt PRIMARY LOOP between step 3 (read decode doc) and step 4:

```
3.5. EXIT GATE: Confirm the decode doc exists (Read succeeds, file has >= 50 lines).
     If not, the prerequisite decode is broken. STOP. Send message to team-lead:
     "Decode prerequisite #X has no usable output. Cannot proceed with compare task."
     Do not mark the compare task failed; leave it for team-lead to recover.
```

### Change 5 — Decoder one-task-at-a-time enforcement

Add to decoder spawn prompts:

```
ONE TASK AT A TIME: You MAY hold at most ONE task in_progress (owner=self).
Before claiming a new task, your previous task MUST be marked completed (with
the exit gate passed) OR explicitly released back to pending. If you find
yourself with two in_progress tasks, that's a bug — release the second one.
```

### Change 6 — Team-lead spot-check in real time

Add to skill Step 5:

```
After each decoder reports task completion, team-lead MUST:
1. Verify the output file exists via Read.
2. Spot-check at least 1 inline Ghidra MCP citation in the doc by re-running
   the cited call and confirming the result.
3. If verification fails, mark the task back to pending and message the decoder
   with the specific failure. Do not let the team advance with phantom completes.
```

This catches the failure mode at human-review latency (~30s) rather than at synthesis time (hours later).

### Change 7 — Runbook templates require explicit "write file" step

The runbook templates in the skill describe the doc structure but don't enforce file creation. Add an explicit step 0 to every runbook template:

```
STEP 0 — file initialization. Before doing any Ghidra work, write a stub file at
the output path with the doc's section headers (Summary, Active in YR, etc.).
This proves the write authority works AND establishes the file. Each subsequent
step appends to or fills sections in that file.
```

---

## What we did salvage

Two real research findings produced during team-lead's preflight (before spawn). These are HIGH-confidence and ready to roll into a non-decode-system doc:

1. **InfantryClass IronCurtain = instakill, not invulnerability.** At `0x00522600`, the function reads `param_1[0x1b0]` (byte offset 0x6C0 — owning house ptr), then calls vtable slot 0x16c (likely `TakeDamage`) with damage = `*(g_RulesClass_Instance + 0xfa8)`. Per CLAUDE.md `int *` pointer-arithmetic rule. Verified via `decompile_function 0x00522600`.

2. **`TechnoClass__StartFidget` (0x004deae4) is misnamed in Ghidra.** Body is the Iron Curtain super-weapon dispatch: detects type-flag deflect (`TechnoTypeClass+0xd97`), handles chrono warp detach, sets timestamps, calls `TechnoClass__IronCurtain`. Verified via `decompile_function 0x004deae4`. Rename recommendation: `TechnoClass__ApplyIronCurtain` or `TechnoClass__IronCurtain_Dispatch`. No auto-rename (read-only).

These should be moved to a regular GHIDRA_REPORT doc in `ra2-rust-game-docs/` rather than left in the iron-curtain/ subdirectory tied to the failed decode-system run.

---

## Verdict on /decode-system skill v1

**Conceptual design is sound.** Pipeline phases (anchor → scope → tasks → decode → compare → synthesis → verify) work in principle, and Steps 1–3 + spawn executed cleanly.

**Runtime layer is broken.** Spawn prompts were descriptive, not enforcing. Spawned teammates ran a symbolic version of the contract (mark tasks complete) without the functional substrate (write files, decompile, verify). The 7 specific changes above plug the holes that emerged in this run.

**Cost so far:** ~10 minutes of team-lead preflight (productive — produced 2 real findings), ~90 seconds of team runtime (unproductive — produced 0 docs and 5 contract violations), ~5 minutes of post-mortem (this doc).

**Recommendation:** Apply changes 1-7 to `.claude/skills/decode-system/SKILL.md`, then re-run the smoke test on a different small target before betting on the skill for a larger system.
