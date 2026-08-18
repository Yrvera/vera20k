# TiberiumClass Queue Seeding & Two-Layer Timing Architecture

**Date:** 2026-04-03
**Method:** Ghidra MCP decompilation of gamemd.exe + YRpp header cross-reference
**Confidence:** High — all findings from direct binary analysis

---

## 1. Queue Seeding at Map Load

**Yes, initial map ore cells ARE added to both queues at map load time.**

Initialization happens during `ScenarioClass::Full_Init` via two functions:

| Function | Address | Purpose |
|----------|---------|---------|
| `FUN_00722D00` | `0x00722D00` | Growth queue alloc + init |
| `FUN_00722240` | `0x00722240` | Spread queue alloc + init |
| `FUN_007233A0` | `0x007233A0` | Populate growth queue from map cells |
| `FUN_007228B0` | `0x007228B0` | Populate spread queue from map cells |

### Population Algorithm (identical for both queues)

1. Clear queue (reset heap count, zero bitmap)
2. Iterate every cell on the map via `MapClass` cell iterator
3. For each cell:
   - Get tiberium type index via `FUN_00485010` (wraps `IsWallOverlay`)
   - Check if it matches this TiberiumClass's index (`+0x98`)
   - **Growth check** (`FUN_00483620`): `GrowthPercentage > 0`, density < max, not a wall
   - **Spread check** (`FUN_00483690`): `SpreadPercentage > 0`, density > threshold, valid for spread
4. If eligible, create entry with **time = 0.0f** (immediate eligibility)
5. Push into min-heap via sift-up

### Timing: Map Load vs Runtime

| Context | Time Value | Effect |
|---------|-----------|--------|
| **Map load** (0x007233A0/0x007228B0) | `0.0f` | All initial cells immediately eligible |
| **Runtime re-insert after growth** (inline in 0x00722F00) | `(float)(currentFrame + Random() % 50)` | 0-49 frame jitter |
| **Runtime re-insert after spread** (inline in 0x00722440) | `0.0f` | Immediately re-eligible |
| **New spread candidate from growth** (0x00722AF0) | `(float)(currentFrame + Random() % 50)` | 0-49 frame jitter |

### Queue Overflow Protection

Both init functions double as compaction/rebuild when queues near capacity:
- Growth: triggers rebuild if `entry_count > map_cell_count - (actualCount * 2)`
- Spread: triggers rebuild if `entry_count > map_cell_count - 20`

---

## 2. Two-Layer Timing Architecture

### Layer 1: Per-Type Interval Timer (Driver)

Both drivers fire execution functions periodically, **NOT every frame**.

**Spread driver** (`0x007221B0`):
- Timer fields at TiberiumClass offsets `+0x100`, `+0x104`, `+0x108`
- `+0x100` = lastFiredFrame (int, init -1)
- `+0x108` = interval = raw `Spread=` value (NO multiplier)
- Logic: `elapsed = currentFrame - lastFiredFrame; if elapsed < interval → skip`
- After firing: `lastFiredFrame = currentFrame; interval = Spread`

**Growth driver** (`0x00722C40`):
- Timer fields at `+0x11C`, `+0x120`, `+0x124`
- `+0x11C` = lastFiredFrame (int, init -1)
- `+0x124` = interval = `ftol(Growth * multiplier)`
  - multiplier = 0.3 when SpecialFlags bit 0x40 set (TiberiumGrows, default ON)
  - multiplier = 1.0 when bit 0x40 clear
- Same skip logic as spread driver

**First-fire behavior:** When `lastFiredFrame == -1`, `elapsed` is enormous (currentFrame - (-1) = currentFrame + 1), so the driver fires on the very first tick after initialization.

### Layer 2: Heap Processing (Execution)

**Critical finding: NO frame-gating on popped entries.** The execution functions pop entries and process them unconditionally. The float priority is purely an ordering key — there is NO `if (entry.priority > currentFrame) → put back` check.

**Spread execution** (`0x00722440`):
1. Exit if heap null/empty or `SpreadPercentage <= 1e-5`
2. Batch: `ftol(heapCount * SpreadPercentage)`, clamped [5, 25]
3. Actual: `Random() % batchSize + 1`
4. Pop and process `actualCount` entries:
   - Check 8 neighbors for valid spread targets
   - If cell can spread (>1 valid neighbor): **re-insert with priority = 0.0**
   - If cannot spread: remove from bitmap
5. Successfully spread cells: call `FUN_00487190(type, 3)` (place density 3)

**Growth execution** (`0x00722F00`):
1. Exit if heap null/empty or `GrowthPercentage <= 1e-5`
2. Batch: `ftol(heapCount * GrowthPercentage)`, clamped [5, 50]
3. Actual: `Random() % batchSize + 1`
4. Pop and process `actualCount` entries:
   - If density < 11: grow (+1), **re-insert with priority = (float)(currentFrame + Random() % 50)**
   - If density >= 11: remove from bitmap (fully grown)
   - After growing: call `FUN_00722AF0` → add to **spread** queue with priority `currentFrame + Random() % 50`

### Priority Differences Between Spread and Growth

| Aspect | Spread Heap | Growth Heap |
|--------|------------|-------------|
| Map load init | priority = 0.0 | priority = 0.0 |
| Re-insert after processing | priority = **0.0** | priority = **currentFrame + Random(0..49)** |
| New candidate from growth | priority = currentFrame + Random(0..49) | N/A |
| Frame-gating on pop? | **NO** | **NO** |

### Implications for Implementation

The priority values create different behavioral patterns:

**Growth heap:** Recently-grown cells go to the back (higher frame number = higher priority = popped later). This creates natural stagger — cells that were just grown won't be grown again immediately even within the same batch.

**Spread heap:** Re-inserted cells always get priority 0.0, so they stay near the front. But since the driver only fires once per `Spread=2200` frames, re-ordering within the heap between firings doesn't matter much. The randomization for spread comes from batch size jitter and random direction choice, not priority ordering.

**Net behavior:** Growth processes ~5-50 cells every ~660 frames (with 0.3x), each cell getting random 0-49 frame jitter for its next growth. Spread processes ~5-25 cells every ~2200 frames, no jitter between re-processing. The combined effect is organic-looking ore expansion without any per-frame overhead.

---

## 3. Queue Structure Layout

### Heap Entry (8 bytes)
```
+0x00: cell_coord (u32, packed CellStruct)
+0x04: priority (f32, ordering key)
```

### Per-Type Queue State
**Spread queue** (TiberiumClass offsets):
- `+0xF0`: entry count (int, number of cells tracked in parallel array)
- `+0xF4`: min-heap pointer (5-field: count, capacity, array_ptr, max, min)
- `+0xF8`: cell-in-queue bitmap pointer (1 byte per map cell)
- `+0xFC`: entries array pointer (8 bytes each)

**Growth queue** (TiberiumClass offsets):
- `+0x10C`: entry count
- `+0x110`: min-heap pointer
- `+0x114`: cell-in-queue bitmap pointer
- `+0x118`: entries array pointer

---

## 4. Key Function Address Summary

| Address | Name | Purpose |
|---------|------|---------|
| `0x007221B0` | Spread driver | Per-tick, fires FUN_00722440 every Spread frames |
| `0x00722C40` | Growth driver | Per-tick, fires FUN_00722F00 every Growth*0.3 frames |
| `0x00722440` | Spread execution | Batch-process spread queue entries |
| `0x00722F00` | Growth execution | Batch-process growth queue, feeds spread |
| `0x007233A0` | Growth queue rebuild | Populate from map cells (time=0) |
| `0x007228B0` | Spread queue rebuild | Populate from map cells (time=0) |
| `0x007235A0` | Growth queue add | Runtime insert with jitter |
| `0x00722AF0` | Spread queue add | Runtime insert with jitter |
| `0x00483620` | Can cell grow? | GrowthPercentage check |
| `0x00483690` | Can cell spread? | SpreadPercentage + density threshold |
| `0x00722D00` | Growth alloc+init | ScenarioClass::Full_Init |
| `0x00722240` | Spread alloc+init | ScenarioClass::Full_Init |
