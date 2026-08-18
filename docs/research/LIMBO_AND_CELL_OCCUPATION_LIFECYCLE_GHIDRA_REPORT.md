# Limbo & Cell Occupation Lifecycle — Ghidra Research Report

**Address(es):** see §10 function table
**Confidence:** HIGH on all findings (verified via live Ghidra MCP decompilation of `gamemd.exe`)
**Active in YR:** Yes (core engine path, always live)
**Date:** 2026-04-24

**Supersedes / extends:**
- `OBJECTCLASS_DRAW_LIMBO_CELLLIST.md` — thorough on Draw and basic Reveal/Conceal; this
  doc fills in the Mark dispatcher, the non-movement state transitions
  (production / transport load/unload / chronoshift / death), and the bridge-height
  constant reconciliation.
- `CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md` — thorough on vehicle movement; this doc
  adds the infantry/anim parallels and the subclass-override pattern.
- `INFANTRY_SUBCELL_POSITIONING.md` — authoritative on subcell placement.

Read all three first if you need struct field layouts or movement-path detail —
this doc references them rather than duplicating.

---

## 1. Overview

Every visible object in gamemd.exe exists in exactly one of two states with respect to
the map:

- **On-map:** `InLimbo=false`, registered in a cell's occupier linked list, in a
  display layer, occupation bits set, visible to scans and targeting.
- **In limbo:** `InLimbo=true`, *not* in any cell list, *not* in any display layer,
  occupation bits cleared. The object is "suspended" — fully allocated, owned by its
  house, tickable by the passenger/production systems, but invisible to the map.

Both the happy-path movement pipeline and every less-obvious transition (production,
transport load/unload, chronoshift, death, cloak over multi-cell buildings) funnel
through the same narrow set of primitive operations. Getting those primitives right
is the foundation for parity across dozens of seemingly-unrelated systems.

This report consolidates the primitives, their invariants, and the transition
lifecycles that use them.

---

## 2. The Primitive Operations

There are four primitive operations on the "is an object on the map?" question.
Every higher-level transition (reveal, conceal, unlimbo, production exit, passenger
load, chronoshift, death) is a composition of these.

### 2.1 `ObjectClass::Mark(mode)` — vtable+0x124 @ `0x005F5850`

**Base class dispatcher.** Manages only the per-object state (`IsMarked` flag and radar
dirty). Subclasses (TerrainClass, TechnoClass) override this slot to *additionally*
add/remove from cell linked lists and foundation cells via
`EnterCell_AddToMultiCells` / `ExitCell_RemoveFromMultiCells`.

```
ObjectClass::Mark(this, mode):
    if InLimbo: return 0                             // refuse to act on limboed obj

    if mode == 2 (MARK_CHANGE):                       // "redraw request"
        if !NeedsRedraw && IsMarked:
            MarkNeedsRedraw()  // vtable+0x134
        return 1

    // modes 0/1/3 — check building-display-suppression short-circuit
    if !(WhatAmI()==6 && TypeClass+0xE58 != 0 && mode==0):
        // Normal path: notify radar/display
        obj = Filter_AbstractType_InMap()            // gets current display/radar
        if obj:
            obj->vtable+0x2C0()                      // RadarClass notification
            obj->vtable+0x38()
            this->vtable+0x1B8(&mode)                // GetCoords / recompute

    if (mode == 1 || mode == 3) && !IsMarked:        // MARK_PUT / MARK_UP
        IsMarked = 1
        MarkNeedsRedraw()
        return 1

    if mode == 0 && IsMarked:                         // MARK_REMOVE
        IsMarked = 0
        return 1

    return 0   // idempotent: already in requested state
```

**Mode enum (inferred from branches):**

| Value | Name | Effect |
|-------|------|--------|
| 0 | `MARK_REMOVE` | Clear `IsMarked`, radar notify (except suppressed buildings) |
| 1 | `MARK_PUT` | Set `IsMarked`, radar notify, mark NeedsRedraw |
| 2 | `MARK_CHANGE` | Just mark NeedsRedraw if already marked |
| 3 | `MARK_UP` | Same effect as MARK_PUT (redundant alias — likely TS holdover) |

**Invariants (HIGH confidence):**
- Limboed objects *cannot* be marked. The first thing Mark does is refuse if `InLimbo=1`.
  This is the tripwire that prevents a concealed object from accidentally re-registering.
- The base implementation does NOT touch cell linked lists. It only sets flags.
- The building-suppression condition `(WhatAmI==6 && typeclass+0xE58!=0 && mode==0)`
  skips the radar notify for certain buildings on unmark. `+0xE58` on BuildingTypeClass
  is most likely `InvisibleInGame` or a similar TS-holdover suppress flag. It is
  defaulted `false` for standard YR buildings, so this branch rarely fires.

### 2.2 `ObjectClass::Mark_Occupation` / `Clear_Occupation` — vtable+0xF0/0xF4

Sets/clears the coarse **occupation bits** on a single cell (`0x20` for vehicles, subcell
bits for infantry, unused for buildings which use `Mark_Put`). Overridden per-class.

| Class | Mark_Occupation | Clear_Occupation | Bit Set |
|-------|-----------------|------------------|---------|
| ObjectClass (base) | `0x7441B0` | `0x744210` | `0x20` (vehicle) |
| UnitClass (inherits base) | same | same | `0x20` |
| InfantryClass | `0x5217C0` | `0x521850` | `0x04/0x08/0x10` (subcell) |
| AnimClass | `0x426270` | `0x426300` | `0x04/0x08/0x10` (as if infantry) |

**Bridge-layer selection** (common to all implementations):
```
if object.Z >= ground_height + BRIDGE_Z_THRESHOLD && cell.Flags & 0x100:
    # bridge layer
    cell.AltOccupationFlags |= bit        // +0x128
else:
    cell.OccupationFlags |= bit            // +0x124
```

**Critical asymmetry (HIGH confidence):** `Mark_Occupation` requires BOTH the height AND
the cell's bridge flag to go to the bridge layer. `Clear_Occupation` skips the bridge
flag check and uses height alone. This is intentional — if a bridge is destroyed under
a unit, the bridge flag is cleared but the unit's bridge-level occupation bit must still
be clearable. A unit left on a destroyed-bridge cell would otherwise be a leaked
occupation bit forever.

**Infantry side-effect:** `InfantryClass::MarkCellOccupancy` additionally writes the
owner house ID to `cell+0x54` (ground) or `cell+0x58` (bridge). This is a fast-path
cache so "who owns this cell?" queries don't need to walk the cell list.

### 2.3 `ObjectClass::Mark_Put` / `Mark_Remove` — vtable+0xEC/? @ `0x5F60A0` / `0x5F6120`

Sets/clears **bit 0x40** on the cell — the "building/structure is present" flag. Called
during building placement and removal (including `Reveal`/`Conceal` on any object per
existing doc §3.4, but dominantly by buildings).

```
Mark_Put(this):
    cell = Get_Cell_At(this.Coord)
    if ground_height + BRIDGE_Z_THRESHOLD <= this.Z && cell.Flags & 0x100:
        cell.AltOccupationFlags |= 0x40
    else:
        cell.OccupationFlags |= 0x40

Mark_Remove(this):
    # symmetric, clear bit 0x40 from the same plane selected by BOTH
    # the inclusive height threshold and cell.Flags & 0x100.
```

Correction verified 2026-08-13 via live `decompile_function 0x005F60A0` and
`decompile_function 0x005F6120`: the height-only clear asymmetry applies to the
Unit/Infantry `0x20` occupation paths described in §2.2, not to ObjectClass's
`0x40` Put/Remove pair. Both ObjectClass functions require the structural bridge
flag before selecting `Cell+0x128`; otherwise both operate on `Cell+0x124`.

### 2.4 `CellClass::AddContent` / `RemoveContent` — `0x47E8A0` / `0x47EA90`

Manages the **cell's singly-linked occupier list** via `object.NextObject` (+0x30).
Full decomp in existing docs; key invariants:

- Head pointer is either `cell.FirstObject` (+0xE4) or `cell.AltObject` (+0xE8) depending on bridge layer.
- **Buildings (WhatAmI==6) are appended to the TAIL**; everything else is prepended to the HEAD.
- AddContent internally calls `Mark_Occupation` after linking (for non-infantry occupiers
  where `IsOccupier()` returns true). RemoveContent calls `Clear_Occupation` symmetrically.
- AddContent triggers `Discovered_By(g_PlayerPtr)` on the newly-added object if the cell
  is shrouded OR fogged to the local player AND `g_GameMode != 0` (not campaign). This is
  how newly-revealed units mark themselves as "seen" for multiplayer shroud.

---

## 3. Composite Operations (the High-Level Transitions)

These are the entry points that game code actually calls. Each is a sequence of
primitives (§2). Understanding the sequence matters for parity because the *order*
determines what other systems observe mid-transition.

### 3.1 `ObjectClass::Reveal` — vtable+0xD8 @ `0x5F4EC0` (enter map from limbo)

Sequence (HIGH confidence, full decomp in existing doc §2.2):

```
1. Reject zero coords, game-inactive, or non-passable cell (CanEnterCell check)
2. InLimbo = 0
3. NeedsRedraw = 0
4. Apply TypeClass coordinate transform (TypeClass vtable+0x6C)
5. Set_Raw_Coords(adjusted)                   // vtable+0x1B4
6. Mark(MARK_PUT=1)                           // sets IsMarked, calls AddContent via subclass
7. IF success AND IsAlive:
     Submit_Object(this)                     // DisplayClass linked list insert
     If TypeClass.AlphaImage: AlphaShape ctor
     If TypeClass.HasLineTrail: LineTrail ctor
8. IF Mark failed: InLimbo = 1 (revert)
```

**Edge case:** If `Mark(MARK_PUT)` fails (e.g., cell became occupied between the
CanEnterCell check and the Mark), Reveal reverts `InLimbo` back to 1 but leaves
the coordinates set. This is a narrow time-of-check-to-time-of-use window — in
gamemd.exe it's protected by the fact that `Reveal` never runs from concurrent
threads (single-threaded sim tick), so this is only a defensive guard.

### 3.2 `ObjectClass::Conceal` — vtable+0xD4 @ `0x5F4D30` (leave map → limbo)

Sequence (HIGH confidence):

```
1. Deselect                                   // vtable+0x150
2. vtable+0xDC(1)                             // Mark/unmark cell occupation (NOT MarkNeedsRedraw)
3. Mark(MARK_REMOVE=0) / DoCloak(0)           // clears IsMarked, RemoveContent via subclass
                                              // on TechnoClass objects this calls DoCloak(0)
                                              // which calls ExitCell_RemoveFromMultiCells
4. DisplayClass::RemoveFromLayer              // remove from draw iteration
5. AnimClass::Detach                          // detach attached anims
6. VocHandle::Stop                            // stop attached sound (corrected 2026-05-29: was
                                              // missing from sequence; binary shows VocHandle__Stop
                                              // between Detach and alpha-shape check —
                                              // decompile_function 0x5F4D30 — OPERATOR_OR_ORDER_DRIFT)
7. Alpha shape / TypeClass+0x234 vacate (if applicable)
8. DirtyScreenRect (if alpha image)
9. ClearDrawnState                            // vtable+0x11C
10. InLimbo = 1
11. NeedsRedraw = 0
```
(corrected 2026-05-29: step 2 was "MarkNeedsRedraw(true)" — binary at `0x5F4D30` shows `vtable+0xDC(1)`, which the existing Ghidra plate comment labels "Mark/unmark cell occupation", NOT MarkNeedsRedraw. MarkNeedsRedraw is at vtable+0x134. ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT — verified via decompile_function 0x5F4D30.)

**Order matters:** Deselect BEFORE the unmark. If Deselect happens after InLimbo=1, the
selection code would refuse to touch the object (it gates on !InLimbo) and the player
would be left with a stale selection pointer. Parity: any Rust equivalent must deselect
before flipping the limbo flag.

### 3.3 `TechnoClass::Unlimbo` — vtable+0x74 @ `0x6F6CA0`

The full "spawn onto the map" path used by factories, spawn lists, MCV deploy, and
most other object creations. Extends Reveal with TechnoClass-specific init:

```
1. ObjectClass::Reveal(coords)                           // §3.1
2. IsInPlayfield = MapClass::Is_Cell_In_Playfield(cell)  // +0x3D5
3. If !IsAlive: return true (limbo-revealed but dead — unusual)
4. UpdateVision(clear)                                    // vtable+0x488
5. MapClass::UpdateFogBorder(coords, 0, Sight+3, 0)       // shroud reveal radius
6. If TypeClass.GapGenerator: FUN_00439080 setup
7. HouseClass::Added_To_Game                               // house-scoped registry
8. FacingClass::UpdateFacing(body, turret)
9. BodyState = 1, TurretState = 0                          // +0x49C/4A0
10. If Infantry: IsLaying = true (temporarily for sensor init)
11. UpdateSensorArrays(1,1)
12. IsLaying = false
13. If CanDeployFire: ActivateDeployFire
14. FallSpeedRaw = Z / LevelHeight * 10                    // +0x108 (for airborne entry)
```

### 3.4 Production / Factory Exit — `BuildingClass::ExitObject_Main` @ `0x443C60`

This is the path for units exiting a factory (war factory, barracks, naval yard), MCV
deploy, and other "building produces unit" flows. The object being spawned was already
allocated and sitting in limbo during construction (`InLimbo=1`, invisible to map). When
construction finishes, `ExitObject_Main` runs and does the actual reveal.

**Common pattern across RTTI cases (UnitClass=2, InfantryClass=15, BuildingClass=6):**

```
1. Determine exit coordinates:
   - Rally point if set
   - Building doorway offset (from TypeClass+0xEC8/0xECC/0xED0 — UnloadOffset)
   - Facing-direction adjacent cell
   - Fallback: FootClass::Find_Nearby_Passable_Cell
2. Call vtable+0xD8 (Reveal) at exit coords    // §3.1 runs
3. For units: vtable+0x124(0) (DoCloak 0) → vtable+0x1B4 (Set_Coords) → vtable+0x124(1)
   (DoCloak 1) — standard "move into foundation cells" sequence
4. Assign initial mission:
   - UnitClass: Mission=Move if rally, else Guard
   - InfantryClass: Mission=Move to exit cell, then Guard
   - BuildingClass: no further mission (stationary)
5. If non-player-owned: Set_Ghost_Cell (AI navigation marker)
```

**Key invariant:** The unit does not exist as "on the map" during construction. From the
engine's perspective it is fully in limbo — not in any cell, not in any display layer,
not targetable, not visible. The factory's `FactoryClass` holds the pointer. When
`ExitObject_Main` succeeds, the unit transitions from construction-limbo straight to
on-map via `Reveal`.

If `Reveal` fails (all exit cells blocked), `ExitObject_Main` returns early. The unit
stays in limbo and the factory retries next opportunity (typical for war factory with
a crowded rally path).

### 3.5 Transport Load — `CargoClass::AddPassenger` @ `0x4733A0`

**Critical finding:** The passenger's `NextObject` pointer (+0x30) — the same field
used by cell occupier lists — is **repurposed as the transport's passenger chain**.

Sequence:

```
1. passenger->vtable+0xD4 (Conceal)          // §3.2 runs — clears cell registration,
                                              // InLimbo=1, NextObject now free
2. Splice passenger into transport's passenger list:
   CargoClass stores {count, head_ptr}       // two pointers
   passenger->NextObject = oldHead
   cargo->head = passenger (or appended based on IsTechno flag on next link)
3. Recount the chain and store back in cargo.count
```

**Why this matters:**
- A single `NextObject` pointer has *two meanings* depending on `InLimbo` state.
  If `InLimbo==0`: next in cell's occupier chain. If `InLimbo==1`: next passenger in transport.
- Any code that walks `NextObject` MUST know which list it's walking. Cell-list iteration
  is done through `cell.FirstObject`/`AltObject` as the head; passenger-list iteration is
  done through `transport.Cargo.head`.
- This is the kind of design that can produce bizarre bugs in a port if you use
  `NextObject` for only one purpose. A port that wants to avoid this dual use needs a
  separate `next_passenger` field.

**Side effect:** Because Conceal is called first, all of Conceal's effects apply:
passenger is deselected, removed from display layer, detached from anims, shroud around
it no longer updates (unit is no longer a vision source). If the passenger was selected,
the player's selection shrinks silently.

### 3.6 Transport Unload — `UnitClass::Mission_Unload` @ `0x740EF0`

The unload path is split across multiple mission states — `Mission_Unload` only drives
the truck-stops-and-ramp-drops choreography. The actual per-passenger *Reveal* happens
via `Unlimbo` calls from the ramp-state handler (case 7 mission sub-state). Each tick
one passenger is popped from the cargo list and `Unlimbo`'d at an adjacent free cell.

Reverse of AddPassenger:
```
1. Pop passenger from cargo.head (or walk to tail, depending on order policy)
2. passenger->NextObject = NULL (free the pointer for cell-list reuse)
3. passenger->Unlimbo(dropoff_coords, facing)   // §3.3 runs
   - InLimbo → 0
   - Added to cell list via Reveal → Mark(PUT) → subclass AddContent
   - Submit_Object
4. passenger->Assign_Mission(Move or Guard or Scatter)
```

**Ownership of NextObject:** While a passenger is concealed (in cargo), `NextObject`
points into the cargo chain. As soon as Unlimbo runs in step 3, `Mark(MARK_PUT)` calls
the subclass Mark override, which runs `CellClass::AddContent` — and AddContent
unconditionally writes `passenger->NextObject = oldCellHead`. Any stale passenger-chain
pointer is silently clobbered. This is intentional but narrow: if you Unlimbo without
first popping from cargo, the cargo chain beyond this passenger *breaks silently*.

### 3.7 Chronoshift — `ChronoSphere::WarpUnitsAtCell` @ `0x65EC30`

Chronoshift does NOT call Conceal/Reveal. Instead it installs a `TeleportLocomotion`
piggyback over the unit's existing locomotor and flags the unit as "being warped".

Sequence (HIGH confidence on mechanism):

```
For each unit at source cell:
    1. Compute dest via Find_Nearby_Passable_Cell (bounces off occupied)
    2. Create TeleportLocomotion via COM: CoCreateInstance(CLSID_TeleportLocomotion)
    3. Get the unit's existing locomotor IPiggyback interface
    4. Bind TeleportLoco to unit; attach old loco as piggyback
    5. unit+0xA2/A3/A4 = {piggyback, dest_x, dest_y}
    6. unit+0x271 = 1                            // BeingWarped flag
    7. unit+0xA1 (ChronoLockDuration) = Rules.ChronoReinfDelay (+0xBF0 on RulesClass)

After loop, for each warped unit:
    8. unit.DoCloak(0)                           // vtable+0x124 — removes from
                                                  //   source cell(s) via
                                                  //   ExitCell_RemoveFromMultiCells
    9. unit+0x280 = 3                             // PendingWarpPhase = 3
    10. unit.vtable+0x1EC()                      // ActivateDeployFire
```

**Key mechanism:** DoCloak(0) is the cell-removal primitive here — misleadingly named
because it ALSO handles non-cloak cell exit via `ExitCell_RemoveFromMultiCells` (see
§3.8). The unit is NOT Concealed (InLimbo stays 0); it simply exits the source cells.
The TeleportLoco then drives the state machine to advance phases — warp-in at dest
after ChronoReinfDelay ticks, re-entering cells via the locomotor's `Process`.

**Parity implication:** A unit mid-chronoshift is `InLimbo=0` but has NO cell
registration. It's in a third state — neither on-map nor in-limbo. Radius-based scans
(targeting, crush, scatter) that walk cell lists will not find it. Scans that iterate
the house's unit array WILL find it. This asymmetry is the source of many chronoshift
bugs in mods (e.g. units being "attackable" by scan-from-house but invisible to
scan-from-cell).

### 3.8 `TechnoClass::DoCloak(mode)` — vtable+0x124 @ `0x4D3780`

Decompiled in this pass — the existing doc's summary was slightly off. The actual code:

```
DoCloak(this, mode):
    if mode == 2: return 1                    // FORCE / no-op

    if !ProcessCloakAndNotify(mode):          // do the visual cloak/uncloak
        return 0                               // cloaking not allowed right now

    if GetMapLayer() == 2:                    // ONLY Ground layer
        GetCoords(&temp)
        if mode == 0:
            TechnoClass::ExitCell_RemoveFromMultiCells(&temp, this)
        elif mode == 1 or mode == 3:
            TechnoClass::EnterCell_AddToMultiCells(&temp, this)

    return 1
```

**`mode` semantics:**
- `0` = UNCLOAK / exit-cell
- `1` = CLOAK / enter-cell
- `2` = FORCE / no-op
- `3` = same as 1 (alias)

**Critical insight:** "DoCloak" is misnamed. It is **not only** for cloaking units —
it's the standard way to remove/add a unit to its foundation cells during *any*
cell transition (movement, chronoshift, facing change for multi-cell objects).
The non-cloaked path through `ProcessCloakAndNotify` is essentially a no-op that
returns true, so the cell-registration side runs regardless.

The **layer == 2 gate** means only ground-layer objects use multi-cell logic. Aircraft
(layer 3) and tunnel units (layer 0) never touch cells this way — their "occupation"
is entirely handled at the movement-layer abstraction.

### 3.9 Death / Destruction — `ObjectClass::UnInit` @ `0x5F65F0`

Sequence (HIGH confidence, labeled in Ghidra):

```
1. If AttachedBomb: BombClass::Defuse
2. If FootClass (AbstractFlags bit 0): FootClass::EMPPassengers(0)    // passengers get EMP'd
3. FUN_007258D0                              // global cleanup (animations? sounds?)
4. Conceal                                    // vtable+0xD4 — §3.2 runs
5. IsAlive = 0                                // +0x90
6. Append to PendingDeleteList @ 0x00B0F69C   // deferred destruction — freed at tick end
```

**Key sequencing:**
- Conceal fires BEFORE IsAlive=0. So during Conceal's body, IsAlive is still true.
  This matters because Conceal may trigger anim detachments / effects that gate on IsAlive.
- After UnInit returns, the object is in "dead limbo": `InLimbo=1`, `IsAlive=0`,
  `IsMarked=0`, queued for delete. It sits in this state for up to one tick until
  `ProcessPendingDelete` (end-of-tick) actually calls the destructor and frees memory.
- If anything reads a pointer to the dead object during that final tick, it will find
  a valid-looking object in dead-limbo state — invisible, unselectable, not in any
  cell, but not yet freed. Systems must gate on `IsAlive` not just `InLimbo`.

**EMPPassengers side effect:** A destroyed transport doesn't automatically kill its
passengers — it EMPs them. The cargo chain is preserved through UnInit (it's just
NextObject pointers within the concealed passengers). The pending-delete sweep later
frees the transport, and the EMP'd passengers remain in cargo-limbo *forever* unless
something else processes them. In practice the transport's own destructor disposes
the cargo chain, but the exact order is brittle.

---

## 4. The Three Faces of `NextObject` (+0x30)

The `ObjectClass.NextObject` pointer at +0x30 is reused across three distinct linked-list
structures depending on the object's state. This is an under-documented source of subtle
bugs.

| Object State | `NextObject` Points To | Head Pointer |
|--------------|------------------------|--------------|
| On-map, ground | next occupier in cell's ground list | `cell.FirstObject` @ +0xE4 |
| On-map, on-bridge | next occupier in cell's bridge list | `cell.AltObject` @ +0xE8 |
| In passenger cargo | next passenger in transport's cargo chain | `CargoClass.head` |
| In-limbo (free) | NULL or undefined | — |
| Pending-delete | NULL (cleared by Conceal) | `PendingDeleteList` uses separate array |

**Invariant:** At any point in time, an object's `NextObject` belongs to *exactly one*
list. Transitioning from one list to another requires nulling out the old pointer
before splicing into the new list — otherwise you create a cross-list loop.

`CellClass::AddContent` clobbers `NextObject = oldHead` unconditionally, so if you
AddContent an object that still has a stale cargo pointer, the cargo chain silently
truncates at that object. The standard call pattern avoids this by always Conceal-ing
(which calls RemoveContent, clearing NextObject) before transitioning, and
Unlimbo-ing (which Reveal → Mark(PUT) → AddContent) after.

**Rust port implication:** If you model limbo/cargo/cells with a single `next_in_list:
Option<u64>` field, you are taking on this same fragility. Separate fields
(`next_in_cell`, `next_in_cargo`) or a tagged enum remove the ambiguity at the cost of
8 extra bytes per entity.

---

## 5. Bridge-Height Constants — Reconciled

The three globals referenced in existing docs (`DAT_00B1D0AC`, `DAT_00A8F234`,
`DAT_0089A1B4`) are **three cached copies of the same value**, computed from the same
rules data via identical code.

**Derivation (identical at all three sites):**
```asm
MOV EAX, [BridgeHeight_raw]    ; 0x00B1D0B8 (also cached) — default 4 from [General]
LEA ECX, [EAX*4]                ; *= 4  (cell-level → sub-lepton units)
PUSH ECX
FILD [ESP]                      ; load as float
FADD [0x007E1738]               ; +0.5 rounding constant (verified: double 0.5)
CALL ftol                       ; round to int
MOV [DAT_XXX], EAX              ; store cached threshold
```

**Verified via `read_memory` on constant `0x007E1738`:** bytes `00 00 00 00 00 00 E0 3F`
= IEEE-754 double `0.5`. Confirmed rounding constant.

**Per-class caches (HIGH confidence):**

| Global | Writer Address | Used By |
|--------|---------------|---------|
| `DAT_00B1D0AC` | `0x00735310` (UnitClass init) | UnitClass, ObjectClass::Mark_Occupation/Clear_Occupation, ScenarioClass::Read_Units_Section |
| `DAT_00A8F234` | `0x005179D0` (InfantryClass init) | InfantryClass::MarkCellOccupancy/UnmarkCellOccupancy, InfantryClass::Load |
| `DAT_0089A1B4` | `0x00421E40` (AnimClass init) | AnimClass::AI, BounceAI, AnimClass::MarkCellOccupancy/ClearCellOccupancy |

**Why three?** Each class lives in a different compilation unit (`.obj` file). The
compiler inlined the derivation separately for each and stored the result in a
function-local static, visible to the linker as a distinct symbol. Functionally they
hold the same value at all times (all three are written during boot from the same
rules source). Values stay at 0 until initialized — hence `read_memory` shows zeros
when gamemd.exe is not running a game.

**Parity implication:** A single `bridge_z_threshold` constant in the Rust engine,
computed once from `[General] BridgeHeight`, suffices. There is no functional reason
to maintain three.

---

## 6. State Transition Diagram

```
                     Factory produces unit
                     (unit allocated, InLimbo=1)
                                │
                                ▼
    ┌────────────────── CONSTRUCTION LIMBO ──────────────────┐
    │  InLimbo=1, IsAlive=1, not in any cell, not drawn      │
    │  NextObject: FactoryClass chain (or NULL)              │
    └─────────────────────────────┬──────────────────────────┘
                                  │ ExitObject_Main (§3.4)
                                  │   → Reveal → Mark(PUT) → AddContent
                                  ▼
    ┌────────────────── ON-MAP (GROUND) ───────────────────────┐
    │  InLimbo=0, IsAlive=1, in cell.FirstObject list          │
    │  IsMarked=1, occupation bits set                          │
    │  NextObject: next occupier in same cell                  │
    └─────────┬─────────────┬──────────────┬──────────────────┘
              │             │              │
              │ move to new │ Mission_Enter │ ChronoSphere
              │ cell (drive)│ (transport)   │ (warp)
              │             │              │
              │ DoCloak(0)  │ AddPassenger  │ DoCloak(0)  (NOT Conceal!)
              │ Move        │ = Conceal +   │ BeingWarped=1
              │ DoCloak(1)  │   splice      │ TeleportLoco
              ▼             ▼               ▼
       [ON-MAP      ]  [CARGO LIMBO]  [WARP LIMBO      ]
       (new cell,     (InLimbo=1,    (InLimbo=0,
        same state)    chain via     but NO cell regist,
                       NextObject)   BeingWarped=1)
                             │              │
                      Unload │        Phase  │ complete
                             │  3 done (reinf delay)
                             ▼              ▼
                       [ON-MAP]       [ON-MAP at dest]
                             │              (DoCloak(1) adds
                             │               to dest cells)
                             │
                           UnInit (damage kills it)
                             │
                             ▼
    ┌────────────── DEAD LIMBO (one tick) ───────────────────┐
    │  InLimbo=1, IsAlive=0, IsMarked=0                       │
    │  In PendingDeleteList, not drawable, not targetable    │
    └─────────────────────────────┬──────────────────────────┘
                                  │ ProcessPendingDelete (tick end)
                                  ▼
                               [FREED]
```

---

## 7. Invariants a Port Must Preserve

Ranked by how often violation causes visible parity bugs.

1. **Mark refuses InLimbo'd objects.** Any Mark-path operation on a limboed object must
   be a no-op. In the binary this is enforced by the very first check in
   `ObjectClass::Mark`. If a port allows marking a limbo'd object, you can double-add
   to a cell list, producing cycles and infinite loops.

2. **Conceal before IsAlive=0.** Death path is Conceal → IsAlive=0, not vice versa. If
   reversed, anim detach and screen dirty happen on an IsAlive=false object and several
   downstream systems (health bar, selection bracket, pip draw) will skip unexpectedly.

3. **Deselect is the FIRST step of Conceal.** Not the last. A selection pointer that
   survives Conceal will point to a limbo'd (but still allocated) object and the
   selection code will refuse to touch it, leaving a stuck selection.

4. **AddContent triggers Mark_Occupation as a side effect.** So does RemoveContent →
   Clear_Occupation. A port that splits cell-list maintenance from occupation-bit
   maintenance will have tick-boundary windows where the list and the bits disagree.
   Co-locate them.

5. **DoCloak is the standard cell-exit primitive, not only for cloak units.** Do not
   gate multi-cell-foundation add/remove on "is this unit cloakable?" — gate on
   "is this in ground layer?" The cloak gate is just the `ProcessCloakAndNotify`
   visual step, which is a no-op for non-cloakers.

6. **Mark_Occupation checks bridge flag; Clear_Occupation does NOT.** Asymmetric
   bridge handling is intentional. Matching symmetrically leaks occupation bits on
   destroyed bridges.

7. **NextObject means different things in different states.** Use separate fields,
   or be extremely disciplined about null-ing before every transition.

8. **Chronoshift in-flight state is a third state** — neither on-map nor in-limbo.
   A unit mid-warp is `InLimbo=0` but has NO cell registration. Cell-walking scans
   miss it; house-array scans find it. This asymmetry is deliberate and must match.

9. **Pending-delete is a one-tick window.** Between UnInit and the end-of-tick sweep,
   a dead object is still allocated. Any pointer comparison that would expect a
   freed slot to be NULL will find a valid (but IsAlive=false) pointer instead.

10. **Factory-construction limbo is identical to cargo limbo at the cell/display
    level.** Unit is fully off-map, not drawn, not scannable. Only the factory's
    cargo-like chain holds it. `ExitObject_Main` is the one transition out.

---

## 8. INI / RulesClass Inputs

No INI keys directly control the limbo/occupation machinery — it's all engine-internal.
The one rules value consumed is `[General] BridgeHeight` (default `4`) which derives the
cached Z-lepton bridge threshold (§5). `[General] ChronoReinfDelay` controls the
warp-in delay (§3.7).

---

## 9. Current Rust Implementation Status (Informational)

Detailed scan in this investigation; reproduced here for cross-reference. All 5
relevant subsystems are implemented:

- **Limbo state** modeled via [src/sim/passenger.rs:128](src/sim/passenger.rs#L128)
  `PassengerRole::Inside` + [src/sim/game_entity.rs:164](src/sim/game_entity.rs#L164) `dying` flag
- **Cell occupation** via [src/sim/occupancy.rs](src/sim/occupancy.rs) `OccupancyGrid` (BTreeMap-backed)
- **Bridge/ground layers** via [src/sim/occupancy.rs:96](src/sim/occupancy.rs#L96) layer tags +
  [src/sim/game_entity.rs:105](src/sim/game_entity.rs#L105) `on_bridge` bool
- **Mark/unmark lifecycle** at [src/sim/world/world_spawn.rs:240](src/sim/world/world_spawn.rs#L240) (spawn)
  and [src/sim/combat/mod.rs:482](src/sim/combat/mod.rs#L482) (death)
- **Transport load/unload** at [src/sim/passenger.rs:260](src/sim/passenger.rs#L260)
- **Chronoshift** at [src/sim/movement/teleport_movement.rs](src/sim/movement/teleport_movement.rs) (state machine with Relocate + ChronoDelay)

Gaps worth noting (not fixes — just observations for whoever owns this area):

- Rust occupancy skips transported entities (see [src/sim/occupancy.rs:94](src/sim/occupancy.rs#L94)). In gamemd,
  the transported unit is Conceal'd outright — `InLimbo=1` is the flag. Functionally
  equivalent but different mechanism; watch for edge cases in selection / targeting
  where the gate variable differs.
- Chronoshift Rust impl does `occupancy.move_entity()` atomically. gamemd instead has a
  "third state" (InLimbo=0, no cell registration) for the duration of ChronoReinfDelay.
  If you support scans mid-warp, they will behave differently.
- Death path in Rust uses `dying=true` for one tick then despawn; gamemd uses
  Conceal+PendingDeleteList. Similar two-tick behavior but different gate variables.

None of these are wrong — just non-identical mechanisms. If a parity bug surfaces
("unit visible for 1 tick when it shouldn't be", "chronoshifted unit can be attacked
by scan X"), the gate variable asymmetry is the first place to look.

---

## 10. Function Address Reference (this investigation)

Addresses new or re-verified in this pass. For the full cross-reference, see the
three source docs.

| Address | Name | Confidence | Verified In |
|---------|------|------------|-------------|
| `0x005F4250` | ObjectClass::Limbo (STUB, returns 0) | HIGH | This pass |
| `0x005F5850` | ObjectClass::Mark (dispatcher) | HIGH | This pass (new decomp) |
| `0x005F65F0` | ObjectClass::UnInit | HIGH | This pass (Ghidra-labeled) |
| `0x0071BFB0` | TerrainClass::Mark (subclass override) | HIGH | This pass |
| `0x005683C0` | TechnoClass::EnterCell_AddToMultiCells | HIGH | This pass (new decomp) |
| `0x005687F0` | TechnoClass::ExitCell_RemoveFromMultiCells | HIGH | Existing doc |
| `0x004D3780` | TechnoClass::DoCloak (corrected semantics) | HIGH | This pass (updated decomp) |
| `0x0047E8A0` | CellClass::AddContent | HIGH | This pass (re-verified) |
| `0x004733A0` | CargoClass::AddPassenger | HIGH | This pass |
| `0x00443C60` | BuildingClass::ExitObject_Main | HIGH | This pass |
| `0x00740EF0` | UnitClass::Mission_Unload | HIGH | This pass |
| `0x0065EC30` | ChronoSphere::WarpUnitsAtCell | HIGH | This pass |
| `0x007441B0` | ObjectClass::Mark_Occupation (vehicle bit 0x20) | HIGH | Existing doc |
| `0x005217C0` | InfantryClass::MarkCellOccupancy | HIGH | Existing doc |
| `0x00426270` | AnimClass::MarkCellOccupancy | HIGH | Existing doc |
| `0x00735310` | UnitClass init — writes `DAT_00B1D0AC` | HIGH | This pass |
| `0x005179D0` | InfantryClass init — writes `DAT_00A8F234` | HIGH | This pass |
| `0x00421E40` | AnimClass init — writes `DAT_0089A1B4` | HIGH | This pass |

Globals:

| Address | Name | Purpose |
|---------|------|---------|
| `0x00B1D0AC` | BridgeZ_Unit (cached) | Bridge-layer Z threshold (vehicles) |
| `0x00A8F234` | BridgeZ_Infantry (cached) | Same, for InfantryClass |
| `0x0089A1B4` | BridgeZ_Anim (cached) | Same, for AnimClass |
| `0x00B1D0B8` | BridgeHeight (raw cells) | `[General] BridgeHeight` rules value, default 4 |
| `0x007E1738` | double `0.5` | Rounding constant in Z-threshold derivation |
| `0x00B0F69C` | PendingDeleteList | Array of ObjectClass* awaiting end-of-tick free |
| `0x00B0F6A8` | PendingDeleteCount | Count for above |

---

## 11. Open Questions

- **`TypeClass+0xE58` (used in ObjectClass::Mark building-suppression).** What exactly is
  this flag? Likely `InvisibleInGame` or similar TS-holdover. Low-priority to resolve
  since it defaults to off for all standard YR buildings. Evidence: branch is only hit
  for WhatAmI==6 AND flag set AND mode==0; gated behavior suggests rare/optional.
- **Exact order policy in `CargoClass::AddPassenger`.** The branch that walks forward
  looking for `IsTechno` bit before inserting — under what specific condition does it
  append rather than prepend? May be "keep terror drones at front" or "keep infantry
  together". Not critical for correctness, but affects unload order.
- **`FUN_007258D0`** called early in UnInit. Some global cleanup — likely removes the
  object from some per-frame processing queue (timers? effects?). Low-priority but
  worth labeling if it comes up again.
- **Pending-delete's `DAT_00B0F6A5` gate.** A byte flag that short-circuits the append.
  Likely "suppress pending-delete during serialization" or "game is shutting down".
  Not resolved.

---

## Sources

- `gamemd.exe` via Ghidra MCP (live decompilation, this pass)
- `OBJECTCLASS_DRAW_LIMBO_CELLLIST.md` (existing)
- `CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md` (existing)
- `INFANTRY_SUBCELL_POSITIONING.md` (existing)
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` (referenced for offsets)
- `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` (referenced for DoCloak gating)
- Rust scan: [src/sim/occupancy.rs](src/sim/occupancy.rs), [src/sim/passenger.rs](src/sim/passenger.rs),
  [src/sim/movement/teleport_movement.rs](src/sim/movement/teleport_movement.rs), [src/sim/combat/mod.rs](src/sim/combat/mod.rs),
  [src/sim/world/world_spawn.rs](src/sim/world/world_spawn.rs)
