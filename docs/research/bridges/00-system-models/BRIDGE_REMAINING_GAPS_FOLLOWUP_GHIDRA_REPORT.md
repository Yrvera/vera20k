# Bridge Remaining Gaps Follow-up - Ghidra Research Report

**Address(es):** `0x00480A30`, `0x0047B3A0`, `0x00578080`, `0x00423AC0`, `0x004251F0`, `0x006E3410`, `0x0046B310`, `0x00483C80`, `0x0047D2B0`, `0x0057BAA0`, `0x0057BCF0`, `0x0057C2B0`, `0x006A05F0`, `0x0069F450`
**Investigation Mode:** coverage-map with bounded exhaustive slices
**Claimed Scope:** four bridge follow-up gaps from the bridge swarm: retail stock-map explicit `[Tubes]` prevalence; damaged/occupied low-bridge zone override details; Nuclear Missile final damaging Z and `CellClass::GetCenterCoords` bridge-Z internals; Ship locomotor high-bridge transition sequence.
**Non-Scope:** full nuke projectile ballistics, high-bridge repair/hut logic beyond low-bridge zone invalidation, runtime screenshot capture, and Rust implementation changes.
**Confidence:** HIGH for stock-map `[Tubes]` absence in the scanned retail install, `GetCenterCoords`, damaged low-bridge overlay/zone recalc, Ship transition writes, and `NukeGroundZero` non-damage. MEDIUM-HIGH for nuke final damage Z because the exact projectile terminal owner chain is broad, but the active damaging `AnimClass::AI` coordinate source is clear.
**Active in YR:** Yes. All named functions are live in standard YR; stock-map scan used the configured retail RA2/YR install.

## 1. Overview

The remaining bridge gaps resolve to four practical implementation rules:

1. Stock retail maps in this install do not author explicit nonzero `[Tubes]`. The low-bridge stock path must work from auto-created low-bridge shell/predicate data, not map-authored tube paths.
2. Low-bridge damage changes overlay state, recalculates the affected three-cell strip, and only rebuilds bridge zones on the second/full-destroy transition. Occupancy zone overrides come from normal `RecalcZoneType` object checks; deck occupants in `CellClass+0xE8` are not the low-bridge zone-type source.
3. `CellClass::Get_Center_Coords` returns cell-center X/Y and ground-only Z. Bridge-aware callers add bridge height explicitly after this call.
4. Ship high-bridge transition is no longer unchecked: `ShipLocomotionClass::Process_Drive_Track` mirrors Drive, writes `FootClass+0x8C` before the relayer/list-sensitive operations, and uses the Ship bridge-height global.

## 2. Retail Stock-map `[Tubes]` Prevalence

Scan target: `<ra2-install>/`.

Raw map-section scan:

| Source | `[Map]` sections | `[Tubes]` sections |
|---|---:|---:|
| `MAPS01.MIX` | 17 | 0 |
| `MAPS02.MIX` | 17 | 0 |
| `mapsmd03.mix` | 14 | 0 |
| `MULTI.MIX` | 97 | 0 |
| `multimd.mix` | 173 | 0 |
| `expandmd01.mix` | 13 | 0 |
| loose `.mmx/.yro/.map/.mpr/.yrm` | 54 | 0 |
| **Total** | **385** | **0** |

Additional full-install literal scan:

```text
rg -a -n -i "\[Tubes\]" "<ra2-install>"
```

Result: no matches.

Conclusion: retail stock-map prevalence of explicit nonzero low-bridge `[Tubes]` is **0 / 385 scanned map payloads** in this install. Any stock low-bridge movement/zone behavior that works in `gamemd.exe` cannot depend on map-authored `[Tubes]` records. It must rely on binary auto-created low-bridge tube shells plus final `LandType == 10` / bridge overlay state.

## 3. `CellClass::Get_Center_Coords` And Bridge Z

`CellClass__Get_Center_Coords @ 0x00480A30`:

```c
coord.X = cell.MapCoord_X * 0x100 + 0x80;
coord.Y = cell.MapCoord_Y * 0x100 + 0x80;
coord.Z = FUN_0047B3A0(center_coord); // same ground-height inner used by GetGroundHeight
```

Verified tiny details:

- X/Y are exact cell center leptons: `cell * 256 + 128`.
- Z comes from the ground-height inner helper, not from `cell.Flags & 0x100`.
- No bridge-height global is read by `Get_Center_Coords`.
- Callers that need deck Z add a bridge global after the center call. Examples rechecked here:
  - Genetic Converter in `SuperClass__Launch @ 0x006CC390`, case `9`, adds `DAT_00B0C07C` if the target cell has `Flags & 0x100`.
  - `House__LaunchNukeDown @ 0x006E3410` computes target ground Z, then adds `DAT_00B0E6D4` if the target cell has `Flags & 0x100`.

Implementation rule: do not make `cell_center_coords` bridge-aware by itself. Keep it ground-Z only and let bridge-aware call sites apply their own verified bridge addend.

## 4. Nuclear Missile Final Damaging Z

The active final object splash is not `NukeGroundZero__ApplyDamage`.

`NukeGroundZero__ApplyDamage @ 0x004251F0` calls:

```c
Apply_area_damage(0, Rules+0xF8C, 0, 0);
```

This hits `Apply_area_damage`'s zero-damage / null-warhead early-out pattern and is not the object-damaging nuke splash.

The damaging nuke anim path is in `AnimClass__AI @ 0x00423AC0`:

- The anim vtable `+0x48` coordinate getter is called into a local `CoordStruct`.
- If the anim type name compares equal to `RING1`, the warhead source is `Rules+0xFA8`.
- Otherwise the branch uses `Rules+0xF88`.
- `Apply_area_damage` is then called with that current anim coordinate.

Bridge-Z consequence:

- Final object damage uses the anim's current `CoordStruct.Z`.
- Because `Get_Center_Coords` and `GetGroundHeight` are ground-Z helpers, bridge-deck targeting only happens if the spawn/launch path explicitly added bridge height before the damaging anim coordinate was established.
- `House__LaunchNukeDown @ 0x006E3410` does add bridge height to the downward nuke bullet target for bridge cells.
- `NukeMaker__SpawnDownwardNuke @ 0x0046B310` seeds the downward bullet from the upward bullet's current coordinate; it does not independently call `Get_Center_Coords` or add bridge height in this function.

Implementation rule: model nuke final object-layer selection from the actual final anim/bullet impact coord. Do not route nuke through `NukeGroundZero__ApplyDamage`, and do not assume `Get_Center_Coords` supplies deck Z.

## 5. Damaged And Occupied Low-bridge Zone Overrides

### 5.1 Zone type recomputation

`CellClass__RecalcZoneType @ 0x00483C80` writes `CellClass+0x4C` in this order:

| Condition | Zone type effect |
|---|---|
| outside playfield | `7` |
| overlay crate/road-like flag `+0x22D` | `1` |
| overlay wall flag `+0x2A8` | `2` |
| overlay land speed row at impassable threshold | `6` |
| overlay gate flag `+0x2B5` | `6` |
| final `LandType == 2` | `4` water |
| final `LandType == 6` | `3` beach |
| final land speed row <= `0.01` | `6` impassable |
| blocking building / special building conditions | `5` building or `2` wall-like |
| default | `0` ground |

For an unobstructed low bridge cell, `LandType == 10` does not itself force an impassable/water zone. It reaches default ground zone type unless overlay/object checks override it.

### 5.2 Occupancy override

The object occupancy branch in `RecalcZoneType` iterates the normal cell object list after terrain/overlay checks. The bridge/deck list used by AoE and high-bridge occupancy (`CellClass+0xE8`) is not a separate low-bridge zone-type source in this function.

Practical effect:

- Ground-list occupants/buildings can override zone type through the normal object/building checks.
- Deck-list high-bridge occupants should not be used to classify the underlying low-bridge/tunnel zone.
- Low-bridge traversal remains primarily terrain/tube/overlay driven; transient object blocking belongs in the normal `CanEnterCell` / A* occupancy checks, not as a replacement for low-bridge zone connectivity.

### 5.3 Damaged low bridge overlays

Low bridge damage is a two-step overlay state machine:

- Healthy main low-bridge triplets become damaged overlays:
  - NS main `0x4A..0x4F -> 0x50`
  - EW main `0x53..0x58 -> 0x59`
- Damaged main triplets become destroyed-anchor overlays:
  - NS damaged `0x50..0x52 -> 0x64`
  - EW damaged `0x59..0x5B -> 0x65`
- Bridgehead overlays also have damaged forms:
  - NS `0x5C -> 0x5D`, `0x5E -> 0x5F`
  - EW `0x60 -> 0x61`, `0x62 -> 0x63`

`MapClass__DestroyBridgeWalker_NS_Low @ 0x0057BCF0` and `MapClass__DestroyBridgeWalker_EW_Low @ 0x0057C2B0` both recalc attributes for exactly the three affected cells after writing the overlays:

```text
CellClass__RecalcAttributes(this);
CellClass__RecalcAttributes(neighbor_a);
CellClass__RecalcAttributes(neighbor_b);
```

They call `MapClass__UpdateBridgeZonesHelper` only when the full-destroy flag is set. A first hit that merely changes healthy -> damaged recalculates terrain/zone bytes for the three cells but does not rebuild bridge-zone graph connectivity as a full destroyed span.

Implementation rule: damage overlay state can change zone classification through `RecalcAttributes/RecalcZoneType`; full bridge-zone connectivity invalidation belongs to the destroyed-anchor transition, not every damaged overlay write.

## 6. Ship Locomotor High-bridge Transition Sequence

`ShipLocomotionClass__Process_Drive_Track @ 0x006A05F0` was rechecked directly. It mirrors Drive's high-bridge transition sites with Ship globals:

| Behavior | Ship site | Verified effect |
|---|---:|---|
| destination bridge Z bump | `FUN_0069F450` | if destination cell `Flags & 0x100`, add `g_BridgeZ_Offset` |
| approach Z recompute | `0x006A06B7` region | target ground height plus `g_BridgeZ_Offset` when target cell has bridge flag |
| transition onto deck | `0x006A0EBE` region | writes `FootClass+0x8C = 1` |
| transition off deck / under | `0x006A0EE0` region | writes `FootClass+0x8C = 0` when old cell is bridge and new is not deck |
| blocker list selection | `0x006A0F58` region | uses `ground + g_BridgeZ_Offset` or `FootClass+0x8C` to select `CellClass+0xE4` vs `+0xE8` |
| scatter height layer | `0x006A3Axx` region | bridge scatter flag set only when current height differs from cell level by at least 3 height units |

Sequence detail: in the track-step branch, Ship writes `FootClass+0x8C` immediately after moving the object coordinate via vtable `+0x1B4`, then uses the updated flag for the subsequent bridge/list-sensitive logic. This matches the earlier Drive/Walk transition conclusion: removal/list operations see the previous state before coordinate movement; insertion/subsequent checks see the post-transition `OnBridge`.

Implementation rule: Ship is not a special exception to the high-bridge transition order. Use the same state sequencing as Drive, with Ship's own bridge-height global and water/ship movement checks.

## 7. Current Rust Status / Deltas

Observed surfaces:

- `src/map/tubes.rs` parses explicit `[Tubes]`, but retail stock maps scanned here contain none.
- `src/map/resolved_terrain.rs` builds auto low-bridge tube facts and tests `LandType == 10` plus tube index.
- `src/sim/pathfinding/zone_build.rs` has separate all-active vs high-active bridge record filters.
- `src/sim/combat/mod.rs` has `cell_center_coords` as center X/Y plus terrain Z.
- `src/sim/combat/combat_aoe.rs` already has an impact-Z layer selector compatible with the binary's `Apply_area_damage` selector.
- `src/sim/movement/locomotor.rs` currently treats Ship as ground movement family for layer purposes; this needs caution when implementing high-bridge OnBridge transitions and water/under-bridge behavior.

Required follow-up implementation effects:

| Finding | Rust-facing effect |
|---|---|
| Retail `[Tubes]` absent | Stock low-bridge behavior must not require explicit nonzero map tube steps. Auto low-bridge shells must remain enough for zone/predicate behavior. |
| `Get_Center_Coords` ground-Z only | Keep generic cell-center helpers ground-only; add bridge height at verified callers only. |
| Nuke final damage uses anim coord | When implementing nuke damage, thread the final impact coord into AoE layer selection; do not use `NukeGroundZero` as object splash. |
| Low bridge damage recalc | Recalc the three affected cells after low-bridge overlay transitions; rebuild bridge zones only on destroyed-anchor/full-destroy. |
| Occupancy override | Do not classify low-bridge zone type from bridge/deck occupants. Use object blocking separately from zone graph classification. |
| Ship transition | Add Ship-specific high-bridge transition tests mirroring Drive: old layer removal, new `OnBridge` write, selected occupancy list, and Ship Z offset. |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Retail map `[Tubes]` prevalence | verified | 385 `[Map]` sections, zero `[Tubes]` sections; full-install `rg` no matches | scan another retail language/install only if parity target changes |
| `CellClass__Get_Center_Coords` | verified | `0x00480A30` | none |
| `GetGroundHeight` bridge role | verified by caller contrast | `0x00578080`, `0x0047B3A0`, callers explicitly add bridge height | full slope interpolation internals not expanded here |
| `NukeGroundZero__ApplyDamage` | verified non-damaging | `0x004251F0` zero-damage call | none |
| Nuke final object splash coord | verified to active anim coord | `AnimClass__AI @ 0x00423AC0`; `House__LaunchNukeDown`; `NukeMaker__SpawnDownwardNuke` | full projectile terminal frame trace if exact visual tick is needed |
| Low-bridge damaged overlay transitions | verified | `DestroyBridge_Low`, `0x0057BCF0`, `0x0057C2B0` | bridgehead/hut variants outside this focused gap |
| Low-bridge occupied zone override | verified shape | `RecalcZoneType @ 0x00483C80` | exact semantic names of some object flags remain inferred from prior docs |
| Ship high-bridge transition | verified | `ShipLocomotionClass__Process_Drive_Track @ 0x006A05F0`, `FUN_0069F450` | runtime visual capture not done |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-1 - Do stock retail maps provide explicit nonzero low-bridge [Tubes]? -> No; zero [Tubes] sections found in 385 scanned map payloads/sections.`
- `[RESOLVED] OQ-2 - Does CellClass::GetCenterCoords include bridge deck Z? -> No; it returns ground Z, callers add bridge height explicitly.`
- `[RESOLVED] OQ-3 - Is NukeGroundZero the final damaging object splash? -> No; its zero-damage call is non-damaging.`
- `[RESOLVED] OQ-4 - What Z does nuke object splash use? -> The active damaging path uses the current anim/bullet impact coordinate; bridge layer selection depends on that coord's Z.`
- `[RESOLVED] OQ-5 - Do damaged low bridges immediately rebuild bridge-zone connectivity? -> Only on the full destroyed-anchor transition; first damaged overlay transition recalculates the affected cells but does not call UpdateBridgeZonesHelper.`
- `[RESOLVED] OQ-6 - Do occupied deck objects override low-bridge zone type? -> No direct deck-list zone override was found; RecalcZoneType uses the normal object/building checks, not CellClass+0xE8 as a separate low-bridge source.`
- `[RESOLVED] OQ-7 - Was Ship locomotor high-bridge transition rechecked? -> Yes; it mirrors Drive's transition writes and list selection with Ship globals.`
- `[DEFERRED] OQ-8 - Exact nuke projectile terminal frame/tick visual order.` Category: `requires-broader-projectile-trace`; not needed for the object-layer Z rule closed here.

## Sources

- Ghidra decompiled/rechecked:
  - `CellClass__Get_Center_Coords @ 0x00480A30`
  - `CellClass__GetGroundHeight @ 0x00578080`
  - `FUN_0047B3A0`
  - `AnimClass__AI @ 0x00423AC0`
  - `NukeGroundZero__ApplyDamage @ 0x004251F0`
  - `House__LaunchNukeDown @ 0x006E3410`
  - `NukeMaker__SpawnDownwardNuke @ 0x0046B310`
  - `CellClass__RecalcZoneType @ 0x00483C80`
  - `CellClass__RecalcAttributes @ 0x0047D2B0`
  - `DestroyBridge_Low @ 0x0057BAA0`
  - `MapClass__DestroyBridgeFromCell_Low @ 0x00574780`
  - `MapClass__DestroyBridgeWalker_NS_Low @ 0x0057BCF0`
  - `MapClass__DestroyBridgeWalker_EW_Low @ 0x0057C2B0`
  - `MapClass__UpdateBridgeZonesHelper @ 0x0056C510`
  - `ShipLocomotionClass__Process_Drive_Track @ 0x006A05F0`
  - `ShipLocomotionClass__Apply_Track_Step @ 0x006A01A0`
  - `ShipLocomotionClass__Process @ 0x0069FC10`
  - `FUN_0069F450`
- Local data scanned:
  - `<ra2-install>/MAPS01.MIX`
  - `<ra2-install>/MAPS02.MIX`
  - `<ra2-install>/mapsmd03.mix`
  - `<ra2-install>/MULTI.MIX`
  - `<ra2-install>/multimd.mix`
  - `<ra2-install>/expandmd01.mix`
  - loose `.mmx/.yro/.map/.mpr/.yrm`
- Docs consulted:
  - `docs/research/LOW_BRIDGE_ZONE_PRECHECK_LANDTYPE10_CONNECTIVITY_GHIDRA_REPORT.md`
  - `docs/research/LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md`
  - `docs/research/SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_GHIDRA_REPORT.md`
  - `docs/research/BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`
  - `docs/research/BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md`
- Rust surfaces scanned:
  - `src/map/tubes.rs`
  - `src/map/resolved_terrain.rs`
  - `src/sim/pathfinding/zone_build.rs`
  - `src/sim/combat/mod.rs`
  - `src/sim/combat/combat_aoe.rs`
  - `src/sim/movement/locomotor.rs`
