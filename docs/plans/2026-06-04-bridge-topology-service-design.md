# Bridge-Topology Service Design (Slice 3)

**Status:** DESIGN SPEC (brainstorm output). Doc-only — no `src/` touched this run.
**Date:** 2026-06-04
**Package:** `vera20k` (build/test with `-p vera20k`).
**Rule:** Rust-native structure, gamemd-native semantics. Reproduce the verified observable
contract (formulas to the last decimal, ordering, tie-breaks, defaults, RNG/timer visibility);
do NOT port the C++ CellClass/FootClass trees, raw object-list pointers, or COM vtable plumbing
literally.
**Slots into:** core-engine-substrate program, **map/cell substrate workstream #7**
(`docs/plans/2026-05-29-core-engine-substrate-todo.md`). Rollout rhythm mirrors the Mission/Radio
substrate (`docs/plans/2026-06-01-mission-radio-substrate-implementation-plan.md`) and the
FACTORY_HOUSE study.

## Goal

Consolidate the scattered bridge predicates, offsets, height math, traversal gate, and AoE
layer-selection — currently duplicated across at least five per-consumer cell views — into one
read-only `BridgeTopology` service in `sim/map/` that owns the gamemd-native bit semantics and
formulas, with render draw-offset reached through a `render/`-facing trait so `sim/` never depends
on `render/`.

## Architecture Context

The bridge-helper family is a set of mostly-stateless predicate/offset primitives that movement,
combat-AoE, pathfinding, occupancy, and render all read. The verified behavior contract (C1–C18) is
in `docs/research/BRIDGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` §5 (pass-2 binary-verified
2026-06-04). Today the Rust port reimplements each primitive at its call site against its own cell
view. Read this run:

- **`src/map/bridge_facts.rs`** — map-load stamp producer. Owns the canonical flag-bit constants
  `BRIDGE_FLAG_ANCHOR_SELF=0x80` / `STRUCTURAL=0x100` / `TRANSITION=0x200` / `DESTROYED_OR_RAMP=0x400`
  / `DIRECTION_ZERO=0x800` / `FORWARD_SIDE=0x1000` / `EXTRA_SIDE=0x10000` (lines 3-9) and the
  `BridgeCellFacts::{has_flag, has_structural_bridge, has_transition_flag, is_anchor_self}` predicates
  (lines 63-79). This is the load-time stamp view, not a runtime cell-substrate accessor.
- **`src/sim/pathfinding/core.rs`** — a second view (`PathCell`) with its own bool fields and the
  predicate accessors `has_structural_bridge`/`has_bridge_marker_0x80`/`has_bridgehead_transition`/
  `bridge_deck_level_if_any`/`is_elevated_bridge_cell`/`effective_cell_z_for_layer` (lines 1484-1516)
  plus `is_at_bridge_level` (line 410). **Crucially it also already contains `check_bridge_traversal`
  (lines 506-592)** — a full binary-shaped gate (see "Primary contradiction" below).
- **`src/sim/combat/combat_aoe.rs`** — `AoELayerContext` (line 35) + `select_object_damage_layer`
  with the impact-Z threshold inline at line 220: `impact_z > cell.level as i32 + bridge_height / 2`
  (strict `>`). `bridge_adjusted_impact_z` (line 45).
- **`src/sim/occupancy.rs`** — `OccupancyGrid::rebuild` (line 118) already selects list layer via
  `entity.occupancy_list_layer()` (line 127), which derives Ground/Bridge from `on_bridge` (study
  §4.1, reviewer-verified). NOT a `locomotor.layer` drift.
- **`src/sim/movement/movement_step.rs`** — `process_cell_crossings` (line 891). Computes a single
  `occupancy_layer` from the **post**-transition `projected_on_bridge_state` (lines 1182-1190) and
  uses it for BOTH halves of the move (DRIFT #2).
- **`src/bridge_re.rs`** — pure, well-tested RE-ported overlay-range classifiers and
  `get_cell_zone_id_bridge_policy_decision` (lines 53-360). A parallel island, unowned.
- **`src/rules/bridge_warheads.rs`** — `[CombatDamage] IonCannonWarhead`/`C4Warhead` name resolution.
  Separate damage-state scope; not folded here.
- **`src/app_instances/bridges.rs`** — render-side body/shadow/railing emission with the draw-offset
  constants `BRIDGE_BODY_Y_OFFSET_STATE_0_TO_8 = -16.0` / `STATE_9_TO_17 = -31.0` (lines 34-37),
  `BRIDGE_SHADOW_EW_DX = -15` / `DY = +7` (lines 45-48). The C18 render branch.

`src/sim/map/` **does not exist yet** (verified: no files under `src/sim/map/**`; `sim/mod.rs` has no
`map` module decl). This slice creates it as the first member of the map/cell-substrate workstream.

### Primary contradiction surfaced this run (resolve before implementation)

The study doc's **DRIFT #4 ("`CheckBridgeTraversal` is not reproduced — MISSING")** is **STALE/WRONG
against current code.** `pathfinding/core.rs::check_bridge_traversal` (lines 506-592) already
implements the binary-shaped gate end-to-end:

- `direction == -1` candidate-only seed (`path_height = candidate.signed_level() + 4`), lines 511-519
  → contract C8.
- parent-`None` reconstruction via `((direction - 4) & 7)` over `NEIGHBORS`, lines 495-503 → C7.
- directed `height == -1` parent seed + bridgehead `0x200` gate, lines 535-544 → C9.
- diff-{0,1,4} ladder with the `force_bridge_list` (`*param_4 = 1`) write on the exit-orientation
  path, lines 552-585 → C10/C11.

So the gate is **present but un-owned** — it lives as a `pub(crate)` free function in pathfinding,
coupled to `PathGrid`/`PathCell`, not exposed as a topology-service method that combat/occupancy can
also reach. The real Slice-3 work for the gate is **relocation + single-ownership**, not a
from-scratch implementation. The "biggest player-visible gap" framing in the study is no longer
accurate; the spec scopes the gate as a fold-in (P3), not a new build. This must be reflected when
`/write-plan` runs — do not write a duplicate gate.

## Impact Analysis

**Touched (read or routed through the new service):** `sim/pathfinding/core.rs` (gate + PathCell
predicates + tileset detection), `sim/pathfinding/cell_entry.rs`, `sim/pathfinding/zone_build.rs::inject_bridge_adjacency`
(bridge-endpoint adjacency consumer), `sim/combat/combat_aoe.rs` (AoE layer), `sim/occupancy.rs`
+ `sim/movement/movement_step.rs` (list-layer ordering), `sim/world/bridge_orchestrator.rs` (DropIn
relayer), `sim/movement/movement_bridge.rs` (boundary transition), `map/bridge_facts.rs` (flag-bit
single source), `bridge_re.rs` (fold in as service internals), `render/*` + `app_instances/bridges.rs`
(implement `BridgeDrawOffset`).

**Depends on this:** every mover's path legality, AoE deck-vs-ground hit selection, occupancy list
membership, and bridge sprite offsets.

**Blast radius / determinism:** Most of this slice is read-helper consolidation (hash-neutral). The
two hash-relevant risks: (1) **P3 traversal-gate relocation** must produce bit-identical
`(result, height_out, force_bridge_list)` to the current pathfinding gate or paths change → replay
divergence; (2) **P5 occupancy list-layer + move ordering** may change the hashed occupancy
representation. The state hash lives in `World::advance_tick`'s final phase; `SNAPSHOT_VERSION`
currently `17` (`src/sim/snapshot.rs:24`). Keep `advance_tick` phase order unchanged — this slice
changes *what a phase reads through*, not the phase sequence.

## Tiny-Detail Ledger (parity constraint set)

Every item below must have a home in the chosen design; carried through to `/write-plan`.

- **L1 — Structural bit is `0x100`; bridgehead `0x200`; anchor `0x80`.** Bit values, not derived.
  [doc §5 C1-C3; bridge_facts.rs:3-5; live `decompile 0x00489280` reads `flags & 0x100`/`& 0x80`]
- **L2 — Effective height = `(i8)level + ((flags>>7)&1)*4`.** Level read is **signed**; anchor adds
  exactly 4. NOT the layer-driven `effective_cell_z_for_layer` form. [doc §5 C4; LIVE `0x00487D50`]
- **L3 — Tileset `IsBridge` window `[g_BridgeSet_TileSetBase, +0x10)`, gated on base != -1.** A
  SEPARATE predicate from structural `0x100`. Conflating them is DRIFT #6. [doc §5 C5; LIVE `0x00486750`]
- **L4 — `IsWoodBridge` window `[g_WoodBridgeSet_TileSetBase, +0x10)`, base != -1.** Distinct from
  concrete-tileset AND from structural. Co-used in zone-graph/path-snap and in `Apply_area_damage`'s
  wood-bridge tile-damage branch. [doc §5 C5 / §P2.1; LIVE `0x00486770`, confirmed in
  `decompile 0x00489280` `IsoTileTypeIndex - g_WoodBridgeSet_TileSetBase`]
- **L5 — Low-bridge/tube cell = `tube_index ∈ [0, g_TubeCount) AND land_type == 10`.** Both
  conditions; `land_type == 10` is the tube LandType. NOT subterranean/tunnel (TS-legacy, skip).
  [doc §5 C6; LIVE `0x00484AB0`]
- **L6 — Parent-`0` reconstruction uses `((dir - 4) & 7)` over the 8-entry direction delta table,
  in CELL units, BEFORE the `dir==-1` branch.** `None` parent != "use mover's current cell".
  [doc §5 C7; current code core.rs:495-503; `g_DirectionOffsets` runtime-init, matches
  `util/direction.rs DIRECTION_DELTAS`]
- **L7 — `dir == -1` candidate-only seed:** if `height==-1 && candidate structural`, set
  `height = (i8)candidate.level + 4`, return OK; no bridgehead/diff checks. [C8; core.rs:511-519]
- **L8 — Directed `height==-1` parent seed:** `height = (i8)parent.level + 4`; then if candidate
  lacks `0x200` return BLOCKED. [C9; core.rs:535-544]
- **L9 — Directed diff ladder. `diff = base - candidate_level`, `base = parent structural ? (i8)parent.level : height`.
  Only `abs(diff) ∈ {0,1,4}` may pass.** [C10; core.rs:552-585]
  - **abs==0** blocked iff `((candidate lacks 0x100) OR (candidate lacks 0x200) OR (parent not bridge))
    AND (height != -1 AND height != candidate.level)` — *either* bit missing, NOT both. [study C10
    reviewer fix]
  - **abs==1**: `diff<1` → require `parent.ramp_byte(+0x11C) != 0`; else `candidate.ramp_byte != 0`.
  - **abs==4** has TWO sub-branches in the actual code (`core.rs:568-583`); the ledger originally
    listed only the second. **DESIGN-REVIEW CORRECTION:**
    - *enter orientation* (`parent.level == candidate.level - 4`): pass iff `path_height == candidate.level
      AND parent has 0x100`; does NOT set `list_byte`.
    - *exit orientation* (`candidate.level == parent.level - 4`): require candidate `0x100 && 0x200`,
      then set `list_byte = 1` (force bridge list) and return OK.
    - any other abs==4 (neither orientation): BLOCKED.
    The relocation (P3) must carry BOTH branches verbatim or paths diverge.
- **L10 — AoE object-layer select, once per detonation, from the impact cell:**
  `use_bridge_list = (impact_cell.flags & 0x100) AND (impact_z > ground_z + DAT_0089E864/2)`.
  Comparison is STRICT `>`. `DAT_0089E864 = round(per_level_height × 4)` = full 4-level deck height
  (engine iso-geometry constant, hardcodable). **VERIFIED LIVE this run:** `decompile 0x00489280` →
  `(local_c8[0x50] & 0x100) != 0 && GetGroundHeight() + DAT_0089e864/2 < param_1[2]` (param_1[2] =
  impact_z). [C12]
  **DESIGN-REVIEW CORRECTION (DRIFT — was "Matches combat_aoe.rs:220"):** the `ground_z` operand
  in the binary is **`CellClass__GetGroundHeight()`**, NOT the raw `Level` field. Rust
  `combat_aoe.rs:220` reads `cell.level as i32` (verified this run). These are equal ONLY if
  `cell.level == GetGroundHeight(cell)` for every cell the selector sees — unproven. Default-to-DRIFT:
  the "Matches combat_aoe.rs:220" claim is downgraded to **UNCHECKED**. `aoe_object_layer` must take a
  `ground_z` that is GetGroundHeight-equivalent, and `/write-plan` must prove `level == GetGroundHeight`
  for the impact cell (or route GetGroundHeight) before P4 authority. Same operand question recurs in
  L14 (Mark/Clear also use `GetGroundHeight`, see correction there).
- **L11 — Same layer selector for EVERY spread cell; not recomputed per affected cell.** The whole
  CellSpread reads the selected list (`+0xE4` ground / `+0xE8` bridge = `local_c8[0x39]`/`[0x3a]`).
  [C13; LIVE 0x00489280]
- **L12 — Collect-then-dispatch.** Targets gathered `{object, distance}` first, then `ReceiveDamage`
  (vtable `+0x16C`) in collected order. `Wall=` does NOT decide object splash. [C14; LIVE 0x00489280]
- **L13 — List-layer selector is OnBridge (`ObjectClass+0x8C`), not the locomotor layer.** [C15;
  occupancy.rs:127 already correct]
- **L14 — Occupancy-bit `Mark`/`Clear` asymmetry:** `Mark_Occupation` writes `+0x128` iff
  `ground_z + Zthresh <= obj.z AND (flags & 0x100)`; `Clear_Occupation` uses the same Z test but does
  NOT require `0x100` to clear the bridge bit. [C16]
  **DESIGN-REVIEW CORRECTION (UNCHECKED → VERIFIED LIVE this run):** re-verified
  `decompile 0x007441B0` (Mark) and `0x00744210` (Clear). Confirmed exactly: bridge bit is value
  `0x20` at offset `+0x128` (ground bit `0x20` at `+0x124`); Z test is
  `GetGroundHeight() + DAT_00b1d0ac <= obj.z` (`obj.z` = `*(param_1+8)`) in BOTH; Mark additionally
  gates on `(*(flags @ +0x140) & 0x100) != 0`, Clear does not. NOTE the `ground_z` operand is
  `CellClass__GetGroundHeight()` (same operand caveat as L10) and threshold const is `DAT_00b1d0ac`
  (a DIFFERENT symbol from the AoE `DAT_0089E864`, though both resolve from the same Z-init formula
  L17). Resolves Open Question #3 and the P0 re-verify item.
- **L15 — Boundary transition order:** remove-from-old (OLD OnBridge) → move coords → evaluate
  transition → update OnBridge → add-to-new (NEW OnBridge). Enter iff `dst.level == src.level - 4 AND
  dst structural`; Exit iff `!dst structural AND src structural`. [C17; current move uses single
  post-transition layer for both halves → DRIFT #2]
- **L16 — DestroyableBridges outer gate = scenario `& 0x8000` (map/SpecialFlag), and tile-damage
  inner gate = warhead `+0x144` (`Wall=`).** When `0x8000` off, NO bridge tile damage for ANY warhead.
  **VERIFIED LIVE this run:** `decompile 0x00489280` → `(*g_ScenarioClass_Instance & 0x8000) == 0 ||
  *(param_4 + 0x144) == 0` early-out. BridgeStrength denominator = `Rules+0x1740` =
  `Random__RandomRanged(1, ...)` (LIVE). Tile-damage scope, NOT pure-predicate; gate exposed read-only.
- **L17 — Bridge Z-init = `round(src * 4)`** via `ftol((double)(src*4) + 0.5)`, FPU truncate-toward-zero
  with the `+0.5` constant (`0x007E1738` = IEEE 0.5). Same value for the three thresholds
  (`DAT_0089E864`/`B1D0AC`/`AC13BC`). Hardcodable resolved integer. [doc gate #1 / §2.2]
- **L18 — Draw offset (RENDER):** `0x80` cells get `Y -= 16`; state ∈ [9,0x11] further `-= 15`; NS
  bridges (state 9-17) also shadow-shift (X-15, Y+7). gamemd uses -16/-16; Rust currently -16/-31 (WAE,
  app_instances/bridges.rs:34-37) — a documented deliberate divergence. [C18; DOC-only, not re-verified
  this run — P7 render decision]
  **DESIGN-REVIEW NOTE:** verified `app_instances/bridges.rs:34-37` (-16.0 / -31.0) and the shadow
  consts (DX=-15 @ line 45, DY=+7 @ line 48). The DX=-15 is stated as fact here but the in-code comment
  (`bridges.rs:43-44`) flags it as **unresolved between -15 and -45** (RE doc §10 open Q2). Carry the
  shadow-DX as an OPEN value into P7, not a settled -15.
- **L19 — `BridgeStrength` retail default `1500`** (`ini/rulesmd.ini:816`; struct default when absent
  is 100). [study §2.5; reviewer-verified `ini/rulesmd.ini:816`]

## Chosen Approach

**Approach A — `BridgeTopology` trait over a borrowed `CellBridgeView`, in `sim/map/`, with a
separate `render/`-facing `BridgeDrawOffset` trait.** (Recommended.)

A read-only service in a new `src/sim/map/bridge_topology.rs`. It owns: the gamemd-native flag bit
semantics, signed effective-height (L2), the seven predicates (L1, L3, L4, L5 + structural/bridgehead/
anchor), the binary-shaped traversal gate (L6-L9), the AoE object-layer selector (L10-L12), the
occupancy-bit-layer selector with the `Mark`/`Clear` asymmetry exposed via a `require_structural` flag
(L14), and the boundary transition predicate (L15). It reads cell facts through a borrowed
`CellBridgeView` — it does NOT introduce a fifth owned cell store; it is the single accessor the four
existing views collapse into over the migration. `bridge_re.rs` folds in as private impl detail
(re-exported through the service). Render offset (L18) is a separate `BridgeDrawOffset` trait
implemented in `render/`/`app_instances/` over the same view, so `sim/` never gains a `render/` dep
(invariant #1). All gate/selector math is integer/`i8`-signed — no f32/f64 (the f32/f64 boundary stays
at INI parse only).

**Why A:** It matches the FACTORY_HOUSE / Mission-Radio substrate pattern (one owner, borrowed views,
shadow-first rollout). It keeps the canonical flag constants in `bridge_facts.rs` as the single source
and routes every predicate through them. It honors the layering invariant via the split trait. It
treats the existing pathfinding gate as a fold-in, not a rewrite (the primary contradiction).

**Tiny-detail coverage (A):** L1/L3/L4 → `BridgeFlags` bitflags + tileset-window methods (the two
tileset predicates are distinct methods, never aliased to structural). L2 → `effective_height` uses
signed `i8` level + `(anchor?4:0)`, NOT the layer form. L5 → `is_low_bridge_cell` checks both
`tube_index` range and `land_type==10`. L6-L9 → `check_bridge_traversal` relocated verbatim from
core.rs:506-592 (already correct), with `direction:i32` (-1 allowed), `height_io:&mut i32`,
`list_byte_io:&mut bool`, `parent:Option<Cell>`. L10-L12 → `aoe_object_layer` folds the strict-`>`
threshold (combat_aoe.rs:220) and the once-per-detonation contract; collect-then-dispatch stays in the
combat caller. L13 → list layer stays sourced from `on_bridge` (no change). L14 → `occupancy_bit_layer`
takes `require_structural: bool` so `Mark` passes `true`, `Clear` passes `false`. L15 → `bridge_transition`
folds `compute_bridge_transition`; the move-ordering fix is the caller's (movement_step) responsibility,
service exposes the predicate. L16/L17/L19 → `destroyable_bridges_enabled(scenario)` read-only gate +
named const `BRIDGE_DECK_HEIGHT = round(per_level*4)` + `BRIDGE_STRENGTH_DEFAULT`/INI read. L18 →
`BridgeDrawOffset` in render trait, NOT in `sim/`.

**Approach B — Free functions in `sim/map/bridge_topology.rs` taking explicit field args, no trait.**
Simpler, no trait-object indirection. Rejected: it does not give a single injectable owner that
combat/occupancy/pathfinding can all hold the same way, re-scatters call-site plumbing, and makes
shadow-mode (run-old-and-new-assert-equal) harder to wire uniformly. The trait is the cleaner shadow
seam.

**Approach C — Extend `PathCell` to be THE bridge cell view and have everyone import pathfinding.**
Rejected hard: it inverts layering (combat/occupancy would depend on pathfinding internals), entrenches
the gate's coupling to `PathGrid`, and violates the "one owner in the map/cell substrate" intent. This
is the "bolt it on / hidden coupling" anti-pattern.

## Design

### Components

- `src/sim/map/mod.rs` (new) + `src/sim/map/bridge_topology.rs` (new). Add `pub mod map;` to
  `sim/mod.rs`. First member of the map/cell-substrate workstream.
- `BridgeFlags` bitflags (re-using the `bridge_facts.rs` bit VALUES as the single source — either
  re-export the consts or define `BridgeFlags` in `bridge_facts.rs` and import; one source, not two).
- `CellBridgeView<'a>` — borrowed read view: `{ level: i8, flags: BridgeFlags, ramp_byte: i8,
  iso_tile_index: i32, tube_index: Option<i16>, land_type: u8, state_byte: u8 }`.
- `BridgeTopology` trait (sim-side). `BridgeDrawOffset` trait (render-side, separate file in render/).

### Interfaces / Contracts (signature sketch — illustrative)

```rust
pub trait BridgeTopology {
    fn is_bridge_cell(&self, c: Cell) -> bool;          // L1: flags & STRUCTURAL(0x100)
    fn is_bridgehead(&self, c: Cell) -> bool;           // L1: flags & BRIDGEHEAD(0x200)
    fn is_anchor(&self, c: Cell) -> bool;               // L1: flags & ANCHOR(0x80)
    fn effective_height(&self, c: Cell) -> i32;         // L2: (i8)level + (anchor?4:0)
    fn is_bridge_tileset(&self, c: Cell) -> bool;       // L3
    fn is_wood_bridge_tileset(&self, c: Cell) -> bool;  // L4
    fn is_low_bridge_cell(&self, c: Cell) -> bool;      // L5

    // L6-L9. DESIGN-REVIEW CORRECTION: the existing gate (core.rs:506-592) takes a
    // `BridgeTraversalInput<'_>` struct and RETURNS `BridgeTraversalResult { allowed, path_height,
    // force_bridge_list }` — it does NOT use &mut out-params, and `direction` is `i8` (-1 allowed),
    // not `i32`. The illustrative `&mut i32`/`i32` sketch below is NOT the current shape; relocate the
    // real input/result structs verbatim (P3 must be bit-identical) rather than reshaping the API.
    fn check_bridge_traversal(                          // L6-L9
        &self, input: BridgeTraversalInput<'_>,        // { candidate, candidate_coord, direction: i8,
    ) -> BridgeTraversalResult;                         //   path_height: i32, parent: Option<..> }
                                                        // -> { allowed, path_height, force_bridge_list }

    fn aoe_object_layer(&self, impact: Cell, impact_z: i32, ground_z: i32) -> ListLayer; // L10-L11
    fn occupancy_bit_layer(&self, c: Cell, obj_z: i32, ground_z: i32,
                           require_structural: bool) -> ListLayer;                       // L14
    fn bridge_transition(&self, src: Cell, dst: Cell) -> BridgeTransition;               // L15
    fn destroyable_bridges_enabled(&self) -> bool;      // L16: scenario & 0x8000
}

// render/ only — sim/ never sees this trait.
pub trait BridgeDrawOffset { fn bridge_draw_offset(&self, c: Cell, base_y: i32) -> (i32, i32); } // L18
```

`BRIDGE_DECK_HEIGHT: i32` named const = `round(per_level_height * 4)` (L17), used by `aoe_object_layer`
and `occupancy_bit_layer`. `BRIDGE_STRENGTH_DEFAULT: i32 = 1500` (L19) lives with the damage path, not
the predicate service.

### Data Flow

Map load stamps `bridge_facts.rs` → cell substrate. At runtime, pathfinding/combat/occupancy/movement
construct a `CellBridgeView` from the substrate per cell and call the trait. Render constructs the same
view and calls `BridgeDrawOffset`. No system reads another's private bridge bools.

### Error Handling

Predicates are total (return bool/`ListLayer`); out-of-range cells resolve to "no bridge" at the view
constructor, matching gamemd's null-cell → not-bridge behavior. No `Result` in the hot path.

### Testing Strategy

Per-slice, shadow-first. Each predicate gets an assert-equal-to-current-helper test over three fixture
maps (P1). The gate gets a golden table derived from `decompile 0x004D9C60` covering each L6-L9 branch
(parent-None reconstruct, dir==-1 seed, diff 0/1/4, exit list-byte) PLUS an assert-equal against the
existing core.rs gate (P3) — relocation must be bit-identical. AoE: `aoe_strict_gt_ground_plus_half_deck`,
`layer_chosen_once_per_detonation` (P4). Occupancy: `list_layer_from_on_bridge`,
`transition_removes_old_inserts_new`, `clear_occupation_no_structural_required` + a ramp-crossing replay
diff (P5). A deterministic replay over a high-bridge fixture must match the recorded baseline at every
authoritative cutover.

## Shadow-First Rollout Shape

Mirror the Mission/Radio rhythm: additive → shadow (run new alongside old, assert-equal, log
disagreements, change nothing) → invert/authoritative → retire old copy → bump `SNAPSHOT_VERSION` only
if a hashed field changes → parity harness.

| Slice | Content | Hash class | SNAPSHOT_VERSION |
|---|---|---|---|
| **P0** | Research gate: `BridgeStrength=1500` confirmed (`rulesmd.ini:816`, verified this run); L14 `Clear`/`Mark` `0x100`-asymmetry **VERIFIED LIVE this run** (`0x007441B0` Mark / `0x00744210` Clear). **Remaining P0 item:** prove `cell.level == GetGroundHeight()` for the AoE/occupancy operand (L10/L14 correction) OR plan to route GetGroundHeight. | n/a | no |
| **P1** | `BridgeTopology` + 7 predicates over `CellBridgeView`. Shadow assert-equal to `PathCell`/`BridgeCellFacts`. **Add tileset + wood-tileset distinct methods (DRIFT #6).** | READ-ONLY | no |
| **P2** | Gate shadow: run relocated gate alongside pathfinding, log disagreements (must be zero — same code). | READ-ONLY | no |
| **P3** | Gate authoritative: route `cell_entry.rs` + A*/runtime through the service gate; retire the core.rs copy. Preserve explicit-parent (A*) vs null-parent (runtime) caller distinction. | hash-relevant (paths) — but bit-identical relocation, so **no behavior change** | no (verify replay-identical) |
| **P4** | AoE layer authoritative: fold strict-`>` threshold into `aoe_object_layer`; cut combat callers. | hash-relevant via target set; verify replay | no if bit-identical |
| **P5** | Occupancy list-layer + `movement_step` move-ordering fix (L15) + `Clear` asymmetry (L14). | **HASH-RELEVANT** (occupancy contents) | **BUMP if hashed occupancy repr changes** |
| **P6** | DropIn relayer on collapse (extend `bridge_orchestrator`). | hash-relevant on collapse; verify | bump only if repr changes |
| **P7** | `BridgeDrawOffset` in render/; retire scattered predicate copies (§7). Decide -16/-16 vs -16/-31 (visual verify). | RENDER-only | no |

**Read-only (no hash):** P0, P1, P2, P7. **Hash-relevant (replay-verify; bump only on repr change):**
P3, P4, P5, P6 — P5 is the most likely `SNAPSHOT_VERSION` bump. Keep `advance_tick` phase order
unchanged throughout.

## Architectural Decisions

- **Follows** the FACTORY_HOUSE / Mission-Radio substrate pattern: one owner, borrowed read views,
  shadow-first cutover, hash-bump gated on repr change.
- **Follows** invariant #1 via the split `BridgeTopology` (sim) / `BridgeDrawOffset` (render) traits.
- **Deviation noted:** the traversal gate is RELOCATED from `pathfinding/core.rs`, not built new —
  because it already exists and is correct (contradicts study DRIFT #4). `/write-plan` must not author a
  second gate.
- **Single source of flag bit values:** `bridge_facts.rs` consts; `BridgeFlags` wraps them. No third
  copy.
- **No tech debt introduced** beyond the temporary shadow-mode duplication, which each authoritative
  slice removes.

## Ad-hoc Rust to retire (file:symbol)

- `sim/pathfinding/core.rs::PathCell::{has_structural_bridge, has_bridge_marker_0x80,
  has_bridgehead_transition, bridge_deck_level_if_any, effective_cell_z_for_layer, is_elevated_bridge_cell}`
  + `is_at_bridge_level` (line 410) → FOLD/route through service (keep `PathCell` as a backing view).
- `sim/pathfinding/core.rs::check_bridge_traversal` (lines 506-592) → **RELOCATE** to the service
  (sole owner). [primary contradiction — relocate, do not rewrite]
- `map/bridge_facts.rs::BridgeCellFacts::{has_flag, has_structural_bridge, has_transition_flag,
  is_anchor_self}` → KEEP as stamp producer; route reads through the shared `BridgeFlags` consts.
- `sim/combat/combat_aoe.rs::AoELayerContext` + `select_object_damage_layer` threshold (line 220) →
  FOLD threshold into `aoe_object_layer`; keep context plumbing.
- `sim/movement/movement_step.rs::process_cell_crossings` move-ordering (lines 1142-1207) → RE-ORDER
  to old-layer-remove / new-layer-insert (L15 / DRIFT #2).
- `sim/world/bridge_orchestrator.rs::drop_in_bridge_deck_entities` → EXTEND to relayer persistent
  occupancy (DropIn).
- `sim/pathfinding/cell_entry.rs` (single `target_layer`) → route bridge portion through the service gate.
- `sim/pathfinding/zone_build.rs::inject_bridge_adjacency` → **DESIGN-REVIEW CORRECTION:** this
  function operates over `bridge_records` (BridgeEndpointRecord endpoint adjacency, `zone_build.rs:611`,
  verified this run), NOT tileset detection — the original "DRIFT #6 tileset consumer" citation was
  WRONG. The actual `is_bridge_tileset`/`is_wood_bridge_tileset` (DRIFT #6) consumers live in
  `sim/pathfinding/terrain_cost.rs` and `sim/pathfinding/core.rs` (grep `tileset`/`wood_bridge` this
  run found them there, not in zone_build). `/write-plan` must re-locate the DRIFT #6 routing to those
  two files. `inject_bridge_adjacency` may still route its endpoint legality through the service gate,
  but it is NOT the tileset consumer.
- `bridge_re.rs` overlay classifiers + `get_cell_zone_id_bridge_policy_decision` → KEEP, re-home as
  private service internals.
- `sim/occupancy.rs::OccupancyGrid::rebuild` → NO ACTION (already `on_bridge`-sourced; study DRIFT #1
  retracted).

## Known open port bug (NOTE only — do NOT fix in this slice)

**SEAL/Tanya C4 on CABHUT does nothing.** gamemd has NO Immune gate; the bug is port-side (project
memory `project_c4_bridge_hut_followup`; the 2026-05-12 Ghidra investigation refuted the Immune-gate
hypothesis). It is a CABHUT-action-routing bug adjacent to but not part of the pure-helper family.
The consolidated service makes a later fix EASIER (one place exposes CABHUT/bridgehead cells correctly)
but must not entrench the bug. List as a follow-up, do not bundle.

## Open Questions / Assumptions

1. **Gate relocation vs study DRIFT #4 (highest).** Confirm the intended Slice-3 scope: relocate the
   existing-and-correct `core.rs:506-592` gate into the service (recommended), OR re-derive from the
   binary as a fresh build? They should be identical, but the study assumes "MISSING" — review must
   pick relocation to avoid authoring a duplicate.
2. **`BridgeFlags` home.** Define the bitflags in `bridge_facts.rs` (load-time owner) and import into
   the service, or define in the service and have `bridge_facts.rs` import? Either is fine; pick one to
   keep a single source of bit values.
3. ~~**L14 asymmetry not re-verified live.**~~ **RESOLVED (design-review):** re-verified live this run
   (`0x007441B0` Mark / `0x00744210` Clear). Asymmetry confirmed; see L14 correction. New residual:
   the `ground_z` operand is `GetGroundHeight()` (not raw Level) — see Open Question #7.
4. **`CellBridgeView` construction site.** Where does the substrate expose `level/flags/ramp_byte/
   iso_tile_index/tube_index/land_type/state_byte` as one borrowable struct today? The four current
   views hold subsets; the view constructor must read from whichever store is canonical post-map-load.
   Needs a read pass on the cell substrate during `/write-plan`.
5. **L18 render value.** Keep -16/-31 (WAE) for P7 or switch to gamemd -16/-16? Deferred to a visual
   verify at P7; not blocking the sim-side slices.
6. **Hash-repr at P5.** Does the hashed occupancy representation change when list-layer ordering is
   corrected? If yes, `SNAPSHOT_VERSION` 17→18 + parity baseline regen. Decide at P5 against the actual
   hash input.
7. **`ground_z` operand = `GetGroundHeight()`, not raw `Level` (NEW, design-review).** Both the AoE
   selector (L10) and the occupancy Mark/Clear Z test (L14) use `CellClass__GetGroundHeight()` as the
   floor, but the current Rust AoE path (`combat_aoe.rs:220`) compares against `cell.level`. `/write-plan`
   must either prove `cell.level == GetGroundHeight(cell)` across all impact/occupant cells (including
   ramp/sloped cells where they may differ) or have the service take a GetGroundHeight-equivalent
   `ground_z`. Until proven, treat AoE-layer parity (P4) and occupancy Z (P5) as **UNCHECKED**, not
   "bit-identical relocation."

## Design-review corrections (2026-06-04)

Adversarial review of this spec against current `src/` and live Ghidra. Verdict: **YELLOW** — sound
architecture and rollout shape, but several load-bearing claims needed correction before `/write-plan`.

Verified-correct (no change): flag-bit consts `bridge_facts.rs:3-9`; gate present at `core.rs:506-592`;
L13 list-layer is `on_bridge`-sourced (`game_entity.rs:757`, not locomotor.layer — DRIFT #1 retraction
is sound); AoE strict-`>` at `combat_aoe.rs:220`; `BridgeStrength=1500` at `rulesmd.ini:816`; AoE binary
contract (L11 `+0xE4`/`+0xE8`, L12 vtable `+0x16C`, L16 `&0x8000`+`+0x144` gates, `Random__RandomRanged(1,
Rules+0x1740)` denom) all confirmed live in `0x00489280`; `sim/map/` does not yet exist; all retire-target
symbols grep-real.

Corrections applied inline:
1. **L10 (DRIFT, was "Matches combat_aoe.rs:220"):** binary `ground_z` is `GetGroundHeight()`, Rust uses
   `cell.level` — downgraded to UNCHECKED; see Open Q #7.
2. **L9 abs==4 incomplete:** code (`core.rs:568-583`) has BOTH enter- and exit-orientation sub-branches;
   ledger originally listed only exit. Added the enter branch + "neither → BLOCKED".
3. **DRIFT #6 mis-citation:** `zone_build.rs::inject_bridge_adjacency` operates over endpoint records,
   NOT tilesets; tileset detection lives in `terrain_cost.rs`/`core.rs`. Retarget the DRIFT #6 routing.
4. **L14 (UNCHECKED → VERIFIED LIVE):** re-verified `0x007441B0`/`0x00744210`; asymmetry confirmed
   (bit `0x20` @ `+0x128`, threshold `DAT_00b1d0ac`). Resolves Open Q #3 and the P0 item.
5. **Gate signature sketch:** real gate uses `BridgeTraversalInput`/`BridgeTraversalResult` with
   `direction: i8`, no `&mut` out-params — sketch corrected so P3 relocates verbatim.
6. **L18 shadow-DX:** -15 is NOT settled (in-code comment flags -15 vs -45 unresolved); carry as open.
