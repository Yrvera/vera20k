---
name: Garrison Occupied Building Visual State
description: Healthy/damaged occupied CanBeOccupied civilian building body frames, BState gating, and anim-overlay variant swapping.
type: ghidra-report
date: 2026-05-27
---

# Garrison Occupied Building Visual State - Ghidra Report

## Target Question

Resolve what Rust should render for healthy and damaged occupied `CanBeOccupied=yes` civilian buildings:

- body SHP frame for healthy occupied buildings,
- body SHP frame for yellow/red damaged occupied buildings,
- whether `FUN_00458330` creates a healthy occupied overlay variant for stock static civilian art,
- which stock YR `artmd.ini` entries actually exercise occupied overlay variants.

## Non-goals

- Do not redo garrison combat firing, muzzle flash cadence, weapon selection, kill credit, or passenger lifecycle.
- Do not investigate tank bunker `Bunker=yes` visuals.
- Do not modify Rust, INI, Ghidra state, or sibling research docs.

## Evidence Needed To Mark COMPLETE

- `BuildingClass::GetCurrentFrame @ 0x0043EF90` decompile plus assembly context proving the `CanBeOccupied` body-frame formula and its `BuildingClass+0x534 != 0` gate.
- `BuildingClass::Update @ 0x0043FB20` evidence proving the function is active for `CanBeOccupied` buildings and calls `CheckAutoSellOrCivilian @ 0x00458200`.
- `CheckAutoSellOrCivilian @ 0x00458200` decompile plus assembly context proving `FUN_00458330` is called during civilian ownership reconciliation.
- `FUN_00458330 @ 0x00458330` decompile plus assembly context proving occupied/empty/damaged overlay string selection and the precondition that the corresponding live anim slot pointer is non-null.
- INI evidence from stock `rulesmd.ini`/`artmd.ini` for active `CanBeOccupied` buildings and animation keys.
- Rust surface scan for current body-frame and overlay rendering behavior.

## Stop Conditions

- Stop if Ghidra MCP read-only access is unavailable.
- Stop if the target expands into general building rendering or combat fire animation.
- Stop if evidence requires runtime screenshots or mutating Ghidra labels/comments.

## Findings

### 1. Body Frame Selection Is BState-Gated

**Verified behavior:** `BuildingClass::GetCurrentFrame @ 0x0043EF90` reads `BuildingClass+0x534` before the `CanBeOccupied` branch. If `+0x534 == 0`, it returns the current body frame `+0xF8` after only laser/firestorm/gate/mission handling; it does not inspect garrison occupancy. If `+0x534 != 0` and `Type+0x157B CanBeOccupied` is true, it computes the garrison body frame.

**Evidence:** Decompile `0x0043EF90`; assembly context `0x0043EFC6..0x0043F0B8`:

- `0x0043EFC6`: `MOV ECX,[ESI+0x534]`
- `0x0043EFCC`: `TEST ECX,ECX`
- `0x0043EFCE`: `JNZ 0x0043F02C`
- `0x0043F02C`: reads `Type+0x157B`
- `0x0043F03C..0x0043F040`: calls vtable `+0x408` occupant count
- `0x0043F04A`: sets base frame `2` when occupant count is positive
- `0x0043F05C`: compares health ratio against `Rules+0x1708 ConditionRed`
- `0x0043F07D..0x0043F094`: for buildable garrisons only, compares against `Rules+0x1700 ConditionYellow`
- `0x0043F09D..0x0043F0B3`: civilian collapse rule `TechLevel == -1 && frame == 3 -> 1`

**Active in YR:** Yes. `GetCurrentFrame` is the normal building body render query; `CanBeOccupied` stock buildings use `Type+0x157B` read by `BuildingTypeClass_ReadINI_Water @ 0x00460000`.

**Decision:** A healthy occupied civilian with `+0x534 == 0` renders stock body frame `+0xF8`, normally frame `0`, not frame `2`.

### 2. Damaged Occupied Civilian Body Frames Use Red Only For Civilian Art

**Verified behavior:** In the `CanBeOccupied` formula, occupant count starts at body frame `2`. Red health increments it to `3`, then civilian `TechLevel == -1` collapses `3` to `1`. Yellow health does not increment for civilian `TechLevel == -1`; yellow occupied civilian therefore remains frame `2` if the BState-gated branch is active.

**Evidence:** Decompile `0x0043EF90`; assembly context `0x0043F04F..0x0043F0B8`.

**Active in YR:** Yes for damaged `CanBeOccupied` civilian buildings once `Building+0x534` is nonzero. `rulesmd.ini` stock examples `CAGAS01`, `CABARN02`, `CABUNK01`, `CAMOV02`, and `CASEAT02` all set `TechLevel=-1` and `CanBeOccupied=yes`.

**Decision:** Rust needs a BState/damage-state gate around the garrison frame formula. Once that gate is active, occupied civilian yellow tier renders frame `2`, occupied civilian red tier renders frame `1`.

### 3. `FUN_00458330` Is A Live Ownership-Reconciliation Overlay Swap Helper

**Verified behavior:** `BuildingClass::Update @ 0x0043FB20` calls `BuildingClass::CheckAutoSellOrCivilian @ 0x00458200` for `Type+0x157B CanBeOccupied` buildings. `CheckAutoSellOrCivilian` calls `FUN_00458330` immediately before reverting an empty non-civilian building to Civilian side and immediately before transferring an occupied Civilian building to occupant slot 0 owner.

**Evidence:** Decompile `0x0043FB20` and `0x00458200`; assembly context:

- `0x00458205..0x00458212`: `CheckAutoSellOrCivilian` gates on `Type+0x634 == -1` civilian tech level.
- `0x004582EA`: empty branch calls `FUN_00458330` before vtable `+0x3D4` owner change.
- `0x0045831C`: occupied branch calls `FUN_00458330` before owner change to first occupant owner.
- `0x0043FE20..0x0043FE22`: `Update` calls `UpdateAnimation`.
- `0x004400F2` decompile region: `Update` calls `CheckAutoSellOrCivilian` when `Type+0x157B != 0`.

**Active in YR:** Yes for stock garrisonable civilian buildings during normal building update reconciliation.

### 4. Overlay Variant Swap Requires An Existing Live Slot

**Verified behavior:** `FUN_00458330 @ 0x00458330` checks live per-building anim-slot pointers before selecting a replacement art string. If the slot pointer is null, it skips that slot entirely. For slot `0x12`/idle (`Building+0x5A4`), it selects:

- empty + healthy: `Type+0x1414`,
- occupied + healthy: `Type+0x1434`,
- damaged: `Type+0x1424`.

For active slots it similarly selects:

- slot 3 / `Building+0x568`: `Type+0x1018` empty healthy, `+0x1038` occupied healthy, `+0x1028` damaged,
- slot 4 / `+0x56C`: `+0x105C`, `+0x107C`, `+0x106C`,
- slot 5 / `+0x570`: `+0x10A0`, `+0x10C0`, `+0x10B0`,
- slot 6 / `+0x574`: `+0x10E4`, `+0x1104`, `+0x10F4`.

**Evidence:** Decompile `0x00458330`; assembly context:

- `0x00458335..0x0045833D`: skip idle slot if `[ESI+0x5A4] == 0`.
- `0x00458363`: calls occupant count vtable `+0x408`.
- `0x00458376..0x00458394`: chooses `Type+0x1424/+0x1434/+0x1414`.
- `0x0045839D..0x004583AF`: checks non-empty string then calls `BuildingClass::CreateAnimForSlot @ 0x00451890` with slot `0x12`.
- `0x004583B4..0x00458431`: equivalent active slot 3 selection and create call.
- `0x004584AA..0x004584B3`: equivalent slot 4 create call.
- `0x00458511..0x0045853A`: equivalent slot 5 selection and create call.
- `0x00458578..0x004585A1`: equivalent slot 6 selection.

**Active in YR:** Yes, but only visible when the building type has a live animation slot and a non-empty selected string.

**Decision:** Do not render a generic occupied overlay for stock static civilian garrisons. The helper swaps existing building anim slots; it is not a body-frame override.

### 5. Stock YR Static Garrison Art Mostly Has No Occupied Overlay To Swap

**Verified INI evidence:** In active stock `rulesmd.ini`, there are 164 direct `CanBeOccupied=yes` entries. Parsing active keys from `artmd.ini` for those building sections found only two active `CanBeOccupied` image sections with actual Active/Idle animation name keys:

- `CAMOV02`: `ActiveAnim=CAMOV02_A`, `ActiveAnimDamaged=CAMOV02_AD` (`artmd.ini:7778..7779`; `rulesmd.ini:21716..21719`).
- `CASEAT02`: `ActiveAnim=CASEAT02_A`, `ActiveAnimDamaged=CASEAT02_AD` (`artmd.ini:11416..11417`; `rulesmd.ini:16297..16301`).

Common static stock garrisons such as `CAGAS01`, `CABARN02`, and `CABUNK01` have muzzle/damage fire offsets but no active `ActiveAnim`, `IdleAnim`, or occupied variant keys in `artmd.ini` (`CAGAS01 artmd.ini:8019..8041`, `CABARN02 artmd.ini:7056..7067`, `CABUNK01 artmd.ini:9224..9242`).

The only active `ActiveAnimGarrisoned=` key found in stock `artmd.ini` is `CAWASH19 ActiveAnimGarrisoned=CAWA19_AG` (`artmd.ini:8976..8978`), but its `rulesmd.ini` garrison flags are commented out (`rulesmd.ini:14609..14612`), so this is not active as a standard YR `CanBeOccupied` garrison.

**Active in YR:** Static no-overlay result: Yes for ordinary stock `CanBeOccupied` civilian garrisons. `CAWASH19 ActiveAnimGarrisoned` path: No in standard YR because `CanBeOccupied` is commented out. `CAMOV02`/`CASEAT02` normal ActiveAnim damaged swap: Yes when those buildings are present.

### 6. Current Rust Applies The Body Formula Too Broadly

**Current Rust shape:** `src/app_instances/shp.rs` calls `building_frame_index(...)` for every `CanBeOccupied` structure during body rendering, regardless of a native `Building+0x534` equivalent (`src/app_instances/shp.rs:143..161`, helper at `:659`). Tests currently expect `civilian_occupied_healthy_returns_2` and `civilian_occupied_yellow_tier_returns_2` unconditionally (`src/app_instances/shp.rs:760`, `:766`).

`emit_building_anims` also treats parsed `ActiveAnimGarrisoned` as a separate Rust-only overlay kind that loops while `is_garrisoned` (`src/app_instances/shp.rs:521..525`), and `art_data.rs` parses only `ActiveAnimGarrisoned`, not the full native `IdleAnimGarrisoned` / `ActiveAnimTwoGarrisoned` family (`src/rules/art_data.rs:903..918`).

**Active in YR:** Rust mismatch applies whenever a healthy occupied static civilian garrison is rendered.

## Implementation Handoff

1. **Verified behavior ->** `GetCurrentFrame` does not run the `CanBeOccupied` body-frame formula when `Building+0x534 == 0`; healthy occupied static civilians normally draw body frame `0`. **Rust delta ->** gate `building_frame_index` behind a native BState/damage-state equivalent, or for the immediate garrison visual fix, do not apply it for healthy `CanBeOccupied` buildings with no active damaged BState. **Affected surface ->** `src/app_instances/shp.rs` body frame selection and tests. **Acceptance scenario ->** healthy occupied `CAGAS01` renders body frame `0` with no body-frame 2 swap. **Proposed test name ->** `test_healthy_occupied_static_civilian_garrison_body_frame_stays_zero_without_bstate`. **Risk ->** High player visibility; currently every healthy occupied civilian garrison can show the wrong body frame.

2. **Verified behavior ->** once BState/damage state is active, occupied civilian yellow renders frame `2`, occupied civilian red collapses to frame `1`; buildable garrisons use yellow/red increments without the civilian collapse. **Rust delta ->** keep the existing frame formula but add the missing BState gate and preserve `TechLevel == -1 && frame == 3 -> 1`. **Affected surface ->** `src/app_instances/shp.rs` and any future building BState component. **Acceptance scenario ->** occupied `CAGAS01` at yellow under active damaged BState uses frame `2`; at red uses frame `1`. **Proposed test name ->** `test_occupied_civilian_garrison_bstate_red_collapses_to_frame_one`. **Risk ->** Medium; mostly damage-state screenshot parity.

3. **Verified behavior ->** `FUN_00458330` swaps an already-live native anim slot to occupied/empty/damaged variants during ownership reconciliation; it does not create a generic occupied overlay for static art. **Rust delta ->** remove or constrain the Rust-only generic `ActiveAnimGarrisoned` overlay behavior to native-parsed slot variants with live-slot semantics; do not invent an overlay for stock static `CAGAS01`/`CABARN02`/`CABUNK01`. **Affected surface ->** `src/rules/art_data.rs` parser and `src/app_instances/shp.rs::emit_building_anims`. **Acceptance scenario ->** `CAWASH19` does not show `CAWA19_AG` in standard YR because it is not `CanBeOccupied`; modded `ActiveAnimGarrisoned` only swaps an active slot after the base active slot exists. **Proposed test name ->** `test_active_anim_garrisoned_requires_native_live_slot_and_garrisonable_type`. **Risk ->** Medium for stock, high for mod parity if native slot replacement is approximated.

## Negative Facts / Do Not Do

- Do not set healthy occupied civilian body frame directly to `2`; `0x0043EF90` returns the `+0xF8` path when `Building+0x534 == 0`.
- Do not use `ActiveAnimGarrisoned` as a universal occupied-building visual; `FUN_00458330` first checks live slot pointers such as `Building+0x5A4`/`+0x568`.
- Do not treat `ConditionRed` as an anim-overlay threshold in `UpdateAnimation`; overlay branch health uses `Rules+0x1700 ConditionYellow`, while `ConditionRed` is used by `GetCurrentFrame` garrison body formula.
- Do not assume stock `CAWASH19` is a normal garrisoned ActiveAnimGarrisoned example; its `CanBeOccupied` flags are commented out in `rulesmd.ini`.
- Do not conflate combat muzzle flash `MuzzleFlashN` / `OccupyAnim` with body or building anim slot state; this report only covers body frame and building anim overlays.

## Remaining Uncertainty

- Exact initial value and writer call for `Building+0x534` on health-threshold crossing was not re-traced here beyond existing `SetDamagedState @ 0x00451EE0` / `UpdateAnimation @ 0x004509D0` reports. The render decision is still complete because `GetCurrentFrame`'s `+0x534` gate is binary-verified.
- Runtime screenshot confirmation of whether any shipped map places `CAMOV02` or `CASEAT02` occupied while their active anim slot is live was not performed. INI and binary paths show the mechanism is active if placed.

## Proposed Rust Test Names

- `test_healthy_occupied_static_civilian_garrison_body_frame_stays_zero_without_bstate`
- `test_occupied_civilian_garrison_bstate_red_collapses_to_frame_one`
- `test_occupied_civilian_garrison_bstate_yellow_uses_frame_two`
- `test_active_anim_garrisoned_requires_native_live_slot_and_garrisonable_type`
- `test_stock_cawash19_garrisoned_anim_key_is_inactive_without_canbeoccupied`

## Stale-Doc Replacement Wording

- `docs/research/GARRISON_FRAME_SWAP_GHIDRA_REPORT.md` §3.4: replace any implication that healthy occupied visible effects are likely from a generic overlay with: "Healthy occupied static civilian garrisons do not receive a body-frame 2 swap through `GetCurrentFrame`, and stock static entries such as `CAGAS01`, `CABARN02`, and `CABUNK01` have no active anim slot for `FUN_00458330` to swap. `FUN_00458330` is live during ownership reconciliation but only replaces already-live building anim slots."
- `docs/research/GARRISON_FRAME_SWAP_GHIDRA_REPORT.md` stock-count wording: replace "95 garrisonable types found in rulesmd.ini" with "the local `rulesmd.ini` contains 164 direct active `CanBeOccupied=yes` lines; only `CAMOV02` and `CASEAT02` among those have active `ActiveAnim`/`ActiveAnimDamaged` name keys in `artmd.ini`; the sole stock `ActiveAnimGarrisoned=CAWA19_AG` belongs to `CAWASH19`, whose garrison flags are commented out."
- `src/app_instances/shp.rs` tests are stale against the BState gate: the current `civilian_occupied_healthy_returns_2` expectation is valid only for the formula subroutine, not for healthy render output when native `Building+0x534 == 0`.

## Status

COMPLETE

