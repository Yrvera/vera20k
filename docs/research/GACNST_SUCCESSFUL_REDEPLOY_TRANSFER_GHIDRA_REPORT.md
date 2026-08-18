# GACNST Successful Redeploy Transfer - Ghidra Research Report

**Address(es):** `0x00449C30` (`BuildingClass__Sell`, state 2 successful `UndeploysInto` branch), `0x005F5C60`, `0x00465D70`, `0x004060F0`, `0x00406060`, `0x006AF580`, `0x007C5F00`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** successful stock YR `GACNST -> AMCV` redeploy transfer only: health calculation, selected/control-state transfer, destination/path order, sound-event fields, radio/target-linked unit rebinding, and source-building cleanup.
**Non-Scope:** failed AMCV unlimbo refund except contrast, generic sell survivor/refund flow, AMCV forward deploy, custom non-GACNST `UndeploysInto` units, and full `UnitClass::Unlimbo` internals.
**Confidence:** High for direct branch ordering and copied fields; Medium for semantic names of `Techno+0x214/+0x150` because this slice verifies copies but does not audit all readers.
**Active in YR:** Yes. Stock `rulesmd.ini` has `[GACNST] ConstructionYard=yes`, `UndeploysInto=AMCV`, `Strength=1000`, `Cost=3000`; `[AMCV] Strength=1000`, `DeploysInto=GACNST`; `[MultiplayerDialogSettings] MCVRedeploys=yes`.

## 1. Overview

When a selected/player-controlled stock Allied Construction Yard finishes its reverse buildup and the AMCV `Unlimbo` succeeds, the binary removes the building from the map, places one AMCV at the computed redeploy coordinate, transfers selected state, selected sound-event state, several techno fields, optional powerup-manager ownership, and retargets any technos that were pointing at the old building.

The success path keeps the health-ratio double and refund integer as separate saved values. The double drives only the new AMCV health calculation; the refund integer is only used by allocation/unlimbo failure branches.

## 2. Class Layout / Key Offsets

| Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `Building+0xBC` | sell/undeploy mission state; state `2` completes transfer | `0x00449C47..0x00449C68` | Yes |
| `Building+0x6DD` | reverse animation complete gate | `0x00449C99..0x00449CA1` | Yes |
| `Object+0x83` | selected byte captured at function entry | `0x00449C4D`, stored to `[ESP+0x12]` | Yes |
| `TechnoType+0x408` | `UndeploysInto` UnitType pointer | `0x00449Dxx`, `0x00449E3B` | Yes for GACNST |
| `TechnoType+0x16B9` | `ConstructionYard=yes` redeploy gate | `0x00449BC0`, `0x00449Dxx` | Yes for GACNST |
| `Building+0x218` | source techno pointer field copied into local destination slot, later given to new AMCV `Set_Destination` | `0x00449E84..0x00449E8A`, `0x0044A091..0x0044A0AE` | Conditional |
| `Object+0x6C/+0x70` | current health and display/secondary health mirror | `0x0044A02C..0x0044A039` | Yes |
| `Techno+0x2B4` | linked/target pointer scanned for references to source building | `0x00449F60..0x00449F7A` | Conditional |
| `Techno+0x2D8` | powerup manager pointer | `0x0044A03C..0x0044A047`, `0x006AF580` | Conditional |
| `Techno+0x214` | copied source techno field, semantic name not proven here | `0x0044A04C..0x0044A052` | Yes when nonzero |
| `Techno+0x150` | copied source techno field, semantic name not proven here | `0x0044A058..0x0044A05E` | Yes when nonzero |
| `Techno+0x4DC..0x4EF` | 20-byte sound/event subobject copied to AMCV | `0x0044A0D4..0x0044A0E7` | Yes |
| `Techno+0x4F0/+0x4F4` | two additional sound/event handle fields copied to AMCV then cleared on source | `0x0044A0E9..0x0044A115` | Yes |
| vtable `+0xD4` | source building removed from map/cell before AMCV `Unlimbo` | `0x00449FE2..0x00449FE7` | Yes |
| Unit vtable `+0xD8` | new AMCV `Unlimbo(coord, facing)` | `0x00449FF8..0x0044A00A` | Yes |
| Unit vtable `+0x480` | `TechnoClass__Set_Destination(new, source+0x218, 1)` | `0x0044A091..0x0044A0A0`, `0x00741970` | Conditional |
| vtable `+0x14C` | select new AMCV if source was selected | `0x0044A11B..0x0044A141` | Conditional |
| vtable `+0x3C8` | retarget linked technos from source building to new AMCV | `0x0044A146..0x0044A15A` | Conditional |
| vtable `+0xDC`, `+0xF8` | source building limbo/remove then uninit/delete cleanup | `0x0044A1B5..0x0044A1D2` | Yes |

## 3. Core Logic

Entry requires the same live GACNST gate already verified by the failed-refund report: `UndeploysInto` exists, and for `ConstructionYard=yes`, `g_GameMode != 0`, source `+0x218 != 0`, owner is player-control, `MCVRedeploys` is on, and source `+0x2C0 == 0`.

Successful transfer order:

1. State 2 waits for `Building+0x6DD != 0`, dirties owner byte `House+0x1FC`, and clears source target/archive state through vtable `+0x3C8(0)`.
2. `g_MapEditorMode` is incremented, `operator_new(0x8E8)` allocates a unit, `UnitClass__Constructor(UndeploysInto, Owner)` constructs the AMCV, then `g_MapEditorMode` is decremented.
3. `ObjectClass__GetHealthRatio(source)` stores a double at `[ESP+0x24]`; source vtable `+0x2BC` stores refund integer at `[ESP+0x30]`. These are distinct slots.
4. For stock GACNST `Foundation=4x4`, spawn coordinate uses the large-foundation branch: source location plus `DAT_0089F6F0/F4`, signed `+0xFF` correction before `>> 8`, then `*0x100 + 0x80` centering. Z is copied from source.
5. If source `LightSource` is non-null, `FUN_00554A80(0)` runs before linked-target scan.
6. The function scans `g_TechnoClass_Array` before source removal. It collects technos whose `+0x2B4` points at the source building, whose pointed object's RTTI is `6`, whose byte `+0x90` is nonzero, and which are neither the source nor the new AMCV.
7. Source building vtable `+0xD4` runs before AMCV placement. Then facing is `Deploy_facing_calculator(source.Type)`, which returns `source.Type+0xEDC`.
8. New AMCV `Unlimbo(&coord, facing)` is called exactly once. Only if it returns nonzero does the success-transfer block run.
9. Health is computed from the saved source health ratio and the new AMCV type strength: `ftol((double)new_unit_type.Strength * saved_source_health_ratio)`, then clamped to minimum `1`, then written to AMCV `+0x6C` and `+0x70`.
10. If source has `+0x2D8`, `PowerUp_Cleanup(manager, new_amcv)` rebinds that manager to the AMCV and updates contained powerups' owner/backpointer field.
11. Source `+0x214` and `+0x150` are copied to the AMCV.
12. TS/artillery-style facing refresh is conditional on source type `+0x16CA`; stock GACNST does not use this branch unless mod data sets that flag.
13. If the saved source `+0x218` pointer is non-null, the AMCV receives `Set_Destination(source+0x218, 1)` through vtable `+0x480`, then mission `2` via vtable `+0x1E8(2,0)`.
14. If source `+0x34` is non-null, `FUN_005F5B50(source+0x34, new_amcv)` rebinds that reference and then decrements the same referenced object's `+0x2C` and clears source `+0x34`. This exact double-step needs a separate owner/reference audit before naming.
15. Five dwords from source `+0x4DC` are copied to AMCV `+0x4DC`, then source `+0x4F0/+0x4F4` are copied to AMCV and source sound event is detached with `SoundEvent__SetLoopHandle(source+0x4DC, 0, 0)`. Source `+0x4F0/+0x4F4` are set to `-1`.
16. If the captured selected byte was nonzero, global `g_SelectionVoice_Enable` is saved, set to `0`, AMCV vtable `+0x14C` is called, then the global is restored. This selects the AMCV without playing selection voice.
17. Each collected linked techno gets vtable `+0x3C8(new_amcv)`.
18. The temporary dynamic vector is freed if allocated/owned.
19. The source building is cleaned up unconditionally after success or failure: vtable `+0xDC(1)`, `SoundEvent__Release(source+0x6A0)`, vtable `+0xF8`, return `1`.

## 4. INI Keys

| Section/key | Stock value | Effect | Evidence | Active in YR |
|---|---:|---|---|---|
| `[MultiplayerDialogSettings] MCVRedeploys` | `yes` | Required for ConYard redeploy gate | `rulesmd.ini`, `0x00449BC0` | Yes |
| `[GACNST] ConstructionYard` | `yes` | Uses ConYard-special gate instead of non-ConYard shortcut | `rulesmd.ini`, `0x00449BC0` | Yes |
| `[GACNST] UndeploysInto` | `AMCV` | UnitType constructed on completion | `rulesmd.ini`, `0x00449E3B..0x00449E44` | Yes |
| `[GACNST] Strength` | `1000` | Source health-ratio denominator | `rulesmd.ini`, `0x005F5C60` | Yes |
| `[GACNST] Cost` | `3000` | Source refund getter input; not used on success except saved | `rulesmd.ini`, `0x00449E74..0x00449E80` | Yes |
| `[AMCV] Strength` | `1000` | New health multiplier via AMCV type `+0xA0` | `rulesmd.ini`, `0x0044A014..0x0044A020` | Yes |
| `[AMCV] DeploysInto` | `GACNST` | Confirms the pair but not read by this reverse branch | `rulesmd.ini` | Yes, outside this branch |
| `artmd.ini [GACNST] Foundation` | `4x4` | Selects large-foundation coordinate branch | `artmd.ini`, `0x00449E8E..0x00449F0E` | Yes |

## 5. Integration Points

`BuildingClass__CanUndeployMCV @ 0x00449BC0` gates command availability. `BuildingClass__Sell @ 0x00449C30` owns the pack-up state machine and final transfer.

The success branch is active in stock YR multiplayer/skirmish when MCV redeploy is enabled and the local/player-controlled GACNST completes its reverse buildup. The AMCV is placed by `UnitClass::Unlimbo`; no alternate-cell search was found in this branch.

The branch retargets live technos that held the old building in a target/link field before source cleanup, so consumers do not keep a dangling pointer to the removed GACNST.

## 6. Current Rust Implementation Status

Current Rust starts undeploy in `src/sim/world/world_spawn.rs::undeploy_building`. It stores `BuildingDown` with spawn type, owner, computed center cell, height, and `was_selected`.

Completion occurs in `src/sim/world/mod.rs::tick_building_down`: it despawns the building first, calls `spawn_object_at_height`, and only copies `selected` to the new unit. It does not preserve health ratio, source target/destination, sound-event fields, linked target/radio refs, powerup manager, source techno fields, or failure refund.

Current `BuildingDown` in `src/sim/components.rs` has no fields for source health, target/destination pointer, linked refs, sound-event state, powerup manager, or copied techno fields.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| GACNST redeploy command gate | verified | `0x00449BC0`; stock INI | none |
| State 2 completion gate | verified | `0x00449C47..0x00449CA1` | none |
| Unit construction and `g_MapEditorMode` bracket | verified | `0x00449E3B..0x00449E58`; `0x007353C0` | none |
| Health-ratio save and refund save separation | verified | `0x00449E66..0x00449E80` | none |
| Large-foundation coordinate calculation | verified | `0x00449E8E..0x00449F12`; `artmd.ini [GACNST] Foundation=4x4` | exact global constants' values not re-read |
| Linked techno scan | verified | `0x00449F58..0x00449FDC` | semantic name of `+0x2B4` remains partially inferred |
| Source map removal before AMCV unlimbo | verified | `0x00449FE2..0x0044A00A` | none |
| Success health calculation and min clamp | verified | `0x0044A014..0x0044A039`; `0x007C5F00` | none |
| Optional powerup manager rebind | verified | `0x0044A03C..0x0044A047`; `0x006AF580` | stock GACNST usually has none; custom cases not exhausted |
| `+0x214/+0x150` copies | verified write/copy | `0x0044A04C..0x0044A05E` | semantic names need separate field audit |
| Destination/order transfer | verified | `0x00449E84..0x00449E8A`; `0x0044A091..0x0044A0AE`; `0x00741970` | exact player-visible mission enum label external |
| Sound/event transfer | verified | `0x0044A0D4..0x0044A115`; `0x004060F0`; `0x00406060` | exact audible result depends on current event contents |
| Selection transfer | verified | `0x00449C4D..0x00449C56`; `0x0044A11B..0x0044A141` | control-group membership not separately observed |
| Linked techno retarget | verified | `0x0044A146..0x0044A15A` | vtable target function name not fully audited |
| Source building cleanup | verified | `0x0044A1B5..0x0044A1D2` | none |
| Current Rust parity | verified from source scan | `world_spawn.rs::undeploy_building`; `world/mod.rs::tick_building_down`; `components.rs::BuildingDown` | implementation missing most transfer state |

## 8. Open Questions - Final State

- `[RESOLVED] OQ1 - Is the success path live for stock YR GACNST? -> Yes, gated by stock `UndeploysInto=AMCV`, `ConstructionYard=yes`, and `MCVRedeploys=yes`.` (evidence: `0x00449BC0`; `rulesmd.ini`)`
- `[RESOLVED] OQ2 - Does successful transfer use source health ratio? -> Yes, `GetHealthRatio(source)` is saved before placement and multiplied by AMCV type strength after `Unlimbo` succeeds.` (evidence: `0x00449E66..0x00449E70`; `0x0044A014..0x0044A039`)`
- `[RESOLVED] OQ3 - Is health clamped? -> Yes, converted health is written, compared to `1`, and values `<=1` become exactly `1` before writing `+0x6C/+0x70`.` (evidence: `0x0044A029..0x0044A039`)`
- `[RESOLVED] OQ4 - Does success pay refund? -> No; success skips the failure `HouseClass__Add_Credits([ESP+0x30])` branch.` (evidence: `0x0044A008..0x0044A16B`)`
- `[RESOLVED] OQ5 - Is source removed before or after AMCV unlimbo? -> Source vtable `+0xD4` runs before AMCV `Unlimbo`; final source uninit runs after transfer.` (evidence: `0x00449FE2..0x0044A002`; `0x0044A1B5..0x0044A1D2`)`
- `[RESOLVED] OQ6 - Is selection transferred? -> Yes, selected byte `+0x83` is captured at function entry and if set, new AMCV select is called with selection voice disabled.` (evidence: `0x00449C4D..0x00449C56`; `0x0044A11B..0x0044A141`)`
- `[RESOLVED] OQ7 - Is control-group state explicitly copied? -> No explicit control-group array copy was found in this branch; only selected state and two techno fields `+0x214/+0x150` are copied.` (evidence: `0x0044A04C..0x0044A05E`; `0x0044A11B..0x0044A141`)`
- `[RESOLVED] OQ8 - Is destination/path order preserved? -> Source `+0x218` is saved before source removal; after AMCV health/field copies, if non-null the AMCV receives `Set_Destination(saved, 1)` and mission `2`.` (evidence: `0x00449E84..0x00449E8A`; `0x0044A091..0x0044A0AE`)`
- `[RESOLVED] OQ9 - Are sound/event fields moved? -> Yes, `+0x4DC..+0x4F4` are copied to AMCV; source sound event loop is detached and source `+0x4F0/+0x4F4` become `-1`.` (evidence: `0x0044A0D4..0x0044A115`; `0x004060F0`)`
- `[RESOLVED] OQ10 - Are linked technos rebound? -> Yes, technos with `+0x2B4 == source building` are collected before source removal and later receive vtable `+0x3C8(new_amcv)`.` (evidence: `0x00449F58..0x00449FDC`; `0x0044A146..0x0044A15A`)`
- `[RESOLVED] OQ11 - Does source cleanup still release source sound? -> Yes, after transfer the source runs `+0xDC(1)`, `SoundEvent__Release(source+0x6A0)`, and `+0xF8`.` (evidence: `0x0044A1B5..0x0044A1D2`)`
- `[DEFERRED] OQ12 - Exact semantic names/readers of `Techno+0x214`, `+0x150`, `+0x2B4`, and source `+0x34`.` (category: `requires-different-system-context`; reason: this slice proves copies/rebinds but not every reader; next-step-if-pursued: field audit across TechnoClass target/link/control-group systems)`
- `[DEFERRED] OQ13 - Exact custom-mod behavior for `Type+0x16CA` artillery/tick-tank timer/facing branch.` (category: `out-of-scope`; reason: stock GACNST does not set the flag; next-step-if-pursued: investigate non-stock deployer type with `+0x16CA`)`

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Successful AMCV health is `ftol(source_health / source_strength * AMCV_strength)`, clamped to min `1`, written to both health mirrors. | `0x005F5C60`; `0x0044A014..0x0044A039`; `0x007C5F00` | missing; Rust spawns default-health unit | `src/sim/world/world_spawn.rs::undeploy_building`; `src/sim/world/mod.rs::tick_building_down`; `src/sim/components.rs::BuildingDown` | Snapshot source health ratio when the branch starts or at completion to match binary timing, then apply to spawned AMCV on success. | Damage GACNST to half, redeploy successfully; AMCV appears at half of AMCV strength, never below 1 HP. | Do not copy raw source HP unless source and target strengths happen to match; do not use failed-refund value. |
| Success transfer preserves selected state by selecting the AMCV with selection voice suppressed; no explicit control-group copy was found. | `0x00449C4D..0x00449C56`; `0x0044A11B..0x0044A141` | partial; Rust copies `selected` only | selection/entity state; future audio selection voice gate | Preserve selected visual state without emitting selection voice. Leave control-group transfer unimplemented unless a separate control-group audit proves it. | Selected GACNST redeploys; resulting AMCV is selected, with no extra unit-selected voice event. | Do not invent control-group membership transfer from this branch. |
| Success path saves source destination/link field before source removal, then gives it to AMCV via `Set_Destination(saved, 1)` and mission `2`; linked technos targeting source are rebound to AMCV. | `0x00449E84..0x00449E8A`; `0x0044A091..0x0044A0AE`; `0x00449F58..0x00449FDC`; `0x0044A146..0x0044A15A` | missing | movement target/order state; radio/contact/target-link state in `GameEntity` | Carry source target/destination and rebind any radio/target-linked entities from old building to new AMCV after successful spawn. | Any unit linked/targeting the packed ConYard no longer points to a despawned entity after AMCV appears; AMCV inherits relevant move/order state. | Do not despawn source before collecting linked refs; binary scans first. |
| Sound/event state moves from source to AMCV; source sound handles are detached/cleared before final source release. | `0x0044A0D4..0x0044A115`; `0x004060F0`; `0x00406060`; `0x0044A1C2..0x0044A1D2` | missing/unmodeled | future sim audio event handles; current `sound_events` queue | If persistent looping sound handles are modeled, transfer the active sound-event subobject to AMCV and clear source handles before source uninit. | Redeploy during an active per-object loop does not leave a dangling source-owned loop. | Do not only copy one sound id; binary copies a 20-byte subobject plus two handle fields. |
| Source cleanup is terminal after success: source cell/map removal before AMCV unlimbo, final limbo/uninit/release after transfer. | `0x00449FE2..0x0044A00A`; `0x0044A1B5..0x0044A1D2` | mostly terminal, but Rust despawns before spawning and before link scan | `tick_building_down`; occupancy/despawn flow | Reorder completion so transfer data and linked refs are collected before despawn, then remove source and spawn AMCV with success/failure branches. | Successful redeploy leaves exactly one AMCV and no GACNST; linked refs and selected state survive. | Do not let `despawn_entity` erase source data before transfer snapshot. |

### Stale Docs / Follow-up Docs

- `docs/research/BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md:208-210` should use the replacement wording from `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md`: failed unlimbo refunds the separate integer saved at `[ESP+0x30]` from source vtable `+0x2BC`, not a high dword of the health-ratio double.
- `docs/research/BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md:213-230` should be refined to: `On successful AMCV Unlimbo, the new unit health is ftol(saved source health ratio * new AMCV type Strength), clamped to minimum 1, then written to +0x6C and +0x70. The branch optionally rebinds powerup manager state, copies source fields +0x214 and +0x150, optionally assigns saved source +0x218 as the AMCV destination with mission 2, moves sound-event state +0x4DC..+0x4F4 while clearing the source handles, selects the AMCV only if the source selected byte was captured at entry and does so with selection voice disabled, then retargets collected technos whose +0x2B4 pointed at the source building to the new AMCV.`

## 10. Negative Facts / Do Not Do

- Do not use the source building refund integer or AMCV cost for successful AMCV health; success uses saved health ratio times AMCV type strength.
- Do not run success transfer fields on failed `Unlimbo`; the failure branch jumps around health, destination, sound, selection, and linked-techno transfer.
- Do not despawn the source before collecting source `+0x218`, selected byte, sound fields, and technos pointing at source `+0x2B4`.
- Do not preserve source GACNST for retry after successful AMCV unlimbo; source cleanup always runs.
- Do not claim explicit control-group membership transfer from this branch; only selected state is directly verified here.

## 11. Remaining Uncertainty

- Exact semantic names and all readers for `Techno+0x214`, `Techno+0x150`, `Techno+0x2B4`, and source `+0x34` remain outside this slice. The copy/rebind operations are verified, but names should be treated as provisional in implementation notes.
- Exact custom-mod behavior for the `Type+0x16CA` timer/facing branch remains outside stock GACNST scope.

## Sources

- Ghidra read-only decompile/disassembly: `BuildingClass__Sell @ 0x00449C30`, `BuildingClass__CanUndeployMCV @ 0x00449BC0`, `ObjectClass__GetHealthRatio @ 0x005F5C60`, `Deploy_facing_calculator @ 0x00465D70`, `UnitClass__Constructor @ 0x007353C0`, `TechnoClass__Set_Destination @ 0x00741970`, `FUN_005F5B50`, `PowerUp_Cleanup @ 0x006AF580`, `SoundEvent__SetLoopHandle @ 0x004060F0`, `SoundEvent__Release @ 0x00406060`, `Math__ftol @ 0x007C5F00`.
- Prior docs referenced: `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md`, `GACNST_ISDEPLOYABLE_SPECIAL_BRANCH_GHIDRA_REPORT.md`, `GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`, `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md`, `SELECTION_LIFECYCLE_GHIDRA_REPORT.md`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini` `[MultiplayerDialogSettings]`, `[AMCV]`, `[GACNST]`; `ini/artmd.ini [GACNST]`.
- Rust scan: `src/sim/world/world_spawn.rs::undeploy_building`, `src/sim/world/mod.rs::tick_building_down`, `src/sim/components.rs::BuildingDown`, `src/sim/game_entity.rs`.
