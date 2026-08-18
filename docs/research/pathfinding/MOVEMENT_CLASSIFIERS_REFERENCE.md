# Movement Classifiers Reference — MovementZone, SpeedType, ZoneType, LandType

**Purpose:** Canonical reference that ties together the four enums used by movement/pathfinding. Most existing docs reference one or two of these without cross-linking; this doc maps the relationships in one place and adds 3-axis confidence labelling.

**Primary source doc:** `ZONE_PASSABILITY_VERIFIED.md` (2026-03-23, corrections through 2026-04-06) — has the full enum values, parsing addresses, and matrix. This doc does NOT duplicate that content; it organizes it as a cross-reference and verifies the load-bearing claims.

**Companion docs:**
- `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` — naval-specific passability matrix walkthrough
- `TODO_ZONE_FIDELITY_FIXES.md` — Rust-side fixes needed
- `CELLCLASS_ZONES_SPEED_BRIDGES.md` — CellClass field offsets
- `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md` — zone flood-fill build
- `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` — zone-registry storage
- `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §8.6 — speed lookup using `g_SpeedType_LandType_Table`
- `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §6.2.4 — same speed lookup mechanism

---

## 1. The four enums and how they relate

```
                    ┌──────────────────┐
                    │   INI parsing    │
                    └──────────────────┘
                             │
            ┌────────────────┼────────────────┐
            │                │                │
            ▼                ▼                ▼
      MovementZone=     SpeedType=       (cell terrain)
      (TT+0x5B4)        (TT+0x67C)              │
            │                │                  │
            │                │                  ▼
            │                │            LandType (cell+0xEC)
            │                │            (12 values, per-cell)
            │                │                  │
            │                │                  ▼
            │                │            RecalcZoneType
            │                │            (cell+0x4C derivation)
            │                │                  │
            │                │                  ▼
            │                │            ZoneType (cell+0x4C)
            │                │            (8 values, derived)
            │                │                  │
            └─Row──┐         │                  │
                   ▼         │                  ▼
              13×8 PASSABILITY MATRIX ────── Col
              @ 0x82A594
                   │
                   ▼
              passable? (1) / blocked (2) / OoB sentinel (3)
                   │
                   ▼
              A* zone gate (Can_Reach_Zone)

                   ┌─────────────────────┐
                   │ g_SpeedType_LandType_Table │
                   │ indexed [SpeedType + LandType*9]  │
                   └─────────────────────┘
                             │
                             ▼
                  base movement speed multiplier
                  (consumed in ProcessMovement)
```

**Two parallel uses of the classifiers:**
1. **Passability** (binary pass/block decision): uses **MovementZone** as matrix row + **ZoneType** (derived from LandType+overlays) as column → consult 13×8 matrix.
2. **Speed multiplier** (continuous decimal): uses **SpeedType** + **LandType** directly (12×9 lookup, not the 8-zone reduction).

A unit's MovementZone is parsed from `MovementZone=` in its INI. A unit's SpeedType from `SpeedType=`. Both stored on `TechnoTypeClass`. The cell's LandType is parsed from the TMP tile; the ZoneType is computed at runtime from LandType + overlays + objects via `CellClass::RecalcZoneType @ 0x483C80`.

---

## 2. MovementZone enum (13 values, the matrix row)

| Idx | Name | Profile | Stock unit examples |
|---|---|---|---|
| 0 | Normal | Ground only | (most armored vehicles) |
| 1 | Crusher | Ground + roads | (crushing vehicles — Rhino, Apocalypse) |
| 2 | Destroyer | Ground + roads + walls | (wall-destroying vehicles — Demolition Truck?) |
| 3 | AmphibiousDestroyer | Land + water + beach + walls | (amphibious destroyer) |
| 4 | AmphibiousCrusher | Land + walls + beach + water | (amphibious crusher) |
| 5 | Amphibious | Land + water | Hovercraft (LCAC), maybe Hover units |
| 6 | Subterranean | Ground + roads + walls + "impassable" col | TS legacy (Devil's Tongue) |
| 7 | Infantry | Ground + buildings (garrison) | All infantry |
| 8 | InfantryDestroyer | Ground + roads + walls + buildings | (special infantry?) |
| 9 | Fly | Everything except OoB sentinel | All aircraft |
| 10 | Water | Water only | Destroyer, Aegis, Carrier, Dreadnought, Sub |
| 11 | WaterBeach | Water + beach | Dolphin, Squid |
| 12 | CrusherAll | Same profile as Destroyer (2) | (crush-all variant — differs only in gameplay-side crush behaviour) |

**Parsing:** `CCINIClass::ReadMovementZone @ 0x474E40` → result stored at `TechnoTypeClass+0x5B4`. String table at `0x81BAF4` (verified by `read_memory` this pass).

**Subtle detail:** The binary string table has `"Subterannean"` (typo — missing R). This is the canonical INI key — `MovementZone=Subterannean` (mis-spelled) is what the parser accepts. A correctly-spelled `MovementZone=Subterranean` would NOT match.

**C** = HIGH (matrix bytes verified via `read_memory 0x82A594 len 416` this pass — confirms 13 rows × 8 cols × 4 bytes), **I** = HIGH (parser at 0x474E40 confirmed via `[[feedback_caller_trace_before_finding]]` — single xref from `TechnoTypeClass__ReadINI @ 0x712170` call-site 0x716065), **B** = HIGH. (corrected 2026-05-29: was "@ 0x716065"; `get_function_by_address 0x716065` returns entry 0x712170; 0x716065 is a call-site inside the function body, not the entry; ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT)

---

## 3. SpeedType enum (8 values, the speed-table row)

| Idx | Name | Used by |
|---|---|---|
| 0 | Foot | All infantry |
| 1 | Track | Tanks |
| 2 | Wheel | Wheeled vehicles |
| 3 | Hover | Hovercraft |
| 4 | Winged | Aircraft |
| 5 | Float | Naval ships |
| 6 | Amphibious | Amphibious vehicles |
| 7 | FloatBeach | Beach-capable naval |

**Parsing:** `CCINIClass::ReadSpeedType @ 0x476FC0` → result stored at `TechnoTypeClass+0x67C`. String table starts at `0x81DBA8` (verified by `search_strings` this pass — found "Float" at 0x81DBAC, "Winged" at 0x81DBB4, "Hover" at 0x81DBBC, "Wheel" at 0x81DBC4, "Track" at 0x81DBCC).

**C** = HIGH, **I** = HIGH, **B** = HIGH.

---

## 4. ZoneType enum (8 values, the matrix column)

Derived per-cell by `CellClass::RecalcZoneType @ 0x483C80`. Stored at `CellClass+0x4C` (4-byte int).

| Col | Name | Assignment Rule |
|---|---|---|
| 0 | Ground/Clear | Default fallthrough (most terrain) |
| 1 | Road/Crate | Cell has road overlay (`OverlayType+0x22D = IsCrate` per ZONE_PASSABILITY_VERIFIED.md 2026-04-06 correction) |
| 2 | Wall | Cell has wall overlay (`OverlayType+0x2A8 = IsWall`) |
| 3 | Beach | `CellClass.LandType == 6` |
| 4 | Water (deep) | `CellClass.LandType == 2` |
| 5 | Building | Terrain/Building object on cell |
| 6 | Impassable | Tiberium overlay with speed=0, or `g_SpeedType_LandType_Table[Wheel + LT*9] ≤ 0.01` (**Wheel** column, base `0x89EA48`; see [SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md](../SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md) §3.2) |
| 7 | OutOfBounds | Cell not in playfield diamond; **always value 3 (sentinel) in matrix** |

**Decision tree** (verified in `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` §1.RecalcZoneType decompilation):

```
1. If not in playfield → 7 (OoB), RETURN
2. Check overlay:
   a. IsRoad/IsCrate → 1 (Road), RETURN     [OverlayType+0x22D]
   b. IsWall → 2 (Wall), RETURN              [OverlayType+0x2A8]
   c. overlay speed == 0.0 → 6 (Impassable), RETURN   [speed_table[overlay.LandType @ OverlayType+0x298, *9] == 0.0]
   d. IsGate → 6 (Impassable), RETURN        [OverlayType+0x2B5] (corrected 2026-05-29: was "IsTiberium → 6 (Impassable)"; binary at 0x483CC8 checks +0x2B5 (Ghidra: IsGate) → ZoneType=6, NOT +0x2B4; ROOT_CAUSE: MISLEADING/INFERENCE_HARDENED; verified via decompile_function 0x483C80)
   e. flag at OverlayType+0x2B4 → 0 (Ground), RETURN  (added 2026-05-29: binary `if (*(char*)(iVar3+0x2b4)) goto LAB_00483dd4` → ZoneType=0; this step was missing from the decision tree; verified via decompile_function 0x483C80)
3. Check base LandType:
   a. LandType == 2 (Water) → 4 (Water), RETURN
   b. LandType == 6 (Beach) → 3 (Beach), RETURN
4. If g_SpeedType_LandType_Table[Wheel + LandType*9] ≤ 0.01 → 6 (Impassable), RETURN  (base `0x89EA48`, Wheel column, NOT Foot)
5. Check objects on cell (buildings, terrain) → 5 or 6
6. Default → 0 (Ground)
```

**Subtle detail — Rough/Railroad/Ice/Tunnel/Weeds terrain** all fall through to ZoneType **0 (Ground)** (since they're not Water, not Beach, not impassable). Only WATER, BEACH, overlays, buildings, and ≤1% speed terrain get non-zero ZoneTypes. This is **why a Rough cell is passable for Normal-zone vehicles** — the matrix is consulted at column 0 (Ground), where Normal=1 (passable).

The widespread misconception that "Rough is its own column" is wrong. The 8-zone reduction collapses many LandTypes into "Ground" because passability is binary, not graduated. Speed differences for Rough/Ice/etc. come from `g_SpeedType_LandType_Table` (the speed multiplier), not from the passability matrix.

**C** = HIGH, **I** = HIGH, **B** = HIGH.

---

## 5. LandType enum (12 values, per-cell)

Stored at `CellClass+0xEC` (int). String table at `0x81DBC0`–`0x81DC1C`.

| Idx | Name | Speed-table impact |
|---|---|---|
| 0 | Clear | Baseline |
| 1 | Road | Faster on roads (for wheeled) |
| 2 | Water | Deep water — naval only |
| 3 | Rock | Impassable for almost everyone |
| 4 | Wall | Overlay terrain |
| 5 | Tiberium | Hostile to most units |
| 6 | Beach | Shore-adjacent water cells |
| 7 | Rough | Slower than Clear |
| 8 | Ice | Slippery — speed mult applies |
| 9 | Railroad | Track-only fast traversal |
| 10 | **Tunnel** | TS-legacy subterranean cells |
| 11 | Weeds | TS-legacy terrain |

**Parsed indirectly:** LandType is set from the TMP tile theater data (`TMP→LandType` table at `0x8288E4`, per TODO_ZONE_FIDELITY_FIXES.md). Not parsed from a `LandType=` INI key.

**Tunnel (10) is the only LandType referenced specifically in stuck-detection logic:** `Process_Movement` checks `dest_cell.LandType != 10` before allowing CloseEnough early-stop (see `STUCK_DETECTION_SYNTHESIS.md` State D). Tunnel cells require full traversal.

**C** = HIGH, **I** = HIGH, **B** = HIGH.

---

## 6. The 13×8 passability matrix @ `0x82A594` — verified

Read this pass via `read_memory 0x82A594 len 416`. The 13 rows × 8 columns × 4 bytes layout matches:

```
                    Col0  Col1  Col2  Col3  Col4  Col5  Col6  Col7
                    Grnd  Road  Wall  Bch   Wtr   Bldg  Imps  OoB
 0 Normal            1     2     2     2     2     2     2     3
 1 Crusher           1     1     2     2     2     2     2     3
 2 Destroyer         1     1     1     2     2     2     2     3
 3 AmphibDestroyer   1     1     1     1     1     1     2     3
 4 AmphibCrusher     1     1     2     1     1     2     2     3
 5 Amphibious        1     2     2     1     1     2     2     3
 6 Subterranean      1     1     1     2     2     2     1     3
 7 Infantry          1     2     2     2     2     1     2     3
 8 InfantryDestroyer 1     1     1     2     2     1     2     3
 9 Fly               1     1     1     1     1     1     1     3
10 Water             2     2     2     2     1     2     2     3
11 WaterBeach        2     2     2     1     1     2     2     3
12 CrusherAll        1     1     1     2     2     2     2     3
```

**Values:** 1 = passable; 2 = blocked; 3 = OoB sentinel (only ever in col 7). Per `[[ZONE_PASSABILITY_VERIFIED.md §6 correction 2026-04-06]]`: only value 1 passes the `== 1` check in zone logic. Values 2 AND 3 are both "not passable" semantically.

**Subtle detail — Fly (9) col 7 = 3, not 1:** Aircraft cannot fly into out-of-bounds. They're confined to the playfield diamond. The "Fly passes everything" colloquially refers to in-bounds cells.

**C** = HIGH (binary bytes verified), **I** = HIGH, **B** = HIGH (consumed at `AStar_pathfind_search @ 0x42C900` and `ZoneMap__FloodFillReachableZones @ 0x5840C0`).

---

## 7. Cross-row team pathfinding (`FindBestCompatibleMovementZone @ 0x5889F0`)

When a team contains units with different MovementZones, the engine finds the most permissive row compatible with all team members:

```c
int FindBestCompatibleMovementZone(int mz1, int mz2) {
    int bestRow = -1, bestScore = 0;
    for (int cand = 0; cand < 13; cand++) {
        bool valid = true; int score = 0;
        for (int col = 0; col < 8; col++) {
            int cv = matrix[cand][col], v1 = matrix[mz1][col], v2 = matrix[mz2][col];
            if ((v1 == 2 || v2 == 2) && cv == 1) valid = false;   // disqualify if cand permits where input blocks
            if (v1 == cv && v2 == cv && cv == 1) score++;          // count agreements
        }
        if (valid && score > bestScore) { bestScore = score; bestRow = cand; }
    }
    return bestRow;
}
```

Called from `TeamTypeClass::ComputeZoneCategory @ 0x6F1FA0` during scenario init. The result becomes the team's effective MovementZone for group-pathfinding.

**This is the answer to the index TODO "formation / group-move pathing" — the team selects a common compatible MovementZone.** See `CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT.md` for convoy-specific extensions.

**C** = HIGH (decompilation in source doc §7), **I** = HIGH, **B** = HIGH (caller verified via xref to `TeamTypeClass__ComputeZoneCategory`).

---

## 8. g_SpeedType_LandType_Table (the speed multiplier table)

Referenced from `ShipLocomotionClass::Process_Movement` and `WalkLocomotionClass::ProcessMovement` as:
```c
speed = (double) g_SpeedType_LandType_Table[TT.SpeedType + LandType * 9];
```

**Stride is 9, not 8.** Each LandType row holds 8 SpeedType floats (Foot..FloatBeach) + a 9th slot. The 9th slot is **NOT padding** — it is the `Buildable=` bool byte for that LandType, written by the loader as `MOV byte ptr [EBX + 0x1c], AL` at `0x0067421b` from `CCINIClass::ReadBool(section, "Buildable", default=0)`. Anyone reading `g_SpeedType_LandType_Table[8 + LT*9]` as a float gets denormalized garbage. See [SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md](../SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md) §2 for the full per-column layout.

**Subtle detail — the 0.01 threshold in RecalcZoneType:** `if (g_SpeedType_LandType_Table[Wheel + LandType * 9] ≤ 0.01) → ZoneType = 6 (Impassable)`. The check uses **`SpeedType = 2 (Wheel)`** as the reference (base `0x89EA48`), NOT Foot. If wheeled vehicles can't move on this terrain at >1% speed, the cell is classified Impassable for everyone. For stock INI sections, no row differs between Foot and Wheel speeds, so the outward behavior is the same — but the Wheel column is what the binary actually consults.

**The cliff-speed multipliers** (`RulesClass+0x768/0x770/0x778/0x780`) are applied AFTER the base table lookup, in Process_Movement. See `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §8.6 for the full per-tick speed-computation formula.

**Resolved (2026-05-18):** `g_SpeedType_LandType_Table` is at base `0x0089EA40`, 12 rows × 9 slots (8 floats + Buildable byte), stride `0x24`, populated by `RulesClass__ReadSpeedTypeLandTypeTable @ 0x00674000` at scenario init. Loaded from rulesmd.ini `[Clear]`/`[Road]`/etc. sections, with `Winged=` column hardcoded to 1.0 (INI key ignored). Full layout + values in [SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md](../SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md).

---

## 9. INI binding summary

| INI key | Section | ReadINI parser | Stored at | Default |
|---|---|---|---|---|
| `MovementZone=` | per-unit (UnitType/InfantryType/etc.) | `CCINIClass::ReadMovementZone @ 0x474E40` | `TechnoTypeClass+0x5B4` | (no default — required) |
| `SpeedType=` | per-unit | `CCINIClass::ReadSpeedType @ 0x476FC0` | `TechnoTypeClass+0x67C` | (no default — required) |
| `IsTrain=` (related) | per-unit | inline in `TechnoTypeClass::ReadINI @ 0x712277` | `TechnoTypeClass+0xC94` (byte) | false |

**Subtle parser detail:** `MovementZone=Subterannean` (mis-spelled) is the binary's expected token. Players/modders accidentally writing `Subterranean` (correctly spelled) won't match and will get the default fallback (likely Normal=0).

---

## 10. Status of related TODO_ZONE_FIDELITY_FIXES items

Per `TODO_ZONE_FIDELITY_FIXES.md` (2026-04+ updates), the existing Rust implementation has several known bugs in this area:

1. **Rust MovementZone enum missing Amphibious (index 5)** → 12 variants instead of 13
2. **Rust SpeedType enum wrong order** → indices don't match binary
3. **`zone_layer_for_movement_zone()` architecturally wrong** → each MovementZone IS its own row; no remap layer needed
4. **Passability matrix row 5 (Amphibious) has wrong values** → `[1,2,2,2,2,2,1,3]` instead of `[1,2,2,1,1,2,2,3]` — Beach/Water cols swapped
5. **Column 6 (Impassable) values wrong** → Subterranean and Fly should pass; Rust blocks them
6. **Only 6 zone maps in Rust** (one per ZoneCategory) → binary has 13 (one per MovementZone). Crusher and Normal share Land-category map but have different actual passability rows.

These are implementation TODOs, not research gaps. The binary behaviour is fully documented here and in the cross-referenced docs.

---

## 11. Sources

**Memory reads (this pass):**
- `0x82A594` len 416 — passability matrix bytes verified
- `0x81BAF4` len 160 — MovementZone enum string table
- `0x81BB38` len 80 — MovementZone string continuation + Foundation type strings (0x0/6x4/3x4/etc.)
- `0x81DBA8`–`0x81DBD0` — SpeedType enum string table

**Ghidra functions referenced:**
- `CCINIClass::ReadMovementZone @ 0x474E40`
- `CCINIClass::ReadSpeedType @ 0x476FC0`
- `CellClass::RecalcZoneType @ 0x483C80`
- `ZoneMap::FindBestCompatibleMovementZone @ 0x5889F0`
- `AStar_pathfind_search @ 0x42C900`
- `ZoneMap::FloodFillReachableZones @ 0x5840C0`
- `TeamTypeClass::ComputeZoneCategory @ 0x6F1FA0`

**Primary source docs:**
- `ZONE_PASSABILITY_VERIFIED.md` — comprehensive enum + matrix doc (2026-03-23, corrected 2026-04-06)
- `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` — RecalcZoneType decompilation
- `TODO_ZONE_FIDELITY_FIXES.md` — Rust-side implementation bugs

**INI files cross-referenced:**
- `ini/rulesmd.ini` — 154 `MovementZone=` entries (verified by Grep count this pass)

**Memory feedback references:**
- `[[feedback_research_confidence_axes]]` — 3-axis confidence applied
- `[[feedback_caller_trace_before_finding]]` — caller traces via cross-doc xrefs

---

*End of reference. This doc is intentionally a cross-reference index, not a deep dive — its job is to make the existing documentation discoverable from a single canonical entry point and verify the load-bearing claims at current confidence standards.*
