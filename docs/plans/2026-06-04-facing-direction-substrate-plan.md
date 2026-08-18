# Facing / Direction Lookup-Table Substrate — Foundation Plan (S1–S4)

> **For Claude:** Execute task-by-task. Each task is self-contained, ends in a
> `cargo test` + commit. Pure-data slices — hash-neutral, additive, no consumer
> cutover.

**Goal:** Stand up the `sim/substrate/direction_tables/` read-only service with the
four gamemd-exact pure-data table groups — cell-delta, lepton-delta, facing/direction
quantization, and the DRAGON 32-way frame table — each proven by an exact-equality
test against the gamemd dump. Closes the two MISSING-table DRIFTs (lepton D1, dragon
D3) at the data layer; consumer re-pointing is a later slice.

**Architecture:** A new pure, stateless `sim/substrate/direction_tables/` service
(`const` tables + free functions). It depends only on `util/` (sim→util is allowed)
and re-exports the already-PARITY `util::direction` cell table + 8-bit quantization,
while ADDING the missing integer lepton table, the 16-bit quantization form, the
muzzle-anim rotation, and the DRAGON frame table. No `render/ui/audio/net` dep, no
new mutable state, no state-hash impact.

**Design Doc:** `docs/research/substrate/tables/FACING_DIRECTION_SUBSTRATE_STUDY.md`
(verified 2026-06-04) + `docs/research/substrate/LOOKUP_TABLE_SUBSTRATE_SERVICE_STUDY.md`
roadmap rows U1–U4.

---

## Grounding Summary

- **docs/research/** — the Facing/Direction study (HIGH; all S1–S4 table values
  VERIFIED live against gamemd 2026-06-04, Verification Log #1/#2/#4/#5) is the
  spec. Cell table = compass order N(0,-1)…NW(-1,-1) (init `0x0049F2F0`); lepton =
  cell ×256 (`0x0089F6D8`); DRAGON = `(28-i)&31` (`read_memory 0x007F4890`);
  quantization `((f>>4)+1)>>1&7` PROVEN bit-identical to `(f+16)/32&7`.
- **Repo reality (read this pass):** `util/direction.rs` holds `DIRECTION_DELTAS`
  (cell, **PARITY**), `direction_from_facing` (8-bit quantization, **PARITY**),
  `opposite_direction`. No integer lepton table exists (DRIFT D1). No DRAGON table
  exists (DRIFT D3). `sim/substrate/` does not exist yet.
- **Pattern mirrored:** the just-shipped cell-spread / house-color lookup-table
  slices — embed gamemd-exact tables as pure read-only data, prove by
  exact-dump-equality, no shadow→invert.
- **INI:** none — these are engine constants, not INI-driven.
- **Still unknown (deferred):** U3/drive-track full byte-equality +
  `transform_track_point` flag math (blocking gate — separate plan); the float-atan2
  facing question and all consumer cutovers (D1/D2/D3 *behavior* fixes) are later
  slices; U19 FacingClass turn parity (stateful) deferred.

## Key Technical Decisions

- **Service lives in `sim/substrate/direction_tables/`, additive only.** It
  re-exports util's PARITY cell table + 8-bit quantization (sim→util, legal) and
  ADDs the missing pieces. No move out of `util/` yet → no `util→sim` layering
  violation, no consumer churn, hash-neutral. **Confidence:** high — **Source:**
  study §6.1 + `util/direction.rs` (read this pass) + CLAUDE.md layering invariant.
- **Lepton table is `const`-derived as `CELL_DELTAS[i] * 256`.** The ×256 identity
  is proven (study §4.2/Verification Log #2), so deriving it cannot drift; a test
  still asserts it equals the gamemd dump verbatim. **Confidence:** high —
  **Source:** study §6.2.
- **DRAGON table is `const`-derived as `(28 - i) & 31`** with a test asserting it
  equals the dumped `[28,27,…,0,31,30,29]`. `&31` on `i32` gives the correct wrap
  for the negative tail (`-1&31==31`). **Confidence:** high — **Source:** study §5 /
  Verification Log #4.
- **Scope = S1–S4 only.** U3/drive-track (S5) is deferred: it is a ~3,393-line move
  with a BLOCKING `transform_track_point`-vs-binary gate and is already at spot-check
  parity (study §4.5/D5). **Confidence:** high — **Source:** study §8 (S5 blocked) /
  DRIFT ledger D5.

## Open Questions

### Resolved During Planning
- *Are cell/quantization already correct?* Yes — `util/direction.rs` is PARITY
  (study §4.1/§4.3). This plan re-exports, does not reimplement.
- *Where does the service live?* `sim/substrate/direction_tables/` (study §6.1).
- *Does adding these tables change the state hash?* No — `const` data, no consumer
  reads them yet; purely additive.

### Deferred to Implementation / Later Slices
- U3 drive-track move + full byte-equality + `transform_track_point` verification
  (separate plan; blocking Ghidra gate).
- Consumer cutovers that actually fix D1 (locomotor sin/cos → `lepton_delta`) and
  D3 (`app_fire_effects` → `dragon_frame_index`) — later re-point slice.
- U19 FacingClass / turret turn parity (stateful) — separate plan.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/substrate/mod.rs` | `sim/substrate` umbrella + `//!` header |
| Create | `src/sim/substrate/direction_tables/mod.rs` | submodule decls + facade re-exports |
| Create | `src/sim/substrate/direction_tables/cell.rs` | cell-delta re-export + checked/unchecked accessors + dump-equality test |
| Create | `src/sim/substrate/direction_tables/lepton.rs` | NEW `LEPTON_DELTAS` (cell×256) + `lepton_delta` + `lepton_to_cell` |
| Create | `src/sim/substrate/direction_tables/quantize.rs` | 8/16-bit quantization, `opposite_dir`, `facing8_to_16`, `muzzle_anim_index_8way` |
| Create | `src/sim/substrate/direction_tables/dragon.rs` | NEW `DRAGON_FRAME_TABLE` + `dragon_frame_index` |
| Modify | `src/sim/mod.rs` | add `pub mod substrate;` |

## Interface Changes

All new public API under `crate::sim::substrate::direction_tables` (additive; nothing
existing changes). `util::direction` is untouched (its consumers keep working).

## Sim Checklist

- [x] All math integer — `LEPTON_DELTAS` is the exact integer table (the point of
  D1); no `f32`/`f64` anywhere.
- [x] No new state in the deterministic hash — `const` tables, no consumer yet.
- [x] No dependency on render/ui/sidebar/audio/net — only `util/` + `std`.
- [x] Tick ordering unaffected — no tick code touched.
- [x] BTreeMap iteration order — N/A.

## Risk Areas

Low. Purely additive: re-exports existing PARITY tables, adds new `const` tables
guarded by exact-equality tests. No consumer reads them yet, so no behavior change
and no regression surface. The only "risk" is the foundation sitting unused until the
cutover slice — acceptable and intended (matches cell-spread/house-color sequencing).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | Cell-delta table values + compass order | Every adjacent-cell step (A*, bridges, walls, anim, locomotors) | `cell_delta_table_equals_gamemd_dump` vs study §5 (init `0x0049F2F0`) |
| 2 | Lepton-delta = cell×256 exact integers (±256, not ±181) | Per-tick locomotor step; √2 diagonal speed/path bug if wrong (D1) | `lepton_delta_table_equals_gamemd_dump` vs `0x0089F6D8` |
| 3 | Quantization `((f>>4)+1)>>1&7` / 16-bit form / muzzle `+1` | Facing→direction every move/turret tick; muzzle-flash anim | full-256-input proof + `0x004B4B00` form |
| 4 | DRAGON `(28-i)&31` table + `(((bam)>>10)+1)>>1&0x1F` index | `Rotates=yes` projectile sprite frame (D3) | `dragon_frame_table_equals_gamemd_dump` vs `read_memory 0x007F4890` |

---

## Tasks

### Task 1: Create `sim/substrate` tree + cell-delta module (S1)

**Why:** Establish the service module and its first (canonical, already-PARITY) table
as the sim-facing entry point. Foundation everything else hangs off.

**Files:**
- Create: `src/sim/substrate/mod.rs`
- Create: `src/sim/substrate/direction_tables/mod.rs`
- Create: `src/sim/substrate/direction_tables/cell.rs`
- Modify: `src/sim/mod.rs` (add `pub mod substrate;` near the other `pub mod` lines)

**Pattern:** new pure-data substrate service (mirrors the lookup-table slice style).

**Step 1: `src/sim/substrate/mod.rs`**
```rust
//! `sim/substrate` — pure, read-only, deterministic engine-data services
//! (gamemd-exact lookup tables) consumed by the sim. "Rust-native structure,
//! gamemd-native semantics." No render/ui/audio/net dependency.

pub mod direction_tables;
```

**Step 2: `src/sim/substrate/direction_tables/mod.rs`** (cell only for now)
```rust
//! Facing / direction lookup-table substrate — pure, read-only, deterministic
//! services for the gamemd "which-way / where-next" table family (cell-delta,
//! lepton-delta, facing↔direction quantization, DRAGON 32-way frame). Tables are
//! gamemd-exact, proven by exact-equality tests; no shadow→invert.
//!
//! Foundation slice (S1–S4): canonical sim-facing tables. Drive-track tables (S5)
//! and consumer cutovers (S6+) are later slices.
//!
//! ## Dependency rules
//! - Part of sim/substrate — depends only on util/. No render/ui/audio/net.

pub mod cell;

pub use cell::{CELL_DELTAS, cell_delta, cell_delta_unchecked};
```

**Step 3: `src/sim/substrate/direction_tables/cell.rs`**
```rust
//! gamemd 8-direction cell-delta table + stepping accessors.
//!
//! Single sim-facing entry point for the "which adjacent cell" primitive. The
//! values are the PARITY-verified `util::direction::DIRECTION_DELTAS`
//! (re-exported; sim may depend on util). Compass order 0=N..7=NW, +X=east,
//! +Y=south; gamemd runtime-init at the foundation direction-table initializer.

use crate::util::direction::DIRECTION_DELTAS;

/// gamemd 8-direction cell-delta table, compass order. Canonical reference
/// (identical to `util::direction::DIRECTION_DELTAS`).
pub const CELL_DELTAS: [(i32, i32); 8] = DIRECTION_DELTAS;

/// Checked cell-delta. `None` for `dir > 7` (incl. the tube sentinel 8) — the
/// safe sim accessor.
pub fn cell_delta(dir: u8) -> Option<(i32, i32)> {
    CELL_DELTAS.get(dir as usize).copied()
}

/// Faithful mirror of gamemd's unchecked `MapCoord_Step_By_Direction` indexing
/// (no mask/bounds; callers sanitize upstream). Debug-asserts `dir <= 7` and
/// masks `&7` to stay memory-safe; use only when mirroring that contract.
pub fn cell_delta_unchecked(dir: u8) -> (i32, i32) {
    debug_assert!(dir <= 7, "cell_delta_unchecked: dir {dir} > 7 (gamemd OOB read)");
    CELL_DELTAS[(dir & 7) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_delta_table_equals_gamemd_dump() {
        // gamemd 0x0089F688, decoded from init 0x0049F2F0 (study Verification Log #1).
        let expected = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        assert_eq!(CELL_DELTAS, expected);
        for (i, &e) in expected.iter().enumerate() {
            assert_eq!(cell_delta(i as u8), Some(e));
        }
        assert_eq!(cell_delta(8), None); // tube sentinel, not a 9th compass dir
        assert_eq!(cell_delta(255), None);
    }
}
```

**Step 4: Modify `src/sim/mod.rs`** — add `pub mod substrate;` alongside the other
`pub mod` declarations (keep alphabetical if the file is ordered).

**Step 5: Verify** — `cargo test -p vera20k --lib -- substrate::direction_tables::cell`
→ PASS.

**Step 6: Commit** (`sim/substrate: direction_tables cell-delta service (S1, re-export + accessors)`).

---

### Task 2: Lepton-delta table — the MISSING integer step table (S2)

**Why:** Add the integer per-tick locomotor step vector gamemd uses (= cell ×256).
This is DRIFT D1's data half — the table that replaces the sin/cos diagonal.

**Files:**
- Create: `src/sim/substrate/direction_tables/lepton.rs`
- Modify: `src/sim/substrate/direction_tables/mod.rs` (add `pub mod lepton;` + re-export)

**Step 1: `src/sim/substrate/direction_tables/lepton.rs`**
```rust
//! gamemd lepton-delta (8-direction, sub-cell) table — the integer per-tick
//! locomotor step vector = cell-delta ×256. gamemd source
//! `g_DirectionDeltaX/Y_Table @ 0x0089F6D8` (runtime-init; study Verification
//! Log #2). 256 leptons = 1 cell. This is the exact integer table gamemd uses
//! for the 8-direction body translation — NOT sin/cos (closes DRIFT D1 at the
//! data layer; locomotor cutover is a later slice).

use super::cell::CELL_DELTAS;

const LEPTONS_PER_CELL: i32 = 256;

/// 8-direction lepton-delta table = `CELL_DELTAS[i] * 256`, compass order.
/// Const-derived from the (proven-identical) cell table so it cannot drift.
pub const LEPTON_DELTAS: [(i32, i32); 8] = {
    let mut out = [(0i32, 0i32); 8];
    let mut i = 0;
    while i < 8 {
        out[i] = (
            CELL_DELTAS[i].0 * LEPTONS_PER_CELL,
            CELL_DELTAS[i].1 * LEPTONS_PER_CELL,
        );
        i += 1;
    }
    out
};

/// Checked lepton-delta for a direction. `None` for `dir > 7`.
pub fn lepton_delta(dir: u8) -> Option<(i32, i32)> {
    LEPTON_DELTAS.get(dir as usize).copied()
}

/// Signed lepton→cell toward zero, matching gamemd `(v + (v>>31 & 0xFF)) >> 8`.
pub fn lepton_to_cell(v: i32) -> i32 {
    (v + ((v >> 31) & 0xFF)) >> 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lepton_delta_table_equals_gamemd_dump() {
        // gamemd 0x0089F6D8 (study Verification Log #2): cell ×256.
        let expected = [
            (0, -256),
            (256, -256),
            (256, 0),
            (256, 256),
            (0, 256),
            (-256, 256),
            (-256, 0),
            (-256, -256),
        ];
        assert_eq!(LEPTON_DELTAS, expected);
        // Diagonal step is exactly ±256 per axis, NOT the ±181 sin/cos diagonal.
        assert_eq!(lepton_delta(1), Some((256, -256)));
        assert_eq!(lepton_delta(8), None);
    }

    #[test]
    fn lepton_is_cell_times_256() {
        for i in 0..8 {
            assert_eq!(
                LEPTON_DELTAS[i],
                (CELL_DELTAS[i].0 * 256, CELL_DELTAS[i].1 * 256)
            );
        }
    }

    #[test]
    fn lepton_to_cell_rounds_toward_zero() {
        assert_eq!(lepton_to_cell(256), 1);
        assert_eq!(lepton_to_cell(-256), -1);
        assert_eq!(lepton_to_cell(255), 0);
        assert_eq!(lepton_to_cell(-1), 0);
        assert_eq!(lepton_to_cell(-255), 0);
        assert_eq!(lepton_to_cell(384), 1); // 1.5 cells → 1 toward zero
        assert_eq!(lepton_to_cell(-384), -1);
    }
}
```

**Step 2: Modify `mod.rs`** — add under the cell decl:
```rust
pub mod lepton;
```
and extend the re-export block:
```rust
pub use lepton::{LEPTON_DELTAS, lepton_delta, lepton_to_cell};
```

**Step 3: Verify** — `cargo test -p vera20k --lib -- substrate::direction_tables::lepton`
→ PASS.

**Step 4: Commit** (`sim/substrate: add integer LEPTON_DELTAS table (S2, closes D1 data layer)`).

---

### Task 3: Quantization + 8↔16-bit + muzzle (S3)

**Why:** Expose the sim-facing facing↔direction quantization (re-export the PARITY
8-bit form), and ADD the 16-bit form, `opposite_dir`, `facing8_to_16`, and the
8-way muzzle-anim rotation gamemd uses in Fire_At.

**Files:**
- Create: `src/sim/substrate/direction_tables/quantize.rs`
- Modify: `src/sim/substrate/direction_tables/mod.rs`

**Step 1: `src/sim/substrate/direction_tables/quantize.rs`**
```rust
//! Facing↔direction quantization + 8/16-bit facing helpers (pure functions).
//!
//! gamemd `((f>>4)+1)>>1 & 7` (8-bit) / `((f>>12)+1)>>1 & 7` (16-bit) — PROVEN
//! bit-identical to `(f+16)/32 & 7` (study §4.3 / Verification Log #5). Opposite
//! `(dir-4)&7 == (dir+4)&7` (study §4.4). The 8-bit form already lives in util;
//! this is the sim-facing entry point that ADDS the 16-bit form + muzzle rotation.

/// gamemd 8-bit facing → 8-direction: `((f>>4)+1)>>1 & 7`.
pub fn dir_from_facing8(f: u8) -> u8 {
    crate::util::direction::direction_from_facing(f)
}

/// gamemd 16-bit facing → 8-direction: `((f>>12)+1)>>1 & 7`. The low byte is
/// irrelevant (only bits ≥12 feed the quantization).
pub fn dir_from_facing16(f: u16) -> u8 {
    ((((f >> 12) + 1) >> 1) & 7) as u8
}

/// 8-bit facing widened to gamemd's 16-bit facing (high byte authoritative).
pub fn facing8_to_16(f: u8) -> u16 {
    (f as u16) << 8
}

/// Opposite direction: `(dir-4)&7` (== `(dir+4)&7`).
pub fn opposite_dir(dir: u8) -> u8 {
    dir.wrapping_sub(4) & 7
}

/// gamemd 8-way muzzle-flash anim index (used when the weapon's anim count == 8):
/// `(dir_from_facing16(f)+1) & 7`. The `+1` rotation is real (study §5).
pub fn muzzle_anim_index_8way(f16: u16) -> u8 {
    (dir_from_facing16(f16) + 1) & 7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_from_facing8_full_input_space() {
        for f in 0u8..=255 {
            let gamemd = ((((f as u16) >> 4) + 1) >> 1) as u8 & 7;
            assert_eq!(dir_from_facing8(f), gamemd, "f={f}");
            assert_eq!(dir_from_facing8(f), (f.wrapping_add(16) / 32) & 7, "f={f}");
        }
        // Boundaries (study S3): rounds UP at 16+32n; 240..255 wrap to N.
        assert_eq!(dir_from_facing8(15), 0);
        assert_eq!(dir_from_facing8(16), 1);
        assert_eq!(dir_from_facing8(240), 0);
        assert_eq!(dir_from_facing8(255), 0);
    }

    #[test]
    fn dir_from_facing16_ignores_low_byte() {
        for hi in 0u8..=255 {
            for lo in [0u8, 1, 127, 255] {
                let f16 = ((hi as u16) << 8) | lo as u16;
                assert_eq!(dir_from_facing16(f16), dir_from_facing8(hi), "hi={hi} lo={lo}");
            }
        }
    }

    #[test]
    fn opposite_dir_is_plus_or_minus_4() {
        for d in 0u8..8 {
            assert_eq!(opposite_dir(d), (d + 4) & 7);
            assert_eq!(opposite_dir(d), d.wrapping_sub(4) & 7);
        }
    }

    #[test]
    fn facing8_to_16_high_byte() {
        assert_eq!(facing8_to_16(0), 0);
        assert_eq!(facing8_to_16(0x20), 0x2000);
        assert_eq!(facing8_to_16(0xFF), 0xFF00);
    }

    #[test]
    fn muzzle_anim_8way_plus1_rotation() {
        // f=0 → bucket 0 → anim 1 (the +1 rotation).
        assert_eq!(muzzle_anim_index_8way(0x0000), 1);
        for f16 in [0u16, 0x2000, 0x4000, 0x8000, 0xE000] {
            assert_eq!(muzzle_anim_index_8way(f16), (dir_from_facing16(f16) + 1) & 7);
        }
    }
}
```

**Step 2: Modify `mod.rs`** — add `pub mod quantize;` + re-export:
```rust
pub use quantize::{
    dir_from_facing8, dir_from_facing16, facing8_to_16, muzzle_anim_index_8way, opposite_dir,
};
```

**Step 3: Verify** — `cargo test -p vera20k --lib -- substrate::direction_tables::quantize`
→ PASS.

**Step 4: Commit** (`sim/substrate: facing/dir quantization + 16-bit + muzzle (S3)`).

---

### Task 4: DRAGON 32-way frame table (S4)

**Why:** Add the missing `Rotates=yes` projectile frame table + index formula
(DRIFT D3's data half).

**Files:**
- Create: `src/sim/substrate/direction_tables/dragon.rs`
- Modify: `src/sim/substrate/direction_tables/mod.rs`

**Step 1: `src/sim/substrate/direction_tables/dragon.rs`**
```rust
//! DRAGON 32-way rotating-SHP frame table + index formula. gamemd source
//! `0x007F4890` = `i32[32]` where `table[i] = (28 - i) & 31` (study Verification
//! Log #4, read_memory 0x007F4890 len128). Used for `Rotates=yes` projectiles
//! (DRAGON / AAHeatSeeker2). Rust previously lacked it (closes DRIFT D3 at the
//! data layer; the app_fire_effects cutover is a later slice).

/// gamemd DRAGON 32-way frame map `0x007F4890`: `table[i] = (28 - i) & 31`,
/// i.e. `[28,27,…,1,0,31,30,29]`. `&31` on i32 wraps the negative tail correctly
/// (`-1 & 31 == 31`).
pub const DRAGON_FRAME_TABLE: [i32; 32] = {
    let mut out = [0i32; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (28 - i as i32) & 31;
        i += 1;
    }
    out
};

/// DRAGON 32-way frame index from a BAM (binary-angle) value:
/// `index = (((bam) >> 10) + 1) >> 1 & 0x1F` (study §5).
pub fn dragon_frame_index(bam: u16) -> usize {
    ((((bam >> 10) + 1) >> 1) & 0x1F) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragon_frame_table_equals_gamemd_dump() {
        // read_memory 0x007F4890 len128 (study Verification Log #4).
        let expected: [i32; 32] = [
            28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7,
            6, 5, 4, 3, 2, 1, 0, 31, 30, 29,
        ];
        assert_eq!(DRAGON_FRAME_TABLE, expected);
        for i in 0..32 {
            assert_eq!(DRAGON_FRAME_TABLE[i], (28 - i as i32) & 31);
        }
    }

    #[test]
    fn dragon_frame_index_formula() {
        for bam in [0u16, 0x0400, 0x0800, 0x8000, 0xFC00, 0xFFFF] {
            assert_eq!(
                dragon_frame_index(bam),
                ((((bam >> 10) + 1) >> 1) & 0x1F) as usize
            );
        }
        assert_eq!(dragon_frame_index(0), 0);
    }
}
```

**Step 2: Modify `mod.rs`** — add `pub mod dragon;` + re-export:
```rust
pub use dragon::{DRAGON_FRAME_TABLE, dragon_frame_index};
```

**Step 3: Verify** — `cargo test -p vera20k --lib -- substrate::direction_tables::dragon`
→ PASS.

**Step 4: Commit** (`sim/substrate: add DRAGON 32-way frame table (S4, closes D3 data layer)`).

---

### Task 5: Full regression + scope check

**Why:** Confirm the additive service compiles clean, all tables pass equality, and
nothing else moved.

**Step 1:** `cargo test -p vera20k --lib -- substrate::direction_tables` — read the
literal `test result:` line; all module tests PASS.
**Step 2:** `cargo test -p vera20k` (full) — confirm zero regressions (the change is
additive; count rises by the new tests only).
**Step 3:** `cargo clippy -p vera20k` — no new warnings in the new files (pre-existing
warnings elsewhere are not in scope).
**Step 4:** `git diff --name-only` — confirm only the 5 new files + `src/sim/mod.rs`.
**Step 5:** Commit any cleanups (`sim/substrate: direction_tables foundation regression pass`).

---

### Task 6: gamemd fidelity note (data-layer; in-game deferred)

**Verify:** These four table groups are proven gamemd-exact by the per-module
exact-equality tests (vs the study's binary-verified dumps). No consumer reads them
yet, so there is no in-game behavior to compare this slice — in-game fidelity is
verified when the cutover slice re-points locomotors (D1) and `app_fire_effects` (D3)
onto `lepton_delta` / `dragon_frame_index`. Record here that the foundation is
exact-equality-clean and ready for that cutover.

---

## Sources & References

- **Design/study:** `docs/research/substrate/tables/FACING_DIRECTION_SUBSTRATE_STUDY.md`
  (verified 2026-06-04; Verification Log #1/#2/#4/#5 confirm cell/lepton/DRAGON/
  quantization live).
- **Roadmap:** `docs/research/substrate/LOOKUP_TABLE_SUBSTRATE_SERVICE_STUDY.md` rows
  U1–U4.
- **gamemd.exe (kept here, not in code comments):** cell table `0x0089F688` (init
  `0x0049F2F0`); lepton table `0x0089F6D8`; DRAGON table `0x007F4890`; quantization
  in `Can_Use_Track 0x004B4B00`; opposite in `CheckBridgeTraversal 0x004D9C60`.
- **Repo:** `src/util/direction.rs` (re-exported cell + 8-bit quantization, PARITY).
- **Prior lookup-table slices:** cell-spread (`1266b61e`), house-color (D9).
- **Deferred:** U3/drive-track (S5, blocking gate), consumer cutovers (S6/S7), U19
  FacingClass turn parity (S8/S9).
