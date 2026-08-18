# Crate System — Ghidra Research Report

**Primary Addresses:**
- `0x00481a00` — `CellClass__Can_Enter_Cell_General` (pickup dispatch, 0x481a00–0x483393)
- `0x0056bd40` — Crate placement function (find random cell + place)
- `0x0056bbe0` — Crate regen timer update (called every tick)
- `0x0056c020` — Crate removal on pickup
- `0x004a17c0` — Crate slot: place overlay + init timer
- `0x004a1750` — Crate slot: clear/remove overlay
- `0x004a18f0` — Crate slot: validate cell for placement
- `0x004a1aa0` — Remove crate overlay from cell (checks it's WoodCrateImg/CrateImg/WaterCrateImg)
- `0x0066b900` — `RulesClass::ReadCrateRules` (reads `[CrateRules]` INI section)
- `0x005fe7a0` — `OverlayTypeClass::ReadINI` (reads `Crate=` and `CrateTrigger=` flags)
- `0x00747620` — `UnitTypeClass::ReadINI` (reads `CrateGoodie=`, `CarriesCrate=`)

**Confidence:** HIGH (core pickup, regen, MP placement, death-drop paths all verified). See §8 Follow-up Pass (2026-04-21) for resolution of Open Questions 7.1, 7.2, 7.3, 7.6 (all now HIGH), and full extraction of the anim table (7.5).
**Active in YR:** Yes — multiplayer crate spawning/regen is active when "Crates" game option is enabled. Single-player uses map-placed crate overlays with no respawning.

> **Timer correction (from §8.4):** §3.3 below originally stated the regen timer is `CrateRegen * 1800`. This is the **upper bound**; the actual timer is uniformly random in `CrateRegen * [450, 1800]` frames (i.e. 25 % .. 100 % of `CrateRegen` minutes). See §8.4 for the verified x87 sequence.

---

## 1. Overview

The crate system in gamemd.exe has **no standalone CrateClass**. Instead, crate logic is distributed across several classes:

- **OverlayTypeClass** defines which overlays are crates (`Crate=true`)
- **CellClass** hosts the overlay and contains the pickup dispatch (a giant switch statement)
- **MapClass** (or a related global) manages a 256-slot crate timer array for spawning/regen
- **RulesClass** stores all `[CrateRules]` configuration
- **UnitTypeClass** has per-unit crate flags (`CrateGoodie`, `CarriesCrate`)
- **BuildingTypeClass** has `CrateBeneath` / `CrateBeneathIsMoney` flags

---

## 2. Class Layouts / Key Offsets

### CellClass Crate-Relevant Fields

| Offset | Type | Field | Notes |
|--------|------|-------|-------|
| +0x24  | packed short×2 | MapCoord (X,Y) | Cell coordinates |
| +0x44  | int  | OverlayTypeIndex | -1 = no overlay |
| +0xEC  | int  | LandType | 2 = water (determines crate image) |
| +0x11E | byte | OverlayData | Bits 7:4 = damage level, bits 3:0 = sub-data. For crates: if < 0x13, used as predetermined crate type index |

### RulesClass `[CrateRules]` Fields

| Offset | INI Key | Type | Default | Notes |
|--------|---------|------|---------|-------|
| +0x40  | `FreeMCV` | bool | false | Force MCV crate when player has no MCV and >1500 credits |
| +0xF8  | `WoodCrateImg` | OverlayTypeClass* | — | Overlay for land crates |
| +0xFC  | `CrateImg` | OverlayTypeClass* | — | Overlay for silver/default crates |
| +0x100 | `WaterCrateImg` | OverlayTypeClass* | — | Overlay for water crates |
| +0x718 | `HealCrateSound` | int (VocClass idx) | — | Sound when heal crate collected |
| +0x1140 | `SoloCrateMoney` | int | — | Fixed money amount for single-player crate |
| +0x1148 | `UnitCrateType` | UnitTypeClass* | NULL | Forced unit type for unit crate (overrides random) |
| +0x1464 | `SilverCrate` | int (crate type) | — | Fixed crate type for silver crate overlay |
| +0x1468 | `WoodCrate` | int (crate type) | — | Fixed crate type for wood crate overlay |
| +0x146C | `WaterCrate` | int (crate type) | — | Fixed crate type for water crate overlay |
| +0x1470 | `CrateMinimum` | int | — | Min crates on map (initial placement) |
| +0x1474 | `CrateMaximum` | int | — | Max crates on map (caps initial placement) |
| +0x1678 | `CrateRegen` | double (minutes) | — | Time before crate respawns after pickup |
| +0x172C | `CrateRadius` | int (leptons) | — | Area-of-effect radius for cloak/vet/armor/speed/firepower crates |

### Crate Sound Fields (RulesClass, read in `ReadAudioVisual`)

These are read from `[AudioVisual]` section, not `[CrateRules]`:

| INI Key | Purpose |
|---------|---------|
| `CratePromoteSound` | Veterancy crate |
| `CrateUnitSound` | Unit crate |
| `CrateSpeedSound` | Speed crate |
| `CrateArmourSound` | Armor crate |
| `CrateFireSound` | Firepower crate |
| `CrateRevealSound` | Reveal crate |
| `CrateMoneySound` | Money crate |
| `HealCrateSound` | Heal base crate (also in `[CrateRules]`) |

### OverlayTypeClass Crate Fields (read at `0x005fe7a0`)

| Offset | INI Key | Type | Notes |
|--------|---------|------|-------|
| +0x2A8 | (Explodes flag) | bool | Used in DestroyOverlay for chain reactions |
| +0x2AA | `Crate` | bool | Marks this overlay as a crate |
| +0x2AB | `CrateTrigger` | bool | Fires trigger action 0x31 when picked up |

### UnitTypeClass Crate Fields (read at `0x00747620`)

| Offset | INI Key | Type | Notes |
|--------|---------|------|-------|
| +0xE0D | `CrateGoodie` | bool | Unit can be spawned from unit crate |
| +0xE1A | `CarriesCrate` | bool | Unit drops a crate when destroyed |

### BuildingTypeClass Crate Fields (read at `0x0045fe50`)

| Offset | INI Key | Type | Notes |
|--------|---------|------|-------|
| +0x1767 | `CrateBeneath` | bool | Building leaves a crate when destroyed |
| +0x1769 | `CrateBeneathIsMoney` | bool | The crate-beneath contains money specifically |

---

## 3. Core Logic

### 3.1 Crate Slot Array

The crate system uses a **256-entry timer array** stored at offset +0x158 on a map-related global object. Each entry is 16 bytes:

```
struct CrateSlot {           // 16 bytes
    int  start_frame;        // +0x00: frame when timer started (-1 = paused)
    int  unknown;            // +0x04: high 32 bits of timer calculation
    int  remaining_frames;   // +0x08: frames until regen (0 = expired)
    int  cell_coord;         // +0x0C: packed (X:16, Y:16) or sentinel if empty
}
```

Sentinel value: `DAT_00abd480` — when `cell_coord` matches this, the slot is unused.

### 3.2 Initial Crate Placement (`ScenarioClass::Post_Map_Init`, `0x00686890`)

Only in multiplayer (`g_GameMode != 0`) with crates enabled (`DAT_00a8b261 != 0`):

```
count = max(CrateMinimum, DAT_00a8b54c)
count = min(count, CrateMaximum)
for i in 0..count:
    PlaceCrate()  // FUN_0056bd40
```

`DAT_00a8b54c` is the human session-player count; AI slots are held separately at
`DAT_00a8b274`. A stock one-human/seven-AI Skirmish therefore requests one crate.

### 3.3 Crate Placement (`FUN_0056bd40`, `0x0056bd40`)

```
1. Search 256 crate slots for an available one (matching sentinel)
2. If all 256 slots full → return 0 (can't place more)
3. Try up to 1000 random positions:
   a. Generate X and Y in `1..=Size.width + Size.height - 1`
   b. Check cell land type:
      - LandType == 2 (water) → use SpeedType 5 (amphibious), place WaterCrateImg
      - Otherwise → use SpeedType 1 (foot), place WoodCrateImg
   c. Call FootClass::Find_Nearby_Passable_Cell with zone -1, footprint 1x1,
      cap min(Size.width+Size.height, 32), target (0,0), bridges allowed,
      and no extra occupancy-rectangle check
   d. Call CrateSlot_Place (FUN_004a17c0) which:
      - Verifies cell is in playfield
      - Verifies cell has no existing overlay (overlay == -1)
      - Creates OverlayClass with appropriate image
      - Sets regen timer: frames = CrateRegen * 1800.0
      - Records start_frame = current frame
4. Return 1 on success, 0 on failure
```

**Timer conversion:** `DAT_007e44b8 = 1800.0` (double). This is `60 seconds × 30 ticks/second` at normal game speed.

### 3.4 Crate Pickup Dispatch (`CellClass::Can_Enter_Cell_General`, `0x00481a00`)

Called when a unit's locomotor enters a cell. Full flow:

```
1. Check cell has overlay with Crate=true flag
2. If unit's house has Civilian=true → skip (no crate for civilians)
3. If CrateTrigger=true → fire trigger action 0x31
4. Determine crate type:
   a. If OverlayData < 0x13 → use OverlayData as predetermined type index
   b. Else → weighted random from probability table (0x0081da8c)
5. Single-player override: crate type determined by overlay image
   (SilverCrate / WoodCrate / WaterCrate from [CrateRules])
6. Multiplayer guards (prevent stacking):
   - Type 1 (unit/MCV): if house money > 50 → force type 0 (money)
   - Type 3 (cloak): if already cloaked → force type 0
   - Type 6 (tiberium): if ore count > 100 → force type 0
   - Type 9 (armor): if armor already modified → force type 0
   - Type 10 (speed): if speed already modified → force type 0
   - Type 11 (firepower): if firepower already modified → force type 0
   - Water cell check: types not valid on water → force type 0
7. FreeMCV check: if no MCV owned AND credits > 1500 → force type 1 (unit)
8. Remove crate overlay (FUN_0056c020):
   - Single-player: directly clear cell overlay
   - Multiplayer: find matching crate slot, call CrateSlot_Clear
9. If multiplayer + crates enabled → immediately place new crate (FUN_0056bd40)
10. Execute crate effect (giant switch on type index)
11. Play anim from anim table (DAT_0081dad8) at crate location (+200 Z offset)
12. Return 1 (crate was picked up)
```

### 3.5 Crate Type Table

Jump table at `0x004833c4`, probability weights at `0x0081da8c`, anim indices at `0x0081dad8`:

| Idx | Type | Weight | Effect | Active in YR? |
|-----|------|--------|--------|---------------|
| 0 | Money | 50 | Add random credits (base + 0..900) | Yes |
| 1 | Unit | 20 | Spawn random unit (filtered by CrateGoodie=yes) | Yes |
| 2 | Heal Base | 1 | Heal all owned buildings to full HP | Yes |
| 3 | Cloak | 3 | Cloak all units within CrateRadius | Yes |
| 4 | Explosives | 5 | Damage picker + 5 random area blasts | Yes |
| 5 | Napalm | 5 | INTNAPALM anim + area damage | Yes |
| 6 | *(remapped)* | 20 | Forced to type 0 (money) before switch | Yes (redirected) |
| 7 | Shroud | 1 | Restore shroud for all players | Yes |
| 8 | Reveal | 1 | Reveal entire map for picking player | Yes |
| 9 | Armor | 10 | Multiply ArmorMult for nearby units (double at +0x158) | Yes |
| 10 | Speed | 10 | Multiply SpeedMult for nearby units (double at +0x580) | Yes |
| 11 | Firepower | 10 | Multiply FirepowerMult for nearby units (double at +0x160) | Yes |
| 12 | ICBM | 1 | Grant superweapon cameo via SidebarClass::AddCameo | Yes |
| 13 | *(unused)* | 3 | Falls through to anim-only | — |
| 14 | Veterancy | 1 | Promote nearby units (rookie→vet→elite) | Yes |
| 15 | *(unused)* | 1 | Falls through to anim-only | — |
| 16 | Poison Gas | 1 | Area damage on 8 adjacent cells | Yes |
| 17 | Tiberium/Ore | 1 | Spawn 10-20 ore patches around cell | Yes |

Total weight = 50+20+1+3+5+5+20+1+1+10+10+10+1+3+1+1+1+1 = **144**

### 3.6 Periodic Regen (`FUN_0056bbe0`, `0x0056bbe0`)

Called every tick from `Main_Tick` (via `FUN_0055afb0` at `0x0055afb0`).

```
if game_mode == 0 (single-player) OR crates_disabled:
    return  // no regen in single-player

for each of 256 crate slots:
    if slot is empty (sentinel):
        skip
    if timer still running:
        remaining = slot.remaining_frames - (current_frame - slot.start_frame)
        if remaining > 0:
            skip
    // Timer expired — remove old crate and place new one
    CrateSlot_Clear(slot)       // FUN_004a1750
    PlaceCrate()                // FUN_0056bd40
```

**Correction (2026-07-23 planning verification):** pickup clearing preserves the
remaining duration numerically, but it also writes the cell-coordinate sentinel
and sets `start_frame = -1`. `MapClass::UpdateCrateRegenTimers` tests the cell
sentinel first and skips empty slots, so that paused empty slot does **not**
later create an additional crate. Multiplayer pickup performs the immediate
`PlaceCrateAtRandomCell` replacement only. That helper scans from slot 0 and
uses the first sentinel slot, so the replacement may reuse the just-cleared
slot or an earlier free slot. Evidence: live read-only Ghidra
`batch_decompile(0x004a1750, 0x0056c020, 0x0056bbe0)` and
`decompile_function(0x0056bd40)` on 2026-07-23.

### 3.7 Crate Removal on Pickup (`FUN_0056c020`, `0x0056c020`)

**Single-player path (`g_GameMode == 0`):**
- Lookup CellClass by coordinates
- Verify overlay has `Crate=true`
- Clear overlay: `OverlayTypeIndex = -1`, `OverlayData = 0`
- Mark screen dirty

**Multiplayer path:**
- Search 256 crate slots for matching coordinates
- Call `CrateSlot_Clear` (`FUN_004a1750`):
  - Reset cell coord to sentinel
  - Preserve remaining timer frames
  - Set start_frame to -1 (paused)
  - The regen sweep skips this slot while its coordinates remain sentinel; the
    preserved duration is reused only if a later placement initializes that
    slot, rather than scheduling a second delayed replacement

---

## 4. INI Keys

### `[CrateRules]` Section

| Key | Type | Default | Effect |
|-----|------|---------|--------|
| `FreeMCV` | bool | false | Force unit crate when no MCV owned and credits > 1500 |
| `CrateMinimum` | int | — | Minimum crates placed at game start |
| `CrateMaximum` | int | — | Maximum crates on map simultaneously |
| `CrateRadius` | int (leptons) | — | AoE radius for cloak/vet/armor/speed/firepower |
| `CrateRegen` | double (minutes) | — | Respawn delay after pickup |
| `CrateImg` | string (overlay name) | — | Silver/default crate overlay |
| `WoodCrateImg` | string (overlay name) | — | Wood (land) crate overlay |
| `WaterCrateImg` | string (overlay name) | — | Water crate overlay |
| `HealCrateSound` | string (sound name) | — | Sound for heal base crate |
| `SilverCrate` | int (type index) | — | Fixed type for silver crate (single-player) |
| `WoodCrate` | int (type index) | — | Fixed type for wood crate (single-player) |
| `WaterCrate` | int (type index) | — | Fixed type for water crate (single-player) |
| `UnitCrateType` | string (unit name) | — | Forced unit type for unit crate |
| `SoloCrateMoney` | int | — | Fixed money amount for single-player money crate |

### `[AudioVisual]` Section (Crate Sounds)

| Key | Effect |
|-----|--------|
| `CratePromoteSound` | Veterancy crate pickup |
| `CrateUnitSound` | Unit crate pickup |
| `CrateSpeedSound` | Speed crate pickup |
| `CrateArmourSound` | Armor crate pickup |
| `CrateFireSound` | Firepower crate pickup |
| `CrateRevealSound` | Reveal crate pickup |
| `CrateMoneySound` | Money crate pickup |

### Per-Type INI Keys

| Section | Key | Type | Offset | Notes |
|---------|-----|------|--------|-------|
| `[UnitType]` | `CrateGoodie` | bool | +0xE0D | Unit can be spawned from crate |
| `[UnitType]` | `CarriesCrate` | bool | +0xE1A | Unit drops crate on death |
| `[BuildingType]` | `CrateBeneath` | bool | +0x1767 | Building leaves crate on destruction |
| `[BuildingType]` | `CrateBeneathIsMoney` | bool | +0x1769 | Crate-beneath is money type |
| `[OverlayType]` | `Crate` | bool | +0x2AA | Overlay is a crate |
| `[OverlayType]` | `CrateTrigger` | bool | +0x2AB | Fires trigger when picked up |

---

## 5. Integration Points

### Who calls the crate system?

| Caller | Function | When |
|--------|----------|------|
| `DriveLocomotionClass::Process_Movement` | `CellClass::Can_Enter_Cell_General` | Vehicle enters cell |
| `DriveLocomotionClass::Force_Track` | same | Vehicle forced onto track |
| `DriveLocomotionClass::Process_Drive_Track` | same | Vehicle processes drive track |
| `ShipLocomotionClass::Process_Movement` | same | Ship enters cell |
| `ShipLocomotionClass::Process_Drive_Track` | same | Ship processes drive track |
| `WalkLocomotionClass::FindSubCellDest` | same | Infantry enters cell |
| `TeleportLocomotionClass::InitiateWarp` | same | Chrono unit teleports to cell |
| `Main_Tick → FUN_0055afb0` | `FUN_0056bbe0` | Every game tick (regen timers) |
| `ScenarioClass::Post_Map_Init` | `FUN_0056bd40` | Initial crate placement |

### What does the crate system call?

| Function | Purpose |
|----------|---------|
| `OverlayClass::Constructor` | Create crate overlay on cell |
| `AnimClass::Constructor` | Visual feedback animation after pickup |
| `VocClass::PlayAt` | Spatial sound effect |
| `VoxClass::PlayEVA` | EVA voice lines (armor/speed/firepower) |
| `HouseClass::Add_Credits` | Money crate |
| `HouseClass::CountOwnedInstances` | Check MCV ownership for FreeMCV |
| `MapClass::BlackoutShroud` / `RestoreShroud` | Reveal/shroud crates |
| `SidebarClass::AddCameo` | ICBM crate grants superweapon |
| `VeterancyStruct::SetVeteran/SetElite` | Veterancy crate promotions |
| `Apply_area_damage` | Napalm, poison gas, explosives |
| `FootClass::Find_Nearby_Passable_Cell` | Validate cell for crate/unit placement |
| `Random::RandomRanged` | Type selection, cell placement, money amounts |

### Tick cycle position

From `Main_Tick` (`FUN_0055afb0`), crate regen runs **after**:
- Trigger processing
- Ore/tiberium growth timer
- Lightning storm timer
- Object AI updates (all TechnoClass::AI calls)
- AlphaShape purge

And **before**:
- Tactical view update
- Factory AI updates
- HouseClass AI updates

---

## 6. Current Rust Implementation Status

**Not implemented.** No crate-related code exists in the Rust codebase. The overlay system (`src/`) has overlay rendering but no gameplay logic for crate pickup, spawning, or effects.

---

## 7. Open Questions

1. **DAT_00a8b54c** — Referenced in `Post_Map_Init` as an alternative minimum. Likely a session/multiplayer settings value. Need to trace where it's written to confirm. **Confidence: LOW**

2. **CrateBeneath trigger** — How exactly does `CrateBeneath=true` on a BuildingTypeClass cause a crate to appear when the building is destroyed? The flag is read but the destruction → crate placement path was not traced. **Confidence: LOW**

3. **CarriesCrate trigger** — Similarly, `CarriesCrate=true` on UnitTypeClass should drop a crate when the unit dies. The death → crate placement path was not traced. **Confidence: LOW**

4. **Index 6 remap semantics** — Crate type index 6 is always remapped to 0 (money) in multiplayer after `LAB_00481d86`. Its weight of 20 effectively adds to money's probability. Is this intentional design or a vestigial TS behavior? **Confidence: MEDIUM**

5. **Anim table entries** — The anim index table at `0x0081dad8` maps each crate type to an AnimTypeClass index. Some entries are -1 (no anim). The exact mapping for all 18+ entries was not fully extracted. **Confidence: MEDIUM**

6. **Random jitter on timer** — `FUN_004a17c0` calls `Random::RandomRanged(0, 0x7FFFFFFE)` during timer setup. The decompiler lost track of whether this modifies the timer value via x87 FPU state. The timer might be `CrateRegen * 1800 + random_jitter` rather than just `CrateRegen * 1800`. **Confidence: MEDIUM**

7. **OverlayData byte as crate type** — When `CellClass.OverlayData < 0x13` (19), it's used directly as the crate type index instead of random selection. This is how map-placed crates in single-player missions can have predetermined contents. Confirm this path is also active in multiplayer for pre-placed overlays. **Confidence: MEDIUM**

---

## Sources

### Ghidra Functions Decompiled
- `0x00481a00` — `CellClass::Can_Enter_Cell_General` (pickup dispatch, 788 lines)
- `0x0056bd40` — Crate placement (find random cell)
- `0x0056bbe0` — Crate regen timer update
- `0x0056c020` — Crate removal on pickup
- `0x004a17c0` — Crate slot: place + init timer
- `0x004a1750` — Crate slot: clear
- `0x004a18f0` — Validate cell for crate placement
- `0x004a1aa0` — Remove crate overlay from cell
- `0x0066b900` — `RulesClass::ReadCrateRules`
- `0x005fe7a0` — `OverlayTypeClass::ReadINI`
- `0x00747620` — `UnitTypeClass::ReadINI`
- `0x0045fe50` — `BuildingTypeClass::ReadINI` (CrateBeneath area)
- `0x00686890` — `ScenarioClass::Post_Map_Init`
- `0x0055afb0` — `Main_Tick` (calls crate regen)
- `0x00480cb0` — `CellClass::DestroyOverlay`
- `0x007660f0` — WDT crate game option

### Memory Inspected
- `0x0081da8c` — Crate probability weight table (19 int32 entries)
- `0x0081dad8` — Crate anim index table (19 int32 entries, -1 = no anim)
- `0x004833c4` — Crate type switch jump table (18 addresses)
- `0x007e44b8` — Minutes-to-frames conversion constant (double: 1800.0)

### INI Files Checked
- `ini/rulesmd.ini` — `[CrateRules]` section
- `ini/rules.ini` — base RA2 values

---

## 8. Follow-up Pass (2026-04-21) — Resolving Open Questions

This pass focused on the three LOW-confidence items in §7 plus two MEDIUM items.
All findings below were verified against live `gamemd.exe` via Ghidra MCP (decompilation + raw x86 + memory reads).

### 8.1 CrateBeneath destruction path (Q7.2 resolved)

The `CrateBeneath` flag is consumed in **`BuildingClass::Place_OccupyMap` at `0x00441f60`**.
Despite the name, this function also runs on the **destroy / un-place** side: it is the final
step that clears the building's occupied cells and, critically, the tail of the function
drops a crate when the building's type has `CrateBeneath=true`.

Verbatim tail of the decompile (addresses preserved):

```c
if (*(char *)(param_1[0x148] + 0x1767) != '\0') {     // BuildingTypeClass->CrateBeneath
    piVar4 = (int *)(**(code **)(*param_1 + 0xac))(local_20);  // get building center coord
    uVar10 = CONCAT22( ...piVar4[1]>>8..., ...*piVar4>>8... );  // → packed cell X,Y
    if (*(char *)(param_1[0x148] + 0x1769) != '\0') {  // CrateBeneathIsMoney
        WallOverlay_HeightAdjust(uVar10, 0);           // OverlayData = 0 → type 0 (Money)
        return;
    }
    WallOverlay_HeightAdjust(uVar10, 0x14);            // OverlayData = 20 → falls through to random
}
```

Confirmed facts:

- **`param_1` (BuildingClass `this`) is a genuine pointer** (`int *`). `param_1[0x148]` is
  `*(int *)(this + 0x520)` — the TechnoClass's Type pointer — so `+0x1767` and `+0x1769`
  are direct BuildingTypeClass byte offsets. Matches the existing §2 table.
- The function called through `vtable + 0xac` returns the building's center cell coord.
- The helper `WallOverlay_HeightAdjust` at **`0x0056bec0`** is mislabeled in Ghidra; its
  callers prove it is the **"place a crate at a specific cell"** entry point (see §8.2).
  Its second argument is written into `CellClass.OverlayData` (`+0x11E`), which §3.4 step 4a
  already documents: when `OverlayData < 0x13 (19)`, it is used **directly** as the crate-type
  index in the pickup dispatch. `0` forces a Money crate; `0x14 (20)` is `≥ 0x13`, so the
  pickup dispatch discards it and re-rolls via the weighted random table.
- No `SpecialFlags` or TS-legacy gating: the only guards are `CrateBeneath=true` on the
  type, and (inside `WallOverlay_HeightAdjust`) the multiplayer-crates-enabled flag
  `DAT_00a8b261` and a free crate-slot in the 256-entry table.
- Net effect: a destroyed building **always** spawns a crate in MP if `CrateBeneath=yes`.
  In single-player this path still runs, but `FUN_0056bec0` itself does not hard-gate
  on `g_GameMode`; however because it uses the shared 256-slot array (initial-place path)
  and in SP the array is sparsely seeded, the result is effectively one-shot.

**Caller map of `FUN_0056bec0` / "PlaceCrateAtCell"** (from `get_function_callers`):

| Caller | Address | Purpose | Type arg |
|--------|---------|---------|----------|
| `BuildingClass::Place_OccupyMap` | `0x00441f60` | Building destruction (CrateBeneath) | `0` or `0x14` |
| `UnitClass::ReceiveDamage` | `0x00737c90` | Unit death (CarriesCrate) | *(not passed; defaults)* |
| `TriggerAction__Execute` | `0x006dd8b0` case `0x6c` | Map trigger action "Drop Crate @ Waypoint" | from trigger data |

**Confidence: HIGH** — both the producer flag (read at `00460ec1` / `00460edb`, decompiled at
`0x0045fe50`) and the consumer call site (`0x00441fce`, `0x00441fda`, `0x00441fe0`) are
verified from the decompile; the OverlayData convention is corroborated by §3.4 step 4a.

### 8.2 CarriesCrate death path (Q7.3 resolved)

The `CarriesCrate` flag (UnitTypeClass `+0xE1A`) is consumed in
**`UnitClass::ReceiveDamage` at `0x00737c90`**, near the end of the death branch
(label `LAB_0073838a:`):

```c
puVar10 = param_1[10].vtable_INoticeSource;                // puVar10 is a byte* (correct for +0xE1A)
if ((puVar10[0xe1a] != '\0') &&                            // UnitType->CarriesCrate
   (((*(char *)(DAT_00a8b230 + 0x34b1) != '\0' && (puVar10[0xc94] == '\0')) ||  // land branch
    ((*(char *)(DAT_00a8b230 + 0x34a5) != '\0' && (puVar10[0xc94] != '\0')))))) // water branch
{
    uVar7 = (**(code **)(param_1->vtable + 0x1b8))(&param_5, 1, 0xffffffff, ...);
    piVar3 = (int *)FootClass__Find_Nearby_Passable_Cell(&param_7, uVar7, ...);
    iVar6 = *piVar3;
    param_6 = iVar6;
    if ((((short)iVar6 != (short)DAT_00b1cfb8) ||
        ((bVar11 = param_6._2_2_ != DAT_00b1cfb8._2_2_), bVar11)) &&
        (DAT_00a8b261 != '\0'))                            // MP crates-enabled
    {
        WallOverlay_HeightAdjust(iVar6);                   // 1-arg call: OverlayData defaults → random
    }
}
```

Confirmed facts:

- `puVar10` is a byte pointer (`undefined *vtable_INoticeSource`), so `puVar10[0xE1A]` is
  the raw byte at offset 0xE1A — a direct byte offset, **not** an int-index. Matches §2.
- Two additional global flags gate the behaviour:
  - `DAT_00a8b230 + 0x34b1` — required when the unit is **not water-capable** (`+0xC94` is
    the amphibious/naval locomotor flag on the UnitType).
  - `DAT_00a8b230 + 0x34a5` — required when the unit **is** water-capable.
  - These flags are both `bool`s on the scenario/session block at `DAT_00a8b230`. They are
    toggled by map-trigger actions `0x55` (+0x34A7), `0x56` (+0x34A6), `0x57` (+0x34A8),
    and `0x61` (XOR-toggle of +0x34A5) — see `TriggerAction__Execute`. Defaults appear to be
    **true** in a vanilla skirmish (no trigger action ever fires, flags retain startup
    value). Not traced far enough to verify the default byte; see "remaining gap" below.
- `DAT_00a8b261` (CratesEnabled) is the same MP game-option flag used by regen (§3.6) and
  Post_Map_Init (§3.2). If crates are disabled in the lobby, no unit drops happen.
- The cell coord is chosen via `FootClass::Find_Nearby_Passable_Cell`, not the unit's exact
  cell — this avoids dropping on the crater/wreck. `DAT_00b1cfb8` is the "invalid coord"
  sentinel (`0xFFFFFFFF` pair), so the `!= sentinel` check verifies a cell was found.
- `WallOverlay_HeightAdjust` is called with **one argument** here, not two — meaning the
  `OverlayData` byte is left as its default (0 from `CellClass` init or whatever was on
  that cell). In practice `Find_Nearby_Passable_Cell` returns an empty cell, so OverlayData
  starts at 0 → forces **Money crate** most of the time. (Worth spot-checking in
  live play; could also fall through to random if the callee treats "no override" as
  `>= 0x13`.)

**TS-legacy check:** `CarriesCrate` existed in TS too. The gating flags at `+0x34A5` /
`+0x34B1` are in the scenario block and can be set by trigger actions — but they are
**not** `SpecialFlags` bits and do not default to false in YR. They are live.
I verified the `UnitClass::ReceiveDamage` caller lives in the non-trampoline region
(`0x007379a0` range is real code) and is reachable from the virtual-dispatch death path
in `TechnoClass::ReceiveDamage`. **This path is live in a normal YR skirmish.**

**Remaining (minor) gap:** default startup values of `DAT_00a8b230 + 0x34A5` and `+0x34B1`.
They are initialized somewhere in `ScenarioClass::Read_INI` or session reset. Not traced
this pass — mechanically irrelevant for a baseline implementation (assume `true`) but
worth verifying before shipping scenario-trigger support.

**Confidence: HIGH** for the mechanism, field offset, and trigger. **MEDIUM** for the
default values of the two gate bytes.

### 8.3 DAT_00a8b54c writer (Q7.1 resolved)

`DAT_00a8b54c` is the **current multiplayer player count** (`Session::NumPlayers`).
It is **not** static/default memory, not a mod-loaded setting, and not a crate-specific
value. It is the live count of connected human + AI player slots.

Writers found via `get_xrefs_to(0x00a8b54c)`:

| Address | Function | Operation |
|---------|----------|-----------|
| `0x005e7486` | `FUN_005e7460` (pregame setup) | `DAT_00a8b54c = DAT_00a8da84;` where `DAT_00a8da84` is the player-array count |
| `0x005bad22` | `FUN_005bac60` (modem host game-start) | `DAT_00a8b54c = 0;` then later `DAT_00a8b54c = DAT_00a8da84;` |
| `0x005bb1f0` | same | set to player count on session enter |
| `0x005e1c43` | `FUN_005e00b0` (netdlg2 message 0xB / 0xE handler — "game starting") | `DAT_00a8b54c = DAT_00a8da84;` |
| `0x005dab60` | `FUN_005da750` (player-leave handler) | `DAT_00a8b54c = DAT_00a8b54c + -1;` |
| `0x005c4cb3`, `0x005c4e6f` | unnamed session ctor | init/reset |
| `0x00792f18`, `0x00792ffb`, `0x00793009` | internet/WOL session code | various |
| `0x00686890` | `ScenarioClass::Post_Map_Init` | READ only — used as the min clamp |
| `0x0056bbec` | `MapClass::UpdateCrateRegenTimers` | READ only — no, actually this reads a **different** symbol; see note below |

(Note: the `0x0056bbec` reference in the xref list may belong to the adjacent
`DAT_00a8b261` (CratesEnabled) in the same cache line — the Ghidra xref index sometimes
groups nearby bytes. `FUN_0056bbe0` decompile shows no direct use of `0x00a8b54c`.)

So §3.2's `count = max(CrateMinimum, DAT_00a8b54c)` means: "at least one crate per
multiplayer player". `CrateMinimum` guards the lower-bound for tiny lobbies, and the
player-count clamp scales initial crate density with lobby size. Makes sense.

**Single-player / skirmish-vs-AI:** `DAT_00a8b54c` is set to the full seat count including
the human and every AI slot by `FUN_005e7460`/`FUN_005e00b0`, so e.g. a 1v7 skirmish
starts with at least 8 crates (assuming `CrateMaximum` allows).

**Confidence: HIGH** — multiple independent writers all assign `DAT_00a8da84` (player
array length) and a decrement in the leave handler. This is unambiguously
`Session::NumPlayers`.

Recommended Ghidra rename: `DAT_00a8b54c` → `g_Session_NumPlayers`.
**Not applied** in this pass — 90 %-confidence bar met, but the user's CLAUDE.md says
to leave rename decisions to follow-up batches. Flagged here for reviewer approval.

### 8.4 Crate regen random jitter (Q7.6 resolved)

The decompile lost the FPU state; reading the raw x86 at `FUN_004a17c0` (`0x004A185B`
through `0x004A18B5`) makes it unambiguous:

```asm
004a185b  MOV   EAX, [0x008871e0]             ; EAX = &g_RulesClass_Instance deref
004a1860  MOV   EDX, dword ptr [0x00a8b230]   ; scenario block (unused here)
004a1868  FLD   double ptr [EAX + 0x1678]     ; ST0 = CrateRegen  (minutes, field +0x1678)
004a186e  FMUL  double ptr [0x007e44b8]       ; ST0 *= 1800.0
004a187a  FSTP  double ptr [ESP + 0x20]       ; upper_bound = CrateRegen * 1800
004a187e  FLD   double ptr [EAX + 0x1678]     ; reload CrateRegen
004a1884  FMUL  double ptr [0x007e5d78]       ; ST0 *= 450.0
004a188a  FSTP  double ptr [ESP + 0x18]       ; lower_bound = CrateRegen * 450
004a188e  CALL  Random__RandomRanged(0, 0x7fffffe)
004a1893  FLD   double ptr [ESP + 0x18]       ; ST0 = lower_bound
004a1897  FSUB  double ptr [ESP + 0x10]       ; ST0 = lower - upper           (negative)
004a18a5  FILD  dword ptr [ESP + 0x28]        ; ST0 = random int (saved from EAX)
004a18a9  FMUL  double ptr [0x007e3570]       ; ST0 *= (1.0 / 0x7fffffff)  ≈ normalize to [0,1]
004a18af  FMULP                               ; ST0 = rand01 * (lower - upper)
004a18b1  FADD  double ptr [ESP + 0x10]       ; ST0 = upper + rand01 * (lower - upper)
004a18b5  CALL  Math__ftol                    ; → EAX = final timer in frames (integer)
```

Memory constants (read back via `read_memory`):

| Address | Bytes (LE) | Double value | Meaning |
|---------|------------|--------------|---------|
| `0x007e44b8` | `00 00 00 00 00 20 9c 40` | `1800.0` | minutes → frames (30 fps × 60 sec) |
| `0x007e5d78` | `00 00 00 00 00 20 7c 40` | `450.0`  | minutes → min frames (= 1800/4) |
| `0x007e3570` | `00 00 40 00 00 00 00 3e` | `4.6566e-10` | `1.0 / 0x7FFFFFFE` (normalizer) |

Formula (verified from the assembly, with `upper = CR*1800`, `lower = CR*450`):

```
rand01        = RandomRanged(0, 0x7FFFFFFE) / 0x7FFFFFFE      ; uniform in [0, 1]
timer_frames  = upper + rand01 * (lower - upper)
              = CrateRegen * (1800 - rand01 * 1350)
              = CrateRegen * [450 .. 1800]                    ; uniform over that range
```

So with `CrateRegen=1.3` (default in `rulesmd.ini`):

- min timer: `1.3 * 450 = 585 frames` = **19.5 s** at 30 fps
- max timer: `1.3 * 1800 = 2340 frames` = **78 s** at 30 fps
- mean:      `1.3 * 1125 = 1462.5 frames` ≈ **48.75 s**

This **contradicts** the original §3.3 / §3.6 claim that the period is exactly
`CrateRegen * 1800`. The existing doc's "Timer conversion" sentence is the **upper bound**,
not the actual value. The `_DAT_007e44b8 = 1800.0` constant is real, but there is a
second constant `0x007e5d78 = 450.0` and jitter normalizer `0x007e3570`.

**Replay/lockstep implication:** the RNG call DOES happen (single call to
`RandomRanged(0, 0x7FFFFFFE)` per crate placement, advancing the deterministic RNG state).
A reimplementation that uses a fixed period `CrateRegen * 1800` would diverge after the
first crate spawn in any MP game.

**Confidence: HIGH** — verified from raw assembly and memory-read constants.

### 8.5 Full crate anim table (Q7.5 resolved)

The table at `0x0081dad8` is **not a static compile-time table**. It is populated at
runtime from `rulesmd.ini [Powerups]` by **`FUN_00673e80`** (the `[Powerups]` INI parser).

Verbatim from the decompile (address `0x00673fXX` area):

```c
iVar1 = FUN_00526810(PTR_s_Powerups_007f0cd8);    // Powerups section handle
iVar1 = 0;
do {
    // Read "Name=weight,anim,sound,data"
    iVar2 = CCINIClass__ReadString(..., (&PTR_s_Money_007e523c)[iVar1], ...);
    if (iVar2 != 0) {
        iVar2 = CRT__strtok(local_80, ",");
        if (iVar2 != 0) (&DAT_0081da8c)[iVar1] = atoi(iVar2);   // weight → table A
        iVar2 = CRT__strtok(0, ",");
        if (iVar2 != 0) (&DAT_0081dad8)[iVar1] = FUN_00422b20();  // anim name → AnimType idx
        iVar2 = CRT__strtok(0, ",");
        // sound yes/no → &DAT_0089ecc0[i]
        iVar2 = CRT__strtok(0, ",");
        // numeric data (armor/firepower/speed/money mults) → double* pdVar5
    }
    pdVar5++;
    iVar1++;
} while (pdVar5 < 0x89ecc0);
```

The name table `PTR_s_Money_007e523c` is the key-name array:
`Money, Unit, HealBase, Cloak, Explosion, Napalm, Squad, Darkness, Reveal, Armor,
Speed, Firepower, ICBM, Invulnerability, Veteran, IonStorm, Gas, Tiberium, Pod` (19 entries,
matching the §3.5 table).

**Memory read at `0x0081dad8` (uninitialized)** returned 18 × `0xFFFFFFFF` — that's just
the pre-init image. At runtime the real values are whatever `FUN_00422b20` returned for
the anim name in each `[Powerups]` line.

Resolving against `ini/rulesmd.ini [Powerups]`:

| Idx | Key | Weight | Anim name | Sound | Data | Anim resolved? |
|-----|-----|--------|-----------|-------|------|----------------|
| 0 | `Money` | 20 | `MONEY` | yes | 2000 | Yes |
| 1 | `Unit` | 20 | `<none>` | no | — | **−1** (no anim) |
| 2 | `HealBase` | 10 | `HEALALL` | yes | — | Yes |
| 3 | `Cloak` | 0 | `CLOAK` | yes | — | Yes (but weight=0 = disabled) |
| 4 | `Explosion` | 0 | `<none>` | yes | 500 | **−1** (weight=0) |
| 5 | `Napalm` | 0 | `<none>` | no | 600 | **−1** (weight=0) |
| 6 | `Squad` | 0 | `<none>` | no | — | **−1** (weight=0) |
| 7 | `Darkness` | 0 | `SHROUDX` | yes | — | Yes (weight=0) |
| 8 | `Reveal` | 10 | `REVEAL` | yes | — | Yes |
| 9 | `Armor` | 10 | `ARMOR` | yes | 1.5 | Yes |
| 10 | `Speed` | 10 | `SPEED` | yes | 1.2 | Yes |
| 11 | `Firepower` | 10 | `FIREPOWR` | yes | 2.0 | Yes |
| 12 | `ICBM` | 0 | `CHEMISLE` | yes | — | Yes (weight=0) |
| 13 | `Invulnerability` | 0 | `ARMOR` | yes | 1.0 | Yes (weight=0) |
| 14 | `Veteran` | 20 | `VETERAN` | yes | 1 | Yes |
| 15 | `IonStorm` | 0 | `<none>` | yes | — | **−1** (weight=0) |
| 16 | `Gas` | 0 | `<none>` | yes | 100 | **−1** (weight=0) |
| 17 | `Tiberium` | 0 | `<none>` | no | — | **−1** (weight=0) |
| 18 | `Pod` | 0 | `<none>` | no | — | **−1** (weight=0) |

**Correction to §3.5 table:** the index-→-effect mapping in §3.5 was based on the switch
jump table at `0x004833c4`, which is a C-compile-time fixed layout of 18 entries. But
that **is not** the index ordering used here. The INI `[Powerups]` array ordering above
is the actual runtime index — and it's different. This means §3.5's "Idx | Type" column
labels are probably correct **for the jump-table ordering**, but the weight column
(coming from `DAT_0081da8c`) and anim column (coming from `DAT_0081dad8`) are aligned
to the `[Powerups]` INI ordering. A reimplementation should:

- Parse `[Powerups]` in file order, indexed 0..18 matching the `PTR_s_Money_007e523c` name array
- Use that index into the weight table `DAT_0081da8c` for probability selection
- Use the same index into `DAT_0081dad8` for the anim lookup
- Use the same index into the jump table `DAT_004833c4` for the effect dispatch

**Live weights in stock YR** (from rulesmd.ini above):
`20,20,10,0,0,0,0,0,10,10,10,10,0,0,20,0,0,0,0` → total **100** (not 144 as §3.5 computed).
§3.5 used outdated RA2 base values. Re-check against `ini/rules.ini` if pure-RA2 weights
are wanted:

```
grep-after [Powerups] ini/rules.ini
```

…which the reimplementation MUST do, since these are INI-driven.

**Confidence: HIGH** — the dynamic-load mechanism is verified from decomp, and the
index/weight mapping is cross-referenced against shipping rulesmd.ini.

### 8.6 Ancillary finding — index 6 remap (Q7.4, partial)

Not the primary focus of this pass, but worth noting: with the corrected §8.5 mapping,
index 6 is `Squad`, not a vestigial type. Its weight defaults to 0 in YR, which is why
the pickup dispatch's `LAB_00481d86` forcibly remaps any index-6 pickup to type 0 (Money)
in multiplayer — it's a safety net for maps/mods that set `Squad=` non-zero.

The remap is not TS-legacy; it's deliberate protection because `Squad` (TS dropship
squad) was never re-wired to a YR equivalent. **Confidence: MEDIUM** (the remap existence
was already documented; the TS-legacy attribution is inferred, not decompile-verified).

### 8.7 Summary of confidence changes

| Open Q | Topic | Before | After |
|--------|-------|--------|-------|
| 7.1 | `DAT_00a8b54c` writer | LOW | **HIGH** — it's `Session::NumPlayers` |
| 7.2 | CrateBeneath destruction path | LOW | **HIGH** — `BuildingClass::Place_OccupyMap` tail |
| 7.3 | CarriesCrate death path | LOW | **HIGH** — `UnitClass::ReceiveDamage` tail (mechanism). Scenario flag defaults still MEDIUM |
| 7.4 | Index-6 remap | MEDIUM | **MEDIUM** (unchanged; §8.6 clarifies semantics) |
| 7.5 | Anim table entries | MEDIUM | **HIGH** — table is INI-driven, mechanism & mapping resolved |
| 7.6 | Regen timer jitter | MEDIUM | **HIGH** — confirmed jittered, formula extracted |

### 8.8 New items discovered

- **`FUN_0056bec0`** (currently mislabeled `WallOverlay_HeightAdjust`) is the shared
  "place a crate at a specific cell" entry point. Three callers:
  building-destruction, unit-death, trigger action 0x6C (Drop Crate at Waypoint).
  Recommended rename: `MapClass__PlaceCrateAtCell`. Not applied this pass.
- **`FUN_00673e80`** is the `[Powerups]` INI reader. Populates three parallel arrays:
  `DAT_0081da8c` (weights), `DAT_0081dad8` (anim indices), `DAT_0089ecc0` (sound flags),
  and a doubles array starting at `DAT_0089ec28` (effect magnitudes). Recommended rename:
  `RulesClass__ReadPowerups`. Not applied.
- **Timer constant `0x007e5d78 = 450.0`** is the regen-jitter lower bound. Not previously
  documented.
- **Scenario-block byte flags `+0x34A5`, `+0x34A6`, `+0x34A7`, `+0x34A8`, `+0x34B1`**
  control trigger-driven gameplay toggles. `+0x34A5` and `+0x34B1` gate water-unit and
  land-unit crate drops respectively. Worth cataloguing in a separate "scenario flag"
  report.

---

## 9. Verification Pass (2026-07-19) — Canonical Ordering, RNG Instance, Jump-Table Resolution + IMPLEMENTATION HANDOFF

This pass re-verified the load-bearing facts against live `gamemd.exe` (name array, weight
table, selection loop, RNG receiver, effect jump table, money formula), corrected one
arithmetic error in §8.5, resolved the two weight-0 "mystery" indices, and adds the missing
**implementation handoff** for the Rust port. Every claim cites its Ghidra MCP call inline.
Confidence for everything in §9.1–§9.6: **HIGH — verified from binary this session.**

### 9.1 Canonical crate-type index order (VERIFIED)

The runtime index is fixed by the internal name array `PTR_s_Money_007e523c`, NOT by
`[Powerups]` file order. `RulesClass__ReadPowerups` (`decompile_function 0x00673e80`) loops
`i = 0..18`, reading `[Powerups]` key `nameArray[i]`, writing weight→`(&DAT_0081da8c)[i]`,
anim→`(&DAT_0081dad8)[i]`, sound-flag→`(&DAT_0089ecc0)[i]`, data-magnitude→`(&DAT_0089ec28)[i]`
(a `double`; loop bound `while ((int)pdVar5 < 0x89ecc0)` = exactly 19 entries).

Name array read via `read_memory 0x007e523c` (19 pointers) → each string via
`read_memory` (0x0081d98c block, 0x0081746c, 0x00817278):

| Idx | Key | Idx | Key | Idx | Key |
|----|-----|----|-----|----|-----|
| 0 | Money | 7 | Darkness | 14 | Veteran |
| 1 | Unit | 8 | Reveal | 15 | IonStorm |
| 2 | HealBase | 9 | Armor | 16 | Gas |
| 3 | Cloak | 10 | Speed | 17 | Tiberium |
| 4 | Explosion | 11 | Firepower | 18 | Pod |
| 5 | Napalm | 12 | ICBM | | |
| 6 | Squad | 13 | Invulnerability | | |

This index feeds all four parallel arrays AND the effect jump table `0x004833c4` — a single
enum drives selection, magnitude, sound, anim, and dispatch.

### 9.2 Weight table — static default vs. INI-loaded (CORRECTS §8.5)

`read_memory 0x0081da8c` (76 bytes) returned the **compile-time defaults** (present before
`[Powerups]` is parsed): `[50,20,1,3,5,5,20,1,1,10,10,10,1,3,1,1,1,1,1]`, sum **144** — these
are the values §3.5 tabulated. At runtime `RulesClass__ReadPowerups` overwrites them from
`ini/rulesmd.ini [Powerups]`. Mapped by name into canonical index order, the **stock-YR live
weights** are:

`[20,20,10,0,0,0,0,0,10,10,10,10,0,0,20,0,0,0,0]` → sum **110**.

> **Correction:** §8.5 stated the stock total is "100". It is **110**
> (20+20+10+10+10+10+10+20). A reimplementation MUST sum the parsed weights, not hardcode
> either constant.

The effect-magnitude array (`read_memory 0x0089ec28`, 152 bytes) is all-zero in the static
image — it is INI-populated. The authoritative magnitudes are the `[Powerups]` 4th field:
Money 2000, Explosion 500, Napalm 600, Armor 1.5, Speed 1.2, Firepower 2.0, Invulnerability
1.0, Veteran 1, Gas 100 (others absent → 0).

### 9.3 Selection algorithm + RNG instance (VERIFIED — determinism-critical)

From `disassemble_function 0x00481a00` at `0x00481ad3`–`0x00481b1a`:

```
; total = sum of all 19 weights
00481ad5  MOV ECX, 0x81da8c
00481ada  loop: EAX += [ECX]; ECX += 4; cmp ECX,0x81dad8; jl loop      ; sum 19 ints
00481ae9  MOV EDX, [0x00a8b230]          ; EDX = scenario block ptr
00481aef  PUSH EAX                        ; max = total
00481af0  PUSH 1                          ; min = 1
00481af2  LEA ECX, [EDX + 0x218]          ; ECX(this) = ScenarioClass::Random  <<< SYNCED RNG
00481af8  CALL 0x0065c7e0                 ; RandomClass::RandomRanged(1, total)
; then walk cumulative weights, break when roll <= running_sum → index in EDI
```

**RNG instance = `Scen->Random` = `*(0x00a8b230) + 0x218`**, invoked via
`RandomClass::RandomRanged` at `0x0065c7e0` (a thiscall the decompiler renders as the bare
`Random__RandomRanged(min,max)`). This is the **network-synchronized scenario RNG**, the same
stream ore-growth/lightning use — NOT `g_MainRng`. Confirmed at the same receiver for:
placement X/Y (`0x0056bd90`, `0x0056bdaa`), tiberium type/count/offset (`0x00481eb2`,
`0x00481ee7`, `0x00481f00`), and free-unit selection (`0x0048216b`). Every crate roll advances
`Scen->Random`. **A port that uses any other RNG stream, or a different roll count, diverges
in lockstep after the first crate event.**

Selection: `roll = RandomRanged(1, total)`; iterate weights accumulating `running`; the first
index where `roll <= running` wins. (Standard inclusive-cumulative weighted pick.)

### 9.4 Post-roll fixups (VERIFIED)

- **Predetermined type:** if `CellClass+0x11E (OverlayData) < 0x13`, the roll is skipped and
  OverlayData IS the index (`0x00481ace CMP BL,0x13 / JC`). This is how map-placed and
  death-drop crates carry fixed contents.
- **Squad(6) → Money(0):** `0x00481db8 CMP EBX,0x6 / JNZ / XOR EBX,EBX` — after
  remove+replace, index 6 is forced to 0 unconditionally (SP and MP).
- **Pod(18) and any idx > 0x11:** `0x00481dca CMP EBX,0x11 / JA 0x004832f5` — skip the whole
  effect switch, fall to the anim-only tail. Pod has no effect.
- **MP anti-stack guards** (first switch `0x00481c11`, jump table `0x00483394` indexed by
  `type-1`): Unit→Money if `house credits(+0x2e8) > 0x32` (50); Cloak→Money if already cloaked
  (`+0x3d2`); Armor/Speed/Firepower→Money if that unit's mult already ≠ 1.0
  (`[0x56]/[0x160]/[0x58]` doubles, "≠ 1.0" = low≠0 || high≠0x3ff00000); Veteran→Money if
  already elite. Water cell (`CellClass+0xEC == 2`) forces Money unless that type's sound-flag
  byte `(&DAT_0089ecc0)[idx]` is set (the `[Powerups]` "over water?" 3rd field).
- **FreeMCV:** `0x00481bb8`–`0x00481bff` — if house has no factory (`+0x2f0==0`), credits
  `> 0x5dc` (1500), owns 0 MCVs (`CountOwnedInstances`), and rule flag `DAT_00a8b258`
  (`FreeMCV`) set → force index 1 (Unit) with a "spawn base unit / MCV" flag.

### 9.5 Effect jump table resolved — Invulnerability/IonStorm are no-ops (VERIFIED)

`read_memory 0x004833c4` (18 dwords) → dispatch targets by canonical index:

| Idx | Type | Handler addr | Idx | Type | Handler addr |
|----|------|-------------|----|------|-------------|
| 0 | Money | 0x00482463 | 9 | Armor | 0x00482d56 |
| 1 | Unit | 0x00482041 | 10 | Speed | 0x00482f36 |
| 2 | HealBase | 0x00482b8f | 11 | Firepower | 0x00483125 |
| 3 | Cloak | 0x00482840 | 12 | ICBM | 0x00482ca1 |
| 4 | Explosion | 0x00482565 | 13 | Invulnerability | **0x004832f5** (anim-only) |
| 5 | Napalm | 0x0048271e | 14 | Veteran | 0x00482972 |
| 6 | Squad | **0x004832f5** (anim-only; also remapped) | 15 | IonStorm | **0x004832f5** (anim-only) |
| 7 | Darkness | 0x00481f6d | 16 | Gas | 0x00481de7 |
| 8 | Reveal | 0x00481f9d | 17 | Tiberium | 0x00481e99 |

**Invulnerability(13), IonStorm(15), Squad(6), and Pod(18) have NO gameplay handler** — they
jump straight to the shared anim-only tail at `0x004832f5` (or are excluded). They are inert
in gamemd regardless of weight. See §9.8 TS-legacy.

**Money handler (`0x00482465`, VERIFIED):** `if (SoloCrateMoney(local_17c)==0) roll =
RandomRanged(data, data+900)` then `Add_Credits`. So MP money = `RandomRanged(2000, 2900)`
(data=2000); SP money = fixed `SoloCrateMoney` (stock 5000, no roll). Two other verified
magnitudes/counts: Tiberium spawns `RandomRanged(10,0x14)` = 10–20 patches, each at
`RandomRanged(0,0x300)` offset; Explosion does the center blast + a fixed **5** extra blasts
at `RandomRanged(0,0x200)` offsets. Armor/Speed/Firepower multiply the unit's mult double by
`data` for every owned object within `CrateRadius` leptons (`Rules+0x172c`); stock
CrateRadius=3.0 cells = 768 leptons.

### 9.6 Placement, regen cadence, and lobby gate (VERIFIED / re-confirmed)

- **Lobby gate:** `ini/rulesmd.ini` line 3034 `[MultiplayerDialogSettings] Crates=yes` is the stock default
  (ON). The runtime flag is `DAT_00a8b261` (CratesEnabled), gating both regen (§3.6) and the
  instant replace-on-pickup. `Crates` also gates `UpdateCrateRegenTimers` (vtable slot 27) per
  MAPCLASS report.
- **Placement** (`decompile_function 0x0056bd40` + `disassemble_function`): find a free slot in
  the 256-entry array (`DAT_00abd480` sentinel), then up to **1000** attempts: X and Y are
  each `RandomRanged(1, Size.width+Size.height-1)` — **2 Scen->Random rolls per attempt**.
  Water cell (`+0xEC==2`) →
  SpeedType 5 + `WaterCrateImg`; else SpeedType 1 + `WoodCrateImg`. `Find_Nearby_Passable_Cell`
  snaps to a reachable cell; `CrateSlot__PlaceOverlayAndInitTimer (0x004a17c0)` places the
  overlay and rolls **1 more** Scen->Random for the regen jitter (§8.4). First success returns.
- **Regen cadence:** timer = `CrateRegen_minutes × [450 .. 1800]` frames, uniform (verified x87,
  §8.4). Stock `CrateRegen=3` → **[1350 .. 5400] frames = 45 s .. 180 s** at 30 fps (mean
  112.5 s). Per-slot; on expiry `UpdateCrateRegenTimers (0x0056bbe0)` clears + re-places.
- **Initial count:** `Post_Map_Init` places `clamp(max(CrateMinimum, Session::HumanPlayers),
  CrateMaximum)` crates (§3.2, §8.3). Stock `CrateMinimum=1`, `CrateMaximum=255`.

### 9.7 Stock-YR outcome distribution (what a default skirmish actually shows)

Only the 8 weight>0 types can appear from a random roll. With total 110:

| Outcome | Weight | P(roll) | MP notes |
|---------|-------:|--------:|----------|
| Money | 20 | 18.18 % | + absorbs Unit/Armor/Speed/Firepower/Veteran when guard trips |
| Unit | 20 | 18.18 % | random `CrateGoodie=yes` unit; MCV if FreeMCV condition |
| Veteran | 20 | 18.18 % | promote units in radius by 1 level |
| HealBase | 10 | 9.09 % | heal all picker buildings |
| Reveal | 10 | 9.09 % | reveal map for picker |
| Armor | 10 | 9.09 % | ×1.5 armor of units in radius |
| Speed | 10 | 9.09 % | ×1.2 speed of units in radius |
| Firepower | 10 | 9.09 % | ×2.0 firepower of units in radius |

Disabled in stock (weight 0, never rolled but reachable via map-placed OverlayData):
Cloak, Explosion, Napalm, Darkness, ICBM, Gas, Tiberium. Inert (no handler): Squad,
Invulnerability, IonStorm, Pod.

### 9.8 TS-legacy / inert outcomes (flagged, verified)

- **Pod(18)** — "drop pod special" (TS Dropship). Excluded by the `idx>0x11` guard; no anim, no
  effect. Do NOT implement.
- **Squad(6)** — TS infantry-squad drop; forcibly remapped to Money and its handler slot is the
  anim-only tail. Never produces a squad. Do NOT implement as a squad.
- **Invulnerability(13), IonStorm(15)** — jump-table slots point at the anim-only tail; no
  gameplay code exists in gamemd. Even if a mod sets their weight >0, they do nothing but play
  their configured anim. Do NOT implement effects for these.
- All four are weight-0 in stock YR, so they never even roll. This is verified from the jump
  table (`0x004833c4`), not inferred.

---

## 10. IMPLEMENTATION HANDOFF (Rust / vera20k)

**Current disposition (2026-08-13):** initial scenario-start placement exists in `src/sim/crates.rs`
and now matches the bounded `Post_Map_Init` count, search, overlay-commit, and RNG contract.
Runtime slot timers, pickup effects, immediate replacement, and regeneration remain incomplete;
those are the remaining parity/determinism surface for GSI-04.23 and must stay driven by parsed
INI plus the scenario RNG.

### 10.1 INI parsing additions (`src/rules`)

1. **`[CrateRules]` → new `CrateRules` struct** on `RuleSet` (parse in `RuleSet::from_ini`,
   next to `general`). Fields (stock values): `crate_maximum: u32 = 255`,
   `crate_minimum: u32 = 1`, `crate_radius_leptons: i32 = 768` (parse cells `3.0` × 256),
   `crate_regen_minutes: Fixed = 3`, `free_mcv: bool = true`, `solo_crate_money: i32 = 5000`,
   `unit_crate_type: Option<TypeId>` (`none`→None), `silver/wood/water_crate: CrateType`
   (`HealBase`/`Money`/`Money` — the SP fixed-type overrides), `wood/normal/water_crate_img:
   overlay name` (`CRATE`/`CRATE`/`WCRATE`), `heal_crate_sound`.
2. **`[Powerups]` → new `Powerups` table** (19 fixed entries in the canonical §9.1 order — key
   the parser off an internal name array exactly like gamemd, do NOT trust file order). Each
   entry: `weight: u32`, `anim: Option<AnimTypeId>` (`<none>`→None), `over_water: bool`
   (yes/no), `data: Fixed` (4th field; `%` suffix → ×0.01). Store as
   `[PowerupEntry; 19]` indexed by a `CrateType` enum (0..18).
3. **`[General] Crates`** — already the lobby default source; confirm the skirmish option maps
   to a runtime `crates_enabled: bool` in the session/game-options (it already exists as a
   hashed toggle; wire it to gate spawning).
4. **Overlay flags** — the overlay-type registry must carry `crate: bool` (`Crate=`) and
   `crate_trigger: bool` (`CrateTrigger=`) from `[CRATE]`/`[WCRATE]` (`Crate=yes`,
   `CrateTrigger=yes`, `RadarInvisible=yes`, `Land=Clear`/`Water`). These two overlays are the
   only crate overlays in stock.

### 10.2 Data structures (`src/sim/crates.rs`, new)

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CrateType { Money=0, Unit, HealBase, Cloak, Explosion, Napalm, Squad,
    Darkness, Reveal, Armor, Speed, Firepower, Icbm, Invulnerability, Veteran,
    IonStorm, Gas, Tiberium, Pod } // canonical order, drives weight/anim/dispatch

pub struct CrateSlot {           // mirrors the 256-entry native array
    cell: Option<(u16, u16)>,    // None = free (native sentinel)
    regen_timer: u32,            // frames remaining; started at place time
    start_tick: u64,
}
pub struct CrateState {
    slots: Vec<CrateSlot>,       // cap 255 (native 256; enforce CrateMaximum)
    // NOTE per project scale target (20k units/30 players): size to
    // max(CrateMaximum, players) rather than a fixed 256 bitmask analog.
}
```

Crate overlays live in the existing `overlay_grid` (`src/sim/overlay_grid.rs`) as the `CRATE`/
`WCRATE` overlay indices with an `OverlayData` byte (predetermined type when `< 0x13`). The
`CrateState` slot array parallels the overlay for the regen timers, exactly as gamemd keeps a
separate timer array.

### 10.3 Tick-phase placement (`World::advance_tick`)

gamemd runs crate regen in `Main_Tick` **after all object AI, before Factory/House AI**, and
uses `Scen->Random`. In the Rust order that is **Phase 7 (Scatter + Production + Repairs +
Docks + Ore)** — place the regen sweep adjacent to ore-growth, which already uses
`self.scenario_rng` (see `world/mod.rs` ~L2609–2657 smudge/ore dispatch). Concretely:

- **Pickup** is event-driven, not a tick phase: hook the movement/locomotor "entered cell"
  commit (`sim/movement`) — when a unit finishes entering a cell whose overlay has
  `crate=true`, run the pickup dispatch (the `CellClass::Can_Enter_Cell_General` port). Fire it
  for ground drive, ship, infantry sub-cell arrival, AND chrono/teleport arrival (all five
  locomotors call it in gamemd, §5). Skip if the picker's house is civilian.
- **Regen sweep** once per tick in Phase 7: for each occupied slot whose timer expired, clear
  the old overlay + place a new crate (`place_crate`). Gate on `crates_enabled && multiplayer`.
- **Initial placement** at match bootstrap (`match_bootstrap.rs` / scenario session): place
  `clamp(max(CrateMinimum, human_player_count), CrateMaximum)` crates.
- **Ordering discipline:** the pickup's own `place_crate` (instant MP replacement) and its
  effect rolls happen INSIDE the pickup, at the tick position where the unit's move commits —
  keep that before the Phase-7 regen sweep so the `Scen->Random` cursor order matches gamemd.

### 10.4 RNG (determinism)

Use **`self.scenario_rng`** (the synced scenario stream) for ALL crate rolls, matching
`Scen->Random`. Exact roll sequence to reproduce (order matters for the hash):

- **place_crate:** per attempt → `rand_ranged(1, Size.width+Size.height-1)` for X and Y; on the
  first cell that passes `Find_Nearby_Passable_Cell` + is empty, one more `rand_ranged(0, i32::MAX-1)`
  for the regen-jitter timer = `regen_minutes × (450 + rand01×1350)` frames. Up to 1000 attempts.
- **pickup:** `roll = rand_ranged(1, sum_of_19_weights)` then cumulative walk. Then
  effect-specific rolls (Money: 1; Tiberium: `rand_ranged(10,20)` + N×`rand_ranged(0,768)`;
  Explosion: 5×`rand_ranged(0,512)`; Unit free-MCV branch: `rand_ranged(0, unit_type_count-1)`
  possibly looped until a `CrateGoodie=yes` type). Do NOT skip rolls on the guard-downgrade
  path — the roll already happened before the guard.

Match `rand_ranged` semantics to `RandomClass::RandomRanged (0x0065c7e0)` — inclusive both
ends — which the sim RNG already implements for other scenario-stream consumers.

### 10.5 Effect handlers (stock-active first)

Implement the 8 weight>0 outcomes first (Money, Unit, Veteran, HealBase, Reveal, Armor, Speed,
Firepower) — these are all a default skirmish can show. Then the weight-0-but-map-placeable set
(Cloak, Explosion, Napalm, Darkness, ICBM, Gas, Tiberium) for map/trigger crates. Skip Squad,
Invulnerability, IonStorm, Pod (§9.8). Effect details: §9.5 + §3.5. Radius effects iterate all
objects within `crate_radius_leptons` (3-D distance, `ftol` of `sqrt`) owned by the picker's
house; multiply the per-object mult double by `data`; play EVA + sound once if any applied.

### 10.6 Overlay/cell + death/destruction integration

- `CarriesCrate=yes` units (`UnitType+0xE1A`) drop a crate at a nearby passable cell on death
  (§8.2) — hook `UnitClass::ReceiveDamage` port in `sim/combat/damage/receive.rs`.
- `CrateBeneath` / `CrateBeneathIsMoney` buildings (`BuildingType+0x1767/+0x1769`) drop a crate
  on destruction (§8.1: OverlayData 0 = Money, 0x14 = random) — hook building destruction /
  occupy-map clear.
- `CrateTrigger=yes` overlay fires trigger action `0x31` on pickup (§3.4 step 3); defer until
  map-trigger runtime supports it.

### 10.7 Acceptance tests (name the check — see CLAUDE.md certification rule)

1. **`test_powerups_parse_canonical_order`** — parse stock `[Powerups]`; assert index→weight ==
   `[20,20,10,0,0,0,0,0,10,10,10,10,0,0,20,0,0,0,0]`, sum 110; magnitudes Money=2000,
   Armor=1.5, Speed=1.2, Firepower=2.0, Explosion=500, Napalm=600, Gas=100, Veteran=1.
2. **`test_crate_rules_parse`** — CrateMaximum=255, CrateMinimum=1, CrateRadius=768 leptons,
   CrateRegen=3, FreeMCV=true, SoloCrateMoney=5000, images CRATE/CRATE/WCRATE.
3. **`test_crate_selection_weighted`** — with a seeded scenario RNG matching the native stream,
   assert the cumulative-walk pick given a fixed roll lands on the expected index for boundary
   rolls (1, 20, 21, 110).
4. **`test_crate_regen_timer_jitter`** — timer ∈ `[regen×450, regen×1800]` and reproduces the
   native `upper + rand01×(lower−upper)` value for a fixed RNG draw (golden from emulation).
5. **`test_money_crate_amount`** — MP money ∈ `[2000, 2900]` for data=2000; SP money == 5000
   (no roll).
6. **Determinism golden** — a scripted skirmish that spawns + picks up N crates must reproduce
   a `Scen->Random`-derived state hash from a gamemd capture (not Rust-vs-Rust). Until such a
   capture exists, mark **UNVERIFIED-pending-instrument** — the roll *count/order* in §10.4 is
   the contract to hold constant.

### 10.8 Effort / risk

Medium. The selection, regen, and 8 active handlers are small and fully specified above. The
determinism risk is entirely in the RNG roll count/order (§10.4) — get that wrong and the
match-hash diverges even if every visible effect looks right. Radius-effect object iteration
and the death/destruction drop hooks are the fiddly parts; the four inert outcomes save work.

---

## 11. Verification Ledger (§9–§10 claims)

| Claim | Ghidra call | Result |
|-------|-------------|--------|
| 19 powerups, INI-driven, name-array order | `decompile_function 0x00673e80` | VERIFIED |
| Canonical index → name mapping | `read_memory 0x007e523c` + string reads | VERIFIED |
| Static weight defaults sum 144 | `read_memory 0x0081da8c` | VERIFIED |
| Stock INI weights sum 110 (not 100) | `ini/rulesmd.ini:30345` + name map | VERIFIED — corrects §8.5 |
| Selection: sum→RandomRanged(1,total)→walk | `disassemble_function 0x00481a00` @0x481ad3 | VERIFIED |
| RNG = Scen->Random (`[0xa8b230]+0x218`) | disasm @0x481af2, @0x0056bd90 | VERIFIED |
| Squad(6)→Money remap | disasm @0x00481db8 | VERIFIED |
| Pod/idx>0x11 → anim-only | disasm @0x00481dca | VERIFIED |
| Jump table targets (Invuln/IonStorm no-op) | `read_memory 0x004833c4` | VERIFIED |
| Money = RandomRanged(data,data+900), SP fixed | disasm @0x00482465 | VERIFIED |
| Placement 2 rolls/attempt, ≤1000, water branch | `decompile_function 0x0056bd40` | VERIFIED |
| Pickup clear pauses + empties the slot; empty slots are skipped, so there is no delayed second replacement | `batch_decompile 0x004a1750,0x0056c020,0x0056bbe0` + `decompile_function 0x0056bd40` (2026-07-23) | VERIFIED — corrects §3.6 |
| `[General] Crates=yes` default | `ini/rulesmd.ini:3034` | VERIFIED |
| Effect magnitudes INI-driven (static all-zero) | `read_memory 0x0089ec28` | VERIFIED |
