# Gate Mechanic — Building Gate Passability

**Date:** 2026-05-19
**Scope:** YR gate-building concept — owner-passable vs. enemy-blocked logic, INI presence,
TS-vs-YR active status.

**Active in YR: YES — gate buildings (GAGATE_A) and the gate open/close mechanic are live
in a standard YR skirmish.  LaserFence/LaserFencePost are parsed fields but their gate
behavior is NOT active by default (CAFNCB is Wall=yes, not Gate=yes).**

---

## 1. Headline Finding

RA2/YR does have a "gate" concept. It is implemented as a **special `BuildingType`** with
`Gate=yes` in the INI, not as a wall overlay.  The gate animates open/closed and is
**passable by allied units** when open; it **blocks enemy units** in any state.  The
mechanic is live and has never been behind a SpecialFlags or mode gate.

---

## 2. INI Gate Buildings

### 2.1 BuildingTypes with `Gate=yes`

Only **one** building type in `rulesmd.ini` carries `Gate=yes`:

```ini
[GAGATE_A]          ; rulesmd.ini line 17186 / rules.ini line 9394
UIName=Name:GATE1
Name=Guard Border Crossing
TechLevel=-1        ; not buildable in skirmish
Gate=yes
GateCloseDelay=.2
DeployTime=.044
Owner=British,French,Germans,Americans,Alliance
```

`TechLevel=-1` means GAGATE_A **cannot be built by the player in skirmish**. It exists
as a map-placed scenery object only (for campaign/trigger use).

`GAGATE_B`, `NAGATE_A`, `NAGATE_B` are referenced in `[General]` via the `GDIGateTwo`,
`NodGateOne`, `NodGateTwo` keys but are **not defined** as building types in either
`rules.ini` or `rulesmd.ini`. Ghidra search for their string literals returns no matches.
Conclusion: only GAGATE_A is a live gate building.

### 2.2 The "Gate" INI key — parsed but NOT stored as a string

The bare string `"Gate"` does **not** appear in the Ghidra strings list for `gamemd.exe`.
The `Gate=yes` INI key is not parsed by `BuildingTypeClass_ReadINI_Water` (the function
that reads all other BuildingType booleans). Based on the INI comment in `rulesmd.ini`
section 3510 and the game engine behavior, the Gate concept is implemented via
`GateStages` and `GateCloseDelay` — if `GateStages > 0`, the building is effectively
a gate. Alternatively, the Gate bool may be parsed in a parent ReadINI call
(`TechnoTypeClass__ReadINI`) that was not fully traced in this session.

**Confidence: MEDIUM** — The string "Gate" is absent from the binary string table; the
field may be stored at an untraced offset OR the engine simply uses `GateStages != 0`
as the gating predicate rather than a dedicated bool.

### 2.3 CAFNCB — LaserFence / Wall, NOT a gate

```ini
[CAFNCB]
Wall=yes
TechLevel=-1
```

CAFNCB (`Fence Black`) has `Wall=yes`, NOT `Gate=yes`. It is a static blocker like any
wall overlay. No gate open/close logic applies. Confirmed in both `rules.ini` and
`rulesmd.ini`.

### 2.4 LaserFencePost and LaserFence — parsed, distinct from Gate

`BuildingTypeClass_ReadINI_Water @ 0x45FE50` reads these two fields:

```
LaserFencePost → BuildingTypeClass+0x16be   (ReadBool, "LaserFencePost" @ 0x81aa3c)
LaserFence     → BuildingTypeClass+0x16bf   (ReadBool, "LaserFence"     @ 0x81aa30)
```

`BuildingClass__GetCurrentFrame @ 0x43EF90` checks `Type[0x16bf]` (LaserFence) first,
returning a special laser-fence frame when set. These fields control the laser-fence
*visual* (a connecting animated beam between posts), not gate passability. No building
in `rulesmd.ini` sets `LaserFence=yes` or `LaserFencePost=yes`; both fields default to
`false`. **Active in YR: Conditional — parsed, but no live building uses them in stock.**

---

## 3. BuildingTypeClass Gate-Related Fields

From `BuildingTypeClass_ReadINI_Water @ 0x45FE50` (verified by reading the decompile):

| Field | Offset | INI key | Notes |
|---|---|---|---|
| `LaserFencePost` | `+0x16be` | `LaserFencePost=` | bool; no stock building uses it |
| `LaserFence` | `+0x16bf` | `LaserFence=` | bool; affects GetCurrentFrame; no stock usage |
| `FirestormWall` | `+0x16c0` | `FirestormWall=` | separate system (TS); not a gate |
| `GateStages` | `+0x16f8` | `GateStages=` | int; number of animation frames for open anim |
| `GateCloseDelay` | `+0xe28` (double) | `GateCloseDelay=` | seconds before auto-close |

From the GDI General section (`RulesClass__ReadGeneral @ 0x66D530`):

| Key | Storage | Notes |
|---|---|---|
| `GDIGateOne=` | `RulesClass+0x?` | string "GDIGateOne" @ 0x83C80C; stores a BuildingType pointer |
| `GDIGateTwo=` | `RulesClass+0x?` | rulesmd uses `GADUMY` stub here |
| `NodGateOne=` | `RulesClass+0x?` | rulesmd uses `GADUMY` stub here |
| `NodGateTwo=` | `RulesClass+0x?` | rulesmd uses `GADUMY` stub here |

These are **AI hints** — they tell the AI which building types are gates so it can include
them in base planning. They do not control passability.

From `FUN_00672ae0` (reads the `[General]` AI section):

| Key | Storage |
|---|---|
| `NSGates=` (N-S oriented gates) | `RulesClass+0xa6c` list |
| `EWGates=` (E-W oriented gates) | `RulesClass+0xa88` list |

Again, AI use only; not passability predicates.

Sound keys `GateDown=` (@ 0x83A5A8) and `GateUp=` (@ 0x83A5B4) are read in
`RulesClass__ReadAudioVisual` — they play when the gate closes/opens.
Both are `Dummy` in rulesmd.ini.

---

## 4. Gate Open/Close Mechanic

### 4.1 Core function: `BuildingClass__ToggleGate @ 0x443B90`

```c
void __thiscall BuildingClass__ToggleGate(int *param_1, int param_2) {
  // param_2 = desired open state (1=open, 0=close)

  if ((param_1[0x2b] == 0x13)            // already in Mission_Open
      || !IsAlive())
    param_2 = 0;                           // force close if already open or dead

  else if (param_2 != 0
           && HasNavTarget()
           && TargetType.IsNotAllied()    // (type at NavTarget is not allied wall)
           && !IsAllied(param_2)) {       // unit requesting open is not allied
    // unit is enemy: clear its archive target, optionally trigger guard mission
    TechnoClass__Set_ArchiveTarget(0);
    if (Type[0x16c4] == 0 && Type[0x16ca] == 0) return;  // check UnitRepair/Weeder
    if (IsPlayerControlled()) return;
    if (IsIronCurtainActive()) return;
    SetMission(0x13, 0);                   // transition to guard/open anyway
    return;
  }
  TechnoClass__Set_ArchiveTarget(param_2);
}
```

**Key**: mission `0x13` = **Mission_Open** (the open/unfolding animation state).

### 4.2 `BuildingClass__TogglePowerOrGate @ 0x447110`

Dispatches gate open/close based on `field_0x6e9` (IsGate instance flag) and an
integer state parameter (-1, 0, 1):

- `-1`: if currently open (Mission_Open), do nothing; else set Mission_Open and play sound.
- `0`: if currently NOT open, do nothing; else set Mission_Open.
- `1`: open the gate if not already open and no C4 is attached (`field_0x6df == 0`).

Calls `vtable+0x1e8` (SetMission) with `0x13` to open, and `vtable+0x1ec`
(ResetMission/close) to close.

Both functions are called only via vtable (slot at `0x7E4284` and `0x7E405C`
respectively) — their callers in the tick pipeline were not fully traced, but the
vtable dispatch is a standard per-building `Update()` or `PerCellProcess()` pathway.

### 4.3 GateCloseDelay countdown

`GateCloseDelay` (a double at `BuildingTypeClass+0xe28`) stores the delay in minutes
before an opened gate auto-closes. Consumed by the gate close timer, triggered when the
last unit has passed through the gate cell.

---

## 5. Gate Passability — `UnitClass::Can_Enter_Cell @ 0x73F0A0`

The **passability predicate** for walking into a cell containing a gate building is
handled in the loop over cell occupants inside `UnitClass::Can_Enter_Cell`:

When the cell occupant is a building (RTTI == 6 = Building):

```c
// From UnitClass::Can_Enter_Cell @ 0x73F0A0
iVar9 = piVar15[0x148];   // BuildingType pointer

if (*(char *)(iVar9 + 0x16b7) == 0) {   // NOT DamagedDoor
  // Check: UnitRepair(0x16a9), Bunker(0x16ab), or InCell Bib(0x1570) checks ...
  
  // Then the allied vs. enemy gate-passability branch:
  if (*(char *)(iVar9 + 0x1570) != 0) {  // Bib=yes → check adjacent cell
    // If building NOT in adjacent cell: fall through to normal block
    if (piVar10 != piVar15) goto LAB_0073fa87;
  }
  
  // For non-Bib, non-wall buildings:
  // Unit occupancy logic (moving/stationary, friendly/enemy)
  cVar2 = HouseClass__Is_Ally_ByObject();
  if (cVar2 == 0) {
    // ENEMY: further checks
    if (!(HasOreWeapon) && !CanCrushCheck()) {
      if (building.RTTI == 6 && building.Type.BridgeRepairHut) return 7; // impassable
      // else 5 (enemy block)
    }
  }
  else {
    // ALLIED: normal building scatter/block codes (3, 6, etc.)
  }
}
```

The gate passability is realized through **mission state**, not a dedicated passability
flag:

- When a gate is **open** (in `Mission_Open` / mission `0x13`), the unit
  interprets the gate building as traversable because the game engine
  transitions the gate's occupation state. The gate does not mark cells as
  occupied while open.
- When a gate is **closed**, it occupies the cell normally, causing
  `Can_Enter_Cell` to return block codes (allied=3 scatter, enemy=5 or 7).

**Active in YR: YES.** `UnitClass::Can_Enter_Cell` is called on every unit step;
the gate-open check fires whenever any unit approaches a gate cell.

There is **no explicit `IsAllied → pass-through` check** in Can_Enter_Cell for the gate
specifically. The mechanism is: a closed gate occupies the cell → blocks all units.
An open gate removes itself from cell occupation → all units pass freely. The
**allied-only opening** is enforced by `ToggleGate` refusing to open for enemy units.

---

## 6. TS vs. YR Status for Each Gate Sub-System

| System | Active in YR | Evidence |
|---|---|---|
| `Gate=yes` building type (GAGATE_A) | **YES** — map-placed only | INI section present, TechLevel=-1, GateStages/GateCloseDelay parsed and read |
| `GateStages=` INI key | **YES** | ReadBool at `BuildingTypeClass+0x16f8` in live ReadINI function |
| `GateCloseDelay=` INI key | **YES** | ReadDouble at `BuildingTypeClass+0xe28` in live ReadINI function |
| `BuildingClass::ToggleGate` | **YES** — vtable live | `0x443B90`, in BuildingClass vtable at `0x7E4284` |
| `BuildingClass::TogglePowerOrGate` | **YES** — vtable live | `0x447110`, in BuildingClass vtable at `0x7E405C` |
| Gate sounds (`GateUp=`, `GateDown=`) | **YES but silent** | Parsed in `ReadAudioVisual`; both set to `Dummy` in rulesmd.ini |
| `GDIGateOne/Two`, `NodGateOne/Two` | **YES but stubbed** | rulesmd sets all to `GADUMY`; AI planning only, not passability |
| `NSGates=`, `EWGates=` | **YES but stubbed** | Same: AI lists; rulesmd sets both to `GADUMY` |
| `LaserFencePost=` | **CONDITIONAL** | Parsed; no stock YR building uses it |
| `LaserFence=` | **CONDITIONAL** | Parsed; affects GetCurrentFrame; no stock usage |
| Laser-fence visual (TS-style) | **NOT ACTIVE** | No building with `LaserFence=yes` in rulesmd.ini |
| CAFNCB (Fence Black) | **Wall blocker** | `Wall=yes` only; no gate mechanic |

---

## 7. Negative Finding — No Enemy-Blocked Gate Predicate in Can_Enter_Cell

The investigation found **no dedicated gate-owner check** in `UnitClass::Can_Enter_Cell`.
There is no code of the form `if (building.IsGate && !IsAllied(building.Owner)) return 7`.
Instead:

1. **Enemy units cannot open the gate** — `ToggleGate` checks the requesting unit's
   house against the gate's house via vtable `+0x3ac` (`IsAllied` variant); enemies are
   refused and the gate stays closed.
2. **Closed gate = normal building blocker** — While closed, the gate is an ordinary
   building occupying its cell; Can_Enter_Cell returns the same block codes as any
   other non-crushable building.
3. **Open gate = no occupation** — When opened (mission 0x13), the gate no longer
   blocks cell entry; all units, including enemies, can walk through an open gate.

This means: **a gate forced open by a trigger/script is passable by both sides**. The
ally-only guarantee only holds when the gate is controlled normally by the trigger-open /
approach logic.

---

## 8. Struct Offsets Summary

`BuildingTypeClass` offsets verified from `BuildingTypeClass_ReadINI_Water @ 0x45FE50`:

| Offset | Size | Field | INI Key |
|---|---|---|---|
| `+0x16a9` | byte | UnitRepair | `UnitRepair=` |
| `+0x16b6` | byte | BridgeRepairHut | `BridgeRepairHut=` |
| `+0x16b7` | byte | DamagedDoor | (art INI, `DamagedDoor=`) |
| `+0x16be` | byte | LaserFencePost | `LaserFencePost=` |
| `+0x16bf` | byte | LaserFence | `LaserFence=` |
| `+0x16c0` | byte | FirestormWall | `FirestormWall=` |
| `+0x16f8` | int  | GateStages | `GateStages=` |
| `+0xe28` | double | GateCloseDelay | `GateCloseDelay=` |

`BuildingClass` instance offsets referenced in TogglePowerOrGate:

| Offset | Field | Meaning |
|---|---|---|
| `+0x6df` | field_0x6df | C4 attached / gate state flag (dual-use, see C4 report) |
| `+0x6e9` | field_0x6e9 | IsGate instance flag (non-zero if this building is a gate) |

---

## 9. Key Function Addresses

| Address | Function | Role |
|---|---|---|
| `0x443B90` | `BuildingClass__ToggleGate` | Core gate open/close logic; checks alliance |
| `0x447110` | `BuildingClass__TogglePowerOrGate` | Dispatches gate by state param (-1/0/1) |
| `0x45FE50` | `BuildingTypeClass_ReadINI_Water` | Parses GateStages, GateCloseDelay, LaserFence, LaserFencePost |
| `0x43EF90` | `BuildingClass__GetCurrentFrame` | Checks LaserFence (+0x16bf) for special frame |
| `0x73F0A0` | `UnitClass__Can_Enter_Cell` | Passability; no gate-specific predicate; gate closed = normal block |
| `0x66D530` | `RulesClass__ReadGeneral` | Reads GDIGateOne/Two, NodGateOne/Two |
| `0x672AE0` | `FUN_00672ae0` (ReadGeneral cont.) | Reads NSGates=, EWGates= (AI lists) |

---

## 10. Rust Port Implications

1. **No `Gate=` bool field in binary string table.** Do not add a `gate: bool` field to
   `BuildingTypeClass` based on the `Gate=yes` INI value — the binary does not parse it
   that way. Use `gate_stages > 0` (i.e., `BuildingTypeClass+0x16f8 != 0`) as the
   gate-is-a-gate predicate, or trace the parent `TechnoTypeClass__ReadINI` for the
   exact bool offset.

2. **GAGATE_A is map-placed only** (`TechLevel=-1`). The Rust port does not need to
   support building gates from the sidebar in skirmish.

3. **Passability is mission-state driven**, not a flag check: closed gate occupies
   the cell normally; open gate does not. No special `is_gate` branch needed in
   `can_enter_cell` beyond normal building occupancy.

4. **Enemy units never open the gate** via `ToggleGate`; but an open gate is traversable
   by all units (no owner check at the passability layer).

5. **LaserFence visual** is parsed but no stock building activates it. Safe to defer.

---

## 11. Confidence Summary

| Claim | Confidence | Source |
|---|---|---|
| GAGATE_A is the only Gate=yes building in rulesmd.ini | HIGH | INI grep + confirmed section |
| Gate= INI key not in binary string table | HIGH | Ghidra search_strings "Gate" returns no bare match |
| GateStages at `BuildingTypeClass+0x16f8` | HIGH | ReadINI decompile, s_GateStages_0081a6f4 |
| GateCloseDelay at `BuildingTypeClass+0xe28` (double) | HIGH | ReadINI decompile |
| LaserFencePost at `+0x16be`, LaserFence at `+0x16bf` | HIGH | ReadINI decompile |
| Mission 0x13 = gate-open state | HIGH (by context) | ToggleGate comparison `param_1[0x2b] == 0x13`; `TogglePowerOrGate` SetMission(0x13) |
| No dedicated gate-owner check in Can_Enter_Cell | HIGH | Full decompile of UnitClass::Can_Enter_Cell at 0x73F0A0 — no gate branch found |
| Gate is active in YR (not TS-only) | HIGH | GAGATE_A in both rules.ini and rulesmd.ini; ToggleGate and TogglePowerOrGate in vtable with no SpecialFlags gate |
| NSGates/EWGates are AI-only, not passability | HIGH | FUN_00672ae0 stores into RulesClass AI lists; not referenced in Can_Enter_Cell |

---

## Sources

- `ini/rulesmd.ini` lines 17186–17211 ([GAGATE_A]), 374–379 (GDIGate/NodGate), 3078–3079 (NSGates/EWGates), 675–676 (GateUp/GateDown sounds), 16406–16430 ([CAFNCB])
- `ini/rules.ini` lines 9394–9419 ([GAGATE_A])
- `gamemd.exe` Ghidra MCP — live decompile of all addresses above
- `BuildingTypeClass_ReadINI_Water @ 0x45FE50` — verified all field offsets
- `BuildingClass__ToggleGate @ 0x443B90` — gate owner-check and mission-state logic
- `BuildingClass__TogglePowerOrGate @ 0x447110` — gate state dispatcher
- `BuildingClass__GetCurrentFrame @ 0x43EF90` — LaserFence frame branch at `+0x16bf`
- `UnitClass::Can_Enter_Cell @ 0x73F0A0` — confirmed absence of gate-specific predicate
- `RulesClass__ReadGeneral @ 0x66D530` — GDIGateOne/Two, NodGateOne/Two strings
- `FUN_00672ae0 @ 0x672AE0` — NSGates, EWGates strings
