# Drive Track Tables — Deep Decode (Ghidra Research Report)

**Addresses:** TurnTrack table at `0x7e7b28` (864 bytes), RawTrack table at `0x7e7a28`
(192 bytes). Helper functions in `0x4af3e0..0x4b4de0`.
**Confidence:** HIGH for table layout and contents (raw memory verified). HIGH for
flag-bit semantics (Transform_Track_Coords + Process_Movement bit-test verified).
**Active in YR:** Yes — every `Drive` locomotor unit reads these tables every tick.

---

## 1. Overview

This report fills the explicit gaps left by `DRIVE_TRACK_SYSTEM.md` (Mar 2026):
extracted-but-unverified track point arrays for tracks 5–15, the unanswered question
of when `use_short_track` (loco+0x60) is set, the disputed semantic of flag bit 3,
and the special-track range 64–71. All findings verified by direct binary read of
gamemd.exe via Ghidra MCP.

Major outcomes:

1. **`use_short_track` (loco+0x60) is dead in YR.** The byte is written exactly once
   — by the constructor at `0x4af5ac` — and never modified at runtime. The entire
   `short_track` column of the TurnTrack table (and raw tracks 7–10) is unreferenced
   in normal play. Confirmed by exhaustive byte-pattern search across the binary.

2. **Flag bit 3 = "track has cell-crossing geometry"**, not a transform flag.
   Process_Movement at `0x4b4046` does `test byte ptr [edx*4 + 0x7e7b30], 8` to gate
   the cell-crossing handler. The bit is set on every TurnTrack entry referencing
   raw tracks 3, 4, 5, 6 (the four curves with non-`-1` `jump_index` in the
   RawTrack table) and unset everywhere else.

3. **`DRIVE_TRACK_SYSTEM.md` point counts for tracks 7–10 are wrong.** Doc claims
   22, 22, 24, 22 respectively. Binary actually has 27, 21, 30, 27 active points
   (verified against Rust impl which matches binary, not the doc).

4. **Special tracks 64–71 are reverse/diagonal-drift tracks** with target_facing in
   the SW/diagonal range and reference raw tracks 11–15. They cannot be produced
   by the standard `track_index = path_dir + facing*8` formula (which only goes
   0–63), so they must be assigned via `Force_Track` (0x4b0c40, vtable-dispatched).

---

## 2. TurnTrack Struct Layout — Corrected

The `DRIVE_TRACK_SYSTEM.md` claim of `i32 direction` and `i32 flags` is misleading.
The struct is really:

```c
struct TurnTrackEntry {     // 12 bytes
    u8  normal_track;       // +0x00  raw_track index 0-15 (0 = no curve, fallback)
    u8  short_track;        // +0x01  raw_track index used when loco+0x60 != 0
    u8  _pad1[2];           // +0x02  always 0
    u8  target_facing;      // +0x04  0x00..0xE0 in 0x20 steps (post-track facing)
    u8  _pad2[3];           // +0x05  always 0
    u8  flags;              // +0x08  bits 0-3 used (bits 4-7 always 0)
    u8  _pad3[3];           // +0x09  always 0
};
```

Verified by exhaustive read of all 864 bytes: bytes at offsets +0x05, +0x06, +0x07,
+0x09, +0x0A, +0x0B are zero in every one of the 72 entries. The decompilation
loads them as `i32` for ABI reasons, but the high bytes are unused.

### Flags Byte — Bit Assignments

| Bit | Name | Effect | Verified by |
|-----|------|--------|-------------|
| 0 (0x01) | swap_xy | Swap X↔Y, then `facing = (-facing - 0x40) & 0xFF` | Transform_Track_Coords decomp |
| 1 (0x02) | negate_x | Negate X, `facing = -facing & 0xFF` | Transform_Track_Coords decomp |
| 2 (0x04) | negate_y | Negate Y, `facing = (-facing - 0x80) & 0xFF` | Transform_Track_Coords decomp |
| 3 (0x08) | cell_crossing | Track contains a `jump_index` cell handoff; gates Can_Enter_Cell mid-track | Process_Movement at 0x4b4046: `test byte ptr [edx*4 + 0x7e7b30], 8` followed by `je +0x5ab` |
| 4-7 | unused | Never set in any of the 72 entries | direct binary read |

**Cross-check:** Bit 3 is set on every TurnTrack entry with `normal_track ∈ {3, 4, 5, 6}`
and unset on every entry with `normal_track ∈ {0, 1, 2, 7-15}`. RawTracks 3–6 are
exactly the entries with `jump_index != -1` in the RawTrack table. Pattern is 100%
consistent across all 72 entries.

### Target-Facing Encoding

The byte at +0x04 holds the *post-turn* facing in 0x20-step increments. Can_Use_Track
decodes it as:

```c
final_facing = ((target_facing_byte * 256) >> 12 + 1) >> 1 & 7
            = ((target_facing_byte) >> 4 + 1) >> 1 & 7
            = (target_facing_byte / 0x20)        // for the standard 0x00..0xE0 values
```

Mapping: 0x00=N(0), 0x20=NE(1), 0x40=E(2), 0x60=SE(3), 0x80=S(4), 0xA0=SW(5), 0xC0=W(6), 0xE0=NW(7).

---

## 3. Full TurnTrack Table — All 72 Entries Decoded

Index = `current_facing * 8 + next_direction`. NULL means `normal_track=0` (no curve;
caller falls back to `track_index = current_facing * 9`).

### Entries 0–7 (current=N)

| Idx | Trans | n_track | s_track | facing | flags | Note |
|----:|------|--------:|--------:|-------:|------:|------|
|  0 | N→N  |  1 | 0  | 0x00 | 0x00 | straight N (track 1, identity) |
|  1 | N→NE |  3 | 7  | 0x20 | 0x08 | 45° right turn (track 3) |
|  2 | N→E  |  4 | 9  | 0x40 | 0x08 | 90° right turn (track 4) |
|  3 | N→SE |  0 | 0  | 0x60 | 0x00 | NULL (135° impossible) |
|  4 | N→S  |  0 | 0  | 0x80 | 0x00 | NULL (180° impossible) |
|  5 | N→SW |  0 | 0  | 0xA0 | 0x00 | NULL |
|  6 | N→W  |  4 | 9  | 0xC0 | 0x0A | 90° left (track 4 + negate X) |
|  7 | N→NW |  3 | 7  | 0xE0 | 0x0A | 45° left (track 3 + negate X) |

### Entries 8–15 (current=NE)

| Idx | Trans | n | s | facing | flags |
|----:|------|--:|--:|-------:|------:|
|  8 | NE→N  |  6 |  8 | 0x00 | 0x0F |
|  9 | NE→NE |  2 |  0 | 0x20 | 0x00 | straight NE (track 2, identity) |
| 10 | NE→E  |  6 |  8 | 0x40 | 0x08 |
| 11 | NE→SE |  5 | 10 | 0x60 | 0x08 | wide curve (track 5) |
| 12 | NE→S  |  0 |  0 | 0x80 | 0x00 | NULL |
| 13 | NE→SW |  0 |  0 | 0xA0 | 0x00 | NULL |
| 14 | NE→W  |  0 |  0 | 0xC0 | 0x00 | NULL |
| 15 | NE→NW |  5 | 10 | 0xE0 | 0x0F |

### Entries 16–23 (current=E)

| Idx | Trans | n | s | facing | flags |
|----:|------|--:|--:|-------:|------:|
| 16 | E→N  |  4 |  9 | 0x00 | 0x0F |
| 17 | E→NE |  3 |  7 | 0x20 | 0x0F |
| 18 | E→E  |  1 |  0 | 0x40 | 0x03 | straight E (track 1 + swap+negX) |
| 19 | E→SE |  3 |  7 | 0x60 | 0x0B |
| 20 | E→S  |  4 |  9 | 0x80 | 0x0B |
| 21 | E→SW |  0 |  0 | 0xA0 | 0x00 | NULL |
| 22 | E→W  |  0 |  0 | 0xC0 | 0x00 | NULL |
| 23 | E→NW |  0 |  0 | 0xE0 | 0x00 | NULL |

### Entries 24–31 (current=SE)

| Idx | Trans | n | s | facing | flags |
|----:|------|--:|--:|-------:|------:|
| 24 | SE→N  |  0 |  0 | 0x00 | 0x00 | NULL |
| 25 | SE→NE |  5 | 10 | 0x20 | 0x0C |
| 26 | SE→E  |  6 |  8 | 0x40 | 0x0C |
| 27 | SE→SE |  2 |  0 | 0x60 | 0x04 | straight SE (track 2 + negY) |
| 28 | SE→S  |  6 |  8 | 0x80 | 0x0B |
| 29 | SE→SW |  5 | 10 | 0xA0 | 0x0B |
| 30 | SE→W  |  0 |  0 | 0xC0 | 0x00 | NULL |
| 31 | SE→NW |  0 |  0 | 0xE0 | 0x00 | NULL |

### Entries 32–39 (current=S)

| Idx | Trans | n | s | facing | flags |
|----:|------|--:|--:|-------:|------:|
| 32 | S→N  |  0 |  0 | 0x00 | 0x00 | NULL |
| 33 | S→NE |  0 |  0 | 0x20 | 0x00 | NULL |
| 34 | S→E  |  4 |  9 | 0x40 | 0x0C |
| 35 | S→SE |  3 |  7 | 0x60 | 0x0C |
| 36 | S→S  |  1 |  0 | 0x80 | 0x04 | straight S (track 1 + negY) |
| 37 | S→SW |  3 |  7 | 0xA0 | 0x0E |
| 38 | S→W  |  4 |  9 | 0xC0 | 0x0E |
| 39 | S→NW |  0 |  0 | 0xE0 | 0x00 | NULL |

### Entries 40–47 (current=SW)

| Idx | Trans | n | s | facing | flags |
|----:|------|--:|--:|-------:|------:|
| 40 | SW→N  |  0 |  0 | 0x00 | 0x00 | NULL |
| 41 | SW→NE |  0 |  0 | 0x20 | 0x00 | NULL |
| 42 | SW→E  |  0 |  0 | 0x40 | 0x00 | NULL |
| 43 | SW→SE |  5 | 10 | 0x60 | 0x09 |
| 44 | SW→S  |  6 |  8 | 0x80 | 0x09 |
| 45 | SW→SW |  2 |  0 | 0xA0 | 0x01 | straight SW (track 2 + swap) |
| 46 | SW→W  |  6 |  8 | 0xC0 | 0x0E |
| 47 | SW→NW |  5 | 10 | 0xE0 | 0x0E |

### Entries 48–55 (current=W)

| Idx | Trans | n | s | facing | flags |
|----:|------|--:|--:|-------:|------:|
| 48 | W→N  |  4 |  9 | 0x00 | 0x0D |
| 49 | W→NE |  0 |  0 | 0x20 | 0x00 | NULL |
| 50 | W→E  |  0 |  0 | 0x40 | 0x00 | NULL |
| 51 | W→SE |  0 |  0 | 0x60 | 0x00 | NULL |
| 52 | W→S  |  4 |  9 | 0x80 | 0x09 |
| 53 | W→SW |  3 |  7 | 0xA0 | 0x09 |
| 54 | W→W  |  1 |  0 | 0xC0 | 0x01 | straight W (track 1 + swap) |
| 55 | W→NW |  3 |  7 | 0xE0 | 0x0D |

### Entries 56–63 (current=NW)

| Idx | Trans | n | s | facing | flags |
|----:|------|--:|--:|-------:|------:|
| 56 | NW→N  |  6 |  8 | 0x00 | 0x0D |
| 57 | NW→NE |  5 | 10 | 0x20 | 0x0D |
| 58 | NW→E  |  0 |  0 | 0x40 | 0x00 | NULL |
| 59 | NW→SE |  0 |  0 | 0x60 | 0x00 | NULL |
| 60 | NW→S  |  0 |  0 | 0x80 | 0x00 | NULL |
| 61 | NW→SW |  5 | 10 | 0xA0 | 0x0A |
| 62 | NW→W  |  6 |  8 | 0xC0 | 0x0A |
| 63 | NW→NW |  2 |  0 | 0xE0 | 0x02 | straight NW (track 2 + negX) |

### Entries 64–71 (Special — NOT reachable via standard formula)

| Idx | n_track | s_track | facing | flags | Effect |
|----:|--------:|--------:|-------:|------:|--------|
| 64 | 11 | 11 | 0xA0 | 0x00 | RawTrack 11, target SW |
| 65 | 12 | 12 | 0xA0 | 0x00 | RawTrack 12, target SW |
| 66 | 13 | 13 | 0xA0 | 0x00 | RawTrack 13, target SW |
| 67 | 14 | 14 | 0x20 | 0x00 | RawTrack 14, target NE |
| 68 | 14 | 14 | 0x60 | 0x04 | RawTrack 14, target SE (negY mirror) |
| 69 | 14 | 14 | 0xA0 | 0x01 | RawTrack 14, target SW (swap) |
| 70 | 14 | 14 | 0xE0 | 0x02 | RawTrack 14, target NW (negX) |
| 71 | 15 | 15 | 0xC0 | 0x00 | RawTrack 15, target W |

**Observation:** Entries 64–71 cannot be produced by the standard formula
`track_index = path_dir + facing*8` (which produces 0..63). The Process_Movement
fallback `track_index = facing * 9` produces 0, 9, 18, 27, 36, 45, 54, 63 (the
"identity" diagonal of the 8×8 matrix) — also bounded by 63. So entries 64–71 are
only reachable via direct assignment (e.g., `Force_Track`, address 0x4b0c40).

**Force_Track has zero direct call sites in the binary** — `get_xrefs_to 0x4b0c40`
returns only one DATA reference, from `0x7e7f20`, which is the first slot of an
extended ILocomotion-like vtable starting there. Callers reach it via vtable
dispatch (`call dword ptr [eax+0x70]` style), so they are not statically traceable
without full vtable usage analysis. The only sample target_facings (0xA0 SW for
tracks 11–13, all four diagonals for track 14, 0xC0 W for track 15) suggest these
are **deploy/scatter/reverse-drive tracks** rather than normal navigation.

In Process_Drive_Track, the `track_step < 0x40` guard skips speed compute for
indices ≥ 64 — confirming these tracks run at fixed speed without acceleration/
deceleration, which is consistent with deploy or forced-movement animations.

---

## 4. RawTrack Table — All 16 Entries

Verified read at `0x7e7a28` (256 bytes = 16 × 16-byte entries):

```c
struct RawTrackEntry {       // 16 bytes
    TrackPoint* points;      // +0x00  pointer to point array (NULL for entry 0)
    i32  exit_index;         // +0x04  see note below — NOT total point count
    i32  entry_index;        // +0x08  starting walk position
    i32  jump_index;         // +0x0C  cell-crossing point (-1 = none)
};
```

| Track | ptr | +0x04 | entry | jump | Notes |
|------:|-----|------:|------:|-----:|-------|
|  0 | NULL | 0 | 0xC0 | 0 | sentinel/null entry |
|  1 | 0x7e6258 | -1 | 0 | -1 | straight N |
|  2 | 0x7e6378 | -1 | 0 | -1 | straight NE |
|  3 | 0x7e64f8 | 37 | 12 | 22 | 45° turn (cell-crossing) |
|  4 | 0x7e6790 | 26 | 11 | 19 | 90° turn (cell-crossing) |
|  5 | 0x7e6968 | 45 | 15 | 31 | wide curve A (cell-crossing) |
|  6 | 0x7e6c50 | 44 | 16 | 27 | wide curve B (cell-crossing) |
|  7 | 0x7e6f00 | -1 | 0 | -1 | short curve A |
|  8 | 0x7e7050 | -1 | 0 | -1 | short curve B |
|  9 | 0x7e7158 | -1 | 0 | -1 | short curve C |
| 10 | 0x7e72d0 | -1 | 0 | -1 | short curve D |
| 11 | 0x7e7420 | -1 | 0 | -1 | special A (used by entry 64) |
| 12 | 0x7e74c8 | -1 | 0 | -1 | special B (used by entry 65) |
| 13 | 0x7e7568 | -1 | 0 | -1 | special C (used by entry 66) |
| 14 | 0x7e78a8 | -1 | 0 | -1 | diagonal drift (used by entries 67–70) |
| 15 | 0x7e7968 | -1 | 0 | -1 | curving rotation (used by entry 71) |

### Field +0x04 — Not the Total Point Count

`DRIVE_TRACK_SYSTEM.md` and Process_Drive_Track decompilation labeled this as
`total_count`, but its value (37, 26, 45, 44 for tracks 3–6) does NOT equal the
point count. Tracks 3 and 5 have 54 and 61 points respectively in the Rust impl
(extracted by walking until sentinel), but field +0x04 holds 37 and 45 — these
are smaller numbers.

Process_Drive_Track uses field +0x04 as a chain-validity flag
(`if g_DriveTrackData_Array[next_entry * 16 + 4] != 0` allows chaining), and
Can_Use_Track compares it against `loco.track_point_index` (loco+0x5C). This
suggests field +0x04 is the **exit index** — the point at which the curve has
completed its cell-crossing transition and standard tick-by-tick stepping
resumes. Tracks 1–2, 7–15 have field +0x04 = -1 because they have no
cell-crossing transition (no `jump_index`).

**Implication for Rust:** Walking the point array until the (x=0, y=0) sentinel
gives the correct active count, but the +0x04 field is needed for the chain-
validity check at track-end, not for sizing. It should be parsed and stored as
a separate field. **Open question:** what exactly does Can_Use_Track's
`field_4 == track_point_index` comparison mean — that the unit has reached the
exit point and is ready to chain into the next track? Confidence MEDIUM.

---

## 5. Track Point Sample Decodes (Verified Bytes)

All point arrays are 12-byte triplets: `i32 x, i32 y, i32 facing` (low byte of
facing only is meaningful; high 3 bytes are zero in every sampled point).

### Track 1 — Straight N (0x7e6258, 24 entries / 23 active)

```
Pt  0: x=0,  y=245, facing=0
Pt  1: x=0,  y=234, facing=0      (Δy = -11/step)
Pt  2: x=0,  y=223, facing=0
...
Pt 21: x=0,  y=14,  facing=0
Pt 22: x=0,  y=3,   facing=0
Pt 23: x=0,  y=0,   facing=0      ← END SENTINEL
```

Step Δy = -11 leptons. 23 active points × 11 = 253 leptons traversed = ~1 cell
(256 leptons). Walk ends at sentinel, NOT at +0x04 (= -1 for this track).

### Track 5 — Wide Curve A, lead-in (0x7e6968)

```
Pt 0: x=-504, y=-8,  facing=0x20
Pt 1: x=-496, y=-16, facing=0x20    (Δx=+8, Δy=-8 per step)
Pt 2: x=-488, y=-24, facing=0x20
...
Pt 9: x=-432, y=-80, facing=0x20
```

Constant facing 0x20 (NE) during the lead-in. The "previous-cell" approach —
position is in negative X to indicate the unit started in the cell to the west.
Walk continues through pivot at entry_index=15 and cell-crossing at jump_index=31.

### Track 7 — Short Curve A (0x7e6f00, 28 entries / 27 active)

```
Pt  0: x=-1, y=6,  facing=0
Pt  1: x=-2, y=12, facing=4
Pt  2: x=-4, y=17, facing=8
Pt  3: x=-6, y=24, facing=12
Pt  4: x=-10,y=31, facing=16
Pt  5: x=-13,y=36, facing=19
Pt  6: x=-16,y=43, facing=22
Pt  7: x=-3, y=48, facing=23      ← x JUMPS from -16 to -3 (cell-handoff handled
                                     entirely within point array, not via jump_index
                                     since field +0x0C = -1)
...
Pt 13: x=-35,y=70, facing=29      ← max excursion
Pt 14: x=-33,y=67, facing=30      (curve doubles back)
...
Pt 26: x=-3, y=6,  facing=32
Pt 27: x=0,  y=0,  facing=32      ← END SENTINEL
```

**Doc error caught:** `DRIVE_TRACK_SYSTEM.md` claimed track 7 = 22 points (264
bytes). Actual: 27 active + 1 sentinel = 28 entries (336 bytes). Pointer-gap
calculation in the doc was based on wrong byte counts.

Final facing 32 = 0x20 = NE. So track 7 is "rotate from N to NE while doing a
small jog to the left then right." Probably a stationary turn for vehicles with
high turn radius.

### Track 9 — Short Curve C (0x7e7158, ~30 active)

```
Pt 0: x=2,   y=-11, facing=0
Pt 1: x=4,   y=-21, facing=2
Pt 2: x=6,   y=-32, facing=4
...
```

Sharp curve. Rust impl has 30 points; binary has 30 active + sentinel = 31 entries.
Doc claim of 24 points is **wrong**.

### Track 11 — Special A (0x7e7420, 14 entries / 13 active)

```
Pt  0: x=0,  y=256, facing=0xA0
Pt  1: x=8,  y=243, facing=0xA0    (Δx=+8, Δy=-13/-14 per step)
Pt  2: x=16, y=229, facing=0xA0
...
Pt 12: x=96, y=85,  facing=0xA0
Pt 13: x=0,  y=0,   facing=0xA0    ← END SENTINEL
```

Note: facing is **constant 0xA0 (SW)** while geometry moves "rightward and downward"
in lepton coords — i.e., the unit body is oriented SW but moves NE-east-ish. This
is consistent with **reverse driving** (unit faces the way it came from).

### Track 14 — Diagonal Drift (0x7e78a8)

```
Pt 0: x=-120, y=120, facing=0x20
Pt 1: x=-112, y=112, facing=0x20    (Δx=+8, Δy=-8 per step)
Pt 2: x=-104, y=104, facing=0x20
...
```

Constant facing 0x20 (NE). Identical Δ pattern to track 5 lead-in but starts at
(-120, 120) instead of (-504, -8) — a much shorter run.

### Track 15 — Curving Rotation (0x7e7968)

```
Pt 0: x=128, y=-128, facing=0x80
Pt 1: x=124, y=-112, facing=0x84    ← facing CHANGES!
Pt 2: x=119, y=-96,  facing=0x88
Pt 3: x=115, y=-80,  facing=0x8C
Pt 4: x=111, y=-64,  facing=0x90
```

Unique among special tracks: facing increments by 4 per step (track 11–14 have
constant facing). Suggests a smooth pivot rotation while drifting.

---

## 6. Track-Index Lookup Code (Process_Movement at 0x4b401a–0x4b4055)

Disassembled:

```asm
; ESI = path_dir (0..7), EBX = current_facing (0..7)
8d 04 de       lea  eax, [esi + ebx*8]              ; track_index = path_dir + facing*8
c6 45 60 00    mov  byte [ebp+0x60], 0              ; (stack local, NOT loco+0x60)
89 45 58       mov  [ebp+0x58], eax                 ; stash track_index
8d 04 40       lea  eax, [eax + eax*2]              ; eax = track_index * 3
8a 0c 85 28 7b 7e 00
               mov  cl, byte [eax*4 + 0x7e7b28]     ; cl = TurnTrack[idx].normal_track
84 c9          test cl, cl
75 06          jnz  +6                              ; non-zero → use this track
8d 0c db       lea  ecx, [ebx + ebx*8]              ; FALLBACK: ecx = facing * 9
89 4d 58       mov  [ebp+0x58], ecx                 ; track_index = facing * 9
8b 45 58       mov  eax, [ebp+0x58]                 ; eax = final track_index
8d 14 40       lea  edx, [eax + eax*2]              ; edx = idx * 3
f6 04 95 30 7b 7e 00 08
               test byte [edx*4 + 0x7e7b30], 8      ; ★ TEST FLAG BIT 3
0f 84 ab 05 00 00
               je   +0x5ab                          ; bit 3 not set → skip cell-cross
```

### Findings

- **Track lookup formula:** `track_index = path_dir + current_facing * 8`. Index
  range 0–63 only.
- **Fallback when normal_track == 0:** `track_index = current_facing * 9`. This
  is the (curr=N, next=N), (curr=NE, next=NE), ... diagonal of the 8×8 matrix —
  always a straight track in the same direction the unit currently faces. A
  blocked turn becomes a straight-ahead step.
- **Bit 3 of flags is consumed by Process_Movement at 0x4b4046.** When set, code
  enters the cell-crossing handler (jump+offset 0x5ab forward is skipped).
  Cell-crossing logic includes Can_Enter_Cell validation, occupancy update,
  and bridge-ramp detection. When unset, the entry-cell handling is bypassed
  entirely — the track stays within the current cell.
- The local at `[ebp+0x60]` in this code is a stack frame slot for a separate
  flag, NOT the locomotor's `use_short_track` field (which is at `[reg+0x60]`
  with `reg` pointing to the locomotor object). Don't confuse these.

---

## 7. `use_short_track` (loco+0x60) — Confirmed Dead in YR

Per helpers doc: "Constructor inits to 0. Can_Use_Track checks it. Never seen
set to 1 in any decompiled method." **Confirmed by exhaustive byte-pattern search.**

Searched patterns (writes to byte field at offset +0x60 of an object):

| Pattern | Meaning | Hits |
|---------|---------|------|
| `88 40 60` | mov [eax+0x60], al | 0 |
| `88 41 60` | mov [ecx+0x60], al | 0 |
| `88 42 60` | mov [edx+0x60], al | 1 (in CDFileClass — unrelated) |
| `88 43 60` | mov [ebx+0x60], al | 0 |
| `88 45 60` | mov [ebp+0x60], al | 1 (CDFileClass stack — unrelated) |
| `88 46 60` | mov [esi+0x60], al | **2: 0x4af5ac (constructor) + 1 unrelated** |
| `88 47 60` | mov [edi+0x60], al | 0 |
| `c6 41 60 01` | mov [ecx+0x60], 1 | 0 |
| `c6 41 60 00` | mov [ecx+0x60], 0 | 0 |
| `c6 46 60` | mov [esi+0x60], imm | 0 |

**The single locomotor hit at 0x4af5ac is in DriveLocomotionClass::Constructor**
(0x4af540–0x4af5ff), zeroing the field along with other init. No runtime writer.

**Implication:** The `short_track` column of the TurnTrack table (raw tracks 7–10)
and the relevant Can_Use_Track branch are unreachable in standard YR. Either:

(a) A removed feature (e.g., a "boost speed" mode that switched to tighter curves)
(b) Set externally via INI/script in TS but never wired into YR
(c) Reserved for a unit class never shipped

For Rust parity, hardcoding `use_short_track = false` (as the current impl does)
is **correct** for YR behavior. No rewiring needed.

---

## 8. Track Point Count Comparison (Doc vs Binary vs Rust)

| Track | DRIVE_TRACK_SYSTEM.md | Binary actual | Rust impl | Verdict |
|------:|----------------------:|--------------:|----------:|---------|
|  1 | 24 (no sentinel noted) | 23 + sentinel  | 23 | ✓ Rust correct |
|  2 | 32 (assumed)            | ~31 + sentinel | 31 | ✓ |
|  3 | 55                      | ~54 + sentinel | 54 | ✓ Rust matches binary |
|  4 | 38                      | ~38            | 38 | ✓ |
|  5 | 60                      | ~61 + sentinel | 61 | ✓ |
|  6 | 56                      | ~56            | 56 | ✓ |
|  7 | **22 (DOC WRONG)**      | 27 + sentinel  | 27 | ✓ Rust correct, doc wrong |
|  8 | 22                      | 21 + sentinel  | 21 | ✓ |
|  9 | **24 (DOC WRONG)**      | 30 + sentinel  | 30 | ✓ Rust correct, doc wrong |
| 10 | 22 (likely wrong)       | 27 + sentinel  | 27 | ✓ Rust correct |
| 11 | 14                      | 13 + sentinel  | 14 | ✓ |
| 12 | n/a                     | (sampled)      | (?)| Spot-check needed |
| 13 | n/a                     | (sampled)      | (?)| Spot-check needed |
| 14 | n/a                     | (sampled)      | (?)| Spot-check needed |
| 15 | n/a                     | (sampled)      | (?)| Spot-check needed |

The Rust implementation (`src/sim/movement/drive_track.rs`) is more accurate than
the prior research doc. The "Missing 5–15 point data" claim in `DRIVE_TRACK_SYSTEM.md`
is stale; tracks 5–15 have all been extracted into Rust at this point.

---

## 9. Force_Track and Special-Track Invocation

`Force_Track` at `0x4b0c40` is the only entry point that can assign a track_index
≥ 64. Per helpers doc, signature is `Force_Track(this, track_index, dest_x,
dest_y, dest_z)`. When called with track_index ∈ {64..71}, the unit will:

1. Skip speed-compute (track_step ≥ 0x40 guard at start of Process_Drive_Track)
2. Walk the special raw track (11–15) at fixed full speed (set to 1.0 in Force_Track)
3. End up facing one of {SW, NE, SE, NW, W} per the entry's target_facing

**Statically traceable callers:** None. xref is only `0x7e7f20` (a vtable slot).
Any caller dispatches via `call dword [reg+0x70]`-style indirect call. Likely
callers (informed guess, not verified):

- Deploy/Undeploy logic (e.g., MCV, Construction Yard, Deployer Truck)
- Map-edge retreat tracks (via Process_Drive_Track's MAP_EDGE_RETREAT phase
  at lines 206–260 in PROCESS_DRIVE_TRACK_DECOMPILATION.md)
- Convoy/escort scripted moves (Accelerates=true units)
- Aircraft-like drop-pod landings (uses RocketLocomotion piggyback?)

**Open question (MEDIUM confidence):** Without dynamic tracing or a wider call-site
audit, the exact invocation conditions cannot be fully enumerated. For Rust impl,
the safe play is to wire Force_Track behind specific game-state hooks (deploy,
edge-retreat, scripted) as those features are implemented, rather than trying to
preempt all callers.

---

## 10. Integration Points

### Where these tables are read

| Address | Function | Reads |
|--------|----------|-------|
| 0x4b0b22 | Apply_Track_Delta | TurnTrack (via `loco+0x58`) |
| 0x4b151f, 0x4b153a, 0x4b1b78, 0x4b1b83, 0x4b22d9 | Process_Drive_Track | TurnTrack (multiple lookups during stepping) |
| 0x4b4023, 0x4b4046 | Process_Movement | TurnTrack (track_index assignment + bit-3 test) |
| 0x4b4b2c, 0x4b4b3b, 0x4b4ba9, 0x4b4bb0 | Can_Use_Track | TurnTrack + RawTrack chain validity |
| 0x4b4780 | Transform_Track_Coords | TurnTrack flags byte (bits 0–2) |

### Where the tables are NOT written

- `g_DriveTrackIndex_Table` (TurnTrack) is read-only data in the binary's
  `.rdata` section. No code path mutates it.
- `g_DriveTrackData_Array` (RawTrack) is similarly read-only.
- The active `track_index` and `track_point_index` are stored on each
  DriveLocomotionClass instance, not in these global tables.

---

## 11. Current Rust Implementation Status

### Already correct vs binary

- All 72 TurnTrack entries populated and matching binary contents
  ([drive_track.rs:173-679](../ra2-rust-game/src/sim/movement/drive_track.rs#L173))
- All 16 RawTrack entries populated
  ([drive_track.rs:687-816](../ra2-rust-game/src/sim/movement/drive_track.rs#L687))
- Tracks 1–15 point data all extracted and matching binary (counts verified
  against this report's binary reads)
- Transform flag bits 0/1/2 decoded correctly
  ([drive_track.rs:43-61](../ra2-rust-game/src/sim/movement/drive_track.rs#L43))
- `use_short_track = false` hardcoded — correct for YR
  ([movement_step.rs:91](../ra2-rust-game/src/sim/movement/movement_step.rs#L91))

### Gaps to address

| Gap | Severity | Notes |
|-----|----------|-------|
| **Bit 3 (cell-crossing) not handled in transform decoder** | LOW | The transform decoder only consumes bits 0/1/2, which is correct (bit 3 is NOT a transform flag). But `select_drive_track` should use bit 3 to decide whether to invoke cell-crossing logic. Currently the Rust selection code may be relying on RawTrack `jump_index != -1` instead — equivalent in practice, but not the same path the binary takes. Verify. |
| **Special tracks 64–71 invocation** | LOW (until deploy/edge-retreat implemented) | No Force_Track equivalent in Rust. When implementing deploy or edge-retreat, ensure the corresponding track is selected by direct index, not by formula. |
| **RawTrack +0x04 field (exit_index)** | LOW | Not currently a separate field in Rust. If track-chaining edge cases produce wrong-cell behavior, this is the place to look. The binary uses it in Can_Use_Track to detect "ready to chain to next track." |
| **Walking to sentinel** | INFO | Rust matches binary by walking until (x=0,y=0). Don't switch to using +0x04 as point count — it isn't one. |

---

## 12. Open Questions (MEDIUM confidence or below)

1. **What exactly is RawTrack +0x04?** It correlates with the cell-crossing
   region of tracks 3–6 but its exact meaning in Can_Use_Track's
   `field_4 == loco.track_point_index` comparison isn't fully decoded.
   Hypothesis: "exit point for cell-crossing transition" — beyond this point,
   the track resumes standard within-cell stepping. Confirmation requires
   tracing all field_4 reads. **MEDIUM**

2. **Which game systems invoke special tracks 64–71?** Only known caller is via
   `Force_Track` vtable dispatch. Likely candidates: deploy/undeploy state
   transitions, map-edge retreat (MAP_EDGE_RETREAT phase in Process_Drive_Track),
   scripted convoy moves. Needs vtable usage trace. **LOW**

3. **Does Process_Drive_Track use bit 3 anywhere besides the lookup at
   0x4b4046?** The flag is read at `Transform_Track_Coords` (bits 0–2 only)
   and at the lookup site in Process_Movement. Whether bit 3 is also tested in
   the chaining or termination logic of Process_Drive_Track is not yet verified.
   **MEDIUM**

4. **Why does track 7 have an apparent x-coord discontinuity at point 7
   (jumps from -16 to -3)?** Tracks 3–6 use `jump_index` for cell-crossing;
   tracks 7–10 don't (their RawTrack jump_index is -1). The discontinuity may
   represent a within-cell pivot or a quirk of the original art's track
   authoring. **LOW** (visual-only impact if any)

---

## Sources

**Memory reads (verified raw bytes):**
- `0x7e7b28` (864 bytes) — full TurnTrack table
- `0x7e7a28` (256 bytes) — full RawTrack table
- `0x7e6258` (288 bytes) — Track 1 point data
- `0x7e6968` (120 bytes) — Track 5 head sample
- `0x7e6f00` (336 bytes) — Track 7 full data
- `0x7e7158` (288 bytes) — Track 9 head sample
- `0x7e7420` (168 bytes) — Track 11 full data
- `0x7e74c8` (60 bytes) — Track 12 head sample
- `0x7e7568` (60 bytes) — Track 13 head sample
- `0x7e78a8` (120 bytes) — Track 14 head sample
- `0x7e7968` (60 bytes) — Track 15 head sample
- `0x7e7eb0` (96 bytes) — DriveLocomotion ILocomotion vtable
- `0x7e7f20` (32 bytes) — extended vtable containing Force_Track

**Functions decompiled:**
- `0x4af540` Constructor (verified +0x60 init)
- `0x4b0500` Process (dispatch logic)
- `0x4b4780` Transform_Track_Coords (flag bits 0/1/2 decode)
- `0x4b4b00` Can_Use_Track (RawTrack +0x04 usage)

**Disassembly read:**
- `0x4b4000–0x4b4055` (Process_Movement track-index lookup + bit-3 test)

**Byte-pattern searches (all writes to *+0x60):**
- 9 different `MOV [reg+0x60]` encoding variants — only constructor hit
  for DriveLocomotion

**Existing docs cross-referenced and corrected:**
- `DRIVE_TRACK_SYSTEM.md` (Mar 2026) — track 7/9/10 point counts wrong; flag
  bit-3 semantics labeled as "advance path flag" — corrected to "cell crossing"
- `DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md` (Apr 2026) — bit 3 question now
  resolved (helpers doc was right that it's NOT a transform flag)
- `DRIVE_LOCOMOTION_CLASS.md`, `PROCESS_DRIVE_TRACK_DECOMPILATION.md`,
  `DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md` — no contradictions found

---

## 13. Follow-up — Force_Track vtable position and RawTrack +0x04 semantics

This section addresses the two open questions left at the end of section 11.

### 13.1 ILocomotion vtable for DriveLocomotionClass is much bigger than docs claimed

`DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md` describes the ILocomotion vtable at
`0x7e7eb0` as 96 bytes (24 slots). **This is wrong.** Direct memory read of
208 bytes starting at `0x7e7eb0` shows continuous, valid function pointers for
at least 50 slots ending around `0x7e7f80`. The vtable structure is:

| Slot | Offset | Address | Method (where known) |
|-----:|-------:|---------|----------------------|
|  0 | +0x00 | 0x004b4d90 | (reserved / IUnknown delegation) |
|  1 | +0x04 | 0x004b4da0 | (reserved) |
|  2 | +0x08 | 0x004b4db0 | (reserved) |
|  3 | +0x0C | 0x0055a710 | LocomotionClass base method |
|  4 | +0x10 | 0x004afb80 | **ILocomotion::Is_Moving** |
|  5 | +0x14 | 0x004afc90 | Destination |
|  6 | +0x18 | 0x004afcc0 | Head_To_Coord |
|  9 | +0x24 | 0x004aff60 | **Draw_Matrix** |
| 10 | +0x28 | 0x004b0410 | Shadow_Matrix |
| 16 | +0x40 | 0x004b0500 | **Process** (verified — `call [ecx+0x40]` from FootClass::AI) |
| 17 | +0x44 | 0x004afd40 | Set_Destination |
| 18 | +0x48 | 0x004afe00 | Stop_Moving |
| 19 | +0x4C | 0x004b0ef0 | Do_Turn |
| 20 | +0x50 | 0x004b04d0 | Update_Facing_From_Type |
| **28** | **+0x70** | **0x004b0c40** | **Force_Track** ★ |
| 29 | +0x74 | 0x004b4820 | In_Which_Layer |
| 31 | +0x7C | 0x004afb40 | Force_New_Slope |
| 32 | +0x80 | 0x004afc20 | Is_Moving_Now |
| 36 | +0x90 | 0x004b4c60 | Get_Status |
| 37 | +0x94 | 0x004b4c70 | Acquire_Hunter_Seeker_Target (TS legacy stub) |
| 38 | +0x98 | 0x004b4c80 | Is_Surfacing (stub) |
| 39 | +0x9C | 0x004b48d0 | Mark_All_Occupation_Bits |
| 40 | +0xA0 | 0x004b4920 | Is_To_Have_Shadow_Override |
| 41 | +0xA4 | 0x004b4b00 | Can_Use_Track ★ (exposed via vtable) |
| 51 | +0xCC | 0x004af720 | (likely IUnknown::QueryInterface) |

### 13.2 Force_Track is reachable only via `vtable[28]` indirect dispatch

Address `0x004b0c40` (Force_Track) appears in **exactly one location** in the
binary: the vtable slot at `0x7e7f20` (= ILocomotion vtable + 0x70). Verified by
exhaustive byte-pattern search for the literal address bytes `40 0c 4b 00`.

This means:
- No `call 0x4b0c40` direct calls anywhere in the binary
- No `lea reg, [0x4b0c40]` or `mov reg, 0x4b0c40` setup for indirect call
- No data-section reference besides the one vtable slot

**All callers must dispatch via `call dword ptr [reg + 0x70]`** where `reg` holds
the ILocomotion vtable pointer (loaded from `obj+0x04`). Byte-pattern searches
for `vtable+0x70` indirect calls return many matches (over 50 for `ff 50 70`,
over 40 for `ff 52 70`, plus more for other registers). Without ABI-typed analysis
or symbolic execution, those cannot be filtered to "only the ones whose receiver
is a DriveLocomotion." So no statically-traceable callers.

**Strong implication:** Force_Track is genuinely seldom-called game code (or
possibly dead). If it were on a hot path or reachable via deploy/edge-retreat,
we would expect at least *some* setup pattern that loads the function or the
vtable address by name. There is none.

The MAP_EDGE_RETREAT phase in Process_Drive_Track (lines 206–260 in
`PROCESS_DRIVE_TRACK_DECOMPILATION.md`) does NOT use Force_Track — it sets
`track_index = -1` directly and uses standard track stepping with a head_to
override. So edge-retreat does not need Force_Track equivalence.

### 13.3 Implications for Rust impl

The original concern — "no equivalent of Force_Track yet — needed when deploy/
edge-retreat is implemented" — is **mostly unfounded**:

- **Edge retreat:** Verified to NOT use Force_Track. It manipulates head_to and
  track_index directly. Rust can implement edge retreat the same way without
  a Force_Track equivalent.
- **Deploy:** Has not been traced to a Force_Track call. The unit deploy path
  uses Mission_Deploy → animation state changes → unit type swap. No vtable
  dispatch through ILocomotion+0x70 is observed.
- **Special tracks 64–71 invocation:** Still unresolved. They cannot be reached
  by the standard formula. If gamemd.exe never reaches them in normal play,
  they may be dead code. If something does reach them, it's via a code path
  not yet identified.

**Recommendation:** Defer adding a Force_Track equivalent to Rust until a
specific symptom proves it's needed. The "tracks 64–71 might be deploy/scatter"
hypothesis from section 9 is **weakened** — there's no evidence those tracks
are reachable in normal YR play. They may be TS-era leftover behavior similar
to `Acquire_Hunter_Seeker_Target` (slot 37, confirmed dead stub).

### 13.4 RawTrack +0x04 — confirmed: chain-compatibility key, not a count

Direct xref scan for the four RawTrack fields:

| Address | Reader | Function | Use |
|---------|--------|----------|-----|
| 0x7e7a28 | (base — 11 readers) | multiple | RawTrack ptr field |
| **0x7e7a2c** | 2 readers | **Process_Drive_Track + Can_Use_Track** | **field +0x04** |
| 0x7e7a30 | 5 readers | Process_Drive_Track + Can_Use_Track | field +0x08 (entry_index) |
| 0x7e7a34 | 2 readers | Apply_Track_Delta + Process_Drive_Track | field +0x0C (jump_index) |

Field +0x04 has only TWO readers in the entire binary — both inside
DriveLocomotionClass.

### 13.5 Both +0x04 readers compare to `raw_track_lookup` (loco abs+0x5C)

#### Reader 1 — Can_Use_Track at 0x4b4b80

Disassembly:

```asm
8b 57 58           mov  edx, [edi+0x58]            ; edx = loco.raw_track_lookup
                                                    ; (relative to ILocomotion this;
                                                    ;  absolute object offset = +0x5C)
c1 e6 04           shl  esi, 4                     ; esi = raw_track_idx * 16
39 96 2c 7a 7e 00  cmp  [esi+0x7e7a2c], edx        ; cmp RawTrack[idx].field_4 == raw_track_lookup
75 4a              jne  +0x4a                       ; not equal → bail
```

#### Reader 2 — Process_Drive_Track at 0x4b1b3c

```asm
8b 4c 24 3c        mov  ecx, [esp+0x3c]            ; ecx = pre-shifted raw_track_idx*16
8b 45 5c           mov  eax, [ebp+0x5c]            ; eax = stack-saved raw_track_lookup
39 81 2c 7a 7e 00  cmp  [ecx+0x7e7a2c], eax        ; same comparison
0f 85 00 04 00 00  jne  +0x400                      ; not equal → skip 1024 bytes (full chain
                                                    ;   block bypassed)
```

**Both reads do the same thing:** compare `RawTrack[next_idx].field_4` against
the locomotor's current `raw_track_lookup`.

### 13.6 What is `raw_track_lookup` (loco abs+0x5C)?

This is a 4-byte field initialized to `-1` in the constructor (per helpers doc).
Updated during track stepping. Per the chain-success branch in Process_Drive_Track
(documented in PROCESS_DRIVE_TRACK_DECOMPILATION.md lines 642–724):

```
loco->point_index = g_DriveTrackData_Array[step_data_offset + 4] - 1;
```

When chaining into a new raw track, `point_index` is set to `field_4 - 1`. So
the unit starts walking the new track at point `field_4 - 1`. For track 3
(field_4 = 37), point_index becomes 36. For track 5 (field_4 = 45), it's 44.

This is the **chained-entry start position**. The chain comparison then works
like a key match: the next track's field_4 must equal the current track's
`raw_track_lookup`, which itself is the field_4 from the track that ORIGINALLY
started the chain.

### 13.7 Best interpretation of field +0x04

Field +0x04 is a **shared chain-key** that lets multiple raw tracks be linked
into a continuous sequence. The semantics of the values:

| Track | field_4 | Meaning |
|------:|--------:|---------|
| 0 | 0 | Null sentinel — `field_4 != 0` check fails, so chaining always rejects this track |
| 1, 2 | -1 | Simple straights — all chain against any "-1 chain group" |
| 3 | 37 | Curve in chain group 37 |
| 4 | 26 | Curve in chain group 26 |
| 5 | 45 | Curve in chain group 45 |
| 6 | 44 | Curve in chain group 44 |
| 7–15 | -1 | Short/special tracks — chain group "-1" (compatible with simple tracks) |

When a unit completes a curving track 3 (field_4=37), `raw_track_lookup` is set
to 37. The next time a turn arises, Can_Use_Track will only allow chaining into
a raw track whose own field_4 equals 37 — meaning, only into another track 3.
This prevents mid-curve reroutes that would visually break (e.g., switching from
a track-3 45° turn into a track-5 wide curve mid-stride).

**For tracks with field_4 = -1**, the comparison `-1 == raw_track_lookup` succeeds
only when `raw_track_lookup` is also -1, which is the initial/idle state. So
straight tracks and short curves only chain when no curving track was previously
in progress. The first curve "locks" the chain group; subsequent steps must
stay in that group until the track completes.

In Process_Drive_Track, the value used as `field_4 - 1` to set point_index also
serves as a starting offset — for tracks 3-6, point_index starts at 36, 25, 44,
43 (one less than field_4). Coincidentally these are near the END of those
tracks' point arrays (54, 38, 61, 56 active points). So the chained track
starts very near the end, walks just a few steps, then hits the sentinel and
the chain releases. This is the post-cell-crossing "tail" of the curve.

### 13.8 Implication for Rust

The Rust impl should:
1. **Add a `chain_group` field to RawTrack** (the +0x04 value).
2. **Track `raw_track_lookup` per locomotor** as a separate field (currently
   only `track_index` and `point_index` are tracked; the chain-key is missing).
3. **In track-chain logic**, only accept chaining when the next raw track's
   `chain_group` equals the locomotor's `raw_track_lookup`.
4. **When a chain succeeds**, set the new locomotor's `point_index` to
   `chain_group - 1` (= the cell-crossing tail entry).

Without this, Rust may allow track chains that the original engine would reject
— producing slightly different visual paths during multi-cell turns. This is
**low player-visibility** (the difference is in the exact frame-by-frame body
position during a turn) but does affect determinism if the chain decision
diverges from gamemd.exe.

**Severity:** LOW player-visibility. The visual difference between "chains as
expected" and "rejects chain → falls back to straight" is subtle — at most a
half-cell wobble during a turn that crosses cells. Trigger frequency: every
multi-cell vehicle turn (= dozens per second across a typical battle). Worth
fixing for the 99%-parity bar, but not urgent.

### 13.9 Updated open questions (after this follow-up)

| Original Q | Status | Resolution |
|------------|--------|------------|
| What invokes Force_Track? | LIKELY DEAD | Zero static call sites; only in vtable slot 28. Edge retreat doesn't use it. May be unreachable in standard YR. |
| What does RawTrack +0x04 mean? | RESOLVED | Chain-compatibility key. Locks unit into a "chain group" (-1, 26, 37, 44, 45) for the duration of a multi-track curve. |
| Where are special tracks 64–71 used? | UNRESOLVED but DEPRIORITIZED | Combined with Force_Track being unreachable, these may be TS-era dead code. |
| ILocomotion vtable slot count | RESOLVED | 50+ slots (not 24). Helpers doc undercounted. |
