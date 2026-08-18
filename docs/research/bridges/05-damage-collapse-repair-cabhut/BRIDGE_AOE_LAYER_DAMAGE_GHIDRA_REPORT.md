# Bridge AoE Layer Damage - Ghidra Research Report

**Primary address:** `0x00489280` (`Apply_area_damage`)
**Related addresses:** `0x004690B0` (`WarheadTypeClass__Detonate`), `0x00587180` (`ApplyDamageToCell`), `0x00489180` (`WarheadTypeClass__GetDamage`, referenced via prior damage report)
**Confidence:** HIGH for the layer selector, object-list iteration, bridge tile-damage gate, and Rust disparity. MEDIUM for exact semantic names of two scenario flags because this pass verified their use but did not re-trace their INI/UI writers.
**Active in YR:** Yes. Standard YR combat and superweapon damage route through `Apply_area_damage`; bridge tile damage is conditional on scenario flag `0x8000`. Retail rules have `DestroyableBridges=yes`, but this pass verified the flag use, not the INI writer that maps the key to the flag.

## 1. Overview

`Apply_area_damage` does two bridge-relevant jobs for every CellSpread detonation:

1. It damages objects from exactly one per-cell object list: either the ground list at `CellClass+0xE4` or the bridge/deck list at `CellClass+0xE8`.
2. Independently, it may damage bridge tiles/cells through `ApplyDamageToCell`, `DestroyBridge_Low`, or `DestroyBridge_High` if the warhead has `Wall=yes` and the bridge-destruction scenario flag is enabled.

The important player-visible result is that a single splash detonation should not normally damage both a tank on the bridge deck and another unit standing underneath in the same XY cells. The binary selects one layer for object damage. Bridge tile damage is a separate impact-cell tile/overlay effect and does not imply both occupant layers are damaged.

## 2. Key Offsets And Globals

| Address / Offset | Type | Meaning | Evidence |
|---|---:|---|---|
| `CellClass+0x140 & 0x100` | bit | Structural bridge cell test used before selecting deck list | `Apply_area_damage` at `0x00489562-0x00489568` |
| `CellClass+0xE4` | ptr | Ground object linked-list head | selected at `0x004896CF` |
| `CellClass+0xE8` | ptr | Bridge/deck object linked-list head | selected at `0x004896C7` |
| object `+0x30` | ptr | Next object in selected cell list | `Apply_area_damage` advances at `0x004899B3` |
| `WarheadType+0x124` | float | `CellSpread` | `Apply_area_damage` at `0x004892DD`, prior `DAMAGE_MATH_GHIDRA_REPORT.md` |
| `WarheadType+0x144` | bool | `Wall=yes`; gates bridge tile damage | `0x00489FC5`, `0x0048A14A`; also docs/INI |
| `Rules+0xFF0` | ptr | `IonCannonWarhead`; bypasses BridgeStrength RNG and enables retry behavior | `0x00489FD8`, `0x0048A15D`, `0x0048A229`, `0x0048A283` |
| `Rules+0x1740` | int | `BridgeStrength=` random upper bound | `0x00489FE0`, `0x0048A165`, `0x0048A231`, `0x0048A28B` |
| `DAT_0089E864` | int | Bridge-height offset used in the splash layer threshold | `0x0048957A-0x00489584` |
| `DAT_0089E870` | int | terrain level-height multiplier used in bridge tile Z gate | `0x00489F90`, `0x0048A114` |
| `DAT_007ED3D0[]` | int[] | cell-count table by integer `CellSpread` | `0x004895A3` |
| `DAT_00ABD490/492` | i16[] | CellSpread X/Y offset table | `0x004895C7-0x004895DC` |
| scenario flag `0x20` | bit | no-damage early out | `0x004892C4-0x004892CC` |
| scenario flag `0x800` | bit | `ProtectedFromAOE` filter active | object filter path around `0x00489702` |
| scenario flag `0x8000` | bit | bridge tile-damage/destruction enabled | `Apply_area_damage` decomp branch before `0x00489F27` |

## 3. Layer Selection

### 3.1 Selector is computed once from the impact cell

At the start of `Apply_area_damage`, the impact lepton coordinate is converted to cell coordinates using signed rounding toward the containing 256-lepton cell:

```text
cell_x = (impact_x + sign_adjust_255) >> 8
cell_y = (impact_y + sign_adjust_255) >> 8
impact_cell = MapClass::Get_CellClass(cell_x, cell_y)
```

Then the bridge-object-list selector byte at stack offset `ESP+0x1E` is initialized to zero and may be set to one:

```text
if (impact_cell.Flags & 0x100) != 0:
    ground_z = CellClass::GetGroundHeight(impact_coord)
    threshold_z = ground_z + (DAT_0089E864 / 2)
    if impact_z > threshold_z:
        use_bridge_list = 1
```

Evidence:

- `0x00489357` initializes `byte ptr [ESP+0x1E]` to `0`.
- `0x00489562` reads `impact_cell+0x140`.
- `0x00489568` tests `CH,0x1`, which is `0x100` in the flags dword.
- `0x00489573` calls `CellClass::GetGroundHeight`.
- `0x0048957A-0x00489584` computes `(DAT_0089E864 + sign) / 2` and adds it to ground height.
- `0x00489586-0x0048958D` compares `impact_z` with the threshold and writes `1` to `[ESP+0x1E]` only on strict greater-than.

Tiny detail: the comparison is strict. An explosion exactly at `ground_z + bridge_height / 2` stays on the ground list. Only `impact_z > threshold` selects the bridge list.

### 3.2 The same selector is used for every affected cell

Inside the CellSpread loop, after per-cell overlay effects run, the object list is selected like this:

```text
if use_bridge_list:
    obj = cell+0xE8
else:
    obj = cell+0xE4

while obj != null:
    ...
    obj = obj+0x30
```

Evidence:

- `0x004896BF` loads `[ESP+0x1E]`.
- `0x004896C3-0x004896C7`: nonzero selector loads `EBX+0xE8`.
- `0x004896CF`: zero selector loads `EBX+0xE4`.
- `0x004899B3`: next object is read from `ESI+0x30`.

This is not recalculated per affected cell. A detonation on or above a bridge cell can make the whole CellSpread scan read deck lists from all scanned cells. A detonation on a non-bridge impact cell reads ground lists from all scanned cells, even if some affected cells are bridge cells.

### 3.3 Object damage is collected first, then applied

`Apply_area_damage` appends `{object, distance}` pairs into an internal vector during the cell scan. After all cells are scanned, it loops the vector and calls `obj->ReceiveDamage` via vtable slot `+0x16C`.

Evidence:

- `operator_new(8)` allocates one `{object, distance}` record around `0x004897xx`.
- `0x004899A8-0x004899B0` appends the record into the vector.
- final dispatch calls `vtable+0x16C` in the decompilation after the scan.

Ordering matters: per-cell overlay side effects and object-list collection happen before the final damage dispatch. Targets destroyed earlier in the final vector can still affect later side effects through normal `ReceiveDamage` behavior, but the target set was already chosen.

## 4. Bridge-Tolerance Gate

The function also has a bridge-object tolerance flag (`bVar5` in Ghidra decompilation). It is enabled when a bridge-layer object is detected very close to the impact:

```text
bridge_tolerance_enabled = wh.CellSpread > 0.5

if bridge_tolerance_enabled
   and current scanned object is on the bridge
   and object is not in limbo
   and distance < 0x55:
       bVar5 = true
```

Evidence:

- `0x00489347-0x00489372` compares `wh+0x124` (`CellSpread`) with `DAT_007E5168`, matching the existing `0.5f` threshold.
- `0x0048947F-0x004894A4` sets the flag for airborne/above-ground objects within `0x55` if vtable `+0x160` returns true and object state checks pass.
- `0x004899xx` repeats the check for the impact cell (`cell offset index == 0`) in the ground/deck cell loop.
- final dispatch uses this flag to require bridge/on-bridge status before applying damage when the flag is set.

This is separate from the list selector. The list selector decides which linked list is scanned; the tolerance gate can further suppress non-bridge objects in the final dispatch once a nearby bridge object has been detected.

Open nuance: existing docs describe this as "bridge infantry tolerance." This pass verified the bridge/on-bridge predicate but did not prove the target class is infantry-only at this site. The decompiled checks use object flags plus vtable `+0x160`, not an obvious `WhatAmI == Infantry` guard in the final dispatch.

## 5. Bridge Tile Damage Is Independent

Object splash damage and bridge tile damage are distinct phases.

After object damage and rocker/push handling, `Apply_area_damage` reacquires the impact cell from the original impact-cell coordinate storage and checks bridge tile damage if all of these are true:

```text
scenario_flags & 0x8000
warhead.Wall == true
target cell matches bridge-tile or bridge-overlay identity
warhead == Rules.IonCannonWarhead OR Random(1, BridgeStrength) < effective_damage
```

Evidence:

- `0x00489FC5` and `0x0048A14A` test `WarheadType+0x144`.
- `0x00489FD8`, `0x0048A15D`, `0x0048A229`, and `0x0048A283` compare the warhead pointer against `Rules+0xFF0`.
- `0x00489FE0`, `0x0048A165`, `0x0048A231`, and `0x0048A28B` read `Rules+0x1740`.
- `0x00489FF5`, `0x0048A179`, `0x0048A245`, and `0x0048A29F` call `Random__RandomRanged(1, BridgeStrength)`.
- the random result is compared with the stack damage/effective-damage value at `ESP+0x24`; `JGE` skips damage, so the condition is `random < damage`, not `<=`.

Scope detail verified by follow-up audit:

- `0x00489335-0x0048933D` initializes `[ESP+0x18]` from the original impact cell.
- The object CellSpread loop runs through `0x004895C0-0x004899D4`.
- After that loop, bridge tile damage reacquires a cell from `[ESP+0x18]` at `0x00489E8D-0x00489EA4`.
- The `ApplyDamageToCell` paths push `LEA [ESP+0x18]` at `0x0048A004-0x0048A00E` and `0x0048A188-0x0048A192`.

So bridge tile damage is not a per-scanned-CellSpread-cell effect in this function. The per-spread behavior applies to object-list collection and earlier overlay/tiberium side effects; the bridge tile damage phase uses the impact cell.

Tiny detail: non-Ion bridge damage uses a strict less-than comparison. If the random roll equals the effective damage, the bridge is not damaged.

### 5.1 High/low bridge detection paths

The function checks both tile-derived bridge identity and overlay-derived bridge identity:

- high bridge tile path around `0x00489F27-0x0048A0A5`;
- low bridge tile path around `0x0048A0A5-0x0048A214`;
- low bridge overlay range `0x4A..0x63` at `0x0048A214-0x0048A26A`;
- high bridge overlay range `0xCD..0xE6` at `0x0048A26A-0x0048A2C4`.

For bridge structural cells, the tile path also applies a Z window:

```text
if cell.Flags & 0x100:
    if impact_z > (cell.Level + 1) * LevelHeight + BridgeHeight: skip
    if impact_z <= (cell.Level - 2) * LevelHeight + BridgeHeight: skip
```

Evidence:

- high tile path uses `MOVSX byte ptr [EDI+0x11B]` at `0x00489F82`.
- low tile path uses the same signed level read at `0x0048A10D`.
- both paths compare the impact Z against `(Level + 1)` and `(Level - 2)` forms before bridge damage.

This Z gate is for bridge tile damage. It does not decide which units are damaged; object-list selection was already made earlier from the impact cell and `ground + BridgeHeight/2`.

### 5.2 Retry behavior

If bridge tile damage is attempted through `ApplyDamageToCell`, the caller retries up to three additional times only for the force warhead path.

Observed shape:

```text
success = ApplyDamageToCell(...)
tries_left = 3
while !success:
    if not force_warhead: break
    if tries_left <= 0: break
    success = ApplyDamageToCell(...)
    tries_left -= 1
```

Evidence:

- first call at `0x0048A00E` / `0x0048A192`;
- `ESI=3` after the first call;
- retry branch tests the force-warhead flag byte at `[ESP+0x0F]`;
- `DEC ESI` and loop while result remains false at `0x0048A02C-0x0048A03A` and `0x0048A1AC-0x0048A1BE`.

So the maximum is four attempts total: first call plus three retries. Non-force warheads do not retry after a failed `ApplyDamageToCell`.

## 6. `ApplyDamageToCell` Bridge Dispatch

`ApplyDamageToCell @ 0x00587180` is the tile-level bridge dispatcher called by the area-damage bridge phase.

Verified behavior:

- Overlay `0x4A..0x63` dispatches directly to `DestroyBridge_Low`.
- Overlay `0xCD..0xE6` dispatches directly to `DestroyBridge_High`.
- High bridge tile identity dispatches to `ProcessBridgeDamageStateMachine_High`.
- Low bridge tile identity dispatches to `ProcessBridgeDamageStateMachine_Low`.
- If the cell is structural (`Flags & 0x100`) but not an anchor (`Flags & 0x80` clear), it follows `CellClass+0x2C` back to the anchor cell before testing high-bridge overlay IDs `0x18/0x19`.

Evidence:

- `0x00587180` decompilation.
- direct overlay low/high checks at the start of `ApplyDamageToCell`.
- anchor-pointer fallback reads `cell+0x2C`, then `+0x24` coordinates, then anchor `+0x44`.

This confirms that the bridge tile-damage phase is bridge-topology aware independently of the object AoE layer selection.

## 7. INI Keys In Scope

| Key | Section | Retail YR default | Binary effect |
|---|---|---:|---|
| `DestroyableBridges` | `[CombatDamage]` | `yes` in `rulesmd.ini` line 804 | Associated with bridge destruction in retail rules; this pass verified the `0x8000` scenario flag gate, not the INI-to-flag writer |
| `BridgeStrength` | `[CombatDamage]` | `1500` in `rulesmd.ini` line 816 | Upper bound for `Random(1, BridgeStrength)` bridge tile-damage gate |
| `IonCannonWarhead` | `[CombatDamage]` | `IonCannonWH` in `rulesmd.ini` line 874 | Warhead pointer at `Rules+0xFF0`; bypasses BridgeStrength RNG |
| `C4Warhead` | `[CombatDamage]` | `Super` in `rulesmd.ini` line 818 | Separate C4/self-damage path in `Apply_area_damage`; relevant to recursive chain reactions, not normal bridge layer selection |
| `CellSpread` | `[Warhead]` | varies | Controls object damage radius and integer spread table index |
| `PercentAtMax` | `[Warhead]` | default `1.0`, varies | Downstream falloff in `GetDamage`, not the bridge selector itself |
| `Wall` | `[Warhead]` | varies | Required for bridge tile damage |

Normal bridge-layer object damage does not require `Wall=yes`; any CellSpread warhead can damage occupants on the selected layer. `Wall=yes` only gates terrain/overlay/bridge tile damage.

## 8. Current Rust Implementation Status

Rust has the pieces needed to represent layer-aware damage. A 2026-05-17 implementation pass wired the direct-fire and death-AoE combat paths through layer-aware object splash.

Current behavior after that pass:

- `src/sim/combat/combat_aoe.rs` accepts an `AoELayerContext` containing optional `OccupancyGrid`, `ResolvedTerrainGrid`, and impact Z. When both occupancy and terrain are present, it uses the gamemd-style single object layer; otherwise it preserves the old all-entity fallback.
- When occupancy is supplied, it selects exactly one ground/bridge occupant layer from the impact cell and scans only that selected layer through the CellSpread cells.
- Direct-fire and death-AoE paths in `src/sim/combat/mod.rs` pass occupancy, terrain, and impact Z into `combat_aoe`.
- Superweapon callers that do not yet have verified impact-Z construction still use the default context and keep the older all-entity behavior until their Z paths are traced.
- It still does not model the `CellSpread > 0.5` / `distance < 0x55` bridge-object tolerance gate.
- Bridge tile damage events are emitted separately in `src/sim/combat/mod.rs` for the impact cell in the current direct-fire/death-AoE paths, matching the audited bridge-tile scope.
- `src/sim/occupancy.rs` already stores layer-tagged occupants and exposes per-layer iteration, so the data model can represent the binary's selected-list behavior.

Player-visible gap:

The direct-fire and death-AoE bridge-layer double-hit case is now addressed for occupancy-backed combat calls. Superweapon-specific AoE remains open where the caller's impact Z has not been traced. Retail selects one object layer for splash object damage; any remaining all-entity superweapon path can still damage both layers until its Z construction is verified and threaded into `AoELayerContext`.

## 9. Implementation-Relevant Behavioral Rules

These are research conclusions, not a code plan:

1. Select object-damage layer once per detonation from the impact cell:
   - impact cell must be structural bridge (`Flags & 0x100`);
   - impact Z must be strictly greater than `ground_z + BridgeHeight / 2`;
   - otherwise use ground layer.
2. Use that selected layer for every CellSpread cell's object list.
3. Apply object damage by collecting all selected-layer targets first, then dispatching `ReceiveDamage` in collected order.
4. Do not use `Wall=yes` to decide object splash. `Wall=yes` only decides overlay/wall/bridge tile damage.
5. Bridge tile damage uses the impact cell in this function; it is not the same thing as object splash damage and does not imply both object layers are damaged.
6. Non-Ion bridge tile damage uses `Random(1, BridgeStrength) < effective_damage`; equality fails.
7. `IonCannonWarhead` bypasses the random gate and gets up to four `ApplyDamageToCell` attempts if previous attempts fail.

## 10. Open Questions

1. **Exact `DAT_0089E864` semantic name.** This pass verified it is the value halved for object-layer bridge selection. Existing docs call nearby bridge-height globals by several names. A future verify-doc pass should pin whether this is exactly `Rules.BridgeHeight`, `BridgeZOffset`, or a derived bridge-height constant.
2. **Bridge tolerance target class.** Existing docs call the `bVar5` gate "bridge infantry tolerance." This pass verified bridge/on-bridge predicates and the `0x55` radius but did not prove the final gate is infantry-only. The target-class filter should be checked if implementing this tiny edge behavior.
3. **Full CellSpread offset table dump.** Rust has a generated table matching known counts. This pass did not dump `DAT_00ABD490/492` to prove offset order. Order matters if damage side effects chain through multiple affected objects/cells.
4. **Superweapon-specific impact Z.** Nuclear missile and some superweapons pass impact coordinates into `Apply_area_damage`; this pass did not separately trace each superweapon's Z construction. The layer selector will inherit whatever Z each caller passes.

## Sources

- Ghidra decompiled:
  - `0x00489280` `Apply_area_damage`
  - `0x004690B0` `WarheadTypeClass__Detonate`
  - `0x00587180` `ApplyDamageToCell`
  - `0x0048A4F0` `Warhead__SelectExplosionAnim`
  - `0x00578080` `CellClass__GetGroundHeight`
- Ghidra assembly contexts sampled:
  - `0x00489280-0x00489372` setup, early outs, CellSpread, bridge-tolerance setup
  - `0x00489562-0x0048958D` bridge object-list selector
  - `0x004895A3-0x004896D5` CellSpread loop and object-list selection
  - `0x004899B3` selected-list next pointer
  - `0x00489FC2-0x0048A042` high bridge tile damage and retry path
  - `0x0048A147-0x0048A1C6` low bridge tile damage and retry path
  - `0x0048A214-0x0048A2C4` low/high bridge overlay direct damage paths
- Docs referenced:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_SYSTEM.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/combat/systems/splash_cellspread.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/DAMAGE_MATH_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game/docs/gap-scans/2026-05-15-disparity-scan-bridges-overview.md`
- INI checked:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
- Rust files scanned:
  - `src/sim/combat/combat_aoe.rs`
  - `src/sim/combat/mod.rs`
  - `src/sim/combat/cell_spread.rs`
  - `src/sim/occupancy.rs`
  - `src/sim/world/bridge_orchestrator.rs`
