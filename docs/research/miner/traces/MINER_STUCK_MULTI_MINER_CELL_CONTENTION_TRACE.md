# Multi-Miner Ore-Cell Contention Trace

**Scenario:** Miner A parked on cell (92,187) in Mission_Harvest state 1 (HARVEST).
Miner B fires `Scan_For_Tiberium` from cell (90,185). Cell (92,187) still has ore.
Does gamemd exclude cell (92,187) from miner B's candidate set?

**Scope:** `Scan_For_Tiberium` cell-exclusion logic only. Return-to-refinery and
path-grid/direct-move bugs are out of scope (tracked separately).

**Date:** 2026-05-20
**Sources:** Live Ghidra decompilation of gamemd.exe; `src/sim/miner/miner_system.rs`

---

## Stage Table

| # | Stage | gamemd behavior | Our behavior | Verdict |
|---|-------|----------------|-------------|---------|
| C0 | `Scan_For_Tiberium` ring-0 fast path | Center cell: checks `LandType==5` only — no `Is_Cell_Harvestable`, no `Can_Enter_Cell`. Returns center if it is ore. | `search_local_ore`: ring-0 checks `nodes.get(&center) && remaining > 0` — no occupancy check. | PASS (matches gamemd: both skip occupancy for ring 0) |
| C1 | `Scan_For_Tiberium` rings 1..N — harvestability gate | Each candidate cell is passed to `FootClass::Is_Cell_Harvestable` (0x004DCE80). Returns 1 only if `Can_Enter_Cell` returns 0 (OK/Clear). | `search_local_ore`: only checks `remaining > 0` + optional zone reachability filter. No `Can_Enter_Cell` / occupancy check. | **FAIL** |
| C2 | `Is_Cell_Harvestable` — zone reachability check | Calls `MapClass::Can_Reach_Zone` with harvester's zone ID. If unreachable → not harvestable. | `build_reachable_filter` → `ore_reachable()`: checks 8-neighbor zone connectivity. Semantically equivalent. | PASS |
| C3 | `Is_Cell_Harvestable` — shroud gate | If `g_GameMode==0` (campaign) and unit has shroud bit set at `+0x41A` and cell is shrouded → not harvestable. | No shroud gate (YR skirmish: g_GameMode!=0 in skirmish, gate never fires). | PASS (TS-gated / campaign-only; not a skirmish concern) |
| C4 | `Can_Enter_Cell(cell, -1, -1, 0, 1)` — allied harvester in cell | Allied occupant (same-team miner A) → returns 2 (TemporaryBlock) or 6 (FriendlyStationary). Non-zero → `Is_Cell_Harvestable` returns 0 → cell excluded from scan. | `search_local_ore` checks only `remaining > 0`. Returns cell even if another miner is sitting on it. | **FAIL** |
| C5 | `Can_Enter_Cell(cell, -1, -1, 0, 1)` — enemy harvester in cell | Enemy occupant → returns 5 (EnemyBlock). Non-zero → cell excluded. | Same: no occupancy check; cell returned even with enemy miner on it. | **FAIL** |
| C6 | Post-scan state-0 dispatch — secondary filter | No second "is-anyone-heading-there" reservation check in Mission_Harvest state 0. The scan result is used directly as the drive destination. | Same: no reservation check. | PASS (no secondary filter exists in gamemd either) |
| C7 | Ring-0 center-cell occupancy — miner B already on ore cell | Miner B is standing on an ore cell: ring-0 fast path returns it immediately without Can_Enter_Cell. Miner B harvests its own cell regardless of whether anyone else is there. | Same: ring-0 returns center if ore present, no occupancy check. | PASS |

---

## Binary Evidence

### `FootClass::Scan_For_Tiberium` — 0x004DD0A0 (verified decompile)

Ring-0 fast path (center cell):
```
iVar6 = MapClass__Get_CellClass(&sStack_18);
if (*(int *)(iVar6 + 0xec) == 5) {     // LandType == Tiberium (5)
    *unaff_retaddr = CONCAT22(sStack_16,sStack_18);
    return;                             // returns center IMMEDIATELY — no Is_Cell_Harvestable
}
```
Rings 1..N:
```
cVar3 = FootClass__Is_Cell_Harvestable(&stack0xffffffc4);
if (cVar3 != '\0') {
    // only then compute tiberium value and consider as candidate
}
```

### `FootClass::Is_Cell_Harvestable` — 0x004DCE80 (verified decompile)

After zone check passes and `LandType == 5`:
```
uVar1 = (**(code **)(*param_1 + 0x1ac))(uVar1, 0xffffffff, 0xffffffff, 0, 1);
if (uVar1 == 0) {
    return 1;   // harvestable
}
// else: return 0 (blocked)
```

### vtable slot 0x1AC verified as `UnitClass::Can_Enter_Cell`

- UnitClass vtable base: `0x007f5c70` (confirmed via `list_globals`)
- Slot `0x1AC / 4 = 0x6B`: `read_memory(0x007f5e1c, 4)` → `a0 f0 73 00` = `0x0073F0A0`
- `UnitClass__Can_Enter_Cell @ 0x0073F0A0` (confirmed via `search_functions`)

### `UnitClass::Can_Enter_Cell` return values for occupied ore cell

From decompiled code at 0x0073F0A0 (verified decompile), for `param_3 = -1, param_4 = -1`:
- Allied non-moving vehicle occupant → returns **6** (FriendlyStationary)
- Allied moving vehicle occupant → returns **2** (TemporaryBlock)
- Enemy vehicle occupant → returns **5** (EnemyBlock)
- All three are non-zero → `Is_Cell_Harvestable` returns 0 → cell excluded from scan rings 1+.

---

## Disparity Summary

### FAIL C1/C4: `search_local_ore` does not filter cells occupied by another harvester

gamemd's `Scan_For_Tiberium` passes every ring-1+ candidate through
`Is_Cell_Harvestable` → `Can_Enter_Cell`. Any occupied cell returns non-zero →
excluded. Our `search_local_ore` checks only `remaining > 0` and optional zone
reachability. It will hand miner B the cell miner A is sitting on.

**File:line:** `src/sim/miner/miner_system.rs:1194–1204` (ring inner loop, no occupancy filter)

**Player-visible effect:** Miner B drives toward miner A's occupied cell. Since
`bypass_grid` is not set for ore-approach moves, `movement_occupancy.rs` deferred
check fires → miner B is blocked or scatter-routed. Frequency: every time two
miners target the same ore patch, which is common in dense-ore maps with 2+ miners.

**Note — ring 0 exception:** If miner B's current position is on an ore cell, ring-0
fast path returns it regardless. This matches gamemd. The contention bug only fires
when the target cell is at ring 1+.

### FAIL C5: No enemy-harvester exclusion

Same root cause: `search_local_ore` returns an enemy-miner-occupied cell.
`Can_Enter_Cell` would return 5 (EnemyBlock) in gamemd. Observable: miner B
drives toward an enemy-occupied ore cell and gets stuck at the approach (enemy
unit is an impassable occupant from the pathfinder's perspective).

---

## Key Findings (Top 5 Player-Visible Failures)

1. **C4 — Allied-miner cell returned by scan** (`src/sim/miner/miner_system.rs:1194`):
   `search_local_ore` returns a cell occupied by an allied harvester; gamemd excludes it
   via `Can_Enter_Cell` returning 2/6. Two miners converge on the same cell → movement
   stall. Fires every match whenever 2+ miners harvest the same patch (common).

2. **C5 — Enemy-miner cell returned by scan** (`src/sim/miner/miner_system.rs:1194`):
   Same root; enemy occupant causes `Can_Enter_Cell` = 5 in gamemd → excluded. Our
   miner drives into an enemy-occupied cell and stalls. Fires in any match where
   enemy miners share ore territory (every competitive skirmish).

3. **C1 — Ring-1+ cells checked for density only** (`src/sim/miner/miner_system.rs:1200`):
   The absence of `Is_Cell_Harvestable` means any non-empty ore cell is a candidate
   regardless of occupancy or passability. Combined with the path-grid bug (slots 2/3),
   this produces the observed miner-stuck scenario: scan picks an occupied/blocked cell,
   drive stalls, miner never transitions out of MoveToOre.

4. **C0 — Ring-0 center-cell fast path** (`src/sim/miner/miner_system.rs:1162`):
   This correctly matches gamemd (no occupancy check for center). Not a failure here;
   listed to confirm the ring-0 behavior is intentionally unfiltered on both sides.

5. **C6 — No post-scan reservation filter** (`src/sim/miner/miner_system.rs:268–311`):
   No "is-anyone-else-heading-there" check post-scan, but this matches gamemd too —
   gamemd's state 0 dispatch also has no such filter. Contention at this level is
   handled entirely by the scan-time `Is_Cell_Harvestable` gate (missing in our impl).

---

## Rust Fix Sketch (for reference, do not implement without approval)

`search_local_ore` needs a second optional filter argument for occupancy, or the
caller in `handle_search_ore` / `handle_harvest` should supply a closure that
checks the OccupancyGrid for vehicle occupants. The ring-0 fast path should remain
unfiltered (matching gamemd). Ring 1+ would additionally call:

```rust
// Exclude cells with vehicle occupants (mirrors Is_Cell_Harvestable → Can_Enter_Cell).
// Ring 0 skips this check — gamemd Scan_For_Tiberium returns center directly.
if let Some(occ_filter) = occ_filter && !occ_filter(cell) {
    continue;
}
```

`bypass_grid` is unrelated to this fix — it is only set during the dock choreography,
not during ore approach. The ore-approach move already has `bypass_grid = false`, so
the movement system's occupancy check fires correctly — the problem is that the scan
should never have picked the cell in the first place.

---

## Verdict Tally

**PASS: 4 | FAIL: 3 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0**

(C0, C2, C3, C6 = PASS; C1, C4, C5 = FAIL)

---

**Status: COMPLETE**
