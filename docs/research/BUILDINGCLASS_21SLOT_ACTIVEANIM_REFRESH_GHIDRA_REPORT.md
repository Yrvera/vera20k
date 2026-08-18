# BuildingClass 21-Slot ActiveAnim Refresh - Ghidra Research Report

**Address(es):** `0x004509D0`, `0x00451890`, `0x00451750`, `0x00451E40`, `0x00451EE0`, `0x0043F180`, `0x0043BD00`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** normal 21 building animation slots at `BuildingClass+0x55C..+0x5AC`, their slot layout, refresh/update triggers, radio/mark refresh behavior, owner/attachment semantics, replacement/clear lifecycle, and distinction from damage-fire slots.  
**Non-Scope:** 8 damage-fire slot spawn/cleanup details except contrast, full rendered pixel validation, and a complete caller census for every feature-specific building helper outside the 21-slot refresh family.  
**Confidence:** High for slot layout, helper semantics, update/radio refresh, owner non-attachment, and destructor cleanup; Medium for all rare feature-specific create/clear producers because this report verifies the core helpers and known drivers, not every xref body.  
**Active in YR:** Yes.

## 0. Investigation Contract

**Target question:** How do the normal 21 `BuildingClass` animation slots work: layout, update/refresh triggers, radio/mark refresh, owner-pointer behavior, replacement/clear lifecycle, and how are they different from damage-fire slots?

**Non-goals:** Do not redo damage-fire slot RNG/threshold cleanup; do not implement Rust; do not mutate Ghidra; do not patch sibling docs.

**Evidence needed to mark COMPLETE:** Decompile plus assembly/disassembly success for `UpdateAnimation`, `CreateAnimForSlot`, `SetAnimSlotImage`, `ClearAnimSlot`, `SetDamagedState`, `BuildingClass` vtable `+0x124` case `2`, and destructor cleanup; Rust scan for current surfaces; INI/art parser evidence for active slot keys.

**Stop conditions:** Stop when the core 21-slot helper model and Rust-facing deltas are proven; defer screenshot/pixel capture and unrelated feature-specific callers instead of expanding beyond the 21-slot model.

## 1. Overview

The normal building animation model is a fixed 21-entry `AnimClass*` array at `BuildingClass+0x55C`. These are not free app overlays and not `SetOwnerObject`-attached anims. They are building-owned slot pointers: `BuildingClass::CreateAnimForSlot @ 0x00451890` constructs native `AnimClass` objects with constructor row `delay=<caller extra>, loop=1, drawFlags=0x1600, zAdjust=0, reverse=0`, stores them into the slot array, and later building routines explicitly refresh coordinates, remap/translucency, shroud fields, facing, and shadow frame.

Damage-fire slots are a different system: 8 pointers at `BuildingClass+0x5C8..+0x5E4`, cleaned with anim vtable `+0xF8`, not `ClearAnimSlot`, and not refreshed by the 21-slot helper family.

## 2. Slot Layout / Key Offsets

| Slot | Building ptr | Type slot base | Primary active key family | Native role |
|---:|---:|---:|---|---|
| 0 | `+0x55C` | `+0x0F4C` | PowerUp1-like slot | upgrade/power-state family |
| 1 | `+0x560` | `+0x0F90` | PowerUp2-like slot | upgrade/power-state family |
| 2 | `+0x564` | `+0x0FD4` | PowerUp3-like slot | upgrade/power-state family |
| 3 | `+0x568` | `+0x1018` | `ActiveAnim` | active/refinery tier 0/infantry-absorb |
| 4 | `+0x56C` | `+0x105C` | `ActiveAnimTwo` | active/refinery tier 1/infantry-absorb |
| 5 | `+0x570` | `+0x10A0` | `ActiveAnimThree` | refinery tier 2 |
| 6 | `+0x574` | `+0x10E4` | `ActiveAnimFour` | refinery tier 3+ |
| 7 | `+0x578` | `+0x1128` | `PreProductionAnim` | production/deploy event slot |
| 8 | `+0x57C` | `+0x116C` | `ProductionAnim` | production/depart guard slot |
| 9 | `+0x580` | `+0x11B0` | `TurretAnim` | turret/shadow-direction special slot |
| 10 | `+0x584` | `+0x11F4` | `SpecialAnim` | silo/refinery/special event slot |
| 11 | `+0x588` | `+0x1238` | `SpecialAnimTwo` | repair/special slot |
| 12 | `+0x58C` | `+0x127C` | `SpecialAnimThree` | repair/special slot |
| 13 | `+0x590` | `+0x12C0` | `SpecialAnimFour` | reserved/special slot |
| 14 | `+0x594` | `+0x1304` | `SuperAnim` | superweapon/low-power slot |
| 15 | `+0x598` | `+0x1348` | `SuperAnimTwo` | superweapon charge slot |
| 16 | `+0x59C` | `+0x138C` | `SuperAnimThree` | power-state/SW slot |
| 17 | `+0x5A0` | `+0x13D0` | `SuperAnimFour` | superweapon charge slot |
| 18 | `+0x5A4` | `+0x1414` | `IdleAnim` | idle slot |
| 19 | `+0x5A8` | `+0x1458` | `LowPower` family | low-power slot |
| 20 | `+0x5AC` | `+0x149C` | `SuperLowPower` family | shadow/power-state slot |

Each type slot is `0x44` bytes. Relative fields used by the helpers are `+0x00` undamaged name, `+0x10` damaged name, `+0x20` firing name, `+0x30` resolved `AnimTypeClass*`, `+0x38/+0x3C` draw metadata as used by the constructor helper, and `+0x40..+0x42` power-state flags.  
**Active in YR: Yes.** Evidence: `CreateAnimForSlot @ 0x00451890` decompile uses `slot * 0x44`; `ClearAnimSlot @ 0x00451E40` loops `0x15` entries; disassembly ranges `0x00451890..0x00451B4F`, `0x00451E40..0x00451EAF`.

## 3. Core Logic

### 3.1 Creation and Replacement

**Active in YR: Yes.** `BuildingClass::CreateAnimForSlot @ 0x00451890`:

- If `Building+0x6E6 IsDamaged` differs from the caller's damaged flag, it first writes the new damage byte and re-images every occupied slot by calling `SetAnimSlotImage` for indexes `0..20`.
- It resolves the slot anim through the type slot's resolved anim field and constructs an `AnimClass` with `operator_new(0x1C8)` and `AnimClass__Constructor(anim_type, coords, extra_arg, 1, 0x1600, 0, 0)`.
- It writes draw metadata to `AnimClass+0x100`, `+0x104`, sets `+0x118=1`, propagates building hidden/cloak byte to `AnimClass+0x199`, propagates translucency to `AnimClass+0x178`, and handles shroud-related anim types through `AnimType+0x35C`.
- If the destination slot is already occupied, it copies old `AnimClass+0xAC` into the new anim, nulls the slot, destroys the old anim through vtable `+0x20(1)`, then stores the new pointer.
- Slot 9 plus `Type+0x16C6` sets `AnimClass+0x19D=1`; vtable `+0x1D4` true sets `AnimClass+0x11A=1`.

This is replacement, not "skip if occupied" and not `SetOwnerObject`. Evidence: decompile `0x00451890`; disassembly success `0x00451890..0x00451B4F`.

### 3.2 Art Variant Selection

**Active in YR: Yes.** `SetAnimSlotImage @ 0x00451750` selects:

- undamaged: `Type + 0xF4C + slot*0x44`,
- damaged: `Type + 0xF5C + slot*0x44`,
- firing: `Type + 0xF6C + slot*0x44`.

It returns if the selected string is null or empty; otherwise it delegates to `CreateAnimForSlot`. Evidence: decompile `0x00451750`; disassembly success `0x00451750..0x004517BF`.

### 3.3 Clearing

**Active in YR: Yes.** `ClearAnimSlot @ 0x00451E40` clears one slot or, for sentinel `-2`, all 21 slots. For every non-null slot it writes null first, then calls old anim vtable `+0x20(1)`. Null slots no-op. Evidence: decompile `0x00451E40`; disassembly success `0x00451E40..0x00451EAF`.

### 3.4 Per-Tick Update Driver

**Active in YR: Yes.** `UpdateAnimation @ 0x004509D0` is called from `BuildingClass::Update @ 0x0043FB20` and drives:

- frame/BState timer at `+0xF8/+0x100..+0x110`,
- facing/remap pass over all occupied slots via `UpdateAnimFacingAndDirection @ 0x00451F60` and `SetAnimRemap @ 0x00452170`,
- repair/UnitRepair slots `8`, `11`, `12`,
- infantry-absorb slots `3` and `4`,
- silo/special slot `10`,
- refinery tier slots `3..6`,
- superweapon charge slots `14..17`,
- terminal BState transition that calls vtable `+0x124(2)`.

Health variant selection inside this function uses `Rules.ConditionYellow` at `RulesClass+0x1700`; `ConditionRed` is not used by the 21-slot active anim refresh. Evidence: decompile `0x004509D0`; prior report disassembly plus current live decompile.

### 3.5 Radio/Mark Refresh

**Active in YR: Yes/Conditional.** Building vtable `+0x124` target `0x0043F180` has a mode `2` refresh branch. It does not create or destroy 21-slot anims. It:

- computes owner/visibility/shroud context through vtable `+0x1E4`, `+0x1BC`, `+0x464`,
- writes building cache at `Building+0x700`,
- loops all `21` slots from `Building+0x55C`,
- for occupied shroud-aware anim types (`AnimType+0x35C != 0`), updates anim `+0xD4` and `+0xFC`,
- propagates building translucency/remap byte `Building+0x6ED` to `AnimClass+0x178` for occupied slots, with `0x0F -> 0x10` override for Yuri-side visual state.

This is the radio/mark refresh effect reached by generic `ObjectClass::Receive_Radio(0x0D) -> vtable+0x124(2)` for non-swallowing receivers, and by BState terminal advance in `UpdateAnimation`. Evidence: decompile `0x0043F180`; disassembly success `0x0043F180..0x0043FA8F`; prior radio report for ObjectClass `0x0D` caller evidence. Active condition: only receivers that actually reach the generic ObjectClass fallback; `WeaponsFactory=yes` buildings swallow `0x0D`.

### 3.6 Mark Put / Coord Refresh

**Active in YR: Yes.** `0x0043F180` cases `1` and `3` handle building placement/mark effects. In the placement branch, after adding building occupation, it loops slot pointers at `param_1+0x157` (`+0x55C`) while slot-type offset `< 0x594`, computes `building absolute coords + type slot offset`, and calls each anim's vtable `+0x1B4` to set coordinates. This is how building slot anims follow a building's placement/mark state; they are not owner-relative `SetOwnerObject` followers. Evidence: decompile `0x0043F180`; disassembly success `0x0043F180..0x0043FA8F`.

## 4. Owner / Attachment Semantics

**Active in YR: Yes.** The 21 normal building anim slots are stored/managed by the building but not `SetOwnerObject`-attached:

- `CreateAnimForSlot` directly calls `AnimClass__Constructor` and stores the result in `Building+0x55C+slot*4`.
- It does not call `SetOwnerObject @ 0x00424B50` and does not set `AnimClass+0xCC` owner attachment.
- Coordinate refresh is performed explicitly by building mark/refresh code, not by `AnimClass::GetCoords` owner-offset behavior.
- Cleanup uses `ClearAnimSlot` and anim vtable `+0x20(1)`.

This differs from temporal `SQDG`, parachute, or other owner-attached anims, and also differs from damage fires. Evidence: decompile `0x00451890` and `0x0043F180`; attached-owner report for `SetOwnerObject` contrast.

## 5. Lifecycle And Cleanup

**Active in YR: Yes.**

- Damage-state refresh: `SetDamagedState @ 0x00451EE0` checks `Building+0x6E6`, then re-images occupied slots across the same `0x15` slot span through `CreateAnimForSlot`; evidence `0x00451EE0..0x00451F5F`.
- Per-slot power-state creation/clearing: `OnPowerOff @ 0x004545D0` and `OnPowerOn @ 0x004547C0` inspect slot power flags and call `CreateAnimForSlot` / `ClearAnimSlot`; evidence decompile and disassembly success `0x004545D0..0x004547BF`.
- Destructor cleanup: `BuildingClass::~BuildingClass @ 0x0043BD00` calls `BuildingClass__ClearAnimSlot` after `BuildingClass__Limbo`, then separately iterates the 8 damage-fire slots at `+0x5C8`. This proves two cleanup mechanisms in a fixed order. Evidence: decompile `0x0043BD00`; disassembly success `0x0043BD00..0x0043BE2F`.
- `BuildingClass::Limbo @ 0x00445880` directly clears damage-fire slots but does not show the same `ClearAnimSlot(-2)` 21-slot loop in its early removal path; final full object destruction reaches the destructor cleanup. Evidence: decompile `0x00445880`.
- `BuildingClass::Sell @ 0x00449C30` state `0` clears damage-fire slots immediately, but the observed state-0 body does not directly clear the 21-slot array; later object removal/destruction handles normal slot cleanup. Evidence: decompile `0x00449C30`.

## 6. INI Keys / Art Data

**Active in YR: Yes.** Stock `artmd.ini` declares the 21-slot families through `ActiveAnim*`, `IdleAnim*`, `SpecialAnim*`, `ProductionAnim`, `SuperAnim*`, and power-state variants. The Rust parser currently parses a subset: `ActiveAnim` through `Four`, `IdleAnim` through `Two`, `SuperAnim`, `SpecialAnim` through `Three`, and `ProductionAnim` (`src/rules/art_data.rs`). It does not model all 21 native slots, per-slot power flags, native `AnimStates_0`, or the slot pointer lifecycle.

## 7. Current Rust Implementation Status

Current Rust is an app-layer overlay model, not a native 21-slot model:

- `src/sim/components.rs` has `BuildingAnimOverlays { anims: Vec<AnimOverlayState> }`, not fixed 21 slots with native slot indexes and `AnimClass*`-like identities.
- `src/app_building_anim.rs` advances one-shot overlays, triggers producer crane overlays, and event-triggers refinery `SpecialAnim`; infinite-loop anims use a global idle timer.
- `src/app_instances/shp.rs` renders building anim configs directly from art metadata and optional overlay frame state. It suppresses refinery secondary ActiveAnim layers with a Rust-specific tier shortcut.
- No sim-side fixed slot occupancy exists for native branch consumers like "wait while `Building+0x57C` slot 8 is non-null".

Current Rust can approximate common stock visuals, but it cannot preserve native slot replacement, `+0xAC` transfer, power-state `AnimStates_0`, vtable `+0x124(2)` refresh, or slot-presence-dependent gameplay/timing.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working-note contract | verified | section 0 | none |
| 21-slot pointer layout | verified | `0x00451890`, `0x00451E40` | none |
| Type slot stride and art variant names | verified | `0x00451750`, `0x00451890`, `artmd.ini`, Rust parser scan | rare unnamed slot labels can be refined |
| `CreateAnimForSlot` creation/replacement | verified | decompile `0x00451890`, disasm success `0x00451890..0x00451B4F` | none |
| `SetAnimSlotImage` selector | verified | `0x00451750`, disasm success | none |
| `ClearAnimSlot` clear/destroy | verified | `0x00451E40`, disasm success | exact stack arg at each caller not exhaustively listed |
| `UpdateAnimation` main driver | verified | `0x004509D0` decompile | branch table inherited from prior full report |
| `SetDamagedState` mass re-image | verified | `0x00451EE0`, disasm success | none |
| vtable `+0x124(2)` radio/mark refresh | verified | `0x0043F180`, disasm success | screenshot-level frame delta deferred |
| Destructor cleanup ordering | verified | `0x0043BD00`, disasm success | none |
| Limbo/sell direct 21-slot behavior | touched-not-exhausted | `0x00445880`, `0x00449C30` | full object-removal caller timing can be refined |
| Every caller of `CreateAnimForSlot` / `ClearAnimSlot` | deferred | prior helper doc reports 20+ callers | separate caller-census slice if needed |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-01 - Where are the normal building anim slots? -> 21 pointers at Building+0x55C..+0x5AC.` (evidence: `0x00451890`, `0x00451E40`)
- `[RESOLVED] OQ-02 - Are they damage-fire slots? -> No; damage fires are the separate 8-slot block at +0x5C8..+0x5E4.` (evidence: `0x0043BD00`, `0x00445880`, prior damage-fire report)
- `[RESOLVED] OQ-03 - Are 21-slot anims SetOwnerObject-attached? -> No; creation stores pointers directly and refreshes coords through building routines.` (evidence: `0x00451890`, `0x0043F180`)
- `[RESOLVED] OQ-04 - What constructor row is used? -> `AnimClass__Constructor(anim_type, coords, extra_arg, 1, 0x1600, 0, 0)`.` (evidence: `0x00451890`)
- `[RESOLVED] OQ-05 - What happens on occupied-slot replacement? -> Old `+0xAC` is copied, slot is nulled, old anim destroyed with vtable +0x20(1), new pointer stored.` (evidence: `0x00451890`)
- `[RESOLVED] OQ-06 - How are damaged variants refreshed? -> `SetDamagedState` and `CreateAnimForSlot` mass re-image occupied slots when `Building+0x6E6` changes.` (evidence: `0x00451EE0`, `0x00451890`)
- `[RESOLVED] OQ-07 - What does radio/mark mode 2 do? -> It updates shroud/remap/translucency fields on existing slots; it does not allocate or clear slots.` (evidence: `0x0043F180`)
- `[RESOLVED] OQ-08 - What does full destruction do? -> Destructor calls `ClearAnimSlot` for 21 slots, then separately clears damage-fire slots.` (evidence: `0x0043BD00`)
- `[RESOLVED] OQ-09 - Does sell state 0 directly clear 21 normal slots? -> No direct `ClearAnimSlot` in the observed state-0 damage-fire cleanup section; object removal later reaches destructor cleanup.` (evidence: `0x00449C30`, `0x0043BD00`)
- `[RESOLVED] OQ-10 - Does Rust currently have fixed 21 slots? -> No; it uses `BuildingAnimOverlays` plus direct render of art metadata.` (evidence: `src/sim/components.rs`, `src/app_building_anim.rs`, `src/app_instances/shp.rs`)
- `[DEFERRED] OQ-11 - What exact pixels change for each radio/mark refresh?` (category: `needs-runtime-debugger`; reason: Ghidra proves state writes, not framebuffer output; next-step-if-pursued: breakpoint `0x0043F180` mode 2 and capture slot frames before/after)
- `[DEFERRED] OQ-12 - What is the complete xref matrix for every rare create/clear caller?` (category: `bounded-cost-too-high`; reason: not needed to prove core model; next-step-if-pursued: separate caller-census report)

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Normal building anims are fixed 21 building-owned slots, not a free vector of one-shot overlays. | `0x00451890`, `0x00451E40` | mismatch | `src/sim/components.rs`, `src/app_building_anim.rs`, `src/app_instances/shp.rs` | Add a native slot model or equivalent state preserving slot index, occupancy, replacement, and per-slot metadata. | `building_active_anim_slot8_presence_gates_refinery_state4_wait` | Do not collapse all building anims into render-only `BuildingAnimOverlays`. |
| `CreateAnimForSlot` replacement copies old `AnimClass+0xAC`, destroys old with vtable `+0x20(1)`, then stores the new pointer. | `0x00451890` | missing | future generic AnimClass runtime / building slot manager | Preserve replacement ordering and `+0xAC` carryover for storage/special slots. | `building_anim_slot_replacement_preserves_ac_and_destroys_old_before_store` | Do not skip replacement just because a slot is occupied. |
| vtable `+0x124(2)` refresh updates existing slot coords/shroud/remap/translucency, and `WeaponsFactory=yes` radio `0x0D` may skip reaching it. | `0x0043F180`; prior radio reports | missing/unchecked | radio receiver layer, building slot refresh, render material state | Model mode-2 refresh as a state refresh on existing slots, not as creation or reset. | `building_radio_0d_non_wf_refreshes_existing_slot_state_without_restarting_anim` | Do not use `0x0D` to restart war-factory production overlays. |
| 21-slot cleanup and damage-fire cleanup are distinct: active slots use `ClearAnimSlot`/vtable `+0x20`; damage fires use 8-slot `+0xF8` cleanup. | `0x0043BD00`, `0x00451E40` | partial/mismatch | building lifecycle/despawn, damage-fire bridge | Keep independent cleanup paths and ordering. | `building_destructor_clears_21_active_slots_before_independent_damagefire_cleanup` | Do not treat damage-fire overlay removal as normal active anim cleanup. |
| Native per-tick driver uses `ConditionYellow`, power flags, `AnimStates_0`, and slot presence as state. | `0x004509D0`, `0x004545D0`, `0x004547C0` | missing/partial | `src/rules/art_data.rs`, app/sim building anim runtime | Preserve per-slot power/damage/tier state rather than deriving all frames from global elapsed time. | `building_poweroff_onpoweron_updates_slot_animstates_and_slots` | Do not use final render frame selection as a substitute for native slot state. |

## 11. Negative Facts / Do Not Do

- Do not call `SetOwnerObject` for the normal 21 building anim slots; native code stores building-owned pointers and refreshes coords manually.
- Do not use `drawFlags=0x600` for these slots; `CreateAnimForSlot` uses `0x1600`.
- Do not merge normal active anim slots with damage-fire slots; their storage offsets, cleanup vtable calls, and lifecycle triggers differ.
- Do not key all damage/variant changes directly off current health at render time only; native code stores `Building+0x6E6` and re-images occupied slots.
- Do not treat radio/mark `+0x124(2)` as a spawn/reset event; it refreshes existing slots.

## 12. Remaining Uncertainty

- Exact pixel/frame delta for mode-2 refresh requires runtime capture.
- A complete rare-caller matrix for every `CreateAnimForSlot`/`ClearAnimSlot` xref remains a separate census, but the core helper semantics are verified.
- Some legacy slot names for slots 0..2 and 11..20 can be refined against the full BuildingTypeClass parser, though their pointer/stride behavior is already proven.

## 13. Stale Docs / Replacement Wording

- `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`: replace any wording that groups building active slots with generic free `AnimClass` rows with: "Building active animations are stored in the fixed 21-slot `BuildingClass+0x55C` pointer array. They are created by `BuildingClass::CreateAnimForSlot` with `drawFlags=0x1600`, refreshed by building mark/update paths, and cleared by `ClearAnimSlot`, not by owner attachment or WorldEffect defaults."
- `BUILDING_VTABLE_0X124_RADIO_0X0D_VISUAL_DELTA_GHIDRA_REPORT.md`: replace "attached animation refresh" shorthand with: "`0x0043F180(mode=2)` refreshes existing 21-slot building anim pointers: shroud/visibility fields for `AnimType+0x35C` entries and remap/translucency byte `+0x178`; it does not allocate, clear, or restart slots."
- `REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md`: keep the 21-slot helper facts, but amend constructor wording to include the row `delay=<caller extra>, loop=1, drawFlags=0x1600, zAdjust=0, reverse=0` and to state explicitly that these slots are not `SetOwnerObject`-attached.
- Rust-facing stale comments in `src/sim/components.rs` / `src/app_building_anim.rs` should avoid describing `BuildingAnimOverlays` as equivalent to native 21 slots; it is an app-side bridge until fixed-slot state exists.

## Sources

- Ghidra decompile: `0x0043F180`, `0x004509D0`, `0x00451890`, `0x00451750`, `0x00451E40`, `0x00451EE0`, `0x0043BD00`, `0x00445880`, `0x00449C30`, `0x004545D0`, `0x004547C0`.
- Ghidra disassembly success: `0x0043F180..0x0043FA8F`, `0x00451890..0x00451B4F`, `0x00451750..0x004517BF`, `0x00451E40..0x00451EAF`, `0x00451EE0..0x00451F5F`, `0x0043BD00..0x0043BE2F`, `0x004545D0..0x004547BF`.
- Prior reports: `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`, `REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md`, `BUILDING_VTABLE_0X124_RADIO_0X0D_VISUAL_DELTA_GHIDRA_REPORT.md`, `ANIMCLASS_ATTACHEDOWNER_DETACH_LIFECYCLE_GHIDRA_REPORT.md`, `BUILDING_DAMAGEFIRE_SLOT_CLEAR_DESTROY_LIFECYCLE_GHIDRA_REPORT.md`.
- INI/Rust scan: `ini/artmd.ini`, `src/sim/components.rs`, `src/app_building_anim.rs`, `src/app_instances/shp.rs`, `src/rules/art_data.rs`.

## Status

COMPLETE for the bounded 21-slot active building animation refresh/lifecycle slice.
