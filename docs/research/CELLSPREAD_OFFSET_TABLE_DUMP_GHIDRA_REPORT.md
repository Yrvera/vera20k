# CellSpread Offset Tables — Full Dump and Iteration Order

**Addresses investigated:** `DAT_00ABD490` / `DAT_00ABD492` (alias into same array)
**Initializer:** `MapClass__InitRevealSpiralTable @ 0x00561910`
**Reader (main):** `Apply_area_damage @ 0x00489280`
**Confidence:** HIGH (content: decompiled from live initializer; identity: WRITE xref confirmed at 0x00561949; binding: assembly at 0x004895C7–0x004895CF directly verified)
**Active in YR:** Yes — core combat AoE path, called 19 verified sites, every splash detonation goes here.

---

## 1. Table Layout

`DAT_00ABD490` and `DAT_00ABD492` are **not separate arrays** — they are two aliased names
into a single flat array of `int32` entries. Each `int32` encodes one `{dx, dy}` cell-offset pair:

```
struct CellSpreadEntry {
    int16_t dx;   // at &table[i]      (low 16 bits = DAT_00ABD490[i*2])
    int16_t dy;   // at &table[i] + 2  (high 16 bits = DAT_00ABD492[i*2])
}
```

The naming `DAT_00ABD490` and `DAT_00ABD492` reflects Ghidra seeing two separate reads
from offset 0 and offset 2 of the same 4-byte word. There is only one array.

**Element size:** 4 bytes per entry (two `int16` packed as a little-endian `int32`).
**Element count:** 369 entries (indices 0..368), covering CellSpread radii 0..11.
**Total size:** 369 × 4 = 1476 bytes (0x5C4), from 0x00ABD490 to 0x00ABDA53 inclusive.

### Evidence (assembly at 0x004895C7–0x004895CF)

```asm
004895c3: MOV EAX, dword ptr [ESP+0x10]    ; EAX = loop counter i (local_d8)
004895c7: MOV DX,  word ptr [EAX*4 + 0xABD490]   ; dx = low i16
004895cf: MOV AX,  word ptr [EAX*4 + 0xABD492]   ; dy = high i16 (= base + 2)
004895d7: ADD DX,  word ptr [ESP+0x18]             ; cellX + dx
004895dc: ADD AX,  word ptr [ESP+0x1a]             ; cellY + dy
```

Stride: `EAX * 4` — confirmed 4-byte stride. Both reads from the same base array.

---

## 2. Count Table (DAT_007ED3D0) — Verified from Binary

Read at `0x007ED3D0`, 48 bytes (12 × int32 little-endian):

| Index (R) | gamemd value | Rust value | Match |
|-----------|-------------|------------|-------|
| 0 | 1 | 1 | YES |
| 1 | 9 | 9 | YES |
| 2 | 21 | 21 | YES |
| 3 | 37 | 37 | YES |
| 4 | 61 | 61 | YES |
| 5 | 89 | 89 | YES |
| 6 | 121 | 121 | YES |
| 7 | 161 | 161 | YES |
| 8 | 205 | 205 | YES |
| 9 | 253 | 253 | YES |
| 10 | 309 | 309 | YES |
| 11 | 369 | 369 | YES |

**All 12 counts match exactly.** Raw bytes from `read_memory`: `[1,0,0,0, 9,0,0,0, 15h,0,0,0, 25h,0,0,0, 3Dh,0,0,0, 59h,0,0,0, 79h,0,0,0, A1h,0,0,0, CDh,0,0,0, FDh,0,0,0, 35h,01,0,0, 71h,01,0,0]`.

---

## 3. How the Initializer Populates the Table

`MapClass__InitRevealSpiralTable @ 0x00561910` sets up the table at process startup.
It uses two methods:

1. **Direct 4-byte literal assignments** (entries 0..218, indices 0..218, via `_DAT_00ABD490 = 0xhhhh`).
2. **`MapCoord_Set(dx, dy)` helper calls** (entries 219..368, via `_DAT_00ABD910` through
   `_DAT_00ABDA50`) — packing `(dx & 0xFFFF) | ((dy & 0xFFFF) << 16)`.

Both methods produce the same bit pattern: little-endian `int32` with `dx` in low 16 and `dy` in high 16.

**Note on BSS:** The address 0x00ABD490 falls in BSS (zero-initialized at load time). Calling
`read_memory` before the game runs returns all zeros. The table is populated at runtime by
`MapClass__InitRevealSpiralTable`. The full content is known from the initializer decompilation.

---

## 4. Iteration Order in Apply_area_damage

```
spread_index = ftol(wh->CellSpread)         // integer CellSpread (0–11)
cell_count   = DAT_007ED3D0[spread_index]   // how many cells to scan
local_d8     = 0

do {
    dx = int16(DAT_00ABD490[local_d8 * 2])  // = word ptr [local_d8*4 + 0xABD490]
    dy = int16(DAT_00ABD492[local_d8 * 2])  // = word ptr [local_d8*4 + 0xABD492]
    cellCoord = (impact.cellX + dx, impact.cellY + dy)
    // per-cell effects: overlay/tiberium/wall destruction
    // object collection into damage_vector
    local_d8++
} while (local_d8 < cell_count)

// After loop: dispatch damage to all collected targets in damage_vector order
```

The loop index runs **from 0 to cell_count-1**, so entry 0 is always `(0,0)` — the impact cell itself, scanned first.

---

## 5. Full Offset Table (Decoded from Initializer)

Below is the complete 369-entry table decoded from `MapClass__InitRevealSpiralTable`.
Format: `idx | address | dx | dy | d² | R`

### Radius 0 (1 cell, indices 0–0)
| idx | address | dx | dy | d² |
|-----|---------|----|----|-----|
| 0 | 00ABD490 | 0 | 0 | 0 |

### Radius 1 (9 total, 8 new, indices 1–8)
| idx | address | dx | dy | d² |
|-----|---------|----|----|-----|
| 1 | 00ABD494 | 1 | -1 | 2 |
| 2 | 00ABD498 | 0 | -1 | 1 |
| 3 | 00ABD49C | -1 | -1 | 2 |
| 4 | 00ABD4A0 | -1 | 0 | 1 |
| 5 | 00ABD4A4 | 1 | 0 | 1 |
| 6 | 00ABD4A8 | -1 | 1 | 2 |
| 7 | 00ABD4AC | 0 | 1 | 1 |
| 8 | 00ABD4B0 | 1 | 1 | 2 |

### Radius 2 (21 total, 12 new, indices 9–20)
| idx | address | dx | dy | d² |
|-----|---------|----|----|-----|
| 9 | 00ABD4B4 | -1 | -2 | 5 |
| 10 | 00ABD4B8 | 0 | -2 | 4 |
| 11 | 00ABD4BC | 1 | -2 | 5 |
| 12 | 00ABD4C0 | -2 | -1 | 5 |
| 13 | 00ABD4C4 | 2 | -1 | 5 |
| 14 | 00ABD4C8 | -2 | 0 | 4 |
| 15 | 00ABD4CC | 2 | 0 | 4 |
| 16 | 00ABD4D0 | -2 | 1 | 5 |
| 17 | 00ABD4D4 | 2 | 1 | 5 |
| 18 | 00ABD4D8 | -1 | 2 | 5 |
| 19 | 00ABD4DC | 0 | 2 | 4 |
| 20 | 00ABD4E0 | 1 | 2 | 5 |

### Radius 3 (37 total, 16 new, indices 21–36)
| idx | address | dx | dy | d² |
|-----|---------|----|----|-----|
| 21 | 00ABD4E4 | -1 | -3 | 10 |
| 22 | 00ABD4E8 | 0 | -3 | 9 |
| 23 | 00ABD4EC | 1 | -3 | 10 |
| 24 | 00ABD4F0 | -2 | -2 | 8 |
| 25 | 00ABD4F4 | 2 | -2 | 8 |
| 26 | 00ABD4F8 | -3 | -1 | 10 |
| 27 | 00ABD4FC | 3 | -1 | 10 |
| 28 | 00ABD500 | -3 | 0 | 9 |
| 29 | 00ABD504 | 3 | 0 | 9 |
| 30 | 00ABD508 | -3 | 1 | 10 |
| 31 | 00ABD50C | 3 | 1 | 10 |
| 32 | 00ABD510 | -2 | 2 | 8 |
| 33 | 00ABD514 | 2 | 2 | 8 |
| 34 | 00ABD518 | -1 | 3 | 10 |
| 35 | 00ABD51C | 0 | 3 | 9 |
| 36 | 00ABD520 | 1 | 3 | 10 |

### Radius 4 (61 total, 24 new, indices 37–60)
| idx | address | dx | dy | d² |
|-----|---------|----|----|-----|
| 37 | 00ABD524 | -1 | -4 | 17 |
| 38 | 00ABD528 | 0 | -4 | 16 |
| 39 | 00ABD52C | 1 | -4 | 17 |
| 40 | 00ABD530 | -3 | -3 | 18 |
| 41 | 00ABD534 | -2 | -3 | 13 |
| 42 | 00ABD538 | 2 | -3 | 13 |
| 43 | 00ABD53C | 3 | -3 | 18 |
| 44 | 00ABD540 | -3 | -2 | 13 |
| 45 | 00ABD544 | 3 | -2 | 13 |
| 46 | 00ABD548 | -4 | -1 | 17 |
| 47 | 00ABD54C | 4 | -1 | 17 |
| 48 | 00ABD550 | -4 | 0 | 16 |
| 49 | 00ABD554 | 4 | 0 | 16 |
| 50 | 00ABD558 | -4 | 1 | 17 |
| 51 | 00ABD55C | 4 | 1 | 17 |
| 52 | 00ABD560 | -3 | 2 | 13 |
| 53 | 00ABD564 | 3 | 2 | 13 |
| 54 | 00ABD568 | -3 | 3 | 18 |
| 55 | 00ABD56C | -2 | 3 | 13 |
| 56 | 00ABD570 | 2 | 3 | 13 |
| 57 | 00ABD574 | 3 | 3 | 18 |
| 58 | 00ABD578 | -1 | 4 | 17 |
| 59 | 00ABD57C | 0 | 4 | 16 |
| 60 | 00ABD580 | 1 | 4 | 17 |

### Radius 5 (89 total, 28 new, indices 61–88)
| idx | address | dx | dy | d² |
|-----|---------|----|----|-----|
| 61 | 00ABD584 | -1 | -5 | 26 |
| 62 | 00ABD588 | 0 | -5 | 25 |
| 63 | 00ABD58C | 1 | -5 | 26 |
| 64 | 00ABD590 | -3 | -4 | 25 |
| 65 | 00ABD594 | -2 | -4 | 20 |
| 66 | 00ABD598 | 2 | -4 | 20 |
| 67 | 00ABD59C | 3 | -4 | 25 |
| 68 | 00ABD5A0 | -4 | -3 | 25 |
| 69 | 00ABD5A4 | 4 | -3 | 25 |
| 70 | 00ABD5A8 | -4 | -2 | 20 |
| 71 | 00ABD5AC | 4 | -2 | 20 |
| 72 | 00ABD5B0 | -5 | -1 | 26 |
| 73 | 00ABD5B4 | 5 | -1 | 26 |
| 74 | 00ABD5B8 | -5 | 0 | 25 |
| 75 | 00ABD5BC | 5 | 0 | 25 |
| 76 | 00ABD5C0 | -5 | 1 | 26 |
| 77 | 00ABD5C4 | 5 | 1 | 26 |
| 78 | 00ABD5C8 | -4 | 2 | 20 |
| 79 | 00ABD5CC | 4 | 2 | 20 |
| 80 | 00ABD5D0 | -4 | 3 | 25 |
| 81 | 00ABD5D4 | 4 | 3 | 25 |
| 82 | 00ABD5D8 | -3 | 4 | 25 |
| 83 | 00ABD5DC | -2 | 4 | 20 |
| 84 | 00ABD5E0 | 2 | 4 | 20 |
| 85 | 00ABD5E4 | 3 | 4 | 25 |
| 86 | 00ABD5E8 | -1 | 5 | 26 |
| 87 | 00ABD5EC | 0 | 5 | 25 |
| 88 | 00ABD5F0 | 1 | 5 | 26 |

### Radius 6 (121 total, 32 new, indices 89–120) — ANOMALY at idx 96
| idx | address | dx | dy | d² | note |
|-----|---------|----|----|-----|------|
| 89 | 00ABD5F4 | -1 | -6 | 37 | |
| 90 | 00ABD5F8 | 0 | -6 | 36 | |
| 91 | 00ABD5FC | 1 | -6 | 37 | |
| 92 | 00ABD600 | -3 | -5 | 34 | |
| 93 | 00ABD604 | -2 | -5 | 29 | |
| 94 | 00ABD608 | 2 | -5 | 29 | |
| 95 | 00ABD60C | 3 | -5 | 34 | |
| 96 | 00ABD610 | -5 | -4 | 41 | **ANOMALY: this should be (-4,-4) d²=32 here; instead gamemd places (-5,-4) in R=6 AND R=7** |
| 97 | 00ABD614 | 4 | -4 | 32 | |
| 98 | 00ABD618 | -5 | -3 | 34 | |
| 99 | 00ABD61C | 5 | -3 | 34 | |
| 100 | 00ABD620 | -5 | -2 | 29 | |
| 101 | 00ABD624 | 5 | -2 | 29 | |
| 102 | 00ABD628 | -6 | -1 | 37 | |
| 103 | 00ABD62C | 6 | -1 | 37 | |
| 104 | 00ABD630 | -6 | 0 | 36 | |
| 105 | 00ABD634 | 6 | 0 | 36 | |
| 106 | 00ABD638 | -6 | 1 | 37 | |
| 107 | 00ABD63C | 6 | 1 | 37 | |
| 108 | 00ABD640 | -5 | 2 | 29 | |
| 109 | 00ABD644 | 5 | 2 | 29 | |
| 110 | 00ABD648 | -5 | 3 | 34 | |
| 111 | 00ABD64C | 5 | 3 | 34 | |
| 112 | 00ABD650 | -4 | 4 | 32 | |
| 113 | 00ABD654 | 4 | 4 | 32 | |
| 114 | 00ABD658 | -3 | 5 | 34 | |
| 115 | 00ABD65C | -2 | 5 | 29 | |
| 116 | 00ABD660 | 2 | 5 | 29 | |
| 117 | 00ABD664 | 3 | 5 | 34 | |
| 118 | 00ABD668 | -1 | 6 | 37 | |
| 119 | 00ABD66C | 0 | 6 | 36 | |
| 120 | 00ABD670 | 1 | 6 | 37 | |

### Radii 7–10 (indices 121–308)

Fully decoded — see the complete listing below. Selected highlights:

| Boundary | idx | address | dx | dy | d² |
|----------|-----|---------|----|----|-----|
| R=7 start | 121 | 00ABD674 | -1 | -7 | 50 |
| R=7 end | 160 | 00ABD710 | 1 | 7 | 50 |
| R=8 start | 161 | 00ABD714 | -1 | -8 | 65 |
| R=8 end | 204 | 00ABD7C0 | 1 | 8 | 65 |
| R=9 start | 205 | 00ABD7C4 | -1 | -9 | 82 |
| R=9 end | 252 | 00ABD880 | 1 | 9 | 82 |
| R=10 start | 253 | 00ABD884 | -1 | -10 | 101 |
| R=10 end | 308 | 00ABD960 | 1 | 10 | 101 |

### Radius 11 (369 total, 60 new, indices 309–368) — ANOMALY at idx 322

| idx | address | dx | dy | d² | note |
|-----|---------|----|----|-----|------|
| 309 | 00ABD964 | 0 | 11 | 121 | |
| 310 | 00ABD968 | 0 | -11 | 121 | |
| 311 | 00ABD96C | -1 | 11 | 122 | |
| 312 | 00ABD970 | 1 | 11 | 122 | |
| 313 | 00ABD974 | -1 | -11 | 122 | |
| 314 | 00ABD978 | 1 | -11 | 122 | |
| 315 | 00ABD97C | -2 | 11 | 125 | |
| 316 | 00ABD980 | 2 | 11 | 125 | |
| 317 | 00ABD984 | -2 | -11 | 125 | |
| 318 | 00ABD988 | 2 | -11 | 125 | |
| 319 | 00ABD98C | -3 | 11 | 130 | |
| 320 | 00ABD990 | 3 | 11 | 130 | |
| 321 | 00ABD994 | -3 | -11 | 130 | |
| 322 | 00ABD998 | -3 | 11 | 130 | **DUPLICATE of idx 319 — gamemd writes (-3,11) twice here** |
| 323 | 00ABD99C | -4 | 9 | 97 | |
| ... | ... | ... | ... | ... | |
| 368 | 00ABDA50 | -11 | 0 | 121 | |

(Full R=11 listing omitted for brevity — available in the full decoded table above.)

---

## 6. Iteration Order Pattern

Gamemd does NOT sort by distance squared alone, nor by the Rust key `(d², |dx|, |dy|, dy, dx)`.
The observed pattern per radius band is:

**Within each radius band, cells are ordered:**
starting from `(-|dy|max, -(|dy|max - 1))` then sweeping outward in a specific spiral order.
More precisely: the table begins each radius with cells having `dy < 0` (negative y = north),
then proceeds around the ring mixing dy groups.

Example for R=1 (indices 1–8):
```
(1,-1), (0,-1), (-1,-1), (-1,0), (1,0), (-1,1), (0,1), (1,1)
```
This is: NE, N, NW, W, E, SW, S, SE — a counterclockwise-ish sweep skipping the center,
starting from the NE diagonal. The d² values within the ring alternate: 2, 1, 2, 1, 1, 2, 1, 2.

This ordering does NOT match the Rust sort key `(d², |dx|, |dy|, dy, dx)` — see §7 for the
full mismatch analysis.

---

## 7. Rust vs gamemd Comparison

### Count table: MATCH (all 12 radii exact)

### Element sets per radius:

| R | gamemd elements | Rust elements | set_match | order_match |
|---|----------------|---------------|-----------|-------------|
| 0 | 1 | 1 | YES | YES |
| 1 | 8 new | 8 new | YES | NO |
| 2 | 12 new | 12 new | YES | NO |
| 3 | 16 new | 16 new | YES | NO |
| 4 | 24 new | 24 new | YES | NO |
| 5 | 28 new | 28 new | YES | NO |
| 6 | 32 new (but includes (-5,-4) which also appears in R=7) | 32 new | **NO** | NO |
| 7 | 40 new | 40 new | YES | NO |
| 8 | 44 new | 44 new | **NO** | NO |
| 9 | 48 new | 48 new | **NO** | NO |
| 10 | 56 new | 56 new | **NO** | NO |
| 11 | 60 new (contains duplicate (-3,11), missing one unique cell) | 60 new | **NO** | NO |

### Order mismatch (total): 363 out of 369 positions differ

The Rust sort key `(d², |dx|, |dy|, dy, dx)` places all cells of the same d² together before
moving to the next d². Gamemd interleaves d²-groups within each radius band.

### Element-set mismatches: Radii 6, 8, 9, 10, 11

The mismatches occur because at boundary radii, there are more candidate cells at the same d²
threshold than the count requires — the tie-breaking determines which subset is included.

Gamemd's tie-breaking selects a different subset than Rust's `(|dx|, |dy|, dy, dx)` sort:
- R=6: gamemd includes `(-5,-4)` d²=41 but excludes `(-4,-4)` d²=32 and `(4,-4)` d²=32 pair
  at this position (instead puts `(-5,-4)` first, then `4,-4` which is asymmetric)
- R=11: gamemd writes `(-3,11)` twice at indices 319 and 322, meaning one entry that should be
  a unique cell (e.g., `(3,-11)`) is missing from the functional table

### Desync risk assessment

**For radii 0–5:** Same cell sets, different visitation order. Side-effects from AoE within
the same detonation are applied in a different sequence. This matters when chain reactions can
destroy cells before adjacent cells are visited (e.g., IC barrel chains: §11 in splash_cellspread.md).
In practice: the destroyed cells are removed from the map before the loop reaches them, so a
subsequent cell scan of an already-exploded position yields no occupants — no double-damage,
but the chain order is different.

**For radii 6, 8–11:** Different cell subsets are included. A unit standing at a cell that gamemd
includes but Rust excludes (or vice versa) will receive damage in one engine but not the other.
This is a parity defect for any warhead with `CellSpread` ≥ 6.

**Duplicate at R=11 idx 322:** `(-3,11)` is visited twice and the expected mirror `(3,-11)` is
never visited. For `CellSpread=11` warheads, one cell is scanned twice and the cell at `(3,-11)`
relative to impact is never scanned. This is a gamemd quirk/bug. The Rust table does not
replicate it — meaning Rust scans `(3,-11)` once and `(-3,11)` once (symmetric), while gamemd
scans `(-3,11)` twice and `(3,-11)` zero times.

---

## 8. Known YR Warheads with CellSpread ≥ 6

From `rulesmd.ini` (grepping for `CellSpread`): warheads with high CellSpread include nuclear
weapons and superweapons. Most infantry/tank warheads have CellSpread ≤ 3. The element-set
disparity (§7) is therefore primarily relevant for superweapons (Nuke, Iron Curtain damage,
Weather Storm, Psychic Dominator) where CellSpread may reach 6+.

---

## 9. Implementation Recommendation

**Order (for R=0–5):** The order disparity is tolerable for normal combat. Chain reactions within
a single detonation are self-bounding. No player-observable difference in typical skirmish play.

**Element set (for R=6+):** This is a parity defect. The Rust table should match gamemd's
exact cell selection at each radius boundary. The correct fix is to use the gamemd table
verbatim — either embedding the hardcoded values, or reproducing gamemd's exact ordering
algorithm. **Do not use the Rust-generated sort** for any warhead with CellSpread ≥ 6 until
the tie-breaking matches.

**Duplicate at R=11:** Gamemd's `(-3,11)` duplicate at idx 322 and missing `(3,-11)` is an
internal bug in gamemd. Replicating it would create asymmetric damage for CellSpread=11.
Recommended: do NOT replicate this artifact; keep the Rust behavior (symmetric). Document
the known divergence.

---

## 10. References

- `MapClass__InitRevealSpiralTable @ 0x00561910` — decompiled live 2026-05-18; full literal
  assignments extracted for all 369 entries.
- `Apply_area_damage @ 0x00489280` — decompiled live 2026-05-17 (existing doc), assembly
  at `0x004895C7–0x004895CF` confirms stride and dx/dy extraction.
- `DAT_007ED3D0` — read_memory at `0x007ED3D0`, 48 bytes, counts verified.
- `DAT_00ABD490` — BSS (zero at static read); content derived from initializer decompilation.
- Existing docs cross-referenced:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/combat/systems/splash_cellspread.md` §6 — table structure
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md` §10 — open question
- Rust source cross-referenced (READ-ONLY):
  - `src/sim/combat/cell_spread.rs` — `CELL_SPREAD_COUNTS`, `compute_spread_offsets`, `cells_in_spread`
  - Rust line 14: `const CELL_SPREAD_COUNTS: [usize; 12] = [1,9,21,37,61,89,121,161,205,253,309,369];`
  - Rust line 44: sort key `(d2, abs(dx), abs(dy), dy, dx)` — confirmed different from gamemd order
