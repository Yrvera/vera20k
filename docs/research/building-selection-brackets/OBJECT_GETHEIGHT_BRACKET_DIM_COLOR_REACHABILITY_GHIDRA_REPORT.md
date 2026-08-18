# ObjectClass::GetHeight and Building Bracket Dim Color - Ghidra Research Report

**Address(es):** `0x005F5F40` primary; bracket consumers `0x006F60D0`, `0x006F5190`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `ObjectClass::GetHeight` formula and reachability of selected-building bracket palette `0x0C` via `GetHeight() < -4`.  
**Non-Scope:** full bridge-height initializer audit, full temporal/warp state machines, and implementation changes.  
**Confidence:** High for formula and bracket branch; Medium for "standard YR reachability" because that part combines binary evidence with standard placement/state constraints.  
**Active in YR:** Conditional. The branch is active for selected drawn buildings; the dim color is reachable only if a selected building's above-ground height is less than `-4` leptons.

## 1. Overview

Selected building brackets choose palette index `0x0F` normally and switch to palette index `0x0C` only when the building vtable `+0x1C8` result is `< -4`. For `BuildingClass`, that vtable entry resolves to `ObjectClass::GetHeight @ 0x005F5F40`.

`GetHeight` is an above-ground measurement, not raw Z. It subtracts the terrain ground height at the object's current coordinate, and subtracts the bridge-height correction again when `ObjectClass+0x8C OnBridge` is set.

## 2. Key Offsets

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass+0x8C` | `OnBridge` byte | read by `0x005F5F40`; written by bridge/unlimbo paths in prior bridge report | Yes |
| `ObjectClass+0x9C` | `Location.X` | read by `0x005F5F40`, written by `0x005F6940` | Yes |
| `ObjectClass+0xA0` | `Location.Y` | read by `0x005F5F40`, written by `0x005F6940` | Yes |
| `ObjectClass+0xA4` | `Location.Z` | read by `0x005F5F40`, written by `0x005F6940` | Yes |
| `0x00AC13BC` | bridge Z correction used by GetHeight/SetHeight | read by `0x005F5F40`, `0x005F5FA0`; cold image reads zero because runtime-initialized | Conditional |
| Building vtable `0x007E4084` | `+0x1C8 = 0x005F5F40` | memory read `40 5F 5F 00` | Yes |
| Building vtable `0x007E4088` | `+0x1CC = 0x005F5FA0` | memory read `A0 5F 5F 00` | Yes |

## 3. Core Logic

`ObjectClass::GetHeight @ 0x005F5F40`:

```text
ground_z = CellClass::GetGroundHeight({ Location.X, Location.Y, Location.Z })
height = Location.Z - ground_z
if OnBridge != 0:
    height -= DAT_00AC13BC
return height
```

Active in YR: Yes. Evidence: direct decompile of `0x005F5F40`.

`ObjectClass::SetHeight @ 0x005F5FA0` applies the inverse correction before writing `Location.Z`: if `OnBridge` is set, it adds `DAT_00AC13BC`, then stores `CellClass::GetGroundHeight(Location) + requested_height`. Active in YR: Yes. Evidence: direct decompile of `0x005F5FA0`; building vtable `+0x1CC` points there.

`TechnoClass::DrawBehind @ 0x006F60D0` and `TechnoClass::DrawExtras @ 0x006F5190` both initialize bracket palette index to `0x0F`, call `vtable+0x1C8`, compare the result against `-4`, and replace the palette index with `0x0C` only on `< -4`. Active in YR: Yes, when `WhatAmI()==6` and selected byte `+0x83 != 0`. Evidence: assembly context at `0x006F6109..0x006F6119` and `0x006F53AD..0x006F53C0`.

## 4. Reachability

### Normal placed building

Active in YR: Yes for the normal bracket path; No evidence that it reaches dim color. A normal selected building with `Location.Z == ground_z` and `OnBridge == 0` returns `0`, so it uses palette `0x0F`.

Evidence:
- `ObjectClass::Reveal @ 0x005F4EC0` copies reveal coordinates through vtable `+0x1B4`.
- Building vtable `+0x1B4` resolves to `ObjectClass::Set_Raw_Coords @ 0x005F6940`, which writes `Location.X/Y/Z` exactly.
- `BuildingClass::GetCoords @ 0x00447AC0` returns foundation-centered X/Y but preserves `Location.Z`.

### Bridge / OnBridge correction

Active in YR: Conditional. The branch exists and can mathematically force dim color if a selected drawn building has `OnBridge=1` while its raw `Location.Z` is not bridge-raised. Then `GetHeight = Location.Z - ground_z - DAT_00AC13BC`, which is below `-4` for any normal nonzero bridge correction.

Standard-YR reachability for ordinary buildings is not supported by this slice. Buildings are not normal bridge-deck movers, and `SetHeight` compensates `OnBridge` by adding the bridge correction before writing Z. A forced/map-corrupt/scripted state can hit the branch; normal placed buildings should not.

Evidence:
- `ObjectClass::Unlimbo @ 0x005F5940` can set `OnBridge=1` for bridge-flagged target cells, but this is a generic object path and can fail before normal placement.
- `ObjectClass::SetHeight @ 0x005F5FA0` adds `DAT_00AC13BC` when `OnBridge` is true.
- Prior `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md` identifies `+0x8C` as live YR bridge-layer state.

### Limbo / conceal

Active in YR: Yes; not a dim-color reachability path. `ObjectClass::Conceal @ 0x005F4D30` deselects via vtable `+0x150`, removes the object from the display layer, then sets `InLimbo=1`. Since selected building brackets are drawn from the render loop for displayed objects, a concealed/limbo building should not remain selected and drawn through the bracket path.

### Falling / drop-in

Active in YR: Conditional for objects; not a normal building bracket case. `ObjectClass::DropIn @ 0x005F4160` clears `OnBridge` before resubmitting the object to the display layer. If a building-like object were forced into falling/drop-in, this path removes the bridge subtraction that would otherwise make height negative.

### Temporal / warped / underground / sinking

Active in YR: Conditional or class-specific, but no standard selected-building dim-color path was found in this slice. The bracket gate itself does not test visual state, underground, sinking, or temporal flags; it only checks selected building RTTI and `GetHeight() < -4`. Underground and sinking are not normal `BuildingClass` movement states. Temporal/warp visuals may affect drawing elsewhere, but no evidence in the bracket code shows they alter `Location.Z` or `OnBridge`.

## 5. INI Keys

No bracket dim-color INI key was found. `PixelSelectionBracketDelta` appears in `rulesmd.ini`, but this investigation confirms it is not the selected-building line-bracket dim-color gate.

`DAT_00AC13BC` is runtime-initialized bridge geometry state, not a direct value read from a visible `rulesmd.ini` key in this slice. The cold Ghidra memory image reads `0`, matching prior bridge reports that warn these bridge constants are initialized at runtime.

## 6. Rust Implementation Status

Follow-up `BUILDING_BRACKET_GETHEIGHT_DIM_COLOR_REACHABILITY_FOLLOWUP_GHIDRA_REPORT.md`
confirms the standard-YR reachability verdict: the selected-building bracket path is
live and the dim palette branch exists, but normal placed/revealed selected buildings
do not produce `GetHeight() < -4`. Forced negative-height or bridge-mismatch states
remain conditional/nonstandard.

Rust bracket instances are currently enabled in `src/app_selection_brackets.rs` and
use a single hardcoded white `BRACKET_COLOR`; they do not implement the conditional
`GetHeight() < -4` palette `0x0C` branch.

Active in YR: N/A; Rust status only.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectClass::GetHeight @ 0x005F5F40` | verified | decompiled function | none |
| Building vtable `+0x1C8` binding | verified | memory `0x007E4084 = 0x005F5F40` | none |
| Bracket palette branch in `DrawBehind` | verified | `0x006F6109..0x006F6119` | none |
| Bracket palette branch in `DrawExtras` | verified | `0x006F53AD..0x006F53C0` | none |
| Normal placed building reachability | verified-with-inference | `Reveal`, `Set_Raw_Coords`, `GetCoords`, formula | exact placement-coordinate producer not expanded |
| Forced OnBridge building reachability | verified conditional | `GetHeight`, `SetHeight`, `ObjectClass::Unlimbo` bridge byte | full building-on-bridge placement legality not expanded |
| Limbo/conceal selected-drawn possibility | verified | `ObjectClass::Conceal @ 0x005F4D30` | none |
| Temporal/underground/sinking | touched-not-exhausted | bracket code has no such gates | full state machines out of scope |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What does `GetHeight` return? It returns `Location.Z - CellClass::GetGroundHeight(Location) - (OnBridge ? DAT_00AC13BC : 0)`. Evidence: `0x005F5F40`.

[RESOLVED] OQ-2 - Does BuildingClass override vtable `+0x1C8`? No. Building primary vtable entry `0x007E4084` points to `0x005F5F40`. Evidence: Ghidra memory read.

[RESOLVED] OQ-3 - What exact bracket condition selects palette `0x0C`? `GetHeight() < -4`; otherwise palette `0x0F`. Evidence: `0x006F6114..0x006F6119`, `0x006F53BB..0x006F53C0`.

[RESOLVED] OQ-4 - Can ordinary selected buildings hit the dim path? No evidence for standard placed buildings; with `Location.Z == ground_z` and `OnBridge==0`, result is `0`. Evidence: `0x005F5F40`, `0x005F4EC0`, `0x005F6940`, `0x00447AC0`.

[RESOLVED] OQ-5 - Can a forced selected building hit the dim path? Yes, conditionally, if `OnBridge=1` and Z was not bridge-raised. Evidence: `0x005F5F40`, `0x005F5FA0`, `0x005F5940`.

[DEFERRED] OQ-6 - Exact runtime initializer and final value of `DAT_00AC13BC`. Category: requires-different-system-context. Next step: focused bridge-height initializer audit.

## Sources

- Ghidra decompiled/read: `0x005F5F40`, `0x005F5FA0`, `0x006F60D0`, `0x006F5190`, `0x005F4EC0`, `0x005F6940`, `0x00447AC0`, `0x005F5940`, `0x005F4D30`, `0x005F4160`, vtable memory `0x007E4084`, `0x007E4088`.
- Prior docs referenced: `OBJECTCLASS_GHIDRA_REPORT.md`, `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`, `PARACHUTE_LANDING_BRIDGE_LAYER_SELECT_GHIDRA_REPORT.md`.
- Repo checked: `src/app_selection_brackets.rs`, `src/app_render/build_instances.rs`, `ini/rulesmd.ini`, `ini/rules.ini`.
