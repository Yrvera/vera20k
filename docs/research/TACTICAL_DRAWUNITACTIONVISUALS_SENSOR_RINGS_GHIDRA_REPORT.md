# Tactical::DrawUnitActionVisuals Sensor Rings - Ghidra Research Report

**Address(es):** `0x006DBE20` primary; `0x004566B0` range helper; `0x00456980` ring renderer; `0x006612C0` color fade helper; `0x0041BE80` empty virtual hook  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** selected-building sensor/gap/psychic radius rings in `Tactical::DrawUnitActionVisuals`, the `TypeClass+0x238` / `vtable+0x130` dispatch in the same pass, and whether this pass is brackets/health-pips.  
**Non-Scope:** full target/action-line rendering, radar minimap action lines, cell-counter add/remove mechanics, full parser reconstruction for `TypeClass+0x238`, and exact alpha-surface raster internals beyond the ring helper's visible call shape.  
**Confidence:** High for primary control flow, selected-building gates, range priority, active INI triggers, and separation from brackets/health pips. Medium for `TypeClass+0x238` semantics because this slot verified the dispatch and base stub but did not drain its parser origin.  
**Active in YR:** Yes for selected-building radius rings when the global display flag is enabled and a selected building reports positive `GetSensorRange()`. Conditional/no-op for the generic `vtable+0x130` hook unless a concrete class overrides the empty base stub.

## 1. Overview

`Tactical::DrawUnitActionVisuals @ 0x006DBE20` is not the normal selected-building bracket or health-pip renderer. The normal building brackets live in `TechnoClass::DrawBehind @ 0x006F60D0` and `TechnoClass::DrawExtras @ 0x006F5190`; this pass runs later as a tactical overlay pass for per-object radial hooks and selected-building radius circles.

For selected buildings, the visible output is a pulsing house-color ellipse/circle centered on the selected map cell. The radius comes from `BuildingClass::GetSensorRange @ 0x004566B0`, which prioritizes `PsychicDetectionRadius`, then gap-generator radii, then cloak/sensor-array radius, then a superweapon range fallback.

## 2. Key Offsets

| Offset / global | Meaning in this slice | Active in YR | Evidence |
|---|---|---:|---|
| `ObjectTypeClass/TypeClass+0x238` | Nonzero gate for generic `obj.vtable+0x130()` dispatch over `g_CurrentObjects` | Conditional | `0x006DBE20` first loop |
| object vtable `+0x130` | Per-object radial/action visual hook; base implementation is empty | Conditional/no-op on base | `0x006DBE20`; `0x0041BE80` returns immediately |
| `DAT_00A8EB7E` | Global gate before selected-building/superweapon radius drawing block | Conditional | `0x006DBE20` branch before selected object logic |
| `DAT_0088098C` | Current selected object pointer used by the building branch | Yes | `0x006DBE20` reads and RTTI-checks it |
| selected object vtable `+0x2C` | `WhatAmI`; building branch requires return `6` | Yes | `0x006DBE20`; prior bracket reports verify `BuildingClass::WhatAmI @ 0x00459EC0 == 6` |
| `BuildingClass+0x520` | `BuildingTypeClass*` used by `GetSensorRange` | Yes | `0x004566B0` |
| `BuildingTypeClass+0x170C` | `PsychicDetectionRadius=`; first priority positive range | Yes | `0x004566B0`; `ini/rulesmd.ini:13353` |
| `TechnoTypeClass+0xCD1` | `GapGenerator=` flag | Yes | `0x004566B0`; `ini/rulesmd.ini:12226` |
| `TechnoTypeClass+0xCD2` | `GapRadiusInCells=` normal gap radius | Yes | `0x004566B0`; `ini/rulesmd.ini:12227` |
| `TechnoTypeClass+0xCD3` | `SuperGapRadiusInCells=` active when `BuildingClass+0x268 != 0` | Conditional | `0x004566B0`; `ini/rulesmd.ini:12228` |
| `BuildingTypeClass+0x16C8` | `SensorArray=` flag | Yes | `0x004566B0`; `ini/rulesmd.ini:13375` |
| `BuildingTypeClass+0x16C7` | `CloakGenerator=` flag; also selects cloak-radius display fallback | No for retail YR standard buildings | `0x004566B0`; prior cloak/sensor report says no retail YR building uses it |
| `BuildingTypeClass+0x1707` | cloak/sensor-array display fallback radius | Conditional | `0x004566B0` |

## 3. Core Logic

The first loop walks `g_CurrentObjects`. For each object, it calls virtual `+0x88` to obtain its type. If the type pointer is non-null and byte `type+0x238` is nonzero, it calls the object's `vtable+0x130`. The base target at `0x0041BE80` is an empty return. Active in YR: conditional; the dispatch is live, but base behavior draws nothing.

The selected-building branch runs only if `DAT_00A8EB7E != 0`, there is a selected object, selected object's `WhatAmI()` returns `6`, and `BuildingClass::GetSensorRange()` returns a positive integer. Active in YR: conditional; buildings such as `[GAGAP]` and `[NAPSIS]` can satisfy the positive range condition.

When active, the branch:

1. Converts the selected map cell to cell-center world coordinates through `MapClass::Get_CellClass` and `CellClass::Get_Center_Coords`.
2. Lazily initializes the pulse color globals if bit `1` of `DAT_00B0CE90` is not set.
3. Advances `DAT_00B0E648` by `DAT_00842950 * 10`, bouncing direction at `< 1` and `>= 100`.
4. Reads the owner house's 3-byte color at `owner+0x56F9`.
5. Calls `FUN_006612C0` to interpolate a 3-byte pulse color.
6. Calls `FUN_00456980(center, color, radius_as_float, 0, 1)` to draw the radius ellipse/circle on `g_PrimarySurface`.
7. Scans `g_PlayerPtr+0x6C/+0x78` object list twice for live same-owner peers with the same `+0x520`/`+0x148` grouping as the selected building; first pass finds the nearest one within `radius * 2`, second pass calls that peer's `vtable+0x130` if its distance equals the nearest distance.

The distance calculation converts world `x/y` to cell coordinates with signed `/256` truncation using `(value + (value >> 31 & 0xFF)) >> 8`, then computes approximate Euclidean distance through `Sqrt_Approx` and `Math__ftol`. The comparison is `distance <= radius * 2` and then strict nearest `distance < current_min`; the second pass draws all peers at exactly the nearest distance. Active in YR: conditional; requires selected building radius display and matching live peer objects.

`FUN_00456980` draws via `g_PrimarySurface` slots `+0x20` and `+0x3C` after converting world coordinates to client coordinates and clipping to the radar/tactical viewport rectangle. Despite prior wording in some docs, this is not the building wireframe bracket path and not the health-pip path.

## 4. Range Priority

`BuildingClass::GetSensorRange @ 0x004566B0` returns:

1. `BuildingTypeClass+0x170C` if `PsychicDetectionRadius > 0`.
2. If `GapGenerator` is true: `SuperGapRadiusInCells` when `building+0x268 != 0`, otherwise `GapRadiusInCells`.
3. If `SensorArray` or `CloakGenerator` is true: signed byte `BuildingTypeClass+0x1707`.
4. If a superweapon helper pair returns active and the superweapon type has positive `+0xB4` range-like value, return that value divided by 256 with signed truncation.
5. Otherwise `0`.

Important consequence: retail `[NAPSIS]` has `PsychicDetectionRadius=15`, so its selected ring uses 15 and does not fall through to `SensorArray`/`SensorsSight`. Retail `[GAGAP]` has `GapGenerator=yes`, `GapRadiusInCells=10`, and `SuperGapRadiusInCells=10`, so its selected ring uses 10. `SensorsSight=15` on `[NAPSIS]` is important for sensor cell counters, but it is not the range returned by this function because the psychic radius wins first.

## 5. INI Keys

| Section | Key(s) | Visible ring effect in this pass | Active in YR |
|---|---|---|---|
| `[NAPSIS]` | `PsychicDetectionRadius=15` | selected Psychic Sensor draws radius 15 | Yes, `ini/rulesmd.ini:13342`, `:13353` |
| `[NAPSIS]` | `SensorArray=yes`, `SensorsSight=15` | sensor cell-counter system; selected ring still uses psychic radius first | Yes, `ini/rulesmd.ini:13375-13376` |
| `[NAPSIS]` | `DetectDisguise=yes`, `DetectDisguiseRange=15` | disguise detection counters, not this ring's range source | Yes, `ini/rulesmd.ini:13372-13373` |
| `[GAGAP]` | `GapGenerator=yes`, `GapRadiusInCells=10`, `SuperGapRadiusInCells=10` | selected Gap Generator draws radius 10 or super-gap radius if `building+0x268` set | Yes, `ini/rulesmd.ini:12221-12228` |
| unit/naval examples | `Sensors=yes`, `SensorsSight=7/8` | not selected-building ring path; unit sensor detection is separate | Yes for detection, No for this building branch; e.g. `ini/rulesmd.ini:8016-8017`, `:8953-8954` |
| `[GASPYSAT]` | `Radar=yes`, `SpySat=yes` | no positive `GetSensorRange()` from these keys alone in verified helper | No for this ring unless another range source exists; `ini/rulesmd.ini:12187-12195` |

## 6. Integration Points

Prior render-order docs place `Tactical::DrawUnitActionVisuals @ 0x006DBE20` after the main object rendering and separate from `TechnoClass::DrawBehind` / `DrawExtras`. This report confirms that its selected-building logic is radius drawing, not the bracket or health-pip pass.

Target/action lines remain separate. Prior `TARGET_LINES_GHIDRA_REPORT.md` identifies `TechnoClass::DrawActionLines @ 0x004DC060` and a distinct call site in `TacticalClass::Draw`; this slot did not re-investigate those line renderers.

## 7. Current Rust Implementation Status

Rust parses and simulates some related systems, but not this selected-building radius overlay:

| Rust area | Status | Evidence |
|---|---|---|
| Gap generator fog suppression | implemented at sim/fog level | `src/sim/world/mod.rs:851-881`, `src/sim/vision/mod.rs:787` |
| Gap generator INI fields | parsed on object type | `src/rules/object_type.rs:311`, `:888`, `:1120-1124` |
| PsychicReveal superweapon radius | implemented as reveal, not selected-building ring | `src/sim/superweapon/psychic_reveal.rs:15-27` |
| Selected-building sensor/gap/psychic radius ring overlay | not found | source scan for `DrawUnitActionVisuals`, sensor/psychic ring overlay, and selected-building radius rendering returned no implementation |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Tactical::DrawUnitActionVisuals @ 0x006DBE20` primary pass | verified | Fresh Ghidra decompile | none for scoped branches |
| Generic `TypeClass+0x238` / `vtable+0x130` dispatch | touched-not-exhausted | `0x006DBE20`, `0x0041BE80` | parser/name for `+0x238` and override census |
| Base `vtable+0x130` implementation | verified | `0x0041BE80` empty return | none |
| Selected-building gate | verified | `0x006DBE20`, `WhatAmI()==6` requirement | none |
| `BuildingClass::GetSensorRange` priority | verified | `0x004566B0` | superweapon fallback semantic name at `+0xB4` deferred |
| Ring drawing helper | touched-not-exhausted | `0x00456980` | exact surface slot names/raster details out of scope |
| Pulse color interpolation | verified | `0x006612C0` signed-truncating per-channel interpolation | none |
| Retail `[NAPSIS]` trigger | verified | `0x004566B0`; `ini/rulesmd.ini:13353` | none |
| Retail `[GAGAP]` trigger | verified | `0x004566B0`; `ini/rulesmd.ini:12226-12228` | none |
| Health pips and wireframe brackets | verified as non-scope/separate | prior reports `TECHNO_DRAWEXTRAS...`, `TECHNO_DRAWBEHIND...`; primary function lacks those calls | none |

## 9. Open Questions - Final State

[RESOLVED] OQ-DUAV-001 - Is `0x006DBE20` the normal bracket/health-pip pass? No. The selected-building branch draws radius rings, while brackets/pips are in `TechnoClass::DrawBehind` and `DrawExtras`. Evidence: `0x006DBE20`, prior reports at `0x006F60D0` and `0x006F5190`. Active in YR: Yes, as a separate pass.

[RESOLVED] OQ-DUAV-002 - What gates selected-building radius rings? `DAT_00A8EB7E != 0`, selected object pointer non-null, selected object's `WhatAmI()==6`, and `GetSensorRange() > 0`. Evidence: `0x006DBE20`. Active in YR: Conditional.

[RESOLVED] OQ-DUAV-003 - Which retail YR buildings visibly trigger the verified selected-building range source? `[NAPSIS]` via `PsychicDetectionRadius=15` and `[GAGAP]` via `GapGenerator=yes` plus radius keys. Evidence: `0x004566B0`, `ini/rulesmd.ini:13353`, `:12226-12228`. Active in YR: Yes.

[RESOLVED] OQ-DUAV-004 - Does `SensorsSight=15` drive the Psychic Sensor's displayed selected radius? No for `[NAPSIS]`: `PsychicDetectionRadius=15` has first priority and returns before `SensorArray` fallback. Evidence: `0x004566B0`; `ini/rulesmd.ini:13353`, `:13375-13376`. Active in YR: Yes.

[RESOLVED] OQ-DUAV-005 - Is the `vtable+0x130` base hook visible by itself? No; `0x0041BE80` returns immediately. Evidence: Ghidra decompile `0x0041BE80`. Active in YR: No for base implementation; conditional for overrides.

[DEFERRED] OQ-DUAV-006 - What exact INI parser name writes `TypeClass+0x238`? Category: out-of-scope. Reason: this slot was bounded to the render pass; the primary pass and base hook are verified, but parser-origin and concrete override census require a separate TypeClass/vtable investigation.

[DEFERRED] OQ-DUAV-007 - Exact semantic name for the superweapon helper fallback returned from `FUN_0070e1a0` / `FUN_0070e240` and `type+0xB4`. Category: out-of-scope. Reason: fallback is after standard `[NAPSIS]` and `[GAGAP]` range sources and needs a separate superweapon-type range report.

## Sources

- Ghidra decompile: `Tactical::DrawUnitActionVisuals @ 0x006DBE20`
- Ghidra decompile: `BuildingClass::GetSensorRange @ 0x004566B0`
- Ghidra decompile: empty hook `0x0041BE80`
- Ghidra decompile: ring helper `0x00456980`
- Ghidra decompile: color interpolation helper `0x006612C0`
- `ini/rulesmd.ini:12221-12228` (`[GAGAP]`)
- `ini/rulesmd.ini:13342-13376` (`[NAPSIS]`)
- `ini/rulesmd.ini:12187-12195` (`[GASPYSAT]`)
- Prior research: `building-selection-brackets/SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md`
- Prior research: `building-selection-brackets/TECHNO_DRAWEXTRAS_BUILDING_BRACKET_BLOCK_GHIDRA_REPORT.md`
- Prior research: `building-selection-brackets/TECHNO_DRAWBEHIND_BUILDING_BRACKET_EDGES_GHIDRA_REPORT.md`
- Prior research: `TARGET_LINES_GHIDRA_REPORT.md`
