# Mission_Enter Refinery Dock — Verification & Corrections

**Date:** 2026-04-19
**Companion to:** `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` (1012 lines, 2026-04-03)
**Companion to:** `MINER_DOCK_GAPS_RESEARCH.md` Gap 3 (FUN_00500200)
**Confidence:** HIGH (re-decompiled `UnitClass::Mission_Enter` at 0x739EC0 and `FUN_00500200` at 0x500200)
**Active in YR:** YES — corrections apply to live code paths

---

## Why this doc exists

The existing `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` is comprehensive and largely
correct (radio protocol, queue cell calc, locomotor piggyback, end-of-mission cleanup).
Re-investigating the topic, I verified:

- ✅ Radio command IDs and return values (§6 of original) — **correct**
- ✅ Queue cell hardcoded `(X+3, Y+1)` from building top-left in radio 0xE — **correct**
- ✅ `unaff_retaddr` is the state parameter; values 0 and 2 — **correct**
- ✅ The CLSID_WalkLocomotion piggyback check at the dock-cell arrival branch — **correct**
- ✅ Setting `DockLink` (`+0x254` field, the FootClass `+0x84` slot) when at dock — **correct**
- ✅ End-of-mission ore overlay destruction on dock cell — **correct**

But two specific sections need correction. Both involve the same code block:
**the `if (radio 8 returned QUEUED)` branch in Mission_Enter** (~offset 0x73AA00).

---

## Correction 1: §5.3 of original report has the harvester / non-harvester branches **inverted**

### What the original says (paraphrased)

> **§5.3 Radio 8 Response Handling — `if (result == 0x17) // QUEUED`:**
> - **`if (!isHarvester && !isWeeder)`** → just wait or fall back to dock link
> - **`else // IS harvester/weeder`** → AI harvester calls `FUN_00500200` to wander to a new cell, then `Set_Mission(0xB)`
> - The pseudocode then describes the wander logic *under the harvester branch*.

### What the binary actually does

Re-decompilation of `UnitClass::Mission_Enter` (0x739EC0) at the QUEUED-result branch shows the
**conditions are correct but the bodies are swapped**:

```c
// At 0x739EC0, after `if (Transmit_Radio(8) == 0x17)`:

if ((TypeClass.Harvester == 0)               // TechnoTypeClass+0xE0E
    && (TypeClass.Weeder == 0)) {            // TechnoTypeClass+0xE0F
    // *** NON-HARVESTER, NON-WEEDER branch ***
    if (this.PlayerOrdered == 0) {           // unit+0x2D8
        if (!HouseClass::IsPlayerControl()
            && destBuilding != null
            && destBuilding.TypeClass.WeaponsFactory != 0) {  // BuildingType+0x16BD
            // AI-controlled NON-harvester trying to enter a WeaponsFactory queue:
            cell = FUN_00500200(this);
            if (cell.valid) {
                Set_Mission(MISSION_MOVE = 2, 0);
                Set_Destination(cell, 1);
                QueueMission();
                SetGhostCell(cell);
                Set_Mission(0xB, 0);          // sub-state for "queued at building"
            }
        } else {
            // Player-controlled OR not a WeaponsFactory: hold or fallback
            if (this.field_0x218 == 0 || this.field_0x218 == this.DockLink) {
                FootClass::Stop_Moving();
                Move_To(InvalidCell, 1, 0);
            } else {
                Set_Destination(this.field_0x218, 1);
            }
        }
    } else {
        SlaveManagerClass::RecallAllSlaves();
    }
} else {
    // *** HARVESTER OR WEEDER branch ***
    Set_Mission(MISSION_HARVEST = 10, 1);    // queue Mission_Harvest immediately
    // No wander, no special handling — the harvester will re-enter Mission_Harvest
    // next tick, which re-runs Find_Docking_Bay and may queue at the same/different
    // refinery.
}
```

### Why this matters

- **Harvesters do NOT call `FUN_00500200`** when queued at a refinery. They simply transition
  to Mission_Harvest (mission 10), which re-runs the State 2 dock-search next tick.
- **`FUN_00500200` is called by AI-controlled NON-harvester units** (most often regular ground
  units like Rhino/Grizzly waiting outside a War Factory) when their target building's queue
  is full. The wander gives them somewhere to go instead of clogging the entrance.
- The original doc (and `MINER_DOCK_GAPS_RESEARCH.md` Gap 3) characterized the wander as
  "AI harvester wander point generator" — that's misleading. It IS a wander generator, but
  it's used for **non-harvester AI units approaching War Factories**, not for harvesters.

The function ITSELF *is* harvester-aware in its biasing logic — it reads ore storage via
vtable slots `+0x2D4/+0x2D8/+0x2DC` and uses storage > 0 vs == 0 to choose between
"directional bias" (cases 1–4) and "any direction" (case 0). For a non-harvester caller,
storage is always 0, so it always picks case 0. That's why the bias logic *looks*
harvester-specific even though the call site isn't a harvester path in YR.

### Practical implications for the Rust port

If the Rust harvester FSM was ported from the original report's §5.3 verbatim:

- ❌ It may have implemented "AI harvester wanders when refinery is busy" — that's wrong.
- ✅ The correct behavior is "harvester re-enters Mission_Harvest (which re-runs dock search)
  on the same tick the QUEUED radio came back."

Recommend a quick check of `src/sim/miner/miner_system.rs` `handle_return()` and the
`miner_dock` reservation logic for any "wander when busy" behavior that doesn't match this.

---

## Correction 2: `FUN_004D85D0` is **`FootClass__PerCellProcess`**, not "Dock State Transition"

### What the original says (§7.2)

> "FUN_004d85d0 — Dock State Transition (0x4D85D0). param_2 = 2: 'Entering dock' mode"

The original doc describes it as the dock-specific transition that powers off locomotion.

### What it actually is

The function at `0x4D85D0` is now labeled **`FootClass__PerCellProcess`** in Ghidra. Its
callers are:

```
InfantryClass__Mission_Enter (0x5196A0)
UnitClass__Mission_Enter     (0x739EC0)
```

Both Mission_Enter handlers call it. The behavior is more general than "dock transition":

- **`param_2 == 2` (the "I just stepped onto a critical cell" branch):**
  - Clears `+0x6B2` and `+0x6B0` (per-cell pathfinding flags)
  - If turret count > 0: applies stored facing via vtable+0x4EC and vtable+0x4E8
  - Adjusts ghost-cell occupancy: removes from old ghost cell, adds to new (`FUN_0070F6A0` /
    `FUN_0070F670`) — including 8-direction adjacent-cell crowd counters at `CellClass+0x122`
  - If `param_1[0x88] == 2` (some scan trigger): scans 8 adjacent cells for crushable threats
    and calls `vtable+0xFC` if found (likely Scatter / cancel mission)
  - Computes distance to dock_link and clears nav target if mission is in {0x15, 0xB, 1, 0xF}
  - Calls `TechnoClass::ProcessCellAction` with action codes 1, 0x3B, 0x35, 0x36, 0x19, 0x1A,
    0x18 — these are the cell-action effects (gem pickup, ore pickup, parachute deploy, etc.)
  - **At the very end** of the param_2==2 branch: looks up building at current cell, if it
    has flag at +0x16BF set AND building+0x618 < 8 AND unit RTTI in {1, 2, 0xF}: applies
    `RulesClass+0xFA8` damage to unit. (This is the "self-destruct on impassable cell"
    handler.)

- **`param_2 == 0` and `param_2 == 1`** (other states): handle different per-cell-arrival
  scenarios. Not investigated here.

### Why this matters

- The function is NOT specific to dock transitions. Calling it "dock state transition" is
  misleading — it's the generic "I just entered a new cell during Mission_Enter" handler.
- The dock-specific behavior (powering off locomotion via `vtable+0x5C` on the locomotor)
  is performed by **the caller in Mission_Enter, NOT inside `FootClass__PerCellProcess`**.
- A second call to `FootClass__PerCellProcess(unknown_state)` happens at the end of
  `UnitClass::Mission_Enter` regardless of dock outcome — its state argument is read from
  stack memory and varies by code path.

### Practical implications

If the Rust port treats this as a dock-only function, it will miss the per-cell ghost-cell
reslotting and adjacent-cell crowd counter updates. These affect:

- AI threat avoidance (units check adjacent cell crowd counters when picking move targets)
- Multi-unit cell occupancy (so two crushable units don't stack into the same defended cell)

Search Rust for any equivalent of "ghost cell rebroadcast on Mission_Enter cell entry."

---

## Spot-checked claims that are correct

To be clear, these claims from the original report ARE verified correct against the binary:

| Claim | Source line in original | Verified |
|-------|-------------------------|----------|
| Mission_Enter handles 4 distinct paths: Grinder / UnitAbsorb / Refinery / Generic Enter | §3.1 | ✅ |
| Grinder branch (RTTI==9) destroys passengers recursively then UnInits self | §3.2 | ✅ |
| Refinery dock-cell arrival queries IPiggyback for CLSID_WalkLocomotion match | §4.1 | ✅ |
| Queue cell calculation `(X+3, Y+1)` is hardcoded in BuildingClass radio 0xE handler | §6.2 | ✅ (radio 0xE handler at 0x43C2D0 case 0xE) |
| WeaponsFactory always returns 0x17 (QUEUED) for radio 8 | §6.6 | ✅ |
| Radio 0x15 to refinery sets the *unit's* mission to 0x10 (Unload), not the building's | §6.4 | ✅ |
| Locomotor `Power_Off` is called via vtable slot +0x5C | §4.1 | ✅ |
| Mission_Enter mid-branch creates self-destruct explosion if unit can't enter cell, not on bridge, not sinking | §10.2 | ✅ |
| Ore overlay destruction on dock cell at end of Mission_Enter | §10.3 | ✅ |

---

## What this verification did NOT cover

I did not re-verify in Ghidra, but the original is HIGH confidence on these so I'm not
flagging them:

- Multi-harvester queue handling (§8) — would require decompiling Building radio 0xE in full
- Locomotor piggyback swap-back timing (§9) — covered separately in MINER_DOCK_GAPS_RESEARCH.md
- All the specific BuildingClass radio handlers (radio 0xF, 0x15, 0xE branches for Hospital
  / Helipad / Bunker)

These remain trusted as documented.

---

## Open Questions — RESOLVED 2026-04-19

### Q1: `field_0x218` — RESOLVED ✓

`UnitClass+0x218` is the **WarpTarget** (ghost cell) field, already documented in
[WAR_MINER_REFERENCE.md](WAR_MINER_REFERENCE.md) §10. UnitClass inherits from
TechnoClass, so `TechnoClass+0x218` accessed in Mission_Enter is the same byte —
the docs label it "WarpTarget" from the harvester angle.

In §5.3 the field is used as a fallback "remembered cell to head toward" — when a
harvester is queued at a refinery and has a WarpTarget set (e.g., the half-harvested
ore cell it left to dock), it heads back toward that. If WarpTarget is null, it just
holds position.

### Q2: Mission ID `0xB` = "Area Guard" — RESOLVED ✓

Verified by reading the `g_MissionNameTable` string pointer table at `0x00816CAC`.
Each entry is a 4-byte string pointer; entry 11 (index `0xB`) points to `0x00816E10`
which contains the literal `"Area Guard"`.

For completeness, the **full mission ID → name table** decoded from `g_MissionNameTable`:

| ID | Name | Handler (FootClass / UnitClass) |
|----|------|---------------------------------|
| 0  | Sleep | (default state) |
| 1  | Attack | `FootClass::Mission_Attack` (0x4D4DC0) |
| 2  | Move | `FootClass::Mission_Move` (0x4D4200) |
| 3  | QMove | (queued move) |
| 4  | Retreat | `FootClass::Mission_Retreat` (0x4DA2C0) |
| 5  | Guard | `FootClass::Mission_Guard` (0x4D5070) / `UnitClass::Mission_Guard` (0x740A90) |
| 6  | Sticky | (placeholder) |
| 7  | **Enter** | `FootClass::Mission_Enter` (0x4D9290) / `UnitClass::Mission_Enter` (0x739EC0) |
| 8  | Capture | `FootClass::Mission_Capture` (0x4D4B20) / `InfantryClass::Mission_Capture` (0x5202F0) |
| 9  | Eaten | `FootClass::Mission_Eaten` (0x4D4CB0) |
| 10 (0xA) | Harvest | `UnitClass::Mission_Harvest` (0x73E5E0) |
| **11 (0xB)** | **Area Guard** | `FootClass::Mission_AreaGuard` (0x4D6AA0) |
| 12 (0xC) | Return | (no separate handler decompiled) |
| 13 (0xD) | Stop | |
| 14 (0xE) | Ambush | |
| 15 (0xF) | Hunt | `FootClass::Mission_Hunt` (0x4D5350) / `UnitClass::Mission_Hunt` (0x740B60) |
| 16 (0x10) | **Unload** | `UnitClass::Mission_Unload` (0x740EF0) |
| 17 (0x11) | Sabotage | |
| 18 (0x12) | Construction | (used by buildings under construction) |
| 19 (0x13) | Selling | |
| 20 (0x14) | Repair | `UnitClass::Mission_Repair_Thunk` (0x7447A0) |
| 21 (0x15) | Rescue | `FootClass::Mission_Rescue` (0x4DDF90) |
| 22 (0x16) | Missile | `BuildingClass::Mission_Missile` (0x44C980) |
| 23 (0x17) | Harmless | |
| 24 (0x18) | Open | `AircraftClass::Mission_Open` (0x4158E0) |
| 25 (0x19) | Patrol | `FootClass::Mission_Patrol` (0x4D4280) |
| 26 (0x1A) | Paradrop Approach | `AircraftClass::Mission_ParaDropApproach` (0x4155F0) |
| 27 (0x1B) | Paradrop Overfly | `AircraftClass::Mission_ParaDropOverfly` (0x4157C0) |
| 28 (0x1C) | Wait | |
| 29 (0x1D) | Attack Move | |
| 30 (0x1E) | Spyplane Approach | `AircraftClass::Mission_SpyPlane` (0x417300) |
| 31 (0x1F) | Spyplane Overfly | |

So when Mission_Enter's wander branch issues `Set_Mission(0xB, 0)` after
`FUN_00500200`, the unit transitions to **Area Guard** at the wander point. Area Guard
(`FootClass::Mission_AreaGuard` at 0x4D6AA0) is a "patrol around a position" handler
that scans for nearby threats and self-cancels back to Mission_Harvest if the unit is a
harvester (verified: it explicitly checks `Harvester=yes` at +0xE0E and falls back to
mission 10).

### Q3: `field_0x2D8` is **NOT PlayerOrdered** — it's the **SlaveManager pointer** — RESOLVED ✓

The original report (and most existing TechnoClass field tables) labels this offset
"PlayerOrdered". That label is **incorrect** — at least for the Mission_Enter and
Mission_AreaGuard usage.

Evidence:

1. **`SlaveManagerClass::RecallAllSlaves` (0x6B0CC0)** signature:
   ```c
   void __fastcall SlaveManagerClass__RecallAllSlaves(int param_1) {
       if (*(int *)(param_1 + 0x5C) == 0) {
           *(undefined4 *)(param_1 + 0x5C) = 1;
           *(undefined4 *)(param_1 + 0x60) = 0x7FFFFFFF;
           int count = *(int *)(param_1 + 0x48);
           while (--count >= 0) {
               int *slave = *(int **)(*(int *)(param_1 + 0x3C) + count * 4);
               int *unit = (int *)*slave;
               if (slave[1] != 6 && unit != NULL) {
                   (**(code **)(*unit + 0x3D0))(); // Slave_Recall vtable slot
               }
           }
       }
   }
   ```
   `param_1` here is a **`SlaveManagerClass*`** — fields at +0x5C, +0x60, +0x48, +0x3C
   are SlaveManager internals (recall flag, timer, slave count, slave array).

2. **In `FootClass::Mission_AreaGuard`** (0x4D6AA0):
   ```c
   if (param_1[0xB6] != 0) {           // param_1[0xB6] = byte 0x2D8
       SlaveManagerClass__RecallAllSlaves();
   }
   ```
   The pointer-null check followed by an unconditional call (with implicit ECX = the
   field value via `__fastcall`) confirms that `field_0x2D8` holds a `SlaveManagerClass*`,
   not a boolean flag.

3. **In `UnitClass::Mission_Enter`** (0x739EC0), the QUEUED branch:
   ```c
   if (*(int *)&param_1->field_0x2D8 == 0) {
       // wander branch (only AI non-harvester non-weeder units reach here)
   } else {
       SlaveManagerClass__RecallAllSlaves();   // unit has slaves → recall
   }
   ```
   Same pattern: null-check then call.

**Practical interpretation:** `TechnoClass+0x2D8` = `SlaveManager*`. Set to non-NULL on
**Slave Master units** — which in YR includes:
- Yuri Prime (mind-control with slave list)
- *(potentially)* mod-added Slave Master units

The deployed-form Slave Miner is a **building**, not a unit, and its SlaveManager lives
on the BuildingClass instance (not relevant here). The undeployed SMIN unit *is*
Harvester=yes, so it never enters the wander branch.

So the `Set_Mission(...)` else-branch in Mission_Enter handles the case where a Slave
Master (e.g., Yuri Prime) is told to enter a building — it recalls its slaves before
queuing.

**Initial concern:** I worried this mislabel might be widespread across the 145+ docs.
**Verification result (2026-04-19):** The mislabel was **isolated to one line** in
`MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` §12. The canonical struct layout doc
(`TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` line 185) has always correctly labeled
`+0x2D8` as `SlaveManagerClass*` (HIGH confidence). Multiple other system reports
(SLAVE_MINER_ORE_SYSTEM, FOOTCLASS_NON_MOVEMENT_FIELDS, TECHNOCLASS_SYSTEMS) reference
the SlaveManager correctly. **Rust source has zero references** to either `0x2D8` or
`PlayerOrdered`, so the Rust port is unaffected.

The single mislabel has been fixed in the original report.

### Q4: `BuildingType+0x16BF` = **LaserFence** flag — RESOLVED ✓

Verified by examining `BuildingTypeClass_ReadINI_Water` (0x45FE50) disassembly:

| Offset | INI key string addr | INI key name |
|--------|---------------------|--------------|
| +0x16BB | `0x81AA5C` | `Refinery` |
| +0x16BC | (between, not investigated) | (likely `Weeder`) |
| +0x16BD | `0x81AA4C` | `WeaponsFactory` |
| +0x16BE | `0x81AA3C` | `LaserFencePost` |
| **+0x16BF** | **`0x81AA30`** | **`LaserFence`** |
| +0x16C0 | `0x81AA20` | `FirestormWall` |
| +0x16C1 | `0x81AA10` | (likely `Hospital`) |

The `LaserFence` and `FirestormWall` entries — plus `LaserFencePost` — confirm these are
**Tiberian Sun: Firestorm** legacy structure flags. They persist in YR but only matter
when those building types are present in a match.

The usage in `FootClass::PerCellProcess` (0x4D85D0) at param_2==2:
```c
building = Look_up_building_in_cell(unit_cell);
if (building != NULL
    && building->TypeClass->LaserFence            // BuildingType+0x16BF
    && building->state_618 < 8                    // BuildingClass+0x618 (likely active duration)
    && unit.RTTI in {1 (Unit), 2 (Aircraft), 0xF (Infantry)}
    && unit.Health > 0) {
    unit.ReceiveDamage(unit.Health,
                       0,
                       Rules.LaserFenceWarhead,    // RulesClass+0xFA8
                       building, 1, 1, 0);
}
```

**Effect:** When a LaserFence building (or Firestorm wall) is "active" (state_618 < 8)
and a unit walks onto its cell, the unit takes lethal damage from `RulesClass+0xFA8`
warhead. This is the laser-fence-burns-you behavior.

**Active in YR: CONDITIONAL.** LaserFence (`LASR`, `GAGAP`) building types exist in
`rulesmd.ini` and are buildable in some campaign missions, but **standard YR skirmish
maps do not feature them**. Mods (Mental Omega, etc.) re-enable them. The code path
itself is fully live in the binary — it triggers any time a unit steps on a cell
containing a LaserFence-flagged building.

`BuildingClass+0x618` — best guess: a "fence segment activation timer" or "laser segment
state count" — the < 8 check suggests a count of segments-still-active or a frame
countdown. Not investigated further (off-topic for harvester docking).

---

## Summary of corrections — STATUS

| Doc / claim | Status |
|-------------|--------|
| `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` §5.3 (inverted branches) | ✅ **FIXED 2026-04-19** — pseudocode rewritten to match binary, and a "CORRECTED" note added pointing here |
| `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` §12 (`+0x2D8 = PlayerOrdered`) | ✅ **FIXED 2026-04-19** — relabeled `SlaveManager*` with cross-ref to canonical doc |
| `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` §7.2 (FUN_004D85D0 = "Dock State Transition") | ⚠️ **Open** — Ghidra has the function correctly labeled `FootClass__PerCellProcess`. The doc could be updated for clarity but the misnomer is contained and not spreading. |
| `MINER_DOCK_GAPS_RESEARCH.md` Gap 3 | ⚠️ **Open** — characterizes FUN_00500200 as "AI harvester wander point generator." Should clarify the actual call site is for **non-harvester AI units** at WeaponsFactory queues. Function itself has 4 callers (Mission_Enter, BuildingClass exit, Find_Path, Mission_Rescue) — too generic to call "harvester wander" at all. |
| BuildingTypeClass field reference tables | ⚠️ **Open** — recommend adding `+0x16BE = LaserFencePost`, `+0x16BF = LaserFence`, `+0x16C0 = FirestormWall` (TS Firestorm legacy, conditional in YR — only active when those building types exist on the map) to the canonical building-type field tables. |
| Rust source (`src/sim/miner/`) | ✅ **No bug present** — verified that Rust dock-busy handler correctly polls in place via `phase_wait_for_dock` (`miner_dock_sequence.rs:285-296`); no random-cell / wander logic anywhere in the miner module. |
| `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` (canonical) | ✅ **Already correct** — line 185: `0x2D8 SlaveManagerClass* HIGH`. No fix needed. |

---

## Recommended doc cleanup

For future investigators, recommend updating the original
`MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`:

1. Swap the `if (!isHarvester...)` and `else` body content in §5.3.
2. Rename §7.2 from "Dock State Transition" to "FootClass::PerCellProcess (general per-cell
   handler)" and note the function is shared with InfantryClass::Mission_Enter.
3. Cross-reference this verification doc.

The original `MINER_DOCK_GAPS_RESEARCH.md` Gap 3 should similarly be updated to clarify
that FUN_00500200 is invoked for non-harvester AI units (e.g., approaching War Factory),
not for AI harvesters.

---

## Sources

- Re-decompiled `UnitClass::Mission_Enter` (0x739EC0) — full body, ~620 decompiled lines
- Re-decompiled `FootClass::PerCellProcess` (0x4D85D0) — full body, ~250 lines
- Re-decompiled `FUN_00500200` — full body
- Verified xrefs of `FootClass::PerCellProcess` callers
- Cross-checked `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` (2026-04-03)
- Cross-checked `MINER_DOCK_GAPS_RESEARCH.md` Gap 3 (2026-04-03)
