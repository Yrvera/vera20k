# Cliff / Ramp Traversal — Ghidra Research Report

**Date:** 2026-05-17
**Active in YR:** Yes — every cell has a SlopeIndex, every map has ramps and cliffs.
**Confidence:** HIGH overall (decompiled and verified this pass)

## 1. Overview

This doc consolidates the cliff/ramp/slope system: how slope is stored on cells, how it affects movement speed, how ramps are detected, how bridge ramps differ from terrain ramps, and how the pathfinder estimates traversal cost across slopes.

**The three slope-related cell bytes:**
- `CellClass+0x11A` = **RampType** (byte) — bridge-ramp orientation code (`0x02 / 0x04 / 0x08 / 0x0C`)
- `CellClass+0x11B` = **Level** (signed byte) — height in level units (0..N)
- `CellClass+0x11C` = **SlopeIndex** (byte) — terrain slope variant (0=flat, 1..19 = ramps)

These are **three distinct fields**, often conflated. The `Level` is a per-cell height; the `SlopeIndex` determines how the cell visually slopes between its 4 corners; the `RampType` is a bridge-specific orientation.

---

## 2. SlopeIndex enumeration (`CellClass+0x11C`)

Verified from `CellClass::ApplyLAT_and_SlopeFixup @ 0x47CA80`:

| SlopeIndex | Tile mapping | Neighbor check | Visual meaning |
|---|---|---|---|
| **0** | flat — no ramp suffix | n/a | Flat ground |
| **1** | `RampSmooth + 0..1` | checks N + W (cardinal up) | Ramp rising NE |
| **2** | `RampSmooth + 2..3` (offset +2) | checks N + E | Ramp rising NW |
| **3** | `RampSmooth + 5..6` (offset +5) | checks S + W | Ramp rising SE |
| **4** | `RampSmooth + 8..9` (offset +8) | checks S + E | Ramp rising SW |
| **5..19** | `RampBase + (SlopeIndex - 1)` | direct lookup | Steeper / corner slopes |

**Subtle detail — the offset jumps (0,2,5,8):** Each SlopeIndex 1..4 has its own 2-tile sub-table (smooth-pair), so smooth-ramp tile offsets aren't `(SI-1)*2` linearly. The offsets are hand-curated: 0, 2, 5, 8. This is hand-tuned visual sequencing in the original tileset.

**The 1..4 cardinal ramps use a 2-bit mask** from neighbor flatness:
```
mask = 0
if neighbor_A is flat: mask |= 1
if neighbor_B is flat: mask |= 2
final_tile = RampSmooth_offset + mask    // mask in 0..3 = 4 sub-variants per ramp direction
```

If `mask == 0` (both neighbors non-flat): fallback to `RampBase + (SlopeIndex - 1)` — the "isolated steep ramp" variant.

**For SlopeIndex 5..19**: direct `RampBase + (SI - 1)` lookup. No neighbor-check sub-tables. These are the **steeper corner slopes** that don't have smooth-blend variants.

C=HIGH (decompilation of all 5 branches in ApplyLAT_and_SlopeFixup verified), I=HIGH, B=HIGH (xrefs: 3 calls from CellClass::RecalcAttributes at 0x47D551, 0x47D7CD, 0x47DD36).

---

## 3. RampType byte (`CellClass+0x11A`) — bridge-specific

Verified from `MapClass::IsBridgeRampTile @ 0x5746C0`:

| RampType value | Bridge orientation | Tile IDs used |
|---|---|---|
| `0x02` | N/S long-axis | `DAT_00AA1028` family (4 IDs: base, +1, +2, +3) |
| `0x04` | E/W long-axis | `DAT_00ABAD30` family (4 IDs) |
| `0x08` | NE/SW diagonal | `DAT_00ABC2B4`, `DAT_00AA1130` (2 IDs) |
| `0x0C` | NW/SE diagonal | `DAT_00AA1548`, `DAT_00AA0740` (2 IDs) |
| `0x00` | Not a bridge ramp | n/a |

**`IsBridgeRampTile(tile_id, cell*)` returns 1** only if the tile ID matches one of the bridge-ramp tile IDs AND the cell's RampType (+0x11A) matches that family's orientation. The tile IDs themselves are theater-dependent (loaded at map init).

**Subtle detail:** A cell can have **both** SlopeIndex AND RampType set independently — SlopeIndex is the terrain slope (e.g. a hill), RampType is bridge connection metadata. A bridge cell with `cell.Level = 4, SlopeIndex = 1, RampType = 0x02` is a 4-level-high bridge ramp ascending NE, going onto the N/S bridge.

C=HIGH (full decomp verified), I=HIGH, B=HIGH.

---

## 4. Cell Level (`CellClass+0x11B`) and Effective Height

Verified from `CellClass::GetEffectiveHeight @ 0x487D50`:

```c
int GetEffectiveHeight(int cell) {
    return (int)*(char *)(cell + 0x11B)        // signed byte Level
         + (*(uint *)(cell + 0x140) >> 7 & 1) * 4;   // bit 7 (0x80) of Flags adds 4
}
```

**Two-bit flag system on `cell.Flags @ +0x140`:**
- Bit 7 (`0x80`) — **"elevated bridge cell"** → effective height +4 levels. Set by bridge-construction code on cells where bridge geometry covers the underlying terrain.
- Bit 8 (`0x100`) — **"on bridge"** → triggers Z-bump in `Set_Destination` (per `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`). Set on the bridge-deck cell itself.

**These are DIFFERENT bits.** Both can be set on a true bridge cell. A unit on a bridge has:
- Renderer reads `GetEffectiveHeight` → uses bit 0x80 → unit drawn 4 levels higher
- Locomotor reads bit 0x100 → applies bridge Z-bump to destination Z

**The level is a SIGNED byte** (`*(char *)`). So negative levels are technically representable, but in stock maps `Level ∈ [0, 15]`.

C=HIGH, I=HIGH, B=HIGH (verified call site in renderer + locomotor docs).

---

## 5. Slope speed factor (`FootClass+0x530`)

Verified from `FootClass::Get_Slope_Speed_Factor @ 0x4DC760`:

```c
double Get_Slope_Speed_Factor(int foot) {
    // Exemption: if linked to a special object (likely train tracks or convoy lead)
    if (FootClass+0x5D4 != 0
        && *(char *)(*(int *)(FootClass+0x5D4) + 0x24 + 0xF2) != 0) {
        return 1.0;                         // exempt — full speed
    }
    return *(double *)(FootClass + 0x530);  // cached factor
}
```

**Subtle details:**
1. **The exemption check at `+0x5D4`:** This is the FootClass's "linked convoy/train pointer" field. If the linked object's TypeClass byte at `+0xF2` is non-zero (likely "IsTrain" or "IgnoreSlope"), the unit ignores slope speed penalties. Trains on tracks always run at full speed regardless of slope.

2. **The value at `+0x530`:** A `double`. **NOT a per-cell slope cache** — corrected 2026-05-18, see [SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md](SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md). The field is written **exactly once per unit**, at `FootClass::Unlimbo @ 0x4D72F4`, by copying a static `double` from `TechnoTypeClass+0x2F0` (the `ThreatAvoidanceCoefficient` INI key). The value is constant for the unit's lifetime; SlopeIndex never enters the computation. The function name `Get_Slope_Speed_Factor` is a Ghidra-label misnomer — a better name would be `Get_Pathfinder_Cost_Coefficient`. Consumers use it as a multiplier/gate inside the zone pathfinder and path-smoother, not as a per-tick speed modifier. Stock rulesmd.ini sets it to 1.0 on most units; harvester variants override to 0.65 (lines 7344, 7402, 8210, 8262, 9034, 9086).

**Caller xrefs (`get_xrefs_to 0x4DC760`):**
- `Zone_precheck @ 0x42C2BA` (pre-A* zone check — consulted before pathfind)
- `Path_smooth_single_segment @ 0x42B50E` (path smoothing — adjusts costs along smoothed segments)
- `Path_Reroute_Straight_Line @ 0x42BEC3` (straight-line reroute — slope cost on rerouted segments)

**Notably absent from callers:** `Process_Movement` itself. The per-tick speed calc in `Process_Movement` reads `g_SpeedType_LandType_Table[ST + LT*9]` directly (see `MOVEMENT_CLASSIFIERS_REFERENCE.md` §8). Slope speed factor is a **pathfinder-only cost adjustment**, not a per-tick speed multiplier.

C=HIGH (decomp), I=HIGH, B=HIGH (caller xrefs verified).

---

## 6. Slope cost lookup tables

### 6.1 `MapClass::Get_Slope_Cost_At_Cell @ 0x56BCD0`

```c
int Get_Slope_Cost_At_Cell(short *cell_coord, int zone_map_base) {
    int cell_index = cell_coord[1] * 0x200 + cell_coord[0];     // y * 512 + x
    if (cell_index < 0 || cell_index > 0x3FFFF) goto invalid;
    cell = g_CellArray[cell_index];
    if (cell == NULL) goto invalid;
    short x_coord = (short)*(int *)(cell + 0x24);
    short y_coord = (short)((*(int *)(cell + 0x24)) >> 16);
    return *(int *)(zone_map_base + 0x59F0
                  + ((x_coord >> 2) + (y_coord >> 2) * 0x82) * 4);
}
```

**Subtle details:**
1. **The `>> 2` right-shift** divides cell coords by 4 — coarse-grained cost grid at quarter-cell resolution.
2. **Stride `0x82` (130)** = 512 cells / 4 per chunk = 128 chunks wide, +2 for boundary padding.
3. **The base offset `+0x59F0`** in the zone map points to the slope cost subgrid (32 KB region after the zone-id arrays).
4. **Coarse-grained**: each entry covers a 4×4 cell region. A unit's slope cost is shared across its 16-cell neighborhood. This is the **zone-pathfinder hierarchical cost grid**.

### 6.2 `Zone_Estimate_Slope_Cost @ 0x585F40`

Used during **zone flood-fill build** to populate the cost grid. The function handles three cases by param_2:
- `param_2 == 0` → return 0 (no slope cost)
- `param_2 == 1` → simple lookup: `*(int *)(zone_map + 0x57E4 + DAT_0087F890[0x20 + param_4*0x24] * 4)` (per-orientation cost)
- `param_2 == 2` → **bilinear interpolation** between 4 adjacent corner heights with directional bias from the `0x82A984` direction table

**The `0x82A984` direction table** (24 entries × 8 bytes = 192 bytes, read this pass via `read_memory`):

| Diagonal case | Primary direction | Secondary direction |
|---|---|---|
| 0 (---) | 0 | 1 |
| 1 (NE-up,SE-up) | 1 | 1 |
| 2 (NW-up,SE-up) | 1 | 3 |
| 3 (NW-up,NE-down) | 3 | 3 |
| 4 (---) | 2 | 3 |
| 5 (SW-up,NE-up) | 2 | 2 |
| 6 (--,--) | 0 | 2 |
| 7 (--,--) | 0 | 0 |

These indices select from `DAT_00ABD460` (cell-offset lookup table). The two-direction lookup performs bilinear interpolation of slope cost between 4 adjacent cells. The complex math averages corner-height costs for the slope-traversal estimate.

C=HIGH (full decompilation), I=HIGH, B=HIGH (Zone_Estimate_Slope_Cost is the BUILD-time analogue of Get_Slope_Cost_At_Cell).

---

## 7. Cliff-as-impassable rules

A "cliff" (steep terrain transition) is NOT a separate cell type. Cliff behavior emerges from three rules:

### 7.1 LandType=3 (Rock) is impassable

Per `MOVEMENT_CLASSIFIERS_REFERENCE.md` §4: Rock terrain (`cell.LandType == 3`) maps to ZoneType=6 (Impassable) via `RecalcZoneType` (verified at `0x483C80`). The passability matrix blocks Rock for all MovementZones except Fly (row 9 col 6 = 1) and Subterranean (row 6 col 6 = 1).

**Player-observable:** Tanks cannot drive over visually-rocky cliffs. Aircraft fly over them. TS-legacy subterranean units could tunnel under them (not active in YR).

### 7.2 Height-diff threshold in Process_Movement

Per `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §8.6 and `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §6.2.4: when a unit's current cell and next cell differ in level by **≥ 2** levels (`abs(diff) ≥ 2`), the speed-table LandType lookup uses **`LandType = 1 (Clear)`** instead of the next cell's actual LandType. This is "vaulting over the cliff" — the unit treats the high terrain as if it were clear, applying the cliff multiplier instead of a per-terrain multiplier.

```c
height_diff = effective_level - new_cell.Level;
if (abs(height_diff) < 2) {
    LandType = new_cell.LandType;       // use cell's real LandType
} else {
    LandType = 1;                       // Clear (we're "vaulting")
}
```

The height-diff is the unit's effective level (including bridge bit 0x80) minus the new cell's flat Level.

### 7.3 Cliff speed multipliers (the 4 RulesClass constants)

When `Mission_ID == 1` (Mission_Move) AND `new_cell.GroundHeight != old_cell.GroundHeight`:

```c
if (new_ground < old_ground) {                      // going DOWN
    if (SpeedType == 1 /*Track*/)
        speed *= RulesClass+0x770;                  // TrackedDownhill   (=1.2 stock)
    else
        speed *= RulesClass+0x780;                  // WheeledDownhill   (=1.2 stock)
} else if (new_ground > old_ground) {               // going UP
    if (SpeedType == 1 /*Track*/)
        speed *= RulesClass+0x768;                  // TrackedUphill     (=1.0 stock)
    else
        speed *= RulesClass+0x778;                  // WheeledUphill     (=1.0 stock)
}
```

INI keys (verified — see [RULES_CLIFF_SPEED_MULTIPLIERS_GHIDRA_REPORT.md](RULES_CLIFF_SPEED_MULTIPLIERS_GHIDRA_REPORT.md)):
section `[General]`; exact key names `TrackedUphill` / `TrackedDownhill` /
`WheeledUphill` / `WheeledDownhill` (no `Speed` suffix); each is a sole-write
`double` field. Stock rulesmd.ini lines 401–404 ship 1.0 / 1.2 / 1.0 / 1.2. No
clamping on read.

C=HIGH (formula in Ship/Walk locomotor docs + offset/key mapping verified from binary), I=HIGH, B=HIGH.

---

## 8. SlopeIndex propagation on object move — REFUTED 2026-05-18

**This section's premise was wrong.** Per [SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md](SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md):

- `LocomotionClass::ForEach_SetSlopeIndex @ 0x4E1570` has **zero direct CALL sites** in `.text` (only `[DATA]` xrefs from vtables). It is not actively invoked.
- `vtable+0x6C` in the FootClass vtable (read at `0x7E8D00`) is **`FootClass::ComputeChecksum @ 0x4DBAD0`**, not a slope cache updater. It reads `+0x530` into the network checksum stream — it does NOT write the field.
- Cells do not propagate slope values to objects, and objects do not maintain a per-cell slope cache. The per-pathfind cost coefficient at `+0x530` is set once at Unlimbo (§5.2) and stays constant.

C=HIGH (refutation verified via vtable read + xref enumeration), I=HIGH, B=HIGH.

---

## 9. TMP theater data — slope type extraction

Verified from `TMP_ReadSlopeType @ 0x5471B0`:

```c
int TMP_ReadSlopeType(int *tmp_file_ptr, int frame_index) {
    int *frame_descriptor = tmp_file_ptr->vtable+0x9C();   // get frames array
    if (frame_descriptor != NULL) {
        int width_x_height = frame_descriptor[1] * frame_descriptor[0];
        int subframe = frame_descriptor[frame_index % width_x_height + 4];
        if (subframe != 0) {
            return (int)*(char *)(subframe + 0x2A);        // SlopeType at offset 0x2A in subframe
        }
    }
    return 0;
}
```

**TMP subframe layout — byte at offset 0x2A** holds the slope type. The TMP file format encodes per-frame slope metadata that the cell loads when applying the tile.

**Frame indexing:** `frame_index % (width × height)` — handles randomized tile orientations.

C=HIGH (decomp), I=HIGH, B=MEDIUM (caller traces deferred — not critical for movement).

---

## 10. Open questions

1. ~~**Exact SlopeIndex → speed-factor table** at `FootClass+0x530` update site.~~ **RESOLVED 2026-05-18:** No such table exists. `FootClass+0x530` is set once at Unlimbo from `TechnoTypeClass+0x2F0` (`ThreatAvoidanceCoefficient`) and is not updated on cell change. See [SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md](SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md).

2. ~~**RulesClass+0x768/0x770/0x778/0x780 INI key names** — likely `TrackedDownhillSpeed`, `TrackedUphillSpeed`, `WheeledDownhillSpeed`, `WheeledUphillSpeed`.~~ **RESOLVED 2026-05-19:** Actual keys are `TrackedUphill` / `TrackedDownhill` / `WheeledUphill` / `WheeledDownhill` (no `Speed` suffix), section `[General]`. See [RULES_CLIFF_SPEED_MULTIPLIERS_GHIDRA_REPORT.md](RULES_CLIFF_SPEED_MULTIPLIERS_GHIDRA_REPORT.md). §7.3 offset-to-key mapping has been corrected in this pass.

3. **`DAT_0087F890` direction-cost table** referenced from `Zone_Estimate_Slope_Cost` param_2==1. 36-byte stride suggests 9-int-per-direction × 8 directions. Contents not extracted.

4. **`DAT_00ABD460` cell-offset table** (40 bytes, used by Zone_Estimate_Slope_Cost for bilinear interp). Contents not extracted.

5. **`BridgeSlopeTable_StaticInit @ 0x544691`** writes mostly zeros at boot, then a few specific non-zero entries at offsets 0xC, 0xD, 6, 0x30, 0x1, 0xE. These appear to be tile-orientation defaults for bridge slopes. Worth extracting if bridge fidelity becomes an issue.

6. **TMP subframe byte at +0x2A semantic** — confirmed as slope type but exact value enumeration unverified. Need to cross-reference with TMP loader source / file format spec.

---

## 11. TS-legacy filtering

| Subsystem | Active in YR? | Evidence |
|---|---|---|
| Cell SlopeIndex (0..19) | Yes | every cell in every map has a SlopeIndex |
| Cliff speed multipliers | Yes | RulesClass offsets 0x768-0x780 active in Process_Movement |
| Bridge ramp types (0x02/0x04/0x08/0x0C) | Yes | bridge maps in stock YR use these |
| LAT auto-transition (Rough/Sand/Green/Pave) | Yes | terrain rendering depends on it |
| FootClass+0x530 (per-unit pathfinder cost coefficient, ex-"slope cache") | Yes | every Foot unit has it; set once at Unlimbo from `TechnoTypeClass+0x2F0` (ThreatAvoidanceCoefficient) — NOT a slope cache |
| ~~`LocomotionClass::ForEach_SetSlopeIndex` propagation~~ | **No (not invoked)** | zero CALL sites in .text; vtable+0x6C is ComputeChecksum, not a slope updater. Refuted 2026-05-18. |
| `Get_Slope_Cost_At_Cell` (zone pathfinder) | Yes | used in Zone_precheck |
| Subterranean slope rules | **TS-legacy** | per `[[feedback_no_tunnel_subterranean]]`, dormant in YR |
| Train-track slope exemption (`FootClass+0x5D4 + TypeClass+0xF2`) | **Conditional** | only active if `IsTrain=yes` on the unit's TypeClass. Stock YR has no trains; only modded content uses this. |

**No SpecialFlags-gated cliff branches found.** Standard YR skirmish exercises every cliff/ramp path.

---

## 12. Sources

**Ghidra functions decompiled (this pass):**
- `FootClass::Get_Slope_Speed_Factor @ 0x4DC760` — full body
- `MapClass::Get_Slope_Cost_At_Cell @ 0x56BCD0` — full body
- `CellClass::GetGroundHeight @ 0x578080` — full body
- `CellClass::GetEffectiveHeight @ 0x487D50` — full body (2-line function)
- `IsOnBridgeRamp @ 0x578D80` — full body
- `MapClass::IsBridgeRampTile @ 0x5746C0` — full body
- `CellClass::ApplyLAT_and_SlopeFixup @ 0x47CA80` — full body (~500 lines)
- `TMP_ReadSlopeType @ 0x5471B0` — full body
- `LocomotionClass::ForEach_SetSlopeIndex @ 0x4E1570` — full body
- `Zone_Estimate_Slope_Cost @ 0x585F40` — full body (~100 lines, bilinear interp)
- `BridgeSlopeTable_StaticInit @ 0x544691` — full body
- `FUN_0047B3A0` — partial (boot-time matrix initializer; not directly slope-related)

**Memory reads:**
- `0x82A984` len 96 — slope direction lookup table (24 entries × 8 bytes for bilinear case selection)

**Xref tables:**
- `get_xrefs_to 0x4DC760` (Get_Slope_Speed_Factor) → 3 callers (Zone_precheck, Path_smooth_single_segment, Path_Reroute_Straight_Line)
- `get_xrefs_to 0x483C80` (RecalcZoneType) → 3 callers (all in CellClass::RecalcAttributes)

**Companion docs:**
- `MOVEMENT_CLASSIFIERS_REFERENCE.md` — MovementZone / SpeedType / LandType / ZoneType enums
- `ZONE_PASSABILITY_VERIFIED.md` — passability matrix
- `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` — bridge Z-offset interaction
- `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §8.6 — cliff multiplier consumption
- `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §6.2.4 — Walk-specific slope/bridge thresholds
- `CELLCLASS_ZONES_SPEED_BRIDGES.md` — CellClass field layout

---

*End of report.*
