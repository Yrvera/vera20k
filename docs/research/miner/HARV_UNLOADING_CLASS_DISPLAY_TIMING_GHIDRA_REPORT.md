# HARV UnloadingClass Display Timing - Ghidra Research Report

**Address(es):** `0x0073CEC0` (`UnitClass::DrawExtras`), `0x0073D630` (`UnitClass::Mission_Deploy_Building`), `0x00737430` (`UnitClass::Receive_Radio`), `0x007146E8` (`TechnoTypeClass::ReadINI` `UnloadingClass` block)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** stock YR War Miner (`HARV`) visual swap to `HORV` during standard refinery unload, including swap-on timing, swap-off timing, state-machine relation, and interrupted unload cleanup gates.
**Non-Scope:** full ore credit math, normal refinery exit locomotion, chrono miner teleport return, slave miner deploy harvesting, runtime screenshot capture.
**Confidence:** High for static binary ordering and render gate; Medium for exact same-frame screenshot visibility on missing-refinery abort because runtime frame capture was not performed.
**Active in YR:** Yes. `rulesmd.ini` `[HARV]` has `Harvester=yes`, `Dock=NAREFN,GAREFN`, `Storage=40`, `UnloadingClass=HORV`; `[HORV]` exists as a voxel unit type; `artmd.ini` `[HARV]` and `[HORV]` both have `Voxel=yes`.

## Target Question

When does stock YR render a War Miner as `HORV` instead of `HARV` while unloading at a refinery, when does it return to `HARV`, is the visual tied to `Mission_Deploy_Building` states, and what clears the visual on interruption?

## Non-Goals

- Do not re-open storage drain value, purifier bonus, or refinery owner credit routing.
- Do not re-open dock facing except for timing relative to the visual gate.
- Do not implement Rust.

## Evidence Needed To Mark COMPLETE

- Direct binary verification of `UnloadingClass=HORV` parsing and storage.
- Direct binary verification of the render-time swap gate.
- Direct binary verification of the `Mission_Deploy_Building` writes that set and clear the gate.
- Direct binary verification that first drain is later than the swap-on gate.
- Direct binary verification of interruption branches that clear or do not clear the gate.
- Current Rust surface scan for handoff/tests only.

## Stop Conditions

- Stop at HARV/HORV display timing; treat CMIN/CMON only as corroborating equivalent prior work.
- Stop when every branch in scope has either a direct address or an explicit deferral.
- Do not mutate Ghidra or any Rust/INI file.

## Key Binary Findings

| Finding | Active in YR | Evidence | Confidence |
|---|---:|---|---|
| `UnloadingClass` is parsed from the unit type INI section and stored at `TechnoTypeClass+0x6B8` as a `UnitTypeClass*`, not an anim name and not a VoxelAnimType. | Yes | `0x007146D3` saves old `+0x6B8`; `0x007146E8 PUSH "UnloadingClass"`; `0x007146F0 CALL ReadString`; `0x007146FD CALL UnitTypeClass__FindOrAllocate`; `0x0071471A MOV [EBP+0x6B8],EAX` | High |
| Stock `[HARV]` resolves `UnloadingClass=HORV`; `[HORV]` is a voxel unit type with no `UnloadingClass` of its own in retail data. | Yes | `rulesmd.ini` `[HARV]` line with `UnloadingClass=HORV`; `rulesmd.ini` `[HORV]`; `artmd.ini` `[HORV] Voxel=yes` | High |
| The HARV->HORV change is a render-time temporary type swap inside `UnitClass::DrawExtras`, not a permanent unit type change. | Yes | `0x0073D29C` save current `unit+0x6C4`; `0x0073D2C4 MOV [unit+0x6C4], Type+0x6B8`; draw dispatch; `0x0073D39B/0x0073D3A2` restore saved type to `unit+0x6C4` | High |
| The render gate is exactly: current type has `Harvester=yes`, `unit+0x6D1 != 0`, and current type `+0x6B8 != 0`. No current-mission check appears in `DrawExtras`. | Yes | `0x0073D2A6` reads `Type+0xE0E`; `0x0073D2B0` reads `unit+0x6D1`; `0x0073D2BA` reads `Type+0x6B8`; `0x0073D2C4` swaps | High |
| The first time HARV can render as HORV is after `Mission_Deploy_Building` dump-init writes `unit+0x6D1=1`; this is before state 3 is set and before any storage drain. | Yes | `0x0073DFBD` reads old `+0x6D1`; `0x0073DFD0` resets dump accumulator `+0xF8`; `0x0073DFDA MOV [unit+0x6D1],1`; `0x0073E093 MOV [unit+0xBC],3` | High |
| The swap is not performed by radio `0x16`, radio `0x18`, DOCK_LINK radio `2`, or the first credit drain. | Yes | `BuildingClass::Receive_Radio @ 0x0043C2D0` sends `0x18`/`0x16`; `UnitClass::Receive_Radio @ 0x00737430` case `0x16` only drives facing/possible `0x15`; `unit+0x6D1` write is only in `Mission_Deploy_Building` dump init for this path | High |
| On the same dump-init pass as `+0x6D1=1`, HARV optionally triggers refinery slot 7 (`PreProductionAnim`) and then enters state 3. | Yes | `0x0073E005` tests `Type+0xE0E`; `0x0073E08A PUSH 7`; `0x0073E08E CALL BuildingClass__SetAnimSlotImage`; `0x0073E093 state=3` | High |
| The first storage drain is later, gated by `RulesClass+0x1528 HarvesterDumpRate * 900.0 <= unit+0xF8`, so HORV begins before the first drain event. | Yes | state-3 drain gate at `0x0073E355..0x0073E374`; default INI `HarvesterDumpRate=0.016`, threshold 14.4 frames per slot | High |
| Normal non-weeder completion returns to HARV when state 4 writes `unit+0x6D1=0`. The render gate fails on subsequent draws. | Yes | state 4 non-weeder path: `0x0073E1F6 MOV byte ptr [ESI+0x6D1],0`; then mission/queue/radio cleanup | High |
| Weeder/legacy state-4 path also clears `+0x6D1`, but stock HARV does not use the weeder branch (`Type+0xE0F=0`). | No for HARV; code exists | `0x0073E0B8` reads type; `0x0073E0C4` tests `+0xE0F`; `0x0073E0D2 MOV [unit+0x6D1],0` in weeder branch | High |
| Mission override mid-unload does not clear `+0x6D1` immediately in state 3; it sets state 4 and clears refinery slot 10, then state 4 clears `+0x6D1` on a later pass. | Yes | early-exit check `0x0073E539..0x0073E551`; optional slot 8 at `0x0073E58F`; `state=4` at `0x0073E594`; clear slot 10 at `0x0073E5A8..0x0073E5AC`; `+0x6D1` clear in state 4 at `0x0073E1F6` | High |
| Radio `0x17` redirect clears `+0x6D1` immediately if the unit is a Harvester/Weeder and the flag is set, then scatters and sets mission Harvest. | Conditional | `UnitClass::Receive_Radio`: `0x00737A9E` reads `+0xE0E`; `0x00737AA8` reads `+0xE0F`; `0x00737AB2` reads `+0x6D1`; `0x00737AC9 MOV [unit+0x6D1],0`; `0x00737AD0` scatter; `0x00737ADE SetMission(10,0)` | High |
| Missing-refinery state-3 abort does not clear `+0x6D1` in static code; it optionally sends radio `3`, queues Harvest, and returns. | Yes | `0x0073E306` lookup; null branch `0x0073E311..0x0073E350`; no write to `+0x6D1` in the branch | High static / Medium runtime-frame |
| `BuildingClass::ReleaseDockedHarvester` / `UndockUnit` is not the normal stock zero-link HARV completion path and does not by itself prove unload visual cleanup. | Conditional | `BuildingClass__ReleaseDockedHarvester @ 0x004595C0` clears building anim slots and dock link path; normal zero-link cleanup is state 4 in `Mission_Deploy_Building` per newer dock reports | High |

## Timing Answer

`HARV` becomes visually `HORV` on the first render after `Mission_Deploy_Building` dump-init sets `unit+0x6D1=1` at `0x0073DFDA`.

That means:

- Not at `UnloadingClass` INI parse time.
- Not at radio `2` dock link.
- Not at radio `0x18` accepted/entered.
- Not at radio `0x16` facing pivot itself.
- Not at the first ore/credit drain.
- Yes, it is tied to `Mission_Deploy_Building`: the gate flag is set by dump-init immediately before state `3`, and cleared by state `4`.

The visible swap is still render-time. The mission code only toggles `unit+0x6D1`; `DrawExtras` temporarily writes `unit+0x6C4=HORV`, calls the SHP/VXL body draw path, and restores `unit+0x6C4=HARV` before leaving draw.

## INI Keys

| Key | Section | Retail value | Binary field / use | Active in YR |
|---|---|---:|---|---:|
| `UnloadingClass` | `[HARV]` | `HORV` | parsed to `TechnoTypeClass+0x6B8`; render-time alternate `UnitTypeClass*` | Yes |
| `Harvester` | `[HARV]` / `[HORV]` | `yes` | `TechnoTypeClass+0xE0E`; gates dock-unload acceptance and `DrawExtras` swap | Yes |
| `Storage` | `[HARV]` | `40` | makes HARV a 40-bale cargo miner; drain timing not controlled by this report | Yes |
| `Dock` | `[HARV]` / `[HORV]` | `NAREFN,GAREFN` | refinery selection/entry | Yes |
| `Voxel` | `artmd.ini [HARV]/[HORV]` | `yes` | draw dispatch uses VXL path when selected type `+0x236 != 0` and VXL data `+0xB0 != 0` | Yes |

## Current Rust Implementation Status

Rust has an explicit `display_type_override` instead of gamemd's temporary `unit+0x6C4` swap:

- `src/rules/object_type.rs:364` / `:941` stores `unloading_class`.
- `src/sim/game_entity.rs:249` stores `display_type_override`.
- `src/sim/miner/miner_dock_sequence.rs:345` resolves `UnloadingClass`.
- `src/sim/miner/miner_dock_sequence.rs:670` `phase_linked` sets `display_type_override` at `:698`.
- `src/sim/miner/miner_dock_sequence.rs:717` `phase_pivoting` delays `unload_timer` seed until pivot completion.
- `src/sim/miner/miner_dock_sequence.rs:763` `phase_unloading` drains slots.
- `src/sim/miner/miner_dock_sequence.rs:870` `phase_departing` clears `display_type_override` at `:892`.
- `src/sim/miner/miner_dock_sequence.rs:471` `abort_invalid_refinery` clears the override at `:479`.
- `src/app_instances/units.rs:91` honors `display_type_override` during rendering.

Delta: Rust currently sets the override in `phase_linked`, before its explicit pivot phase completes. Static binary evidence says the render gate turns on at `Mission_Deploy_Building` dump-init (`+0x6D1=1`), not at the initial radio link. Whether that maps closer to Rust `phase_linked` or `phase_pivoting` depends on Rust's local phase model, but the acceptance test should assert the player-visible rule: no HORV before the stock dump-init equivalent, and HORV before the first slot drain.

Rust also intentionally clears `display_type_override` immediately on invalid-refinery abort. Static binary evidence for the missing-refinery state-3 branch does not clear `+0x6D1`; this remains a runtime-frame uncertainty, not a confirmed stock visual cleanup.

## Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoTypeClass::ReadINI` `UnloadingClass` parse | verified | `0x007146E8..0x0071471A` | none |
| `rulesmd.ini` `[HARV]/[HORV]` and `artmd.ini` voxel keys | verified | local INI scan | none |
| `UnitClass::DrawExtras` render swap gate | verified | `0x0073D29C..0x0073D3A2` | none |
| Draw dispatch VXL/SHP choice with swapped type | verified | `0x0073D317..0x0073D38F` | exact pixel capture out of scope |
| Dump-init swap-on flag write | verified | `0x0073DFBD..0x0073E093` | none |
| First drain after swap-on | verified | `0x0073E355..0x0073E374` | none |
| Normal state-4 swap-off | verified | `0x0073E1F6` | none |
| Mission override state-3 early exit | verified | `0x0073E539..0x0073E5AC` | exact tick gap until state 4 clear, if any, needs runtime capture |
| Radio `0x17` immediate clear | verified | `0x00737A98..0x00737AF6` | direct sender inventory out of scope |
| Missing-refinery state-3 abort | verified | `0x0073E306..0x0073E350` | exact visible stale-frame duration needs runtime debugger |
| `ReleaseDockedHarvester` as normal stock exit | conflict-resolved | newer zero-link reports + `0x004595C0` | none for this slice |
| Current Rust comparison | touched-not-exhausted | `rg` scan of miner/render files | run focused tests in implementation session |

## Open Questions - Final State

- `[RESOLVED] OQ-1 - Where is `UnloadingClass` read? -> `TechnoTypeClass::ReadINI` reads the string and resolves it through `UnitTypeClass__FindOrAllocate`.` (evidence: `0x007146E8..0x0071471A`)
- `[RESOLVED] OQ-2 - Is `HORV` a UnitType or an anim? -> UnitType; the parser calls `UnitTypeClass__FindOrAllocate`, and `artmd.ini [HORV]` is `Voxel=yes`.` (evidence: `0x007146FD`, `artmd.ini [HORV]`)
- `[RESOLVED] OQ-3 - Does stock permanently change the unit type? -> No; `DrawExtras` saves `unit+0x6C4`, temporarily swaps it, then restores it after draw dispatch.` (evidence: `0x0073D29C`, `0x0073D2C4`, `0x0073D3A2`)
- `[RESOLVED] OQ-4 - What gate makes HARV render as HORV? -> `Harvester=yes`, `unit+0x6D1 != 0`, and `Type+0x6B8 != 0`.` (evidence: `0x0073D2A6..0x0073D2C4`)
- `[RESOLVED] OQ-5 - When is the gate set? -> Dump-init in `Mission_Deploy_Building`, before state 3 and before first drain.` (evidence: `0x0073DFD0..0x0073E093`)
- `[RESOLVED] OQ-6 - Is the gate set by radio `0x16`? -> No; radio `0x16` drives facing/possible `0x15`, but the `+0x6D1` write is in `Mission_Deploy_Building`.` (evidence: `0x00737430`, `0x0073DFDA`)
- `[RESOLVED] OQ-7 - When does normal completion clear the gate? -> State 4 non-weeder path clears `+0x6D1=0`.` (evidence: `0x0073E1F6`)
- `[RESOLVED] OQ-8 - Does first drain trigger HORV? -> No; HORV gate is already set before the drain threshold block.` (evidence: `0x0073DFDA`, `0x0073E355..0x0073E374`)
- `[RESOLVED] OQ-9 - Does mission override clear immediately? -> No in state 3; it sets state 4 and clears building slot 10, then state 4 clears `+0x6D1`.` (evidence: `0x0073E539..0x0073E5AC`, `0x0073E1F6`)
- `[RESOLVED] OQ-10 - Is there an immediate clear interrupt? -> Radio `0x17` clears `+0x6D1` immediately when Harvester/Weeder and flag set.` (evidence: `0x00737A98..0x00737AF6`)
- `[RESOLVED] OQ-11 - Does missing-refinery state-3 abort clear HORV? -> No static write clears `+0x6D1`; it queues Harvest and returns.` (evidence: `0x0073E306..0x0073E350`)
- `[RESOLVED] OQ-12 - Does the current Rust surface have an equivalent? -> Yes, `display_type_override`, but timing differs in phase naming and invalid-refinery cleanup.` (evidence: `src/sim/miner/miner_dock_sequence.rs`, `src/app_instances/units.rs`)
- `[DEFERRED] OQ-13 - Exact number of rendered stale-HORV frames after missing-refinery state-3 abort.` (category: `needs-runtime-debugger`; reason: static code proves no `+0x6D1` clear in that branch, but the render loop/missional commutation frame needs live capture; next-step-if-pursued: destroy/sell refinery during state-3 unload and capture consecutive frames)

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| HARV renders as HORV only when the stock unload-active gate is set; the swap is render-time and temporary. | `0x0073D29C..0x0073D3A2` | Rust uses persistent `display_type_override` field during dock phase | `src/sim/miner/miner_dock_sequence.rs`, `src/app_instances/units.rs` | Keep normal sim type as HARV; render uses HORV only for active unload visual window. | `war_miner_unloading_class_is_render_override_not_type_change` | Do not mutate the entity's real type/ref into HORV for gameplay/rules lookup. |
| Swap-on happens at dump-init before first drain, not at first credit event. | `0x0073DFDA`, `0x0073E355..0x0073E374` | Rust sets override in `phase_linked`; may be earlier than stock depending on local phase mapping | `phase_linked`, `phase_pivoting`, `phase_unloading` | Ensure no credits/drain are required before HORV becomes visible; ensure HORV is not visible before the stock dump-init equivalent. | `war_miner_horv_visible_before_first_slot_drain_after_dump_init` | Do not tie HORV to `BaleDepositEvent` or first slot drain. |
| Normal completion clears the visual when state 4 clears `+0x6D1`. | `0x0073E1F6` | Rust clears in `phase_departing` | `phase_departing` | Clear override when the stock state-4 handoff/depart path runs. | `war_miner_unloading_class_clears_on_state4_depart_handoff` | Do not leave HORV after normal unload completion. |
| Mission override mid-unload sets state 4 first; visual clears on subsequent state-4 clear, while building slot 10 is cleared in state 3. | `0x0073E539..0x0073E5AC`, `0x0073E1F6` | unchecked for non-harvest queued mission interruption | dock interrupt / command override handling | If a player command interrupts unload, preserve the ordering: stop drain, clear building special anim, then clear miner visual through the depart/state-4 cleanup. | `war_miner_attack_order_mid_unload_clears_horv_through_depart_cleanup` | Do not award partial pending credits just because the visual was active. |
| Radio `0x17` redirect clears `+0x6D1` immediately. | `0x00737AC9`, `0x00737ADE` | no explicit radio layer; local abort paths may stand in | dock reservation / redirect handling | Any implemented redirect-equivalent should immediately clear the display override and return to Harvest. | `war_miner_redirect_interrupt_clears_horv_immediately` | Do not defer this clear to normal unload completion if modeling radio `0x17`. |
| Missing-refinery state-3 abort does not statically clear `+0x6D1`. | `0x0073E306..0x0073E350`; `DrawExtras @ 0x0073CEC0` | Rust currently clears override in `abort_invalid_refinery` | `abort_invalid_refinery` | Treat immediate clear on missing refinery as an intentional Rust choice until runtime proves stock frame behavior. | `war_miner_missing_refinery_abort_visual_cleanup_requires_runtime_verification` | Do not claim gamemd clears HORV on this branch from static evidence; it does not. |

## Negative Facts / Do Not Do

- Do not implement HORV as a real unit type conversion for simulation, weapons, cargo, or selection logic.
- Do not wait until the first drain/credit event to show HORV.
- Do not show HORV merely because the miner has sent/received dock radio `2`, `0x18`, or `0x16`.
- Do not use `ReleaseDockedHarvester` as the normal stock zero-link visual cleanup explanation.
- Do not claim the missing-refinery state-3 branch clears `+0x6D1`; static evidence says it does not.
- Do not clear building `SpecialAnim` slot 10 and miner HORV gate as if they are the same state; they have separate code paths.

## Stale Docs / Follow-Up Wording

- Replace "Swap TO unloading model when unit enters the dock / after `BuildingClass::EnterTransport`" with: "For stock zero-link HARV unload, `DrawExtras` renders `HORV` only after `Mission_Deploy_Building` dump-init writes `unit+0x6D1=1`; this is before first storage drain and after the radio/facing entry sequence."
- Replace "Swap BACK when `BuildingClass::UndockUnit` / `ReleaseDockedHarvester` is called" with: "Normal stock zero-link HARV unload swaps back when `Mission_Deploy_Building` state 4 clears `unit+0x6D1=0`; `UndockUnit`/`ReleaseDockedHarvester` are conditional interrupt/link paths, not the common stock zero-link completion path."
- Replace "interrupted unload clears the override" with: "Interrupt cleanup is branch-specific: radio `0x17` clears immediately; queued non-Harvest mission moves state 3 to state 4 and clears on the state-4 pass; the missing-refinery state-3 abort does not statically clear `+0x6D1`."

## Remaining Uncertainty

- Exact same-render-frame visibility after the missing-refinery state-3 abort needs runtime debugger or screenshot capture. Static evidence shows no clear in the branch; it does not prove how many frames the player sees stale HORV after mission queueing.
- Exact mapping from Rust's `Linked/Pivoting/Unloading` names to stock radio/timer microstates should be validated with an end-to-end visual trace before changing current timing. The binary anchor is `+0x6D1` at `0x0073DFDA`, not the Rust phase name.

## Sources

- Ghidra read-only decompile/assembly: `0x007146E8`, `0x007480D0`, `0x0073CEC0`, `0x0073D630`, `0x00737430`, `0x0043C2D0`, `0x004DB0A0`, `0x004595C0`.
- Prior docs read: `miner/HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md`, `miner/HARVESTER_DOCK_UNLOAD.md`, `miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `UNLOADINGCLASS_RENDER_ORIENTATION_GHIDRA_REPORT.md`, `miner/MISSING_DESTROYED_REFINERY_MID_UNLOAD_ORDERING_GHIDRA_REPORT.md`, `miner/HARVESTER_DOCK_UNLOAD_SEQUENCE.md`, `UNIT_DRAW_EXTRAS_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scan only: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/game_entity.rs`, `src/rules/object_type.rs`, `src/app_instances/units.rs`, `src/sim/miner/miner_tests.rs`.
