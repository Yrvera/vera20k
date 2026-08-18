# DisplayClass — Ghidra Research Report

**Full constructor:** `0x004A8730` (labeled `DisplayClass__constructor`, lowercase)
**Minimal/copy constructor:** `0x006AC840` (labeled `DisplayClass__Constructor`, uppercase)
**Vtable:** `0x007E6114` (64 slots, 256 bytes)
**RTTI:** `.?AVDisplayClass@@` at `0x00816BE0`
**Nested class RTTI:** `.?AVTacticalClass@DisplayClass@@` at `0x00820050`
  → `TacticalClass` is a **nested class inside DisplayClass** (confirmed via MSVC-mangled name)
**Global instance:** part of the 21 868-byte display chain at `g_DisplayChain = 0x00887640`
(see `GSCREEN_RTACTICAL_GHIDRA_REPORT.md` §1)
**Confidence:** HIGH — every address decompiled, vtable enumerated, layer system traced.
**Active in YR:** Yes — the backbone of the tactical view.

## 0. What this report adds

`GSCREEN_RTACTICAL_GHIDRA_REPORT.md`, `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md`,
`BANDBOX_SELECTION_GHIDRA_REPORT.md`, and the five `SELECTION_*_GHIDRA_REPORT.md` files
already cover the **three-pass render architecture**, **band-box drag selection**,
**selection brackets/health bars**, and the **outer Main_Tick orchestration**. This
report fills the remaining DisplayClass gaps:

- **Struct layout** past MapClass (+0x1174 onward)
- **Vtable enumeration** — which 27 slots DisplayClass overrides vs. inherits
- **Render-layer array** (`g_DisplayLayers`) — structure and mechanics
- **Submit_Object / RemoveFromLayer** — how objects enter and leave the 5 draw layers
- **Cursor pipeline** — hover → `DetermineAction` → `SetCursorFromAction`
- **The ~60-case Action → Cursor mapping** — the cursor-icon state machine

Everything below is additive; nothing restates what the existing docs already say.

---

## 1. Inheritance chain (from MAPCLASS report)

```
GScreenClass         (vtable: 0x7EA6FC, size: 0x10)
  └─ MapClass        (vtable: 0x7ED404, adds +0x10 to +0x1173)
      └─ DisplayClass (vtable: 0x7E6114, starts at +0x1174)  ← this report
          └─ RadarClass (vtable: 0x7F0344)
              └─ PowerClass (vtable: 0x7EFF54)
                  └─ SidebarClass (vtable: 0x7F3058)
                      └─ TabClass / ScrollClass / MouseClass
```

**Nested `TacticalClass` sibling** (`DisplayClass::TacticalClass`): confirmed via
RTTI mangled name `.?AVTacticalClass@DisplayClass@@`. The global at
`g_Tactical = 0x00887324` is an instance of this nested type, but the object
**does not live inside** the display-chain mega-object — it is a separately
allocated 3 608-byte sibling (see `GSCREEN_RTACTICAL` §1). Nesting is a
C++ source-level concept (access scoping); the two instances are not adjacent
in memory.

---

## 2. DisplayClass struct layout (+0x1174 onwards)

All byte offsets are from the start of the `g_DisplayChain` mega-object.
DisplayClass starts at **+0x1174** (confirmed by the lowercase constructor
writing `param_1[0x45D]` = byte 0x1174 as its first write, matching the end of
MapClass at 0x1174).

The lowercase constructor (`0x004A8730`) writes fields up to `param_1[0x478]` =
byte `0x11E0`, so DisplayClass's **primary contiguous region** is roughly
**0x1174 → 0x11E0 (~108 bytes)**. Additional fields initialized by
`DisplayClass::Init_Clear` (`0x004A88C0`) extend slightly further, up to
`+0x11D1`.

### Field table (verified from constructor + Init_Clear decompilation)

| Offset | Size | Type | Field | Evidence |
|--------|------|------|-------|----------|
| +0x1174 | 4 | int | field_0 / default_cell_coord (low) | `param_1[0x45D] = DAT_008A03F8` |
| +0x1178 | 2 | short | field_4_lo | `*(short *)(param_1 + 0x45E) = 0` |
| +0x117A | 2 | short | field_4_hi | `*(short *)((int)param_1 + 0x117A) = 0` |
| +0x117C | 4 | int | render_cache_reset / dirty_flag | `= 0` in ctor; also zeroed in `Init_Clear` |
| +0x1180 | 1 | bool | flag_0x460 | `= 0` |
| +0x1181 | 1 | bool | flag_0x1181 | `= 0` |
| +0x1182 | 4 | int | default_cell_coord_copy | `= DAT_008A03F8` |
| +0x1186 | 2 | short | field_pad | `= 0` |
| +0x1188 | 4 | int | field_0x462 (wide) | `= 0` |
| +0x118C | 4 | int | field_0x463 | `= 0` |
| +0x1190 | 4 | int | field_0x464 | `= 0` |
| +0x1194 | 4 | int | field_0x465 | `= 0` |
| +0x1198 | 4 | int | last_ref_object_idx | `= -1` (0xFFFFFFFF). Read/written via `DisplayClass__GetLastRefObject` (0x004AEB10) and `__SetLastRefObject` (0x004AEB30). |
| +0x119C | 1 | bool | has_last_ref | `= 0` |
| +0x11A0 | 4 | int | field_0x468 | `= 0` |
| +0x11A4 | 4 | int | field_0x469 | Set `= 0` in Init_Clear |
| +0x11A8 | 4 | int | field_0x46A | Set `= 0` in Init_Clear |
| +0x11AC | 4 | int | field_0x46B | `= -1` in ctor; Init_Clear sets `= -1` |
| +0x11B0 | 1 | bool | flag_0x46C | `= 0` in ctor; Init_Clear sets `= 0` |
| +0x11B1 | 1 | bool | flag_0x11B1 | `= 0` in both |
| +0x11B2 | 1 | bool | flag_0x11B2 | `= 0` in both |
| +0x11B3 | 1 | bool | suppress_cursor_update | `= 0` — **tested in `SetCursorFromAction`** to skip player-color writes |
| +0x11B4 | 1 | bool | flag_0x46D | `= 0` |
| +0x11B8 | 4 | int | field_0x46E | `= -1` in ctor; Init_Clear sets `= -1` |
| +0x11BC | 4 | int | field_0x46F (hover_target_coord_ptr) | `= 0` — tested in `SetCursorFromAction` as a target-coord output buffer (+0x46F). When set and `flag_0x11B3` is true, the cursor hover writes cell lepton coords + ground height into it. |
| +0x11C0 | 4 | int | field_0x470 | `= 0` |
| +0x11C4 | 4 | int | field_0x471 | `= 0` |
| +0x11C8 | 4 | int | field_0x472 | `= 0` |
| +0x11CC | 4 | int | **tint_rgb** | `= 0` — base cursor tint; Init_Clear keeps; SetCursorFromAction overwrites with computed player-color RGB. Byte layout: `+0x11CC=R, +0x11CD=G, +0x11CE=B`. |
| +0x11CF | 1 | bool | flag_0x11CF | Init_Clear sets `= 0` |
| +0x11D0 | 1 | bool | flag_0x11D0 | Init_Clear sets `= 0` |
| +0x11D1 | 1 | bool | flag_0x474 | `= 0` in ctor; `SetCursorFromAction` zeroes on entry |
| +0x11D4 | 4 | int | field_0x475 | `= 0` |
| +0x11D8 | 4 | int | field_0x476 | `= 0` |
| +0x11DC | 4 | int | field_0x477 | `= 0` |
| +0x11E0 | 4 | int | field_0x478 | `= 0` (last ctor write) |

After +0x11E0 come further DisplayClass fields used by the render pipeline and
selection/bandbox state — these are documented in the selection/bandbox reports
and are not repeated here.

### Globals initialized by the ctor (outside `this`)

```c
_DAT_008A072C = 0;   // unknown — part of placement-preview state
g_PLACE_SHP  = 0;    // current placement-ghost SHP pointer (null = none)
_DAT_008A0418 = 0;   // unknown — related to DAT_008A03F8 sentinel family
```

### The uppercase "minimal constructor" at `0x006AC840`

This writes only three fields (`+0x11CC..+0x11CE`) and calls
`GScreenClass::Constructor(param_2)` on a different pointer. Signature:

```c
DisplayClass__Constructor(DisplayClass *param_1, void *param_2) {
    GScreenClass__Constructor(param_2);         // base-init param_2, not param_1
    param_1[0x11CC] = 0;  // tint_rgb R
    param_1[0x11CD] = 0;  // tint_rgb G
    param_1[0x11CE] = 0;  // tint_rgb B
    param_1->vtable = &vtable_DisplayClass;
}
```

**Not the object's main constructor.** The mega-object at `g_DisplayChain` is
built via the lowercase `0x004A8730` chain
(`MapClass::constructor` → DisplayClass fields). The uppercase variant is
probably a C++ copy-helper or a placement-new template instantiation the
compiler emitted; callers are rare. Not worth decoding further unless a second
DisplayClass instance turns up in the wild.

---

## 3. Vtable (0x007E6114, 64 slots) — overrides vs inherited

Decoded from raw memory. **DisplayClass overrides 27 of MapClass's 64 slots**;
the remaining 37 slots are inherited unchanged.

| Slot | Offset | Address | Owner | Purpose |
|------|--------|---------|-------|---------|
| 0 | 0x00 | 0x004F4240 | GScreenClass | scalar deleting destructor |
| 1 | 0x04 | 0x0040D230 | GScreenClass | helper |
| 2 | 0x08 | 0x0040D240 | GScreenClass | helper |
| 3 | 0x0C | 0x005656D0 | MapClass | CellHasBuilding flag check |
| **4** | 0x10 | **0x004AEBF0** | **DisplayClass** | scalar-deleting destructor (DisplayClass override) |
| **5** | 0x14 | **0x004A8850** | **DisplayClass** | **Init / Alloc** (override — was 0x565800 in MapClass) |
| 6 | 0x18 | 0x004F42B0 | GScreenClass | inherited |
| **7** | 0x1C | **0x004A88C0** | **DisplayClass** | **Init_Clear** (override — calls MapClass's then does layer+state reset) |
| 8 | 0x20 | 0x004F42E0 | GScreenClass | inherited |
| **9** | 0x24 | **0x004A9700** | **DisplayClass** | input handler (override) |
| 10 | 0x28 | 0x004F42F0 | GScreenClass | MarkNeedsRedraw(N) |
| 11 | 0x2C | 0x004F4310 | GScreenClass | inherited |
| 12 | 0x30 | 0x004F4450 | GScreenClass | inherited |
| 13 | 0x34 | 0x004F4480 | GScreenClass | **RenderFrame_main** (inherited — the 3-pass coordinator) |
| 14 | 0x38 | 0x004AEBD0 | inherited | destructor chain helper |
| **15** | 0x3C | **0x004A97B0** | **DisplayClass** | Draw (override) |
| 16-19 | 0x40-0x4C | 0x004C9150 ×4 | abstract | placeholder/pure-virtual shim |
| 20-22 | 0x50-0x58 | 0x565AA0, 0x565B00, 0x565BC0 | MapClass | cell-array reset/resize/destroy (inherited) |
| **23** | 0x5C | **0x00577920** | MapClass | UnregisterBridgeRepairHut — note this is at a *different vtable offset* in DisplayClass vs MapClass (vtable layouts drift because DisplayClass inserts overrides) |
| 24 | 0x60 | 0x004AEBE0 | inherited | destructor helper |
| 25 | 0x64 | 0x0056BBE0 | MapClass | **UpdateCrateRegenTimers** (inherited; called per-tick by LogicClass) |
| 26-28 | 0x68-0x70 | inherited | MapClass | cell-init family |
| 29 | 0x74 | 0x00567230 | MapClass | **Viewport_Resized** (inherited — triggers idle voice on playfield entry) |
| **30** | 0x78 | **0x004A90D0** | **DisplayClass** | override (was thunk in MapClass) |
| **31** | 0x7C | **0x004AE4F0** | **DisplayClass** | override |
| **32** | 0x80 | **0x004AA160** | **DisplayClass** | override |
| **33** | 0x84 | **0x004A98A0** | **DisplayClass** | override |
| **34** | 0x88 | **0x004A9890** | **DisplayClass** | override |
| **35** | 0x8C | **0x004AE9D0** | **DisplayClass** | override |
| **36** | 0x90 | **0x004A9890** | **DisplayClass** | override (same addr as 34) |
| **37** | 0x94 | **0x004A9A90** | **DisplayClass** | override |
| **38** | 0x98 | **0x004A9DD0** | **DisplayClass** | override — probably `UpdateFogOfWarCell` (matches MapClass name) |
| **39** | 0x9C | **0x004AA050** | **DisplayClass** | override |
| 40 | 0xA0 | 0x004C9150 | abstract | placeholder |
| **41** | 0xA4 | **0x004A9840** | **DisplayClass** | override — likely `RefreshRadar` / refresh helper |
| **42** | 0xA8 | **0x004A8960** | **DisplayClass** | override |
| 43 | 0xAC | 0x0040D250 | inherited | helper |
| **44** | 0xB0 | **0x004AAD20** | **DisplayClass** | override |
| **45** | 0xB4 | **0x004AC310** | **DisplayClass** | override (likely BandBox-related) |
| **46** | 0xB8 | **0x004AAE90** | **DisplayClass** | **SetCursorFromAction** (§6) |
| **47** | 0xBC | **0x004AC380** | **DisplayClass** | **BandBox_MouseMove** (documented in BANDBOX report) |
| **48** | 0xC0 | **0x004AB9B0** | **DisplayClass** | **BandBox_LeftUp** (documented in BANDBOX report) |
| **49** | 0xC4 | **0x004AAD30** | **DisplayClass** | override (near 44) |
| 50 | 0xC8 | 0x007FFD08 | import | thunk (outside .text) |
| **51** | 0xCC | **0x004AEC30** | **DisplayClass** | override |
| 52 | 0xD0 | 0x006C9890 | inherited | unit-draw helper |
| 53 | 0xD4 | 0x007BA4D0 | external | non-.text |
| 54 | 0xD8 | 0x007FFD38 | import | thunk |
| **55** | 0xDC | **0x004AEC10** | **DisplayClass** | override |
| 56 | 0xE0 | 0x006C9890 | inherited | dup of 52 |
| 57 | 0xE4 | 0x006C98E0 | inherited | helper |
| 58 | 0xE8 | 0x007FFD88 | import | thunk |
| **59** | 0xEC | **0x004AECA0** | **DisplayClass** | override |
| 60 | 0xF0 | 0x00631D30 | external | helper |
| **61** | 0xF4 | **0x00477740** | **DisplayClass** | override |
| 62 | 0xF8 | 0x00631CC0 | external | helper |
| 63 | 0xFC | 0x007BA3C0 | external | non-.text |

**Note on thunks (slots 50, 54, 58):** addresses in the `0x007FFxxx` range are
outside the main `.text` — they're compiler-generated adjustor thunks or
import-table entries. Normal for MSVC-generated multi-inheritance vtables.

---

## 4. Render layers — `g_DisplayLayers` (0x008A0360, 5 layers)

DisplayClass owns a fixed-size array of **5 render layers** at
`g_DisplayLayers = 0x008A0360`. Each layer is a `LayerClass<ObjectClass*>`
(24 bytes, matching the standard `DynamicVectorClass` header). Total size:
`5 × 24 = 120 bytes`, extending `0x008A0360 → 0x008A03D8`.

**Layer count verified** by `DisplayClass::Init_Clear` loop bound
(`< 0x8A03D8` = +0x78 from base = exactly 5 layers).

### DisplayLayerEntry layout (24 bytes)

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| +0x00 | 4 | ptr | vtable | `LayerClass<ObjectClass*>` vtable |
| +0x04 | 4 | ptr | buffer | `ObjectClass*[]` — the object list |
| +0x08 | 4 | int | capacity | Allocated capacity |
| +0x0C | 1+1+2 | flags+pad | owns_memory + is_valid + padding | Standard DynVec header |
| +0x10 | 4 | int | count | Current object count — **this is what iteration bounds use** |
| +0x14 | 4 | int | grow_step | Grow step |

> **Note on the Ghidra struct label.** Ghidra's `DisplayLayerEntry` definition
> labels the +0x10 field "capacity" and +0x08 "count". That's reversed from
> the actual usage in `DisplayClass::RemoveFromLayer` (which decrements +0x10
> on removal — classic count semantics). Trust the code, not the label.

### Layer indices

By Westwood convention (and matching how `ObjectClass::vtable[0x78]` returns a
layer ID stored at `obj[0x94]`):

| Index | Purpose | Evidence |
|-------|---------|----------|
| 0 | Underground (subterranean units/burrowed) | First iteration slot |
| 1 | Surface (water, ground debris) | Submit_Object's `layer == 2` flag branch suggests Surface ≠ Ground |
| 2 | Ground (infantry, vehicles, buildings) | `Submit_Object` passes `layer == 2` as "sorted insert" flag — Ground is the largest layer and uses sorted-by-Y draw order |
| 3 | Air (aircraft, jump-jets in flight) | Between Ground and Top |
| 4 | Top (anims, beams, overlays rendered on top) | Last iteration slot |

Exact index→name mapping requires cross-checking ObjectClass subclasses'
`vtable[0x78]` returns — can be done in a follow-up pass if needed.

### DisplayClass::Submit_Object — 0x004A9720

```c
void Submit_Object(ObjectClass *obj) {
    if (obj == NULL) return;
    if (obj->layer_id[+0x94] != -1)
        RemoveFromLayer(obj);                    // already in a layer — move it
    int layer = obj->vtable[0x78]();             // ObjectClass::GetLayerID
    if (layer == -1) return;
    if (DynamicVector__Insert(obj, layer == 2))  // sorted insert iff Ground
        obj->layer_id[+0x94] = layer;            // cache the layer index
}
```

The `layer == 2` flag distinguishes **sorted insertion** (Ground layer uses
Y-sorting for correct isometric occlusion) from append-only insertion
(Air/Top/etc. use a flat list).

### DisplayClass::RemoveFromLayer — 0x004A9770

```c
void RemoveFromLayer(ObjectClass *obj) {
    if (obj == NULL) return;
    int layer = obj->layer_id[+0x94];
    if (layer == -1) return;

    // Primary removal from obj's cached layer:
    int idx = layer_vtable[0x10]();              // Layer::FindIndex(obj)
    if (idx != -1 && idx < layer.count) {
        layer.count -= 1;
        // shift-down everything after idx
        for (; idx < layer.count; idx++)
            layer.buffer[idx] = layer.buffer[idx+1];
        obj->layer_id[+0x94] = -1;
    }

    // Defensive pass — scan ALL 5 layers to purge any duplicates:
    if (obj->layer_id[+0x94] != -1) {
        for each layer l in g_DisplayLayers[0..5):
            idx = l.vtable[0x10]();              // find any stale reference
            while (idx != -1 && idx < l.count):
                l.count -= 1
                shift-down
        obj->layer_id[+0x94] = -1;
    }
}
```

**Why the defensive second pass:** an object may have been inserted into a
layer whose index doesn't match the cached one (e.g., a unit that re-computed
`GetLayerID` between Submit and Remove). The second pass is a belt-and-braces
cleanup that guarantees no dangling references remain.

**Implication for Rust parity:** the Rust engine must treat layer membership
as a *cached* hint and always perform a full pass when un-submitting, or
otherwise guarantee the cached layer id can't drift. Skipping the second
pass in gamemd.exe would leave zombie references that crash on render.

---

## 5. The cursor-update pipeline

Mouse movement over the tactical view drives a three-stage chain:

```
user moves mouse
   │
   ▼
GScreenClass::Input (vtable[9] on display chain)
   │  polls mouse position → routes to per-component dispatch
   ▼
DisplayClass::Dispatch (0x006922E0)
   │  trivial: calls FUN_00692F30 + CommandBar_Dispatch
   ▼
FUN_00692F30 (hover handler, 0x00692F30)
   │  1. Translates screen → tactical coords
   │  2. If band-box active → DisplayClass::BandBox_MouseMove (§ bandbox doc)
   │  3. Else → FUN_00692300 to resolve (cell_coord, target_obj) under cursor
   │  4. DisplayClass::DetermineAction(cell, target_obj, 1) → action_code
   │  5. DisplayClass::SetCursorFromAction(cell, target, action_code, …)
   ▼
cursor icon updated, player-color tint applied
```

### 5.1 DisplayClass::DetermineAction — 0x00692610

Returns an **Action code** (uint) given `(cursor_cell, target_obj, modifier)`.
The action code is the logical intent, not the cursor icon — that comes later.

```
int DetermineAction(short *cell_coord, int *target_obj, uint modifier):
    action = 0
    // (A) Selected-objects path — dominates if anything is selected:
    if (g_CurrentObjects_Count != 0):
        best = SelectBestObjectForAction()
        return target_obj == NULL
             ? best->vtable[0x70](cell, modifier, 0)   // "What_Action on cell"
             : best->vtable[0x74](target_obj, 0)       // "What_Action on target"

    // (B) No-selection cursor-hover path:
    bVar1 = true
    hover_obj = FUN_0040DD20()    // resolve object under cursor
    if hover_obj != NULL && hover_obj->flag[+0x41A] == 0:
        if hover_obj->type[0x88] == 2:                  // BuildingClass?
            if CellClass::SensorCountForHouse(player) != 0: goto visible
        else:
            visible:
            type = hover_obj->vtable[0x84]()            // GetType
            if type->flag[0xC9A] == 0: goto not_visible
        bVar1 = false  // hover target is visible and valid

    // (C) Global UI mode flags — inject specific actions:
    if DAT_00880998 != 0:   // deploy/guard mode
        if target_obj && (target_obj->vtable[0x3C]() != 0)
           && HouseClass::IsHumanPlayer()
           && target_obj->vtable[0x94]():
            action = 10 (0x0A)                    // Guard
            goto cursor_check
        action = 15 (0x0F)                        // Deploy/Unload

    if DAT_0088099A != 0:   // sell mode
        action = 0x22                             // Sell (default)
        if (complex building+structure-target check passes):
            action = 0x21                         // Sell specific structure

    if DAT_0088099B != 0:   // chrono/waypoint/paradrop mode
        if hover target is enemy house          : action = 0x2F / 0x2E
        elif cell in playfield and waypoint set : action = 0x2A
        else                                    : action = 0x2B or 0x30 (depending)

    if DAT_0088099C != 0:   // place-ghost mode (building placement preview)
        action = 0x3C

    if DAT_00880999 != 0:   // enter/gather mode
        action = 0x0E (Move) by default; 0x0C (Attack) or 0x0C+2 if bridge-repair
        else if cell has repairable overlay:
            action = 0x0C

    // (D) UIModeLock dispatch — optional global handler override:
    if DAT_008809A0 != -1:
        int iVar8 = ((UIModeHandler *)DAT_00A8E334[DAT_008809A0])
                      ->vtable[0x6C](cell, target_obj)
        if iVar8 != 0: action = iVar8

    // (E) Final fallbacks:
    if action == 0 and cell_has_waypoint: action = 0x2C    // Drop Waypoint
    if g_UIModeLock != 0: return 0                         // cursor frozen
    return action
```

**Action codes observed** (not exhaustive — the full enum lives in
`FUN_0070F0B0` + `SetCursorFromAction`'s switch):

| Action | Purpose |
|--------|---------|
| 0x07 | GRepair (reserved / guard-repair) |
| 0x0A | Guard |
| 0x0C | Attack |
| 0x0E | Move |
| 0x0F | Deploy / Unload |
| 0x21 | Sell (specific target) |
| 0x22 | Sell (generic) |
| 0x2A | Chrono / Waypoint A |
| 0x2B | Chrono / Waypoint B |
| 0x2C | Drop Waypoint |
| 0x2E | Waypoint on enemy |
| 0x2F | Waypoint on enemy variant |
| 0x30 | Select Waypoint |
| 0x3C | Place (ghost preview) |
| 0x3D | Enter Transport |

### 5.2 DisplayClass::SetCursorFromAction — 0x004AAE90

Consumes an Action code and selects the final cursor SHP frame. Two things
happen in parallel:

**Step A — Player-color tint computation (upfront):**
Reads the player's color palette entry at `g_PlayerPtr+0x20C`, extracts RGB
via `g_DD_{R,G,B}Shift/Loss` (pixel-format-aware), and caches the tint bytes
at `this[+0x11CC..+0x11CE]` (the tint_rgb field). Used to color-wash action
cursors to match the player's house color.

**Step B — Action → cursor switch (the big switch):**
The action code goes through `FUN_0070F0B0` first, which substitutes veteran
variants:

```c
int FUN_0070F0B0(int action) {
    bool is_veteran = FUN_00731BF0();
    if (action == 1 && is_veteran) return 0x3E;    // veteran attack cursor
    if (action == 5 && is_veteran) return 0x3F;    // veteran something
    return action;
}
```

Then the result indexes a ~60-case switch that calls `this->vtable[0x48](cursor_id, …)`
with the final cursor SHP frame number. Selected mappings (default-path branch;
`param_3 != 0` branch selects shift-held variants):

| Action (post-veteran) | Default cursor | Shift-held cursor | Meaning |
|---|---|---|---|
| 1 | 0x12 | 0x12 (same) | Select |
| 2 | 0x13 | 0x13 | Move normal |
| 3, 9, 0xB, 0x23 | 0x19 | 0x3C | Attack (normal → force-fire) |
| 4, 0x34 | 0x1B | — | Enter |
| 5 | 0x14 (enter hostile) / 0x15 | 0x12 | Enter-or-select |
| 10 | 0x22 | 0x23 | Deploy |
| 0xC | 0x1E | 0x20 (force-attack) | Attack-move |
| 0xD | 0x1F | 0x20 | Guard-area |
| 0x10, 0x37, 0x38, 0x40 | 0x34 | 0x35 | Sell / sell-unit |
| 0x11, 0x1B, 0x21, 0x22, 0x2A–0x2F, 0x31–0x33 | 0x3C | 0x3C | Place-ghost / waypoint |
| 0x14 | — | 0x35 | Repair |
| 0x1A | FUN_00731CC0(param_6) | FUN_00731CC0(param_6) | Chrono-dependent |
| 0x1D | 0x21 | — | — |
| 0x1E | — | 0x1C | Enter-force |
| 0x1F, 0x24 | — | 0x1A | Select-unit variant |
| 0x25 | — | 0x39 | Deploy-variant |
| 0x26 | — | 0x32 | Select-area |
| 0x27, 0x28 | — | 0x3A | — |
| 0x29, 0x41 | — | 0x2F | — |
| 0x35 | 0x26 | 0x26 | — |
| 0x39 | — | 0x3B | — |
| 0x3C | 0x4E | 0x4E | Place-ghost final |
| 0x3E, 0x3F | FUN_00731CB0 | FUN_00731CB0 | Veteran attack |
| 0x42 | — | 0x53 | — |
| 0x43 | — | 0x55 | — |
| 0x44 | — | 0x51 | — |
| 0x45 | — | 0x4F | — |
| 0x46 | — | 0x50 | — |
| 0x47 | 0x52 | — | — |
| 0x48 | — | 0x54 | — |

**Waypoint pulse animation (actions 0x2A, 0x2B, 0x2F, 0x2C-0x2E, 0x31-0x33):**
Instead of a single cursor SHP, these actions **animate the cursor through 8
color-cycle frames** per tick. Each frame:
1. Takes the player's color index `g_PlayerPtr[0x20C] % 0xC` × 8 → offset
2. Reads 3 bytes of RGB from `DAT_00885180 + offset*3`
3. Re-encodes via `g_DD_{R,G,B}{Shift,Loss}` to the surface's pixel format
4. Writes into the cursor palette at `DAT_0087F6C8[0x174] + frame*2`

This produces the **pulsing cyan/red waypoint cursor** signature to RA2.

**Parity-critical detail:** the HSV rotation base `DAT_00885180` is a
**2-byte + 1-byte packed RGB table** (3 bytes per entry), indexed by
`player_color_index * 8 + frame`. The 8-frame cycle pulses through the
player's house color. If the Rust engine renders this without the exact same
per-frame color progression, the cursor pulse will "feel wrong."

### 5.3 Default cursor (action 0)

The default/no-action path uses the **tint_rgb bytes cached in the object**
(`this[+0x11CC..+0x11CE]`) and re-encodes for the current surface format —
i.e., the default cursor is color-tinted to the player's house color, no
animation.

---

## 6. Init / teardown

### DisplayClass::Init_Clear — 0x004A88C0

Called on scenario reset / save-load restore. Sequence:

```c
void Init_Clear(DisplayClass *this) {
    MapClass::Init_Clear(this);        // 0x005659F0 — calls base, resets crate timers etc.

    this->field_0x469[+0x11A4]     = 0;
    this->field_0x46A[+0x11A8]     = 0;
    this->field_0x46B[+0x11AC]     = -1;
    this->field_render[+0x117C]    = 0;
    this->field_0x46E[+0x11B8]     = -1;
    this->flag_0x46C[+0x11B0]      = 0;
    this->flag_0x11CF              = 0;
    this->flag_0x11D0              = 0;
    this->flag_0x11B1              = 0;
    this->flag_0x11B2              = 0;

    // Clear all 5 render layers:
    for each layer l in g_DisplayLayers[0..5):
        l.vtable[0x0C]();              // LayerClass::Clear — drops all object refs
}
```

**Note:** Init_Clear **does not free** the layer backing buffers — it only
calls the per-layer `Clear` method (slot 0x0C), which typically zeroes the
count without releasing heap memory. Save/load between scenarios reuses the
buffers.

### DisplayClass::GetLastRefObject — 0x004AEB10

Returns `this->field_0x466[+0x1198]` (the cached "last referenced object"
index, used by `LogicClass::PerTickUpdate` step 27 to recenter tactical view
after save-load restore).

### DisplayClass::SetLastRefObject — 0x004AEB30

Writes `this->field_0x466[+0x1198]` with the provided object index. Called
when the player clicks/double-clicks an object or when the camera auto-pans
to a production-complete building.

---

## 7. Integration with other systems

| Caller | Entry point | Purpose |
|--------|-------------|---------|
| `GScreenClass::Input` (inherited vtable[9]) | `DisplayClass::Dispatch` (0x006922E0) | Per-frame input → hover update |
| `RenderFrame_main` (0x004F4480) | `DisplayClass::Draw` (vtable[15] = 0x004A97B0) | Frame-composition chain (see TACTICAL_RENDER_PIPELINE) |
| `LogicClass::PerTickUpdate` (0x0055AFB0) step 27 | `GetLastRefObject` (0x004AEB10) | Recenter tactical view after save/load |
| `ObjectClass::Mark` (various) | `Submit_Object` (0x004A9720) | Add object to render layer |
| `ObjectClass::Limbo` / destructor | `RemoveFromLayer` (0x004A9770) | Remove object from render layer |
| Mouse hover pipeline | `DetermineAction` → `SetCursorFromAction` | Cursor icon selection |
| Player-click pipeline | `SelectBestObjectForAction` → object `->What_Action` → action → execute | Click routing (not in this report — see SELECTION_SYSTEM) |

---

## 8. Current Rust implementation status

| DisplayClass feature | Rust location | Status |
|----------------------|---------------|--------|
| Render-layer array (5 layers, sorted-by-Y for Ground) | `src/render/tactical/` (multi-atlas + draw-order system) | Implemented — different shape but equivalent |
| Submit/Remove to layer | `src/render/tactical/` draw-list build | Implemented per-frame, not cached |
| Action → cursor mapping | `src/ui/cursor.rs` | Partially implemented — depth of the switch not confirmed against binary |
| Player-color cursor tint | — | **Not implemented** (cursor currently renders in a fixed palette) |
| Waypoint pulse 8-frame animation | — | **Not implemented** — signature RA2 feel item |
| `DetermineAction` state machine | `src/sim/command_queue.rs` + player-input layer | Partially implemented — the ~60 actions are not all present |
| Init_Clear reset semantics | `World::clear` / scenario-reload paths | Audit needed — ensure all 5 layer-clear calls have Rust equivalents |
| Last-ref-object (auto-pan target) | — | Not implemented |
| Nested `TacticalClass` | `src/render/camera.rs` / tactical render | Not a nested class in Rust; equivalent functionality split across multiple modules |

**Parity-critical gaps for the 99% bar:**

1. **Waypoint cursor pulse** — 8-frame color-cycle animation driven by
   `DAT_00885180` (2-byte + 1-byte RGB table). Very visible, currently missing.
2. **Player-color cursor tint** — default cursor tints to the player's house
   color. Currently fixed palette in Rust.
3. **The full action → cursor enum** — at least 60 distinct cursor states,
   many triggered by shift/ctrl modifiers. Audit against DetermineAction's
   state machine.
4. **Veteran cursor substitution** (`FUN_0070F0B0`) — attack cursor 1 → 0x3E
   when hovering a veteran unit, 5 → 0x3F. Subtle but visible.
5. **Last-ref-object auto-pan** — camera recenters on the last clicked
   object at scenario restore. Affects save-load feel.

---

## 9. Open questions

1. **The 37 DisplayClass-override vtable slots (30-49, 51, 55, 59, 61) beyond
   the ones already named.** Most are probably radar/power-related or
   layer-helper virtuals. Can be enumerated by decompiling slot-by-slot if a
   Rust feature needs one.

2. **Exact layer index → semantic-name mapping** (indices 0-4). Needs
   cross-checking `ObjectClass::vtable[0x78]` returns per subclass (Infantry,
   Unit, Building, Aircraft, Anim, Bullet). Inferred mapping above is
   probably correct but not verified.

3. **The uppercase "minimal constructor" at 0x006AC840.** Purpose unclear —
   possibly a copy-helper or a placement-new variant the compiler emitted.
   Not blocking; main ctor is the lowercase one.

4. **`DAT_008A03F8` sentinel.** Used as a "no-cell" marker in the ctor and
   `SetCursorFromAction`. Probably `{0xFFFF, 0xFFFF}` as a packed CellStruct,
   but not verified.

5. **`DAT_0087F6C8 + 0x174`** — the current cursor palette / SHP cache
   pointer. Referenced in both cursor-animation paths. Probably
   `g_MouseClass->cursor_shp`.

6. **The `DAT_00885180` color table format.** Assumed 3-byte-per-entry packed
   RGB indexed by `(player_color * 8 + frame)`, producing the pulse. Exact
   palette size and bounds not confirmed — could be 8 colors × 8 frames = 64
   entries, or 0xC colors × 8 = 96 entries. Needs a raw-memory dump to nail
   down.

---

## 10. Sources

### Ghidra addresses decompiled (9)

- **0x004A8730** — DisplayClass main constructor (lowercase)
- **0x006AC840** — DisplayClass minimal/copy constructor (uppercase)
- **0x004A88C0** — DisplayClass::Init_Clear
- **0x006922E0** — DisplayClass::Dispatch (trivial wrapper)
- **0x00692F30** — Hover handler (the real cursor-update pipeline entry)
- **0x00692610** — DisplayClass::DetermineAction (action state machine)
- **0x004AAE90** — DisplayClass::SetCursorFromAction (cursor icon selector)
- **0x0070F0B0** — Veteran cursor substitution helper
- **0x004A9720** — DisplayClass::Submit_Object
- **0x004A9770** — DisplayClass::RemoveFromLayer

### Raw memory reads

- **0x007E6114** (256 bytes) — DisplayClass vtable (64 slots)
- **0x008A0360** (136 bytes) — g_DisplayLayers array header + 5 entries

### Globals resolved

- `g_DisplayLayers = 0x008A0360` — 5 × DisplayLayerEntry (24 bytes each)
- `vtable_DisplayClass = 0x007E6114`
- `g_PLACE_SHP` — building-placement ghost SHP pointer (zeroed in ctor)
- `DAT_008A03F8` — "no-cell" sentinel coord
- `DAT_00880998/999/99A/99B/99C` — UI mode flag register (deploy/gather/sell/waypoint/place)
- `DAT_008809A0` — UIModeLock index (-1 = unlocked)
- `DAT_00A8E334` — UIModeHandler array (dispatched via vtable[0x6C])

### Existing docs referenced (not re-covered)

- `GSCREEN_RTACTICAL_GHIDRA_REPORT.md` — top-level orchestration, g_DisplayChain/g_Tactical
- `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md` — 3-pass render architecture
- `BANDBOX_SELECTION_GHIDRA_REPORT.md` — drag-rectangle selection
- `SELECTION_{LIFECYCLE,BRACKETS,GATES,SYSTEM}_GHIDRA_REPORT.md` — selection state
- `MAPCLASS_GHIDRA_REPORT.md` + follow-up — inherited base class
- `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md` — tick-loop orchestration
- `LAYER_CLASS_GHIDRA_REPORT.md` — underlying LayerClass<T>
