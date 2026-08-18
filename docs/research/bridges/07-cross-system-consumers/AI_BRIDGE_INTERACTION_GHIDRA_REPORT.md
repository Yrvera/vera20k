# AI Bridge Interaction & Targeting — Ghidra Research Report

**Target:** AI_BRIDGE_INTERACTION_TARGETING  
**Date:** 2026-05-19  
**Scope:** Does the AI in gamemd.exe specifically interact with bridges (target, repair, avoid, or route preferentially)?  
**Verdict:** VERIFIED NEGATIVE — No AI-specific bridge interaction exists in gamemd.exe.

---

## Executive Summary

A systematic Ghidra investigation covering the full AI subsystem (script opcode table, HouseClass AI functions, TeamClass targeting, AITriggerTypeClass, A* pathfinding, and all bridge-related strings/xrefs) found **no code path in the AI that specifically targets, prioritizes, repairs, or reacts to bridges or bridge huts (CABHUT)**. The AI treats bridge huts as ordinary buildings and bridge cells as ordinary terrain (with only generic movement-cost adjustments shared with all units).

---

## Investigation Method

1. String search for all bridge-related literals: `Bridge*`, `CABHUT`, `BridgeRepairHut`, `DestroyableBridges`, `BridgeDestruction`
2. Traced xrefs from every bridge string into containing functions, then categorized each function.
3. Enumerated the complete AI script opcode dispatch table (`TeamClass__Recruit_Or_Add` @ `0x006E9380`), opcodes 0x00–0x40 (64 total).
4. Decompiled all HouseClass AI functions: `AI_Tick`, `AI_BuildThreatMap`, `AI_Building_Strategy`, `AI_FindBestRallyTarget`, `AI_FindInfantryTarget`, `AI_FindAirTarget`, `AI_Choose_Building`.
5. Decompiled `TeamClass__Find_Best_Target_Building` (@ `0x006EEBD0`).
6. Examined `TechnoClass__ThreatAvoidance_Modifier` (@ `0x006F79A0`).
7. Examined A* edge cost function `AStar_compute_edge_cost` (@ `0x00429830`).
8. Examined `PathfinderClass__UpdateBridgePassability` (@ `0x0042ACF0`) and `MapClass__ResolvePathCoord_BridgeAware` (@ `0x00583180`).

---

## Findings by Category

### (a) Script Opcode Table — No Bridge Opcodes

**Verified via:** `decompile_function 0x006E9380` (TeamClass__Recruit_Or_Add — the sole script opcode dispatcher)

The complete opcode table (0x00–0x40, 64 opcodes) was enumerated. **No opcode names, parameters, or function calls reference bridges.** The full opcode list:

| Opcode | Action |
|--------|--------|
| 0x00 | Patrol |
| 0x01 | Attack Building |
| 0x03 | Move To Cell |
| 0x04 | Move to waypoint |
| 0x05 | Wait (timer) |
| 0x06 | Jump to script index |
| 0x07 | Flag To Win |
| 0x08–0x0B | Movement/attack variants |
| 0x0C | (formation flag) |
| 0x0D | Set formation |
| 0x0E | Unknown movement |
| 0x0F | Follow Target |
| 0x10 | Move |
| 0x11 | Load new script |
| 0x12 | Create new team |
| 0x13–0x15 | Unit order / house calls |
| 0x16 | Move to rally point |
| 0x17 | Flag To Lose |
| 0x18 | (fall-through complete) |
| 0x19 | Play sound |
| 0x1A–0x1B | Unknown |
| 0x1C | Reduce tiberium around unit |
| 0x1D–0x1E | Set unit state fields |
| 0x1F | Self-repair fire |
| 0x20 | Start Lightning Storm |
| 0x21 | Stop Lightning Storm |
| 0x22 | Para-drop at position |
| 0x23 | Reset Shroud |
| 0x24 | Reveal Entire Map |
| 0x25 | Unknown unit action |
| 0x26–0x28 | Super-weapon checks |
| 0x29 | vtable 0x51C unit action |
| 0x2A | FUN_006EDCA0 (movement?) |
| 0x2B | Wait for tech level |
| 0x2C–0x2D | Load TRUCKB/TRUCKA type |
| 0x2E | Attack Nearest |
| 0x2F | Attack Nearest v2 |
| 0x30 | Attack Production |
| 0x31 | Mark team complete |
| 0x32 | Set speed flag |
| 0x33 | FUN_006EF610 |
| 0x34 | FUN_0070F120 (Iron Curtain?) |
| 0x35 | Attack Move |
| 0x36 | Random Move |
| 0x37 | Super Launch Single SW |
| 0x38 | Super Launch Dual SW |
| 0x39 | Super Launch Dual SW v2 |
| 0x3A | Attack Farthest |
| 0x3B | Attack Building v2 |
| 0x3C–0x40 | Unit state/mission checks |

**Conclusion:** No "attack bridge," "repair bridge," or "destroy bridge" opcode exists.

---

### (b) HouseClass AI Target Scoring — No CABHUT Priority

**Verified via:** `decompile_function 0x00509400` (AI_BuildThreatMap), `decompile_function 0x0050CBF0` (AI_FindBestRallyTarget), `decompile_function 0x00509F60` (AI_FindInfantryTarget), `decompile_function 0x0050A150` (AI_FindAirTarget), `decompile_function 0x004FE3E0` (AI_Choose_Building), `decompile_function 0x004FD500` (AI_Building_Strategy)

None of these functions check building type flags for bridge huts. `AI_BuildThreatMap` scores by unit tech-level, house membership, and position — no bridge-cell filter. `AI_FindBestRallyTarget` scores buildings by distance and factory state — no bridge hut weighting. `TeamClass__Find_Best_Target_Building` (@ `0x006EEBD0`) uses a 4-mode scoring system: closest-by-slope-cost (0,1), closest-by-Euclidean-distance (2,3) — no type filter for CABHUT.

**Conclusion:** CABHUT is not weighted differently from any other building for AI target selection.

---

### (c) Pathfinding — Bridge Logic is Movement-Only, Not AI Targeting

**Verified via:** `decompile_function 0x00429830` (AStar_compute_edge_cost), `decompile_function 0x0042ACF0` (PathfinderClass__UpdateBridgePassability), `decompile_function 0x00583180` (MapClass__ResolvePathCoord_BridgeAware)

Bridge-aware logic exists in the pathfinder but is purely movement mechanics:

- **AStar_compute_edge_cost** applies a 4× cost multiplier (`g_BridgeApproach_CostMult_4_0` at `0x007E37BC`) for cells with bridge flag `0x40000`, and bridge diagonal edges get a 10× or 2× factor. This is a movement penalty shared equally by all units regardless of AI vs human control.
- **PathfinderClass__UpdateBridgePassability** temporarily toggles `CellClass+0x140 & 0x40000` as a per-A* bridge-approach cost marker. The checked consumer is `AStar_compute_edge_cost`, which applies a 4× cost multiplier; this is cost-only movement steering, not an allow/deny destroyed-bridge passage gate.
- **MapClass__ResolvePathCoord_BridgeAware** adjusts sub-cell targeting coordinates when a unit is on a bridge tile.

None of these functions contain AI-side target-selection logic. They fire identically for human-controlled and AI-controlled units.

**Conclusion:** Bridge-aware pathfinding exists but is entirely in the movement layer, not the AI targeting layer. No AI preferential routing over/under bridges beyond what the cost multipliers give every unit.

---

### (d) CABHUT String — Used Only in Map Generation, Not AI Targeting

**Verified via:** `get_xrefs_to 0x0082BA00` (CABHUT string), `get_xrefs_to 0x0081A898` (BridgeRepairHut string)

- `CABHUT` (@ `0x0082BA00`) has exactly **one xref**: into `FUN_005904B0` which is called from `FUN_0058F2C0` — a map generation function that places bridge hut buildings on clear tiles during map setup. This is scenario initialization code, not AI tick code.
- `BridgeRepairHut` (@ `0x0081A898`) has exactly **one xref**: `BuildingTypeClass_ReadINI_Water` (@ `0x0045FE50`). This function reads the `BridgeRepairHut=` boolean INI key and stores it at `BuildingTypeClass+0x16B6`. This flag marks whether a building IS a bridge hut — it is used for bridge repair mechanics (triggering `MapClass__RepairBridge_*`) not for AI prioritization. Confirmed by `decompile_function 0x0045FE50`.

**Conclusion:** Both bridge-hut identifiers appear only in INI parsing and map-generation code paths; neither is referenced from any AI function.

---

### (e) Bridge Destruction/Repair — Triggered by Damage, Not AI Decision

**Verified via:** `decompile_function 0x00577920` (MapClass__UnregisterBridgeRepairHut), `get_xrefs_to 0x0057A0C0` (MapClass__MarkBridgesForRepair_High), `get_xrefs_to 0x00578E60` (MapClass__MarkBridgesForRepair_Low)

`MapClass__MarkBridgesForRepair_High/Low` are called from bridge-state update functions (`FUN_0059A6C0`, `FUN_0059BBC0`, etc.), which are invoked on damage events — not from any AI targeting decision. The AI has no code to "order engineers to repair bridge" or "designate bridge as attack objective."

The `EVA_BridgeRepaired` string (@ `0x00825538`) is purely a sound event.

---

### (f) DestroyableBridges INI Flag — Not AI-Gated

**Verified via:** `decompile_function 0x006B8CA0`, `decompile_function 0x006B8B30`

`DestroyableBridges` is bit 15 of the `SpecialFlags` field, read/written in `FUN_006B8CA0` and `FUN_006B8B30`. When `DestroyableBridges=no`, bridge cells resist damage — this affects all damage pathways uniformly and is not an AI-specific switch.

---

## Verified-Negative Finding (High Confidence)

**The AI in gamemd.exe has no bridge-specific targeting, scoring, routing preference, repair directive, or script opcode.** This was verified by:

1. Full enumeration of 64 script opcodes — zero bridge opcodes (verified via `decompile_function 0x006E9380`)
2. Decompiling all 6 HouseClass AI target-scoring functions — zero CABHUT/bridge checks
3. Checking all xrefs to both bridge-hut identifier strings — both strings used only in INI parsing and map-gen
4. Reviewing A* edge cost and pathfinder bridge functions — all are movement-layer, not AI-targeting-layer

**Confidence: HIGH** for absence of bridge-specific AI targeting. No YR-live bridge AI path was found after exhaustive search of all AI subsystems named in the brief.

---

## TS-Legacy Filter

No bridge-related AI code was found at all, so the TS-vs-YR filter is not material here. The bridge interaction that does exist (movement cost multipliers, damage-triggered repair) is live in YR but is not AI-specific.

---

## Implications for Rust Port

The Rust AI port does NOT need:
- Any script opcode for bridge attack/repair
- CABHUT weighting in building target selection
- Bridge-aware AI routing beyond the 4× movement cost already in A*
- Any AI-triggered bridge repair event

The only bridge-related behavior that must be faithfully ported is in the **movement layer** (A* cost multipliers for bridge approach cells, bridge passability updates on bridge destruction), not in any AI subsystem.
