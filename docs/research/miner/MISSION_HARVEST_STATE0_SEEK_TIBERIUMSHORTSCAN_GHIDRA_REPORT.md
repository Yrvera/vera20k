# Mission_Harvest State 0 (SEEK/SCAN) — Ore-Cell Search Algorithm

**Primary address:** `0x73E5E0` (UnitClass::Mission_Harvest, state 0 = case 0)
**Scan function:** `FootClass::Scan_For_Tiberium` @ `0x4DD0A0`
**Harvestable predicate:** `FootClass::Is_Cell_Harvestable` @ `0x4DCE80`
**Confidence:** HIGH (assembly + decompilation verified from binary this session)
**Active in YR:** YES — live path for War Miner (HARV) and Chrono Miner (CMIN)
**Date:** 2026-05-19

---

## 1. Overview

State 0 is entered on the first tick of Mission_Harvest (mission 10) and any time the
miner needs to find a new ore patch after returning from a refinery or depleting a field.
It performs a diamond-ring scan expanding outward from the miner's current cell, picks
the highest-value cell in the nearest ring that has any harvestable ore, commands the
miner to drive there, and transitions to state 1. If no ore is found anywhere within
TiberiumLongScan radius, transitions to state 4 (LOST → Guard).

**Correction to prior doc (CHRONO_MINER_SYSTEM_OVERVIEW §4):** That document states
"Chrono miner uses TiberiumShortScan (6 cells), regular uses TiberiumLongScan (48)."
This is **WRONG** for state 0. The binary at `0x73E772` and `0x73E851` both read
offset `0x177C` (TiberiumLongScan). **Both war miner and chrono miner use TiberiumLongScan
(48 cells) for the state 0 initial scan.** TiberiumShortScan (offset `0x1778`) is
used only in state 1 continuation scans — never in state 0.

---

## 2. State 0 Entry and Per-Tick Behavior

**Entry condition:** `param_1[0x2F]` (UnitClass+0xBC, harvest substate) == 0

**Firing:** State 0 runs every tick with NO per-tick timer guard. The state machine
returns `1` (process next tick immediately) when ore is found, `0x69` (105 ticks) when
no ore exists and no destination, or the default mission timer via `MissionClass::GetMissionTimerEntry`
+ `Random(0,2)` for the default path. There is no explicit state 0 rate timer.

**Sequence (verified from 0x73E6F1 assembly):**

### Step A: Full-storage early exit (Harvester only, not Weeder)
- At `0x73E6F1`: checks `TypeClass+0xE0F` (Weeder flag). If Weeder=YES, skip.
- At `0x73E700`: calls `vtable+0x2B4` = `UnitClass::Get_Storage_Percentage` (0x7414A0)
- Compares against `_g_Const_1_0` (1.0f) using `FCOMP` + `FNSTSW` / `TEST AH, 0x1`
- If storage >= 1.0 (full): sets `param_1[0x2F] = 2` (state 2 = RETURN), returns 1.
- **Detail:** uses `FCOMP` (non-equal comparison, FPU flag bit 0x01 = CF, not ZF).
  A value of exactly 1.0 AND above both trigger state 2. Float equality to 1.0 is exact
  here because `Get_Storage_Percentage` returns `current_load / capacity` as float.

### Step B: Archive cell handling
- At `0x73E72A`: loads `param_1[0x86]` = `UnitClass+0x218` (Archive/ghost cell).
  If non-zero: calls `vtable+0x480` (Set_Destination) with the archive cell AND flag=1,
  then calls `TechnoClass::SetGhostCell(0)` to clear the archive.
  Also sets `local_50 = 0` (turns off zone parameter for the scan call that follows).
- If `param_1[0x86]` == 0: `local_50` stays at 1 (zone filtering ON).

**Key detail on `local_50`:** Set to 1 at `0x73E730` (`MOV byte [ESP+0x14], 1`),
then conditionally cleared to 0 at `0x73E750` if archive was consumed. This value
becomes the zone parameter passed to `FootClass::Search_For_Tiberium_And_Move` at
`0x73E84E` (PUSH ECX where ECX = [ESP+0x14]). When zone=1 the scan respects zone
connectivity; when zone=0 it scans without zone restriction.

### Step C: `UnitClass+0x6D2` harvesting flag cleared
- At `0x73E75B`: `MOV byte [EBP+0x6D2], 0` — IsHarvesting flag cleared unconditionally.

### Step D: Weeder branch (UnitTypeClass+0xE0F != 0)
- At `0x73E762–0x73E78E`:
  - Reads `RulesClass+0x177C` (TiberiumLongScan), arithmetic-right-shifts by 8
    (`CDQ` / `AND EDX, 0xFF` / `ADD EAX, EDX` / `SAR EAX, 8`) to convert leptons→cells.
  - Calls `FootClass::Search_For_Tiberium_Short_And_Move` (0x4DDB90) with that radius.
  - Jumps to `LAB_0073E879` (shared post-scan logic).

### Step E: Harvester+Chrono common path (Weeder=NO)
- At `0x73E793–0x73E844`: locomotion CLSID check:
  - Dereferences `param_1[0x19D]` (UnitClass+0x674 = locomotion pointer).
  - If null: asserts via `GameDebugLog::Assert`.
  - Calls `FUN_0045A050()` to get the locomotion COM object.
  - Calls `QueryInterface(IID_IPersistStream=0x818858)` on it.
  - On success: calls `vtable+0xC` (GetClassID) on the returned interface.
  - Compares the returned CLSID (4 DWORDs, 16 bytes) against
    `CLSID_TeleportLocomotion` at `0x7E9A90` using `CMPSD.REPE` with ECX=4.
  - If CLSID matches AND `param_1[0x169]` (UnitClass+0x5A4, destination) != 0:
    calls `vtable+0x480(0, 1)` to clear the destination. This cancels any in-progress
    teleport warp before scanning for ore.
  - Releases the QueryInterface pointer via `DriveLocomotionClass::Release_Piggybacked_Helper`.

- At `0x73E844–0x73E864`: radius calculation + scan call:
  - Reads `[0x008871E0 + 0x177C]` = `RulesClass+0x177C` = **TiberiumLongScan** (48, in leptons).
  - Same arithmetic: `CDQ / AND EDX, 0xFF / ADD EAX, EDX / SAR EAX, 8` → converts to cells.
  - Pushes `local_50` (zone parameter: 1 normally, 0 if archive was consumed).
  - Calls `FootClass::Search_For_Tiberium_And_Move` (0x4DCFE0) with (radius_cells, zone).

---

## 3. Chrono-Miner-Specific Branch in State 0

**Location:** `0x73E793–0x73E844` — the locomotion CLSID check.

The Teleporter flag at `TechnoTypeClass+0xCD4` is loaded into BL register at `0x73E6DE`
and carried across all states, but **state 0 does NOT branch on BL (Teleporter flag)
directly** to select a different scan function. Both war miner (BL=0) and chrono miner
(BL=1) go through the same code path at `0x73E793`.

The CLSID check applies to both units. For the war miner, its locomotor is
DriveLocomotionClass — QueryInterface for IPersistStream returns the drive loco,
and its CLSID will not match TeleportLocomotion, so the destination-clear block is
skipped. For the chrono miner, its active locomotor may be TeleportLocomotionClass;
if so and if a destination is set (warp in progress), the warp is cancelled before
scanning for ore.

**Summary:** The chrono-miner difference in state 0 is the locomotor CLSID check that
cancels an in-progress warp. There is NO difference in scan radius, scan function, or
scan pattern between war miner and chrono miner in state 0.

---

## 4. Diamond Spiral Scan Algorithm (FootClass::Scan_For_Tiberium, 0x4DD0A0)

**param_1:** `int*` (FootClass = this pointer, treated as `int*` — multiply indices by 4)
**param_2:** `int` — radius in cells (not leptons)
**Returns:** packed CellStruct (X in low 16 bits, Y in high 16 bits) on caller stack

### Algorithm (verified from decompilation):

```
1. Get unit's lepton coordinates via vtable+0x48 (Get_Coords).
   Convert to cell: cell_x = (lepton_x + (lepton_x >> 31 & 0xFF)) >> 8
                    cell_y = (lepton_y + (lepton_y >> 31 & 0xFF)) >> 8
   (The >> 31 & 0xFF pattern is arithmetic-right-shift sign extension for rounding.)

2. Check center cell (ring 0):
   Call MapClass::Get_CellClass(cell_x, cell_y).
   If CellClass+0xEC == 5 (LandType == Tiberium): return current cell IMMEDIATELY.
   No harvestability check — just LandType test. This is the current cell fast path.

3. Outer loop: ring = 1; best_value = -1; inner_offset = -1
   While ring < param_2:
     Inner loop: col = inner_offset; while col <= ring:
       Check 4 cells (diamond perimeter at this ring/col):
         Cell A: (center_x + col,  center_y - ring)   // top arm
         Cell B: (center_x + col,  center_y + ring)   // bottom arm
         Cell C: (center_x - ring, center_y + col)    // left arm
         Cell D: (center_x + ring, center_y + col)    // right arm
       For each cell:
         Call FootClass::Is_Cell_Harvestable(cell)
         If harvestable:
           Call MapClass::Get_CellClass(cell) → CellClass*
           Call CellClass::Get_Tiberium_Value() → int value
           If value > best_value: best_cell = cell, best_value = value
       col += 1
     End inner loop
     If best_value != -1: BREAK (early exit — stop expanding rings)
     ring += 1; inner_offset -= 1
   End outer loop

4. Return best_cell (or the uninitialized stack value if no ore found at all).
```

**Key behavioral details:**

- **Ring 0 (current cell) fast path:** Only tests `CellClass+0xEC == 5`, does NOT call
  Is_Cell_Harvestable. No zone check, no passability check, no shroud check for the
  center cell. If the unit is standing on ore, it returns immediately. This remains
  true when `OverlayData == 0`; density is not an eligibility gate.

- **Early exit per ring:** The scan breaks as soon as any ring produces a non-(-1) best_value.
  It does NOT continue to ring+1 if ore was found. However, it finishes scanning all cells
  in the CURRENT ring before breaking, picking the highest-value among them.

- **Selection: HIGHEST VALUE in the nearest ring.** NOT first-found, NOT closest.
  The comparison is `if (iVar6 < iVar7)` (strict less-than), so an equal value does
  not replace the current winner: the first accepted candidate wins ties (iteration
  order: top arm → bottom arm → left arm → right arm, inner col first within each
  ring). This was rechecked in the active binary at `0x004DD0A0`.

- **inner_offset starts at -1 and decrements each ring:** At ring=1: col from -1 to 1
  (3 iterations). At ring=2: col from -2 to 2 (5 iterations). At ring=r: col from -r to r
  (2r+1 iterations). This correctly scans the full diamond perimeter.

- **Corner cells are checked TWICE:** At col=±ring, cells A/B and C/D degenerate to the
  diamond corners. Due to the col ranging from -ring to +ring inclusive, corners are hit
  when col=±ring on two different inner iterations but both check all 4 arms, so a corner
  cell may be evaluated up to twice. No deduplication — harmless since it just re-evaluates
  the same cell.

- **No bounds check in scan loop.** Out-of-map cells are passed to Is_Cell_Harvestable,
  which calls `MapClass::Is_Cell_In_Playfield` as its first check — those fail and are skipped.

- **Radius: param_2 cells, NOT leptons.** The caller (Mission_Harvest state 0) converts
  from leptons to cells via SAR 8. RulesClass stores radii in leptons; state 0 divides by 256.

- **Loop range:** `while ring < param_2` (strict less-than). At param_2=48 (TiberiumLongScan),
  rings 1..47 are scanned (47 rings). Ring 0 is the fast-path center check.

---

## 5. Per-Cell Predicate: FootClass::Is_Cell_Harvestable (0x4DCE80)

**param_1:** `int*` (FootClass = harvester instance)
**param_2:** `short*` (packed CellStruct pointer, [0]=x, [1]=y)
**Returns:** uint — non-zero = harvestable, 0 = not harvestable

### Checks in order:

```
1. MapClass::Is_Cell_In_Playfield(param_2)
   — Cell must be within map bounds. Early return 0 if not.

2. Shroud check (single-player only):
   If (g_GameMode == 0) AND (this+0x41A != 0, "IsSelectedByPlayer" or player-unit flag):
     Convert cell to lepton center: lepton_x = cell_x * 0x100 + 0x80
                                    lepton_y = cell_y * 0x100 + 0x80
     Call IsShrouded(lepton_x, lepton_y, ...)
     If shrouded: return 0 (skip shrouded cells in singleplayer)
   This check is SKIPPED in multiplayer (g_GameMode != 0).

3. Zone reachability: MapClass::Can_Reach_Zone(param_2, SpeedType_zone, ...)
   — Gets unit's SpeedType zone via vtable+0xBC then vtable+0x84 (ZoneClass lookup).
   — Calls MapClass::Can_Reach_Zone with the cell coords and zone ID.
   — If not reachable: return 0.
   — This is the zone connectivity check that prevents harvesters from targeting
     ore across impassable terrain (walls, water gaps, etc.).

4. LandType check:
   Call MapClass::Get_CellClass(param_2) → iVar
   If CellClass+0xEC (LandType) != 5 (Tiberium): return 0.

5. Passability check:
   Call vtable+0x1AC(cell_ptr, 0xFFFFFFFF, 0xFFFFFFFF, 0, 1) = Can_Enter_Cell(...)
   If result != 0 (can NOT enter): return 0.

6. If all pass: return 1 (harvestable).
```

**Active in YR:** YES. g_GameMode check is the only conditional — in multiplayer (which
is the standard YR skirmish), the shroud check is skipped entirely.

---

## 6. CellClass::Get_Tiberium_Value (0x485020)

```c
int CellClass::Get_Tiberium_Value() {
    int tib_idx = CellClass::OverlayToTiberiumIndex();  // via overlay type
    if (tib_idx == -1) return 0;
    return g_TiberiumClass_Array[tib_idx]->field_B8 * (this->field_0x11E + 1);
}
```

- `OverlayToTiberiumIndex @ 0x005FDD20` returns `-1` only for no overlay or
  `Tiberium=false`. A `Tiberium=yes` overlay outside all registered primary and
  extra-image ranges logs a warning and returns type index `0`; this function
  therefore evaluates that cell with type-0 value rather than returning zero.
- `CellClass+0x11E` = tiberium density byte (0–11). Value formula: `base_value * (density + 1)`.
- A higher-base-value resource wins only when the complete
  `base_value * (density + 1)` product is higher. With stock values 25 for
  Riparius and 50 for Cruentus, equal-density gems beat ore, but sufficiently
  denser ore can beat gems (for example, ore density 2 scores 75 while gem
  density 0 scores 50).
- At density 0: value = base_value * 1. At density 11: value = base_value * 12.
- **This means the highest computed cell value wins within a ring; neither
  overlay kind nor density alone determines the winner.**
- **Implementation consequence:** a live `LandType == Tiberium` cell must not be
  rejected merely because its economic/resource-node quantity is zero. A level-0
  overlay is selected, moved to, and presented to the later harvest/reduction path.

---

## 7. State 0 Exit Conditions

### → State 1 (HARVEST) — ore found
At `LAB_0073E879` (`0x73E879`):
- `Search_For_Tiberium_And_Move` (or Short variant) returns non-zero (BL != 0).
- Sets `UnitClass+0x6D2 = 1` (IsHarvesting flag).
- Initializes the step timer: `param_1[0x43]` (UnitClass+0x10C) = **2**,
  `param_1[0x40]` (UnitClass+0x100) = `g_CurrentFrameCounter`,
  `param_1[0x42]` (UnitClass+0x108) = **2**, `param_1[0x3E]` (UnitClass+0xF8) = **0**.
- Sets `param_1[0x2F]` (state) = 1.
- Returns 1 (process next tick immediately).
- **Detail:** The timer is initialized with duration=2 and value=2, NOT HarvesterLoadRate.
  State 1 immediately overwrites this on its first tick (checks if timer == 0 to reload
  with HarvesterLoadRate). The duration-2 value means state 1 runs its first timer check
  within 2 ticks of arriving.

### → State 4 (LOST/NO ORE) — no ore, no destination, no archive
At `0x73E8EA–0x73E924`:
- Scan returned false (BL == 0) AND `param_1[0x169]` (destination) == 0
  AND `param_1[0x86]` (archive cell) == 0.
- Sets state = 4, sets `UnitClass+0x3D0 = 1` (FirstTimeFlag).
- If `TypeClass+0xE0E` (Harvester): sets `HouseClass+0x242 = 1` (house ore-depleted flag).
- **Returns 0x69 = 105 ticks** (not 1!) — a ~7-second delay before trying again.
  This is the ONLY state 0 path that returns something other than 1.

### → Continue in state 0 (destination exists, ore moving toward)
At `0x73E925`:
- Scan returned false but `param_1[0x169]` (destination) != 0 (already pathing toward ore).
- Clears `UnitClass+0x3D0 = 0` (FirstTimeFlag cleared — "we have a destination, not lost").
- Falls through to default timer exit: `MissionClass::GetMissionTimerEntry() * constant + Random(0,2)`.

### → State 2 (RETURN) — full storage (step A)
- Happens at the top of state 0 BEFORE the scan if storage >= 1.0 and unit is a Harvester.
- Returns 1 immediately.

### No-ore retry behavior:
When `Search_For_Tiberium_And_Move` returns false but destination IS set (unit was already
sent toward an archive cell at step B), state 0 stays active and returns the default timer
value — it does NOT go to state 4. State 4 is only reached when there is simultaneously no
ore found AND no destination AND no archive. This means a miner with an archive cell will
stay in state 0 driving to the archive even if the scan itself found nothing new.

---

## 8. Scan Radius Source — Verified Offsets

Both state 0 scan calls read from the same RulesClass offset:

| Assembly address | Instruction | Source |
|-----------------|-------------|--------|
| `0x73E772` | `MOV EAX, [ECX + 0x177C]` | RulesClass+0x177C = TiberiumLongScan (Weeder path) |
| `0x73E851` | `MOV EAX, [EDX + 0x177C]` | RulesClass+0x177C = TiberiumLongScan (Harvester/Chrono path) |

State 1 continuation scans read `+0x1778` (TiberiumShortScan). State 0 **always** reads `+0x177C` (TiberiumLongScan).

RulesClass base pointer: `g_RulesClass_Instance` at `0x8871E0`.

INI key defaults (from `ini/rulesmd.ini:311-312`):
- `TiberiumShortScan=6` → 6 cells
- `TiberiumLongScan=48` → 48 cells

---

## 9. Key Struct Offsets for State 0

### UnitClass (param_1 is int*, byte offset = index * 4)

| Byte offset | int* index | Field | State 0 usage |
|-------------|-----------|-------|---------------|
| 0xBC | [0x2F] | HarvestSubState | Switched on; set to 1/2/4 at exits |
| 0xF8 | [0x3E] | StepCounter | Cleared to 0 on state 1 entry |
| 0x100 | [0x40] | RateTimer.StartFrame | Set to g_CurrentFrameCounter on state 1 entry |
| 0x108 | [0x42] | RateTimer.Value | Set to 2 on state 1 entry |
| 0x10C | [0x43] | RateTimer.Duration | Set to 2 on state 1 entry |
| 0x218 | [0x86] | Archive | Ghost-cell target (read + cleared in state 0) |
| 0x3D0 | [0xF4] | FirstTimeFlag | Set to 1 on LOST, cleared to 0 with destination |
| 0x5A4 | [0x169] | Destination | Checked to distinguish "moving" vs "lost" |
| 0x674 | [0x19D] | LocomotionPtr | Used for CLSID check |
| 0x6D2 | (byte) | IsHarvesting | Cleared to 0 at state 0 top, set to 1 on state 1 entry |

### TechnoTypeClass (byte offsets)

| Byte offset | Field | Role in state 0 |
|-------------|-------|----------------|
| 0xCD4 | Teleporter | Loaded into BL at 0x73E6DE; gates CLSID check path |
| 0xE0E | Harvester | Full-storage check gating; radius used |
| 0xE0F | Weeder | Weeder branch taken if non-zero |

---

## 10. Disparity vs. Rust Implementation

**2026-07-24 current-Rust refresh:** the earlier selection-order and state-0
fullness findings below were subsequently implemented. Direct reads of current
`src/sim/miner/miner_system.rs` show the remaining load-bearing level-0 drift:

- `search_local_ore` rejects the center unless `node.remaining > 0` and skips
  every ring candidate whose `remaining == 0`;
- `handle_move_to_ore` abandons a selected target unless `remaining > 0`;
- the global fallback `pick_best_resource_node` also skips zero nodes;
- archive consumption recognizes a zero node by key presence, but the following
  move tick immediately rejects it.

Those checks conflict with the live calls
`decompile_function(0x004DD0A0)`,
`decompile_function(0x004DCE80)`,
`decompile_function(0x004DCFE0)`, and
`decompile_function(0x00485020)` on retail
`gamemd.exe` SHA-256
`1CDD1180E49024FBDA8AD568CAAC2E86E856063FF67AB38F62B7D2C7BB84298C`.
The native eligibility authority is effective `LandType == Tiberium`, while
`OverlayData + 1` affects value only.

The comparison below is historical. Items 1 and 2 are resolved in current Rust;
the remaining items require direct current-code rechecks before use.

The Rust implementation at the time this report was first written:

### Matches gamemd:
- Uses `long_scan_radius` (TiberiumLongScan) for the initial scan. ✓
- Checks archive cell (`last_harvest_cell`) first and drives to it if valid. ✓
- Falls back to WaitNoOre if no ore found (analogous to state 4). ✓
- Zone reachability filter (via `build_reachable_filter`). ✓
- Uses `config.rescan_cooldown_ticks` for the no-ore delay. ✓

### Disparities from gamemd:

1. **No full-storage early-exit at state 0 top.** Gamemd state 0 checks `Get_Storage_Percentage() >= 1.0` at entry and transitions immediately to state 2 if full. The Rust `handle_search_ore` does not check cargo fullness and would proceed to scan even if the miner is full. This fires whenever a miner re-enters SearchOre with a full cargo (rare but possible if the state machine cycles back from Harvest with full cargo — happens when `extract_bales_max` returns empty but cargo is already full).

2. **Selection criterion mismatch.** Gamemd picks the highest-value cell in the nearest ring (`best_value` comparison). The Rust `search_local_ore` ranks by: gems-before-ore, then highest density, then nearest, then ry/rx tie-break. For all-ore fields, Rust picks the highest-density nearest cell (not the nearest-ring highest-value). The difference: gamemd always finds the nearest patch with ore, then picks best density within that ring. Rust scans all cells in the bounding box and picks globally highest density. If there are two ore rings at equal density, gamemd picks the one in the closer ring; Rust picks neither has priority by ring — it uses dist_sq as a tiebreaker.

3. **Archive zone re-check.** Gamemd does not re-check zone reachability when
   consuming the archive — it calls `Set_Destination` directly and clears the
   archive. Rust does re-check
   (`archive_reachable = filter_ref.is_none_or(|f| f(archive))`). This is
   mechanism DRIFT: a still-present archived overlay that became unreachable or
   otherwise fails the current Rust filter remains a native destination but is
   discarded by Rust.

4. **`local_50` zone parameter for scan.** When archive is consumed, gamemd passes zone=0 to Search_For_Tiberium_And_Move (disabling zone filtering). Rust always passes the zone filter. This could cause a disparity in the rare case where the archive was consumed but no ore is found in the zone-filtered scan — gamemd would then retry without zone filter, Rust would not.

5. **No global fall-through to default timer.** When state 0 has a destination (miner is moving toward archive), gamemd returns the default mission timer (not 1) and stays in state 0. Rust's `handle_search_ore` returns nothing — it just stays in `SearchOre` and will be called again next tick. The timer difference means gamemd might wait a few ticks between state 0 calls in this case, Rust calls it every tick.

6. **No CLSID cancel-warp.** Gamemd cancels any in-progress warp (clears destination) if the locomotor is TeleportLocomotionClass and has a destination. Rust handles this at the locomotor level but not in `handle_search_ore` explicitly.

7. **No `HouseClass+0x242` ore-depleted flag.** Gamemd sets `HouseClass+0x242 = 1` when transitioning to state 4 and unit is Harvester. Rust has no equivalent.

8. **WaitNoOre returns immediately in Rust.** Gamemd state 4 immediately queues Mission_Guard (5) and returns. Rust's `WaitNoOre` counts down `rescan_cooldown_ticks` then returns to SearchOre. Gamemd retries via the state 0 scan next time Mission_Harvest is called, which happens after whatever Mission_Guard's own timer returns. The delay mechanism differs but the functional outcome (delay before retry) is similar.

---

## 11. Open Questions — Final State

- `[RESOLVED] Q1` — Does state 0 use TiberiumShortScan or TiberiumLongScan? → **TiberiumLongScan (0x177C) for both war miner and chrono miner.** (evidence: assembly at `0x73E772`, `0x73E851`)
- `[RESOLVED] Q2` — Selection criterion: first-found, nearest, or richest? → **Richest cell within the first/nearest ring that has any ore.** (evidence: `Scan_For_Tiberium` decompilation, `best_value` comparison at `0x4DD0A0`)
- `[RESOLVED] Q3` — Chrono-miner-specific branch in state 0? → **Yes — locomotor CLSID check cancels in-progress warp. No scan function difference.** (evidence: `0x73E793–0x73E844`)
- `[RESOLVED] Q4` — Exit to state 4: immediate or with timer? → **Returns 0x69 (105 ticks) immediately on transition to state 4.** (evidence: `0x73E91B`)
- `[RESOLVED] Q5` — Per-cell predicate order? → **In-playfield → shroud (SP only) → zone reachability → LandType==5 → Can_Enter_Cell==0.** (evidence: `Is_Cell_Harvestable` decompilation at `0x4DCE80`)
- `[RESOLVED] Q6` — Timer on state 0 entry? → **No timer. State 0 runs every tick (returns 1 when ore is found). Returns 0x69 only on LOST transition. Returns default mission timer when destination exists.** (evidence: assembly at `0x73E879–0x73E924`)
- `[RESOLVED] Q7` — CellClass+0x11E role in scan? → **Used by `Get_Tiberium_Value`: value = base_value * (density+1). Not directly tested in Is_Cell_Harvestable — the LandType check at CellClass+0xEC==5 is the harvestable gate.** (evidence: `0x485020`)
- `[RESOLVED] Q8` — Weeder path in state 0? → **Calls `Search_For_Tiberium_Short_And_Move` (which uses `Scan_For_Tiberium_NoZone`) with TiberiumLongScan radius. `Scan_For_Tiberium_NoZone` returns the FIRST valid weed cell, not the richest.** (evidence: `0x4DD890` decompilation)
- `[DEFERRED] Q9` — Exact `Can_Enter_Cell` vtable+0x1AC implementation for harvesters.` (category: out-of-scope; reason: not needed for state 0 disparity analysis; next-step: decompile vtable+0x1AC on UnitClass)
- `[DEFERRED] Q10` — What does `g_MapEditorMode` increment in state 2 actually disable? (category: out-of-scope; reason: state 2 only, not state 0)

---

## 12. Sources

- `gamemd.exe` decompiled this session via Ghidra MCP:
  - `UnitClass::Mission_Harvest` @ `0x73E5E0` (full decompile + disassembly)
  - `FootClass::Scan_For_Tiberium` @ `0x4DD0A0` (full decompile)
  - `FootClass::Search_For_Tiberium_And_Move` @ `0x4DCFE0` (full decompile)
  - `FootClass::Search_For_Tiberium_Short_And_Move` @ `0x4DDB90` (full decompile)
  - `FootClass::Scan_For_Tiberium_NoZone` @ `0x4DD890` (full decompile)
  - `FootClass::Is_Cell_Harvestable` @ `0x4DCE80` (full decompile)
  - `CellClass::Get_Tiberium_Value` @ `0x485020` (full decompile)
- Prior docs consulted (not ground-truth — verified against binary):
  - `MISSION_HARVEST_GHIDRA_REPORT.md` (Apr 3)
  - `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` (Mar 27)
  - `CHRONO_MINER_SYSTEM_OVERVIEW.md`
- INI: `ini/rulesmd.ini:311-312` (TiberiumShortScan=6, TiberiumLongScan=48)
- Rust: `src/sim/miner/miner_system.rs` (current implementation)
