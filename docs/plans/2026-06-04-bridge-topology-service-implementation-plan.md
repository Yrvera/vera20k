# Bridge-Topology Service — Implementation Plan (Slice 3)

**Status:** IMPLEMENTATION PLAN (from reviewed design `2026-06-04-bridge-topology-service-design.md`, verdict YELLOW with corrections applied). Doc-only — no `src/` touched by this plan; code blocks below are the proposed implementation, not yet applied.
**Date:** 2026-06-04
**Package:** `vera20k` (build/test with `-p vera20k`).
**Rule:** Rust-native structure, gamemd-native semantics. Reproduce the verified observable contract; do NOT port the C++ CellClass tree / raw object-list pointers / COM vtable plumbing literally.
**Slots into:** core-engine-substrate program, map/cell-substrate workstream #7 (`docs/plans/2026-05-29-core-engine-substrate-todo.md`). Rollout rhythm mirrors Mission/Radio substrate.
**`advance_tick` phase order unchanged throughout.**

---

## Plan-review corrections (2026-06-04, /review-plan)

Verdict: **YELLOW — fixed, ready** after the corrections below. Every item cites the read/grep done this review run.

1. **`PER_LEVEL_HEIGHT` does NOT exist (P1c will not compile as written).** Grep `src/`: no `PER_LEVEL_HEIGHT` const anywhere (verified `grep -rn "PER_LEVEL_HEIGHT\|LEVEL_HEIGHT" src/`). The placeholder `BRIDGE_DECK_HEIGHT = 4 * crate::map::resolved_terrain::PER_LEVEL_HEIGHT` references a non-existent path. **Worse — it is the wrong DOMAIN.** The existing AoE threshold operates in **cell-level units**, not leptons: `combat_aoe.rs:220` compares `impact_z > cell.level as i32 + bridge_height/2`, and `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS: i32 = 4` (`combat_aoe.rs:31`) is a **level count**, not a lepton height. `LEPTONS_PER_LEVEL=104` lives in `util/lepton.rs:76` and is a SEPARATE domain used by `in_range.rs`. **Fix:** `BRIDGE_DECK_HEIGHT` must resolve to the integer **`4`** (deck = 4 levels), matching the existing `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS`. Do NOT multiply by any per-level lepton constant in this selector. A2's "round(per_level×4)" is the binary's lepton-domain framing; the Rust selector already pre-divides to levels, so the const here is `4`. Re-source `BRIDGE_DECK_HEIGHT` from `combat_aoe::BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS` (or move that const into the service) — single source, value `4`.

2. **`iso_tile_index: i32` is NOT a field of `ResolvedTerrainCell` (P1c `CellBridgeView` and `is_bridge_tileset`/`is_wood_bridge_tileset` will not compile).** Verified `resolved_terrain.rs:119-205`: the fields are `final_tile_index: i32` (`:123`, raw tile id), `source_tile_index: i32` (`:121`), and `tileset_index: Option<u16>` (`:131`, the resolved SET index). There is no `iso_tile_index`. **Also: the wood-bridge tileset predicate ALREADY EXISTS precomputed** as `is_wood_bridge_repair_tile: bool` (`:128`, "first 16 tiles of the theater WoodBridgeSet" — exactly the L4 window). **Fix:** (a) `CellBridgeView` should carry `final_tile_index: i32` (or `tileset_index: Option<u16>`) — pick the one the binary window-compares against and name it after the real field; (b) `is_wood_bridge_tileset` should DELEGATE to / shadow-test against the existing `is_wood_bridge_repair_tile` rather than re-derive the window, satisfying the single-source rule the plan asserts elsewhere; (c) confirm in P1a which field the concrete-bridge window (L3) compares — `final_tile_index` vs `tileset_index` — before writing `is_bridge_tileset`.

3. **`CellBridgeView.level: i8` vs source `ResolvedTerrainCell.level: u8` / `PathCell.ground_level: u8`.** Verified `resolved_terrain.rs:129` (`pub level: u8`) and `core.rs:1505-1507` (`signed_level()` = `ground_level as i8 as i16`). The view's `level: i8` and `effective_height` (`self.level as i32`) are correct ONLY if the constructor casts `u8 -> i8` exactly like `signed_level()` (i.e. `cell.level as i8`). **Fix:** document the `as i8` cast at the view constructor and add it to the `effective_height_anchor_plus4_signed_level` test (feed a raw `u8` level >127, e.g. 0xFE, assert it reads as -2) so the sign-reinterpretation is proven, not assumed.

4. **`parity_replay` test filter matches NOTHING — false-green trap across P3/P4/P5/P6.** Verified `global_parity_harness_tests.rs:132`: the only harness test is `global_skirmish_replay_is_deterministic_and_baseline_stable`. `cargo test` filters by substring and `parity_replay` is not a substring of that name → **0 tests run, reported as pass** (exactly the failure mode CLAUDE.md's cargo note warns about). **Fix (applied via sed in this doc):** every `cargo test -p vera20k parity_replay` in P3/P4/P5/P6 and the acceptance table is now `cargo test -p vera20k global_skirmish_replay` (or use `replay_is_deterministic`). The new named cases (`bridge_high_fixture_replay_identical_after_gate_relocation`, etc.) must be ADDED to that harness file or their own `*_replay` test fns, or they too will not exist to run.

5. **DRIFT #6 retarget (minor).** Confirmed tileset detection is NOT in `zone_build.rs` (verified `ls src/sim/pathfinding/` + grep). The precompute actually lands on `ResolvedTerrainCell` at map-load (`resolved_terrain.rs`), and `terrain_cost.rs` consumes it when building `PathCell`. P7's "route `terrain_cost.rs`/`core.rs` tileset detection" is directionally right; tighten the wording to "route through the existing `ResolvedTerrainCell.is_wood_bridge_repair_tile` precompute, do not re-derive."

6. **`move_entity` split (P5) — CONFIRMED feasible.** Verified `occupancy.rs:233-246`: `move_entity` already internally does `self.remove(old)` (no layer arg) then `self.add(new, ..., layer, ...)`. Splitting into `remove(old)` + `add(new, new_layer)` or adding `move_entity_layered(old_layer, new_layer)` is trivial; only `add` takes a layer, so the old-layer remove needs no layer. The P5 "UNCHECKED — read first" note can be downgraded to CONFIRMED.

7. **L13 already-correct (clarify, not a fix).** `game_entity.rs:757` `occupancy_list_layer()` already sources the layer from `self.on_bridge`, not loco layer (verified `:743-762`). The DRIFT #2 defect is purely the `movement_step.rs:1190-1207` single-layer move (confirmed: one `occupancy_layer` from `projected_on_bridge_state`, used for both halves of `move_entity`). The test `occupancy_list_layer_from_on_bridge_not_loco_layer` documents existing-correct behavior; the behavior CHANGE is only the move-ordering. Keep both, but label which is regression-guard vs new.

**Residual UNCHECKED (binary side) — UPDATED after gate resolutions:** A1's deck-offset arithmetic/domain is now CLOSED (`GATE_BRIDGE_DECK_HEIGHT_RESOLUTION_GHIDRA_REPORT.md`: `2×per_level` leptons, corrects A2's "round(src×4)"; `+4` is a separate Level seed); the only A1 residual is `cell.level == GetGroundHeight` on ramps (P0b). A2's occupancy representation/ordering/Clear-asymmetry is CLOSED (`GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md`). The CheckBridgeTraversal gate (gate "Bridge A4") is CLOSED (`GATE_BRIDGE_TRAVERSAL_RESOLUTION_GHIDRA_REPORT.md`). The render shadow-DX (plan-internal anchor A4, -15 vs -45) stays open (P7). Keep P4/P5 UNCHECKED only until P0b's `cell.level`-vs-`GetGroundHeight` equality lands and the `4`-vs-`2`-level deck-height value is reconciled (see P4).

---

## Anchors verified this run (read before trusting any edit line range)

All line ranges below were READ this run on branch `dev`. Re-confirm before applying — parallel sessions move lines.

- `src/map/bridge_facts.rs:3-9` — flag-bit consts `BRIDGE_FLAG_ANCHOR_SELF=0x80`, `_STRUCTURAL=0x100`, `_TRANSITION=0x200`, `_DESTROYED_OR_RAMP=0x400`, `_DIRECTION_ZERO=0x800`, `_FORWARD_SIDE=0x1000`, `_EXTRA_SIDE=0x10000`. `BridgeCellFacts` predicates at `:63-79`. (READ)
- `src/sim/pathfinding/core.rs:467-481` — `BridgeTraversalInput<'a>` { `candidate: &'a PathCell`, `candidate_coord: (u16,u16)`, `direction: i8`, `path_height: i16`, `parent: Option<(&'a PathCell,(u16,u16))>` } and `BridgeTraversalResult` { `allowed: bool`, `path_height: i16`, `force_bridge_list: bool` }. (READ — the design's "`direction: i8`, no `&mut`" correction is confirmed; `path_height` is `i16` not `i32`.)
- `src/sim/pathfinding/core.rs:483-592` — `resolve_parent_for_bridge_traversal` (`(dir-4)&7` over `NEIGHBORS`) + `check_bridge_traversal(grid: &PathGrid, input: BridgeTraversalInput) -> BridgeTraversalResult`, `pub(crate)`. Both abs==4 sub-branches present at `:568-583`. (READ)
- `src/sim/pathfinding/core.rs:1457-1524` — `PathCell` struct fields + predicate methods (`has_structural_bridge`, `has_bridge_marker_0x80`, `has_bridgehead_transition`, `bridge_deck_level_if_any`, `effective_cell_z_for_layer`, `is_elevated_bridge_cell`, `signed_level()->i16`, `is_low_bridge_tube_cell`). (READ)
- `src/sim/pathfinding/core.rs:1049-1058` (A* caller) and `src/sim/movement/movement_occupancy.rs:159-168` (runtime caller) — the TWO `check_bridge_traversal` call sites. (READ)
- `src/sim/pathfinding/mod.rs:35` — `pub use self::core::*;` (gate + input/result re-exported crate-wide). (READ)
- `src/sim/combat/combat_aoe.rs:206-229` — `select_object_damage_layer(...)` with strict-`>` at `:220` (`impact_z > cell.level as i32 + bridge_height / 2`); `bridge_height_for_selector` at `:227`; `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS=4` at `:31`; `AoELayerContext` at `:35`; `bridge_adjusted_impact_z` at `:45`. (READ)
- `src/sim/occupancy.rs:115-142` — `OccupancyGrid::rebuild` already uses `entity.occupancy_list_layer()` (`:127`). (READ)
- `src/sim/game_entity.rs:743-762` — `occupancy_list_layer()` derives `Bridge`/`Ground` from `self.on_bridge` (`:757`). (READ)
- `src/sim/movement/movement_step.rs:1170-1213` — `process_cell_crossings` computes a SINGLE `occupancy_layer` from `projected_on_bridge_state` (`:1190`) and uses it for both halves of `occupancy.move_entity` (`:1201`). (READ — DRIFT #2 confirmed.)
- `src/sim/world/bridge_orchestrator.rs:1353` — `fn drop_in_bridge_deck_entities(sim, rx, ry)`. (READ via grep)
- `src/app_instances/bridges.rs:34-48` — `BRIDGE_BODY_Y_OFFSET_STATE_0_TO_8=-16.0`, `_STATE_9_TO_17=-31.0`, `BRIDGE_SHADOW_EW_DX=-15` (in-code comment `:43-44` flags -15 vs -45 UNRESOLVED), `BRIDGE_SHADOW_EW_DY=7`. (READ)
- `src/sim/snapshot.rs:24` — `const SNAPSHOT_VERSION: u32 = 17;`. (READ)
- `src/sim/mod.rs:60-99` — module decls; NO `pub mod map;`. `sim/map/` does not exist. (READ)
- Tileset detection (DRIFT #6 consumers): `is_wood_bridge_repair_tile` / `tileset_index` fields live in `src/sim/pathfinding/terrain_cost.rs` and `src/sim/pathfinding/core.rs` (grep this run), NOT `zone_build.rs`. (GREP)

### OPEN ASSUMPTIONS the plan-review MUST verify before P4/P5 authority

- **A1 (Open Q #7) — PARTIALLY RESOLVED (`GATE_BRIDGE_DECK_HEIGHT_RESOLUTION_GHIDRA_REPORT.md`).** A1 closed the deck-offset arithmetic/domain: coordinate-Z deck = `GetGroundHeight + 2×per_level` leptons; `GetGroundHeight 0x00578080` returns ground-only Z in leptons. **Still open (the real P0b question):** whether the Rust `cell.level` (Level units) faithfully equals `GetGroundHeight`/`FUN_0047b3a0`'s lepton ground-Z on ramps/slopes. P0b proves or routes this; until proven, P4/P5 parity is **UNCHECKED**, not "bit-identical relocation." **NEW CONTRADICTION from A1:** the deck-height const is `2 × per_level` (208 leptons / 2 levels), NOT `round(per_level×4)` (A2 framing) and NOT the Rust `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS = 4`. Reconcile the `4`-vs-`2`-level value at P4/P5 cutover (see P4).
- **A2 — DECK-HEIGHT FRAMING CORRECTED (by A1).** AoE uses `DAT_0089E864`, occupancy Mark/Clear uses `DAT_00B1D0AC` — DISTINCT symbols, but A1 shows BOTH = `2 × per_level` (the `×4 then ×0.5` idiom = `×2`), **NOT `round(per_level×4)`**. Use ONE named Rust const `BRIDGE_DECK_HEIGHT` only after confirming both resolve to the same integer at cutover (they share the idiom). The occupancy two-layer representation + ordering + Clear asymmetry are separately CLOSED by `GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md` (folded into P5).
- **A3 — STILL OPEN.** `CellBridgeView` construction site (Open Q #4): which store exposes all seven fields (`level`, `flags`, `ramp_byte`, `iso_tile_index`, `tube_index`, `land_type`, `state_byte`) as one borrowable struct post-map-load. `PathCell` holds the gate-relevant subset; `ResolvedTerrainCell` holds tileset/level. The view is initially a borrow-adapter over BOTH, not a new store. Plan-review must confirm `ResolvedTerrainCell` exposes `iso_tile_index`/`land_type`/`tube_index` (UNCHECKED this run — Task P1a reads it first). *(Not a gate covered by this run's resolutions.)*
- **A4 (render shadow-DX — distinct from the gate "Bridge A4"/CheckBridgeTraversal, which is CLOSED → P3) — STILL OPEN.** L18 shadow-DX -15 is UNRESOLVED (-15 vs -45 per `bridges.rs:43-44`). P7 carries it as open, does not settle it.

---

## Task graph (dependency order)

```
P0a (INI confirm) ─┐
P0b (ground_z proof)─┼─> P1a (read ResolvedTerrainCell) ─> P1b (sim/map skeleton + BridgeFlags)
                     │        └─> P1c (CellBridgeView + 7 predicates, shadow tests)
P1c ─> P2 (gate shadow harness) ─> P3 (gate relocate authoritative)
P0b + P1c ─> P4 (AoE layer authoritative)
P3, P4 ─> P5 (occupancy list-layer + move-ordering; HASH-RELEVANT)
P5 ─> P6 (DropIn relayer; hash-relevant)
P1c ─> P7 (BridgeDrawOffset in render/; retire scattered predicates)
```

**Task count: 11** (P0a, P0b, P1a, P1b, P1c, P2, P3, P4, P5, P6, P7).
**Hash-relevant: P3, P4, P5, P6.** Of these, only **P5** (and possibly **P6**) may change the hashed occupancy representation → `SNAPSHOT_VERSION` bump + parity-baseline regen. P3/P4 are bit-identical relocations (replay-verify, no bump expected) **contingent on A1**.

**Gate resolutions folded in (2026-06-04) — READY status:**
- **A4 (CheckBridgeTraversal gate + warhead `+0x144` + vtable `+0x1B0`) CLOSED** → `docs/research/GATE_BRIDGE_TRAVERSAL_RESOLUTION_GHIDRA_REPORT.md`. P3's §1.3 decision table is fully binary-verified; **P3 is READY TO IMPLEMENT** (was relocating an already-correct gate; the resolution confirms the gate shape verbatim and adds the `+0x144`=Wall and `+0x1B0` inventory facts). See P3.
- **A1 (deck height / GetGroundHeight Z-init) CLOSED** → `docs/research/GATE_BRIDGE_DECK_HEIGHT_RESOLUTION_GHIDRA_REPORT.md`. **CONTRADICTS the prior A2 "round(src×4)" framing**: the coordinate-Z deck offset is `2 × per-level-height` (≈208 leptons), computed `×4 then ×0.5` = `×2`, NOT `round(src×4)`; and the `+4` is a SEPARATE Level-unit pathfinding seed. This resolves P0b's domain question and tightens P4/P5's deck-height const. See P0b, P4, P5.
- **A2 (OnBridge occupancy representation) CLOSED** → `docs/research/GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md`. The two-layer (object-LIST by `on_bridge`, occupancy-BIT by Z-height) representation, the Clear-vs-Mark asymmetry, and the remove-old/write/add-new ordering are all binary-verified. **P5 is READY** (it remains HASH-RELEVANT) and consequently **P6 is unblocked**. See P5, P6.

Implementing any READY task that touches the hashed occupancy representation (P5, possibly P6) flips hashed state → shadow → invert → authoritative → `SNAPSHOT_VERSION 17→18` → regenerate the parity baseline (the global replay harness). P3/P4 stay bit-identical relocations (replay-verify, no bump) contingent on A1's now-resolved domain.

---

## P0a — INI confirm: `BridgeStrength=1500` (READ-ONLY, no hash)

**Files:** none edited. Verification only.
**Action:** Confirm `ini/rulesmd.ini [CombatDamage] BridgeStrength` = `1500` and that the Rust parse default-when-absent is `100` (study §2.5). No code change — this is the research gate for the damage-path const used later (`BRIDGE_STRENGTH_DEFAULT`).
**Verify:**
```
cargo test -p vera20k bridge_strength
```
Named test (add in P4/P6 scope where the const is consumed, not here): `bridge_strength_default_is_1500_from_rulesmd`. P0a itself is a doc/INI confirm; no test compiles in this task.
**Depends on:** none. **Hash:** none.

---

## P0b — Prove or route `cell.level == GetGroundHeight()` for AoE/occupancy operand (READ-ONLY, no hash) — PARTIALLY RESOLVED by A1

**Files:** none edited (research/decision task; resolves A1 / Open Q #7).

**A1 CLOSED — what it settles and what it leaves open** (`docs/research/GATE_BRIDGE_DECK_HEIGHT_RESOLUTION_GHIDRA_REPORT.md`): A1 resolved the *deck-offset arithmetic and domain* — the coordinate-Z path is `unit.Z = GetGroundHeight(Coord) + DECK_OFFSET`, where `DECK_OFFSET = DAT_00AC13BC = 2 × per-level-height` in **leptons** (nominally 208), and `GetGroundHeight 0x00578080` returns ground-only Z in leptons (cell-grid frame), delegating to `FUN_0047b3a0`. **What A1 does NOT settle (the actual P0b question stays OPEN):** whether the Rust `ResolvedTerrainCell.level` (a Level-unit value) equals `GetGroundHeight(cell)` (a lepton ground-Z) on ramps/slopes. A1 in fact shows these are TWO domains — the binary's AoE/occupancy thresholds operate in leptons against `GetGroundHeight`, while the Rust selector pre-divides to Level units against `cell.level`. So the equality is `cell.level (levels) ?= GetGroundHeight (leptons)/LEPTONS_PER_LEVEL` on every operand cell, which A1 neither proves nor refutes for ramps.

**Action (still required — A1 narrowed it, did not close it):** Decide one of:
1. **Prove equality** — empirically show `ResolvedTerrainCell.level == round(GetGroundHeight(cell)/per_level)` across flat, ramp, sloped, and bridge-deck cells (the cells the AoE selector and Mark/Clear see). If proven across that input space, P4/P5 may use `cell.level` and remain bit-identical relocations.
2. **Route GetGroundHeight** — if NOT equal on ramps/slopes, add a `ground_z` accessor mirroring `GetGroundHeight` (lepton ground-Z) and have `aoe_object_layer`/`occupancy_bit_layer` take it explicitly in the SAME domain as the threshold (the design signatures already pass `ground_z: i32`).

**Remaining query (precise):** is `ResolvedTerrainCell.level` a faithful Level-unit projection of `GetGroundHeight`/`FUN_0047b3a0`'s lepton ground-Z on ramp/slope cells? Empirical boundary-cell check, or decode `FUN_0047b3a0` and compare to the Rust level-resolution path.

Default-to-DRIFT: until (1) is empirically demonstrated on boundary cells, treat the operand as DRIFT and plan for (2).
**Verify:** documented finding + (if route chosen) a `ground_z_matches_getgroundheight_on_ramp` fixture test added in P4.
**Depends on:** none. **Hash:** none (decision only; the operand choice lands in P4/P5).

---

## P1a — Read `ResolvedTerrainCell` to finalize `CellBridgeView` fields (READ-ONLY, no hash)

**Files:** none edited (read pass; resolves A3 / Open Q #4).
**Action:** Read `src/map/resolved_terrain.rs` (`ResolvedTerrainCell`, `bridge_facts`, `level`, `bridge_deck_level`, tileset/iso-tile index, land-type, tube-index) and confirm which of the seven `CellBridgeView` fields it exposes vs which only `PathCell` holds. Output: the exact field-source map for the view constructor. This is the only blocker to writing `CellBridgeView` against real types.
**Verify:** none (read pass). Output recorded inline in the P1b/P1c task notes before coding.
**Depends on:** none. **Hash:** none.

---

## P1b — `sim/map/` skeleton + single-source `BridgeFlags` (READ-ONLY additive, no hash)

**Create:** `src/sim/map/mod.rs`, `src/sim/map/bridge_topology.rs`.
**Edit:** `src/sim/mod.rs` (add module decl), `src/map/bridge_facts.rs` (add `BridgeFlags` wrapping the existing consts — single source).

**`src/sim/mod.rs`** — insert after the cell-occupancy block (anchor: `pub mod occupancy;` at `:70`):
```rust
// --- Map/cell substrate (read services over the canonical cell store) ---
pub mod map; // bridge topology service (first member of map/cell-substrate workstream #7)
```

**`src/sim/map/mod.rs`** (new):
```rust
//! Map/cell-substrate read services. First member: bridge topology.
//!
//! Depends on: map/ (bridge_facts flag bits, resolved_terrain), util/direction.
//! NEVER depends on render/, ui/, sidebar/, audio/, net/ (invariant #1).
pub mod bridge_topology;
```

**`src/map/bridge_facts.rs`** — add a `bitflags`-free wrapper that REUSES the existing `:3-9` consts as the single source (no third copy of the bit values). Insert after `:9`:
```rust
/// Typed view of the CellClass flag word, single-sourced from the consts above.
/// Bit values are NOT redefined here — they reference the `BRIDGE_FLAG_*` consts
/// so map-load (this file), the topology service, and render share one source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BridgeFlags(pub u32);

impl BridgeFlags {
    #[inline] pub fn has(self, bit: u32) -> bool { self.0 & bit != 0 }
    #[inline] pub fn anchor(self) -> bool { self.has(BRIDGE_FLAG_ANCHOR_SELF) }
    #[inline] pub fn structural(self) -> bool { self.has(BRIDGE_FLAG_STRUCTURAL) }
    #[inline] pub fn bridgehead(self) -> bool { self.has(BRIDGE_FLAG_TRANSITION) }
}
```
(Project uses no `bitflags!` here per the existing const style; a thin newtype keeps one source of values. If `bitflags` is already a dep elsewhere, the reviewer may swap — but the bit VALUES must still come from the `BRIDGE_FLAG_*` consts, not be re-literal'd.)

**Verify:**
```
cargo check -p vera20k
cargo test -p vera20k bridge_facts
```
Named test (add to `bridge_facts.rs` tests): `bridge_flags_newtype_matches_const_predicates` — asserts `BridgeFlags(0x100).structural()` etc. agree with `BridgeCellFacts::has_structural_bridge` for the same raw flags.
**Depends on:** P1a (field map). **Hash:** none (pure additive; nothing wired in).

---

## P1c — `CellBridgeView` + seven predicates + signed effective-height; shadow assert-equal (READ-ONLY additive, no hash)

**Edit:** `src/sim/map/bridge_topology.rs`.

Implement the borrowed view and the seven predicates. `CellBridgeView` is a borrow-adapter over the canonical store fields established in P1a (NOT a new owned store). All math integer/`i8`-signed — no f32/f64.

```rust
//! Bridge topology read service (Slice 3). Single owner of gamemd-native bridge
//! bit semantics, signed effective-height, the traversal gate, AoE/occupancy
//! layer selectors, and the boundary transition.
//!
//! Depends on: map/bridge_facts (flag bits), map/resolved_terrain, util/direction.
//! NEVER depends on render/ — render offset lives in a separate trait in render/.

use crate::map::bridge_facts::{
    BridgeFlags, BRIDGE_FLAG_ANCHOR_SELF, BRIDGE_FLAG_STRUCTURAL, BRIDGE_FLAG_TRANSITION,
};

/// L17: full 4-level bridge-deck height = round(per_level_height * 4). Engine
/// iso-geometry constant (study §2.2); same resolved integer for AoE and occupancy
/// thresholds (DAT_0089E864 / DAT_00B1D0AC). Resolved value: confirm at cutover (A2).
// CORRECTED (plan-review #1): value is `4` in CELL-LEVEL units (deck = 4 levels), NOT
// leptons. The existing selector compares against `cell.level` in level units and uses
// `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS = 4` (combat_aoe.rs:31). Do NOT multiply by
// LEPTONS_PER_LEVEL here. `PER_LEVEL_HEIGHT` does not exist in src/. Single-source from
// the existing const (move it into the service, or re-export it).
pub const BRIDGE_DECK_HEIGHT: i32 = 4; // = combat_aoe::BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS (levels, not leptons). A2: binary "per_level×4" is the lepton-domain framing; Rust pre-divides to levels.

/// Borrowed read view of one cell's bridge-relevant substrate fields.
/// Field sources finalized in P1a. Does NOT own storage.
#[derive(Debug, Clone, Copy)]
pub struct CellBridgeView {
    pub level: i8,
    pub flags: BridgeFlags,
    pub ramp_byte: i8,          // CellClass+0x11C slope/ramp passability (PathCell.slope_type)
    pub iso_tile_index: i32,
    pub tube_index: Option<i16>,
    pub land_type: u8,
    pub state_byte: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListLayer { Ground, Bridge }

impl CellBridgeView {
    // L1 / C1-C3
    #[inline] pub fn is_bridge_cell(&self) -> bool { self.flags.structural() }
    #[inline] pub fn is_bridgehead(&self) -> bool { self.flags.bridgehead() }
    #[inline] pub fn is_anchor(&self) -> bool { self.flags.anchor() }

    // L2 / C4 — signed level + (anchor?4:0). NOT the layer-driven form.
    #[inline]
    pub fn effective_height(&self) -> i32 {
        self.level as i32 + if self.is_anchor() { 4 } else { 0 }
    }

    // L3 / C5 — concrete bridge tileset window, distinct from structural 0x100.
    #[inline]
    pub fn is_bridge_tileset(&self, base: Option<i32>) -> bool {
        base.is_some_and(|b| b >= 0 && (b..b + 0x10).contains(&self.iso_tile_index))
    }
    // L4 / C5 — wood bridge tileset window, distinct from concrete AND structural.
    #[inline]
    pub fn is_wood_bridge_tileset(&self, wood_base: Option<i32>) -> bool {
        wood_base.is_some_and(|b| b >= 0 && (b..b + 0x10).contains(&self.iso_tile_index))
    }

    // L5 / C6 — both conditions: tube index in range AND land_type == 10.
    #[inline]
    pub fn is_low_bridge_cell(&self, tube_count: usize) -> bool {
        self.tube_index
            .is_some_and(|t| t >= 0 && (t as usize) < tube_count)
            && self.land_type == 10
    }
}
```
**Tileset bases are passed in** (`g_BridgeSet_TileSetBase` / `g_WoodBridgeSet_TileSetBase` equivalents) because they are theater-loaded, not cell-local — the view never aliases tileset to structural (DRIFT #6).

**Shadow test (assert-equal to existing helpers, over fixture cells):** add a `#[cfg(test)] mod tests` to `bridge_topology.rs`. Tests:
- `bridge_topology_predicates_match_pathcell` — for a battery of raw-flag/level fixtures, `CellBridgeView::{is_bridge_cell,is_bridgehead,is_anchor}` == the `PathCell::has_*` / `BridgeCellFacts::has_*` results.
- `effective_height_anchor_plus4_signed_level` — anchor cell with `level=-2` returns `-2+4=2`; non-anchor `level=-2` returns `-2`. (Signed read.)
- `is_bridge_tileset_distinct_from_structural_flag` — a cell with `iso_tile_index` in the bridge window but `flags & 0x100 == 0` returns `is_bridge_tileset==true`, `is_bridge_cell==false` (never aliased).
- `is_wood_bridge_tileset_distinct_from_concrete_and_structural` — wood window true, concrete window false, structural false.
- `is_low_bridge_requires_landtype10_and_tube_in_range` — tube in range + land 10 → true; tube in range + land 9 → false; tube out of range + land 10 → false.

**Verify:**
```
cargo test -p vera20k bridge_topology
```
**Depends on:** P1a, P1b. **Hash:** none (nothing routed yet; pure additive + tests).

---

## P2 — Traversal gate shadow harness (READ-ONLY, no hash)

**Edit:** `src/sim/map/bridge_topology.rs` (add a service-side gate entry that DELEGATES to the existing `pathfinding::check_bridge_traversal`), `src/sim/pathfinding/core_tests.rs` (extend golden table).

The gate already exists and is correct (`core.rs:506-592`). P2 does NOT rewrite it. It introduces a service-facing wrapper that calls through to the existing gate so combat/occupancy could reach it the same way, and runs a shadow golden table proving the wrapper is bit-identical to the existing function. No pathing behavior changes.

```rust
// In bridge_topology.rs — service-facing gate entry (P2). Delegates to the existing
// owner in pathfinding (relocation happens in P3); this proves the seam is identical.
pub use crate::sim::pathfinding::{
    check_bridge_traversal as bridge_traversal_gate, BridgeTraversalInput, BridgeTraversalResult,
};
```

**Golden table test** (extend `core_tests.rs`, which already has gate tests at `:116+`): add `bridge_traversal_golden_table_matches_decompile` covering each branch with `(candidate, dir, height, parent) -> (allowed, path_height, force_bridge_list)`:
- `traversal_parent_none_reconstructs_via_dir_minus4` (parent `None`, directed dir → reconstructs via `(dir-4)&7`).
- `traversal_dir_minus1_candidate_only_seed_no_bridgehead` (already exists as `bridge_traversal_direction_minus_one_seeds_candidate_bridge_height_without_bridgehead:117` — reuse/rename in table).
- `traversal_directed_diff4_enter_orientation` (parent.level == candidate.level-4: pass iff `path_height==candidate.level && parent structural`; no list-byte).
- `traversal_directed_diff4_exit_sets_list_byte` (candidate.level == parent.level-4: require candidate 0x100 && 0x200, set `force_bridge_list=true`).
- `traversal_diff_other_than_0_1_4_blocks`.

**Verify:**
```
cargo test -p vera20k bridge_traversal
cargo test -p vera20k -- bridge_traversal_golden
```
**Depends on:** P1c. **Hash:** none (delegating wrapper + tests; no caller rerouted).

---

## P3 — Gate authoritative: own the gate in the service, route both callers (HASH-RELEVANT, replay-verify, no bump expected) — READY (A4 CLOSED)

**Status:** READY TO IMPLEMENT. A4 is CLOSED — `docs/research/GATE_BRIDGE_TRAVERSAL_RESOLUTION_GHIDRA_REPORT.md` verifies the full §1.3 decision table from decompile + asm of `CheckBridgeTraversal 0x004D9C60`, and confirms the vtable slot `+0x1B0` holds it for Foot/Unit/Infantry (`read_memory 0x007F5E20/0x007EB208/0x007E8E44` all = `0x004D9C60`), Aircraft/Building override with non-bridge functions (dispatch must be ground-unit-only). **This relocates an already-correct Rust gate; the resolution confirms the existing `core.rs:483-592` shape is binary-faithful — so P3 stays a bit-identical relocation with no behavior change.**

**Resolved decision-table facts now binary-verified (cross-check the Rust `check_bridge_traversal` against these during relocation):**
- `parent==0 && dir!=-1` reconstructs the predecessor via `(dir-4)&7` (180° rotation over `g_DirectionOffsets`) — NOT "use the mover's current cell".
- `dir==-1` uses **candidate-only** height seeding (`*height = candidate.Level+4` iff `candidate.Flags&0x100`), returns 0, and skips all directed/bridgehead/diff/slope checks; the reconstruction still runs but its result is unused.
- Directed `*height==-1` & `parent.Flags&0x100` seeds from the **parent** (`*height = parent.Level+4`) and then REQUIRES the candidate be a bridgehead (`candidate.Flags&0x200`), else returns 7.
- Only `diff_abs ∈ {0,1,4}` are legal; `{2,3,5,6,7}` hard-block (return 7).
- `bridge_entered`/`force_bridge_list` is set ONLY on the ascend case (E4b: candidate LOW, `candidate.Level==parent.Level-4`, candidate has both `0x100` and `0x200`).
- The `Level+4` seed here is the **Level-unit pathfinding seed** (1 ElevationIncrement) — A1 confirms this is DISTINCT from the lepton coordinate-Z deck offset; do NOT conflate the two when wiring P3 alongside P4.

**RELOCATE, do not rewrite** (resolves Open Q #1 / study DRIFT #4 stale). Move `check_bridge_traversal` + `resolve_parent_for_bridge_traversal` + `BridgeTraversalInput`/`BridgeTraversalResult` from `pathfinding/core.rs:467-592` into `sim/map/bridge_topology.rs` VERBATIM (both abs==4 sub-branches `:568-583`, the `(dir-4)&7` reconstruct, the dir==-1 seed, diff-{0,1,4} ladder). Keep the `grid: &PathGrid` / `&PathCell` coupling — the gate reads `PathCell` predicates; do not reshape its API. **A4 also confirms the warhead `+0x144 = Wall=` (bool, default false) and that dispatch is ground-unit-only — fold the `+0x144` inventory note into the warhead parser work (out of this slice's scope but record it) and keep the gate off the aircraft/building cell-entry path.**

**Edit:**
- `src/sim/pathfinding/core.rs` — delete the moved block (`:467-592`); re-export from the new home so `pub use self::core::*` consumers still resolve, OR change `mod.rs` to `pub use crate::sim::map::bridge_topology::{check_bridge_traversal, BridgeTraversalInput, BridgeTraversalResult};`. Pick ONE re-export path to avoid a name clash.
- `src/sim/pathfinding/core.rs:1049` (A* caller) and `src/sim/movement/movement_occupancy.rs:159` (runtime caller) — no logic change; only the import path updates. **Preserve the explicit-parent (A*, `parent: Some(...)`) vs null-parent (runtime, `parent: explicit_parent` which is `None` for runtime) distinction.**
- `src/sim/pathfinding/core_tests.rs` — update imports; tests must pass UNCHANGED (proves bit-identity).

The gate still depends on `PathGrid`/`PathCell` (pathfinding types). Moving it to `sim/map/` means `sim/map/` imports `sim/pathfinding` types — acceptable (both are `sim/`, no invariant-#1 break). If the reviewer prefers zero new sim→pathfinding edge, the alternative is to keep the function body in `core.rs` and have the service re-export it as the sole public handle; either way there is ONE owner. **Recommended: keep body in `core.rs`, make the service the canonical re-export and document `core.rs` as private impl** — this avoids churning the `PathGrid` coupling and is still single-ownership at the API surface.

**Verify:**
```
cargo test -p vera20k pathfinding
cargo test -p vera20k bridge_traversal
```
Plus the **global parity replay harness** (the Slice-8 deterministic-replay baseline) over a high-bridge fixture must produce identical `world_hash` at every tick:
```
cargo test -p vera20k global_skirmish_replay
```
Named: `bridge_high_fixture_replay_identical_after_gate_relocation`.
**Depends on:** P2. **Hash:** paths feed pathing → occupancy → hash, but the relocation is bit-identical (no behavior change) → **no SNAPSHOT_VERSION bump expected**; replay must be byte-identical to baseline.
**Rollback:** revert the move; restore `core.rs:467-592` and the original re-export. Because no logic changed, rollback is a pure git revert with no save-format implication.

---

## P4 — AoE object-layer selector authoritative (HASH-RELEVANT via target set, replay-verify) — READY pending P0b; deck-height const CORRECTED (A1)

**Status:** READY TO IMPLEMENT once P0b chooses the operand domain. A1 (`docs/research/GATE_BRIDGE_DECK_HEIGHT_RESOLUTION_GHIDRA_REPORT.md`) closes the deck-height *arithmetic* and CONTRADICTS the prior "round(src×4)" framing — fold the correction in before cutover. Implementing this READY task with the AoE layer authoritative feeds the damage target set → hash; it stays a bit-identical relocation (no bump) ONLY if P0b's operand domain holds AND the deck-height const matches at cutover, else it is a behavior change needing its own shadow slice.

**A1 deck-height CORRECTION (CONTRADICTS A2's "round(src×4)" and the prior assumption):**
- The deck-offset writer (`0x005F3880`) computes `DAT_00AC13BC = ftol(per_level × 4 × 0.5) = per_level × 2` — the `×4 then ×0.5` is the gamemd idiom for `× 2`, so the full deck offset is **`2 × per_level_height`** (nominally 208 leptons = 2 × 104), **NOT `round(src × 4)`** and NOT a literal 4. The SAME idiom produces the AoE bridge threshold `DAT_0089E864 = 2 × DAT_0089E870` (confirmed in A1 §3 cross-check) — so the AoE selector's full-deck constant is `2 × per_level` in leptons, and the `/2` half-deck term is `per_level` (= 1 level).
- **CONTRADICTION to flag at cutover:** plan-review correction #1 set `BRIDGE_DECK_HEIGHT = 4` in CELL-LEVEL units (from the existing Rust `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS = 4`). A1 resolves the binary full-deck to **2 levels** (208 leptons ÷ 104 leptons/level), making the half-deck **1 level**, NOT 2. If the Rust const is truly `4` levels, the selector threshold `cell.level + 4/2 = cell.level + 2` would diverge from the binary `ground + per_level = ground + 1 level`. **This is a DRIFT to resolve before P4 cutover:** confirm whether the Rust `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS = 4` is itself correct against the binary's `2×per_level` (it may already be wrong, independent of this slice), or whether the Rust domain folds the factor differently. Default-to-DRIFT until the level-domain value is reconciled against A1's `2 × per_level` leptons.
- The `+4` Level-unit value seen in pathfinding (`CheckBridgeTraversal`/`GetEffectiveHeight` = `Level + 4`) is a SEPARATE representation (1 ElevationIncrement seed) and must NOT be reused as the coordinate-Z or AoE-threshold deck height. Two parallel reps of one deck; never mixed.

**Edit:** `src/sim/combat/combat_aoe.rs` (route `select_object_damage_layer` through the service), `src/sim/map/bridge_topology.rs` (add `aoe_object_layer`).

Fold the strict-`>` threshold (`combat_aoe.rs:220`) into the service, computed ONCE per detonation for the whole CellSpread (L10/L11). Keep collect-then-dispatch in the combat caller (L12).

```rust
// bridge_topology.rs — L10/L11. STRICT `>`. ground_z is GetGroundHeight-equivalent
// (A1): pass cell.level only if P0b proved equality, else pass routed ground_z.
// NOTE (A1): BRIDGE_DECK_HEIGHT must equal the binary 2×per_level (208 leptons / 2 levels),
// so the half-deck term `BRIDGE_DECK_HEIGHT / 2` = per_level (1 level / 104 leptons).
// Resolve the level-vs-lepton domain + the `4`-vs-`2` value contradiction before cutover.
impl CellBridgeView {
    pub fn aoe_object_layer(&self, impact_z: i32, ground_z: i32) -> ListLayer {
        if self.is_bridge_cell() && impact_z > ground_z + BRIDGE_DECK_HEIGHT / 2 {
            ListLayer::Bridge
        } else {
            ListLayer::Ground
        }
    }
}
```

`combat_aoe.rs:206-225` `select_object_damage_layer` becomes a thin adapter: build a `CellBridgeView` for the impact cell, call `aoe_object_layer(impact_z, ground_z)`, map `ListLayer` → `MovementLayer`. The current `bridge_height_for_selector` (`:227`, `max(deck-level, 4)`) is REPLACED by the fixed `BRIDGE_DECK_HEIGHT` const — **note:** current code uses `(deck_level - level).max(4)` which can exceed 4; the binary uses the fixed deck height. Confirm at cutover that the impact cells the selector sees never have `deck-level - level > 4` (or this is a DRIFT, not bit-identical). Flag for plan-review.

**Verify:**
```
cargo test -p vera20k combat_aoe
cargo test -p vera20k -- aoe
cargo test -p vera20k global_skirmish_replay
```
Named tests (combat_aoe tests): `aoe_strict_gt_ground_plus_half_deck` (impact_z == ground_z + DECK/2 → Ground, not Bridge — boundary), `aoe_layer_chosen_once_per_detonation`, `aoe_does_not_double_hit_deck_and_under_bridge`.
**Depends on:** P0b (A1 operand), P1c. **Hash:** target set feeds damage → hash; bit-identical ONLY if A1 holds AND the `max(4)` vs fixed-4 question resolves to equality → then **no bump**; otherwise this is a behavior change requiring its own shadow slice. Replay must match baseline.
**Rollback:** revert `combat_aoe.rs` to the inline `:220` form; service `aoe_object_layer` is additive and harmless if unused.

---

## P5 — Occupancy list-layer + `movement_step` move-ordering fix + Clear asymmetry (HASH-RELEVANT — likely SNAPSHOT_VERSION bump) — READY (A2 CLOSED; A1 corrects the const)

**Status:** READY TO IMPLEMENT. A2 is CLOSED — `docs/research/GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md` verifies the two-layer representation, the remove-old/write/add-new ordering, and the Clear-vs-Mark asymmetry from the locomotor body. This stays HASH-RELEVANT (it changes the hashed occupancy representation). **Implementing it flips hashed state → bump `SNAPSHOT_VERSION 17→18` + regenerate the parity baseline.**

**A2 resolved facts now binary-verified (cross-check during P5):**
- **List layer is selected by the occupant's `Object+0x8C` OnBridge byte, sampled at the AddContent/RemoveContent callsite** (`CellClass::AddContent 0x0047E8A0` / `RemoveContent 0x0047EA90` select `FirstObject +0xE4` when byte==0 else `AltObject +0xE8`). Confirms L13: list layer = entity `on_bridge`, NOT loco/path layer.
- **Occupancy BIT layer is a SEPARATE selection** by object Z-height vs ground (+ the `Flags&0x100` gate on Mark): `Mark_Occupation 0x007441B0` sets `AltOccupationFlags +0x128` only when `groundZ + DAT_00B1D0AC <= obj.Z` AND `cell.Flags&0x100`, else `OccupationFlags +0x124`. The two layer selectors (list-by-`on_bridge`, bit-by-Z-height) are INDEPENDENT and are ALLOWED to disagree at ramp boundaries — a verified gamemd behavior, keep them separate.
- **Clear asymmetry (load-bearing):** `Clear_Occupation 0x00744210` clears `0x20` by the Z threshold ALONE — it does NOT re-check `Flags&0x100`. So Mark passes `require_structural=true`, Clear passes `require_structural=false`. Matters for collapse cleanup (the bridge flag may be gone while obj.Z still reflects the deck).
- **Cell-crossing ORDER (Walk body `0x0075C117..0x0075C1AE`):** Mark(0) REMOVE from old cell using the **old** OnBridge → coordinate update → evaluate the transition predicate (`dst.Level == src.Level-4 && dst.Flags&0x100` → set OnBridge=1; `!dst.Flags&0x100 && src.Flags&0x100` → clear OnBridge=0) → Mark(1) ADD to new cell using the **new** OnBridge. Destination bridge flag ALONE does NOT set OnBridge (the exact `-4` level relation is required) — ground→ramp and body→ramp do not change the byte.
- Within a layer, AddContent prepends non-structures, appends `WhatAmI()==6` buildings.

**Edit:** `src/sim/movement/movement_step.rs:1170-1213` (re-order the move), `src/sim/map/bridge_topology.rs` (add `occupancy_bit_layer` with `require_structural`), and the occupancy bit-layer consumer if/when one exists.

**L15 / DRIFT #2 fix** — `process_cell_crossings` currently computes ONE `occupancy_layer` from the post-transition `projected_on_bridge_state` (`:1190`) and uses it for BOTH the remove-from-old and insert-into-new halves of `occupancy.move_entity` (`:1201`). gamemd (A2-verified) removes from old with OLD OnBridge, then inserts into new with UPDATED OnBridge. Re-order to:
1. capture OLD layer (pre-transition `on_bridge`),
2. remove from old cell with OLD layer,
3. evaluate transition / update OnBridge,
4. insert into new cell with NEW layer.

This requires splitting `occupancy.move_entity` (single layer) into remove(old_layer)+add(new_layer), or adding a `move_entity_layered(old_layer, new_layer)`. Read `occupancy.rs` `move_entity` signature before editing (UNCHECKED this run — Task starts with a read).

**L14 / Clear asymmetry** — service exposes the Mark/Clear difference via `require_structural`:
```rust
// bridge_topology.rs — C16. Mark passes require_structural=true; Clear passes false.
impl CellBridgeView {
    pub fn occupancy_bit_layer(&self, obj_z: i32, ground_z: i32, require_structural: bool) -> ListLayer {
        // A2-verified: bridge bit layer iff (groundZ + DAT_00B1D0AC <= obj.Z) [Mark/Clear]
        // AND, ONLY for Mark, the cell is structural (Flags&0x100). Clear passes
        // require_structural=false (it clears by Z alone, no flag re-check).
        let z_on_deck = ground_z + BRIDGE_DECK_HEIGHT <= obj_z; // verified `<=`, threshold DAT_00B1D0AC
        let structural_ok = !require_structural || self.is_bridge_cell();
        if z_on_deck && structural_ok { ListLayer::Bridge } else { ListLayer::Ground }
    }
}
```
**Note A2 (CORRECTED by A1):** the occupancy threshold `DAT_00B1D0AC` is a distinct runtime global from the AoE threshold `DAT_0089E864`, but A1 shows BOTH share the deck-height idiom `= 2 × per_level` (the `×4 then ×0.5` byte shape = `×2`), **NOT `round(per_level×4)`** as previously stated. So `DAT_00B1D0AC` resolves to the full deck height `2 × per_level` (nominally 208 leptons). Use `BRIDGE_DECK_HEIGHT` here only after confirming `DAT_00B1D0AC` and the AoE `DAT_0089E864` resolve to the same integer at cutover (they should, per the shared idiom — A1 §3). The Z test is the FULL deck height `<=` (not `/2` — that halving is AoE-only). The Mark uses `Flags&0x100`; Clear does not (A2 §b). **Domain caution (A1):** this threshold is in LEPTONS in the binary; if the Rust occupancy path operates in level units, convert consistently — do not mix the lepton `2×per_level` with the Level-unit `+4` pathfinding seed.

**Shadow step:** before flipping authority, run the new occupancy ordering alongside the old and assert the hashed occupancy bytes are equal where expected to be (i.e. log every cell where they differ — those are the ramp/transition cells the fix intentionally changes).

**Verify:**
```
cargo test -p vera20k occupancy
cargo test -p vera20k movement_step
cargo test -p vera20k global_skirmish_replay
```
Named tests: `occupancy_list_layer_from_on_bridge_not_loco_layer`, `transition_removes_old_layer_inserts_new_layer`, `clear_occupation_no_structural_flag_required`, plus `ramp_crossing_replay_diff_recorded` (a deliberate, recorded behavior change at ramps).
**Depends on:** P3, P4. **Hash:** **HASH-RELEVANT.** If the hashed occupancy representation changes (Open Q #6 — decide against the actual hash input at this task), bump `SNAPSHOT_VERSION 17 -> 18` in `snapshot.rs:24` with a comment, and regenerate the parity baseline.
**Rollback:** revert `movement_step` ordering + `occupancy_bit_layer` usage; if `SNAPSHOT_VERSION` was bumped, revert `snapshot.rs:24` to 17 and discard the regenerated baseline. Saves made under v18 become unloadable on rollback — acceptable for in-dev branch; note in commit.

---

## P6 — DropIn relayer on collapse (hash-relevant; bump only if repr changes) — UNBLOCKED (rides P5 / A2)

**Status:** UNBLOCKED by A2 (`docs/research/GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md`). The resolution's handoff confirms the collapse contract: on collapse the two layers take different effects — the ground list is killed and the bridge/deck list is relayered DOWN via DropIn. The Clear-by-Z-alone asymmetry (A2 §b — `Clear_Occupation` does not re-check `Flags&0x100`) is exactly why collapse cleanup works after the bridge flag is gone. Depends on P5; rides P5's `SNAPSHOT_VERSION` bump.

**Edit:** `src/sim/world/bridge_orchestrator.rs:1353` (`drop_in_bridge_deck_entities`).

Extend the collapse handler to relayer the persistent `OccupancyGrid` entry (remove from bridge layer, re-add to ground layer) like gamemd `DropIn`, instead of only clearing entity state (DRIFT #3). Use `BridgeTopology::occupancy_bit_layer`/list-layer to compute the post-drop layer.

**Verify:**
```
cargo test -p vera20k bridge_orchestrator
cargo test -p vera20k -- drop_in
cargo test -p vera20k global_skirmish_replay
```
Named tests: `collapse_dropin_relayers_occupancy_to_ground`, `collapse_ground_list_takes_c4_damage_deck_list_drops_in`.
**Depends on:** P5. **Hash:** collapse changes occupancy contents → hash-relevant; bump only if the hashed repr changes (P5 likely already bumped — if so, P6 rides the same version).
**Rollback:** revert `drop_in_bridge_deck_entities` to state-clear-only.

---

## P7 — `BridgeDrawOffset` in render/; retire scattered predicates (RENDER-only, no hash)

**Edit:** `src/app_instances/bridges.rs` (or a new `src/render/bridge_draw_offset.rs`) — implement `BridgeDrawOffset` over the same `CellBridgeView`. `sim/` never imports this trait (invariant #1).

```rust
// render-side only. L18.
pub trait BridgeDrawOffset {
    fn bridge_draw_offset(&self, view: &CellBridgeView, overlay_base_y: i32) -> (i32, i32);
}
```
Fold the `:34-48` constants behind it. **Open assumption A4 (render shadow-DX — this is the plan's internal anchor A4, NOT the now-CLOSED gate "Bridge A4"/CheckBridgeTraversal which is resolved in P3):** keep `BRIDGE_SHADOW_EW_DX` as an OPEN value (in-code comment `:43-44` flags -15 vs -45 unresolved). The -16/-31 (WAE) vs -16/-16 (gamemd) decision is a visual-verify call deferred to this task — do NOT silently change it; surface both and let the user choose.

After consumers are on the service, delete the folded predicate copies in `core.rs` PathCell (keep `PathCell` as a backing view), and route `terrain_cost.rs`/`core.rs` tileset detection (DRIFT #6) through `is_bridge_tileset`/`is_wood_bridge_tileset` (NOT `zone_build.rs`).

**Verify:**
```
cargo check -p vera20k
cargo test -p vera20k bridge_draw_offset
```
Named tests: `bridge_draw_offset_ns_extra_minus15`, `bridge_shadow_shift_ns_x_minus15_y_plus7` (assert against the chosen DX). Visual regression on a temperate high-bridge fixture (manual).
**Depends on:** P1c. **Hash:** none (render-only).

---

## Acceptance tests (deterministic, named)

| Test | Slice | Proves |
|---|---|---|
| `bridge_flags_newtype_matches_const_predicates` | P1b | Single-source flag bits; newtype agrees with `BridgeCellFacts`. |
| `effective_height_anchor_plus4_signed_level` | P1c | L2: signed level + exactly 4 for anchor; NOT layer form. |
| `is_bridge_tileset_distinct_from_structural_flag` | P1c | DRIFT #6: tileset window ≠ structural 0x100, never aliased. |
| `is_wood_bridge_tileset_distinct_from_concrete_and_structural` | P1c | L4: wood window distinct from concrete AND structural. |
| `is_low_bridge_requires_landtype10_and_tube_in_range` | P1c | L5: BOTH conditions. |
| `bridge_topology_predicates_match_pathcell` | P1c | Shadow assert-equal to existing helpers. |
| `bridge_traversal_golden_table_matches_decompile` (+ the 5 branch tests) | P2 | L6-L9: every gate branch incl. both abs==4 orientations, dir==-1 seed, parent-None reconstruct. |
| `bridge_high_fixture_replay_identical_after_gate_relocation` | P3 | Gate relocation is bit-identical (no path/hash drift). |
| `aoe_strict_gt_ground_plus_half_deck` | P4 | L10: STRICT `>` at the boundary (== → Ground). |
| `aoe_layer_chosen_once_per_detonation` | P4 | L11: one selector for the whole CellSpread. |
| `occupancy_list_layer_from_on_bridge_not_loco_layer` | P5 | L13: list layer sourced from `on_bridge`. |
| `transition_removes_old_layer_inserts_new_layer` | P5 | L15 / DRIFT #2: old-layer remove, new-layer insert. |
| `clear_occupation_no_structural_flag_required` | P5 | L14: Clear lacks the 0x100 gate. |
| `collapse_dropin_relayers_occupancy_to_ground` | P6 | DRIFT #3: persistent occupancy relayered on collapse. |
| `bridge_shadow_shift_ns_x_minus15_y_plus7` | P7 | L18 render offset (DX still open per A4). |
| `parity_replay` (Slice-8 harness) | P3,P4,P5,P6 | Determinism preserved at every hash-relevant cutover. |
| `bridge_strength_default_is_1500_from_rulesmd` | P0a/P4 | L19 INI default. |

---

## Rollback notes (hash-flipping tasks)

- **P3** — pure git revert (bit-identical relocation; no save-format change).
- **P4** — revert `combat_aoe.rs` to inline `:220`; service method is dormant if unused. No save change unless P4 was found to be a behavior change (then it should have been a separate shadow slice).
- **P5** — revert `movement_step` ordering + `occupancy_bit_layer`. If `SNAPSHOT_VERSION` was bumped to 18, revert `snapshot.rs:24` and discard the regenerated baseline; v18 saves become unloadable (acceptable on `dev`, note in commit).
- **P6** — revert `drop_in_bridge_deck_entities`; rides P5's version.

---

## What plan-review MUST verify before execution

1. **A1 (highest) — domain RESOLVED, equality STILL OPEN.** A1 (`GATE_BRIDGE_DECK_HEIGHT_RESOLUTION_GHIDRA_REPORT.md`) closed the deck-offset arithmetic (`2×per_level` leptons, NOT `round(src×4)`, NOT `+4`) and the operand domain (binary uses lepton `GetGroundHeight`). Still verify: `cell.level` (Level units) == lepton `GetGroundHeight` on ramp/slope cells, or route GetGroundHeight (P0b). Until proven, P4/P5 parity is UNCHECKED, not bit-identical.
2. **A2 — framing CORRECTED by A1; the `4`-vs-`2` value contradiction is NOW the live check.** AoE `DAT_0089E864` and occupancy `DAT_00B1D0AC` are DISTINCT symbols but BOTH = `2 × per_level` (A1 idiom), NOT `round(per_level×4)`. The prior `BRIDGE_DECK_HEIGHT = PER_LEVEL_HEIGHT*4` placeholder is wrong on BOTH counts: `PER_LEVEL_HEIGHT` does not exist in `src/` (plan-review correction #1), AND the factor is `×2` not `×4`. Decide: in LEPTONS the full deck = `2 × per_level` (208); in LEVEL units = `2`, so half-deck = `1`. The existing Rust `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS = 4` is suspect against this — confirm/correct it against A1's `2×per_level` before P4 cutover (it may be a pre-existing DRIFT independent of this slice).
3. **A3 — STILL OPEN.** `ResolvedTerrainCell` exposes `iso_tile_index`/`land_type`/`tube_index` for the `CellBridgeView` constructor (P1a read pass) — UNCHECKED this run. (Not covered by this run's gate resolutions.)
4. **P4 const drift:** current `bridge_height_for_selector` uses `(deck_level-level).max(4)`; A1 confirms the binary uses a FIXED deck height (`2×per_level`), not a per-cell `max`. Confirm impact cells never make `(deck_level-level).max(4)` diverge from the fixed `2×per_level`, or P4 is a behavior change.
5. **P5 occupancy threshold — A2-CONFIRMED.** L14 uses full deck height `<=` (`groundZ + DAT_00B1D0AC <= obj.Z`, not the `/2` AoE halving) and Mark adds the `Flags&0x100` gate that Clear omits (A2 §b). Still verify `occupancy.move_entity` can be split into remove(old)+add(new) — plan-review correction #6 CONFIRMED this feasible (occupancy.rs:233-246).
6. **P5 hash repr:** decide against the actual hash input whether the occupancy-ordering correction changes the hashed bytes → SNAPSHOT_VERSION 17→18 (A2 confirms the ordering change is real, so expect a bump).
7. **A4 (render shadow-DX, NOT the CLOSED gate Bridge A4):** L18 shadow-DX stays open (-15 vs -45); P7 surfaces, does not settle.
