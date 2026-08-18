# Gate #1 Resolution — `MapClass__IsRectInPlayfield` corner formula

**Verdict: CLOSED** for the corner-formula question (a/b/c/e). The far-corner offset, corner
set, corner order, and degenerate-size behavior are fully resolved from the binary. The
underlying bound *source* (d) is resolved to exact `MapClass` field offsets and the exact
inequality form, but the **human names** of those fields (MapRect-vs-visible-bounds) are
left as a labeled-but-not-renamed note (see §4): the formula is exact regardless of naming.

**One-sentence resolved fact:** `IsRectInPlayfield` tests exactly four corners — NW `(x,y)`,
NE `(x+w-1, y)`, SW `(x, y+h-1)`, SE `(x+w-1, y+h-1)` — in that order, each via
`Is_Cell_In_Playfield`, using **inclusive** `x+width-1` / `y+height-1`; the containment test
itself is an **isometric diamond** test on the cell's `x+y` sum and `x-y` difference against
`MapClass` bound fields, **not** a rectangular `0 <= x < 512` array-index test.

---

## 1. Confirmed function identity

- `0x00578390` = `MapClass__IsRectInPlayfield` — confirmed via `get_function_by_address 0x00578390`
  (label) AND body shape: four sequential `CALL 0x00578460` corner tests, AND-chained, returns
  1 only if all four pass (`disassemble_function 0x00578390`).
- Sole caller: `CellRect__CheckOccupancy @ 0x00586780`, calling site `0x0058687B`
  (`get_xrefs_to 0x00578390` → "From 0058687b ... [UNCONDITIONAL_CALL]"; `get_function_callers 0x00578390`).
- Callee `0x00578460` = `MapClass__Is_Cell_In_Playfield` — confirmed via
  `get_function_by_address 0x00578460`; it is the central playfield-containment primitive
  (`get_function_callers 0x00578460` lists 80+ live YR callers: A*, locomotors, Unlimbo,
  Scatter, harvest/weed checks, cursor/action, radar bounds, etc.).
- Sibling wrapper `MapClass__IsCoordsInPlayfield @ 0x005785F0` converts lepton coords to cells
  (`>>8`) then calls `Is_Cell_In_Playfield(&cell, 1)` (`decompile_function 0x005785F0`) — confirms
  the third arg is the "apply height/slope extension" flag and that `1` is the live value.

Signatures (reconstructed from bodies):
```
char __thiscall MapClass__IsRectInPlayfield(MapClass* this /*ECX=EDI*/, CellRect* rect, int height_flag)
uint __thiscall MapClass__Is_Cell_In_Playfield(MapClass* this /*ECX*/, short* cellXY, char height_flag)
```
`CellRect` is four 32-bit fields: `+0x0 x`, `+0x4 y`, `+0x8 width`, `+0xC height`; only the low
16 bits of x/y are used as signed cell coords (matches `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS`
§2). `CheckOccupancy` passes `height_flag = 1` (decompile: `IsRectInPlayfield(rect, 1)`).

---

## 2. (a) Exact far-corner formula — INCLUSIVE `x+width-1` / `y+height-1`

Verified from `disassemble_function 0x00578390`. Each far edge is built with `LEA ... + -0x1`:

- NE x-coord: `LEA EDX,[EAX + ECX*1 + -0x1]` where `EAX=rect.width (+0x8)`, `ECX=rect.x (+0x0)` (`0x005783CC`) → `x + width - 1`.
- SW y-coord: `LEA EDX,[EAX + ECX*1 + -0x1]` where `EAX=rect.y (+0x4)`, `ECX=rect.height (+0xC)` (`0x005783FD`) → `y + height - 1`.
- SE: both `x+width-1` (`0x0057841C`) and `y+height-1` (`0x0057842B`).

Decompile mirror: `(short)puVar1[2] + -1 + (short)*puVar1` (width+x-1) and
`(short)puVar1[1] + -1 + (short)puVar1[3]` (y+height-1). **Inclusive far corner — NOT `x+width`.**

## 3. (b)/(c) Corners tested, order, and operators

(b) Exactly **four** corners, AND-chained, short-circuit on first failure (`JZ 0x00578452`
after each `TEST AL,AL`). Order is fixed:

1. **NW** `(x, y)` — `0x005783A0..0x005783B9`
2. **NE** `(x+width-1, y)` — `0x005783C6..0x005783E5`
3. **SW** `(x, y+height-1)` — `0x005783EE..0x0057840D`
4. **SE** `(x+width-1, y+height-1)` — `0x00578416..0x0057843B`

Returns `1` (`MOV EAX,0x1`, `0x00578446`) only if all four pass; else `XOR EAX,EAX` (`0x00578454`).

(c) The comparison operators live in `Is_Cell_In_Playfield @ 0x00578460`, NOT in the rect
function. The per-cell test (`disassemble_function 0x00578460`, decompile mirror) is, with
`sx = (signed short)cellXY[0]`, `sy = (signed short)cellXY[1]`, and `h` the height extension
(see §4/§5):

```
pass  iff  ( base + LOW  <  sy + sx )                      // strict <   (JLE fails)  0x005784E1
       &&  ( sy + sx     <= base + HIGH )                  // inclusive <= (JG fails)  0x005784FE
       &&  ( sx - sy     <  RIGHT )                         // strict <   (JGE fails)  0x00578516
       &&  ( sy - sx     <  LEFT  )                         // strict <   (JGE fails)  0x00578521
```

So it is NOT a simple `>= 0` lower bound and `< width` upper bound on raw x/y. It is a diamond:
the **sum** `sx+sy` must lie in a half-open band `(low, high]` (low exclusive, high inclusive),
and the two **differences** `sx-sy` and `sy-sx` must each be strictly `<` their bound. There is
no explicit `>= 0` guard on raw x/y; the four diagonal inequalities define the playable diamond.

## 4. (d) Playfield-bound SOURCE — `MapClass` fields, NOT the 512 cell array

The bounds come from `MapClass` instance fields (the `this`/`ECX` pointer = `param_1`), read at:
`+0xF4`, `+0xFC`, `+0x100`, `+0x104`, `+0x108`. The fixed `0x200` (512) stride and `0x3FFFF`
limit appear ONLY in the cell-array index used to *fetch the CellClass* for the height byte —
they are NOT the playfield bound. Exact bound expressions (decompile + asm):

- `base = *(int*)(this+0xF4)` (`0x005784D3`).
- LOW edge of sum band: `base + (this+0x100)*2 + h` (`0x005784CD..0x005784DF`).
- HIGH edge of sum band: `base + 2 + ((this+0x108)+(this+0x100))*2 + h` (`0x005784E5..0x005784F7`).
- RIGHT diff bound: `((this+0x104)+(this+0xFC))*2 - base` (`0x00578500..0x00578512`).
- LEFT diff bound: `base - (this+0xFC)*2` (`0x0058050E`-region: `LEA ECX,[ESI+ESI]; SUB EAX,ECX`, `0x0057851A..0x0057851F`).

These five fields are the map's playfield rectangle in the engine's internal map coord frame
(origin/offset at `+0xF4`, plus width/height-like extents at `+0xFC/+0x100/+0x104/+0x108`),
doubled because the iso map packs two cell-axes. The fixed 512-wide array
(`g_CellArray_Base @ 0x0087F924`, read at `0x00578489`; reads 0 in static dump — runtime
pointer) is only the storage indexer, with out-of-range/null → dummy cell `DAT_00ABDC50`
(`0x00578496..0x0057849D`). **Source = MapClass bound fields; the 512 array is unrelated to
the bound.** (Field human-names left UNVERIFIED — see §7; the formula is exact regardless.)

## 5. height_flag (`param_3`) and the `+0x11B`/`+0x11C` extension

When `height_flag != 0` (the live value `1`), the cell at `sy*0x200+sx` is fetched and:
- `h = (signed char) cell[0x11B]` (cell level byte) — `0x005784A8`.
- If `cell[0x11C] != 0` (slope/ramp byte) AND `sx+sy < base + 4 + (this+0x100)*2 + h`, then
  `h += 1` (`0x005784AF..0x005784CC`). This extends the diamond for sloped/elevated cells.

When `height_flag == 0`, `h = 0` and the cell fetch/height block is skipped entirely
(`JZ 0x005784CD`), so the diamond uses the flat bounds. `CheckOccupancy` always passes `1`, so
the rect-in-playfield path is the **height-aware** diamond.

## 6. (e) Degenerate 0-width / 0-height behavior

`IsRectInPlayfield` does **no** loop and does **not** special-case `width<=0`/`height<=0`.
With `width=0`: NE x = `x+0-1 = x-1`; SE x = `x-1`. With `height=0`: SW y = `y-1`; SE y = `y-1`.
So a 0-width/0-height rect tests corners *outside/behind* the nominal origin (e.g. `(x-1,y-1)`),
and all four must still satisfy the diamond test. It is fully reachable: `CheckOccupancy`'s
scan loop skips when dims are nonpositive but **still falls through** to
`IsRectInPlayfield(rect, 1)` (`CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS` §3.2 / OQ-9). A
genuinely empty rect therefore evaluates the diamond at decremented coords, not as a no-op.

## 7. YR-active vs TS-legacy

- `IsRectInPlayfield`, `Is_Cell_In_Playfield`, and the height extension are **live in YR**:
  sole rect caller `CheckOccupancy` is reached by `Find_Nearby_Passable_Cell` and AI site
  placement (`CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS` §5); `Is_Cell_In_Playfield` has 80+
  live callers (`get_function_callers 0x00578460`). No SpecialFlags/Fog/subterranean gate.
- The `+0x11C` slope bump is normal RA2/YR ramp terrain, not a TS-only path.

**UNVERIFIED (YELLOW):** human names for `MapClass +0xF4/+0xFC/+0x100/+0x104/+0x108`
(MapRect origin vs visible-cell bounds). Not needed for the formula; if a future task needs the
name, decode the writers of these fields (`MapClass` init / `RecalcCellsAndRebuildZones @ 0x00586990`).

## 8. Rust handoff

cell-validation plan **T3.5 (`rect_in_playfield` corner formula)** should implement: test the
four corners NW `(x,y)`, NE `(x+w-1,y)`, SW `(x,y+h-1)`, SE `(x+w-1,y+h-1)` in that order,
short-circuit AND, **inclusive** `w-1`/`h-1`; and the corner predicate must be the **isometric
diamond** test of §3(c) on `(sx+sy)` half-open band `(low, high]` and strict `(sx-sy) < RIGHT`,
`(sy-sx) < LEFT`, using the five `MapClass` bound fields of §4 (doubled, `+base` origin) — NOT a
`0 <= x < 512 && 0 <= y` rectangular test. Include the height-flag extension (§5) since the
live caller passes `1`. The currently `#[ignore]`'d test
`occupancy_zero_size_rect_still_runs_playfield_corners` should assert that a 0-size rect still
evaluates corners at `(x-1,y-1)`-style decremented coords (NOT a no-op / NOT auto-pass) per §6.

## Sources

- `get_function_by_address 0x00578390`, `0x00578460`; `decompile_function 0x00578390`,
  `0x00578460`, `0x005785F0`; `disassemble_function 0x00578390`, `0x00578460`.
- `get_function_callers 0x00578390` (sole: CheckOccupancy), `0x00578460` (80+ live YR callers);
  `get_xrefs_to 0x00578390` (From 0058687b CheckOccupancy).
- `read_memory 0x0087F924` (g_CellArray_Base; 0 in static dump = runtime pointer).
- Prior docs: `pathfinding/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md` (§3.2, OQ-8/9),
  `pathfinding/CELLRECT_CHECKPASSABILITY_0056E7C0_FULL_ARG_DECODE_GHIDRA_REPORT.md`,
  `skirmish-ui/CELLRECT_CHECKPASSABILITY_START_RECTANGLE_CALLER_SLICE_GHIDRA_REPORT.md`.
