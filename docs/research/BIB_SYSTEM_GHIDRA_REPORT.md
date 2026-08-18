# Bib System — Ghidra Report

**Scope:** "Building bibs" in Yuri's Revenge — the concrete/asphalt slab that extends
the front of certain buildings, both visually and as part of the occupied footprint.

**Verdict up front:** There is **no `BibClass` in gamemd.exe**. "Bib" is three
loosely related pieces of state on `BuildingTypeClass` that share a name but have
**different meanings**:

| Concept | INI key | Field | What it actually does in YR |
|---|---|---|---|
| Bib flag | `Bib=yes` | `BuildingTypeClass+0x1570` (byte, `HasBib`) | Live but subtle. Has **no** effect on the building's foundation cell list (occupancy, placement, render origin all use the raw foundation). Read by `UnitClass::Can_Enter_Cell` to *relax* the entry block on one edge of the foundation — see §2.5. Magnitude depends on the runtime value of `DAT_0089F690`, which is set by an initializer (`FUN_0049f300`) whose call chain is unanalyzed. |
| Bib-clear scatter | `WeaponsFactory=yes` | `BuildingTypeClass+0x16BD` (byte) | Gates the "scatter units off the exit strip before producing a vehicle" routine. |
| Bib graphic | `BibShape=` | `BuildingTypeClass+0x1518` (SHP ptr) + `+0x151C` (flag) | The visual SHP for the bib. Stock YR uses this on every bib-having building (~10+) via `art.ini`/`artmd.ini`. There is **no default-bib fallback** in YR. |

Three traps:
- **Do not assume `ClearBibArea` is gated on `Bib=yes`.** It is gated on
  `WeaponsFactory=yes`. The two flags are independent.
- **Do not assume `Bib=yes` extends the building's footprint cell list.** It
  does not. `Place_OccupyMap` (which marks occupied cells at placement time)
  walks the foundation cell list from `vtable[0x108]`; the bib row is not in
  that list. Bib cells are not marked as owned.
- **Do not assume `Bib=yes` is a no-op either.** It changes the per-cell
  passability check in `UnitClass::Can_Enter_Cell` — see §2.5. The exact
  observable effect requires runtime verification.

**Confidence:** High — verified by live Ghidra decompilation of
`BuildingTypeClass__LoadVisualAssets` (0x45F230) and `BuildingClass__ClearBibArea`
(0x449540), plus cross-reference with existing docs
(`BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md`, `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`,
`BUILDING_SYSTEMS_GHIDRA_REPORT.md`, `DAMAGE_FIRE_ANIMS_GHIDRA.md`).

---

## 1. INI → field mapping

### `Bib=yes` → `BuildingTypeClass+0x1570` (byte, "HasBib")

This is the real "bib" flag. In-repo INI uses:

- `ini/rulesmd.ini` — lines 11730, 11791, 12523, 12581, 13325
- `ini/rules.ini` — lines 8439, 8474, 8562, 8605

Sample in rulesmd.ini: the Allied/Soviet War Factory, Barracks, Radar, ConYard,
and similar "large" buildings set `Bib=yes`. Small (1×1, 2×2) buildings like
pillboxes, walls, silos do not.

Internal field name per decompiled call sites is `HasBib`. It is a single byte
(boolean).

### `WeaponsFactory=yes` → `BuildingTypeClass+0x16BD` (byte)

This is a completely separate flag. Documented in
`BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md:231` as the "vehicle production" flag.
Testing it at runtime tells the mission code that this building produces vehicles
and therefore needs to clear its exit strip before spawning one.

### `BibShape=` → `BuildingTypeClass+0x1518` (SHP ptr), `+0x151C` (has-been-set flag)

Verified at `0x45F862` in `BuildingTypeClass__LoadVisualAssets`. The ReadString
loads the filename, `LoadFileFromMIX` retrieves the SHP handle, and both the
pointer (+0x1518) and a "set" byte (+0x151C) are updated. This is the **custom
override** for the bib graphic. If `BibShape=` is not specified, the default
derived art (see §4) is used instead.

---

## 2. What `Bib=yes` actually does (and does not)

Earlier docs claimed `Bib=yes` extends the building's foundation height by +1 row
and adds bib cells to the occupancy footprint. **The cell-list claim is wrong**:
the bib row is not in the foundation cell list, never gets marked owned, and is
never validated for placement. **But `Bib=yes` is not a no-op** — it modifies
how `UnitClass::Can_Enter_Cell` evaluates the cells that DO contain the building.

### 2.1 The +1 bib branch in `GetFoundationHeight` is a caller opt-in, and no caller opts in

`BuildingTypeClass::GetFoundationHeight` at `0x0045ECA0`:

```c
int GetFoundationHeight(BuildingTypeClass *this, char wantBibExtension) {
    if ((wantBibExtension != 0) && (this->HasBib_0x1570 != 0))
        return g_FoundationHeightTable[this->FoundationType_0xEF0] + 1;
    return g_FoundationHeightTable[this->FoundationType_0xEF0];
}
```

Auditing every BuildingClass-side caller (12+ sites), **every single one
passes `wantBibExtension == 0`**:

| Caller (BuildingClass / SlaveManagerClass) | Address | What it does |
|---|---|---|
| `Unlimbo` | 0x440580 | Building-placement entry; walks `(W+2)×(H+2)` adjacency grid |
| `GetCoords` | 0x447ACC | Center coordinate query |
| `GetHalfFoundationSize` | 0x458E00 | Selection / center sizing |
| `DrawBody` | 0x43D71A | Render path (computes pixel offset for SHP draw) |
| `OnDestroyed` | 0x445C99 | Decrements `cell+0x122` counter on destruction |
| `ExitObject_Main` | 0x444754, 0x444B34 | Vehicle exit ghost-cell calculation |
| `CanCloak` | 0x4577B5 | Scans foundation cells for non-cloakable adjacents |
| `Sell` | 0x449EA0 | Selling routine |
| `GetDockCellForObject` | 0x44F3B9, 0x44F4C3, 0x44F592 | Dock cell search |
| `ReceiveDamage` | 0x44279B | Damage-anim spawn-cell randomization |
| `CreateDamageFireAnims` | 0x43C1F5 | Fire animation spawning |
| `SlaveManagerClass::FindDeployCell` | 0x6B0353 | Slave deploy cell search |

Plus several `FUN_*` callers (DiskLaserClass::AI, FUN_005f6360, FUN_004a8eb0, FUN_00455f10, etc.) — all pass 0.

A byte-pattern search for `cmp byte ptr [reg+0x1570], 0` returned no hits
anywhere in the binary outside of `GetFoundationHeight`. **The +1 branch is
unreachable in normal YR gameplay.**

Plate comments documenting this finding have been added in Ghidra to
`GetFoundationHeight` (0x0045ECA0).

### 2.2 The authoritative cell-occupancy footprint

When a building is placed, **`BuildingClass::Place_OccupyMap` at `0x00441F60`**
walks the foundation cell list returned by `vtable[0x108]`:

- An array of `(short, short)` cell-deltas relative to the building origin,
  terminated by sentinel `(0x7FFF, 0x7FFF)`.
- For each foundation cell: clears the overlay, sets
  `cell->OverlayTypeIndex = 0xEF`, recalculates pathfinding zone,
  re-runs cell attribute fixup.
- The **origin cell only** gets its `cell+0x40` (owner-type pointer) set
  to the `BuildingTypeClass *`.

The bib row is **not** in this cell list. Plate comment added in Ghidra.

### 2.3 Consequence: bib cells are not marked

Bib cells are not in the foundation cell list and never get owned-cell or
sub-position markers. They lie outside the foundation rectangle, are not
validated for placement, and are not added to pathfinding as part of the
building. The bib SHP just happens to draw over cells that other entities
can otherwise use freely. (See §2.5 below for the one wrinkle this leaves.)

### 2.4 The Unit_Can_Enter_Cell HasBib check — what it actually does

`UnitClass::Can_Enter_Cell` (`0x0073F0A0`, body 0x0073F0A0–0x0073FD45)
contains a HasBib branch at `0x0073F7D3`:

```c
// Inside the per-entity loop walking the cell+0xE4 occupant chain.
// piVar15 = current entity in the chain
// iVar9 = piVar15->Type (BuildingTypeClass *)
// iVar18 = current cell pointer

if (piVar15->Type[0x16c0] == 0) {            // not a LaserFence segment
    if (piVar15->Type[0x1570] != 0) {        // HasBib
        offCell = MapClass.Get_CellClass({
            cell->X + (short)DAT_0089F690,
            cell->Y +  DAT_0089F690._2_2_,
        });
        if (Look_up_building_in_cell(offCell) != piVar15)
            goto LAB_0073fa87;                // skip this entity
    }
    LAB_0073f823:
    // ...rest of building-occupies-cell entry rules...
}
```

What this *means* in plain English: when a unit is checking whether it can
enter a cell that contains a `HasBib` building, the engine probes the cell
at `cell + DAT_0089F690` and asks "is the same building also there?" If
not, the engine treats this cell as if the building isn't blocking it
(skips the entity in the per-cell occupant chain) — the unit can enter.

In other words, **`HasBib` makes the cells along one edge of the building's
foundation passable** to vehicles. The "edge" is whichever edge has no
neighbor in the direction of `DAT_0089F690`.

**Caveat — runtime value of `DAT_0089F690` is uncertain.**

- The .data section initializer is `(0, 0)`. With `(0, 0)` offset, the probe
  cell equals the current cell, the building lookup trivially returns the
  same building, the comparison passes, and the bib check is degenerate
  (no-op).
- `FUN_0049f300` (`0x0049f300`) writes `DAT_0089F690 = (1, param_2)` and
  populates seven adjacent slots that look like an 8-direction offset table.
  If `FUN_0049f300` runs at startup, the runtime value is meaningful.
- **`FUN_0049f300` has no Ghidra-traced callers.** It might be invoked via
  C runtime static-init chains (which Ghidra often misses), or it might be
  genuinely dead. Static analysis cannot resolve this — a debugger reading
  `DAT_0089F690` mid-game would.

### 2.5 Net behavioral picture

- Bib cells are NEVER added to the building's footprint. Placement validation,
  cell ownership, render origin, selection bracket, and damage-area queries
  all use the raw foundation rectangle (the `vtable[0x108]` cell list,
  terminated by `(0x7FFF, 0x7FFF)`).
- `Bib=yes` enables a passability-relaxation in `Unit_Can_Enter_Cell` that
  makes one edge of the foundation passable to units, *if* `DAT_0089F690`
  has a non-zero runtime value.
- The `cell+0x38` "owner-type index" mechanism cited by sibling docs is the
  generic per-cell foundation marker; it applies to all foundation cells of
  any building, not specifically to bibs. There is no bib-specific cell
  marking.

---

## 3. `ClearBibArea` — Weapons Factory exit-strip scatter

**Function:** `BuildingClass__ClearBibArea` at `0x00449540`
**Gate:** `Type+0x16BD` (WeaponsFactory), **not** `Type+0x1570` (HasBib)

### Behavior (decompiled)

```c
uint BuildingClass__ClearBibArea(BuildingClass *this) {
    BuildingTypeClass *T = this->Type;
    if (T->WeaponsFactory_0x16BD == 0) return 0;     // gate

    // Base coord: CellStruct read from +0x28 off whatever Type+0xED4 points to.
    // Type+0xED4 is a per-building reference block; exact field name is unverified.
    int baseX = *(int*)(*(int*)(T + 0xED4) + 0x28);

    // Foundation offset via vtable[0x1B8] (dimension/footprint accessor)
    short *foundation = vtable[0x1B8](this, &local_14);
    short cellX = foundation[0] + (short)baseX - 1;
    short cellY = foundation[1] + baseY_short;

    // Look for a blocker in the bib row
    CellClass *bibCell = Map.Get_CellClass(&cell);
    Object *blocker = bibCell->Find_Nearest_Object(..., this);
    if (!blocker) return 0;

    // NOTE: string order is OPPOSITE to what reads intuitively.
    // Initial scatter uses the shorter string; loop iterations use the longer one.
    // (corrected 2026-05-28: was reversed; binary confirmed via decompile_function 0x00449540 — ROOT_CAUSE: INFERENCE_HARDENED)
    log("Weapons factory clearing %s from bib\n", blocker->GetName());       // 0x00819008
    bibCell->Scatter_Objects(&leptonCoord, 1, 1, 0);

    // Up to 8 scatter iterations with pathfinding updates
    for (int i = 0; i < 8; i++) {
        Pathfinding_update_continued(i);
        blocker = bibCell->Find_Nearest_Object(..., this);
        if (blocker) {
            log("Weapons factory clearing %s from bib area\n", blocker->GetName());  // 0x00818FDC
            bibCell->Scatter_Objects(&leptonCoord, 1, 1, 0);
        }
    }
    return 1;
}
```

### Callers (complete set — 2)

Both callers are entries in the **BuildingClass mission dispatch table** at
`0x007E4090` (an array of per-mission function pointers; the dispatcher
indexes it by `MissionType` and calls the entry to advance one tick).

- **`FUN_004496b0` — mission slot 18** (table offset `0x48`). Unlabeled in
  Ghidra. Calls `BuildingClass__GrandOpening` on first tick
  (`param[0x2f] == 0`), then in the steady-state branch calls
  `ClearBibArea` (gated on `WeaponsFactory`) and logs
  `"Weapons factory clearing %s from bib area"`. Returns a randomized
  per-tick timer. Tests `IsRepairDepot` (Type+0x16a9), Type+0x16aa, and
  Type+0x16ab. Pattern matches `Mission_Construction` (because
  `GrandOpening` is the build-completion ritual) — but the exact YR
  mission enum index for slot 18 is **not yet confirmed**, so treat
  the name as a hypothesis.

- **`FUN_0044d880` — mission slot 26** (table offset `0x68`). Unlabeled in
  Ghidra. Two subsystems: (a) slave deployment when Type+0x16ae /
  Type+0x16af are set, and (b) WF vehicle eject when Type+0x16BD is set —
  a switch on `BuildingClass+0xBC` with cases 0..4. Case 1 calls
  `ClearBibArea` and logs `"Weapons factory bib clear - kicking out unit"`
  on failure. Cases 2..4 instantiate the unit's locomotor (Drive /
  Teleport / Hover) via `CoCreateInstance`. Pattern matches
  `Mission_Unload` (slave deploy + vehicle eject both fit "unload cargo"
  semantics) — exact YR enum index for slot 26 also unconfirmed.

  The earlier `0x0044dcb9` "function" referenced in v1 of this doc is a
  **Ghidra analysis artifact** — it is reached only by conditional jumps
  from inside `FUN_0044d880` and is not a real function entry point.
  Plate comments were added in Ghidra to flag this. The real caller is
  `FUN_0044d880`.

  Note: this is **not** `BuildingClass__ExitObject_Main` (`0x00443C60`,
  per `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md:1011`). `ExitObject_Main`
  is the routine that places a freshly-built unit into the world;
  `FUN_0044d880` is the per-tick mission routine that drives the unit
  off the bib once it's there.

### Debug strings (all three are embedded in this system)

- `0x818FDC` — `"Weapons factory clearing %s from bib area\n"`
- `0x819008` — `"Weapons factory clearing %s from bib\n"`
- `0x819098` — `"Weapons factory bib clear - kicking out unit\n"`

The phrasing is the original Westwood developer language — it confirms the
design intent was specifically "the strip in front of a vehicle-producing
building," not a generic property of any building with a bib.

---

## 4. Bib rendering — solved

The bib is drawn by `BuildingClass_DrawBody` at `0x0043D290` (bib branch is within that function body), in this branch:
<!-- corrected 2026-05-28: was "at 0x0043D71A"; 0x0043D71A is an inner offset, not the function entry. Function entry confirmed 0x0043D290 via get_function_by_address — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

```c
if (this->Type->BibShape_0x1518 != NULL && this->field_0x534 != 0) {
    // pixel offset for the bib SHP
    cellOffsetX = foundationWidth * 0x100 - 0x100;     // last column
    cellOffsetY = foundationHeight * 0x100 - 0x100;    // last row
    pixelOffset = TacticalClass::CellToPixel(origin, &cellOffset);
    drawX = baseDrawX - pixelOffset.x;
    drawY = baseDrawY - pixelOffset.y;

    TechnoClass_DrawSHP(
        this->Type->BibShape_0x1518,
        BuildingClass::GetCurrentFrame(this),    // frame index
        ..., drawX, drawY, ...);
}
```

Key facts:

- The bib SHP comes **only** from `BibShape=` — `Type+0x1518` populated by
  `LoadVisualAssets`. Gating: `Type+0x1518 != NULL` (SHP loaded) AND
  `BuildingClass+0x534 != 0` (per-instance "draw enabled" / placed flag).
- **There is no default-bib fallback.** A search of the binary for filename
  patterns like `BIB1.SHP`/`BIB2.SHP`/`BIB3.SHP` returned nothing. If
  `BibShape=` is not set in `art.ini`, no bib is drawn — period.
- The frame index is `BuildingClass::GetCurrentFrame(this)`, which lets the
  bib animate in lockstep with the main building (e.g. damaged-bib frames
  if the SHP provides them).
- `BuildingClass__Draw` at `0x4E0240` is a mislabeled vtable accessor, not
  the real draw — `DrawBody` is the real per-frame building draw.

### Stock YR uses BibShape= heavily

Confirmed in this repo's INI:

| Building | art.ini SHP | artmd.ini SHP |
|---|---|---|
| Allied War Factory | `GAWEAPBB` | `GAWEAPBB` |
| Soviet War Factory | `NAWEAPBB` | `NAWEAPBB` |
| Yuri War Factory | — | `YAWEAPBB` |
| Allied Refinery | `GAREFNBB` | `GAREFNBB` |
| Soviet Refinery | `NAREFNBB` | `NAREFNBB` |
| Allied Helipad | `GAHPADBB` | `GAHPADBB` |
| Soviet Helipad | `NAHPADBB` | `NAHPADBB` |
| Airfield | `GAAIRCBB` | `GAAIRCBB` |
| Allied Service Depot | `GADEPTBB` | `GADEPTBB` |
| Soviet Service Depot | `NADEPTBB` | (verify) |
| Civilian Outpost | `CAOUTPBB` | (verify) |
| Wasteland | `NAWASTBB` | (verify) |

Naming convention: `<BuildingName>BB.SHP`. There is no automatic name
derivation in the engine — each building explicitly lists its bib SHP, or
gets no bib.

### So what does `Bib=yes` do at all?

See §2.4 for the answer: `HasBib` is read by `UnitClass::Can_Enter_Cell` to
relax the entry block on one edge of the foundation. That is the only live
HasBib effect in YR. The +1 in `GetFoundationHeight` is a separate
caller-opt-in branch that no caller activates.

Whether the passability relaxation is observable in normal play depends on
the runtime value of `DAT_0089F690` (see §2.4) — uncertain without runtime
verification.

---

## 5. Implications for the Rust port

### What you should implement

1. **`BibShape=` on `BuildingType` (parsed from `art.ini`/`artmd.ini`).**
   Load the SHP via the asset pipeline. Drawn at the foundation last-row,
   last-column cell offset, with the same frame index as the main building
   image. Gate the draw on "BibShape SHP loaded" + "building visible/placed."
2. **`WeaponsFactory=yes`** triggers a per-tick "clear the cell in front of
   the building" scatter while the WF is producing a vehicle. Independent of
   `Bib=yes`.
3. **`Bib=yes`** — parse it. The minimum-fidelity behavior is to mirror what
   YR appears to do at the runtime values we can observe: do nothing.
   A higher-fidelity implementation should mirror the
   `UnitClass::Can_Enter_Cell` HasBib branch (§2.4): when computing per-cell
   passability for a `HasBib` building, the cells along one foundation edge
   become passable to units. The exact edge depends on the runtime value of
   `DAT_0089F690` and was not pinned down in this audit. Defer this until
   you observe a parity bug that traces back to it (e.g. units unable to
   path through a War Factory exit area) — at that point, instrument the
   original game to read `DAT_0089F690` mid-skirmish.

### What you must NOT implement

- **Do not extend the building's foundation cell list when `Bib=yes`.**
  Placement validation, cell ownership, render origin, selection bracket,
  damage-area queries, and the cell-occupancy marker (`cell+0x40` owner-type)
  all use the raw foundation rectangle from `vtable[0x108]` (terminated by
  `(0x7FFF, 0x7FFF)`). The bib row is not in this list.
- **Do not mark bib cells as owned by the building.** The engine doesn't.
- **Do not implement a default-bib SHP fallback** (no `BIB1.SHP`/`BIB2.SHP`
  by foundation width). YR doesn't have one. If `BibShape=` is missing, no
  bib draws.
- **Do not gate `ClearBibArea` scatter on `Bib=yes`.** It is gated on
  `WeaponsFactory=yes`.
- **Do not create a `BibClass`** — no such class exists.

### Tiberian Sun / live-status check

- `Bib=yes` (`Type+0x1570 HasBib`) → **live but with uncertain magnitude.**
  Has a real reader in `UnitClass::Can_Enter_Cell` (§2.4). Whether the
  effect is observable depends on `DAT_0089F690`'s runtime value, which
  could not be determined from static analysis. The +1 branch in
  `GetFoundationHeight` is separately unreachable (no caller opts in) and
  does look TS-legacy.
- `WeaponsFactory=yes` → **live in YR.** Used by all vehicle factories.
- `BibShape=` → **live in YR, widely used.** Every bib-having stock
  building lists one in `art.ini`/`artmd.ini`.
- `ClearBibArea` scatter → **live in YR.** Triggered on real production
  events; not gated behind any `SpecialFlags` bit.

---

## 6. Address / field summary

| Address / offset | Symbol | Purpose |
|---|---|---|
| `Type+0x1518` | BibShape SHP pointer | Custom bib graphic override (loaded via `LoadFileFromMIX`) |
| `Type+0x151C` | BibShape-set flag (byte) | Nonzero when `BibShape=` was specified in INI |
| `Type+0x1570` | `HasBib` (byte) | `Bib=yes` flag. Read by `UnitClass::Can_Enter_Cell` to relax entry block on one foundation edge (§2.4); also tested in `GetFoundationHeight`'s never-reached +1 branch |
| `Type+0x16BD` | WeaponsFactory (byte) | `WeaponsFactory=yes` flag — gates `ClearBibArea` |
| `Type+0xEF0` | Foundation type enum | Index into width/height tables |
| `DAT_008192B8` | Foundation width table | `[foundation_type] → width_cells` |
| `DAT_00819310` | Foundation height table | `[foundation_type] → height_cells` |
| `DAT_0089F690` | Bib offset (`CellStruct`-like, X+Y as two shorts) | The HasBib-check offset in `UnitClass::Can_Enter_Cell`. Initialized by `FUN_0049f300` to `(1, param_2)` along with seven adjacent slots that look like an 8-direction offset table. `FUN_0049f300` has no traced callers; .data section value is `(0, 0)`. Runtime value uncertain (see §2.4) |
| `0x0049F300` | `FUN_0049f300` | Initializer that writes `DAT_0089F690` and seven adjacent direction-offset slots. No traced callers — may run via CRT static-init or be dead. Plate-comment if you can pin down its caller chain |
| `0x007E4090` | BuildingClass mission dispatch table | Function-pointer array indexed by `MissionType`; bib-related routines live at slots 18 (FUN_004496b0) and 26 (FUN_0044d880). Disassembly comment added in Ghidra. |
| `0x00449540` | `BuildingClass::ClearBibArea` | Weapons Factory exit-strip scatter |
| `0x004496B0` | `FUN_004496b0` (BuildingClass mission slot 18) | Calls `GrandOpening` + `ClearBibArea`. Plate comment added in Ghidra. Likely `Mission_Construction`, unconfirmed. |
| `0x0044D880` | `FUN_0044d880` (BuildingClass mission slot 26) | Slave deploy + WF vehicle-eject state machine; case 1 calls `ClearBibArea`. Plate comment added in Ghidra. Likely `Mission_Unload`, unconfirmed. |
| `0x0044DCB9` | (Ghidra artifact) | NOT a real function — inner code block of `FUN_0044d880`. Plate comment added in Ghidra warning future readers. |
| `0x0045F230` | `BuildingTypeClass::LoadVisualAssets` | Loads `BibShape=` and other SHPs |
| `0x0045ECA0` | `BuildingTypeClass::GetFoundationHeight` | Applies the HasBib +1 adjustment **when caller passes `param_2 != 0`** |
| `0x0045EE70` | `BuildingTypeClass::CanBePlacedAt` | Placement validation over the foundation cell list (raw foundation, no bib) |
| `0x00440580` | `BuildingClass::Unlimbo` | Building-placement entry; calls `GetFoundationHeight(0)` (no bib row) when walking surrounding cells |
| `0x00441F60` | `BuildingClass::Place_OccupyMap` | Authoritative occupancy path: walks `vtable[0x108]` foundation cell list and marks cells. Bib row not in the list. Plate comment added in Ghidra |
| `0x0073F0A0` | `UnitClass::Can_Enter_Cell` | Per-cell passability check. Reads `HasBib` at 0x0073F7D3 and uses `DAT_0089F690` to optionally relax block (§2.4) |
| `0x00818FDC` | String | `"Weapons factory clearing %s from bib area\n"` |
| `0x00819008` | String | `"Weapons factory clearing %s from bib\n"` |
| `0x00819098` | String | `"Weapons factory bib clear - kicking out unit\n"` |
| `0x00819428` | String | `"BibShape"` (INI key literal) |

---

## 7. Resolved

1. **Default bib SHP loader.** No default loader. Stock YR uses `BibShape=`
   explicitly on every bib-having building (~10+ entries in `art.ini`/
   `artmd.ini`). If `BibShape=` is absent, no bib draws.
2. **Bib draw offset.** Drawn by `BuildingClass_DrawBody` at foundation
   `(width-1, height-1)` cell offset, frame = `BuildingClass::GetCurrentFrame`.
3. **Bib cells in the foundation cell list.** Not present. `Place_OccupyMap`
   walks `vtable[0x108]` and the bib row is not in that list.
4. **`GetFoundationHeight` +1 reachability.** All 12+ audited BuildingClass
   callers pass `param_2 = 0`. The +1 branch in this function is unreachable
   in normal YR. (Note: this is *separate* from the HasBib effect via
   `Unit_Can_Enter_Cell` — see §2.4.)
5. **Stock YR usage of `BibShape=`.** Heavily used (see §4 table).
6. **HasBib has a live reader.** `UnitClass::Can_Enter_Cell` at `0x0073F7D3`
   reads `Type+0x1570` and uses `DAT_0089F690` as a bib offset. The earlier
   "HasBib is dead" claim in this doc was wrong — it's now corrected (§2.4).
   The sibling `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` was substantially
   correct about this code path.

## Remaining open questions

1. **Runtime value of `DAT_0089F690`.** Initializer `FUN_0049f300` writes
   `(1, param_2)` along with seven adjacent direction-offset slots, but has
   no Ghidra-traced callers. Either it runs via CRT static-init (and the
   runtime value is meaningful) or it's dead (and `DAT_0089F690 = (0, 0)`,
   making the HasBib check degenerate). Needs runtime debugger verification.
   Determines whether the §2.4 passability relaxation is actually observable
   in normal play.
2. **Which foundation edge does the relaxation apply to?** Depends on the
   sign and direction of `DAT_0089F690`. If `(1, 0)`, the east edge becomes
   passable; if `(0, 1)`, the south edge; etc. Pinpoint after #1.
3. **`vtable[0x1B8]`** — foundation/footprint dimension accessor. Returns
   a short[2] (foundation width-1, height-1). Exact contract still loose.
4. **Identity of BuildingClass mission table slots 18 and 26.** Confirmed
   both are entries in the dispatch table at `0x007E4090`. Slot 18
   (`FUN_004496b0`) likely `Mission_Construction` (GrandOpening pattern).
   Slot 26 (`FUN_0044d880`) likely `Mission_Unload` (slave deploy + WF
   eject). Standard TS/RA2 mission enum does not produce a clean fit, so
   YR may have inserted entries that shift indices. Tracing the dispatcher
   would confirm.
