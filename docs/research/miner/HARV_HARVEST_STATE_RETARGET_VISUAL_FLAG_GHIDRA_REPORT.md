# HARV Harvest-State Retarget Visual Flag - Ghidra Research Report

**Address(es):** `0x0073E5E0` (`UnitClass::Mission_Harvest`), `0x0073D450` (`UnitClass::Harvest_Ore_Tick`), `0x004DCFE0` (`FootClass::Search_For_Tiberium_And_Move`), `0x0073CEC0` (`UnitClass::DrawExtras`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard YR War Miner (`[HARV]`) already in `Mission_Harvest` substate 1, current ore cell depletes before cargo is full, `TiberiumShortScan` finds reachable nearby ore, and the stock mission/visual flag state after that retarget.
**Non-Scope:** Full state-0 scan parity, refinery return/dock behavior, chrono teleport return, slave miner/weed behavior, combat acquisition while mining, and runtime pixel capture.
**Confidence:** High for mission substate, field writes, short-scan radius, and OREGATH draw gates; Medium for non-OREGATH visual consumers because this slice verified the known `DrawExtras` consumer only.
**Active in YR:** Yes. `[HARV]` in `rulesmd.ini` has `Harvester=yes`, `Storage=40`, `Primary=20mmRapid`, and no `Teleporter=yes`; the investigated branch is gated by live `UnitTypeClass+0xE0E` Harvester, not by TS-only fog/weed behavior.

## Working Notes

**Target question:** When a War Miner drains its current ore cell before it is full and nearby ore is found by the state-1 short scan, does stock YR leave harvest mission/substate 1 active and keep `UnitClass+0x6D2` set, or does it transition to a separate move-to-ore presentation state?

**Non-goals:** Do not re-investigate state 2 return, refinery docking, Chrono Miner teleport, slave miner logic, weapon behavior, or Rust implementation patches.

**Evidence needed to mark COMPLETE:** `UnitClass::Mission_Harvest` state-1 false-extraction branch; exact read of `RulesClass+0x1778`; exact retarget success/failure conditions; writes to `UnitClass+0xBC` and `UnitClass+0x6D2`; a verified consumer for `+0x6D2`; and current Rust surfaces that would diverge.

**Stop conditions:** Stop after the state-1 War Miner retarget branch, scan wrapper, extraction-failure source, and OREGATH consumer are drained with no unresolved open questions for this slice. Defer unrelated helper internals once their return contracts are enough for this target.

## 1. Overview

Stock YR does not leave the harvest mission or switch to a separate move-to-ore mission/substate when a War Miner short-retargets after depleting its current ore cell. In `Mission_Harvest` state 1, a failed extraction clears `UnitClass+0x6D2` briefly inside the same tick, runs a `TiberiumShortScan` continuation, and if a scan hit or existing destination is present, writes harvest substate `1` and sets `UnitClass+0x6D2 = 1` before returning.

The visual implication is subtle: `+0x6D2` remains the stock "harvesting presentation active" flag across the retarget, but `UnitClass::DrawExtras` also requires the locomotor moving predicate to be false before drawing `OREGATH.SHP`. Therefore stock does not prove OREGATH should be visible while the unit is actually driving to the next ore cell; it proves the active-harvest flag is preserved so presentation resumes without a mission/substate break once the movement gate clears.

## 2. Class Layout / Key Offsets

| Owner | Byte offset | int* index | Type | Purpose in this slice | Evidence |
|---|---:|---:|---|---|---|
| `UnitClass` | `0xBC` | `[0x2F]` | int | `Mission_Harvest` substate. State 1 is the harvest/continuation state. | `0x0073E5E0`, switch on `param_1[0x2f]`; state-1 success writes at `0x0073EB0F` |
| `UnitClass` | `0xF8` | `[0x3E]` | int | Step counter; state 1 waits until counter is at least `9` before extraction. | `0x0073E96F`, `CMP [EBP+0xF8],0x9` |
| `UnitClass` | `0x100` | `[0x40]` | int | Step timer start frame. | `0x0073E946` path and `Harvest_Ore_Tick` reset |
| `UnitClass` | `0x108` | `[0x42]` | int | Step timer step amount. | `0x0073E966` setup from `HarvesterLoadRate` |
| `UnitClass` | `0x10C` | `[0x43]` | int | Step timer rate; `0` means state 1 first-entry setup. | `0x0073E934` check |
| `UnitClass` | `0x5A4` | `[0x169]` | pointer/cell target | Movement destination. Used as a second success condition after retarget attempt. | `0x0073EAE4..0x0073EAEC` |
| `UnitClass` | `0x674` | `[0x19D]` | locomotor pointer | DrawExtras queries vfunc `+0x80`; OREGATH draw requires that call to return false. | `0x0073D0F7..0x0073D11C` |
| `UnitClass` | `0x6C4` | `[0x1B1]` | `UnitTypeClass*` | Type pointer for `Harvester` flag. | `0x0073EA8D`, `0x0073D0D1` |
| `UnitClass` | `0x6D2` | byte | byte | Active harvesting / OREGATH candidate flag. Cleared on failed extraction, set again on retarget success or destination-present continuation. | clear at `0x0073E99A`; set at `0x0073EB19`; read at `0x0073D0E9` |
| `UnitTypeClass` | `0xE0E` | n/a | bool | `Harvester=yes`; selects ore short-retarget path and gates OREGATH draw. | `0x0073EA93`; `0x0073D0DB` |
| `RulesClass` | `0x1778` | n/a | int/leptons | `TiberiumShortScan`; state-1 continuation radius, converted to cells by signed divide-by-256 idiom. | `0x0073EAA6`, `0x0073EAC6` |

## 3. Core Logic

### State-1 entry and extraction gate

In `UnitClass::Mission_Harvest @ 0x0073E5E0`, case 1 is reached when `UnitClass+0xBC == 1`.

Verified order:

1. If `UnitClass+0x10C == 0`, initialize the state-1 step timer from `RulesClass+0x1520` (`HarvesterLoadRate`): set steps to `0`, start frame to `g_CurrentFrameCounter`, step amount to load rate, and rate to load rate.
2. If `UnitClass+0xF8 < 9`, return `1` immediately and do not attempt extraction.
3. Once the counter is at least `9`, call `UnitClass::Harvest_Ore_Tick @ 0x0073D450`.

### Extraction failure source

`UnitClass::Harvest_Ore_Tick @ 0x0073D450` returns success when bales are actually removed, or when a destination is already present. It returns failure with low byte zero when no extraction happens. The failure path relevant to depleted current ore:

1. Get current cell from unit coordinates.
2. If `UnitClass+0x5A4` destination is non-zero, return success (`1` in low byte), so moving units do not harvest and do not trigger the empty-cell retarget branch.
3. If not a harvester, storage is full, or current cell `CellClass+0xEC != 5` (`LandType != Tiberium`), reset the step timer fields to zero and return failure.
4. Otherwise remove ore via `CellClass::Reduce_Tiberium`; if the removed amount is positive, add storage and reset the timer to `HarvesterLoadRate`, returning success.

For the target scenario, the current cell has already been depleted by the previous extraction, so its land type is no longer `5`; this returns failure to `Mission_Harvest` state 1.

### State-1 empty-cell short-retarget branch

After `Harvest_Ore_Tick` returns false, `Mission_Harvest` does this exact sequence:

1. Clear `UnitClass+0x6D2 = 0` at `0x0073E99A`.
2. For live harvesters (`UnitTypeClass+0xE0E != 0`), call `FootClass::Search_For_Tiberium_And_Move` with radius from `RulesClass+0x1778`, converted from leptons to cells by `(value + (value >> 31 & 0xff)) >> 8`, and zone argument `0`.
3. If the search returns true, jump to the continuation-success block.
4. If search returns false but `UnitClass+0x5A4` destination is non-zero, also jump to the continuation-success block.
5. Only if search returns false and destination is zero, call `TechnoClass::SetGhostCell(0)`, set substate `2`, and return.
6. Continuation-success block writes `UnitClass+0xBC = 1` and `UnitClass+0x6D2 = 1`, then returns `1`.

The decisive instructions are:

| Address | Instruction / behavior | Meaning |
|---|---|---|
| `0x0073EA93` | read `UnitTypeClass+0xE0E` | Standard HARV takes the live harvester branch. |
| `0x0073EAA6` | read `RulesClass+0x1778` | Radius is `TiberiumShortScan`, not long scan. |
| `0x0073EAB9` | call `0x004DCFE0` | Calls zone-aware `Search_For_Tiberium_And_Move(radius, 0)`. |
| `0x0073EAE0` | `TEST AL,AL` | Tests scan wrapper low-byte success. |
| `0x0073EAE4..0x0073EAEC` | test `UnitClass+0x5A4` | Existing destination keeps state 1 alive even if wrapper returned false. |
| `0x0073EAF8` | write `UnitClass+0xBC = 2` | Only miss + no destination returns to refinery. |
| `0x0073EB0F` | write `UnitClass+0xBC = 1` | Retarget success remains harvest substate 1. |
| `0x0073EB19` | write `UnitClass+0x6D2 = 1` | Active-harvest visual flag is restored before return. |

### Search wrapper behavior

`FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0` first checks `UnitClass+0x5A4`.

If no destination is present, it calls the unit vtable `+0x338` scanner, compares the result against the invalid cell sentinel, and:

- returns true immediately if the selected ore cell is the unit's current cell;
- otherwise converts the selected cell to a `CellClass*` and calls vtable `+0x480` to set destination.

If a destination is already present at wrapper entry, it does not scan. Its low byte return is false, but `Mission_Harvest` state 1 has the separate destination-present test described above, so the mission still stays in substate 1 and restores `+0x6D2`.

### Visual consumer: OREGATH draw gates

`UnitClass::DrawExtras @ 0x0073CEC0` is a verified consumer of `UnitClass+0x6D2`.

OREGATH draw requires all of these:

1. `UnitTypeClass+0xE0E != 0` (`Harvester=yes`) at `0x0073D0DB`.
2. `UnitClass+0x6D2 != 0` at `0x0073D0E9`.
3. Locomotor pointer at `UnitClass+0x674` is non-null; otherwise assert.
4. Locomotor vfunc `+0x80` returns false; if it returns true, branch skips OREGATH draw at `0x0073D11A`.
5. `UnitClass+0x278 == 0` (not deploying), vtable `+0x1D4` false (not cloaking), and vtable `+0x1D8` false (not being chronoshifted).
6. If all pass, call `CC_Draw_Shape @ 0x004AED70` with `OREGATH.SHP`.

Therefore the binary supports this distinction:

- `+0x6D2` remains set during the state-1 retarget continuation.
- OREGATH is still hidden while the locomotor reports moving.
- A renderer should not equate "retarget hop" with "leave harvesting presentation mode"; it also should not force OREGATH to render while moving.

## 4. INI Keys

| INI file / section | Key | Value | Effect in this slice | Binary evidence |
|---|---|---|---|---|
| `ini/rulesmd.ini [General]` | `TiberiumShortScan` | `6` | State-1 continuation radius. Stored at `RulesClass+0x1778`, then converted from leptons to cells by `>> 8`. | `0x0073EAA6`, `0x0073EAC6` |
| `ini/rulesmd.ini [General]` | `TiberiumLongScan` | `48` | Not used in this state-1 retarget branch; used by state 0. | Prior state-0 report; not the branch at `0x0073EA8D` |
| `ini/rulesmd.ini [General]` | `HarvesterLoadRate` | inherited/default `2` in prior reports | Controls state-1 extraction timer setup and reset. | `RulesClass+0x1520` read in `Mission_Harvest` and `Harvest_Ore_Tick` |
| `ini/rulesmd.ini [HARV]` | `Harvester` | `yes` | Makes the state-1 HARV branch use `Search_For_Tiberium_And_Move` and enables OREGATH candidate draw. | `UnitTypeClass+0xE0E` reads at `0x0073EA93`, `0x0073D0DB` |
| `ini/rulesmd.ini [HARV]` | `Storage` | `40` | Target scenario is partial cargo, so the full-storage branch is not taken. | Fullness check via vtable `+0x2B4`; `[HARV] Storage=40` |
| `ini/rulesmd.ini [HARV]` | `Primary` | `20mmRapid` | Confirms this is the armed War Miner chassis; combat is non-scope here. | INI only for this slice |
| `ini/rulesmd.ini [HARV]` | `UnloadingClass` | `HORV` | Not used in this branch; dock/unload non-scope. | INI only |

## 5. Integration Points

`UnitClass::Mission_Harvest` is the mission 10 vtable handler for harvesters. The relevant stock path is:

`Mission_Harvest state 1 -> step counter >= 9 -> Harvest_Ore_Tick -> false because current cell is no longer Tiberium -> clear +0x6D2 -> TiberiumShortScan continuation -> Set_Destination to nearby ore -> write state 1 and +0x6D2=1 -> return 1`.

`UnitClass::DrawExtras` is called by the unit render path and reads `+0x6D2` as part of the OREGATH draw predicate. The draw call itself uses `CC_Draw_Shape` with frame:

`(UnitClass+0x538 + g_CurrentFrameCounter) % 15 + facing_index * 15`

That frame formula is global-time based plus a per-unit offset; it is not a stateful animation counter reset when the harvest mission retargets.

## 6. Current Rust Implementation Status

Current Rust has a known top-level `MinerState::MoveToOre` separate from `MinerState::Harvest`.

Relevant Rust surfaces read in this investigation:

| Surface | Current behavior | Binary delta |
|---|---|---|
| `src/sim/miner/miner_system.rs:561-564` | Empty-cell continuation hit sets `target_ore_cell = Some(next_cell)` and `state = MinerState::MoveToOre`. | Stock writes `Mission_Harvest` substate 1 and restores `+0x6D2=1`; no separate mission/substate break. |
| `src/sim/miner/miner_system.rs:156-184` | `VoxelAnimation.playing` and `HarvestOverlay.visible` are driven directly from `miner.state == MinerState::Harvest`; leaving Harvest resets animation/overlay frame/elapsed. | Stock `+0x6D2` remains set after retarget; OREGATH draw also has a moving gate, but the active-harvest flag is not cleared across the retarget. |
| `src/sim/animation.rs:530-549` | `HarvestOverlay` animation advances only while `visible`; hide/show resets can restart component frame at 0. | Stock OREGATH frame is `(unit random offset + global frame) % 15`, not a local counter restarted by retarget. |
| `src/sim/miner/miner_tests.rs:3822-3870` | Current test expects `MinerState::MoveToOre` after the short scan. | This expectation is stale for stock mission/substate parity; a future fix should test the active-harvest presentation flag separately from physical movement. |

No Rust files were edited in this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Mission_Harvest` state 1 retarget branch | verified | `0x0073E934..0x0073EB2B` decompile and assembly context | none for this slice |
| `UnitClass::Harvest_Ore_Tick` depleted-cell false return | verified | `0x0073D450` decompile | none for this slice |
| `FootClass::Search_For_Tiberium_And_Move` return/destination contract | verified | `0x004DCFE0` decompile | exact vtable `+0x480` side effects beyond destination set are out of scope |
| `FootClass::Scan_For_Tiberium` selection details | touched-not-exhausted | `0x004DD0A0`; prior state-0 report | Full scan parity belongs to existing state-0/scan reports |
| `FootClass::Is_Cell_Harvestable` predicate | touched-not-exhausted | `0x004DCE80`; prior state-0 report | Occupancy/passability details are covered by slot 4, not this report |
| `CellClass::Reduce_Tiberium` depletion behavior | touched-not-exhausted | `0x00480A80` decompile | Growth/spread side effects non-scope |
| `CellClass::Get_Tiberium_Value` value formula | touched-not-exhausted | `0x00485020` decompile | Selection tie-breaks non-scope here |
| `UnitClass::DrawExtras` OREGATH consumer | verified | `0x0073CEC0`, assembly context at `0x0073D0DB..0x0073D11C` | none for known OREGATH consumer |
| Non-OREGATH consumers of `UnitClass+0x6D2` | deferred | bounded read did not perform whole-binary field-xref sweep | Follow-up data-xref sweep if another presentation consumer is suspected |
| Current Rust miner retarget state | verified | `src/sim/miner/miner_system.rs:561-564` | implementation work is separate |
| Current Rust overlay/voxel gating | verified | `src/sim/miner/miner_system.rs:156-184`; `src/sim/animation.rs:530-549` | implementation work is separate |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is the state-1 depleted-cell continuation branch live for standard YR HARV? -> Yes. HARV has Harvester=yes, and the branch checks UnitTypeClass+0xE0E before calling the ore short-retarget path.` (evidence: `ini/rulesmd.ini [HARV]`; `0x0073EA93`)
- `[RESOLVED] OQ-02 - Does state 1 use TiberiumShortScan or TiberiumLongScan for continuation? -> TiberiumShortScan from RulesClass+0x1778, converted to cells with the signed divide-by-256 idiom.` (evidence: `0x0073EAA6..0x0073EAB8`)
- `[RESOLVED] OQ-03 - Does a successful nearby-ore retarget leave Mission_Harvest substate 1? -> Yes, the success block writes UnitClass+0xBC = 1.` (evidence: `0x0073EB0F`)
- `[RESOLVED] OQ-04 - Is UnitClass+0x6D2 kept set after retarget success? -> Yes. It is cleared after extraction failure, then restored to 1 in the same state-1 continuation success block.` (evidence: clear at `0x0073E99A`; set at `0x0073EB19`)
- `[RESOLVED] OQ-05 - What happens if the scan wrapper returns false but a destination exists? -> The mission still takes the success block and restores state 1/+0x6D2.` (evidence: `0x0073EAE0..0x0073EAEC`; `0x0073EB0F..0x0073EB19`)
- `[RESOLVED] OQ-06 - What happens if scan misses and no destination exists? -> It clears the ghost cell, writes substate 2, and returns 1.` (evidence: `0x0073EAEE..0x0073EB0D`)
- `[RESOLVED] OQ-07 - Does Harvest_Ore_Tick itself retarget? -> No. It extracts or returns false; retargeting is in Mission_Harvest state 1 after a false return.` (evidence: `0x0073D450`; `0x0073EA8D..0x0073EB19`)
- `[RESOLVED] OQ-08 - Does OREGATH draw directly from +0x6D2? -> Yes, but only after Harvester=yes and before other gates including locomotor-not-moving.` (evidence: `0x0073D0DB..0x0073D11C`)
- `[RESOLVED] OQ-09 - Is OREGATH visible while the retargeted War Miner is physically moving? -> Not if the locomotor vfunc +0x80 reports moving; DrawExtras skips the draw when that call returns true.` (evidence: `0x0073D10B..0x0073D11C`)
- `[RESOLVED] OQ-10 - Does OREGATH animation reset on retarget? -> No reset was found in this branch; DrawExtras computes frame from UnitClass+0x538 plus g_CurrentFrameCounter modulo 15.` (evidence: `0x0073D24E..0x0073D283` from prior OREGATH report; decompile at `0x0073CEC0`)
- `[RESOLVED] OQ-11 - Does this branch require Teleporter=yes? -> No. The state-1 HARV branch is selected by Harvester=yes; HARV has no Teleporter=yes in rulesmd.` (evidence: `0x0073EA93`; `ini/rulesmd.ini [HARV]`)
- `[RESOLVED] OQ-12 - Does the state-1 retarget branch save/archive the found nearby cell? -> No archive write was found on the non-full retarget success path; archive writes belong to the full branch or miss cleanup.` (evidence: `0x0073E9D0..0x0073EA7B` full branch; `0x0073EAEE..0x0073EAF2` miss branch; success block `0x0073EB0F..0x0073EB19`)
- `[RESOLVED] OQ-13 - What exact current Rust line causes the state/substate mismatch? -> The continuation hit assigns MoveToOre at miner_system.rs:561-564.` (evidence: `src/sim/miner/miner_system.rs:561-564`)
- `[RESOLVED] OQ-14 - What exact current Rust line causes presentation flag loss? -> Overlay and voxel animation are driven from MinerState::Harvest only at miner_system.rs:156-184.` (evidence: `src/sim/miner/miner_system.rs:156-184`)
- `[DEFERRED] OQ-15 - Are there other live YR consumers of UnitClass+0x6D2 besides UnitClass::DrawExtras?` (category: bounded-cost-too-high; reason: this slot verified the known OREGATH consumer but did not perform a whole-binary offset-xref inventory; next-step-if-pursued: run a dedicated data-xref sweep for byte offset `0x6D2`)
- `[DEFERRED] OQ-16 - What exact locomotor vfunc +0x80 method name and all return conditions apply for DriveLocomotion?` (category: requires-different-system-context; reason: the visual conclusion only needs DrawExtras' branch on the return value; next-step-if-pursued: investigate DriveLocomotion moving predicate and compare to Rust movement-active state)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| State-1 short-retarget success leaves harvest substate 1 and restores `+0x6D2=1`. | `0x0073EA8D..0x0073EB19` | mismatch: Rust sets `MinerState::MoveToOre` | `src/sim/miner/miner_system.rs` retarget branch | Preserve an "active harvest continuation" presentation/state flag across nearby retarget movement, even if Rust keeps a movement state internally. | `war_miner_short_retarget_keeps_harvest_visual_flag`: HARV at `(20,20)` drains partial cargo, short-retargets `(21,20)`, and the presentation flag remains active while target is set. | Do not key all presentation solely on top-level `MinerState::Harvest` if movement-to-next-ore is stock substate 1. |
| OREGATH is gated by `Harvester=yes`, `+0x6D2`, not-moving, not deploying, not cloaking, and not chronoshifting. | `UnitClass::DrawExtras @ 0x0073CEC0`, especially `0x0073D0DB..0x0073D11C` | partial mismatch: Rust hides on MoveToOre, but lacks an explicit stock-style not-moving plus active-harvest split in the miner presentation gate | `src/sim/miner/miner_system.rs`; render extraction of `HarvestOverlay.visible` | Separate "stock active-harvest flag" from "currently draw OREGATH"; draw only when active flag is true and movement/render gates allow it. | `war_miner_oregath_hidden_while_retarget_moving_but_not_reset`: after retarget while movement active, overlay not visible but active-harvest flag remains true; when movement completes, overlay resumes without state break. | Do not force OREGATH to render during the drive hop just because `+0x6D2` is true. |
| OREGATH frame is global-frame based with per-unit desync, not a local counter reset by retarget. | `0x0073CEC0` frame expression; prior `OREGATH_RENDERING_GHIDRA_REPORT.md` | likely mismatch: Rust `HarvestOverlay` has local `frame/elapsed_ms` and miner_system resets them when hidden | `src/sim/animation.rs`; `src/sim/components.rs`; `src/sim/miner/miner_system.rs` | Avoid restarting the visible OREGATH frame at 0 on state-1 nearby retarget; compute or preserve frame continuity consistent with global tick plus unit offset. | `war_miner_oregath_frame_does_not_restart_after_short_retarget`: record frame before retarget hide, advance ticks during movement, complete movement, visible frame equals global-tick/desync formula rather than 0. | Do not model stock OREGATH as an INI-rate local animation that restarts on every hide/show. |
| State-1 retarget miss and no destination is the branch that exits to return substate 2. | `0x0073EAE0..0x0073EB0D` | Rust already returns on miss, but no claim made about all refinery fallback behavior here | `src/sim/miner/miner_system.rs` miss branch; separate slot 1/2 docs | Keep the visual flag clear only for genuine miss/return paths, not for found-nearby retarget. | `war_miner_short_scan_miss_clears_harvest_visual_flag_and_returns`: depleted cell, partial cargo, no ore in short radius, state moves toward refinery and active-harvest flag false. | Do not preserve active-harvest flag across return-to-refinery or dock/unload paths. |

### Stale Docs / Follow-up Docs

- `miner/traces/MINER_FSM_ORE_DEPLETION_RETARGET_ARCHIVE_TRACE.md` Stage 8 should replace "exact render consumer from `+0x6D2` to pixels was not traced" with: "`UnitClass::DrawExtras @ 0x0073CEC0` consumes `UnitClass+0x6D2` for OREGATH, but also requires the locomotor moving predicate to be false; stock keeps the active-harvest flag set across short-retarget movement without drawing OREGATH while the locomotor reports moving."
- `src/sim/miner/miner_tests.rs` expectation text around `harvester_continues_to_short_scan_when_partial_then_empty` is stale as a parity claim if it treats `MoveToOre` as equivalent to stock state-1 behavior. Replacement wording for a future test update: "Rust may use an internal movement state for pathing, but stock mission state remains harvest substate 1 and the active-harvest presentation flag remains true after a short-retarget hit."
- No shared claims file was updated because no canonical shared claims file was found by filename search in `ra2-rust-game-docs` or in-repo `docs`.

## Negative Facts / Do Not Do

- Do not switch the stock mental model to "HARV leaves Mission_Harvest state 1 and enters MoveToOre" on a nearby continuation hit. The binary writes substate 1 again.
- Do not clear the active-harvest presentation flag for the entire retarget movement just because the unit has a movement destination.
- Do not draw OREGATH during actual movement unless the locomotor moving predicate is false; `+0x6D2` is necessary but not sufficient.
- Do not reset OREGATH animation to frame 0 on short-retarget hide/show; stock frame derives from global frame plus per-unit offset.
- Do not save a ghost/archive cell on the non-full short-retarget success path; this report found archive handling in the full branch and miss cleanup, not in the success continuation block.

## Remaining Uncertainty

- A whole-binary field-xref sweep for `UnitClass+0x6D2` was not performed. The verified live consumer for this report is OREGATH in `UnitClass::DrawExtras`.
- The exact DriveLocomotion vfunc `+0x80` implementation was not decompiled in this slot. The report only claims that DrawExtras suppresses OREGATH when that vfunc returns true.
- Runtime pixel timing for the one frame at movement completion was not captured. The binary evidence is static and sufficient for state/flag handoff, but a future trace could validate the rendered frame in-game.

## Sources

- Ghidra decompiled/read this session:
  - `UnitClass::Mission_Harvest @ 0x0073E5E0`
  - `UnitClass::Harvest_Ore_Tick @ 0x0073D450`
  - `FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0`
  - `FootClass::Scan_For_Tiberium @ 0x004DD0A0`
  - `FootClass::Is_Cell_Harvestable @ 0x004DCE80`
  - `CellClass::Get_Tiberium_Value @ 0x00485020`
  - `CellClass::Reduce_Tiberium @ 0x00480A80`
  - `UnitClass::DrawExtras @ 0x0073CEC0`
  - `CC_Draw_Shape @ 0x004AED70`
- Assembly contexts checked:
  - `0x0073D0C3..0x0073D11C` (`DrawExtras` Harvester/+6D2/movement gates)
  - `0x0073EA8D..0x0073EB19` (state-1 short-retarget branch)
- Prior reports read:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/MINER_FSM_ORE_DEPLETION_RETARGET_ARCHIVE_TRACE.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/OREGATH_RENDERING_GHIDRA_REPORT.md`
- INI checked:
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` `[General]` and `[HARV]`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini` base `[HARV]` fallback
- Rust scanned read-only:
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_system.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/mod.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_tests.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/animation.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs`
