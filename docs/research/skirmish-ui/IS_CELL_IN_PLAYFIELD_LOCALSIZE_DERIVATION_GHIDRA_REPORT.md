# MapClass::Is_Cell_In_Playfield — closed-form derivation in LocalSize/Size terms

Target: `MapClass__Is_Cell_In_Playfield @ 0x00578460`, third argument = 0 (the mode
`GenerateTerrainPreview @ 0x00641140` calls it in). Scope: this predicate, the five
`MapClass` fields it reads (`+0xf4`, `+0xfc`, `+0x100`, `+0x104`, `+0x108`), and the
sites that write those fields (map/scenario load). Preview rasteriser internals,
the RMG generator's own bounds, and unrelated `MapClass` behavior are out of scope.

Target question: what closed-form cell-space test does `Is_Cell_In_Playfield(cell,
param_3=0)` evaluate, expressed only in terms of `[Map] Size=` and `LocalSize=`, with
exact inclusive/exclusive boundaries — and is the Rust port's `LocalBounds` substitute
equivalent to it?

Non-goals: `GenerateTerrainPreview`'s pixel-blit passes, the RMG generator's internal
terrain-painting diamond, `MapClass::Resize`'s cell-reallocation bookkeeping beyond the
one write of interest, `RadarClass::ComputeRadarMapBounds` beyond confirming field identity.

Evidence needed to mark COMPLETE: assembly-cited write sites for all five fields;
assembly-cited comparison operators (JL/JLE/JG/JGE) for all four half-plane tests;
confirmation that `GenerateTerrainPreview`'s two call sites pass `param_3=0`; a
verdict on Rust-port equivalence backed by closed-form algebra, not prose.

Stop conditions: once the five field write sites and the four comparison directions
are each pinned to a cited instruction address, and the equivalence verdict is
algebraically decided — stop; do not expand into the generator or the rasteriser.

## Field identity (this = `g_Map`, the global at `0x0087f7e8`)

`Read_Map_Section_And_IsoMapPacks @ 0x004ace70` is called as
`MOV ECX,0x87f7e8; CALL 0x004ace70` from `ScenarioClass__Full_Init`
(verified via get_assembly_context 0x006879ff), so `this` inside it is the same global
object referenced by `Is_Cell_In_Playfield`'s `ECX` and by
`RadarClass__ComputeRadarMapBounds`'s `this` (which reads the identical offsets
`+0xf4/+0xfc/+0x100/+0x104` and itself calls `MapClass__Is_Cell_In_Playfield(&cell,1)` —
verified via decompile_function 0x00654490).

- `+0xfc` = LocalSize.X (left, cells)
- `+0x100` = LocalSize.Y (top, cells)
- `+0x104` = LocalSize.Width (cells)
- `+0x108` = LocalSize.Height (cells)
- `+0xf4` = **full map** Size.Width (cells) — NOT part of the LocalSize rect
- `+0xf8` = full map Size.Height (cells) (read by the predicate's D-bound math only via `+0xf4`; `+0xf8` isn't touched by `Is_Cell_In_Playfield` param_3=0, confirmed by the disassembly below)

### `+0xfc/+0x100/+0x104/+0x108` write site — direct, non-virtual

Inside `Read_Map_Section_And_IsoMapPacks` (disassemble_function 0x004ad76b range):

```
004ad75f: LEA ESI,[EAX + 0xfc]          ; ESI = this+0xfc (destination rect)
004ad765: ADD EAX,0xec                   ; EAX = this+0xec (default rect, see below)
004ad76b: PUSH 0x820164                  ; "LocalSize" (string bytes confirmed: read_memory 0x820164 -> "LocalSize\0")
004ad77a: CALL 0x00527cc0                ; generic "parse Rect from INI, else use default" helper
004ad784: MOV dword ptr [ECX],EDX        ; this+0xfc  = result.X
004ad789: MOV dword ptr [ECX+4],EDX      ; this+0x100 = result.Y
004ad78f: MOV dword ptr [ECX+8],EDX      ; this+0x104 = result.W
004ad795: MOV dword ptr [ECX+0xc],EAX    ; this+0x108 = result.H
```
Verified via disassemble_function 0x004ad76b (Read_Map_Section_And_IsoMapPacks). `Active in
YR: Yes` — this is the unconditional scenario/map-load path (`ScenarioClass__Full_Init`),
reachable on every skirmish/multiplayer map load, not a TS-only or editor-only branch.

`FUN_00527cc0` decompiled (decompile_function 0x00527cc0): it does
`CRT__sscanf(value, "%d,%d,%d,%d", &r0,&r1,&r2,&r3)` against the format string at
`0x825bbc` (bytes confirmed via read_memory 0x825bbc = `"%d,%d,%d,%d\0"`), falling back
verbatim to the caller-supplied default rect (also 4 ints, same order) if the key is
absent/unparsable. Field order is therefore **(X, Y, Width, Height)**, matching the
standard `Key=X,Y,W,H` INI convention — confirms `param_2[0..3]` = X,Y,W,H everywhere
in this report.

The **default** rect passed for "LocalSize" when the key is absent is `this+0xec`
(the FullSize rect written by `MapClass::Resize`, see next section) — i.e. a map with
no `LocalSize=` key defaults its local/playable rect to the full map rect. `Active in
YR: Conditional` (only exercised when a map omits `LocalSize=`; retail YR maps always
carry the key, but the fallback path is live code, not dead).

### `+0xf4/+0xf8` write site — inside `MapClass::Resize @ 0x00565c10`

`Read_Map_Section_And_IsoMapPacks` first reads INI key **"Size"** (string bytes
confirmed: read_memory 0x820178 = `"Size\0"`) with default `(1,1,50,50)` via the same
`FUN_00527cc0` helper (disassemble_function 0x004ace70, `PUSH 0x820178` at `0x004aceac`),
then invokes a virtual call at vtable slot `+0x70` on `this` with `(&rect,1,0,1)`
(`0x004acf0d: CALL dword ptr [EAX + 0x70]`). Parameter shape (`rect*, bool, u8, bool`)
matches `MapClass::Resize(this, Rect* param_2, int* param_3, u8 param_4, char param_5)`
exactly, and the write pattern is unambiguous regardless of caller identity:

```
005662bc: MOV EDI,dword ptr [EDX]        ; EDI = param_2[0] = X
005662be: MOV dword ptr [ESI],EDI        ; this+0xec = X
005662c0: MOV EDI,dword ptr [EDX + 0x4]  ; EDI = param_2[1] = Y
005662c3: MOV dword ptr [ESI + 0x4],EDI  ; this+0xf0 = Y
005662c6: MOV EDI,dword ptr [EDX + 0x8]  ; EDI = param_2[2] = Width
005662c9: MOV dword ptr [ESI + 0x8],EDI  ; this+0xf4 = Width   <-- the predicate's F4
005662cc: MOV EDX,dword ptr [EDX + 0xc]  ; EDX = param_2[3] = Height
005662cf: MOV dword ptr [ESI + 0xc],EDX  ; this+0xf8 = Height
005662d8: MOV dword ptr [EAX],EBP        ; this+0xec = 0   (EBP=0, origin X forced to 0)
005662e1: MOV dword ptr [EBX + 0xf0],EBP ; this+0xf0 = 0   (origin Y forced to 0)
```
Verified via disassemble_function 0x00565c10 (`MapClass::Resize`). So `+0xf4` = the
**full map Size Width** (in cells) from `[Map] Size=`, and the map's origin
(`+0xec/+0xf0`) is always normalized to `(0,0)` regardless of what `Size=`'s X,Y
components said. `Active in YR: Yes` — same unconditional load path as above.

Both calls inside `GenerateTerrainPreview` pass `param_3=0`:
```
0064117b..0064118c: PUSH 0x0 ; PUSH ESI ; MOV ECX,0x87f7e8 ; CALL 0x00578460
006412ce..006412de: PUSH 0x0 ; ...       ; MOV ECX,0x87f7e8 ; CALL 0x00578460
```
Verified via get_assembly_context 0x0064118c,0x006412de.

## The predicate itself — `Is_Cell_In_Playfield(cell, param_3=0)`

Disassembled in full (disassemble_function 0x00578460). With `param_3=0` the byte
comparison `TEST AL,AL; JZ 0x005784cd` at `0x00578475/0x00578477` skips straight past
the "adjust for CellClass+0x11b/0x11c" branch, so `ESI` (the adjustment term, called
`iVar4` in the earlier decompile) stays **0** for the entire remainder of the function.
Let `X = param_2[0]`, `Y = param_2[1]` (both `MOVSX` from 16-bit cell coords, confirmed
at `0x0057846d`/`0x00578472`), `s = X+Y`, `d = X-Y`, and name the fields `MW=+0xf4`,
`LX=+0xfc`, `LY=+0x100`, `LW=+0x104`, `LH=+0x108`.

```
005784e1: CMP EBP,EBX     ; EBP=s, EBX=MW+2*LY           -> JLE fail            (test 1)
005784fc: CMP ESI,EBX     ; ESI=s, EBX=MW+2*LY+2*LH+2    -> JG  fail            (test 2)
00578516: CMP EBX,ECX     ; EBX=d, ECX=2*(LX+LW)-MW      -> JGE fail            (test 3)
00578521: CMP EDX,EAX     ; EDX=-d(=Y-X), EAX=MW-2*LX     -> JGE fail            (test 4)
```
All four verified via disassemble_function 0x00578460 (the exact instruction addresses
and register provenance are traced instruction-by-instruction in the investigation; the
citation above names the four decisive `CMP`/`Jcc` pairs). Falling through all four
(no jump taken) reaches `MOV AL,0x1; RET 0x8` — pass. Any taken branch reaches
`XOR AL,AL; RET 0x8` — fail.

**Closed form** (all four required; `s=X+Y`, `d=X-Y`):

| axis | lower bound | upper bound |
|---|---|---|
| `s` | `s > MW + 2·LY` (strict, test 1 fails on `<=`) | `s <= MW + 2·LY + 2·LH + 2` (inclusive, test 2 fails on `>`) |
| `d` | `d > 2·LX − MW` (strict, test 4 fails on `>=`) | `d < 2·(LX+LW) − MW` (strict, test 3 fails on `>=`) |

Equivalently: `s ∈ (MW+2·LY, MW+2·LY+2·LH+2]` and `d ∈ (2·LX−MW, 2·LX−MW+2·LW)` — a
diamond over cell-space `(X+Y, X-Y)` built purely from **LocalSize** (`LX,LY,LW,LH`)
and the **full map Width** (`MW`); full map **Height** (`+0xf8`) never enters the
param_3=0 path at all.

### Cross-check against the RMG generator's own diamond (context only, out of scope)

The task's settled context states the generator paints terrain out to
`map_w <= (x+y) <= map_w + 2·map_h` with `map_w=gen_w+4, map_h=gen_h+12`, and the
emitter writes `local_left=2, local_top=5, local_width=gen_w, local_height=gen_h`,
`Size=(gen_w+4, gen_h+12)`. Substituting `MW=gen_w+4, LX=2, LY=5, LW=gen_w, LH=gen_h`
into the derived closed form gives `s ∈ (gen_w+14, gen_w+16+2·gen_h]` and
`d ∈ (-gen_w, gen_w)`. **These do not agree** with the generator's own
`gen_w+4 <= s <= gen_w+28+2·gen_h` window — the generator paints a strictly larger
region than what `Is_Cell_In_Playfield` (and hence the preview bounds pass) actually
admits. This is expected: the generator's own diamond governs where it *places
terrain*; `Is_Cell_In_Playfield` governs what the *preview/playfield* considers
in-bounds. Flagging per the task's request, not investigating further (out of scope).

## VERDICT: NOT EQUIVALENT (proven algebraically, not merely "different mechanism")

Rust: `src/map/terrain.rs` `LocalBounds::from_header` (lines 112-137) +
`LocalBounds::contains` (140-145); consumed by `src/map/rmg/preview.rs`
`preview_cells_from_map` (117-142) via `iso_to_screen(cell.rx, cell.ry, cell.z)` then
`bounds.contains(screen_x, screen_y)`.

**1. Height-dependence (structural, no algebra needed).** `iso_to_screen` computes
`screen_y = (rx+ry)*15 + 15 - z*15` — a function of cell elevation `z`. The native
predicate reads only `X`, `Y` (cell coords) and the five `MapClass` fields; it never
reads a cell's height/Z at all (confirmed: the only reads in `Is_Cell_In_Playfield`'s
body are `[EBX]/[EBX+2]` for coords and `[ECX+0xf4/0xfc/0x100/0x104/0x108]` for bounds
— disassemble_function 0x00578460). So for a fixed `(rx,ry)`, gamemd's admission
decision is **constant across all Z**, while the Rust port's is **not** — there exist
concrete `(rx,ry,z)` triples (any cell whose z pushes `screen_y` across a `pixel_y`
boundary) where the two disagree. A function that is constant in a variable cannot be
identical to one that is non-constant in that variable — this alone is a complete
non-equivalence proof, not an edge-case dismissal.

**2. Even fixing z=0, the two windows are shifted/padded, not merely rounded.**
Converting `LocalBounds::contains` back into cell-space `(s,d)` at `z=0`:

- `d`-axis: Rust admits `d ∈ [2·LX−MW+2, 2·LX−MW+2·LW+2)`; native admits
  `d ∈ (2·LX−MW, 2·LX−MW+2·LW)`. Both have width `~2·LW`, but Rust's window is shifted
  **+2** (one full cell) toward higher `d` relative to native's — native's lowest
  admitted column is dropped, and Rust admits one extra column past native's highest.
- `s`-axis: Rust admits `s ∈ [MW+2·LY−6, MW+2·LY+2·LH+4)`; native admits
  `s ∈ (MW+2·LY, MW+2·LY+2·LH+2]`. Rust's window is **6 cells taller at the low end and
  ~1 cell taller at the high end** — total extra height `8`, which is exactly
  `TS_INITIAL_HEIGHT(3) + TS_HEIGHT_ADDITION(5)` from `src/map/terrain.rs` lines 100-103.

This confirms the Rust `LocalBounds` is deliberately implementing the WAE/community
"LocalSize-to-preview-rect" padding convention (extra headroom above/below for tall
terrain rendering) — a real and useful concept for a renderer, but **not** what
`Is_Cell_In_Playfield` computes. The native predicate is an exact, unpadded diamond
over the raw `LocalSize` rect and the map's `Size` width; the Rust substitute is a
different, larger, height-sensitive region built for a different purpose. Verdict:
**DRIFT** (per CLAUDE.md's default-to-drift rule) — not INTERNAL-ONLY, since this
directly changes which cells are admitted to the preview and therefore the emitted
`(max_col-min_col)*2 x (max_row-min_row)` preview surface dimensions for every
generated map.

## Implementation Handoff

- Verified behavior: `Is_Cell_In_Playfield(param_3=0)` admits cell `(X,Y)` iff
  `MW+2·LY < X+Y <= MW+2·LY+2·LH+2` AND `2·LX−MW < X−Y < 2·LX−MW+2·LW`, where
  `MW=Size.Width`, `(LX,LY,LW,LH)=LocalSize` (all in cells; verified via
  disassemble_function 0x00578460, 0x004ad76b, 0x00565c10).
  -> Rust delta: replace `LocalBounds::from_header`/`contains` (the WAE-padding pixel
  rectangle) with a direct integer test on `(cell.rx+cell.ry, cell.rx-cell.ry)` against
  the four bounds above, computed straight from `MapHeader.width` and
  `MapHeader.{local_left,local_top,local_width,local_height}` — no pixel/TS-scale
  conversion, no z term.
  -> Affected surface: `src/map/rmg/preview.rs::preview_cells_from_map` (and any other
  caller of `LocalBounds::contains` that means "is this cell in the playfield," not
  "should this be padded for rendering headroom").
  -> Acceptance scenario: for a header with `width=68, local_left=2, local_top=5,
  local_width=64, local_height=64`, the admitted cell set's `(min_col,max_col,min_row,
  max_row)` in `(s,d)` terms must equal `s∈{83..150}, d∈{-63..63}` (open/closed per the
  table above) exactly, matching a hand-computed reference table, not the current
  `LocalBounds` output.
  -> Proposed test name: `test_playfield_predicate_matches_localsize_bounds`
  -> Risk: every existing generated-map preview's pixel dimensions will change once
  fixed (this is the intended fix — they are currently drift).

- Verified behavior: default `LocalSize` (when the INI key is absent) equals the full
  `Size` rect with origin forced to `(0,0)`, i.e. `(0,0,MW,MH)` (verified via
  disassemble_function 0x004ad76b + 0x00565c10).
  -> Rust delta: if `MapHeader` parsing ever needs a LocalSize default, use
  `(0,0,width,height)`, not `(1,1,width,height)` or any other guess.
  -> Affected surface: `src/map/map_file.rs` MapHeader parsing (only if/when a
  LocalSize-absent map is ever loaded — most retail maps carry the key).
  -> Acceptance scenario: parse a synthetic `.map` with `Size=1,1,50,50` and no
  `LocalSize=` key; assert the effective local rect is `(0,0,50,50)`.
  -> Proposed test name: `test_missing_localsize_defaults_to_full_size_zero_origin`
  -> Risk: low — no known retail/RMG map currently omits `LocalSize=`.

- Verified behavior: the RMG generator's internal terrain-painting diamond
  (`map_w <= s <= map_w+2·map_h` using full `Size`, not `LocalSize`) is provably wider
  than the actual playfield/preview-admitted diamond derived here.
  -> Rust delta: none proposed here (out of scope — the generator's own diamond isn't
  this predicate); flagged only so a future preview-dimension fix isn't papered over by
  assuming the generator's diamond is the ground truth.
  -> Affected surface: `src/map/rmg/preview.rs` (bounds-pass consumer), not the
  generator itself.
  -> Acceptance scenario: N/A — informational cross-check, not an independent fix.
  -> Proposed test name: N/A
  -> Risk: N/A

## Negative Facts / Do Not Do

- Do NOT assume `+0xf4` is part of the `LocalSize` rect — it is the **full map**
  `Size.Width`, eight bytes before the `LocalSize` rect starts at `+0xfc` (verified via
  disassemble_function 0x004ad76b and 0x00565c10; the gap at `+0xf8` holds `Size.Height`,
  untouched by this predicate).
- Do NOT port the Rust `LocalBounds` pixel-rectangle/TS-scale conversion (48/24 cell
  pixels, `-3`/`+5` padding, 1.25 scale) as if it were the native playfield test — it is
  a WAE/community rendering-headroom convention with no counterpart in
  `Is_Cell_In_Playfield` (see VERDICT).
- Do NOT reuse the RMG generator's own `map_w <= s <= map_w+2·map_h` diamond as the
  playfield/preview-admission test — it is measurably wider (see cross-check above) and
  answers a different question (where the generator paints, not what's in-bounds).
- Do NOT treat `Is_Cell_In_Playfield`'s Z-independence as an oversight to "fix" by
  adding elevation sensitivity to a Rust replacement — the native predicate is provably
  constant in Z for fixed `(X,Y)`; adding Z-dependence would itself be a new drift.
- Do NOT assume the origin fields `+0xec/+0xf0` ever hold the `Size=` INI's raw X,Y —
  `MapClass::Resize` always zeroes them immediately after the transient read (verified
  via disassemble_function 0x00565c10, instructions `0x005662d8`/`0x005662e1`).

## Remaining Uncertainty

- The vtable slot `+0x70` call site (`0x004acf0d`) was matched to `MapClass::Resize`
  by parameter-shape correspondence and by the fact that `Resize`'s body performs
  exactly the `this+0xf4=W` write the calling context requires — not by a live vtable
  dereference (the static image's vtable pointer at `0x0087f7e8` reads as zero at rest,
  since it's runtime-constructed; read_memory 0x87f7e8 confirmed all-zero). Treat the
  caller-identity link as HIGH-confidence-by-correspondence, not binding-verified.
- `FUN_00527cc0`'s first two parameters (`param_1`, an opaque INI-cache struct) were not
  further decoded beyond what's needed to confirm the X,Y,W,H field order and the
  default-fallback behavior; its caching/lookup internals are out of scope.

Status: COMPLETE
