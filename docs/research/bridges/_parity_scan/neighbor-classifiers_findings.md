# Parity Scan — neighbor-classifiers (Perpendicular neighbor classifiers + pick_destruction_overlay table)

Facet: `CheckBridgeNeighbors_{NS,EW}_{High,Low}` 4-bit assignment + east-first/north-first
switch order, and the 0..15-index `pick_destruction_overlay` output table for each axis/level.

Rust source: `src/sim/bridge_state/walker.rs` (`check_bridge_neighbors_*`), `src/sim/bridge_specs.rs`
(`pick_destruction_overlay` + 4 `DESTRUCTION_OVERLAY_*` tables).

gamemd anchors verified live this session:
- `CheckBridgeNeighbors_NS_High @ 0x0057CBE0` (identity confirmed via `get_function_by_address`)
- `CheckBridgeNeighbors_EW_High @ 0x0057CAB0`
- `CheckBridgeNeighbors_NS_Low  @ 0x0057B990`
- `CheckBridgeNeighbors_EW_Low  @ 0x0057B870`
- `ApplyBridgeDestruction_NS_High @ 0x0057E7A0` (holds `local_70` HIGH NS table)
- `ApplyBridgeDestruction_EW_High @ 0x0057ED00` (HIGH EW table)
- `ApplyBridgeDestruction_NS_Low  @ 0x0057DD50` (LOW NS table)
- `ApplyBridgeDestruction_EW_Low  @ 0x0057E2A0` (LOW EW table)
- Sentinel cell `DAT_00abdc50` read 64 bytes → all zero → overlay byte at +0x44 = 0.

## Result: NO DRIFT FOUND in this facet.

Every bit-membership set, every probe→geometry mapping, the switch ordering, and all 64
table entries (4 tables × 16) match the binary byte-for-byte. The two candidate divergences
(off-map sentinel substitution; second-switch early-return) are both proven observationally
identical. One genuine edge-frame divergence exists (column-0 / row-wrap neighbor read) but
is unreachable by any placed bridge in a real YR map, so it is noted as UNCHECKED-not-reachable
rather than a reportable gap.

---

## PARITY-CONFIRMED

### P1: HIGH NS classifier bit membership + geometry — `check_bridge_neighbors_ns_high` (walker.rs:667)
Binary `0x0057CBE0`: probe `puVar2 = (X, Y-1)` = NORTH; probe `puVar3 = (X, Y+1)` = SOUTH.
- First switch on NORTH: `{0xDA,0xDC,0xDE,0xE4} → idx|=1`; `{0xDD,0xE8} → idx|=2`.
- Second switch on SOUTH: `{0xDB,0xDC,0xDD,0xE6} → return idx|4`; `{0xDE,0xE8} → idx|=8`.
Rust (walker.rs:678-687): north `{0xDA,0xDC,0xDE,0xE4}=>1`, `{0xDD,0xE8}=>2`; south
`{0xDB,0xDC,0xDD,0xE6}=>4`, `{0xDE,0xE8}=>8`. IDENTICAL.
Verify-call: `decompile_function 0x0057cbe0`.

### P2: HIGH EW classifier bit membership + geometry — `check_bridge_neighbors_ew_high` (walker.rs:636)
Binary `0x0057CAB0`: `puVar2 = (X-1, Y)` = WEST; `puVar3 = (X+1, Y)` = EAST. First switch
operates on **puVar3 (EAST)**, second on **puVar2 (WEST)**.
- EAST: `{0xD1,0xD3,0xD5,0xE0} → 1`; `{0xD4,0xE7} → 2`.
- WEST: `{0xD2,0xD3,0xD4,0xE2} → 4`; `{0xD5,0xE7} → 8`.
Rust (walker.rs:647-656): east `{0xD1,0xD3,0xD5,0xE0}=>1`,`{0xD4,0xE7}=>2`; west
`{0xD2,0xD3,0xD4,0xE2}=>4`,`{0xD5,0xE7}=>8`. IDENTICAL — including the non-obvious detail that
the binary evaluates EAST first then WEST (Rust reads east then west, same bit assignment).
Verify-call: `decompile_function 0x0057cab0`.

### P3: LOW NS classifier — `check_bridge_neighbors_ns_low` (walker.rs:1054)
Binary `0x0057B990`: NORTH `{0x57,0x59,0x5B,0x61}→1`, `{0x5A,0x65}→2`; SOUTH
`{0x58,0x59,0x5A,0x63(=99)}→4`, `{0x5B,0x65}→8`. Rust matches all eight bytes and both bits.
IDENTICAL. Verify-call: `decompile_function 0x0057b990`.

### P4: LOW EW classifier — `check_bridge_neighbors_ew_low` (walker.rs:1023)
Binary `0x0057B870`: EAST (puVar3) `{0x4E,0x50,0x52,0x5D}→1`, `{0x51,0x64(=100)}→2`;
WEST (puVar2) `{0x4F,0x50,0x51,0x5F}→4`, `{0x52,0x64}→8`. Rust matches all eight bytes and
both bits. IDENTICAL. Verify-call: `decompile_function 0x0057b870`.

### P5: HIGH NS destruction table — `DESTRUCTION_OVERLAY_HIGH_NS` (bridge_specs.rs:419)
Binary `0x0057E7A0` `local_70[0..15]`:
`[-1, 0xD2, 0xD5, FF, 0xD1, 0xD3, 0xD5, FF, 0xD4, 0xD4, 0xE7, FF, FF, FF, FF, FF]`.
Rust: `[FF, D2, D5, FF, D1, D3, D5, FF, D4, D4, E7, FF, FF, FF, FF, FF]` (binary `-1` ⇒ Rust
`0xFF` sentinel ⇒ `None`). All 16 entries IDENTICAL. Verify-call: `decompile_function 0x0057e7a0`.

### P6: HIGH EW destruction table — `DESTRUCTION_OVERLAY_HIGH_EW` (bridge_specs.rs:425)
Binary `0x0057ED00`: `[-1, 0xDB, 0xDE, FF, 0xDA, 0xDC, 0xDE, FF, 0xDD, 0xDD, 0xE8, FF×5]`.
Rust: `[FF, DB, DE, FF, DA, DC, DE, FF, DD, DD, E8, FF...]`. IDENTICAL.
Verify-call: `decompile_function 0x0057ed00`.

### P7: LOW NS destruction table — `DESTRUCTION_OVERLAY_LOW_NS` (bridge_specs.rs:435)
Binary `0x0057DD50`: `[-1, 0x4F, 0x52, FF, 0x4E, 0x50, 0x52, FF, 0x51, 0x51, 0x64(=100), FF×5]`.
Rust: `[FF, 4F, 52, FF, 4E, 50, 52, FF, 51, 51, 64, FF...]`. IDENTICAL.
Verify-call: `decompile_function 0x0057dd50`.

### P8: LOW EW destruction table — `DESTRUCTION_OVERLAY_LOW_EW` (bridge_specs.rs:443)
Binary `0x0057E2A0`: `[-1, 0x58, 0x5B, FF, 0x57, 0x59, 0x5B, FF, 0x5A, 0x5A, 0x65, FF×5]`.
Rust: `[FF, 58, 5B, FF, 57, 59, 5B, FF, 5A, 5A, 65, FF...]`. IDENTICAL.
Verify-call: `decompile_function 0x0057e2a0`.

### P9: `pick_destruction_overlay` dispatch + out-of-range guard (bridge_specs.rs:397)
Binary indexes `local_70[neighbor_check]` only when `0 < iVar` (i.e. idx ≥ 1) and reads up to
idx 15. Rust guards `neighbor_check >= 16 → None`; idx 0 yields table[0]=0xFF→None, matching
the binary's `if (0 < iVar2)` early skip on idx 0 (idx 0 means no neighbor pattern → no-op).
The `0xFF → None` mapping reproduces the binary's `-1`-sentinel "leave overlay alone". PARITY.
Verify-call: `decompile_function 0x0057e7a0` (the `if (0 < iVar2)` / `local_70[iVar2]` block).

### P10: no-op-when-equal semantics
Binary: after `iVar2 = local_70[iVar]`, `if (iVar8 == iVar2) return;` — table hit equal to
current overlay is a no-op. Rust caller (`apply_bridge_destruction_ns_high` walker.rs:737):
`Some(n) if n != cur => n, _ => return final_cells`. IDENTICAL no-op-on-equal. PARITY.
Verify-call: `decompile_function 0x0057e7a0`.

### P11: second-switch early-return (`return uVar4 | 4`) vs setting `|8` — proven equivalent
Concern: in every classifier the second switch returns immediately on its bit-4 case, so the
bit-8 case in the same switch is skipped. This is only a divergence if a single overlay byte
could match BOTH the bit-4 set AND the bit-8 set of the same probe. Checked all four:
- NS_High SOUTH: bit4 `{0xDB,0xDC,0xDD,0xE6}` ∩ bit8 `{0xDE,0xE8}` = ∅.
- EW_High WEST: bit4 `{0xD2,0xD3,0xD4,0xE2}` ∩ bit8 `{0xD5,0xE7}` = ∅.
- NS_Low SOUTH: bit4 `{0x58,0x59,0x5A,0x63}` ∩ bit8 `{0x5B,0x65}` = ∅.
- EW_Low WEST: bit4 `{0x4F,0x50,0x51,0x5F}` ∩ bit8 `{0x52,0x64}` = ∅.
Sets are disjoint, so a single C `switch` byte can only ever take one case; the early-return
cannot suppress a bit-8 that would otherwise be set. Rust's independent `match` arms produce
the identical result. PROVEN-PARITY (algebraic, full input space = 256 possible bytes).

### P12: off-map / null-neighbor sentinel substitution
Binary: when a probe cell is off-map (`iVar1 < 0 || 0x3FFFF < iVar1`) or null, it substitutes
`&DAT_00abdc50` and reads its overlay at +0x44. Read live: `DAT_00abdc50`+0x44 = `0`. Byte `0`
is not a member of any bit-set in any of the four classifiers, so the sentinel contributes no
bit — identical to the Rust treating off-map/missing neighbors as `0` (walker.rs:641-645,
668-676, etc.). PROVEN-PARITY for all in-bounds-row off-map cases.
Verify-call: `read_memory 0x00abdc50 len=64` (all zero).

### P13: triple write target geometry matches axis
Binary NS writes chosen overlay to `local_c4`(center=`(X,Y)`), `local_b8`(`(X,Y-1)`=north),
`local_cc`(`(X,Y+1)`=south). Binary EW writes to `this`(center), `local_bc`(`(X-1,Y)`=west),
`local_c8`(`(X+1,Y)`=east). Rust `ns_triple` = (this, north=Y-1, south=Y+1); `ew_triple` =
(this, west=X-1, east=X+1) (walker.rs:693-703). IDENTICAL axis geometry for both the walker
bodies and the cascade leaves. PARITY.
Verify-call: `decompile_function 0x0057e7a0` / `0x0057ed00`.

### P14: progressive-intermediate gates (table path vs fixed 0xDF/0xE1 etc.)
Binary NS_High: `if (iVar8 < 0xdf) iVar2 = local_70[iVar]` else `0xDF→0xE0`, `0xE1→0xE2`,
else return. Rust mirror (walker.rs:736-748) `cur < 0xDF → table`, `0xDF→0xE0`, `0xE1→0xE2`,
else return. EW_High: `< 0xE3 → table`, `0xE3→0xE4`, `0xE5→0xE6` (Rust 790-801). NS_Low:
`< 0x5C → table`, `0x5C→0x5D`, `0x5E→0x5F` (Rust 1102-1113). EW_Low: `< 0x60 → table`,
`0x60→0x61`, `0x62→0x63(=99)` (Rust 1154-1165). All four IDENTICAL. PARITY.
Verify-call: `decompile_function 0x0057e7a0 / 0x0057ed00 / 0x0057dd50 / 0x0057e2a0`.

---

## UNCHECKED

### U1: column-0 / row-wrap neighbor read at the absolute map edge (not reachable by placed bridges)
At X=0, the binary's EW probe computes `iVar1 = Y*0x200 + (short)(0-1) = Y*0x200 - 1`, which
for Y≥1 is a valid linear index pointing to cell `(0x1FF, Y-1)` (last column of the previous
row) — a real cell whose overlay byte is read. The Rust treats `rx==0` west as `0`
(walker.rs:641-645, 1028-1032). So for a bridge body cell sitting at X=0 the binary could read
a genuine wrap-around neighbor while Rust reads 0.
- Why UNCHECKED rather than DRIFT: every stock/placed bridge fixture observed sits at interior
  coords (e.g. `(57,49)`, `(60,52)`, `(49,87)`, `(79,50)` per the stock low-bridge collapse
  trace doc); RA2 maps inset the playable area away from column 0 / row 0, so no bridge body or
  perpendicular sibling cell reaches X=0 or Y=0 in normal play. The NS-axis equivalent at Y=0
  does NOT wrap — `(short)(Y-1)*0x200 = -0x200`, giving negative `iVar1` → sentinel(0) → matches
  Rust — so even the wrap asymmetry only theoretically affects EW at X=0.
- To resolve: confirm via map format that no DestroyableBridge can be authored with a body or
  ±1 perpendicular cell at X=0. If one can, this becomes PROVEN-DRIFT (binary reads prev-row
  last column; Rust reads 0).
- Verify-call basis: `decompile_function 0x0057cab0` (the `iVar1 = param_1[1]*0x200 +
  (int)(short)(*param_1 - 1)` west-probe index with no separate `X >= 0` guard).
