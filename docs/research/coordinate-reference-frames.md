# Coordinate reference frames — porting binary offsets

Reference material relocated from CLAUDE.md. Direction bugs (refinery exit at wrong
corner, miner drives backwards, facing byte misread as drive-track index,
foundation-relative offset applied to the wrong reference point) are a recurring bug
class — see `feedback_direction_bugs` in memory. Root cause: gamemd.exe juggles five
reference frames for "where is this entity," and porting goes wrong when a binary
offset starting from one frame gets applied to a Rust value in another. The discipline
below catches it at the binary-to-Rust translation step.

## The five binary reference frames

When you read an offset out of Ghidra, name which frame it starts from:

| # | Frame | Source | Unit | Notes |
|---|-------|--------|------|-------|
| 1 | **Location** | `(class) + 0x9C` / `+ 0xA0` direct read | Leptons (1 cell = 256) | For buildings: NW-corner cell origin in leptons. Verified via `BuildingClass::GetCoords` (`0x00447AC0`) reconstructing geometric center as `Location + ((w-1)*128, (h-1)*128)`. |
| 2 | **Get_Cell_Packed (NW cell)** | `vtable + 0x1B8` → `ObjectClass::Get_Cell_Packed` at `0x0041BEA0` | Cell index | Location ÷ 256 with sign-correct arithmetic shift. For buildings = NW-corner cell. |
| 3 | **GetCoords (foundation center)** | `vtable + 0x48` → for buildings: `BuildingClass::GetCoords` at `0x00447AC0` | Leptons | `Location + ((w-1)*128, (h-1)*128)`. Used as the base for `Force_Track` destinations. |
| 4 | **Foundation outline** | `BuildingTypeClass.vtable + 0x90` returns array of `short[2]` cell deltas | Cells **relative to NW** | Array of cell-relative shorts terminated by `(0x7FFF, 0x7FFF)`; defines the building's footprint shape. |
| 5 | **Dock/refinery reference points** | `Receive_Radio 0x0E`, `BuildingClass::GetDockCoord`, art `QueueingCell`, approach-angle docking | Mixed (cells or leptons) | Stock refinery has three distinct cells: accepted `0x0E` move target `NW + (3, 1)`, stock 4x3 `GetDockCoord` arrival cell `NW + (2, 1)`, and art `QueueingCell=4,1` fallback/wait target `NW + (4, 1)`. Do not collapse them. The `(-0x80, +0x80)` lepton shift is the general approach-angle dock adjustment, not a refinery-only offset. |

## The Rust canonical frame

All `sim/` coordinates are **cell-grid `(u16, u16)`** with **+X = east, +Y = south**,
anchored at the NW corner of the building footprint. Convert binary findings to this
frame before applying. Never carry raw leptons or vtable-relative offsets into game logic.

## Required annotation pattern

Every binary-derived offset written into a research doc or Rust constant must name its
reference frame inline:

```rust
// from <frame name> (<unit>): <one-sentence explanation>
```

Concrete examples:

```rust
// from GetCoords (foundation center, leptons), shifted by (-0x80, +0x80)
// = (-0.5, +0.5) cells. Force_Track destination from ReleaseDockedHarvester step 8.
const FORCE_TRACK_DST_OFFSET_LEPTONS: (i32, i32) = (-0x80, 0x80);

// from Get_Cell_Packed (NW cell index), shifted by (-1, +1) cells.
// ReleaseDockedHarvester step 10 spiral anchor — vestigial, overwritten by
// Mission_Harvest case 0 SCAN before becoming a visible waypoint.
```

In research docs, prefix every offset/address citation with the frame:

> "`vtable+0x1B8` returns NW cell index (cells); the `(-1, +1)` literal is applied in
> cell-space, landing the anchor one cell west and one south of the foundation NW corner."

## Translation-time checklist

Before applying ANY binary offset in Rust:

1. **Name the source frame.** Which of the five frames produced this offset? If you can't name it, go back to Ghidra.
2. **Name the unit.** Cells or leptons? Lepton values need ÷ 256, with the same sign-correct arithmetic shift the binary uses (`(x + ((x >> 31) & 0xFF)) >> 8`). Forgetting the floor-correction term flips negative coords by one cell.
3. **Pick the Rust target.** All sim/ logic uses NW-cell-indexed `(u16, u16)`. Convert at the boundary.
4. **Walk a concrete fixture before claiming the formula is right.** Canonical refinery case: GAREFN 4×3 at NW `(10, 10)` — foundation cells `(10..14, 10..13)`, accepted `0x0E` move cell `(13, 11)`, stock 4×3 `GetDockCoord` arrival cell `(12, 11)`, `QueueingCell` `(14, 11)`, geometric center cell `(11, 11)`, geometric center leptons `(2944, 2816)`. Plug your formula into this fixture; if the result doesn't land where the player would expect, the offset is wrong before any code is written.

## Isometric rendering vs. cell axes

The cell grid (+X east, +Y south) is **not** the same as on-screen direction. Isometric
rendering rotates the grid 45°: cell `+X` axis renders to screen down-right, cell `+Y`
axis to screen down-left. When the user reports an in-game observation ("the miner exits
to the right of the building"), confirm whether they mean cell-east (+X) or screen-right
(mixed +X/+Y) before translating it into a coordinate claim.

## Facing bytes vs. drive-track indices — not the same thing

These two encodings look like single-byte values and are easy to confuse:

- **Facing byte (8-bit)**: `0x00` = North, `0x40` = East, `0x80` = South, `0xC0` = West. Clockwise from north.
- **Facing word (16-bit)**: same convention shifted — `0x4000` = East.
- **Drive-track index**: an index into the `g_DriveTrackData_Array` curve table (0–127). `0x47` is a curve index, NOT a facing. Setting `facing = 0x47` makes the unit drive backwards (body facing ESE while moving toward a west destination). See `feedback_direction_bugs`.

When you see a single-byte value passed to a locomotor or drive method, trace which
parameter slot it lands in before declaring it a facing.
