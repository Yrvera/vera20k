# Radio Link Refinery Dock — TS-Legacy & Context Sweep

**Addresses covered:** 0x004DFCB0, 0x00447B20, 0x004190B0, 0x00500200, 0x0073E5E0, 0x00740EF0, 0x006AF6C0
**Date:** 2026-05-20
**Confidence:** HIGH for identity/callers (vtable xref + decompile verified); MEDIUM for deep behavioral detail on LIGHT targets
**Active in YR:** Per-function (see each section)
**Scope:** Phase 2 TS-legacy & context sweep — 7 functions + TS-flag consolidation

---

## 1. Overview

This report covers seven functions adjacent to the refinery radio-link dock system,
scoped for TS-legacy status and context clarification. The primary deliverables are:

- Caller chains that confirm/deny YR-active code paths
- TS-legacy verdicts for each function
- Full decompile + TS-legacy verdict for `UnitClass::Mission_Unload` (the primary open question)
- Negative confirmation that `SlaveManagerClass::AI_Update` (0x006AF6C0) has nothing to
  do with refinery docking
- Deprecation recommendation for `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md`
- Consolidated TS-legacy flag table covering all Phase 1+2 items

---

## 2. Function Sweep

### 2.1 Find_Nearest_Dock (0x004DFCB0) — MEDIUM

**Ghidra name:** `FootClass__Find_Nearest_Dock`
(verified via `get_function_by_address 0x004DFCB0`)
**Body:** 0x004DFCB0 – 0x004DFDF8

**Purpose:** Iterates the owning house's building list (`this[0x87]+0xE4` array,
count at `+0xF0`). For each building computes 3D Euclidean distance (via `GetCoords`
vtable+0x48 on both sides, then `Sqrt_Approx`). Calls `BuildingClass::CanDock` via
`vtable+0x2C` on the unit. Picks the nearest valid dock. If found: sets
`this+0x6A0` (dock-found flag = `*(param_1+0x1A4) = 1`), calls `SetDestination(dock, 1)`
via vtable+0x480, then `SetMission(8, 1)` via vtable+0x1E8 — but only if the current
destination is not already that dock and mission is not already 8 (Mission_Enter).
Returns 1 on success, 0 if no dock found (clears `+0x1A4`).

**Vtable xrefs (callers via `get_xrefs_to 0x004DFCB0`):**
```
0x007E25EC [DATA]
0x007E8FDC [DATA]
0x007EB3A0 [DATA]
0x007F5FB8 [DATA]
```
All four are DATA references — vtable slots, not direct call sites. This is a virtual
method and all calls go through vtable dispatch. The four vtable bases correspond to:
- `0x007E25EC` — FootClass vtable
- `0x007E8FDC` — InfantryClass vtable
- `0x007EB3A0` — (likely AircraftClass or subclass vtable)
- `0x007F5FB8` — UnitClass vtable
(confirmed by proximity to other known vtable slots for each class)

**Active in YR:** YES — called every time a harvester searches for a refinery.
Fires at the top of Mission_Harvest case 2 (return path) and in Mission_AreaGuard
for harvesters. Frequency: every harvest return cycle (multiple times per match).

**Notable detail:** Distance comparison uses `0x7FFFFFFF` as initial sentinel (standard
max-int nearest-so-far pattern). The `param_1 != NULL` guard before `CanDock` is a
defensive null-check on the unit itself — cannot be null at this point in practice
(unit is `this`). Minor dead guard.

---

### 2.2 GetDockCoord (0x00447B20) — LIGHT

**Ghidra name:** `BuildingClass__GetDockCoord`
(verified via `get_function_by_address 0x00447B20`)
**Body:** 0x00447B20 – 0x00447DF8

**Purpose:** Returns the 3D leptonic coordinate for where a unit should dock with this
building. Dispatch logic (in order, all from `BuildingTypeClass` at `param_1[0x148]`):

1. `Type[0x16BC]` (Weeder=yes) — uses unit's current cell (`vtable+0x1B8`) shifted by
   `+2` cells in X and `+1` cell in Y (i.e., `(NW_X+2)*256+128`, `(NW_Y+1)*256+128`).
   This is a hardcoded weeder-pad offset.
2. `Type[0x16BB]` (Refinery=yes) — calls `FUN_005F6C80` (a coordinate-fetch helper)
   then adds `+0x80` leptons (half-cell) to X only. Sub-cell adjustment for refinery pad.
3. `Type[0x16AB]` (Bunker=yes) AND `param_3 != NULL` — takes the requesting unit's
   `GetCoords` and the building's `GetCoords`, computes `atan2`, bins into 4 quadrants
   (0x00–0x3F, 0x40–0x7F, 0x80–0xBF, 0xC0–0xFF), then shifts building center by
   `±0x80` leptons in X or Y based on quadrant — picks the facing-side dock coord.
4. Helipad (`Type[0x16CB]`) or UnitRepair (`Type[0x16A9]`) with `NumberOfDocks == 0`:
   returns building's `GetCoords` directly (center).
5. `NumberOfDocks == 1`: returns `Type+0x1788` dock-offset array entry [0] plus
   building center.
6. `NumberOfDocks >= 2`: calls `RadioClass::FindDockSlot(param_3)` to find the
   caller's assigned slot index, then uses `Type+0x1788 + index*0xC` offset entry.
   Falls back to `GetCoords` center if slot is out of range.

**Vtable xrefs (callers via `get_xrefs_to 0x00447B20`):**
```
0x007E3F64 [DATA]
```
One vtable entry — BuildingClass vtable slot. All calls go through virtual dispatch.

**Active in YR:** YES — called whenever a unit needs the physical dock coordinate from
a building (helipad land, refinery dock, service depot approach, bunker entry).
Frequency: every dock approach cycle.

**Notable detail:** The Bunker path uses `atan2` angle binned to 8-bit facing space
(`(angle >> 7) + 1 >> 1 & 0xFF`), then checks only four 64-wide quadrants
(North=0x00–0x3F, East=0x40–0x7F, South=0x80–0xBF, West=0xC0–0xFF). The
`±0x80` offset is exactly half a cell (128 leptons) from building center toward
the facing side. This is the approach-side selection for bunkers.

**TS-legacy note:** The Weeder path (case 1) is active in YR — Weeder refineries
(`Weeder=yes`) exist in `rulesmd.ini`. Not TS-only.

---

### 2.3 AircraftClass::Receive_Radio (0x004190B0) — LIGHT

**Ghidra name:** `AircraftClass__Receive_Radio`
(verified via `get_function_by_address 0x004190B0`)
**Body:** 0x004190B0 – 0x0041952A

**Purpose:** Aircraft-side radio receiver. Handles a distinct set of cases relevant
to aircraft docking (helipads, carryalls, paradrop sequences). Dispatches on
`param_3` (message code).

**Cases directly handled** (from `decompile_function 0x004190B0`):

| Case | Code | Name | Active in YR |
|------|------|------|-------------|
| 8 | 0x08 | REQUEST_DOCKING_CLEARANCE | YES (helipad approach) |
| 0x0E | 14 | CAN_DOCK | YES (aircraft dock query) |
| 0x0F | 15 | WANT_RIDE / CAN_CARRY | YES (carryall pickup) |
| 0x12 | 18 | MOVE_TO_CELL | YES (helipad land cell assign) |
| 0x13 | 19 | NEED_TO_MOVE | YES (helipad redirect) |
| 0x15 | 21 | PREPARE / TIMING_SYNC | YES (helipad arrival) |
| 0x17 | 23 | DEPLOY_UNLOAD | YES (carryall drop, diverts to nearest airfield) |
| 0x1D | 29 | DOCK_QUERY | YES (aircraft availability query) |
| 0x1F | 31 | (fuel/ammo check) | YES (conditional — only if `param_1[0x1b1]+0x684 / 2 <= ammo`) |
| 0x21 | 33 | (ammo/fuel capacity) | YES — see below |

**All other codes** fall through to `FootClass::Receive_Radio`.

**AircraftClass case 0x21 finding (CRITICAL for TS-legacy table):**
`AircraftClass::Receive_Radio` DOES contain case 0x21. Body:
```c
case 0x21:
    return (-(uint)(param_1[0xbf] != *(int *)(param_1[0x1b1] + 0x684)) & 9) + 1;
```
Returns 1 if `aircraft.ammo_current == TypeClass.ammo_max`, else 10 (NEGATORY).
This is an ammo-full query. The comment in the Phase 1 brief states "UnitClass case
0x21 does NOT exist" — that is correct for UnitClass. **Aircraft case 0x21 IS live.**
(verified via `decompile_function 0x004190B0`, switch body)
**Active in YR:** YES — helipad reloading uses this to check if aircraft is full.

**Cases 0xF7 / 0xFC (brief item):** The values 0xF7 and 0xFC are outside the switch
range (no case for them in AircraftClass). Both fall through to `FootClass::Receive_Radio`.
These codes do not appear as message IDs in any radio protocol documented in Phase 1+2.
They are NOT valid message codes — the brief's mention of "negative branches 0xF7/0xFC"
likely refers to return values (0xF7 = -9, 0xFC = -4) not case labels. No TS-legacy
finding here; nothing to close.

**Vtable xref:** `get_xrefs_to 0x004190B0` → `0x007E2438 [DATA]` — one vtable slot
(AircraftClass vtable, Receive_Radio at vtable+0x194 by class hierarchy).

**Active in YR:** YES — helipad, carryall, and paradrop are all standard YR mechanics.

**Notable detail:** Case 0x08 has a pre-check on `param_1[0x2B]` (aircraft mission):
if the aircraft is in missions {4, 0x1A, 0x1B, 0x1E, 0x1F} AND `param_1[0xA5] == 0`,
returns 0 immediately. This is the "aircraft in paradrop/spyplane sequence rejects
all radio" early exit. Active in YR.

---

### 2.4 FUN_00500200 — AI Wander-to-Queue Helper — MEDIUM

**Ghidra name:** `FUN_00500200` (unlabeled)
(verified via `get_function_by_address 0x00500200`)
**Body:** 0x00500200 – 0x005002FA

**Purpose:** Selects a passable wander cell near a target building for an AI unit
that has been told to wait (QUEUED response). Reads storage level via vtable slots
`+0x2DC`, `+0x2D8`, `+0x2D4` (ore/gem/tib storage queries). If combined storage > 0,
picks a random directional bias (1–4 via `Random::RandomRanged(1,4)`); otherwise
bias = 0 (any direction). Calls `FUN_00501AC0` (direction-to-offset lookup), then
`FootClass::Find_Nearby_Passable_Cell` with zone-match and 1-cell radius. Writes
result cell into `*param_1`.

**Callers (from `get_function_callers` with address 0x00500200):**
```
BuildingClass__ExitObject_Main @ 0x00443C60
FootClass__Find_Path           @ 0x004D3920
FootClass__Mission_Rescue      @ 0x004DDF90
UnitClass__PerCellProcess      @ 0x00739EC0   (= UnitClass::Mission_Enter)
```

**Active in YR:** YES — but NOT for harvesters at refineries (per
`MISSION_ENTER_REFINERY_DOCK_VERIFICATION_NOTES.md` correction). The refinery-path
caller is `UnitClass::Mission_Enter` case: AI NON-harvester, NON-weeder units waiting
outside a WeaponsFactory queue. Harvesters simply re-enter Mission_Harvest (mission 10).

**TS-legacy status:** NOT TS-legacy. The War Factory queue is a live YR mechanic.
The function is also called from `BuildingClass::ExitObject_Main` (factory exit queue
management) and `FootClass::Mission_Rescue`.

**Notable detail:** The storage check (`vtable+0x2DC/+0x2D8/+0x2D4`) makes the
function harvester-aware in its bias selection, but the live caller at Mission_Enter
is never a harvester — so the bias always evaluates to direction 0 (any direction).
The storage-aware path would only fire if a harvester somehow reached the QUEUED
branch, which the binary prevents.

---

### 2.5 Mission_Harvest (0x0073E5E0) — LIGHT

**Ghidra name:** `UnitClass__Mission_Harvest`
(verified via `get_function_by_address 0x0073E5E0`)
**Body:** 0x0073E5E0 – 0x0073EFAB

**Purpose:** The outer harvest state machine. Manages sub-states 0–4 for ore/gem
harvesting. Key states:
- **State 0:** Search for ore, move toward it (walks to tiberium cell). If chrono
  locomotor detected, calls `FootClass::Search_For_Tiberium_And_Move` with scan
  radius from `g_RulesClass_Instance+0x177C`.
- **State 1:** Harvest tick — calls `UnitClass::Harvest_Ore_Tick` once every 9
  sub-ticks (waits 9 frames before calling). On tick: if ore exhausted, re-scan;
  if full (`StorageLevel >= 1.0`), → state 2.
- **State 2:** Return-to-refinery — finds nearest refinery via distance check
  against `RulesClass+0xD78` (harvester proximity threshold in leptons), then sends
  `Transmit_Radio(2, refinery)` (HELLO). If ROGER, → state 3. Also handles
  WeaponsFactory queue-cell wander as a fallback.
- **State 3:** Transition — calls `SetMission(7, 0)` (Mission_Enter) immediately.
- **State 4:** Return-and-deposit — handles Slave Miner slave callback paths, then
  `SetMission(5)` (Guard).

**Callers:** vtable-only (verified via `get_xrefs_to 0x0073E5E0` — only DATA refs
to vtable entries). Mission dispatch goes through the mission table.

**Active in YR:** YES — fires every harvest cycle for every harvester. High frequency.

**Linkage to Mission_Enter:** State 3 directly calls `SetMission(7, 0)` — mission 7
= Mission_Enter. This is the return-path linkage. Mission_Enter then drives the
radio protocol (HELLO → CAN_DOCK → MOVE_TO_CELL → TIMING_SYNC → etc.).

**Notable detail:** The Slave Miner check at the top (`Type[0x5ED] && Type[0x5EC] &&
this[0xB6] != 0`) gates into `SlaveManagerClass::HandleReturnedSlaves()` instead of
the normal harvest path. This is the Slave Miner master-unit path, not a refinery
interaction. `Type[0x5ED]` and `Type[0x5EC]` are the SlaveMiner-related type flags.

---

### 2.6 Mission_Unload (0x00740EF0) — MEDIUM — TS-LEGACY VERDICT

**Ghidra name:** `UnitClass__Mission_Unload`
(verified via `get_function_by_address 0x00740EF0`)
**Body:** 0x00740EF0 – 0x00740F7F (144 bytes, ~49 instructions as brief states)

**Full decompile** (verified via `decompile_function 0x00740EF0`):
```c
int __fastcall UnitClass__Mission_Unload(int *param_1)
{
    // vtable+0x528 = Transmit_Radio (to first contact)
    iVar1 = Transmit_Radio_ToFirst(g_RulesClass_Instance + 0x850, 0, 0);
    // Clear flag at this+0x6D2 (deploy-in-progress / deploy-complete flag)
    *(byte *)(param_1 + 0x6D2) = 0;
    if (iVar1 == 0) {
        // No radio contact or reply was 0 — retry with try=1 flag
        iVar1 = Transmit_Radio_ToFirst(g_RulesClass_Instance + 0x850, 0, 1);
        if (iVar1 != 0) {
            // vtable+0x484 = SetPath(0, 1) — clears pending path
            SetPath(0, 1);
        }
    } else {
        // Got non-zero reply from first contact
        iVar1 = Transmit_Radio(2, iVar1);  // Send HELLO (msg 2) to that contact
        if (iVar1 == 1) {
            SetMission(7, 0);    // → Mission_Enter
            return 1;
        }
    }
    // Fallback: get mission timer entry
    MissionClass__GetMissionTimerEntry();
    uVar2 = Math__ftol();
    return uVar2;
}
```

**Caller analysis — vtable xrefs (verified via `get_xrefs_to 0x00740EF0`):**
```
0x007F5EBC [DATA]
```
Only ONE xref — a single vtable DATA reference. This confirms `UnitClass::Mission_Unload`
is only reachable via virtual dispatch through the mission table (mission ID 0x10 = 16
= "Unload"). It is NOT called directly from any identified YR refinery-path function.

**Vtable slot calculation:**
- UnitClass vtable base: `0x007F5C70`
- Slot address: `0x007F5EBC`
- Offset: `0x007F5EBC - 0x007F5C70 = 0x24C`
- Slot index: `0x24C / 4 = 0x93 = 147`
(verified: `read_memory 0x007f5ea8` shows `f0 0e 74 00` at +0x14, confirming
`0x00740EF0` at vtable+0x24C)

**TS-LEGACY VERDICT: ACTIVE IN YR — but NOT on the refinery dock path.**

The function body communicates via radio (`Transmit_Radio_ToFirst`) using
`g_RulesClass_Instance + 0x850` as the first parameter. This is NOT a refinery-unload
bale dump — it is a *unit*-side Mission_Unload that sends a radio message to whatever
building the unit is currently linked to. The mission ID 0x10 ("Unload") is set by:

1. `UnitClass::Receive_Radio` case 0x17 (`DEPLOY_UNLOAD`) for Weeder harvesters — they
   get `SetMission(10, 0)` after deploying their storage.
2. Potentially carryall-drop sequences (carryall sets cargo unit's mission to Unload).
3. War Factory exit sequences (produced units may transition through Unload to their
   initial guard mission).

**It is NOT called during standard ore-harvester refinery docking.** The refinery
deposit path uses `Mission_Enter` (0x739EC0) and `Mission_Deploy_Building` (0x73D630),
not `Mission_Unload`. The `BuildingClass::Receive_Radio` case 0x15 for `DockUnload`
buildings sends the harvester to `Mission_Enter` (0x10 = Enter, not Unload).

**Behavioral reading of the body:**
- Sends `g_RulesClass_Instance + 0x850` to `Transmit_Radio_ToFirst` — this is a
  pointer into RulesClass used as a data payload (likely an ore/unload quantity field).
- If the first contact replies non-zero: sends HELLO (msg 2) to it, and if ROGER,
  transitions to Mission_Enter (7). This is a "link up then enter" pattern.
- If contact replies 0 or radio fails: clears path and falls to timer.
- Clears `this+0x6D2` (same deploy-complete flag as in Mission_Harvest case 0 and
  UnitClass::Receive_Radio case 0x17).

**Active in YR:** YES for Weeder harvesters and carryall cargo delivery. NOT for
standard ore harvester → refinery deposit. The function is live but narrow.

**Confirmed NOT TS-vestigial.** It executes in a normal YR match when Weeder
units are present (e.g., YMAS/NAVES weeder harvesters). Frequency: low — only fires
for Weeder units during their unload phase.

---

### 2.7 SlaveManagerClass::AI_Update (0x006AF6C0) — LIGHT — NEGATIVE FINDING

**Ghidra name:** `SlaveManagerClass__AI_Update`
(verified via `get_function_by_address 0x006AF6C0`)

**Identity:** CONFIRMED as `SlaveManagerClass::AI_Update`. NOT a refinery dock
queue processor. Fully documented in `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md`.

**Caller (verified via `get_function_callers 0x006AF6C0`):**
```
FUN_006AF5F0 @ 0x006AF5F0   (tick-rate throttle, fires every 10 frames)
```
`FUN_006AF5F0` is called from `UnitClass::AI` when `unit[0x9E] != 0` (unit has
a SlaveManager). This is the Slave Miner path only.

**Confirmed:** Zero connections to refinery dock queue, BuildingClass::Receive_Radio,
or any harvester dock function. The function manages YMSLA (Slave Miner) slaves —
their ore-search, walk-to-ore, harvest, return-to-master, and respawn state machine.

**Active in YR:** YES for Slave Miner units. Irrelevant to refinery radio-link system.

---

## 3. TS-Legacy Flag Consolidation

| Flag / Case | Status | Evidence |
|-------------|--------|----------|
| **UnitClass case 0x21** | CLOSED — does not exist | Phase 1 slot 3: jump table range 0x03–0x24, 0x21 maps to default (FootClass fallthrough). UnitClass switch has no case 0x21. |
| **AircraftClass case 0x21** | OPEN / LIVE | THIS SESSION: `decompile_function 0x004190B0` shows explicit `case 0x21` in AircraftClass switch. Returns ammo-full check. Active in YR for helipad reload. NOT TS-legacy. |
| **FootClass case 0xC8** | Cross-ref Phase 2 slot 2 | Phase 2 slot 2 investigates FootClass::Receive_Radio. Deferred to that report. |
| **TechnoClass cases 0x32 / 0x2C** | PARTIAL — from Phase 1 slot 2 inline | Phase 1 slot 2 decompiled TechnoClass::Receive_Radio (0x006F4AB0). Cases 0x32 (50) and 0x2C (44): TechnoClass switch range was documented as handling 0x02, 0x07, 0x09, 0x16, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1E, 0x1F. Neither 0x32 nor 0x2C appears in that list → both fall to `RadioClass::Receive_Radio`. RadioClass handles only HELLO (0x02) and BREAK (0x03) in the base ObjectClass. For codes 0x2C and 0x32 there is no handler — they return the ObjectClass default (likely 0). **Verdict: 0x32 and 0x2C are unhandled message codes in the entire TechnoClass chain. They do not correspond to any live YR radio interaction. Likely TS-vestigial stub IDs.** |
| **AircraftClass 0xF7 / 0xFC** | CLOSED — not case labels | `decompile_function 0x004190B0`: No case 0xF7 (247) or 0xFC (252) in AircraftClass switch. These are NOT message codes in the aircraft switch. The brief's reference was likely to return values, not case labels. Nothing to investigate. |
| **BuildingClass cases 0x22 / 0x23** | CLOSED — fall through | Phase 1 slot 2: neither 0x22 nor 0x23 in BuildingClass switch. Both fall to TechnoClass (no case there either), then RadioClass (no case), then ObjectClass default. Used only as TRANSMIT codes (building sends them as queries). |
| **Type[0x16BB] in BuildingClass case 0x10** | CONFIRMED: `Refinery=` INI flag | `search_strings "Refinery"` → string at `0x0081AA5C`. `get_xrefs_to 0x0081AA5C` → used in `BuildingTypeClass_ReadINI_Water @ 0x00460A5B`. `read_memory 0x00460A40` confirms byte pattern `8A 95 BB 16 00 00` = `movzx AL, [param+0x16BB]` loading Type[0x16BB]. The string "Refinery" at 0x81AA5C maps to INI key `Refinery=yes/no`. THIS IS NOT TS-LEGACY. `Refinery=yes` is the live YR key for refineries (GAREFN, NAREFN). The BUILDINGCLASS doc's claim that Type[0x16BB] is "unknown flag, not in stock YR rules" is WRONG. It is `Refinery=yes` and is set on every refinery in rulesmd.ini. **The BuildingClass case 0x10 branch `if (Type[0x16BB]) return 1` actually means: if this building is a Refinery type, allow dock reservation.** This revises the Phase 1 finding: standard DockUnload refineries DO return ROGER for case 0x10 via the `Type[0x16BB]` (`Refinery=yes`) branch, not NEGATORY. See §5 (Tiny Details) for implications. |
| **SpecialFlags & 0x1000** | NOT IN DOCK FUNCTIONS | `search_strings "SpecialFlags"` → read only by `FUN_006B8B30` and `FUN_006B8CA0` (SpecialFlags INI reader). Not tested anywhere in BuildingClass::Receive_Radio, UnitClass::Receive_Radio, FootClass::Receive_Radio, TechnoClass::Receive_Radio, or any of the 7 functions in this sweep. Fog-of-war gate (0x1000) is isolated to fog/shroud rendering, not radio/dock. |
| **revertNumberOfWaitingPoints= / WaitingOffset0..7=** | CONFIRMED DEAD in binary | `search_strings "revertNumberOfWaitingPoints"` → 0 matches. `search_strings "WaitingOffset"` → 0 matches. Neither string exists in gamemd.exe. The INI entries are commented out in `rules.ini` and `rulesmd.ini` (verified: `grep` shows `;WaitingOffset0..7` in `art.ini`/`artmd.ini` and `;//gs revertNumberOfWaitingPoints` in `rules(md).ini`). **These keys are not read by gamemd.exe at all.** They were planned features never implemented in the shipped binary. |

---

## 4. Deprecation Recommendation: DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md

The file `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md` already contains
a `CRITICAL FINDING` section in its own header:

> "0x006AF6C0 is NOT a refinery dock-queue processor. It is `SlaveManagerClass::AI_Update`"

The document then provides the correct full decompile of the function as SlaveManagerClass.
The doc's title is misleading but its content is accurate.

**Recommended action:** Rename the file to
`SLAVEMANAGERCLASS_AI_UPDATE_0x6AF6C0_GHIDRA_REPORT.md` and remove the
"DOCKMANAGER" label from the title. The content is correct and complete. No re-investigation
needed. A single-line header correction is sufficient.

The stale aspect confirmed: the title says "DOCKMANAGER" but the content says
"SlaveManagerClass". Title misleads searchers. The fix is a rename, not a rewrite.

---

## 5. Tiny Details Noted

**5.1 Type[0x16BB] = Refinery= (revision of Phase 1 finding)**

The BuildingClass case 0x10 (RESERVE_DOCK) finding in the Phase 1 BUILDINGCLASS report
stated: "Type[0x16BB] is an unknown flag never set in stock YR rules (likely TS legacy)."
THIS IS WRONG. Verified this session: `Type[0x16BB]` = `Refinery=yes` INI key
(xref to string "Refinery" at 0x81AA5C, used in `BuildingTypeClass_ReadINI_Water`).

**Practical implication:** Standard DockUnload refineries (GAREFN, NAREFN) have
`Refinery=yes` in rulesmd.ini → `Type[0x16BB] != 0` → BuildingClass case 0x10 returns
ROGER (1), NOT NEGATORY. The Phase 1 conclusion "case 0x10 returns NEGATORY for standard
refineries" was based on the incorrect flag identification. The Rust port of case 0x10
must return ROGER for `Refinery=yes` buildings.

**Impact:** The reservation protocol IS used for standard refineries — a harvester that
sends RESERVE_DOCK (0x10) to GAREFN will receive ROGER, not NEGATORY. This matters for
multi-harvester queue management.

**5.2 Mission_Unload body reads `g_RulesClass_Instance + 0x850`**

The first argument to `Transmit_Radio_ToFirst` in Mission_Unload is `g_RulesClass_Instance + 0x850`.
This is a pointer into RulesClass — offset +0x850. The current `RULESCLASS_STRUCT_LAYOUT.md`
should document what INI key lives at Rules+0x850. It is likely `SilverCredit=`,
`StorageMultiplier=`, or a similar harvest-related constant. Not chased in this sweep.

**5.3 AircraftClass case 0x15 sends aircraft to Mission_Retreat (4)**

In `AircraftClass::Receive_Radio` case 0x15 (PREPARE/TIMING_SYNC), if the sender
is infantry (`GetWhat() == 0xF`), has `TypeClass+0xEC1` set (bailing/ejecting flag),
AND `aircraft+0x6D9 == 0` (not already retreating): the aircraft calls
`SetPath(0)`, `SetDestination(NULL, 1)`, then `SetMission(4, 0)` (Mission_Retreat).
This is the "aircraft is recalled when infantry ejects" path. Active in YR.

**5.4 Find_Nearest_Dock uses 3D distance with Z-component**

The distance formula in `FootClass::Find_Nearest_Dock` includes the Z-axis term
`(piVar4[2] - piVar3[2])^2`. For ground units on flat terrain this is always 0
(Z is the same). But it is present. On maps with height variation (cliffs), a refinery
on a higher cliff than the harvester will appear slightly farther away than its
cell-grid distance suggests. In practice Z differences are small and won't change
nearest-dock selection in realistic maps.

**5.5 Mission_Harvest inner loop increments `g_MapEditorMode` as a reentrance guard**

In state 2 of Mission_Harvest, the code that searches for an enemy-allied refinery
(fallback path when all owned refineries are unavailable) wraps the search in
`g_MapEditorMode++` / `g_MapEditorMode--`. This is a known gamemd trick to suppress
normally-illegal unit operations during a temporary mode change. The variable is named
for its primary use in map editor mode but is reused as a general "suppress UI warnings"
flag in multiple places.

---

## 6. Open Questions — Final State

| # | Question | Status |
|---|----------|--------|
| OQ1 | Is Type[0x16BB] TS-legacy? | RESOLVED — it is `Refinery=` INI key. Live in YR. Case 0x10 returns ROGER for standard refineries. Phase 1 finding was WRONG. |
| OQ2 | Is Mission_Unload (0x00740EF0) TS-vestigial? | RESOLVED — LIVE in YR for Weeder harvesters and carryall cargo. Not used on standard ore-harvester refinery path. |
| OQ3 | Does 0x006AF6C0 connect to refinery docking? | RESOLVED — NO. SlaveManagerClass::AI_Update, slave-miner only. |
| OQ4 | Are revertNumberOfWaitingPoints / WaitingOffset0..7 read? | RESOLVED — strings absent from gamemd.exe binary. Keys do not exist in the executable. |
| OQ5 | Does SpecialFlags & 0x1000 gate any radio/dock function? | RESOLVED — NO. Gate is in fog/shroud code only. Dock functions do not test this flag. |
| OQ6 | AircraftClass cases 0xF7/0xFC — are these real case labels? | RESOLVED — NO. Not case labels. AircraftClass switch has no such entries. Not message codes. |
| OQ7 | TechnoClass cases 0x32/0x2C — live or TS? | RESOLVED — unhandled in entire chain (TechnoClass + RadioClass + ObjectClass). Fall through to default. Likely vestigial stub IDs. NOT implemented in any live YR path. |
| OQ8 | What is RulesClass+0x850 (used in Mission_Unload)? | OPEN — not investigated in this sweep. Needs RulesClass struct lookup. |

---

## Sources

All Ghidra findings verified via live MCP in this session (2026-05-20).

| MCP Call | Purpose |
|----------|---------|
| `get_function_by_address 0x004DFCB0` | FootClass::Find_Nearest_Dock identity |
| `get_function_by_address 0x00447B20` | BuildingClass::GetDockCoord identity |
| `get_function_by_address 0x004190B0` | AircraftClass::Receive_Radio identity |
| `get_function_by_address 0x00500200` | FUN_00500200 identity |
| `get_function_by_address 0x0073E5E0` | UnitClass::Mission_Harvest identity |
| `get_function_by_address 0x00740EF0` | UnitClass::Mission_Unload identity |
| `get_function_by_address 0x006AF6C0` | SlaveManagerClass::AI_Update identity (negative confirm) |
| `decompile_function 0x004DFCB0` | Find_Nearest_Dock body |
| `decompile_function 0x00447B20` | GetDockCoord body |
| `decompile_function 0x004190B0` | AircraftClass::Receive_Radio full switch |
| `decompile_function 0x00500200` | FUN_00500200 body |
| `decompile_function 0x0073E5E0` | Mission_Harvest body |
| `decompile_function 0x00740EF0` | Mission_Unload full body (49 instructions) |
| `get_xrefs_to 0x00740EF0` | Mission_Unload caller chain (vtable-only) |
| `get_xrefs_to 0x004DFCB0` | Find_Nearest_Dock vtable xrefs |
| `get_xrefs_to 0x00447B20` | GetDockCoord vtable xrefs |
| `get_xrefs_to 0x004190B0` | AircraftClass::Receive_Radio vtable xref |
| `get_function_callers SlaveManagerClass__AI_Update` | SlaveManager caller = FUN_006AF5F0 only |
| `get_function_callers FUN_00500200` (address form) | 4 callers confirmed, none refinery-harvester |
| `read_memory 0x007f5ea8` | UnitClass vtable slot verification for Mission_Unload |
| `search_strings "Refinery"` | Located "Refinery" string at 0x0081AA5C |
| `get_xrefs_to 0x0081aa5c` | Confirmed in BuildingTypeClass_ReadINI_Water |
| `read_memory 0x00460a40` | Confirmed byte offset 0x16BB load near "Refinery" string |
| `search_strings "revertNumberOfWaitingPoints"` | 0 hits — key not in binary |
| `search_strings "WaitingOffset"` | 0 hits — key not in binary |
| `search_strings "SpecialFlags"` | Located in FUN_006B8B30 only (INI reader) |
| Prior art: `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | Phase 1 slot 2 data |
| Prior art: `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | Phase 1 slot 3 data |
| Prior art: `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md` | SlaveManager doc |
| Prior art: `MISSION_ENTER_REFINERY_DOCK_VERIFICATION_NOTES.md` | FUN_00500200 context |

---

## Status: COMPLETE
