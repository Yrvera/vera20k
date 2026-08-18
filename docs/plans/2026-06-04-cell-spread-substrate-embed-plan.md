# Cell-Spread Exact-Table Embed — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace the Rust-*generated* cell-spread offset table with gamemd's exact 369-entry
embedded sweep, and replace the wrong `floor(CellSpread)` count/threshold rules with gamemd's
`ftol(CS+0.99)` count index and `ftol(CS×256)` lepton threshold — so splash/ore/wall AoE hits the
same cells, in the same order, with the same radius as gamemd.exe.

**Architecture:** This is the first slice of the lookup-table substrate program — a **data-parity +
API-consolidation** change, not a stateful shadow→invert migration. The cell-spread family is pure,
read-only, deterministic const data; we embed gamemd's verified table and give it one owner with the
gamemd index/threshold semantics, then re-point the two consumers (`combat_aoe.rs`, `mod.rs`).

**Design Doc:** `docs/research/substrate/tables/CELL_SPREAD_SUBSTRATE_STUDY.md`
(master: `docs/research/substrate/LOOKUP_TABLE_SUBSTRATE_SERVICE_STUDY.md`)

---

## Grounding Summary

- **docs/research/** — `CELL_SPREAD_SUBSTRATE_STUDY.md` (2026-06-04, adversarially re-verified:
  21 VERIFIED / 1 corrected / 0 unverifiable) is the authoritative contract. Supporting:
  `CELLSPREAD_OFFSET_TABLE_DUMP_GHIDRA_REPORT.md` (full 369-entry dump — **known stale on idx96 and
  the `ftol(CS)` count rule; do not trust those two points**), `WARHEAD_DETONATE_GHIDRA_REPORT.md` §4,
  `PSYCHIC_REVEAL_SUPERWEAPON_GHIDRA_REPORT.md` §3.
- **Ghidra confirmed (this session, in the study)** — count table `0x007ED3D0 =
  [1,9,21,37,61,89,121,161,205,253,309,369]`; count index `ftol(CS+0.99)` (`FADD double 0x007E5160`,
  `CALL ftol 0x007c5f00`); lepton threshold `ftol(CS×256)` (`FMUL 0x007E2224`); air flag `CS>0.5`
  (`FCOMP 0x007E5168`); offset reader `[EAX*4+0xABD490]` low=dx / `[+0xABD492]` high=dy. The offset
  table at `0x00ABD490` is **BSS (all-zero in the static image)** — its 369 entries exist only at
  runtime; ground truth is the initializer `MapClass__InitRevealSpiralTable @ 0x00561910` (idx0=(0,0),
  R1 sweep, idx96=(-4,-4), R11 duplicate all read from its body).
- **Repo pattern** — mirrors the const-table + pure-accessor style this branch added
  (`src/sim/map/bridge_topology.rs` read-only service; `src/sim/world/substrate.rs`). Fixed-point only,
  `'static` const data, no allocation.
- **INI** — `CellSpread` is `warhead+0x124`, parsed to `SimFixed` at `src/rules/warhead_type.rs:117`.
  No new INI keys. Stock `CellSpread` values must be enumerated from `ini/rulesmd.ini` (Task 7) to
  prove the fixed-point index rule agrees with gamemd's float rule over the actual input set.
- **Still unknown after grounding** → the literal 360 of 369 offset entries beyond the verified
  spot-checks (idx0, R1, band-starts, R11) must be transcribed from `0x00561910` in Task 1; and the
  CS=0 force-fire-on-cell equivalence (§4e) is UNCHECKED, handled as a decision in Task 9.

## Key Technical Decisions

- **Embed the const offset table from the initializer dump, not generate it.** — The generated sort
  (`compute_spread_offsets`) provably cannot match gamemd order (363/369 positions differ) or element
  set (R6,8–11). Only a verbatim embed matches. **Confidence:** high — **Source:**
  `CELL_SPREAD_SUBSTRATE_STUDY.md` §4b/§6b + `decompile_function 0x00561910`.
- **Implement `ftol(CS+0.99)` / `ftol(CS×256)` in fixed-point, not float.** — `(cs + 0.99).to_num::<i64>()`
  and `(cs × 256).to_num::<i64>()` (truncate-toward-zero; `cs ≥ 0` guarded). **Confidence:** high for
  the rule; medium for boundary-exactness vs gamemd's *float* CS at non-half decimals — pinned by the
  stock-INI enumeration test (Task 7). **Source:** study §5a/§5b Verification Log #5/#6.
- **Fix in place at `src/sim/combat/cell_spread.rs`; defer the relocation to `sim/world/substrate/`.** —
  Keeps this slice's blast radius to the parity fix (the player-visible win). The module move is a
  zero-behavior follow-up consolidation slice. **Confidence:** high — **Source:** YAGNI / study §7
  (relocation listed but separable).
- **Preserve the R11 duplicate-entry defect verbatim.** — It is the real gamemd table; it is a
  regression guard, not a bug to fix. Stock-unreachable (max stock CS=10 → index ≤10). **Confidence:**
  high — **Source:** study §3 / Verification Log #13.

## Open Questions

### Resolved During Planning
- *Where does the literal table come from?* — `decompile_function 0x00561910`; the static BSS read is
  all-zeros (study Verification Log #14). Task 1 transcribes from the initializer body.
- *Does CS=0 reach the table?* — Splash: no (gated by `cell_spread > SIM_ZERO` at `mod.rs:2172`, direct-hit
  else-branch). Ore: yes, `destroy_ore_at_impact` runs unconditionally (`mod.rs:2268`), CS=0 → 1 cell.

### Deferred to Implementation
- **CS=0 force-fire-on-a-cell** (`TargetKind::Cell`, no entity target): gamemd scans the impact cell and
  damages its occupants; Rust's direct-hit else-branch damages only an explicit `Entity` target, so a
  cell-target CS=0 shot damages nobody (study §4e). Whether this is observable depends on which weapons
  force-fire with CS=0 — decided in Task 9 (verify-then-route-or-leave), not blindly rewritten.
- Exact boundary agreement at any non-half stock `CellSpread` — resolved empirically by Task 7's
  enumeration; if a stock value disagrees, switch the addend to the bit-exact form there.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Rewrite | `src/sim/combat/cell_spread.rs` | Owns the embedded 369-entry offset table + 12-entry count table + gamemd splash index/threshold/air-flag helpers (the substrate service, in place) |
| Modify | `src/sim/combat/combat_aoe.rs:96,104` | Use `splash_threshold_leptons` + `splash_cells` instead of `to_num` rules (both occupancy and fallback branches) |
| Modify | `src/sim/combat/mod.rs:1162-1163` | `destroy_ore_at_impact` uses `splash_cells` instead of `to_num::<u32>()` radius |

## Interface Changes

`src/sim/combat/cell_spread.rs` public API (module is `pub(crate) mod cell_spread;` at `mod.rs:20`):
- **Removed:** `pub fn cells_in_spread(radius: u32) -> &'static [(i16,i16)]` (radius-as-u32 rule was wrong).
- **Added:** `splash_count_index(SimFixed) -> usize`, `splash_cells(SimFixed) -> &'static [(i16,i16)]`,
  `splash_threshold_leptons(SimFixed) -> i64`, `splash_air_flag(SimFixed) -> bool`,
  `count_table() -> &'static [u32;12]`, `offset_table() -> &'static [(i16,i16);369]`.
- **Consumers:** exactly two — `combat_aoe.rs` and `mod.rs::destroy_ore_at_impact`. Both updated in this
  plan. No other crate references these symbols (Grep-confirmed).

## Sim Checklist

- [x] All math fixed-point — `SimFixed`/`i64`/`i16`; **no f32/f64**. `ftol` modeled as `to_num::<iN>()`
      truncate-toward-zero on a fixed-point operand.
- [x] **No new serialized state** — tables are `'static const`; **no `SNAPSHOT_VERSION` bump**.
- [⚠] **State-hash VALUES change (intended).** Cell/target scan order, count, and lepton radius change →
      `ReceiveDamage` call order → RNG-consumption order → state hash differs from pre-fix for any match
      with AoE. This is the parity fix landing, not a regression. All lockstep peers run identical new
      logic → determinism + cross-client consistency preserved. **Existing saved replays will diverge.**
- [x] No dependency on render/ui/sidebar/audio/net — pure data + arithmetic.
- [x] Tick ordering unchanged — same call sites, same phase.
- [x] BTreeMap/occupancy iteration — AoE already dedups via `seen: BTreeSet`; new code preserves the
      table-order traversal before the entity-id dedup.

## Risk Areas

- **Combat golden tests will shift.** Any existing test asserting AoE damage counts/targets at fractional
  CellSpread, or relying on the old scan order, will change. Expect to update golden values in
  `src/sim/combat/*` tests and `tests/` — update them to the gamemd-correct values, do not revert the fix.
- **Task 1 (table transcription) is the single highest-risk step.** One wrong (dx,dy) is a silent 1-cell
  parity bug. Mitigated by the spot-check gate (idx0/R1/band-starts/R11), the count-alignment assert, and
  the symmetric-except-R11 assert.
- **Blast radius:** `apply_aoe_damage` feeds every splash detonation (19 gamemd call sites' Rust
  equivalents). Run the full combat test module + a smoke run after Task 8.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1, 2 | 369-entry offset table values **and order** | Target/cell scan order = ReceiveDamage order = RNG-consumption + chain order; fires every match any CS≥1 warhead hits ≥2 targets/ore/walls | `decompile_function 0x00561910`; spot tests idx0=(0,0), R1 sweep, band-starts, R11 dup; count alignment |
| 3 | `splash_count_index = ftol(CS+0.99)` | Fractional-CS warheads scan a 3×3+ block, not 1 cell; every `.5`-family detonation | Boundary test table vs `count[ftol(CS+0.99)]`; INI enumeration (Task 7) |
| 3 | `splash_threshold_leptons = ftol(CS×256)` | Fine radius gate; `floor(CS)×256` collapses to 0 leptons below CS=1.0 | Boundary test 0.5→128, 1.0→256, 0.1→25 |
| 3 | `splash_air_flag = CS>0.5` (strict) | Gates airborne-scan/capture (future); strict `>` boundary at exactly 0.5 | `0.5→false, 0.5+ε→true` |
| 5, 6 | Repointed consumers preserve table-order traversal | The order parity is lost if a consumer re-sorts | `test_aoe_target_order`, `test_ore_cs_2_cells` order asserts |

---

## Tasks

### Task 1: Extract the gamemd 369-entry offset table from the initializer

**Why:** The embedded const table is the heart of the slice and the only source of order/element-set
parity. The static memory image is BSS-zero; the initializer body is ground truth.

**Files:** none yet (produces a verified Rust literal used in Task 2).

**Step 1: Decompile the initializer.**
Load Ghidra tools via ToolSearch (`select:mcp__ghidra-mcp__decompile_function,mcp__ghidra-mcp__disassemble_function,mcp__ghidra-mcp__read_memory`).
Run `decompile_function 0x00561910` (`MapClass__InitRevealSpiralTable`). It writes the 369 entries to
`0x00ABD490` as packed `int32` (low 16 bits = `dx:i16`, high 16 bits = `dy:i16`), either as direct
stores or via `MapCoord_Set(dx, dy)` calls. If the decompile is truncated, fall back to
`disassemble_function 0x00561910` and read the immediates in address order.

**Step 2: Transcribe in write-address order.**
The array index for an entry written to address `A` is `(A - 0xABD490) / 4`. Decode each `int32 v` as
`dx = (v & 0xFFFF) as i16`, `dy = ((v >> 16) & 0xFFFF) as i16` (sign-extend both). Produce the literal
`[(i16,i16);369]` in index order 0..368.

**Step 3: Self-check against verified spot values (HARD GATE — do not proceed if any fails).**
- `[0] == (0,0)`
- `[1..9] == [(1,-1),(0,-1),(-1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)]` (the R1 sweep, NE,N,NW,W,E,SW,S,SE)
- `[96] == (-4,-4)` (NOT `(-5,-4)` — the dump doc is stale here)
- `[319] == (-3,11)` and `[322] == (-3,11)` (the R11 duplicate); `(3,-11)` appears nowhere
- band-start entries: `[121]==(-1,-7)`, `[161]==(-1,-8)`, `[205]==(-1,-9)`, `[253]==(-1,-10)`, `[309]==(0,11)`
- total length exactly 369

**Step 4: Verify (no code yet).**
Paste the spot-check results inline with the `decompile_function 0x00561910` citation. The transcribed
array is the input to Task 2.

**Step 5:** No commit (data artifact only).

---

### Task 2: Embed the const tables + pure accessors in `cell_spread.rs`

**Why:** Establish the single owner of the gamemd-exact data, replacing the generated `LazyLock` vec.

**Files:** Rewrite `src/sim/combat/cell_spread.rs`.

**Pattern:** const-table + pure-accessor (mirrors `src/sim/map/bridge_topology.rs`).

**Step 1: Replace the module head and tables.** Remove `use std::sync::LazyLock;`, `MAX_SPREAD_RADIUS`,
`SPREAD_OFFSETS`, and `compute_spread_offsets()`. Write:

```rust
//! Cell-spread tables — gamemd-exact filled-disk cell enumeration for area-of-effect.
//!
//! Embeds gamemd's two cooperating static tables verbatim:
//! - count table (cumulative cells per integer radius band 0..11),
//! - the 369-entry signed cell-offset sweep (exact order — scan order is player-observable
//!   because it determines damage/RNG/chain ordering).
//!
//! Pure, read-only, deterministic. No allocation, no float, no state.
//!
//! ## Dependency rules
//! - sim/combat — depends only on `crate::util::fixed_math`. Never on render/ui/audio/net.

use crate::util::fixed_math::{SimFixed, SIM_ZERO};

/// Cumulative filled-disk cell counts per integer radius band 0..=11.
/// gamemd `DAT_007ED3D0` (read_memory 0x007ED3D0, 48 bytes).
const COUNT_TABLE: [u32; 12] = [1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309, 369];

/// gamemd's hand-authored cell-offset sweep `DAT_00ABD490`, populated at startup by the
/// initializer (decompiled from MapClass__InitRevealSpiralTable). Each entry is a signed
/// (dx, dy) cell offset from the center; index 0 = (0,0). Order is verbatim and load-bearing.
/// The R=11 duplicate entry ([322]==[319]==(-3,11), mirror (3,-11) absent) is the real gamemd
/// table defect, preserved verbatim (stock-unreachable; regression guard, not a bug to fix).
const OFFSET_TABLE: [(i16, i16); 369] = [
    // <<< PASTE the verified literal from Task 1 here, all 369 entries in index order >>>
];
```

> The `OFFSET_TABLE` body is the Task 1 deliverable, inserted verbatim. Compilation fails fast if the
> length ≠ 369, which is an additional guard.

**Step 2: Add the pure accessors.**

```rust
/// Raw count table (gamemd `DAT_007ED3D0`).
pub fn count_table() -> &'static [u32; 12] {
    &COUNT_TABLE
}

/// Raw offset table (gamemd `DAT_00ABD490`), exact order.
pub fn offset_table() -> &'static [(i16, i16); 369] {
    &OFFSET_TABLE
}
```

**Step 3: Add tests (exact-table-equality).**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_table_exact_gamemd() {
        assert_eq!(*count_table(), [1, 9, 21, 37, 61, 89, 121, 161, 205, 253, 309, 369]);
    }

    #[test]
    fn offset_idx0_is_origin() {
        assert_eq!(offset_table()[0], (0, 0));
    }

    #[test]
    fn offset_r1_sweep_exact_order() {
        assert_eq!(
            offset_table()[1..9],
            [(1, -1), (0, -1), (-1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
        );
    }

    #[test]
    fn offset_band_starts_exact() {
        let t = offset_table();
        assert_eq!(t[96], (-4, -4)); // R6 interior — NOT (-5,-4) (dump doc stale)
        assert_eq!(t[121], (-1, -7));
        assert_eq!(t[161], (-1, -8));
        assert_eq!(t[205], (-1, -9));
        assert_eq!(t[253], (-1, -10));
        assert_eq!(t[309], (0, 11));
    }

    #[test]
    fn r11_duplicate_preserved_verbatim() {
        let t = offset_table();
        assert_eq!(t[319], (-3, 11));
        assert_eq!(t[322], (-3, 11));
        assert!(!t.contains(&(3, -11)), "gamemd never writes the (3,-11) mirror");
    }

    #[test]
    fn count_table_aligns_with_offset_len() {
        assert_eq!(COUNT_TABLE[11] as usize, OFFSET_TABLE.len());
    }
}
```

**Step 4: Verify.** `cargo test -p vera20k cell_spread -- --nocapture`. Expected: PASS (read the literal
`test result:` line). The file will not yet compile if Task 5/6 still call `cells_in_spread` — that is
fixed in Tasks 5/6; build the whole crate only after Task 6.

**Step 5: Commit** (`cell-spread: embed gamemd-exact offset+count tables (Slice 1)`).
Branch first if on a default branch; this branch (`factory-house-substrate-p1p2`) is fine.

---

### Task 3: Add the gamemd splash index/threshold/air-flag helpers

**Why:** Move the radius/threshold rule out of the consumers and make it gamemd-exact (the consumers
currently each get it wrong, two different ways).

**Files:** Modify `src/sim/combat/cell_spread.rs` (append to the impl/module).

**Step 1: Add the helpers.**

```rust
/// gamemd splash count-table index = `ftol(CellSpread + 0.99)` (Apply_area_damage 0x00489280:
/// FADD double[0x007E5160]=0.99, CALL ftol). Truncate-toward-zero on a non-negative operand.
/// Unclamped (the splash reader has no clamp; only the shroud reader clamps to 10). CS<=0 -> 0.
pub fn splash_count_index(cell_spread: SimFixed) -> usize {
    if cell_spread <= SIM_ZERO {
        return 0;
    }
    (cell_spread + SimFixed::from_num(0.99)).to_num::<i64>().max(0) as usize
}

/// gamemd splash cell sweep: `offset_table[..count_table[ftol(CS+0.99)]]`, exact order.
/// Index clamped to the 12-entry table bound (stock CS<=10 never reaches 11; modded OOB clamps).
pub fn splash_cells(cell_spread: SimFixed) -> &'static [(i16, i16)] {
    let idx = splash_count_index(cell_spread).min(COUNT_TABLE.len() - 1);
    &OFFSET_TABLE[..COUNT_TABLE[idx] as usize]
}

/// gamemd fine-filter radius in leptons = `ftol(CellSpread * 256)` (FMUL float[0x007E2224]=256, CALL
/// ftol). An object is damaged only if its 3D lepton distance <= this. CS<=0 -> 0.
pub fn splash_threshold_leptons(cell_spread: SimFixed) -> i64 {
    if cell_spread <= SIM_ZERO {
        return 0;
    }
    (cell_spread * SimFixed::from_num(256)).to_num::<i64>()
}

/// gamemd air/capture flag = `CellSpread > 0.5` (strict float compare, FCOMP float[0x007E5168]=0.5).
pub fn splash_air_flag(cell_spread: SimFixed) -> bool {
    cell_spread > SimFixed::from_num(0.5)
}
```

> If `SIM_ZERO` / `SimFixed` live at a different path than `crate::util::fixed_math`, match the import
> used in `src/sim/combat/combat_aoe.rs:18` (`use super::{...}`); adjust the `use` accordingly.

**Step 2: Add boundary tests (inside the existing `mod tests`).**

```rust
    #[test]
    fn splash_count_index_boundaries() {
        let f = SimFixed::from_num;
        // (CellSpread, expected count) = count_table[ftol(CS+0.99)]
        let cases = [
            (0.0, 1usize), (0.5, 9), (1.0, 9), (1.5, 21), (2.0, 21),
            (2.001, 37), (9.0, 253), (10.0, 309), (10.01, 369),
        ];
        for (cs, want) in cases {
            let idx = splash_count_index(f(cs)).min(11);
            assert_eq!(COUNT_TABLE[idx] as usize, want, "CS={cs}");
        }
    }

    #[test]
    fn splash_threshold_leptons_boundaries() {
        let f = SimFixed::from_num;
        for (cs, want) in [(0.0, 0i64), (0.5, 128), (1.0, 256), (2.0, 512), (2.5, 640), (10.0, 2560), (0.1, 25)] {
            assert_eq!(splash_threshold_leptons(f(cs)), want, "CS={cs}");
        }
    }

    #[test]
    fn splash_air_flag_strict_half() {
        let f = SimFixed::from_num;
        assert!(!splash_air_flag(f(0.5)));      // strict >
        assert!(splash_air_flag(f(0.5) + SimFixed::from_bits(1)));
        assert!(!splash_air_flag(f(0.0)));
        assert!(splash_air_flag(f(1.0)));
    }
```

**Step 3: Verify.** `cargo test -p vera20k cell_spread -- --nocapture`. Expected: PASS.

**Step 4: Commit** (`cell-spread: gamemd splash index/threshold/air-flag helpers (Slice 2)`).

---

### Task 4: Re-point the splash damage consumer (`combat_aoe.rs`) — occupancy branch

**Why:** Make `apply_aoe_damage` use the gamemd cell sweep + lepton threshold instead of the two wrong
`to_num` rules. Occupancy branch first (the normal path).

**Files:** Modify `src/sim/combat/combat_aoe.rs`.

**Step 1: Replace the threshold computation** at `combat_aoe.rs:96`:

```rust
    // gamemd lepton radius gate = ftol(CellSpread * 256).
    let spread_leptons: i64 = cell_spread::splash_threshold_leptons(cell_spread);
    let spread_sq: i64 = spread_leptons * spread_leptons;
```

**Step 2: Replace the cell sweep** at `combat_aoe.rs:104-106`. Delete the `spread_radius` local and
change the loop to drive off `splash_cells`:

```rust
        for &(dx, dy) in cell_spread::splash_cells(cell_spread) {
```

(Leave the body of the loop, the `offset_cell_coord` guards, `seen` dedup, and `push_entity_aoe_damage`
call unchanged — the table-order traversal is what carries the scan-order parity.)

**Step 3:** Confirm `cell_spread` (the `super::cell_spread` module) is in scope — it is imported at
`combat_aoe.rs:18` (`use super::{..., cell_spread, ...}`). No import change needed.

**Step 4: Verify.** `cargo build -p vera20k` will still fail until Task 6 (the fallback branch at
`:177` uses `spread_sq` — still valid — but `mod.rs` still calls `cells_in_spread`). Defer full build to
Task 6. Spot-check this file compiles in isolation by reading the diff.

**Step 5:** No commit yet (grouped with Tasks 5–6 into the Slice-3 commit).

---

### Task 5: Re-point the splash fallback branch (`combat_aoe.rs`) + add end-to-end tests

**Why:** The fallback entity-scan branch (`:159-212`, used when occupancy/terrain context is absent)
relies on `spread_sq`, already fixed by Task 4 Step 1. Confirm it, then pin the behavior with
exact-output tests.

**Files:** Modify `src/sim/combat/combat_aoe.rs` (tests only).

**Step 1:** Read `combat_aoe.rs:159-212`. Confirm it uses `spread_sq` (now `ftol(CS×256)²`) for rejection
at `:177` and does not re-derive a radius from `to_num`. No code change expected; if it independently
re-computes a radius, replace it with `splash_threshold_leptons` the same way. Note the `distance` cell
conversion at `:185` (`dist_leptons/256`) is the falloff input — **out of scope** (combat-falloff
family, study §4d LOW); leave it.

**Step 2: Add end-to-end AoE tests** (new `#[cfg(test)]` block or `tests/` integration, matching the
existing combat test style). Each builds a minimal `EntityStore` + `WarheadType` and asserts the damage
list:

```rust
// CS=0.5: count grows to 9 cells, but the lepton gate is 128 leptons. A unit at the impact cell
// center is damaged; units at orthogonal neighbor cell-centers (256 leptons) are rejected by the
// threshold — proving the two-quantity contract (count != damage radius).
#[test]
fn aoe_cs_half_threshold_bounds_block() { /* impact unit damaged; 4 neighbors rejected */ }

// CS=1.5: 21 cells scanned, threshold = ftol(1.5*256)=384 leptons. Unit at 1 cell (256 lep) damaged;
// unit at 2 cells (512 lep) rejected.
#[test]
fn aoe_cs_1_5_radius() { /* ... */ }

// 3 units inside the disk -> damage-list order equals OFFSET_TABLE scan order, NOT the old sorted
// order. Guards the RNG/chain-order parity.
#[test]
fn aoe_target_order_matches_table() { /* ... */ }
```

Fill each test body with a concrete fixture (place entities at known `rx,ry,sub` and assert the exact
`(stable_id, dmg)` list and its order). Use the existing combat test helpers for store/warhead
construction (grep `apply_aoe_damage` test usages in `src/sim/combat/`).

**Step 3: Verify.** Deferred to Task 6 (crate must build first).

**Step 4:** No commit yet (Slice-3 commit lands after Task 6).

---

### Task 6: Re-point the ore-destruction consumer (`mod.rs`)

**Why:** `destroy_ore_at_impact` uses the wrong `to_num::<u32>()` radius; at CS=0.5 it emits 1 reduction
request instead of gamemd's 9. The ore path has no lepton filter, so it is the clean demonstration of
the count-rule fix.

**Files:** Modify `src/sim/combat/mod.rs:1162-1163`.

**Step 1:** Replace:

```rust
    let spread_radius = cell_spread.to_num::<u32>();
    for &(dx, dy) in self::cell_spread::cells_in_spread(spread_radius) {
```

with:

```rust
    for &(dx, dy) in self::cell_spread::splash_cells(cell_spread) {
```

(Leave the `cx/cy >= 0` guard and `TiberiumReductionRequest` push unchanged.)

**Step 2: Add ore tests.**

```rust
// CS=0.5 ore warhead -> 9 reduction requests (3x3 block) at the embedded-table cells, in order.
// (Was 1 before the fix.)
#[test]
fn ore_cs_half_emits_9_requests() { /* assert requests.len()==9 and offsets==OFFSET_TABLE[..9] */ }

// CS=2 -> 21 requests at exactly the embedded-table offsets, in order.
#[test]
fn ore_cs_2_emits_21_requests_in_table_order() { /* ... */ }
```

**Step 3: Verify.** `cargo build -p vera20k` then `cargo test -p vera20k combat -- --nocapture`. The
crate now builds (no `cells_in_spread` references remain — grep to confirm zero hits). Read the literal
`test result:` line. Some pre-existing combat golden tests may now report different (gamemd-correct)
damage — update those values to match the fix (do not revert), and note each updated test in the commit.

**Step 4: Commit** (`cell-spread: repoint splash + ore consumers to gamemd index/threshold (Slice 3)`).

---

### Task 7: Prove the fixed-point index rule over the real stock input set

**Why:** gamemd computes the index from a *float* `CellSpread`; Rust uses `SimFixed` (I16F16). They can
disagree at non-half decimals (e.g. x.01). Parity is proven over the *actual* inputs, not all reals.

**Files:** add a test to `src/sim/combat/cell_spread.rs`; read `ini/rulesmd.ini`.

**Step 1:** Grep `ini/rulesmd.ini` for every `CellSpread=` value (Warhead sections). Collect the distinct
set (expected: mostly `0`, `0.5`, `1`, `1.5`, `2`, … up to ~10).

**Step 2:** For each distinct stock value `v`, assert `splash_count_index(SimFixed::from_num(v))` equals
the gamemd `ftol(v_as_f64 + 0.99)` (compute the reference with `f64` in the **test only** — tests may use
float; sim logic may not). Likewise assert `splash_threshold_leptons` equals `(v_as_f64 * 256.0) as i64`
(trunc). Embed the enumerated values as a literal array in the test with a comment citing the INI source.

```rust
#[test]
fn stock_cellspread_values_match_gamemd_float_rule() {
    // Distinct CellSpread= values from ini/rulesmd.ini (transcribe in Step 1).
    const STOCK_CS: &[f64] = &[/* 0.0, 0.5, 1.0, 1.5, 2.0, ... */];
    for &v in STOCK_CS {
        let fx = SimFixed::from_num(v);
        let want_count = (v + 0.99) as i64; // ftol truncates toward zero; v>=0
        assert_eq!(splash_count_index(fx).min(11), (want_count.max(0) as usize).min(11), "count CS={v}");
        assert_eq!(splash_threshold_leptons(fx), (v * 256.0) as i64, "lepton CS={v}");
    }
}
```

**Step 3: Verify.** `cargo test -p vera20k stock_cellspread -- --nocapture`. If any value fails, switch
`splash_count_index`'s addend to the bit-exact double form for I16F16 (`SimFixed::from_bits(64881)` ≈
0.99) and/or `splash_threshold_leptons` to `cell_spread.to_bits() as i64 / 256`, re-run until all pass,
and document the chosen form.

**Step 4: Commit** (`cell-spread: prove index rule over stock rulesmd CellSpread set (Slice 2 gate)`).

---

### Task 8: Full regression + state-hash sanity

**Why:** `apply_aoe_damage` is hot; confirm no unrelated breakage and that determinism holds.

**Files:** none (verification).

**Step 1:** `cargo test -p vera20k` (full). Read the literal `test result:` line. Expected: PASS, with
only intentionally-updated AoE golden values changed.
**Step 2:** `cargo clippy -p vera20k` — no new warnings in the touched files.
**Step 3:** Determinism check — run any existing state-hash/replay determinism test twice; assert the
two runs produce the *same* hash as each other (determinism preserved). The hash will differ from a
pre-fix baseline — that is expected and acceptable (behavior parity fix; document it).
**Step 4: Commit** any golden-value updates not already committed (`cell-spread: update AoE golden values to gamemd-correct results`).

---

### Task 9: Decide the CS=0 cell-target case + gamemd fidelity verification

**Why:** Close the one UNCHECKED contract item and confirm the slice matches gamemd end-to-end.

**Files:** possibly `src/sim/combat/mod.rs` (only if a change is warranted).

**Step 1: Verify the CS=0 force-fire-on-cell gap.** gamemd CS=0 scans the impact cell (count[0]=1) and
damages its occupants; Rust's else-branch (`mod.rs:2220`) damages only an explicit `Entity` target, so a
`TargetKind::Cell` shot with CS=0 damages nobody. Determine via `ini/rulesmd.ini` whether any stock
weapon force-fires with a CS=0 warhead at a cell (vs always having an entity target). If none observably
hits this path in stock play, record it as a documented, dormant DRIFT and leave the code (avoid a
regression for a non-stock case). If a stock weapon does, route CS=0 cell-targets through
`splash_cells(SIM_ZERO)` (1 cell) collecting impact-cell occupants, and add a test.

**Step 2: gamemd fidelity check.** Cross-check one concrete scenario against the binary contract:
a CS=2.0 warhead detonation — assert the Rust damage set + order equals `OFFSET_TABLE[..21]` scan order
with the 512-lepton gate (study §5). Use `/fidelity-check` or an in-game side-by-side if available.

**Step 3:** Record the CS=0 decision + fidelity result in the study doc's §4e (one line, cite the call).

**Step 4: Commit** (`cell-spread: resolve CS=0 cell-target case + fidelity note (Slice 3 close)`).

---

## Sources & References

- **Design doc:** `docs/research/substrate/tables/CELL_SPREAD_SUBSTRATE_STUDY.md` (master:
  `docs/research/substrate/LOOKUP_TABLE_SUBSTRATE_SERVICE_STUDY.md`)
- **Ghidra reports:** `CELLSPREAD_OFFSET_TABLE_DUMP_GHIDRA_REPORT.md` (stale on idx96 + `ftol(CS)`),
  `WARHEAD_DETONATE_GHIDRA_REPORT.md` §4, `PSYCHIC_REVEAL_SUPERWEAPON_GHIDRA_REPORT.md` §3.
- **gamemd.exe addresses (kept here, not in code comments):** initializer `0x00561910`; reader
  `Apply_area_damage 0x00489280` (`004895C7/CF` reader, `00489592` count, `004892DD` threshold,
  `00489347` air flag); count table `0x007ED3D0`; offset table `0x00ABD490` (BSS); constants
  `0x007E5160`=0.99, `0x007E2224`=256.0, `0x007E5168`=0.5; `ftol 0x007c5f00`.
- **INI keys:** `ini/rulesmd.ini` `[<Warhead>] CellSpread=` → `src/rules/warhead_type.rs:117`.
- **Related code:** `src/sim/combat/cell_spread.rs`, `combat_aoe.rs:90-156`, `mod.rs:1151-1174,2160-2274`.
- **Out of scope (flagged for later slices):** module relocation to `sim/world/substrate/`; shroud-reveal
  consumer reuse (study Slice 5, gated on the vision port — needs the per-cell `ftol(sqrt(dx²+dy²))≤sight`
  gate); AoE falloff lepton-space rounding (combat-falloff family).
