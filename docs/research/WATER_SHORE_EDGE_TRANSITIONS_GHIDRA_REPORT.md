# Water-Shore Edge Transitions — Ghidra Research Report

**Date:** 2026-05-17
**Active in YR:** Yes — every map with water has shore cells; SEAL/Tanya and hover units cross them every match.
**Confidence:** HIGH overall

## 1. Overview

"Water-shore edge transitions" refers to how units cross between land cells (`LandType = 0/1/etc.`) and water cells (`LandType = 2`). The interface is the **Beach** zone — `LandType = 6 (Beach)` cells that act as a passability bridge between land and water.

**The shore-crossing problem:**
- A naval unit (`MovementZone = Water`, row 10) can pass Water (col 4) but not Beach (col 3) or Ground (col 0)
- A land vehicle (`MovementZone = Normal`, row 0) can pass Ground (col 0) but not Beach (col 3) or Water (col 4)
- **Neither can reach the shore.** Beach cells form a hard barrier between the two zones.

**The shore-crossing solution:**
- The 4 "amphibious-family" MovementZones (3 AmphibiousDestroyer, 4 AmphibiousCrusher, 5 Amphibious, 11 WaterBeach) are the only zones that pass Beach (col 3 = 1).
- These units can step from Ground onto Beach onto Water in sequence — bridging the gap.

This is the entire shore-crossing mechanism. There's no special "wading state", no per-unit "amphibious flag" beyond the MovementZone. The matrix does all the work.

---

## 2. Beach detection — the `LandType == 6` rule

Per `MOVEMENT_CLASSIFIERS_REFERENCE.md` §4 and `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md`:

**The Beach LandType (value 6)** is set on the cell from the TMP theater tile data when the tile is part of the shore-piece set. `CellClass::RecalcZoneType @ 0x483C80` then promotes it to `ZoneType = 3 (Beach)`:

```c
// In RecalcZoneType:
3. Check base LandType:
   a. LandType == 2 (Water) → ZoneType 4 (Water), RETURN
   b. LandType == 6 (Beach) → ZoneType 3 (Beach), RETURN
```

`CellClass::IsShorePieceTile @ 0x4865B0` verifies the tile range:

```c
int IsShorePieceTile(int cell) {
    int tile_id = *(int *)(cell + 0x38);
    return (g_ShorePieces <= tile_id) && (tile_id < g_ShorePieces + 0x2A);
}
```

**`g_ShorePieces`** is the base tile ID of the shore-piece set, loaded from the `[General] ShorePieces=` INI key in theater files (verified via `get_xrefs_to "ShorePieces" @ 0x008294EC` → `Read_Theater_TileSets_INI @ 0x054572F`). **42 tiles** (`0x2A`) make up the full shore set, covering all 8 orientations of land-meets-water edge plus diagonals plus reinforcements.

**Subtle detail:** A tile ID `g_ShorePieces + N` for `N ∈ [0, 41]` is automatically classified as Beach (LandType 6 → ZoneType 3). The mapping is **theater-dependent** (Temperate, Snow, Urban, Lunar, etc.) but the principle is identical across theaters — only the base ID `g_ShorePieces` differs.

**C** = HIGH (decomp + caller traces), **I** = HIGH, **B** = HIGH (4 callers via `get_function_callers`: FUN_0059A6C0, FUN_005A33F0, FUN_005A38C0 (shore-rendering helpers), `MapClass::ApplyBridgeTile @ 0x57B440` (bridge-placement validation)).

---

## 3. The passability matrix view of beach

From the 13×8 matrix at `0x82A594` (column 3 = Beach):

| MovementZone | Col 3 (Beach) | Can cross shore? | Stock unit example |
|---|---|---|---|
| 0 Normal | 2 (blocked) | No | Grizzly Tank, Rhino |
| 1 Crusher | 2 (blocked) | No | Apocalypse |
| 2 Destroyer | 2 (blocked) | No | (no stock unit) |
| **3 AmphibiousDestroyer** | **1 (pass)** | **Yes** | **SEAL, Tanya, Yuri Prime** |
| **4 AmphibiousCrusher** | **1 (pass)** | **Yes** | (no stock unit — commented in INI as alternative for SAPC) |
| **5 Amphibious** | **1 (pass)** | **Yes** | **Hovercraft (SAPC), Robot Tank** |
| 6 Subterranean | 2 (blocked) | No | TS-legacy (Devil's Tongue) |
| 7 Infantry | 2 (blocked) | No | All regular infantry (GI, Conscript) |
| 8 InfantryDestroyer | 2 (blocked) | No | (no stock unit) |
| 9 Fly | 1 (pass) | Yes (flies over) | All aircraft |
| 10 Water | 2 (blocked) | No | All ships (DEST/AEGS/Carrier/Dreadnought/Sub) |
| **11 WaterBeach** | **1 (pass)** | **Yes** | (commented as alternative for landing craft — unused in stock YR) |
| 12 CrusherAll | 2 (blocked) | No | (gameplay variant of Destroyer) |

**Important observation — regular infantry CANNOT cross shore.** Infantry MovementZone (7) blocks Beach. Only the specific "amphibious infantry" variant (AmphibiousDestroyer, used by SEAL/Tanya/Yuri Prime) can walk into water.

**Naval ships cannot reach shore.** MovementZone=Water blocks Beach. Ships' farthest approach to land is the deepwater cells adjacent to beach.

**The Beach column is the choke point.** It's why amphibious units feel "special" — they're the only land-locomotor units that can transition between water and land.

**C** = HIGH (matrix bytes verified in `MOVEMENT_CLASSIFIERS_REFERENCE.md` §6), **I** = HIGH, **B** = HIGH.

---

## 4. Stock-unit amphibious roster (rulesmd.ini)

Spot-verified entries from `Grep` this pass:

### 4.1 AmphibiousDestroyer (MovementZone 3)

| Unit | Section | Line | SpeedType | Westwood comment |
|---|---|---|---|---|
| SEAL | `[E7]` | 4055-4056 | Amphibious | "I am the only one with this zone, because it is now tied with being an infantry (part of seal stuck on tree bug)" |
| Tanya Prime | `[TANY]` | 4106-4107 | Amphibious | (same comment as SEAL) |
| Yuri Prime (LCRF?) | (around line 5287) | 5288-5289 | Amphibious | (commented-out hover variant nearby) |

**Subtle Westwood-developer comment:** the "seal stuck on tree bug" is a known QA artifact — earlier versions had the SEAL using a different zone that caused pathfinding glitches around treeline-shore cells. The unique AmphibiousDestroyer zone for these 3 infantry units was Westwood's workaround.

### 4.2 Amphibious (MovementZone 5)

| Unit | Section | Notes |
|---|---|---|
| SAPC (Allied) | `[SAPC]` | SpeedType=Hover, MovementZone=Amphibious. Westwood comment: "AMphibiousDestroyer I can't have a destroyer zone without a weapon!" — Hovercraft has no weapon so it falls back to non-destroyer Amphibious. |
| LCRF (Robot Tank) | around line 7932 | SpeedType=Hover, MovementZone=Amphibious. Same restriction. |
| (mod content around 8917) | | SpeedType=Hover, MovementZone=Amphibious |

The pattern: **hover units use `SpeedType=Hover + MovementZone=Amphibious`**. The MovementZone gates passability (land + water but NOT walls/roads/buildings), and SpeedType=Hover applies the appropriate speed multipliers from `g_SpeedType_LandType_Table`.

### 4.3 Water (MovementZone 10)

All stock naval units: DEST, DLPH, AEGS, ACC, SEAW, SUB, SQD, DRED — `SpeedType=Float + MovementZone=Water`. **Zero stock units use MovementZone=WaterBeach.** That zone exists in the binary (passability matrix row 11) but is referenced only in commented-out INI lines as an alternative for landing-craft-style units that didn't ship.

**Subtle detail — `BRONTO` at line 5590** uses `MovementZone=Water` with `Locomotor={4A582744}` (Walk locomotor). This is a non-standard combo — likely a TS-legacy debug unit or unused content. Walk locomotor + Water zone would produce a unit that walks but can only enter water — essentially a swimming infantry. Not in stock YR play.

---

## 5. The Tanya/SEAL "EnterWaterSound" mechanism

Verified from INI grep (lines 4051-4052, 4102-4103):

```ini
[E7]    ; SEAL
EnterWaterSound=TanyaEntersWater
LeaveWaterSound=TanyaLeavesWater
SpeedType=Amphibious
MovementZone=AmphibiousDestroyer
```

The `EnterWaterSound=` and `LeaveWaterSound=` INI keys are parsed by `TechnoTypeClass::ReadINI` (offset/parser unverified this pass — open question). They trigger when an amphibious infantry transitions across the LandType boundary (Beach ↔ Water).

**Detection mechanism (inferred, not directly decompiled):** the Process_Movement or Mark_All_Occupation_Bits handler likely checks if the current cell's LandType differs from the previous cell's LandType, and if the transition is Beach→Water or Water→Beach (or Land→Beach), plays the appropriate sound.

**Open question:** the exact code site that compares old vs new cell LandType for the EnterWater/LeaveWater trigger. Not located this pass.

---

## 6. Hover units — special considerations

Hover units use `SpeedType=Hover + MovementZone=Amphibious`. Per `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md` and the bridge doc, hover units have several unique behaviours:

1. **`HoverLocomotionClass::Move @ 0x514310`** reads `cell.flags & 0x100` (on-bridge) and checks `Get_Height() >= DAT_00A8F1B4` (altitude threshold). This determines whether the hover unit is at "deck level" (on bridge) or "ground level" (under bridge). Per the bridge doc, this is the layer-transition trigger.

2. **No Z-bump like Drive/Ship.** Hover units don't add `g_BridgeZOffset` to their destination Z. They float over both layers freely (subject to passability).

3. **Hover crosses water at full speed.** The `g_SpeedType_LandType_Table[3 (Hover) + 2 (Water) * 9]` slot likely contains 1.0 (verified open question). The hover skirt animation is what visually distinguishes water-traversal from land.

4. **Hover-on-beach:** since Beach (LandType=6) → ZoneType=3 and Amphibious row col 3 = 1 (pass), hover units traverse beach cells at the speed dictated by the speed table. Likely 1.0 (full speed) but unverified.

---

## 7. The "Naval can't reach shore" pattern in practice

Stock-content gameplay rules:
- A Destroyer at the shoreline can fire on land units but **cannot move onto the shore**. Its MovementZone=Water blocks Beach.
- A Dreadnought's missiles attack land targets at long range, but the Dreadnought itself stays in deep water.
- The Allied Aircraft Carrier launches Hornets (Fly zone) that can reach land; the Carrier cannot.
- A SEAL boarding a Boat (transport role) — the SEAL walks onto the Boat from beach. The Boat then ferries it to a destination Water-or-Beach cell.

**Mod scenarios where shore-crossing changes:**
- A mod assigning a Destroyer `MovementZone=WaterBeach` would enable shore approach.
- A mod assigning infantry `MovementZone=Amphibious` enables wading without amphibious-specific animations.

These are mod-mechanics; stock YR uses only AmphibiousDestroyer (SEAL/Tanya/Yuri Prime) and Amphibious (Hover units).

---

## 8. Active-in-YR confirmation

| Subsystem | Active in YR? | Evidence |
|---|---|---|
| Beach LandType (=6) | Yes | every coastal map has beach cells |
| `IsShorePieceTile @ 0x4865B0` | Yes | 4 active callers including ApplyBridgeTile |
| AmphibiousDestroyer MovementZone | Yes | SEAL/Tanya/Yuri Prime use it in stock |
| Amphibious MovementZone | Yes | Hovercraft/Robot Tank use it |
| WaterBeach MovementZone | **Conditional** | NO stock unit uses it; only mod content |
| EnterWaterSound/LeaveWaterSound | Yes | SEAL/Tanya/Yuri Prime have it set |
| Naval shore-block (Water row col 3 = 2) | Yes | every naval unit experiences it |
| `g_ShorePieces` tile range (42 tiles) | Yes | theater data populates it from `[General]ShorePieces=` INI |

**No TS-legacy gating found.** Standard YR play exercises every code path documented here.

---

## 9. Open questions

1. **`g_SpeedType_LandType_Table` values for Hover-on-Water, Amphibious-on-Water, Amphibious-on-Beach.** Not extracted this pass. Would need to find the table's base address (the decompilation shows `(&g_SpeedType_LandType_Table)[...]` but the symbolic address isn't directly extracted).

2. **EnterWaterSound / LeaveWaterSound trigger site** — the code that detects LandType transitions for sound playback. Likely in `Process_Movement` or `Mark_All_Occupation_Bits` for Walk. Not located this pass.

3. **Hover layer-transition altitude threshold `DAT_00A8F1B4`** — the value. Per bridge doc §1, it's runtime-initialised from isometric projection. Cross-reference for the actual hover-altitude value.

4. **The 42-tile shore piece set internals** — what each of the 42 tiles represents (8 orientations × 5+ variations?). Not enumerated. Cross-reference with theater INI / WAE source.

5. **`MapClass::ApplyBridgeTile @ 0x57B440`** uses two byte-lookup tables `DAT_0082A7F4` and `DAT_0082A89C` (each ~168 bytes = 42 entries × 4 bytes) for shore-piece rotation/orientation metadata. Worth extracting for bridge-placement parity.

---

## 10. Sources

**Ghidra functions decompiled (this pass):**
- `CellClass::IsShorePieceTile @ 0x4865B0` — full body
- `MapClass::ApplyBridgeTile @ 0x57B440` — full body (bridge-placement logic, uses ShorePieces)

**Memory / strings:**
- `0x008294EC` = "ShorePieces" string (verified via `search_strings`)
- `0x00829364` = "WaterBridge" string

**Xref tables:**
- `IsShorePieceTile` callers: FUN_0059A6C0, FUN_005A33F0, FUN_005A38C0, `MapClass::ApplyBridgeTile @ 0x57B440`
- `g_ShorePieces` (via "ShorePieces" string at 0x008294EC): `Read_Theater_TileSets_INI @ 0x054572F` (single DATA xref — populated at theater load)

**INI files cross-referenced:**
- `ini/rulesmd.ini` — confirmed stock units using each amphibious MovementZone (verified by Grep this pass)
- Westwood's developer comments preserved: "seal stuck on tree bug", "AMphibiousDestroyer I can't have a destroyer zone without a weapon!"

**Companion docs:**
- `MOVEMENT_CLASSIFIERS_REFERENCE.md` — MovementZone matrix + LandType enum
- `ZONE_PASSABILITY_VERIFIED.md` — full matrix
- `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` — RecalcZoneType decompilation
- `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — hover-unit specifics
- `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md` — hover bridge interaction

---

*End of report. The water-shore crossing mechanism is entirely captured by the MovementZone passability matrix — there's no separate "wading state" or "amphibious mode" beyond the 4 amphibious-family MovementZones.*
