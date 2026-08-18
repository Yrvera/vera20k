# UnitClass__Mission_Guard_Harvester @ 0x00740810

**Proposed Ghidra label:** UnitClass__Mission_Guard_Harvester

## Summary

`UnitClass::Mission_Guard_Harvester` is the mission tick handler for harvester units in the
`GUARD` mission state — specifically the "guarding while waiting to harvest" state entered from
`UnitClass::Mission_Harvest` state 4 (LOST: no ore found anywhere). It is dispatched via the
UnitClass vtable mission-handler table (DATA xref at `0x007F5E8C`, confirmed via
`get_xrefs_to 0x00740810`).

The function has four main dispatch paths:

1. **Slave manager recall** (`param_1[0xB6]` = SlaveManager ptr, TS/Yuri slave miner): recall
   slaves if cooldown expired. YELLOW: likely TS/Yuri slave-specific path, irrelevant to CMIN.

2. **Harvester-type unit (Harvester=yes or IsHarvester)**: attempts to switch back to
   Mission_Harvest (mission 10). For AI-controlled teleporter units (`Teleporter=yes`,
   `TechnoTypeClass+0xCD4`), scans 8 adjacent cells for a refinery then re-enters harvest.
   For player-controlled harvesters, checks if the player's refinery list has any free
   refineries; if so, re-enters harvest (mission 10).

3. **HarvesterUnit list check**: if the unit type is in `RulesClass.HarvesterUnits` list
   (`g_RulesClass_Instance + 0x8B0`), switches to Mission_Harvest (`SetMission(0x10)`) for
   AI-controlled units that have been idle.

4. **IsHarvester flag + guard-while-waiting**: clears the "guard while wait" flag and
   re-enters harvest (mission 10).

5. **Fall-through**: calls `FootClass::Mission_Guard @ 0x004D5070` for base guard behavior
   (find adjacent garrison building, scan for nearby refinery to approach, etc.).

**Active in YR:** Yes. CMIN is a harvester unit (`Harvester=yes`, `IsHarvester=yes` in
rulesmd.ini), so this function fires whenever CMIN transitions to GUARD mission after failing
to find ore. The `Teleporter` branch (path 2b) is CMIN-specific — only fires when
`TechnoTypeClass+0xCD4 != 0`, which is true for CMIN.

---

## Decompilation excerpt

Verified via `decompile_function 0x00740810`.

```c
int __fastcall UnitClass__Mission_Guard_Harvester(int *param_1)
{
  char cVar1;
  int iVar2;
  int iVar3;
  int *piVar4;
  float10 fVar5;

  // --- PATH 1: Slave manager recall (TS/Yuri slave miner path) ---
  if (((param_1[0xb6] != 0) &&                            // UnitClass+0x2D8 = SlaveManagerClass* ptr
      (*(int *)(g_RulesClass_Instance + 0x1790) + param_1[0x30] < g_CurrentFrameCounter)) &&
     (cVar1 = SlaveManagerClass__ShouldRecallSlaves(), cVar1 != '\0')) {
    SlaveManagerClass__RecallAllSlaves();
LAB_00740a1f:
    MissionClass__GetMissionTimerEntry();
    iVar2 = Math__ftol();
    iVar3 = Random__RandomRanged(0, 2);
    return iVar3 + iVar2;
  }

  // --- PATH 2: Harvester-type unit ---
  if ((*(char *)(param_1[0x1b1] + 0xe0e) != '\0') ||    // TechnoTypeClass+0xE0E = Harvester flag
      (*(char *)(param_1[0x1b1] + 0xe0f) != '\0')) {    // TechnoTypeClass+0xE0F = IsHarvester flag
    cVar1 = HouseClass__IsPlayerControl();
    if (cVar1 == '\0') {
      // AI-controlled harvester: check if player owns any ore refinery
      // (loops over TechnoType dock list at +0x3F8 / +0x3EC)
      // if yes → SetMission(10, 0) [Mission_Harvest]
      // ...
      // TELEPORTER-specific sub-path: (below)
    } else if (*(char *)(param_1[0x1b1] + 0xcd4) != '\0') {  // TechnoTypeClass+0xCD4 = Teleporter
      // Player-controlled teleporter harvester (CMIN):
      // Scan 8 adjacent cells (Pathfinding_update_continued loop for each direction)
      // for a building with +0x16bb (IsRefinery?) flag AND
      // matching house (param_1[0x87] = unit's HouseClass*):
      (**(code **)(*param_1 + 0x1bc))();  // vtable+0x1BC = GetCurrentCell()
      iVar2 = 0;
      do {
        Pathfinding_update_continued(iVar2);   // iterates 8 directions
        iVar3 = Look_up_building_in_cell();    // 0x0047C520: find building in cell
        if (((iVar3 != 0) &&
             (*(char *)(*(int *)(iVar3 + 0x520) + 0x16bb) != '\0')) &&  // building IsRefinery?
            (*(int *)(iVar3 + 0x21c) == param_1[0x87])) {  // same house?
            // → SetMission(10, 0)
            goto LAB_0074092c;  // SetMission(10, 0)
        }
        iVar2 = iVar2 + 1;
      } while (iVar2 < 8);
      // No adjacent refinery found; check if locomotor is "Is_Moving" == false
      fVar5 = (float10)(**(code **)(*param_1 + 0x2b4))();  // vtable+0x2B4 = ILocomotion::Is_Moving
      if (fVar5 == (float10)_g_Const_1_0) {  // 1.0 = true (moving)
        cVar1 = (**(code **)(*(int *)param_1[0x19d] + 0x10))((int *)param_1[0x19d]);  // Is_Moving
        if (cVar1 != '\0') {
          // SetMission(10, 0)
          (**(code **)(*param_1 + 0x1e8))(10, 0);
          return 1;
        }
      }
    }
  }

  // --- PATH 3: HarvesterUnits list check ---
  if (*(int *)(param_1[0x1b1] + 0x404) != 0) {  // TechnoTypeClass+0x404: HarvesterUnits membership
    // search g_RulesClass.HarvesterUnits list (0x8B0 = ptr, 0x8BC = count)
    // if found AND unit is idle (param_1[0x87]+499 = HasTiberium/InFactory?) AND AI:
    // → SetMission(0x10, 0) + return via LAB_00740a1f
  }

  // --- PATH 4: IsHarvester guard-while-waiting ---
  if ((*(char *)(param_1[0x1b1] + 0xe0f) != '\0') && ((char)param_1[0x1ae] != '\0')) {
    // UnitClass+0x1AE*4 = 0x6B8 = "guard while waiting" flag
    *(undefined1 *)(param_1 + 0x1ae) = 0;   // clear flag
    (**(code **)(*param_1 + 0x1e8))(10, 0);  // SetMission(10) = Harvest
  }

  // --- PATH 5: Fall-through to base guard ---
  iVar2 = FootClass__Mission_Guard();    // 0x004D5070
  return iVar2;
}
```

The label `LAB_0074092c`:
```c
(**(code **)(*param_1 + 0x1e8))(10, 0);  // SetMission(10, 0) = re-enter Harvest mission
return 1;
```

---

## Behavioral analysis

### When is this function called?

Dispatched via vtable mission slot (DATA ref at `0x007F5E8C`, verified via
`get_xrefs_to 0x00740810`). The UnitClass mission dispatcher calls this when `Mission == 10`
(decimal) = GUARD. This is the `Mission_Guard_Harvester` variant (not the generic
`UnitClass::Mission_Guard @ 0x00740A90`); it is distinct from the latter.

`UnitClass::Mission_Harvest` state 4 (LOST) transitions to this mission via `SetMission(10, 0)`.

### CMIN-specific path (Teleporter=yes, player-controlled)

For the chrono miner (player-controlled, `Teleporter=yes`):
```
TechnoTypeClass+0xE0E (Harvester=yes) → enter PATH 2
IsPlayerControl() == true → else branch
TechnoTypeClass+0xCD4 (Teleporter=yes) → enter Teleporter sub-path
  Scan 8 adjacent cells for refinery with same house
  → if found: SetMission(10) → re-enter harvest
  → if not found AND locomotor not moving: SetMission(10) → re-enter harvest
  → otherwise: fall through to PATH 5 (FootClass::Mission_Guard)
```

The "adjacent refinery" check uses `Look_up_building_in_cell @ 0x0047C520` (scans
`CellClass+0xE4` for RTTI==6 building) with `BuildingTypeClass+0x16BB` flag check (IsRefinery?).
The same `Look_up_building_in_cell` pattern is used here as in the refinery-unload path.

### AI harvester path

For AI-controlled harvester (`IsPlayerControl() == false`):
- Iterates the unit type's dock list (`TechnoTypeClass+0x3EC/+0x3F8` = dock building list).
- For each dock type, checks `HouseClass::CountOwnedInstances` — if any friendly refineries
  exist, transitions to Mission_Harvest (mission 10).

### Idle / fall-through behavior

If none of the re-harvest conditions fire:
- PATH 4 checks the "guard-while-waiting" flag (`UnitClass+0x6B8`); if set, clears it
  and re-enters harvest.
- PATH 5: `FootClass::Mission_Guard @ 0x004D5070` — scans adjacent cells for a garrison
  building, approaches it if found (mission 1 = Enter/Garrison), otherwise uses
  `FUN_00703590` (zone passable cell finder) for random roaming. Returns mission timer ticks.

### Return value

Returns number of frames to wait before next tick (mission timer delay). Typical return path:
`MissionClass::GetMissionTimerEntry()` + `Math::ftol()` + optional small random offset `[0..2]`.

### Active in YR

**Active: Yes.** CMIN enters `Mission_Guard_Harvester` every time it exhausts ore search.
The Teleporter-specific sub-path is CMIN-only and fires in standard YR skirmish.

The **slave manager path** (PATH 1): fires only when `UnitClass+0x2D8 != 0` (SlaveManager
present). CMIN has no SlaveManager — path is **dead for CMIN**. Active for Yuri slave miners
only. TS-legacy concern: `SlaveManagerClass` is a YR Yuri mechanic, not TS.

---

## Struct field accesses

`param_1` is `int *` = UnitClass `this` pointer. All offsets computed as `field_index × 4`.

| Expression | Byte offset | Field | Notes |
|---|---|---|---|
| `param_1[0xb6]` | 0x2D8 | `UnitClass::SlaveManagerPtr` (or `FootClass` equivalent) | SlaveManager pointer; non-null only for Yuri slave miners |
| `param_1[0x30]` | 0xC0 | idle frame counter (last action frame) | used with `g_RulesClass_Instance+0x1790` cooldown |
| `param_1[0x1b1]` | 0x6C4 | `TechnoClass::TechnoType` ptr (TechnoTypeClass*) | same field accessed in `TechnoClass::Set_Destination` as `param_1[10].vtable_INoticeSource` |
| `param_1[0x87]` | 0x21C | `TechnoClass::House` ptr (HouseClass*) | unit's owner house |
| `param_1[0x19d]` | 0x674 | `FootClass::Locomotor` (ILocomotion*) | active locomotor; used for `Is_Moving` check |
| `param_1[0x1ae]` | 0x6B8 | `UnitClass::GuardWhileWaiting` flag (bool) | cleared and re-harvest triggered if set |
| `TechnoTypeClass + 0xE0E` | 0xE0E | `TechnoTypeClass::Harvester` (bool) | INI `Harvester=` flag; gates PATH 2 |
| `TechnoTypeClass + 0xE0F` | 0xE0F | `TechnoTypeClass::IsHarvester` (bool) | related flag; used in PATH 2 and PATH 4 |
| `TechnoTypeClass + 0xCD4` | 0xCD4 | `TechnoTypeClass::Teleporter` (bool) | INI `Teleporter=`; verified via TechnoTypeClass__ReadINI (`int*` param: `0x335 × 4 = 0xCD4`) |
| `TechnoTypeClass + 0x3EC` | 0x3EC | dock type list ptr | used in AI refinery scan |
| `TechnoTypeClass + 0x3F8` | 0x3F8 | dock type list count | used in AI refinery scan |
| `TechnoTypeClass + 0x404` | 0x404 | HarvesterUnits membership ptr | PATH 3 check |
| `BuildingClass + 0x520` | 0x520 | BuildingTypeClass* ptr | used to access building type flags |
| `BuildingTypeClass + 0x16BB` | 0x16BB | IsRefinery? flag | used in Teleporter path to identify refinery |
| `BuildingClass + 0x21C` | 0x21C | HouseClass* (owner house) | used to check same-house refinery |

---

## Globals / enums / INI

| Global / INI | Address / offset | Role |
|---|---|---|
| `g_RulesClass_Instance + 0x1790` | int | Harvester idle cooldown threshold (frames) |
| `g_RulesClass_Instance + 0x8B0` | int* | `RulesClass::HarvesterUnits` list ptr |
| `g_RulesClass_Instance + 0x8BC` | int | `RulesClass::HarvesterUnits` list count |
| `g_CurrentFrameCounter` | global int | current frame |
| `_g_Const_1_0` | float10 constant 1.0 | used in `Is_Moving` floating-point comparison |
| `TechnoTypeClass::Teleporter` | +0xCD4 (bool) | INI `Teleporter=` |
| `TechnoTypeClass::Harvester` | +0xE0E (bool) | INI `Harvester=` |

Mission IDs seen:
- `10` = `HARVEST` (decimal) — re-enter harvest state machine
- `0x10` = `16` = also HARVEST (same number via hex path in PATH 3)
- `1` = `ENTER` — enter/garrison building
- `0x11` = `17` — seen in `FootClass::Mission_Guard` for factory-guard sub-mission (YELLOW)

---

## Out-of-scope refs

- `SlaveManagerClass::ShouldRecallSlaves @ 0x006B1020` — slave miner system; out of scope for CMIN
- `SlaveManagerClass::RecallAllSlaves @ 0x006B0CC0` — slave miner system; out of scope for CMIN
- `HouseClass::CountOwnedInstances @ 0x0049FAE0` — AI house query; out of scope
- `HouseClass::IsPlayerControl @ 0x0050B730` — house query; general utility
- `Pathfinding_update_continued @ 0x00481810` — pathfinding; out of scope
- `FootClass::Mission_Guard @ 0x004D5070` — base guard mission (task #52); out of scope for this decode
- `MissionClass::GetMissionTimerEntry @ 0x005B3A00` — mission tick timer; out of scope
- `FUN_00703590 @ 0x00703590` — zone passable cell finder (task #54); appears inside `FootClass::Mission_Guard`
- `Look_up_building_in_cell @ 0x0047C520` — used for adjacent refinery scan

---

## Unverified (YELLOW)

- **`BuildingTypeClass+0x16BB` = IsRefinery?** — field name inferred from context
  (used to decide if adjacent building is worth re-harvesting toward). Not cross-verified
  via ReadINI in this session. `+0x16A9 = Harvester`, `+0x16AB = Guard`, `+0x16B3 = Dock`
  seen in `TechnoClass::Set_Destination`; `+0x16BB` is two slots further — possibly
  `Refinery`. YELLOW.
- **`TechnoTypeClass+0x404` = HarvesterUnits membership** — inferred from context
  (compared against `g_RulesClass.HarvesterUnits` list entries). Not verified via ReadINI. YELLOW.
- **`UnitClass+0x6B8` (param_1[0x1ae]) = "GuardWhileWaiting" flag** — inferred from context
  (set during harvest loop when miner is instructed to wait, cleared here to re-enter harvest).
  Field name not verified in binary. YELLOW.
- **`UnitClass+0x2D8` (param_1[0xb6]) = SlaveManagerClass ptr** — inferred from call to
  `SlaveManagerClass__ShouldRecallSlaves`. YELLOW (exact field name not confirmed in struct doc).
- **Mission IDs 10 vs 0x10**: both appear in the code and both equal decimal 16.
  The consistency of `SetMission(10, 0)` mapping to `Mission_Harvest` is inferred from
  `UnitClass::Mission_Harvest` being the harvest handler and Ghidra's naming; not directly
  verified via a mission-dispatch table lookup in this session. YELLOW.
- **vtable slot at `0x007F5E8C`**: confirmed as the UnitClass mission-handler vtable entry via
  `read_memory 0x007F5E80` (shows `0x007447A0`, `0x004D4B20`, `0x004D4CB0`, `0x00740810` as
  4 consecutive slots). The exact slot index within the vtable was not computed (requires
  knowing vtable base address). YELLOW.
