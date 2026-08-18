# Object Fog/Shroud Visibility Decision Tree — Ghidra Research Report

## Summary

When the engine renders objects, visibility is determined by a multi-layered system.
There is **no single "IsDiscoveredByPlayer" call** gating the rendering loop. Instead,
objects are kept out of the display layers entirely until revealed, and separate
per-object flags control whether `DrawIt` returns early.

**Three visibility states for objects:**
1. **Shrouded** (never seen) — object is NOT in any display layer, `+0x81 = 1`. Completely invisible.
2. **Fogged** (previously seen, not currently visible) — object may remain in display layers but
   uses translucent/dimmed blit flags. A `FoggedObjectClass` ghost may be created to remember
   what was there.
3. **Visible** (currently in a unit's sight range) — object is drawn normally.

---

## Key Object Fields

### ObjectClass+0x80 — "NeedsDraw" dirty flag
- **Type:** `bool` (1 byte)
- **Init:** 0 in constructor
- **Purpose:** Set to 1 when the object needs to be redrawn. Cleared to 0 inside `DrawIt`
  after the draw is processed. If this is 0 AND the `param_3` draw-force flag is 0,
  `DrawIt` returns early (no draw needed).

### ObjectClass+0x81 — "IsUndiscovered" flag (CRITICAL)
- **Type:** `bool` (1 byte)
- **Init:** **1** in constructor (0x005f392a: `MOV byte ptr [ESI + 0x81], 1`)
- **Purpose:** When non-zero, the object is **completely hidden from rendering**.
  `ObjectClass::DrawIt` (vtable 0x104, at 0x005f4b10) checks this field early:
  ```
  if (!g_MapEditorMode && g_hWnd != 0) {
      if (param_force == 0 && this->field_0x80 == 0) return 0;
      if (this->field_0x81 != 0) return 0;  // HIDDEN — skip draw
  }
  ```
- **Set to 0** (revealed) by `ObjectClass::Reveal` (vtable 0xD8, at 0x005f4ec0)
- **Set to 1** (hidden) by `ObjectClass::Conceal` (vtable 0xD4, at 0x005f4d30)

### TechnoClass+0x41A — "HasBeenDiscovered" (by any house)
- **Type:** `bool` (1 byte)
- **Purpose:** Used in `TechnoClass::GetVisualState` to determine cloaking/visibility
  visual state. If 0 and the TypeClass has the "stealth" flag (+0xC9A), the object
  returns visual state 5 (invisible).

### TechnoClass+0x41B — "DiscoveredByPlayer" flag
- **Type:** `bool` (1 byte)
- **Init:** 0
- **Purpose:** Set to 1 when `TechnoClass::Discover` (vtable 0x198, at 0x006f4960) is
  called with `param == g_PlayerPtr`. Once set, the object won't re-trigger the
  discovery event. Also sets `HouseClass+0x5778` and `+0x5779` flags.

---

## Key Cell Fields

### CellClass+0x12C (byte offset 300) — Shroud bitflags
- **Bit 3 (0x08):** Cell has been explored (shroud cleared). `IsShrouded()` at 0x00586360
  returns 1 (shrouded) when this bit is CLEAR.
- **Bit 4 (0x10):** Fully revealed — no shroud edge transition needed.

### CellClass+0x130 — GAP Generator re-shroud counter
- Incremented by `FUN_00487690` when a GAP generator covers this cell
- Decremented by `FUN_00487630` when GAP coverage is removed
- When > 0, the cell is under GAP effect (re-shrouded for enemies)

### CellClass+0x13C — Fog counter (IsFogged check)
- `IsFogged()` at 0x005864a0 returns 1 when `cell+0x13C >= 1`
- This represents the number of "fog sources" affecting the cell

### CellClass+0x140 — Fog-of-war bitflags
- **Bit 1 (0x02):** Fog-of-war is active on this cell (was seen, currently not in sight)
- **Bit 0 (0x01):** Fog edge fully clear (no fog border needed)
- **Bit 5 (0x20):** GAP generator fog overlay active
- **Bit 8 (0x100):** Used by fogged object rendering checks

---

## The Complete Visibility Decision Tree

### Phase 1: Object Reveal / Conceal (sim-time, not per-frame)

When a unit's sight range covers a cell, `MapClass::RevealShroud` (0x005673a0) is called:
1. Sets `CellClass+0x12C |= 0x18` (marks cell as explored)
2. Calls `CellChangeNotify` (0x004aa050) which iterates objects in the cell
3. For each object found, calls vtable 0x198 (`TechnoClass::Discover`)
4. `Discover` calls `ObjectClass::Reveal` (vtable 0xD8) which:
   - **Sets `+0x81 = 0`** (object now drawable)
   - Submits object to display layer via `DisplayClass::Submit_Object`
   - Creates an `AlphaShapeClass` for the fog ghost effect
   - Sets `+0x80 = 0` (clear dirty flag, will be set again when needed)

When an object leaves visibility (enters fog):
1. `ObjectClass::Conceal` (vtable 0xD4, at 0x005f4d30) is called
2. Removes object from display layer (vtable 0x150)
3. **Sets `+0x81 = 1`** (object now hidden from rendering)
4. A `FoggedObjectClass` instance may be created to remember the object's last-known
   appearance for the fog ghost rendering

### Phase 2: TechnoClass::GetVisualState (vtable 0x68)

Called at the START of `TechnoClass::Draw` (0x00706640). Returns a visual state enum:
- **0** = Normal — fully visible (blit flags 0x2000)
- **1** = 25% translucent (blit flags 0x2002) — partially cloaked
- **2, 3** = 50-75% translucent (blit flags 0x2004) — more cloaked
- **4** = Darken (blit flags 0x200A/0x200C) — used for subterranean units
- **5** = **Invisible — IMMEDIATE RETURN, object not drawn at all**

The state-5 return happens when:
- TypeClass+0xC9A is set (stealth-related flag) AND `+0x41A == 0` (never seen)
- Cloaking state (`+0x220`) == 2 (fully cloaked) AND no sensor detects the unit in its cell

For cloaked units on sensor-covered cells, state 3 is returned instead (75% translucent).

### Phase 3: ObjectClass::DrawIt gate (vtable 0x104)

`ObjectClass::DrawIt` at 0x005f4b10 has early-exit checks:
```
if (!g_MapEditorMode && g_hWnd) {
    if (!param_force && !this->needsDraw_0x80)
        return 0;  // No redraw needed
    if (this->isUndiscovered_0x81)
        return 0;  // Object is in shroud — don't draw
}
```
This is the final gate. If `+0x81` is non-zero, the draw is skipped entirely.

### Phase 4: Tactical_ObjectRenderingLoop (0x006d8db0)

This function iterates `g_DisplayLayers[0..4]` (5 layers). For each object:

1. Clears `object+0x99` (on-screen flag) to 0
2. Checks `AbstractFlags` at `object+0x14`:
   - **Bit 2 set** → Building path: gets coords via vtable 0x4C, checks bounds,
     then calls vtable 0x104 (`DrawIt`). For fog-of-war mode (`SpecialFlags & 0x1000`),
     calls `FUN_005865e0` which always returns 0 in YR (fog shroud check disabled/stubbed).
   - **Bit 0 set, bit 2 clear** → Mobile unit path: checks RTTI type, performs bounds
     check, calls vtable 0x104 (`DrawIt`)
   - **Bit 0 clear** → Not on map, skip
3. Sets `object+0x99 = 1` if on screen

After the main loop, buildings in layer 2 get garrison fire updates rendered.

### Phase 5: Post-rendering passes in TacticalClass::Draw

After `Tactical_ObjectRenderingLoop`:
- `DrawRadarOverlays_Fog()` draws radar indicators for allied units in fog
- Iterates `g_TechnoClass_Array` for selection brackets, capture links, and tracking
  lines. The `+0x81 == 0` check appears here to draw tracking dots only for
  discovered enemy buildings.

---

## FoggedObjectClass — Fog Ghost System

When fog-of-war is enabled (`SpecialFlags & 0x1000`), previously-visible objects that
enter fog get a **FoggedObjectClass** instance created as a "ghost" record.

### Structure (from constructor at 0x004d08b0)
- Inherits from `INoticeSink` (not ObjectClass)
- Vtable at 0x7e8b38
- Fields include: RTTI type (`+0x30`), cached coordinates (`+0x34..+0x3C`),
  blit dimensions (`+0x40..+0x4C`), draw record list (`+0x5C`), SHP frame data

### Storage
- Stored in `CellClass+0x28` as a `DynamicVectorClass<FoggedObjectClass*>`
- Created when `MapClass::RevealFogCell` / `MapClass::UpdateFogOfWarCell` transitions
  a cell from visible to fogged
- Destroyed by `FUN_00486c50` when a cell is re-revealed (the ghost is no longer needed)

### Rendering
- Fogged objects are drawn as remembered SHP sprites with translucent blitting
  (50% darkened). They appear as dim versions of the last-known state.
- The `FUN_005ff850` overlay rendering function handles drawing fogged overlays
  with translucency based on `DAT_00833588` blend mode (0-3 = different alpha levels)

---

## SpecialFlags: FogOfWar bit

- **Address:** `DAT_00a8b230` (pointer to SpecialFlags struct)
- **Bit 12 (0x1000):** FogOfWar enabled
- When disabled: all cells are treated as visible, `IsFogged` / `IsShrouded` checks
  are short-circuited, `FoggedObjectClass` instances are not created
- Configured via `[SpecialFlags]` section: `FogOfWar=yes/no`

---

## Function Address Reference

| Address | Name | Purpose |
|---------|------|---------|
| 0x005f4b10 | ObjectClass::DrawIt (vtable 0x104) | Main draw gate, checks +0x81 |
| 0x005f4ec0 | ObjectClass::Reveal (vtable 0xD8) | Sets +0x81=0, submits to display |
| 0x005f4d30 | ObjectClass::Conceal (vtable 0xD4) | Sets +0x81=1, removes from display |
| 0x006f4960 | TechnoClass::Discover (vtable 0x198) | Sets +0x41B, calls Reveal |
| 0x006f6ca0 | TechnoClass::Unlimbo (calls Reveal) | Places object on map |
| 0x006d8db0 | Tactical_ObjectRenderingLoop | Iterates display layers, calls DrawIt |
| 0x006d3d10 | TacticalClass::Draw | Master render entry point |
| 0x00706640 | TechnoClass::Draw | Per-unit draw, calls GetVisualState |
| 0x0041bf40 | TechnoClass::CanFidget (vtable 0x160) | Timer check for idle anims |
| 0x00586360 | IsShrouded | Checks cell+0x12C bit 3 |
| 0x005864a0 | IsFogged | Checks cell+0x13C >= 1 |
| 0x005673a0 | MapClass::RevealShroud | Main shroud reveal, triggers Discover |
| 0x004a9dd0 | MapClass::UpdateFogOfWarCell | Fog transition, creates fogged ghosts |
| 0x004aa050 | RevealCell | Sets shroud flags, notifies objects |
| 0x004d08b0 | FoggedObjectClass::Constructor | Creates fog ghost record |
| 0x00486c50 | FoggedObjectClass removal | Destroys ghosts when cell re-revealed |

---

## Confidence Levels

- **ObjectClass+0x81 as the draw gate:** HIGH (verified from binary — constructor init,
  DrawIt early return, Reveal/Conceal write patterns, 7 write-0 sites, 12 write-1 sites)
- **TechnoClass::Discover at vtable 0x198:** HIGH (verified call chain from CellChangeNotify
  through RevealShroud)
- **GetVisualState return values 0-5:** HIGH (verified from switch in TechnoClass::Draw)
- **CellClass shroud/fog bit layout:** HIGH (verified from IsShrouded, IsFogged, RevealShroudFlags)
- **FoggedObjectClass stored at CellClass+0x28:** MEDIUM-HIGH (inferred from FUN_00486c50
  which iterates cell+0x28 vector and destroys fogged objects)
- **CellClass+0x13C as fog counter:** MEDIUM (IsFogged reads it, but the increment/decrement
  logic wasn't fully traced)
