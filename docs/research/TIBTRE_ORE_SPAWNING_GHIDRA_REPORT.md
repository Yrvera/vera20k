# TIBTRE Ore Spawning System - Ghidra Research Report

## Confidence: HIGH (verified from binary)

## Executive Summary

TIBTRE terrain objects (tiberium trees) **actively spawn ore during normal YR skirmish gameplay**.
This is NOT TS legacy code. Every YR skirmish map contains TIBTRE objects (verified: 41/41
available maps have them, ranging from 2 to 38 per map). The system is live, unconditional,
and not gated behind any SpecialFlags or opt-in settings.

This is a major missing gameplay system in the Rust engine. TIBTRE trees are the primary
mechanism for ore regeneration near starting positions and gem/ore patches. Without this
system, maps will eventually run out of ore, fundamentally changing game economics.

---

## Key Functions

| Address      | Name                         | Role                                          |
|-------------|------------------------------|-----------------------------------------------|
| 0x0071C730  | TerrainClass::AI             | Per-tick update for terrain objects            |
| 0x00483780  | CellClass::SpreadTiberium    | Spreads ore to an adjacent cell               |
| 0x004838E0  | CellClass::CanAcceptTiberium | Checks if a cell can receive ore              |
| 0x00487190  | CellClass::PlaceTiberium     | Creates/grows ore overlay on a cell           |
| 0x0071DEA0  | TerrainTypeClass::ReadINI    | Parses SpawnsTiberium, IsAnimated, etc.       |

## TerrainTypeClass INI Fields (verified offsets)

| Byte Offset | Field                | Type   | TIBTRE Value |
|-------------|---------------------|--------|--------------|
| 0x2A0       | AnimationRate       | int    | 3            |
| 0x2A4       | AnimationProbability| float  | 0.003        |
| 0x2B1       | SpawnsTiberium      | bool   | true         |
| 0x2B2       | IsFlammable         | bool   | false        |
| 0x2B3       | IsAnimated          | bool   | true         |

Note: `param_1` in TerrainTypeClass functions is `int*` (pointer), and field accesses
use the cast `(int)param_1 + offset`, making these direct byte offsets.

## TerrainClass::AI Logic (0x0071C730)

### Step 1: Animation Probability Check (every tick)

```
if (IsAnimated && animation_not_playing) {
    random = Random::Next() % 1000000;
    if ((random * 1.0e-6) < AnimationProbability) {
        start_animation(AnimationRate);
    }
}
```

- `AnimationProbability = 0.003` means 0.3% chance per tick
- At 15 FPS (normal speed): animation starts roughly every ~22 seconds on average
- The constant 1.0e-6 (at address 0x007EF918) normalizes the random value to [0.0, 1.0)

### Step 2: Frame Advancement

When animation is playing (timer > 0):
- Each time the CDTimer expires (every `AnimationRate` = 3 frames)
- Current frame advances by a step field (`param_1[0x31]`, byte offset 0xC4) —
  typically 1, but stored in TerrainClass state, not hardcoded
  (corrected 2026-05-29: was "increments by 1"; binary: `param_1[0x2b] += param_1[0x31]` —
  ROOT_CAUSE: INFERENCE_HARDENED; via `decompile_function 0x0071C730`)

### Step 3: Ore Spawn Trigger

```
if (SpawnsTiberium && IsAnimated) {
    if (current_frame == total_shp_frames / 2) {
        // Reset animation completely
        current_frame = 0;
        animation_timer = 0;
        
        // Get the cell where this terrain object sits
        cell = CellClass::GetCellAt(this->coordinates);
        
        // Spread tiberium to an adjacent cell (force=true)
        cell->SpreadTiberium(true);
    }
}
```

The ore spawns at the **midpoint** of the animation cycle. The animation is then reset,
so it must win the probability check again before the next spawn.

## CellClass::SpreadTiberium (0x00483780)

When called with `force=true` (param_2 = 1) from TerrainClass::AI:

1. **Determine tiberium type**: Calls `CellClass::OverlayToTiberiumIndex()` to get
   tiberium type index from existing overlay. (corrected 2026-05-29: was described as
   `IsWallOverlay()` (misnamed); binary label is `CellClass__OverlayToTiberiumIndex` —
   ROOT_CAUSE: RTTI_LABEL_DRIFT; via `decompile_function 0x00483780`)
   If no tiberium overlay exists on the TIBTRE cell, returns -1, which is then defaulted
   to **type 0 = Riparius (green ore)**.

2. **Find valid adjacent cell**: Iterates all 8 neighbors in random order (random start
   direction via `Random::RandomRanged(0,7)`, tries all 8). For each cell, calls
   `CellClass::CanPlaceTiberium` (corrected 2026-05-29: was `CellClass::CanAcceptTiberium`;
   binary label at 0x004838E0 is `CellClass__CanPlaceTiberium` — ROOT_CAUSE:
   RTTI_LABEL_DRIFT; via `get_function_by_address 0x004838E0`).

3. **Place ore**: Calls `CellClass::PlaceTiberium(tib_type, 3)` on the first valid cell.
   - Creates a new ore overlay if the cell has none
   - Increases density by 3 if ore already exists (Branch A — gated on
     `ScenarioClass+0x34A6 != 0`; corrected 2026-05-29: gate was missing from doc;
     binary: `if (*(char *)(g_ScenarioClass_Instance + 0x34a6) == '\0') return 0;` —
     ROOT_CAUSE: INFERENCE_HARDENED; via `decompile_function 0x00487190`)
   - Selects a random overlay frame variant

### CanPlaceTiberium checks (0x004838E0):
(corrected 2026-05-29: section heading was `CanAcceptTiberium`; binary label is
`CellClass__CanPlaceTiberium` — ROOT_CAUSE: RTTI_LABEL_DRIFT; via
`get_function_by_address 0x004838E0`)
- Cell must be within playfield
- Cell must not have blocked flags (0x500 in cell flags at +0x140)
- Cell must not have an existing BuildingClass (RTTI type 6) with health > 0 on it
  (unless building-type +0xC9A or +0x1701 is set)
- Cell must not already have a TerrainClass object (RTTI type 0x24) with
  SpawnsTiberium(+0x2B1)=true
- Cell's land type must permit tiberium (`DAT_0089ea60[LandType * 0x24] != 0`)
- Cell overlay index must be -1 (empty) and cell slope (+0x11C) must be 0 (flat)
- IsoTileType +0x306 must permit tiberium

### Critical: TiberiumSpreads flag is BYPASSED

The `force=true` parameter causes `SpreadTiberium` to skip the `TiberiumSpreads`
SpecialFlags check (bit 7 of `*g_ScenarioClass_Instance`, i.e., `& 0x80` on the first
byte of the ScenarioClass instance). (corrected 2026-05-29: was "bit 7 of
DAT_00a8b230"; binary reads `*g_ScenarioClass_Instance & 0x80`, not DAT_00a8b230 —
ROOT_CAUSE: INFERENCE_HARDENED; via `decompile_function 0x00483780`)
This means TIBTRE ore spawning is **unconditional** and independent of game settings.
Even if a map or game mode disables TiberiumSpreads, TIBTRE trees will still spawn ore.

## Ore Type Spawned

When the TIBTRE cell has no existing ore overlay (the common case), the tiberium type
defaults to **index 0 = Riparius** (standard green ore in YR). The initial density is 3
(on a 0-11 scale where higher = more ore value).

## Timing Analysis

For TIBTRE01/02/03:
- `AnimationProbability = 0.003` (0.3% per tick)
- Average ticks between animation starts: ~333 ticks
- At 15 FPS: ~22 seconds between animation starts
- Animation plays for `(total_frames / 2) * AnimationRate` frames to reach midpoint
- Ore spawn happens at the animation midpoint, then animation resets

The exact spawn interval depends on the SHP frame count. If TIBTRE01 has N frames:
- Time to reach midpoint: (N/2) * 3 frames = 1.5N frames
- At 15 FPS: 0.1N seconds per animation cycle
- Average time between ore spawns: 22 + 0.1N seconds

## TiberiumSpreadRadius / AnimClass Path (SEPARATE SYSTEM)

The `TiberiumSpreadRadius` INI key (AnimTypeClass offset 0x33C) and `TiberiumSpawnType`
(offset 0x338) are part of a **completely different system** in AnimClass::AI (0x0042413B).
That system handles meteorite-style tiberium deposition from falling animations. It is
NOT related to TIBTRE terrain object ore spawning.

## Map Coverage

**All 41 available YR map files contain TIBTRE objects:**

Maps with highest counts: PowdrKeg (38), GoldSt (25), invasion (25), Death (24),
Roulette (24), BayOPigs (20), Pacific (20).

Maps with lowest counts: HailMary (2), NewHghts (2), xmas (2).

Average: ~11 TIBTRE objects per map.

## Summary: Is This Active in Standard YR?

**YES, absolutely.** This is:
1. Present on every skirmish map
2. Not gated behind any SpecialFlags
3. Called every tick through TerrainClass::AI (virtual dispatch from game loop)
4. Uses standard probability/animation timing, not debug or cheat code
5. The primary mechanism for ore regeneration near starting positions

The Rust engine currently has NO implementation of this system. TIBTRE objects are parsed
from maps and rendered, but they never spawn ore. This is a significant gameplay omission
that affects game economics, balance, and long-game viability.

## Implementation Notes for Rust Engine

Required components:
1. **TerrainTypeClass data**: Parse `SpawnsTiberium`, `IsAnimated`, `AnimationRate`,
   `AnimationProbability` from rules(md).ini
2. **TerrainClass simulation entity**: Track current animation frame, timer state
3. **Per-tick AI**: Probability check -> animation advancement -> ore spawn at midpoint
4. **Ore placement**: Spread to random adjacent cell with density 3, type = Riparius (or
   existing cell's tiberium type)
5. **Cell validation**: Check passability, no existing TIBTRE, no building, within bounds

Key design considerations:
- Must use fixed-point math for deterministic simulation (probability check)
- Must use the game's RNG for lockstep correctness
- The animation probability (0.003) and animation rate (3) are per-TerrainType from INI,
  not hardcoded constants
- The force=true bypass of TiberiumSpreads must be preserved
