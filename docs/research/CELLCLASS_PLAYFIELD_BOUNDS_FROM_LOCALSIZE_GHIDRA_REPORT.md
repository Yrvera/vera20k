# Playfield Diamond Bounds from Size/LocalSize — Ghidra Research Report

**Address(es):** setters `MapClass__Resize 0x00565c10` (base `+0xf4`) and
`Read_Map_Section_And_IsoMapPacks 0x004ad76b` (extents `+0xfc/+0x100/+0x104/+0x108`, via
`INIClass::ReadRect 0x00527cc0`); consumer `MapClass__Is_Cell_In_Playfield 0x00578460`.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** how the five MapClass playfield-diamond fields (`+0xf4/+0xfc/+0x100/+0x104/+0x108`)
are populated from the `[Map] Size=` and `[Map] LocalSize=` INI values on a normal map load.
**Non-Scope:** the consumer diamond predicate itself (already verified bit-exact in
`CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md`); the editor-LocalSize ↔ cell-(X,Y) iso
projection geometry; the "resize visible area" trigger action.
**Confidence:** HIGH (all five field stores decompiled directly — not inferred).
**Active in YR:** Yes (the map loader and `MapClass::Resize` run for every skirmish + campaign map).

## 1. Overview

The engine's isometric playfield diamond is defined by five `MapClass` fields read by
`Is_Cell_In_Playfield`. This report resolves the previously-UNRESOLVED question of how those five
fields are computed from the map header. Result: **four are the raw `LocalSize` CSV values
(untransformed), and the fifth (`base`) is the `Size` rectangle's width.** No scaling, no iso
transform at store time — the transform lives entirely in the consumer's `*2` / `+2` / `+4` arithmetic
(already documented).

## 2. Key Offsets / Fields (MapClass singleton `g_Map @ 0x0087F7E8`)

| Field | Meaning (consumer role) | Set to | Setter |
|------:|-------------------------|--------|--------|
| `+0xf4` | `base` of the diamond predicate | **`Size.width`** (Size CSV 3rd value) | `MapClass__Resize 0x00565c10` |
| `+0xf8` | (Size.height — used by Resize cell-alloc, **NOT** a diamond field) | `Size.height` (4th) | `MapClass__Resize` |
| `+0xfc` | left extent | **`LocalSize.left`** (1st) | `Read_Map…0x004ad76b` |
| `+0x100` | top extent | **`LocalSize.top`** (2nd) | `Read_Map…` |
| `+0x104` | width extent | **`LocalSize.width`** (3rd) | `Read_Map…` |
| `+0x108` | height extent | **`LocalSize.height`** (4th) | `Read_Map…` |
| `+0xec`, `+0xf0` | (Size.left/top, then **zeroed**) | `0` | `MapClass__Resize` |

## 3. Core Logic — the two setters

### 3a. `base = Size.width` — `MapClass__Resize @ 0x00565c10` (vtable slot `+0x70`)

`Read_Map…` reads `[Map] Size=` (default `{1,1,50,50}`) into a local rect, then calls
`this->vtable[0x70](&SizeRect, 1, 0, 1)`. Slot `+0x70` resolves to `0x00565c10` (verified via
`read_memory 0x007ED474` → `0x00565c10`, which `search_functions` names `MapClass__Resize`). Inside
`Resize`, unconditionally (verified via `decompile_function 0x00565c10`):

```c
*(int*)(this + 0xec) = SizeRect[0];   // Size.left
*(int*)(this + 0xf0) = SizeRect[1];   // Size.top
*(int*)(this + 0xf4) = SizeRect[2];   // Size.width   ← BASE
*(int*)(this + 0xf8) = SizeRect[3];   // Size.height
*(int*)(this + 0xec) = 0;             // Size.left  discarded
*(int*)(this + 0xf0) = 0;             // Size.top   discarded
```

So **`base (+0xf4) = Size.width`**, i.e. the Size CSV 3rd value = `MapHeader.width`. `Size.left`/`top`
are written then immediately zeroed (the diamond is anchored at the map-array origin, not the Size
offset). `+0xf8 = Size.height` exists but is **not** one of the five diamond fields the consumer reads.

### 3b. extents = raw `LocalSize` — `Read_Map_Section_And_IsoMapPacks @ 0x004ad76b`

At the tail of the map loader (verified via `decompile_function 0x004ad76b`):

```c
rect = INIClass__ReadRect(&mapINI, "LocalSize" /*0x00820164*/, default=this+0xec);
*(this + 0xfc)  = rect[0];   // LocalSize.left
*(this + 0x100) = rect[1];   // LocalSize.top
*(this + 0x104) = rect[2];   // LocalSize.width
*(this + 0x108) = rect[3];   // LocalSize.height
RadarClass__ComputeRadarMapBounds(this + 0xfc);
```

`INIClass::ReadRect (FUN_00527cc0 @ 0x00527cc0)` `sscanf`s the value as `"%d,%d,%d,%d"`
(`s__d__d__d__d_00825bbc`) into four ints **in CSV order**, falling back to the default rect on a
missing/short key (verified via `decompile_function 0x00527cc0`). `[Map] LocalSize=` is
`left,top,width,height`, so the four extents are those four values verbatim, no transform.

The `"LocalSize"` string (`0x00820164`) is read in exactly these load-time sites (verified via
`get_xrefs_to 0x00820164`): `Read_Map_Section_And_IsoMapPacks`, `ScenarioClass__Full_Init`,
`CCINIClass__Constructor`, and the map writer — i.e. the normal map-load path.

### 3c. Resulting consumer diamond (flat cell, `h=0`), with fields substituted

Using the already-verified `Is_Cell_In_Playfield 0x00578460` predicate and the field values above
(`base = Size.width = sw`, `lx/ly/lw/lh = LocalSize.left/top/width/height`):

```
sw + 2*ly        <  X+Y  <=  sw + 2 + 2*(ly + lh)     // sum band
2*lx - sw        <  X-Y  <   2*(lx + lw) - sw         // diff band
```

## 4. INI Keys

| Key | Section | Format | Default (if absent) | Feeds |
|-----|---------|--------|---------------------|-------|
| `Size` | `[Map]` | `left,top,width,height` | `1,1,50,50` | `base = width` (+0xf4); `+0xf8 = height` |
| `LocalSize` | `[Map]` | `left,top,width,height` | copies `Size` default rect | `+0xfc/+0x100/+0x104/+0x108` verbatim |

(Both are map-file keys, not `rules(md).ini`.)

## 5. Integration Points

- `Read_Map_Section_And_IsoMapPacks 0x004ad76b` is the core `[Map]` + IsoMapPack loader; runs for every
  map. It calls `MapClass::Resize` (via vtable `+0x70`) with the `Size` rect, then writes the
  `LocalSize` extents directly.
- Consumer: `Is_Cell_In_Playfield 0x00578460` / `IsRectInPlayfield 0x00578390` (placement, find-nearby,
  scroll/radar bounds).
- **Separate path (campaign/scenario):** `FUN_006e21e0` — the only caller is `TriggerAction__Execute
  0x006dd8b0`, i.e. the "change visible map area" **trigger action**. It overwrites
  `+0xfc/+0x100/+0x104/+0x108` from a trigger-supplied rect (`param+0x34..0x40`) and re-runs
  `RecalcAttributes` over all cells + `UpdateBridgeZonesHelper` + radar refresh. Same four fields, but
  driven by a scenario trigger, not the map header. Not part of normal skirmish setup; documented so a
  future reader does not mistake it for the loader.

## 6. Worked Example + Cross-Check (Dustbowl: `Size=70x76`, `LocalSize=2,8,65,62`)

Fields: `base = Size.width = 70`; `lx=2, ly=8, lw=65, lh=62`. Flat terrain ⇒ `h=0`.

- Sum band: `70 + 16 < X+Y <= 70 + 2 + 2*(8+62)` → **`86 < X+Y <= 212`**.
- Diff band: `2*2 - 70 < X-Y < 2*(2+65) - 70` → **`-66 < X-Y < 64`** (i.e. `-65 ≤ X-Y ≤ 63`).
- Center ≈ `(X+Y, X-Y) = (149, -1)` → cell ≈ **(74, 75)** — sensibly centered for a 70×76 map.
- Cell count ≈ 8127 (integer `(X,Y)` in the band, respecting `X+Y ≡ X-Y mod 2`) ≈ **2× the
  `lw·lh = 65·62 = 4030`** rectangle — exactly the expected iso `(sum,diff)` parity doubling. The
  diamond is non-degenerate and area-consistent with the LocalSize playable rectangle.

**Cross-check caveat (honest).** `src/map/terrain.rs`'s LocalSize clip (`LocalBounds::from_header`
`:118`, `build_terrain_grid` `:584`) is a **render-side approximation** in screen-pixel space, with
`TS_INITIAL_HEIGHT=3` / `TS_HEIGHT_ADDITION=5` fudge rows that the binary diamond's exact `+2`/`+4`
constants do not mirror. It is therefore **not a bit-exact oracle** for the cell diamond — it is a
different representation of the same visible region in a different (render) layer. The formula here is
**verified directly from the binary stores** (stronger than matching the render approximation); the
bit-exact runtime equivalence test belongs in the implementation acceptance step, not here.

## 7. Coverage Ledger

| Area | Status | Evidence | What remains |
|------|--------|----------|--------------|
| `base (+0xf4) = Size.width` | verified | `decompile_function 0x00565c10` (unconditional store) | none |
| `+0xfc/+0x100/+0x104/+0x108 = LocalSize.{l,t,w,h}` | verified | `decompile_function 0x004ad76b` + `0x00527cc0` (`sscanf "%d,%d,%d,%d"`) | none |
| vtable `+0x70` → `MapClass::Resize` | verified | `read_memory 0x007ED474` = `0x00565c10` | none |
| `"LocalSize"` reader sites | verified | `get_xrefs_to 0x00820164` | none |
| consumer diamond predicate | verified (prior) | `0x00578460`, bit-exact vs `cell_rect.rs:479` | none |
| `FUN_006e21e0` = resize-area trigger | verified | `get_function_callers` → `TriggerAction__Execute` only | none |
| editor-LocalSize ↔ cell-(X,Y) iso projection | deferred | §6 cell-count parity check | not needed for the field formula; consumer already bit-exact |
| `iStack_e4 == g_Map` in `Read_Map…` | verified-by-consistency | fields equal the absolute `g_Map+0xfc..` written by `FUN_006e21e0` | none |

## 8. Open Questions — Final State

- `[RESOLVED]` base derivation → `base (+0xf4) = Size.width` (evidence: `0x00565c10` store
  `*(this+0xf4)=SizeRect[2]`).
- `[RESOLVED]` four extents → raw `LocalSize.{left,top,width,height}` (evidence: `0x004ad76b` +
  `INIClass::ReadRect 0x00527cc0`).
- `[RESOLVED]` which slot `Read_Map…` invokes → vtable `+0x70` = `MapClass::Resize 0x00565c10`
  (evidence: `read_memory 0x007ED474`).
- `[RESOLVED]` is the path normal-skirmish-active → yes, `Read_Map_Section_And_IsoMapPacks` is the
  core map loader (evidence: reads Theater/IsoMapPack1-5/Size/LocalSize/CellTags).
- `[RESOLVED]` who else writes these fields → `FUN_006e21e0` ("resize visible area" trigger action),
  separate from load (evidence: `get_function_callers` → `TriggerAction__Execute`).
- `[DEFERRED]` exact editor-LocalSize ↔ cell-coord iso projection (the §6 factor-of-2). Category:
  `out-of-scope`. Reason: the field *values* are verified; the projection geometry is already encoded
  (and bit-exact-verified) in the consumer predicate. Next step if pursued: derive the (sum,diff)→cell
  mapping from `iso_to_screen` to prove cell-count equality, only needed if a discrepancy appears.
- `[DEFERRED]` live `read_memory` confirmation of the five field values on a loaded map. Category:
  `needs-runtime-debugger`. Reason: statics read all-zero at rest. Next step: attach to a running
  gamemd, load Dustbowl, `read_memory 0x0087F8DC/0x0087F8E4/0x0087F8E8/0x0087F8EC/0x0087F8F0`.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `base = Size.width`; extents = raw `LocalSize.{l,t,w,h}` | `0x00565c10`, `0x004ad76b`, `0x00527cc0` | missing — `PlayfieldBounds::from_map_header` not yet written; field human-names were UNVERIFIED | `src/sim/cell_rect.rs` `PlayfieldBounds` (`:187`) — its `base`/`off_fc`/`off_100`/`off_104`/`off_108` | `base ← MapHeader.width`; `off_fc ← local_left`; `off_100 ← local_top`; `off_104 ← local_width`; `off_108 ← local_height` (all `i32`, no transform) | unit test: Dustbowl header → diamond `86<X+Y≤212`, `-65≤X-Y≤63`; interior cell (74,75) passes, off-diamond corner (0,0) fails | do NOT scale/iso-transform at construction — the transform is already in `cell_in_playfield_diamond`; do NOT use `Size.left/top` (zeroed) or `Size.height` (not a diamond field) |

**Stale docs / follow-up:** this RESOLVES the `CELLCLASS_MAPCLASS_…STUDY.md` §9 note that the
`PlayfieldBounds` field human-names were UNVERIFIED and the Size/LocalSize derivation unknown. It also
unblocks Tasks 1–2 of `docs/plans/2026-06-04-cellrect-diamond-playfield-wiring-plan.md`. The
`cell_rect.rs:187` field doc-comments ("names UNVERIFIED") can now be updated to the verified meanings:
`base = Size.width`, `off_fc/off_100/off_104/off_108 = LocalSize left/top/width/height`.

## Sources

- Ghidra (read-only): `decompile_function` `0x00565c10` (Resize/base), `0x004ad76b` (map loader),
  `0x00527cc0` (ReadRect), `0x006e21e0` (resize-area trigger); `read_memory 0x007ED474` (vtable slot
  `+0x70`); `get_xrefs_to 0x00820164` ("LocalSize"), `0x0087F8E8`/`0x0087F8F0`/`0x0087F8E4` (field
  writers); `get_function_callers 0x006e21e0`; `search_functions MapClass__`; consumer `0x00578460`
  (prior session, bit-exact).
- INI: `[Map] Size=`, `[Map] LocalSize=` (map files).
- Rust: `src/sim/cell_rect.rs` (`PlayfieldBounds` :187, `cell_in_playfield_diamond` :479);
  `src/map/map_file.rs:103-119` (`MapHeader`); `src/map/terrain.rs:76-145,555-620` (render-side LocalSize
  clip — approximation, not oracle).
