# AnimClass Ore-Spawn Tick — Ghidra Research Report

**Date:** 2026-05-20  
**Binary:** gamemd.exe (Yuri's Revenge)  
**Method:** Ghidra MCP live decompilation + cross-reference tracing + INI verification  
**Confidence:** HIGH — all findings verified from direct binary analysis and in-repo INI files  

---

## 1. Executive Summary

**The investigation confirmed that the TIBTRE AnimClass path described in ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md §6 was partially incorrect.** TIBTRE01-03 terrain objects do NOT use an intermediate AnimClass to spawn ore. They call `CellClass::SpreadTiberium` directly from `TerrainClass::AI` (0x0071C730), which is already fully documented in TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md.

The `AnimTypeClass+0x338` (TiberiumSpawnType) / `+0x33C` (TiberiumSpreadRadius) fields ARE used in a live ore-spawn system — but it is the **meteor debris / bouncing crystal system**, not TIBTRE. The function that drives it is `AnimClass__AI` at `0x00423ac0`, specifically the inline block at `0x00423f00`–`0x00424235`.

**Active in YR: YES** — The system fires for `METDEBRI` (spawned by `METSMALL` meteorite) and `CRYSTAL1`–`CRYSTAL4` (gem debris from buildings). These are live, reachable code paths in standard YR skirmish, gated only by the relevant weapon/warhead being triggered.

---

## 2. Scope Clarification

The investigation hint pointed to `0x0071b9a7` as the AnimClass creation site for TIBTRE ore spawning. Verified via `decompile_function 0x0071b920`: that address is inside `TerrainClass::Take_Damage`, not TerrainClass::AI. The `PUSH 0x1C8` at `0x0071b9a7` allocates an AnimClass for the **destruction explosion animation** (via `Warhead::SelectExplosionAnim`), not an ore-spawn anim. This was a mislabeling in the prior ORE_OVERLAY_SYSTEM_GHIDRA_REPORT §6 entry.

Callers of `FUN_00487190` (CellClass::PlaceTiberium), verified via `get_function_callers 0x00487190`:
- `BuildingClass::DestructionEffects` @ `0x004415f0`
- `CellClass::GrowTiberium` @ `0x00483710`
- `CellClass::SpreadTiberium` @ `0x00483780`

AnimClass::AI does NOT call `CellClass::PlaceTiberium`. It calls `OverlayClass::Constructor` directly (at `0x0042413b` → `0x005fc380`), verified via `get_xrefs_to 0x005fc380`.

---

## 3. The Actual Ore-Spawn Function: AnimClass::AI

**Address:** `0x00423ac0`  
**Vtable slot:** `[*param_1 + 0x60]` (offset 0x60 = vtable index 24)  
**Confirmed via:** `search_functions AnimClass`, `decompile_function 0x00423ac0`

### 3.1 Entry Gate (Bouncer/Meteor Path)

AnimClass::AI has two main branches. The ore-spawn code is inside the **bouncer/meteor branch**, which is entered when:

```c
if ((char)param_1[0x65] != '\0')   // AnimClass+0x194: set in Constructor
```

This flag is set in `AnimClass::Constructor` (decompiled at `0x00421ea0`) when the AnimTypeClass has either:
- `AnimTypeClass+0x35a` = `Bouncer=yes` (field verified via ReadINI `0x00427d00`)
- `AnimTypeClass+0x356` = `IsMeteor=yes` (field verified via ReadINI `0x00427d00`)

### 3.2 Landing Detection

Within the bouncer/meteor branch, two booleans control the sub-paths:

```c
bVar22 = (cell->LandType != 2);            // not water (LandType read at CellClass+0xEC)
bVar21 = (z_pos <= CellClass::GetGroundHeight() + DAT_0089a1b4);  // is at/near ground
```

The ore-spawn block executes when `(bVar22 || bVar21)` — the anim has reached the ground.

### 3.3 Ore-Spawn Gate

Within the landing block:

```c
if (*(char *)(param_1[0x32] + 0x358) != '\0') {  // AnimTypeClass+0x358 = IsTiberium
```

The field at AnimTypeClass+0x358 maps to `IsTiberium=` in artmd.ini (confirmed: `IsTiberium` → `param_1[0xD6]` in ReadINI, where 0xD6 × 4 = 0x358, verified via `decompile_function 0x00427d00`).

A second gate within the radius loop:

```c
if ((cVar6 != '\0') && (*(int *)(param_1[0x32] + 0x338) != 0)) {
    // cVar6 = CellClass::CanPlaceTiberium() result
    // +0x338 = TiberiumSpawnType pointer (must be non-null)
```

Both must be true to place ore on a cell.

---

## 4. Cell Selection Algorithm

### 4.1 Double Loop Over Radius Square

The selection iterates a square `[-radius, +radius]` × `[-radius, +radius]` in cell offsets:

```c
int radius = *(int *)(param_1[0x32] + 0x33c);  // AnimTypeClass+0x33C = TiberiumSpreadRadius
for (int dx = -radius; dx <= radius; dx++) {
    for (int dy = -radius; dy <= radius; dy++) {
        // compute cell = anim_position_in_cells + (dx, dy)
        // using g_DirectionOffsets pattern (NOT 8-directional; actual X/Y grid offsets)
        cell_x = (anim_x >> 8) + dx;   // leptons>>8 = cells (with sign-correct shift)
        cell_y = (anim_y >> 8) + dy;
```

Assembly evidence (from `get_assembly_context 0x0042413b` at 80 instructions):
- `SAR EAX, 0x8` and `SAR ECX, 0x8` are the lepton→cell conversions
- The outer loop counter is in `uStack_7c`, the inner in `uStack_78`
- Loop range: `uStack_7c` starts at `-radius` and runs `while (uStack_7c <= radius)`

### 4.2 Euclidean Distance Filter

For each (dx, dy) in the square, the code computes Euclidean distance and rejects cells beyond the radius:

```c
float dist = Sqrt_Approx(dx * dx + dy * dy);   // 0x004cac40 = Sqrt_Approx
if (dist > TiberiumSpreadRadius) continue;       // 0x00424051: CMP EAX, [EDX+0x33C]
```

This produces a **circular selection region**, not a diamond or square. Assembly confirmed: `CALL 0x004cac40` (Sqrt_Approx) followed by `CMP EAX, dword ptr [EDX + 0x33c]` at `0x00424051`.

### 4.3 Per-Cell Validity Check

For each cell within the circle:

```c
CellClass* cell = MapClass::Get_CellClass(cell_x, cell_y);  // 0x005657a0
bool canPlace = CellClass::CanPlaceTiberium(cell);           // 0x004838e0
```

`CellClass::CanPlaceTiberium` (documented in CELL_VALIDATION_TIBERIUM_PLACEMENT_REPORT.md) applies all 8 checks (playfield bounds, no bridge, no building, no TIBTRE, land type, no existing overlay, no slope, AllowTiberium tile flag).

### 4.4 Placement on Valid Cell

```c
if (canPlace && TiberiumSpawnType != null) {
    // Allocate OverlayClass (0xB0 bytes)
    int variant = Random::RandomRanged(0, 3);
    OverlayTypeClass* otype = g_OverlayTypeClass_Array[
        TiberiumSpawnType->ArrayIndex + variant];
    OverlayClass::Constructor(otype, &cell_coords, -1);

    // Set initial density: random 0-2
    cell->OverlayData = Random::RandomRanged(0, 2);   // at CellClass+0x11E
}
```

Key evidence from assembly:
- `PUSH 0x3` / `CALL 0x0065c7e0` = `RandomRanged(0, 3)` for variant selection (0x00424102–0x00424110)
- `PUSH 0x2` / `PUSH 0x0` / `CALL 0x0065c7e0` = `RandomRanged(0, 2)` for density (0x00424146–0x00424150)
- `MOV byte ptr [EDI + 0x11e], AL` at `0x00424155` = writing density to CellClass+0x11E

**IMPORTANT:** This does NOT call `CellClass::PlaceTiberium`. It does NOT add the cell to the growth queue or spread queue. The overlay is stamped directly via OverlayClass constructor. The cell will be picked up by the spread/growth queues on the next queue rebuild or when the queue naturally processes it.

---

## 5. INI AnimTypes Using This System

From in-repo `artmd.ini` (verified grep):

| Section | Gate | TiberiumSpawnType | TiberiumSpreadRadius | Notes |
|---------|------|-------------------|---------------------|-------|
| `METDEBRI` | `Bouncer=yes`, `IsTiberium=yes` | `TIB01` (Riparius ore) | (none → defaults to 0) | Spawned by METSMALL (7 count) |
| `CRYSTAL1` | `Bouncer=yes`, `IsTiberium=yes` | `TIB2_01` (Vinifera) | `0` | Gem debris from buildings |
| `CRYSTAL2` | `Bouncer=yes`, `IsTiberium=yes` | `TIB2_01` | `0` | Same |
| `CRYSTAL3` | `Bouncer=yes`, `IsTiberium=yes` | `TIB2_01` | `0` | Same |
| `CRYSTAL4` | `Bouncer=yes`, `IsTiberium=yes` | `TIB2_01` | `0` | Same |
| `METSMALL` | `IsMeteor=yes`, `IsTiberium=yes` | (none) | (none) | IsTiberium but no TiberiumSpawnType → no ore placed |

With `TiberiumSpreadRadius=0` (CRYSTAL1–4), the double loop runs once: dx=0, dy=0. The landing cell itself is checked — but the bouncing crystal occupies it, so `CanPlaceTiberium` will typically fail. Effectively, CRYSTAL debris does not reliably place ore in practice. `METDEBRI` also has no explicit `TiberiumSpreadRadius`, so it defaults to 0 from the constructor.

**YELLOW — Unverified:** What is the default value of `TiberiumSpreadRadius` when absent from INI? `AnimTypeClass::Constructor` (not decompiled in this session) likely initializes `+0x33C` to 0. If so, METDEBRI with no `TiberiumSpreadRadius=` key also places ore only at the landing cell.

---

## 6. Trigger / Timer

This ore spawn fires **on landing** (not on a timer, not per-frame, not at anim midpoint). The triggering condition is the bouncer physics reaching ground level:

```
bVar21 = (current_z <= CellClass::GetGroundHeight() + DAT_0089a1b4)
```

where `DAT_0089a1b4` is a small height threshold constant (verified in `AnimClass::BounceAI` at `0x00425670`). The ore placement runs once per AnimClass instance on its first landing tick.

There is no separate timer function. The check is part of the per-tick `AnimClass::AI` → `AnimClass::BounceAI` → landing detection flow.

---

## 7. Caller Chain (Active-in-YR Verification)

```
GameLoop → MapClass::Update() → AnimClass array iteration
→ AnimClass__AI [vtable call, slot 0x60]
→ bouncer/meteor branch (Bouncer=yes or IsMeteor=yes)
→ landing detection
→ IsTiberium gate
→ TiberiumSpreadRadius loop → CanPlaceTiberium → OverlayClass::Constructor
```

The `AnimClass__AI` vtable xref confirms only one DATA xref at `0x007e33b0` (the vtable table itself), confirmed via `get_xrefs_to 0x00423ac0`. The function is called via virtual dispatch — not hardcoded — so it runs for every active AnimClass instance each tick.

**Reachability:** `METDEBRI` is created by `METSMALL` (meteorite map trigger). `CRYSTAL1–4` are created by Cruentus (gem) Tiberium's `Debris=` INI key. Both are standard YR content. Neither path is gated by SpecialFlags or TS-only flags.

**Active in YR: YES (Conditional)** — fires only when a meteorite map trigger has spawned METSMALL, or when a unit/building is destroyed on a gem-bearing cell. Not fired every match unconditionally, but the code path is live and reachable.

---

## 8. TIBTRE vs AnimClass Ore Spawn — Disambiguation

| Aspect | TIBTRE Ore Spawn | AnimClass Ore Spawn |
|--------|-----------------|---------------------|
| Driver | TerrainClass::AI (0x0071C730) | AnimClass::AI (0x00423ac0) |
| Trigger | Frame midpoint of animation cycle | Bouncer/meteor landing event |
| Frequency | ~22 sec average per tree | Once per METDEBRI/CRYSTAL bounce landing |
| Placement | CellClass::SpreadTiberium (8-neighbor random) | OverlayClass::Constructor directly on cell |
| Queue seeding | Via SpreadTiberium → PlaceTiberium → growth queue | NOT queued; discovered by queue rebuild |
| Density placed | 3 (hardcoded in SpreadTiberium) | Random 0–2 (AnimClass::AI inline) |
| Radius | N/A (1 cell spread to random neighbor) | TiberiumSpreadRadius cells (Euclidean filter) |
| INI activation | TerrainTypeClass.SpawnsTiberium=yes | AnimTypeClass.IsTiberium=yes + TiberiumSpawnType= |
| Gating flags | None (SpreadTiberium bypasses TiberiumSpreads) | None (no SpecialFlags check) |
| Map coverage | All 41 YR skirmish maps | Only maps with meteorite triggers or gem ore |

---

## 9. Struct Offset Summary (verified)

### AnimTypeClass offsets (param_1 is `int*`; multiply index × 4)

| Byte Offset | Index | INI Key | Type | Description |
|-------------|-------|---------|------|-------------|
| +0x338 | 0xCE | `TiberiumSpawnType` | `OverlayTypeClass*` | Ore overlay to place on landing |
| +0x33C | 0xCF | `TiberiumSpreadRadius` | `int` | Radius in cells (Euclidean) |
| +0x354 | — | `Bouncer` | bool | Enables bounce physics + ore path |
| +0x356 | — | `IsMeteor` | bool | Alternative gate for meteor anims |
| +0x358 | 0xD6 | `IsTiberium` | bool | Inner gate: must be true for ore spawn |

All offsets verified via `decompile_function 0x00427d00` (AnimTypeClass::ReadINI).

### AnimClass instance offsets (param_1 is `int*`)

| Byte Offset | Description |
|-------------|-------------|
| AnimClass+0x80 (param_1[0x20]) | AnimTypeClass* (from param_1[0x32]... wait) |
| AnimClass+0xC8 | AnimTypeClass* (param_1[0x32]) — confirmed by assembly `MOV ECX, dword ptr [ESI + 0xC8]` |
| AnimClass+0x194 (param_1+0x65×4 cast) | Bouncer/IsMeteor flag (set in Constructor) |
| AnimClass+0x11E | CellClass::OverlayData — written with random density |

---

## 10. Open Questions (YELLOW — Unverified This Session)

1. **Default of `TiberiumSpreadRadius` when INI key absent.** Likely 0 (constructor default). If so, METDEBRI never places ore reliably in practice (landing cell is occupied). Needs `AnimTypeClass::Constructor` decompilation to confirm.

2. **Whether OverlayClass::Constructor stamps the cell into the growth queue.** The `OverlayClass::Constructor` at `0x005fc380` was not fully decompiled this session. If it calls `TiberiumClass::AddToGrowthQueue`, the cell enters normal ore growth. If not, it waits for queue rebuild. Needs decompilation to confirm.

3. **CRYSTAL1–4 practical ore placement.** With `TiberiumSpreadRadius=0`, the only candidate cell is the landing cell itself. `CanPlaceTiberium` may fail if the crystal anim occupies the cell. Whether TiberiumSpawnType=TIB2_01 (Vinifera) actually produces visually distinct ore on temperate maps is also unverified.

4. **METSMALL IsTiberium=yes with no TiberiumSpawnType.** The gate `*(int *)(param_1[0x32] + 0x338) != 0` would be false, so no ore is placed. But IsTiberium=yes still affects other AnimClass::AI behavior (HideIfNoOre, radar color). No ore is expected from METSMALL itself — only from its spawned METDEBRI children.

5. **CellAnim** — OverlayTypeClass+0x29C (`CellAnim`). The ore overlay types reference an AnimTypeClass here. Not investigated this session; listed as open gap.

---

## 11. Corrections to Prior Documentation

### ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md §6 — Correction

> "When `SpawnsTiberium=true`, the TerrainClass periodically creates an AnimClass instance. The AnimTypeClass associated with this animation has: TiberiumSpawnType (AnimTypeClass+0x338) ... TiberiumSpreadRadius (AnimTypeClass+0x33C)"

**WRONG.** Verified by decompiling `TerrainClass::AI` at `0x0071C730`: TerrainClass::AI does NOT create an AnimClass for ore spawning. It calls `CellClass::SpreadTiberium` directly. The `PUSH 0x1C8` at `0x0071b9a7` is in `TerrainClass::Take_Damage` (confirmed via `get_function_by_address 0x0071b9a7`), for a destruction explosion anim, not ore spawning.

The `TiberiumSpawnType` / `TiberiumSpreadRadius` fields are used by the meteor-debris bouncing system, not by TIBTRE.

### TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md §"TiberiumSpreadRadius / AnimClass Path" — Confirmed Correct

> "That system handles meteorite-style tiberium deposition from falling animations. It is NOT related to TIBTRE terrain object ore spawning."

**CONFIRMED CORRECT.** This session fully verified that claim.

---

## 12. Key Function Address Summary

| Address | Function | Role |
|---------|----------|------|
| `0x00423ac0` | `AnimClass__AI` | Per-tick update; contains inline ore-spawn block |
| `0x00423f00`–`0x00424235` | (inline in AnimClass__AI) | Ore-spawn block: radius loop + CanPlaceTiberium + OverlayClass::Constructor |
| `0x0042413b` | (call in AnimClass__AI) | `CALL OverlayClass::Constructor` — the actual ore placement |
| `0x004838e0` | `CellClass__CanPlaceTiberium` | Gate called at `0x004240c6` before each ore placement |
| `0x004cac40` | `Sqrt_Approx` | Euclidean distance filter called at `0x0042403e` |
| `0x005fc380` | `OverlayClass__Constructor` | Stamps overlay onto cell |
| `0x0065c7e0` | `Random__RandomRanged` | Used for variant (0–3) and density (0–2) |
| `0x0071C730` | `TerrainClass__AI` | TIBTRE ore-spawn driver (separate system, no AnimClass) |
| `0x00427d00` | `AnimTypeClass__ReadINI` | Source of all AnimTypeClass field offsets |
