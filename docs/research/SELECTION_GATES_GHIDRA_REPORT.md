# Selection Gates (CanBeSelected / CanBeSelectedNow) — Ghidra Research Report

**Primary addresses:**
- `ObjectClass::CanBeSelected` (vtable +0x138) — `0x005F6C30` (static "can this type be selected at all")
- `TechnoClass::CanBeSelectedNow` (vtable +0x13C) — `0x006FC030` (dynamic "is it selectable right now")
- `BuildingClass::CanBeSelectedNow` (vtable +0x13C override) — `0x00459C00` (1×1+UndeploysInto only)

**Overall confidence:** HIGH for the gate chain and offsets. MEDIUM for who calls `+0x13C` — the dynamic slot is inherited everywhere but the click/bandbox/T-key paths all use `+0x138` instead. `+0x13C` is reachable but I have not conclusively traced its primary caller (see §7).

**Active in YR:** Yes. All gates fire in a standard YR skirmish.

---

## 1. Overview

Two selectability predicates exist in the vtable:

- **Static (`+0x138`, "CanBeSelected")** — "can this kind of object ever be selected",
  based on the INI `Selectable=` flag + player discovery. Called by every path that
  adds an object to `g_CurrentObjects` (click, bandbox, T-key, team recall, cursor).

- **Dynamic (`+0x13C`, "CanBeSelectedNow")** — "is this specific instance clickable at
  this instant", based on per-unit state flags (enslaved, mid-boarding, bunkered,
  docking-as-harvester). Present on every TechnoClass vtable but not wired into the
  main selection pipeline — see §7.

Both predicates exist; the Rust reimplementation should fold the dynamic gates into the
click path even though the original engine structure is slightly indirect, because the
visible *behavior* (a docking harvester can't be click-selected, for example) is
controlled by these flags.

---

## 2. Key Offsets

### Fields read by CanBeSelectedNow (`TechnoClass`)

| Offset | Size | Field | Non-zero = | Confidence source |
|---|---|---|---|---|
| `+0x1C8` | byte | **IsDeploying** | mid-deploy animation (MCV→ConYard, Prism undeploy, etc.) | Set by `TechnoClass::OnDeployBegin` (0x0070FC90); cleared by `OnUndeployComplete` (0x0070FBE0). Previous doc name "IsEnteringTransport" was **wrong**. |
| `+0x2DC` | dword | **SlaveOwner** (`TechnoClass*`) | enslaved by a Slave Miner | `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` (HIGH) |
| `+0x2E4` | dword | **BunkerLinkedBuilding** (`BuildingClass*`) | garrisoned in bunker | `BUNKER_SYSTEM_GHIDRA_REPORT.md` (HIGH) |
| `+0x418` | byte | **IsDockedViaRadio** | has an active radio-dock link (harvester-at-refinery is one case; docker-builder etc. also trigger it) | Set by `BuildingClass::ChangeOwner` + `TechnoClass::Receive_Radio`; cleared by `EventClass::Execute` + `Receive_Radio`. Previous doc name "IsHarvester" was too narrow. |

### Fields read by ObjectClass::CanBeSelected

| Offset | Size | Field | Role |
|---|---|---|---|
| `TypeClass+0x230` | byte | **Selectable** | `Selectable=` INI key on TechnoTypeClass |
| (via `vtable+0xC8`) | — | `Discovered_By(HouseClass*)` | has the player seen this object? |
| (via `vtable+0xD0`) | — | `GetDisplayOwner(bool)` | returns display owner; null = not selectable |

### Adjacent lifecycle flags that gate selection through other paths

| Offset | Size | Field | Where checked |
|---|---|---|---|
| `+0x81` | byte | **InLimbo** | `ObjectClass::Select` early-exit (SELECTION_SYSTEM report) |
| `+0x83` | byte | **IsSelected** | `ObjectClass::Select` duplicate check |
| `+0x41b` | byte | **IsControlledByHuman** | `TechnoClass::Select` gate — AI-unit lock |
| `+0x220` | dword | **CloakState** (0=off, 1=cloaking, 2=cloaked, 3=uncloaking) | rendering visible-objects list — cloaked enemy never reaches selection |
| `+0x271` | byte | **IsBeingWarped** | chrono-warp animation (likely sets `+0x81` during warp — **unverified**, see §7) |

---

## 3. Core Logic

### 3.1 `ObjectClass::CanBeSelected` (`+0x138`, address `0x005F6C30`)

```pseudocode
bool ObjectClass::CanBeSelected(this):
    if this.Discovered_By(g_PlayerPtr):            // vtable+0xC8
        if this.GetDisplayOwner(1) == null:        // vtable+0xD0
            return false
    return this.GetTypeClass().Selectable          // TypeClass+0x230
```

**Interpretation:** If the player has ever discovered this object, it MUST have a non-null
"display owner" to remain selectable (this catches disguised spies whose disguise has
been revealed — the disguise house pointer is cleared, so the unit becomes unselectable
to the enemy).

If the player has never seen the object, the static INI `Selectable=` flag is the sole
answer. (Note: in practice you can't click an undiscovered object because shroud
hides it — but the predicate itself doesn't enforce shroud.)

### 3.2 `TechnoClass::CanBeSelectedNow` (`+0x13C`, address `0x006FC030`)

```pseudocode
bool TechnoClass::CanBeSelectedNow(this):
    if this.SlaveOwner != 0:              return false    // +0x2DC
    if this.IsEnteringTransport != 0:     return false    // +0x1C8
    if this.BunkerLinkedBuilding != 0:    return false    // +0x2E4
    if this.IsHarvester != 0:                              // +0x418
        coord = this.GetCoords()                           // vtable+0x1BC
        if Map.Lookup_Building_In_Cell(coord) != null:
            return false                                   // harvester docked in refinery
    return this.CanBeSelected()                            // vtable+0x138 (static fallback)
```

**Interpretation — why each gate exists:**
- **SlaveOwner** — Slave Miner (Yuri) slaves are owned by the miner; clicking a slave
  selects the miner instead (or rather, the slave never becomes a selection target).
- **IsEnteringTransport** — during the last few frames of the "enter IFV / Flak Track /
  transport" animation, the unit is mid-merge into the transport. Same flag also blocks
  firing (`FIRE_ILLEGAL`).
- **BunkerLinkedBuilding** — a GI or similar infantry inside a bunker (like the desolator
  bunker or Paradrop bunker) is tagged with a back-reference to the bunker. The infantry
  is visually absent and can't be clicked; you click the bunker to interact.
- **IsHarvester + building-in-cell** — when a harvester drives onto a refinery for
  unloading, the cell it occupies hosts the refinery building. Rather than let the
  player click the harvester through the refinery, the gate redirects clicks to the
  refinery. Released once the harvester leaves the cell.

### 3.3 `BuildingClass::CanBeSelectedNow` override (`+0x13C`, address `0x00459C00`)

```pseudocode
bool BuildingClass::CanBeSelectedNow(this):
    if !BuildingTypeClass.Is1x1WithUndeploy(this):
        return false                                       // 99% of buildings: never
    return TechnoClass::CanBeSelectedNow(this)             // FUN_006FC030
```

**Interpretation:** All standard buildings (factories, defences, tech) return **false**
from CanBeSelectedNow. Only the *deployed Construction Yard* — which has a 1×1 footprint
(via `UndeploysInto=MCV`) — passes through to the TechnoClass check. This is the same
special case used by the bandbox filter (documented in `BANDBOX_SELECTION_GHIDRA_REPORT`),
suggesting bandbox-style multi-select is its primary consumer.

### 3.4 Control-flow ordering inside CanBeSelectedNow

From the raw assembly (`0x006FC030`), each gate is a single TEST/JZ pair with an
immediate `XOR AL,AL; RET` on fail. There is no short-circuit ambiguity and no side
effect (no reads or writes beyond the field tests and one vtable call for coords).
The last branch (harvester on building) is the only one that calls another function
(`Look_up_building_in_cell` at `0x0047C520`) before deciding.

---

## 4. INI Keys

| Key | Section | Default | Effect on selection |
|---|---|---|---|
| `Selectable` | `[UnitType] / [BuildingType]` | `yes` | TechnoTypeClass+0x230 byte; read directly by `CanBeSelected` |
| `IsSelectableCombatant` | any combat TypeClass | `no` | Separate "select combatants" hotkey; not read by CanBeSelectedNow |
| `Cloakable`, `Underwater` | units | `no` | Gate CloakState / rendering; cloaked enemies never enter the visible-objects list and so never reach the selection path |

No INI key directly toggles the dynamic gates — they are all driven by engine-internal
state machines (transport boarding, slave enslavement, bunker docking, harvester docking).

---

## 5. Integration Points

### 5.1 Where the **static** gate (`+0x138`) is called

All confirmed via decompilation in the SELECTION_SYSTEM investigation plus this one:

- `ObjectClass::Select` (`0x005F4520`) — early-exit gate before list insertion.
- Bandbox callback `FUN_006DA5C0` — per-object filter during rectangle resolution.
- `TechnoClass::What_Action_OnObject` (`0x006FFEC0`) — cursor decision ("show select cursor?").
- Several `WhatAction` overrides on `UnitClass`, `InfantryClass`, `AircraftClass`.

So every selection-action path that the player can trigger calls `+0x138` somewhere.

### 5.2 Where the **dynamic** gate (`+0x13C`) is called

**Unknown in the click/bandbox/T-key pipeline.** I traced those flows end-to-end and
none of them call `+0x13C` — they all use the static `+0x138` gate and delegate the
transport/slave/bunker/harvester tests to implicit state (e.g. harvesters "dock" by
changing mission and position, which removes them from the visible-objects array).

The only caller I found is `BuildingClass::CanBeSelectedNow` (`0x00459C00`) invoking
`TechnoClass::CanBeSelectedNow` via the fallback path when the building is a 1×1-MCV.

The function body IS correct; it just isn't invoked by the paths I traced. Candidates
for who calls it (not verified):
- AI target validation (not investigated).
- Scripting / trigger actions (not investigated).
- Radar-click selection paths (not investigated).

### 5.3 Visibility → Selection pipeline (what actually filters clicks)

The real click-selectability story is closer to:

```
Tactical::Draw        — populates g_VisibleObjects (skips cloaked/fogged enemies, InLimbo,
                         destroyed, undiscovered, etc.)
click/bandbox         — hit-test against g_VisibleObjects only
per-object gates      — vtable+0x138 (static), +0x81 (InLimbo), +0x83 (dup), +0x41b
                         (IsControlledByHuman), g_IsMapEditor
Select                — insert into g_CurrentObjects
```

So cloaked enemies, warping units, in-transport units, and dead units are filtered at
the render/visible-objects stage rather than at the Select gate. The dynamic `+0x13C`
gate is redundant for click input because those states already remove the object from
the visible set.

---

## 6. Current Rust Implementation Status

From the scan in [app_entity_pick.rs](../ra2-rust-game/src/app_entity_pick.rs):

| Gate | Original behaviour | Rust current |
|---|---|---|
| `Selectable` static INI flag | `TypeClass+0x230` read by CanBeSelected | Not read from rules at click-time |
| Owner / fog discovery | `Discovered_By(g_PlayerPtr)` | Fog + ownership check in `is_selectable_entity` (lines 456-475) |
| InLimbo (+0x81) | Blocks all selection paths | `passenger_role == Inside{..}` — **not gated at click** |
| IsSelected (+0x83) dup check | Blocks re-add | Per-entity `selected` flag; no duplicate issue in Vec-of-ids snapshot model |
| IsControlledByHuman (+0x41b) | Gates AI-unit selection | Implicit via owner ownership check |
| SlaveOwner (+0x2DC) | Block slaves | Not implemented — slave system absent |
| IsEnteringTransport (+0x1C8) | Block mid-boarding | `PassengerRole::Boarding{..}` exists but not gated at click |
| BunkerLinkedBuilding (+0x2E4) | Block garrisoned | `garrison_slot` exists but not gated at click |
| IsHarvester + docked (+0x418) | Block docked harvester | Not gated |
| CloakState (+0x220) | Enemies filtered from visible list | Cloak system not implemented |
| IsBeingWarped (+0x271) | Likely InLimbo during warp | `teleport_state` exists but not gated |
| BuildingClass override: non-1×1 buildings unselectable via bandbox | Excludes all structures from bandbox | Rust excludes all structures from bandbox (no 1×1+UndeploysInto exception for deployed CY — small deviation) |
| `dying` state | Dead units have `+0x81` set; not selectable | `dying: bool` exists but **not checked** at click-time |

### Concrete gaps worth closing

1. **`dying` check** — on-screen dead-but-animating entities are selectable in Rust today.
2. **`PassengerRole::Inside` and `Boarding`** — passengers should be unclickable; boarders
   should be unclickable during the final merge frames.
3. **Garrisoned infantry** — should not be click-selectable.
4. **Harvester docked at refinery** — click should pass through to the refinery.
5. **Deployed CY exception in bandbox** — faithful original allows bandbox-select of a
   deployed MCV because it's 1×1+UndeploysInto.

---

## 7. Open Questions — all resolved

1. **Primary caller of `+0x13C`** — **resolved**. Binary search for the pattern
   `FF ?? 3C 01 00 00` (CALL [reg+0x13C]) returned 8 call sites:

   | Address | Site |
   |---|---|
   | `0x004AC2D9` | `Selection::PickCallback` — click AND bandbox callback |
   | `0x004AA318` / `0x004AA3E8` | MouseClass cursor/hover resolve |
   | `0x007323E1` / `0x007325E5` / `0x00732C57` | TypeSelect (T key) predicate |
   | `0x00733AD2` / `0x00733CAC` | keyboard nav / health-select path |

   The `Selection::PickCallback` path is the important one: both mouse-click-on-unit
   and bandbox-release invoke this callback, and it runs the full gate chain
   (+0x138, +0x13C, +0x81, +0x14C). So **every user-driven selection in YR passes
   through the dynamic gate**. My earlier claim that "the main pipeline uses +0x138
   only" was wrong — the callback form uses both. The inline path in
   `Tactical::IterateObjectsInRect` (`0x006DA5C0`) is only reached when called WITHOUT
   a callback, which doesn't happen for the normal bandbox flow.

2. **Does `IsBeingWarped` set `InLimbo`?** — **resolved: no**.
   `TeleportLocomotion::InitiateWarp` sets `+0x270 IsWarpingOut` and `+0x271
   IsBeingWarped` but does **not** touch `+0x81 InLimbo`. Instead, the dynamic
   gate chain reads `+0x270` via `vtable+0x1D4` (`TechnoClass::IsWarpingOut` at
   `0x0070C5B0`), which is called from `ObjectClass::Select` to reject
   mid-warp selection. So warping units are gate-blocked from NEW selection
   but keep their existing `IsSelected` flag through the warp gap — which
   matches the visible gameplay of Chrono Legionnaires staying selected
   through their warp.

3. **`vtable+0xC8 Discovered_By`** — `UnitClass::Discovered_By` at `0x00746750`
   is labelled. Other classes inherit from a base that checks owner alliance
   and cell fog state; not traced further because the logic is identical for
   selection purposes (has the player ever seen this object?).

4. **`+0x418` write path** — **resolved**. The field was previously called
   `IsHarvester` which is **too narrow**. Byte-pattern search shows:
   - **Set to 1**: `BuildingClass::ChangeOwner` (two writes in its radio-link
     establishment loop, `0x004492B7` + `0x004492C3`) and `TechnoClass::Receive_Radio`
     (`0x006F4B72`).
   - **Cleared to 0**: `EventClass::Execute` (`0x004C7342`) and
     `TechnoClass::Receive_Radio` (`0x006F4BA6`, the OUT-message branch).

   Correct name: **`IsDockedViaRadio`** (has an active radio-dock link). Harvester
   at refinery is one instance of this; any other radio-docked unit (transport
   cargo exchange, docker-builder, etc.) sets it too. The cell-building check in
   CanBeSelectedNow means a click-through happens only when the docked partner
   is specifically a building at the unit's current cell.

---

## Sources

**Ghidra addresses decompiled (this investigation):**

| Address | Name | Purpose |
|---|---|---|
| `0x006FC030` | `TechnoClass::CanBeSelectedNow` (NEW LABEL) | Core dynamic gate |
| `0x00459C00` | `BuildingClass::CanBeSelectedNow` (NEW LABEL) | 1×1-MCV exception |
| `0x005F6C30` | `ObjectClass::CanBeSelected` | Static gate |
| `0x00746750` | `UnitClass::Discovered_By` | vtable+0xC8 for units |
| `0x007465F0` | `UnitClass::GetDisplayOwner` | vtable+0xD0 for units |
| `0x005F6690` | `ObjectClass::IsDead` | Helper (reads +0x90) |
| `0x006FFEC0` | `TechnoClass::What_Action_OnObject` | Cursor decision (calls +0x138) |
| `0x006DA5C0` | Bandbox callback | Calls +0x138, not +0x13C |
| `0x004AC380` | `DisplayClass::BandBox_MouseMove` | Drag-in-progress branch |
| `0x0070C5C0` | `TechnoClass::IsBeingWarped` | Reads +0x271 |
| `0x00732D00` | (returns `DAT_00B0FE65`, "Shift held") | Helper |

**Docs referenced:**
- `SELECTION_SYSTEM_GHIDRA_REPORT.md` (SelectClass/CurrentObjects — predecessor)
- `BANDBOX_SELECTION_GHIDRA_REPORT.md`
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` (+0x2DC SlaveOwner, +0x41B IsControlledByHuman)
- `BUNKER_SYSTEM_GHIDRA_REPORT.md` (+0x2E4 BunkerLinkedBuilding)
- `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md` (+0x1C8 IsEnteringTransport)
- `WAR_MINER_REFERENCE.md`, `DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md` (+0x418 IsHarvester)
- `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`
- `IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md`
- `PARASITE_CLASS_GHIDRA_REPORT.md`
- `CHRONO_WARP_VISUAL_RENDERING.md`
- `MCV_DEPLOY_GHIDRA_REPORT.md`

**INI files:** `rulesmd.ini` / `rules.ini` searched — only `Selectable=`,
`IsSelectableCombatant=`, `Cloakable=`, `Underwater=` are relevant. None touch the
dynamic gate directly.
