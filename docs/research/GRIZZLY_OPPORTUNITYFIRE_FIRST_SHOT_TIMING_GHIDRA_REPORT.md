# Grizzly OpportunityFire First-Shot Timing - Ghidra Report

**Target question:** When stock `[MTNK]` passively acquires a target through `OpportunityFire=yes` while on ordinary move, can it fire in that same frame/AI pass, or only after a later mission/combat dispatch?

**Investigation mode:** exhaustive-slice.

**Claimed scope:** static scheduling around `TechnoClass::AI_Update`, `MissionClass::Mission_Dispatch`, `FUN_00709290`, passive scanner `vtable+0x39C`, `UnitClass::AI`, `UnitClass::Fire_At_Target`, and turret-facing gate for stock Grizzly.

**Non-goals:** target ranking inside scanner `vtable+0x39C`, projectile/FLH origin, damage math, elite weapon cadence, and full mission-state recovery.

**Confidence:** High for scheduling order and same-pass fire eligibility; Medium for exact runtime cooldown/timer edge cases because no live runtime trace was used.

**Active in YR:** Yes. `[MTNK]` is stock YR content, `OpportunityFire=yes` parses to `TechnoType+0x6AF`, and the checked functions are live UnitClass/TechnoClass AI paths.

## Target Question

Does an ordinary-moving Grizzly that acquires a target through the `OpportunityFire` passive scan fire immediately in the same frame/AI pass, or does firing wait for the next mission/combat dispatch?

## Answer

It can attempt to fire in the same `UnitClass::AI` pass after passive acquisition, but not from `Mission_Dispatch` itself.

The precise distinction matters:

1. `TechnoClass::AI_Update` calls `MissionClass::Mission_Dispatch` first.
2. Later in that same `TechnoClass::AI_Update` pass, the `OpportunityFire` gate may run the passive scanner (`vtable+0x39C`) and set/change `ArchiveTarget`/TarCom at `+0x2B4`.
3. Ground-unit mission handlers, including `FootClass::Mission_Attack`, do not fire weapons.
4. `UnitClass::AI` then reaches its per-frame tail where `TurretAI` runs before `Fire_At_Target`.
5. `Fire_At_Target` can therefore see the target acquired earlier in the same UnitClass AI pass.
6. Actual firing still depends on normal weapon/cooldown/range/facing gates.
7. For a newly side-acquired Grizzly target, the shot does not happen that pass because `Fire_At_Target` runs before `Facing_Update`; the first pass can start turret rotation, and a later pass fires after `BarrelFacing` reaches the target.

So the answer is not "next mission dispatch." It is "same frame/AI pass is possible for an already fire-ready/aligned target; misaligned turret shots wait for later frame(s)."

## Non-Goals

- Do not recover the scanner's target ranking.
- Do not trace `PrimaryFireFLH`.
- Do not trace projectile launch or impact.
- Do not trace elite burst cadence.
- Do not modify Rust.

## Evidence Needed To Mark COMPLETE

- `OpportunityFire` passive scan runs after `Mission_Dispatch` in `TechnoClass::AI_Update`.
- Passive scan changes the current combat target field used by later fire code.
- Ground `Mission_Attack`/mission dispatch does not fire weapons for UnitClass.
- UnitClass per-frame fire gate runs after base/foot AI and after TurretAI, but before Facing_Update.
- Turret-facing gate explains the common side-target first-frame no-shot case.

All five are covered by existing Ghidra reports and byte/decompile evidence listed below.

## Stop Conditions

- Stop before scanner ranking internals.
- Stop before runtime debugger measurement of cooldown/timer edge cases.
- Stop before projectile/FLH/damage.
- Stop once same-pass eligibility versus next mission dispatch is resolved.

## Verified Findings

1. **Passive scan is after mission dispatch inside `TechnoClass::AI_Update`.** Active in YR: Yes. Evidence: `GRIZZLY_OPPORTUNITYFIRE_CONSUMER_GHIDRA_REPORT.md` records `TechnoClass__AI_Update @ 0x006F9E50`: `MissionClass__Mission_Dispatch()` runs first, then the mission `2/10/5` block calls `FUN_00709290`, writes `g_CurrentFrameCounter` to `+0x4FC`, and dispatches scanner `vtable+0x39C` (`0x006FA699..0x006FA6C1`).

2. **The passive scan path updates the combat target, not a separate "fire now" latch.** Active in YR: Yes. Evidence: `GRIZZLY_OPPORTUNITYFIRE_CONSUMER_GHIDRA_REPORT.md` and `TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md` identify `+0x2B4` as the resolved combat target/ArchiveTarget/TarCom, with passive acquisition saving old `+0x2B4`, running the scanner, and setting `+0x50C` if the target changed (`TechnoClass__Passive_Target_Acquire @ 0x00709480`, bytes `0x00709488..0x007094AD`).

3. **Ground mission dispatch is not the weapon-fire clock.** Active in YR: Yes. Evidence: `FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` verifies `FootClass::Mission_Attack @ 0x004D4DC0` does not call `Fire_At`; UnitClass uses that base mission handler unchanged, and firing happens in the per-frame `UnitClass::AI -> Fire_At_Target` chain. `MissionClass::Mission_Dispatch @ 0x005B3060` consumes handler return values as dispatch-delay frames, not weapon shots.

4. **UnitClass can consume a target later in the same AI pass.** Active in YR: Yes. Evidence: `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md` records `UnitClass::AI @ 0x007360C0` order: `FootClass::AI`/base work before `TurretAI`, then `Fire_At_Target`, then `Facing_Update`. It explicitly notes `TurretAI` runs before `Fire_At_Target`, so target acquisition before that fire gate can be attempted in the same tick.

5. **A newly misaligned Grizzly still does not fire immediately.** Active in YR: Yes. Evidence: `GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md` records `0x007365E1` `Fire_At_Target` before `0x007365E8` `Facing_Update`, and `Fire_At_Target @ 0x00736F78..0x00736FAC` sets `BarrelFacing` (`+0x3A0`) for `Turret=yes`; `[MTNK] ROT=5` stores `0x0500`, so a 90-degree side target waits about 12 binary frames before the fire gate is satisfied.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| OpportunityFire passive acquisition can feed the same frame's UnitClass fire gate when target/facing/cooldown are already ready. | Add passive acquisition before combat/fire processing, not only post-combat. | `src/sim/world/mod.rs`, `src/sim/world/world_orders.rs`, `src/sim/combat/mod.rs` | Moving Grizzly with `OpportunityFire=yes`, already turret-aligned toward a newly scanned target in weapon range, fires on the acquisition tick if cooldown is ready. | `grizzly_opportunity_fire_aligned_target_can_fire_same_tick` | Do not delay every passive acquire until the next mission-dispatch interval. |
| Mission dispatch itself does not fire UnitClass weapons; it only precedes passive scan and sets the next dispatch timer. | Keep attack firing in the per-frame combat phase, not inside a movement/mission timer emulation. | `src/sim/world/mod.rs`, `src/sim/combat/mod.rs`, future mission-state code | Passive-acquired target can fire before the next `Mission_Move` dispatch would occur. | `grizzly_opportunity_fire_not_waiting_for_next_mission_dispatch` | Do not tie first shot to the 14-16 frame move mission cadence. |
| Side-target Grizzly acquisition starts turret rotation but cannot fire until `BarrelFacing` reaches target because fire gate precedes same-tick facing update. | Preserve combat-before-turret-update ordering and add OpportunityFire-specific acceptance. | `src/sim/movement/turret.rs`, `src/sim/combat/combat_turret_facing_tests.rs` | Moving north Grizzly acquires east target through OpportunityFire; no damage on acquisition tick, then fire after `ROT=5` alignment. | `grizzly_opportunity_fire_side_target_waits_for_turret_alignment` | Do not make passive acquire rotate turret before the same tick's fire check. |

## Negative Facts / Do Not Do

- Do not put firing inside `FUN_00709290` or passive scanner `vtable+0x39C`; they acquire/change target.
- Do not require the next `MissionClass::Mission_Dispatch` before a UnitClass shot can occur.
- Do not assume every passive acquisition fires the same tick; normal cooldown, range, target validity, and turret-facing gates still apply.
- Do not rotate `BarrelFacing` before `Fire_At_Target` in the same tick to make side-acquired Grizzlies shoot immediately.
- Do not add an MTNK-specific branch; the behavior is generic UnitClass/TechnoClass plus stock INI flags.

## Remaining Uncertainty

No remaining uncertainty for the scheduling question: same-pass fire attempt is possible and next mission dispatch is not required.

Deferred runtime nuance: exact visible first-shot frame for a particular scenario still depends on current weapon cooldown, target angle, binary frame, scanner timer, and whether the turret was already aligned before acquisition. A runtime debugger trace would be useful for a golden scenario, but it is not needed to answer the same-pass versus next-dispatch edge.

## Stale-Doc Replacement Wording

`OpportunityFire=yes` passive acquisition is not a direct fire call and does not fire from `Mission_Dispatch`; `TechnoClass::AI_Update @ 0x006F9E50` runs `Mission_Dispatch` first, then the `OpportunityFire` gate/scanner may set the current combat target in the same AI update. For UnitClass ground vehicles, weapon firing is the later per-frame `UnitClass::AI -> Fire_At_Target` path, not the mission handler, so a newly acquired Grizzly target can be attempted in the same AI pass if range/cooldown/facing are already valid. For the common side-target case, `Fire_At_Target` runs before `Facing_Update`, so the acquisition pass starts `BarrelFacing` rotation and the shot waits until the turret timer reaches the target on a later frame.

## Sources

- Existing Ghidra report: `GRIZZLY_OPPORTUNITYFIRE_CONSUMER_GHIDRA_REPORT.md`.
- Existing Ghidra report: `GRIZZLY_TURRET_ROT_BODY_FIRE_SPLIT_GHIDRA_REPORT.md`.
- Existing Ghidra report: `UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`.
- Existing Ghidra report: `FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`.
- Existing Ghidra report: `TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini` `[MTNK] Primary=105mm`, `Turret=yes`, `ROT=5`, `OpportunityFire=yes`.
- Rust source scan: `src/sim/world/mod.rs`, `src/sim/world/world_orders.rs`, `src/sim/combat/mod.rs`, `src/sim/movement/turret.rs`.

**Status:** COMPLETE.
