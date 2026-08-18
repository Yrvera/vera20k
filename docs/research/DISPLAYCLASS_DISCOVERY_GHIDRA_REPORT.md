# DisplayClass — Discovery Report (2026-04-24)

First-pass survey of DisplayClass in gamemd.exe. DisplayClass is the
third class in the display hierarchy (GScreenClass → MapClass →
DisplayClass → RadarClass → …), starting at MapClass+0x1174.

**Confidence:** MEDIUM (overview only) to HIGH for the structural
facts (vtable layout, constructor fields, key method addresses).

**Active in YR:** Yes — owns the tactical-view render pipeline, all
mouse interaction, and object layering for draw.

**Scope:** This is a *discovery* report, not a full Ghidra dive.
Goal: enumerate the class's surface area so future focused reports
can target specific subsystems (band-box selection, cursor dispatch,
layer ordering, etc.). Full decompilation of all ~40 DisplayClass
methods is out of scope.

---

## 1. Overview

DisplayClass adds the *interactive tactical view* on top of the
passive map infrastructure that MapClass owns. Where MapClass owns
cells, zones, shroud, and crate state, DisplayClass owns:

- **Layered render submission** — which objects draw into which of
  the N display layers (ground, objects, overlays, effects, etc).
- **Mouse dispatch** — cursor tracking, band-box selection, click
  resolution, hover state.
- **Action determination** — the "what would happen if I clicked
  here?" logic that drives cursor-shape changes and context commands.
- **Dirty-rect invalidation** — knowing which screen regions need
  redrawing after a state change.

The single global instance is the same `0x0087F7E8` object as
MapClass (they share storage in one mega-struct). DisplayClass's
fields occupy bytes `+0x1174..+0x11E0+` within that mega-struct.

---

## 2. Class layout (partial — what the constructor touches)

From `DisplayClass__constructor` at `0x4A8730`. All offsets are byte
offsets from the mega-struct base (`0x87F7E8`). The constructor
calls MapClass's constructor first, then initializes DisplayClass
fields.

| Offset | Size | Init value | Likely purpose |
|--------|------|------------|----------------|
| +0x1174 | 4 | `DAT_008A03F8` (null-cell sentinel, likely `0xFFFFFFFF`) | "last ref cell" packed CellStruct |
| +0x1178 | 2 | 0 | (short) |
| +0x117A | 2 | 0 | (short) |
| +0x117C | 4 | 0 | also set in Init_Clear; frame/tick counter? |
| +0x1180 | 1 | 0 | |
| +0x1181 | 1 | 0 | |
| +0x1182 | 4 | null-cell sentinel | another packed CellStruct |
| +0x1186 | 2 | 0 | |
| +0x1188 | 2 | 0 | |
| +0x118C | 4 | 0 | |
| +0x1190 | 4 | 0 | |
| +0x1194 | 4 | 0 | |
| +0x1198 | 4 | `0xFFFFFFFF` | "ref index, -1 = none" pattern |
| +0x119C | 1 | 0 | |
| +0x11A0 | 4 | 0 | |
| +0x11A4 | 4 | 0 | also set in Init_Clear (=0) |
| +0x11A8 | 4 | 0 | also set in Init_Clear (=0) |
| +0x11AC | 4 | `0xFFFFFFFF` | also set in Init_Clear (=-1) — ref index |
| +0x11B0 | 1 | 0 | also set in Init_Clear — flag |
| +0x11B1 | 1 | 0 | also set in Init_Clear — flag |
| +0x11B2 | 1 | 0 | also set in Init_Clear — flag |
| +0x11B3 | 1 | 0 | |
| +0x11B4 | 1 | 0 | |
| +0x11B8 | 4 | `0xFFFFFFFF` | also set in Init_Clear (=-1) |
| +0x11BC | 4 | 0 | |
| +0x11C0..+0x11D4 | various | zero/flag | selection-state/cursor-state fields |
| +0x11D4..+0x11E0 | 12 | 0 | more state fields |

The constructor initializes through at least `+0x11E0`. The
destructor and other methods likely touch fields beyond that — this
table only captures what the constructor writes.

**Last line of constructor:** `*param_1 = &vtable_DisplayClass` —
overwrites the MapClass vtable pointer with DisplayClass's vtable at
`0x7E6114`.

**Globals initialized:**
- `g_PLACE_SHP = 0` — placement SHP reference
- `_DAT_008A072C = 0` — SHP/tile cache
- `_DAT_008A0418 = 0` — cursor/selection state

---

## 3. Vtable (`0x7E6114`)

Dumped 512 bytes. The first 30 slots are **inherited directly from
MapClass** with most slots pointing at the same code. The six slots
DisplayClass overrides in the inherited range are:

| Slot | Address | Name | Override of |
|------|---------|------|-------------|
| 4 | `0x4AEBF0` | scalar deleting destructor (DisplayClass) | MapClass slot 4 `0x588BF0` |
| 5 | `0x4A8850` | DisplayClass::Init_Alloc (loads PLACE SHP + sets up view bounds) | MapClass slot 5 `0x565800` |
| 7 | `0x4A88C0` | **DisplayClass::Init_Clear** — clears state fields, iterates all DisplayLayers to clear | MapClass slot 7 `0x5659F0` |
| 8 | `0x4A8930` | DisplayClass override (likely Init) | GScreenClass base |
| 10 | `0x4A9700` | DisplayClass override (Submit_Object helper chain?) | GScreenClass base |

Slots **30 onward are DisplayClass-specific** additions. The vtable
extends at least through slot ~70, but the later portion likely
contains secondary-inheritance vtable fragments (MSVC MI layout) —
slots 50 and 54 contain `0x007FFD08`/`0x007FFD38` which are RTTI
COL pointers in `.rdata`, not function pointers. So the **primary**
DisplayClass vtable is roughly slots 0–49.

### DisplayClass-specific vtable slots (30–49, primary)

| Slot | Address | Labeled name |
|------|---------|--------------|
| 30 | `0x4AE6F0` | (accesses `g_DisplayLayers`) |
| 31 | `0x4AE720` | (accesses `g_DisplayLayers`) |
| 32 | `0x4ACE70` | — |
| 33 | `0x4AE4F0` | — |
| 34 | `0x4AE6B0` | — |
| 35 | `0x4AEAD0` | — |
| 36 | `0x4A9890` | — |
| 37 | `0x4A9CA0` | — |
| 38 | `0x4A9DD0` | — |
| 39 | `0x4AA050` | — |
| 40 | `0x4C9150` | abstract placeholder |
| 41 | `0x4A9840` | — |
| 42 | `0x4A8960` | — |
| 43 | `0x40D250` | ctor helper |
| 44 | `0x4AAD20` | — |
| 45 | `0x4AC310` | — |
| 46 | `0x4AAE90` | **DisplayClass::SetCursorFromAction** |
| 47 | `0x4AC380` | **DisplayClass::BandBox_MouseMove** |
| 48 | `0x4AB9B0` | **DisplayClass::BandBox_LeftUp** |
| 49 | `0x4AAD30` | (paired with 44) |

---

## 4. Key labeled methods (from `search_functions("Display")`)

These labels already exist in the Ghidra project — use them as
research anchors for targeted deep dives.

| Address | Name | Purpose (inferred) |
|---------|------|--------------------|
| `0x4A8730` | `DisplayClass__constructor` | — |
| `0x4A8850` | `DisplayClass__Init_Alloc` (slot 5) | — |
| `0x4A88C0` | `DisplayClass__Init_Clear` (slot 7) | Reset view-state fields, iterate g_DisplayLayers clearing each |
| `0x4A9720` | `DisplayClass__Submit_Object` | Add an object to its appropriate DisplayLayer after determining layer via vtable+0x78; removes from previous layer first |
| `0x4A9770` | `DisplayClass__RemoveFromLayer` | Counterpart to Submit_Object |
| `0x4AAE90` | `DisplayClass__SetCursorFromAction` (slot 46) | Pick cursor SHP for a given action code |
| `0x4AB9B0` | `DisplayClass__BandBox_LeftUp` (slot 48) | Complete band-box selection on mouse release |
| `0x4AC380` | `DisplayClass__BandBox_MouseMove` (slot 47) | Update band-box rect during drag |
| `0x4AEB10` | `DisplayClass__GetLastRefObject` | — |
| `0x4AEB30` | `DisplayClass__SetLastRefObject` | — |
| `0x6922E0` | `DisplayClass__Dispatch` | Top-level input/command dispatch (calls `FUN_00692F30` then `CommandBar_Dispatch`) |
| `0x692610` | `DisplayClass__DetermineAction` | Compute "what action does a click here perform?" — drives cursor and context menu |
| `0x6AC840` | `DisplayClass__Constructor` (second labeled) | Unclear — possibly a post-init hook; worth investigating separately |

**Missing labels worth adding** in a future Ghidra session (after
decompilation confirms purpose):
- Slots 30–45 in the vtable (mostly untyped)
- Display layer management internals
- Hover-state transitions

---

## 5. Global state owned by DisplayClass

### `g_DisplayLayers`

A contiguous array of `DisplayLayerEntry` ending at `0x008A03D8`.
The `Init_Clear` loop iterates from the start until past the end
address, calling `vtable[3]` (likely `Clear()`) on each entry.

Each entry has a vtable pointer as its first field, indicating
polymorphic DisplayLayer implementations.

**Usage pattern** seen in `Submit_Object`:
```
iVar2 = (**(code **)(*obj + 0x78))()  # obj->GetLayerIndex()
if (iVar2 != -1):
    DynamicVector__Insert(obj, iVar2 == 2)
    obj->layer_index = iVar2
```

So each drawable object advertises its layer via `vtable+0x78`
(`GetLayerIndex`), and the display system inserts it into that
layer's DynVec. Layer index 2 triggers a special "high-priority"
insertion (head vs tail of the DynVec).

### `g_PLACE_SHP`, `DAT_008A072C`

SHP resource handles loaded in `Init_Alloc` from the MIX files.
Used during placement preview (ghost cursor when placing a
building).

### `DAT_008A0418`

Cursor/selection state byte. Reset to 0 in the constructor. Likely
a small state machine counter.

### `DAT_008A03F8`

The "null cell coord" sentinel used throughout MapClass and
DisplayClass to mark "unset cell reference". Almost certainly
`0xFFFFFFFF` = packed `CellStruct(-1, -1)`.

---

## 6. What DisplayClass is responsible for (summary)

Based on the method set:

1. **Display layer management** — inserting/removing objects into
   per-layer dynamic vectors, iterating layers for draw.
2. **Band-box selection** — `BandBox_MouseMove` / `BandBox_LeftUp`,
   with supporting infra at slots 44–49.
3. **Cursor shape decision** — `SetCursorFromAction`, driven by
   `DetermineAction`.
4. **Action dispatch** — `DetermineAction` computes the action code
   for a click; `Dispatch` routes it to the command bar.
5. **Scenario state init** — `Init_Clear` / `Init_Alloc` reset the
   view on scenario load.
6. **Dirty-rect / redraw coordination** — infrastructure lives
   higher up the hierarchy (MapClass's `MarkNeedsRedraw`, slot 14).

The actual tactical *draw* (base terrain, objects, shadows, etc.)
lives in the `Tactical_layer_*` family of functions starting at
`0x6D3470` — called from DisplayClass's draw pipeline but not owned
directly by DisplayClass.

---

## 7. Rust parity status

Rust has no single class that mirrors DisplayClass. Its
responsibilities are spread across:

| DisplayClass responsibility | Rust location (approximate) |
|-----------------------------|------------------------------|
| Layered render submission | `src/render/` (tactical render pipeline — not a Submit-Object model but a per-layer walk) |
| Band-box selection | `src/app_entity_pick.rs` (band-box logic) |
| Cursor shape from action | `src/app_cursor.rs` |
| Action determination | `src/app_commands.rs` / `src/app_context_order.rs` |
| Click dispatch | `src/app_sim_tick.rs` (via `screen_point_to_world_cell`) |
| Scenario init state | `src/sim/world/mod.rs` |
| Dirty-rect invalidation | not equivalent — Rust redraws every frame via wgpu |

Rust doesn't use a "Submit_Object into per-layer DynVec" model — it
builds draw lists per frame from the world state directly. That's
a valid architectural choice for a modern GPU renderer, but it
means the *layer-ordering contract* that DisplayClass encodes is
not explicitly preserved. Any parity bugs with draw ordering
(selection brackets above vs below chrome, pips behind cameos,
etc.) should trace back to comparing Rust's per-frame draw order
against DisplayClass's layer indices.

---

## 8. Recommended follow-up investigations (separate reports)

Each of these is ~1 focused Ghidra session:

1. **DisplayClass::DetermineAction (0x692610)** — the "click →
   action code" lookup. High-value for parity because every cursor
   shape and context-sensitive command flows through here. Should
   catalog all action codes and their conditions.

2. **DisplayClass layer system** — enumerate all `DisplayLayerEntry`
   instances, document the per-layer DynVec contents, and nail down
   the draw order for a frame.

3. **DisplayClass::BandBox_\*** — the selection-rect state machine.
   Small but detail-dense (single-vs-multi-click, double-click-to-
   select-type, ctrl/shift modifiers).

4. **DisplayClass::Init_Alloc inner** — what view-bounds computation
   happens on load; how does it relate to the MapClass `[Map]` Size
   parsing.

5. **The 15–20 unlabeled primary-vtable slots (30–45)** — bulk-
   decompile and name. Many are probably per-frame hooks or
   per-event handlers that don't need deep study once labeled.

6. **Secondary-inheritance vtable fragments** — confirm which base
   interface(s) DisplayClass implements beyond MapClass. COL
   pointers at slots 50 and 54 suggest 2+ secondary bases.

---

## Sources

### Newly decompiled
- `0x4A8730` DisplayClass constructor
- `0x4A8850` DisplayClass::Init_Alloc (vtable slot 5)
- `0x4A88C0` DisplayClass::Init_Clear (vtable slot 7)
- `0x4A9720` DisplayClass::Submit_Object
- `0x6922E0` DisplayClass::Dispatch
- `0x4AAE90` DisplayClass::SetCursorFromAction (partial — labeled only)

### Raw memory
- `0x7E6114, 512 bytes` — DisplayClass vtable dump
- `0x7FFD08, 24 bytes` — RTTI COL embedded in vtable tail

### Function search
- `search_functions("Display")` — 17 labeled methods
- `get_xrefs_to(0x008A03D8)` — confirms g_DisplayLayers array extent

### Referenced docs
- `MAPCLASS_GHIDRA_REPORT.md` (inheritance hierarchy + shared
  mega-struct model)
- `MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md` (vtable overlay layout)
- `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` (corrected MapClass
  vtable size of 30 slots, which this report relies on for the
  inherited-slot count)
