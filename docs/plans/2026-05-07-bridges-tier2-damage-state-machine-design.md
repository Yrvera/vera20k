# Bridges Tier 2 — Damage Gating + State Machine Design

## Goal

Land full gamemd.exe parity for bridge damage in one feature: `Apply_area_damage`
gating (SpecialFlags 0x8000 + warhead.Wall + IonCannonWarhead bypass +
BridgeStrength RNG + 3-retry loop), the 18-state per-cell two-axis damage state
machine (body cells + bridgehead cells, both High and Low), `BlowUpBridge`
ground-occupant kill via C4Warhead force_kill, MetallicDebris spawn, anchor-span
granularity (4–5 cells per destruction unit), `UpdateAdjacentBridges` rim
re-evaluation, and the zone-refresh hook. Single landing on `dev`. No
intermediate parity gap.

## Architecture Context

**Bridge state today** ([src/sim/bridge_state.rs](../../src/sim/bridge_state.rs)):

- `BridgeRuntimeCell` carries `deck_present`, `destroyed`, `destroyable`,
  `deck_level`, `bridge_group_id`. No damage-state byte, no axis, no role.
- HP is per-group (`group_hitpoints: BTreeMap<u16, u16>`). Single `apply_damage`
  subtracts and on `<= 0` marks all group cells destroyed atomically.
- Groups are BFS components over `has_bridge_deck`. One long bridge = one group.
- `BridgeEndpointRecord` already extracts ground-zone endpoint pairs for zone
  connectivity (single-pair-per-group approximation of `MapClass+0x54
  BridgeRecord`).

**Damage flow today**:

```
Combat (warhead.wall && damage > 0 && !cell_has_wall_overlay):
  → push BridgeDamageEvent { rx, ry, damage }   (NO gate)

World::advance_tick:
  → apply_bridge_damage_events(events)
    → per event: bridge_state.apply_damage(event)
      → group HP -= damage; if <= 0 → all group cells destroyed
  → resolve_bridge_state_changes(changes)
    → spawn_bridge_explosions (BUGGY: spawns 1 immediate + 50%-delayed BridgeExplosion;
                              binary actually spawns 50% MetallicDebris no-delay
                              + 1 always-delayed BridgeExplosion — see ledger #46-49)
    → snap on_bridge entities to ground OR despawn
    → does NOT walk +0xE4 ground occupants
    → does NOT spawn MetallicDebris
```

**Pure helpers awaiting consumers** ([src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs)):

- `low_bridge_overlay_damage_step_ra2` — RA2 RNG gate + AtomDamage bypass +
  pattern transitions for low-bridge overlay triple. Only called from tests.
- `low_bridge_connected_section_selector_yr` — band/anchor selector. Only
  called from tests.
- `decode_zone_connection_record`, `zone_connection_matches_cell`,
  `get_cell_zone_id_bridge_policy_decision` — zone helpers. Untouched here.

**Combat emit sites** ([src/sim/combat/mod.rs](../../src/sim/combat/mod.rs)):
three sites at lines 669–680, 1315–1323, 1344–1352. All gate only on
`warhead.wall && weapon.damage > 0` and emit raw `BridgeDamageEvent`.

**Stale duplicate**: [src/bridge_re.rs](../../src/bridge_re.rs) at the crate
root mirrors `bridge_specs.rs`. Out of scope here; flagged for cleanup.

## Impact Analysis

**Files touched:**

| File | Change |
|---|---|
| [src/sim/bridge_state.rs](../../src/sim/bridge_state.rs) | Major rewrite. `BridgeRuntimeCell` extends with `damage_state: DamageState`, `axis: Option<Axis>`, `role: BridgeCellRole`, `anchor_span_id: Option<u16>`. New `AnchorSpan` struct + `BTreeMap<u16, AnchorSpan>` registry. New constructor that runs the anchor walker. `apply_damage` replaced by `apply_area_damage(rx, ry, damage, ctx, rng) -> BridgeDamageOutcome`. Per-cell HP retired. |
| [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs) | Extend with body-cell + bridgehead state-machine drivers for both High and Low ranges. Add 16-entry overlay neighbor lookup table. Add `apply_ramp_transition(span, slot, axis, phase, damage_set)`. The existing `low_bridge_overlay_damage_step_ra2` and `low_bridge_connected_section_selector_yr` get wired through the new state machine. |
| [src/sim/combat/mod.rs](../../src/sim/combat/mod.rs#L669) | Replace 3 emit sites. `BridgeDamageEvent` extends with `warhead_ref: WarheadId`, `is_ion_cannon: bool`. Combat does NOT do the RNG gate (must happen at world boundary so RNG order matches binary's `Apply_area_damage`). |
| [src/sim/world/mod.rs](../../src/sim/world/mod.rs#L673) | `apply_bridge_damage_events` becomes the gate + dispatch orchestrator. `resolve_bridge_state_changes` extends to walk +0xE4 ground occupants and kill via C4Warhead force_kill, run `UpdateAdjacentBridges` rim re-eval, run `InvalidateBridgeZones` + zone refresh hook. **Rewrites** existing `spawn_bridge_explosions` at [world/mod.rs:851-919](../../src/sim/world/mod.rs#L851-L919) into `spawn_bridge_debris(destroyed_cells)` matching binary's structure: 95% outer gate → 2 jitter draws → 50%-MetallicDebris (no delay) → 1-always-BridgeExplosion (delay 1–5). New helper `kill_ground_occupants_at(rx, ry, c4_warhead)` for the ground-list traversal. |
| [src/rules/ruleset.rs](../../src/rules/ruleset.rs) | Pre-resolve `IonCannonWarhead` and `C4Warhead` refs at rules-build time as interned `WarheadId`s on a new `RuleSet.bridge_warheads` sub-struct (existing `CombatDamageDefaults` struct in `src/rules/combat_damage.rs` is scoped to particle-system slots only — wrong target). Parse `[General] MetallicDebris=` list (Tier 1 deferred). |
| [src/sim/rng.rs](../../src/sim/rng.rs) | Add `SimRng::next_range_u32_inclusive(low: u32, high: u32) -> u32` mirroring binary's `RandomRanged(low, high)` (both inclusive). Used at all bridge gate + retry sites. |
| [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs) | No new fields. Anchor walker reads existing `bridge_layer.direction: BridgeDirection { EastWest, NorthSouth, Low }` to derive `Axis`, and identifies anchor cells from the existing `bridge_layer.overlay_id` matched against the BridgeSet anchor-overlay class (0x18 / 0x19 high, 0xED / 0xEE low). |
| [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) | Hash new `BridgeRuntimeState` fields (per-cell damage state + AnchorSpan registry). |
| Render layer (terrain renderer / `app_instances/`) | Query `BridgeRuntimeState::display_tile(rx, ry, base_tile)` for bridge cells. Build `(base_tile, axis, damage_state) → display_tile` lookup at map load. `ResolvedTerrainGrid` stays immutable. |
| [src/sim/world/world_tests.rs](../../src/sim/world/world_tests.rs#L413) | Existing tests at 413, 455, 500, 539, 578, 617 assert collapse on first hit; rewrite to use IonCannonWarhead inputs (which retain single-shot collapse semantics with retries) or extend to the new progression model. |

**Determinism:**

- New per-tick RNG draws: BridgeStrength gate (1 per dispatch path × up to 4
  paths per damage event); MetallicDebris probability (50%-gated, count-checked)
  + slot index per spawned-cell. Existing BridgeExplosions draws (95% gate,
  slot, second-shot 50% gate, second slot, delay) are preserved.
- Draw count and order MUST match binary. Item 5 in the ledger (IonCannon-only
  retry) means IonCannon paths consume FEWER draws than the audit assumed —
  this is a parity correction, not a bug.
- State hash extends to per-cell `DamageState`, axis, role, anchor membership,
  and the AnchorSpan registry.
- All new fields are `serde::Serialize/Deserialize` for snapshot round-trip.

**Tick ordering:** unchanged at the macro level. Bridge damage events drained
between combat and ore growth (current order). Within
`apply_bridge_damage_events`: gate → per-path RNG draws → retry loop →
state advance OR collapse trigger. Collapse fallout (kill ground occupants →
Limbo bridge occupants → debris) runs in `resolve_bridge_state_changes`,
maintaining binary's two-list traversal order. Zone refresh fires AFTER all
cell mutation, before next tick's pathfinding.

**What might break:**

- Existing world_tests using single-shot collapse against
  `apply_bridge_damage_events` need to either pass IonCannonWarhead-equivalent
  context or be rewritten as state-progression tests.
- Render layer: tile selection for bridge cells now consults
  `BridgeRuntimeState`. Cells with no `BridgeRuntimeCell` entry still render
  from baked terrain (ground cells, water, etc.) — no breakage there.
- Snapshot binary format breaks for in-flight dev saves. No production save
  format yet, so impact is dev-test only.

**Blast radius:** sim core (combat boundary, world orchestrator), map load
(anchor walker, ResolvedTerrainCell new fields), render (display tile lookup),
parser (MetallicDebris). Save format breaks for dev-test snapshots only.

## Chosen Approach

**Approach Alpha — first-class structs, full (a-strict) parity, single
landing.**

- `BridgeRuntimeCell` extends with typed `damage_state: DamageState`,
  `axis: Option<Axis>`, `role: BridgeCellRole`, `anchor_span_id: Option<u16>`.
- `AnchorSpan` is a first-class struct in `BridgeRuntimeState.anchor_spans:
  BTreeMap<u16, AnchorSpan>`. Anchor pattern is data, not flag-bit emergent.
- Anchor walker at map load mirrors `SetBridgeDirection_NESW` step-for-step
  (4–5 cell pattern per anchor, axis from orientation overlay).
- State machine driver lives in `bridge_specs.rs` as pure functions of
  `(BridgeRuntimeCell, AnchorSpan, hit) → StateOutcome`. Tested in isolation.
- Single `apply_ramp_transition(span, slot, axis, phase, damage_set)` with a
  static 16-entry overlay-neighbor lookup table replaces the binary's 16
  parallel `UpdateRamp_*` functions.
- Renderer queries `BridgeRuntimeState::display_tile(rx, ry, base_tile)` and
  picks from a `(base_tile, axis, damage_state) → display_tile` table built at
  map load. `ResolvedTerrainGrid` stays immutable.
- Pre-resolved `IonCannonWarhead` and `C4Warhead` interned `WarheadId`s on
  `RuleSet`. Combat reads them once.
- The gate, RNG draws, and retry loop live at the world boundary in
  `apply_bridge_damage_events` — NOT in combat — so the draw count/order
  matches `Apply_area_damage`.

**Single-landing sequencing.** No Tier 2a/2b split. Reason: the user's stated
priority is parity, and a split introduces a documented temporary parity gap
between landings (no bridgehead damage in 2a-only). Single landing keeps `dev`
at parity throughout. Implementation may proceed via WIP commits on `dev`, but
the feature lands as a coherent set of commits, not in two parity-different
states.

## Tiny-Detail Ledger

The implementation must preserve all of these. Each item has a designated
home in this design.

### Gate + retry (combat boundary → world)

| # | Detail | Source | Home |
|---|---|---|---|
| 1 | Outer gate: `g_ScenarioClass+0 & 0x8000` (DestroyableBridges) AND `warhead+0x144` (Wall=yes); else skip ALL bridge processing | `[GHIDRA 0x4894B0 LAB_00489ed0]` | `World::apply_bridge_damage_events` early-return when `!self.bridge_destroyable_flag` or `!warhead.wall`. The flag is the existing per-map override merged with rules default. |
| 2 | Four dispatch paths per cell: high body/bridgehead, low body/bridgehead, low direct (overlay 0x4A–0x63), high direct (overlay 0xCD–0xE6) | `[GHIDRA 0x4894B0]` | `BridgeRuntimeState::apply_area_damage` evaluates all four paths in order, each independently. |
| 3 | Each path runs an INDEPENDENT BridgeStrength RNG draw — not shared | `[GHIDRA 0x4894B0 verified]` | One `rng.next_range_u32_inclusive(1, strength)` per path that runs. |
| 4 | Per-path gate: `warhead == IonCannonWarhead` short-circuits the RNG draw entirely; else `RandomRanged(1, BridgeStrength) < damage` (strict `<`) | `[GHIDRA 0x4894B0 LAB_0048a0a5]` | `if !ctx.is_ion_cannon { let roll = rng.draw(); if !(roll < damage) { skip path } }` — short-circuit avoids the draw on IonCannon, matching binary RNG order. |
| 5 | Retry loop: ONLY for IonCannonWarhead; up to 3 retries (max 4 total `ApplyDamageToCell` calls); exits on first non-zero return | `[GHIDRA 0x4894B0 verified live — corrects audit]` | `BridgeRuntimeState::apply_damage_to_cell` returns `bool`; orchestrator loops `if ctx.is_ion_cannon { for _ in 0..3 { if applied { break } else { retry } } }`. |
| 6 | Non-IonCannon Wall warhead: 1 attempt, no retry | `[GHIDRA 0x4894B0]` | Same orchestrator without the loop. |
| 7 | RNG draw range `[1, BridgeStrength]` inclusive; comparison `draw < damage` strict | `[GHIDRA 0x4894B0]` | `next_range_u32_inclusive(1, strength)` helper on `World::rng`. |
| 8 | Default `BridgeStrength = 1500` | `[Tier 1 locked]` | Already in `BridgeRules::default()`. |
| 9 | Default `IonCannonWarhead = "IonCannonWH"` | `[ini: rulesmd.ini:874]` | `RuleSet.bridge_warheads.ion_cannon: WarheadId` resolved at world init. |
| 10 | Default `C4Warhead = "Super"` | `[ini: rulesmd.ini:818]` | `RuleSet.bridge_warheads.c4: WarheadId` resolved at world init. |

### State machine — body cells

| # | Detail | Source | Home |
|---|---|---|---|
| 11 | `+0x11E` state byte: 18 values (0–17) | `[doc: HIGH §2]` | `enum DamageState { Healthy { variant: u8 }, Damaged, PartialCollapseA, PartialCollapseB, Destroyed }` × `Axis`. Variant 0–5 = healthy frame jitter. |
| 12 | States 0–5 = NS healthy variants (passability identical, visual only) | `[doc: HIGH §2]` | `Healthy { variant }` with `axis = NS`. Variant set at map placement, never advances. |
| 13 | State 6 = NS damaged; next hit collapses | `[doc: HIGH §3.1]` | `Damaged` with `axis = NS`. |
| 14 | States 7/8 = NS partial-collapse (only via bridgehead cascade) | `[doc: HIGH §3.1]` | `PartialCollapseA` / `PartialCollapseB` with `axis = NS`. Reached only from bridgehead final-step path. |
| 15 | States 9–14 = EW healthy variants | `[doc: HIGH §2]` | Same pattern with `axis = EW`. |
| 16 | State 15 = EW damaged | `[doc: HIGH §3.1]` | Same. |
| 17 | States 16/17 = EW partial-collapse | `[doc: HIGH §3.1]` | Same. |
| 18 | Body-cell branch entry: `flags & 0x100` set, anchor overlay `0x18`/`0x19` (high) or `0xED`/`0xEE` (low) | `[doc: HIGH §4]` | `role == BridgeCellRole::Body \| Anchor` — checked by `BridgeRuntimeState::apply_area_damage` body branch. |
| 19 | Non-anchor body cell follows `BridgeAnchorPtr` (+0x2C) when `flags & 0x80 == 0` | `[doc: HIGH §3.1]` | Resolve via `cell.anchor_span_id` → `anchor_spans[id].anchor_cell`. |
| 20 | Axis derived from state byte (`state > 8` → EW); parity contract `BridgeDirection::EastWest ↔ Axis::EW ↔ state 9–17` (body runs E-W, ramps face N/S) and `BridgeDirection::NorthSouth ↔ Axis::NS ↔ state 0–8` (body runs N-S, ramps face E/W). **Do NOT inherit Ghidra `Walker_NS_High`/`Walker_EW_High` function-name labels — they are swapped vs physical axis (per doc HIGH §7).** Key transitions off overlay range, not function names. | `[doc: HIGH §3.1, §7]` | `cell.axis` is explicit; mapping locked at the `from_resolved_terrain` walker site (reads `bridge_layer.direction`). |
| 21 | Healthy → Damaged: state=6/15, `UpdateRamp_*_DamageA(2 NS / 4 EW)` + `DamageB(6 NS / 0 EW)`, return 0 | `[doc: HIGH §3.1]` | `body_cell_advance_state` driver in `bridge_specs.rs`. Calls `apply_ramp_transition(span, A_slot, axis, Damage, set=true)` × 2. Returns `StateOutcome::Absorbed`. |
| 22 | Damaged → Collapse (state 6): `UpdateRamp_NS_CollapseA(2)` + `CollapseB(6)`, `SetBridgeDirection_NESW(0,0)`, state=0, `IsoTileTypeIndex=-1`, `UpdateAdjacentBridges`, zone refresh, return 1 | `[doc: HIGH §3.1]` | Same driver branch. Calls `apply_ramp_transition(span, A_slot, NS, Collapse, set)` × 2 then `set_bridge_direction(span, dir=0, set=false)` (destruction path) which emits BlowUpBridge calls. Returns `StateOutcome::Collapsed`. |
| 23 | Damaged → Collapse (state 15): EW-axis equivalent | `[doc: HIGH §3.1]` | Same branch with `axis = EW`. |
| 24 | Partial states 7 (NS) / 17 (EW): fire CollapseA only — `UpdateRamp_NS_CollapseA(2)` for state 7, `UpdateRamp_EW_CollapseA(4)` for state 17 | `[doc: HIGH §3.1, verified 2026-05-07]` | Driver branch handles partial-collapse states explicitly. |
| 25 | Partial states 8 (NS) / 16 (EW): fire CollapseB only — `UpdateRamp_NS_CollapseB(6)` for state 8, `UpdateRamp_EW_CollapseB(0)` for state 16 | `[doc: HIGH §3.1, verified 2026-05-07]` | Same. |

### State machine — bridgehead cells

| # | Detail | Source | Home |
|---|---|---|---|
| 26 | Bridgehead branch entry: `flags & 0x100 == 0`, overlay class in bridgehead range | `[doc: HIGH §3.2]` | `role == BridgeCellRole::Bridgehead` checked by bridgehead branch in `apply_area_damage`. |
| 27 | Bridgehead state from overlay class offset (4 values: 0..3) | `[doc: HIGH §3.2]` | `cell.bridgehead_step: u8` (0..3) on `BridgeRuntimeCell` (only meaningful when `role == Bridgehead`). |
| 28 | Walk to anchor reads `CellClass+0x11A` (height byte; **NOT +0x52** as design originally hypothesized). NS branch: while `(height & 1) != 0` early-return (rejects all odd heights 1/3/5/7/...), then walk `+DirectionOffset` until `height == 4`. EW branch: while `4 < height` early-return (rejects heights > 4 only — **different predicate from NS, not symmetric**), then walk `+DirectionOffset` until `height == 2`. Companion field `+0x11B` is the Level byte (used by `SetOverlayAndPropagate(level - 4)` per HIGH §3.2 step 3). | `[verified live 0x576BA0 on 2026-05-07]` | `bridgehead_walk_to_anchor(cell, axis, terrain)` helper in `bridge_specs.rs`. Reads `+0x11A` height field (added to `BridgeRuntimeCell` as `height_byte: u8` or derived from `ResolvedTerrainCell` if available). Per-axis early-return predicate differs — implement explicitly. |
| 29 | Steps 0..2: `SetOverlayAndPropagate(anchor, base+offset+1)` + `UpdateRamp_*_DamageA + DamageB`, return 0 | `[doc: HIGH §3.2]` | `bridgehead_advance_state` driver branch for steps 0–2. |
| 30 | Step 3 (final): `BlowUpBridge × 3` perpendicular cells, `SetOverlayAndPropagate(anchor, base+3+BridgeSet, level-4)`, `UpdateRamp_*_CollapseA + CollapseB`, `UpdateAdjacentBridges × 2`, zone refresh, 10-slot debris loop, return 1 | `[doc: HIGH §3.2]` | Same driver, step-3 branch. Emits `BridgeStateChange` with the 3 perpendicular cells + the anchor span. |

### UpdateRamp_* helpers

| # | Detail | Source | Home |
|---|---|---|---|
| 31 | 16 helpers (8 high + 8 low: NS/EW × DamageA/DamageB/CollapseA/CollapseB). State-indexed: each computes `next_state` from `current_state` per `(axis, phase)`. `_Low` variants share state transitions with `_High`; only the propagated overlay-base constant differs. | `[doc: HIGH §11.1]` | Single `apply_ramp_transition(current_state: u8, axis: Axis, phase: Phase) -> Option<u8>` implemented as a `match` on `(axis, phase, state)`. Tiny: ~16 arms total. Caller writes the returned state byte to the cell; collapse-final case (returning `Some(0)`) signals the caller to also clear bridge-direction flag and set IsoTileTypeIndex = -1. |
| 32 | Two distinct 16-entry next-overlay tables per `ApplyBridgeDestruction_*_High` family (one per axis), indexed by `CheckBridgeNeighbors_*` result — NOT phase-indexed. LOW family has its own pair (decompile gap, Task 11.5b). | `[doc: HIGH §11.2]` | Separate helper: `pick_destruction_overlay(neighbor_check: u8, axis: Axis, is_high_bridge: bool) -> Option<u8>`. Backed by 4 static `[u8; 16]` tables: `DESTRUCTION_OVERLAY_{HIGH,LOW}_{NS,EW}`. Sentinel `0xFF` for unused slots (binary's `-1`). |
| 33 | Each `UpdateRamp_*` writes the new state byte at the target cell (computed by walker), distinct from §11.2 overlay write. | `[doc: HIGH §11.1]` | Driver applies `apply_ramp_transition` to determine new state byte, separately calls `pick_destruction_overlay` for the visual overlay byte, writes both via `BridgeRuntimeState`. |

### SetBridgeDirection_NESW walker

| # | Detail | Source | Home |
|---|---|---|---|
| 34 | Touches up to 6 cells: anchor (cell 1), walk +direction × 3 (cells 2/3/4), walk –direction × 1 (cell 5, computed as `(param_2 - 4) & 7`), optional fixed-offset (cell 6, only when `param_2 == 6`, fetched at `DAT_0089F690 = E direction (+1, 0)`). Compass index `dir` matches binary `g_DirectionOffsets @ 0x89F688`: `0=N(0,-1), 1=NE(+1,-1), 2=E(+1,0), 3=SE(+1,+1), 4=S(0,+1), 5=SW(-1,+1), 6=W(-1,0), 7=NW(-1,-1)`. State machine calls walker with **dir=2 (E) or 6 (W)** for NS-axis collapse and **dir=4 (S) or 0 (N)** for EW-axis collapse. | `[doc: HIGH §11.5, §11.7, verified live 0x47E040]` | `set_bridge_direction(span, dir: Direction, set: bool)` where `Direction` is an enum with discriminants matching binary compass indices. Emits `Vec<(u16, u16)>` cell list with per-cell action flags. |
| 35 | Cell 1 (anchor) flag mask: clear `0xFFFEE07F`, set bits 7\|8\|9\|12\|16 from set, 10 from !set, 11 from !dir | `[doc: HIGH §11.5]` | Per-cell flag-set logic in `set_bridge_direction`. Internally we update `BridgeCellRole`/`is_destroyed`/`is_anchor`/`is_propagated` named bools instead of raw bits — same observable behavior. |
| 36 | Cells 2/3 (walk +direction × 1, ×2): clear `0xFFFEE8FF`, set + clear `0xFFFFF7FF`. Identical between cells 2 and 3 | `[verified live 0x47E040]` | Same. |
| 37 | Cell 4 (walk +direction × 3): flag-only write — `Flags & 0xFFFFEFFF \| (set << 12)`. **NO `field_0x2C` update, NO `+0x11E` reset, NO BlowUpBridge** | `[verified live 0x47E040]` | Cell 4 in the walker emits a `FlagOnly` action; the destruction-path BlowUpBridge skip-list is `{4, 6}`. |
| 38 | Cell 5 (walk –direction): clear `0xFFFFF8FF` then `0xFFFEE7FF`, sets bits 8\|9\|10\|11\|16 | `[verified live 0x47E040]` | Same. |
| 39 | Cell 6 (only `param_2 == 6`): sets `+0x2C = anchor` and bit 16. **NO `+0x11E` reset, NO BlowUpBridge** | `[verified live 0x47E040]` | `set_bridge_direction` only appends cell 6 when direction matches; emits `FlagOnly` action. |
| 40 | Anchor's `field_0x11E` set to 0 (NS) or 9 (EW) based on direction; cells 2/3/5 also reset to 0/9 (intact) or 0 (destruction); cells 4/6 do NOT reset state byte | `[verified live 0x47E040]` | Anchor + cells 2/3/5 reset their `damage_state` to `Healthy { variant: 0 }` with appropriate axis on intact, or to `Destroyed` on destruction. |
| 41 | Destruction path (`set==false`): **exactly 4 BlowUpBridge calls per `SetBridgeDirection_NESW` call** — cells 1, 2, 3, 5. Cell 4 receives flag-only write; cell 6 (when present) sets only `+0x2C` + bit 16 | `[verified live 0x47E040 — corrects design ledger and audit]` | `set_bridge_direction` with `set=false` emits `BridgeCellCollapse` for cells 1/2/3/5 only; cells 4/6 get `FlagOnly`. RNG draw count downstream depends on this — must be exact for state-hash parity. |
| 42 | `BridgeAnchorPtr` (+0x2C) written to anchor on cells 2/3/5/6 (intact path); cleared to 0 on cells 1/2/3/5 (destruction). Cell 4 never touches `+0x2C` | `[verified live 0x47E040]` | `cell.anchor_span_id` set/cleared per per-cell action; cell 4 leaves `anchor_span_id` untouched. |

### BlowUpBridge per-cell

| # | Detail | Source | Home |
|---|---|---|---|
| 43 | Step 1 — kill ground (+0xE4): `ReceiveDamage(coord, damage=0, warhead=C4Warhead, dist=0, force_kill=1, flag=1, source=0)` | `[doc: HIGH §11.4, verified live 0x47DD70]` | `World::kill_ground_occupants_at(rx, ry, c4_warhead)` — iterates entities at cell on ground layer, calls death pipeline with C4Warhead as killing warhead and damage=0. Uses existing death-effects pipeline (which selects `InfDeath` from warhead). |
| 44 | Step 2 — destroy bridge-deck (+0xE8): `vtable+0xEC` (Limbo, no spawn) | `[verified live 0x47DD70]` | Existing `to_despawn` path in `resolve_bridge_state_changes` is the equivalent — silent removal, no death anim. |
| 45 | Step 3 — append cell coord to `DAT_0087F8BC` global ring buffer | `[verified live 0x47DD70]` | `BridgeStateChange.destroyed_cells: Vec<(u16, u16)>` already serves this purpose. |
| 46 | Step 4 outer gate: 95% per cell — `(double)RandomRanged(0, 0x7FFFFFFE) * (1/2^31) < 0.95` (threshold `_DAT_007e4f58 = 0.95`, factor `_DAT_007e3570 = 1/2^31`); 5% of cells skip ALL debris | `[verified live 0x47DD70 + memory read 0x7e4f58]` | `spawn_bridge_debris(destroyed_cells)` new method on World. Outer gate `if rng.next_range_u32(20) != 0` (95% pass). |
| 47 | Step 4 jitter draws: 2 RNG draws consumed for X/Y jitter (results applied to `iStack_c`/`iStack_8` via `Math__ftol`) — must consume to keep RNG order parity | `[verified live 0x47DD70]` | After outer gate passes, draw 2 jitter values from `rng` (apply to spawn coords). |
| 48 | Step 4a — MetallicDebris: 50%-RNG-gated (threshold `_DAT_007e1738 = 0.5`), gated AND on `BridgeVoxelMax > 0` (`Rules+0x14C`); slot `RandomRanged(0, voxel_max - 1)`; **no frame delay**; uses `Rules+0x140` array | `[verified live 0x47DD70 + memory read 0x7e1738]` | `spawn_bridge_debris` 50% inner check via 1-of-2 RNG draw; consults `bridge_rules.voxel_max > 0`; reads from `rules.general.metallic_debris`. |
| 49 | Step 4b — BridgeExplosions: ALWAYS spawned (modulo alloc) when outer gate passes; slot `RandomRanged(0, count - 1)` from `Rules+0x15C/+0x168`; delay `RandomRanged(1, 5)` frames | `[verified live 0x47DD70 — corrects existing Rust drift]` | `spawn_bridge_debris` always emits 1 BridgeExplosion (when outer gate passed). **Rewrites existing `spawn_bridge_explosions` at world/mod.rs:851**: today it spawns 1 immediate + 50%-gated delayed second BridgeExplosion (wrong); must become 50%-MetallicDebris (no delay) + 1-always-BridgeExplosion (delay 1–5). |
| 50 | World coord for spawn: `x = cell_x * 0x100 + 0x80 + jitter`, `y = cell_y * 0x100 + 0x80 + jitter`, `z = (cell.Level) * DAT_0089E7C0 (heightStep) + DAT_0089E7B4 (heightStep_offset)`. Both height constants are runtime-init in binary (zero in static image). Our existing constants: `SHIP_HEIGHT_STEP = 90 leptons/level`, bridge deck = ground + 4 levels. The `0x600` literal in the binary's `AnimClass::Constructor(anim, &coord, 0, 1, 0x600, 0, 0)` call is the **5th constructor arg (anim flags)**, NOT a Z offset — earlier ledger drafts conflated them. | `[verified live 0x47DD70]` | Use `bridge_deck_level_if_any().unwrap_or(level)` cell-level value (existing pattern at `world/mod.rs:866`). Render layer applies `HEIGHT_STEP`. No new lepton constant needed. |
| 51 | RNG draw count per cell that passes outer gate: 1 (outer) + 2 (jitter) + 1 (50% inner) + 1 (MetallicDebris slot, only if 50% passes) + 1 (BridgeExplosion delay) + 1 (BridgeExplosion slot) = **6 or 7 draws** | `[verified live 0x47DD70]` | State-hash parity contract — must match exactly for replay determinism. |
| 52 | BlowUpBridge does NOT modify cell overlay/flags itself | `[verified live 0x47DD70]` | Honored — overlay transitions happen in `apply_ramp_transition`, not in `kill_ground_occupants_at`. |
| 53 | Two-list traversal: ground (+0xE4) FIRST, then bridge-deck (+0xE8), then ring buffer append, then debris | `[verified live 0x47DD70]` | `resolve_bridge_state_changes` does ground-kill loop before despawn loop before debris spawn. |

### Zone refresh

| # | Detail | Source | Home |
|---|---|---|---|
| 54 | `InvalidateBridgeZones` toggles per-record `is_intact` byte | `[doc: HIGH §11.8]` | `BridgeEndpointRecord.active` already serves; flipped per-record on collapse. |
| 55 | `UpdateBridgeZonesHelper` runs full recompute only if a record changed | `[doc: HIGH §11.8]` | `World::refresh_bridge_zones_if_dirty()` called from `resolve_bridge_state_changes` when any record changed. Reuses existing `World::rebuild_zone_grid` at world/mod.rs:614 (incremental fallback already in place via [zone_incremental.rs](../../src/sim/pathfinding/zone_incremental.rs)). |
| 56 | `UpdateAdjacentBridges_*` rim re-evaluation: walks neighbors of changed cell, refreshes flags | `[doc: HIGH §11.9]` | `update_adjacent_bridges(rx, ry)` helper. For each cardinal neighbor that's a bridge cell: re-evaluate role/axis/state from anchor span + current overlay, write back. |

### Anchor span infrastructure

| # | Detail | Source | Home |
|---|---|---|---|
| 57 | `SetBridgeDirection_NESW` runs at MAP LOAD when bridges placed | `[doc: HIGH §11.5]` | `BridgeRuntimeState::from_resolved_terrain` runs the anchor walker. |
| 58 | Anchor cell identified by `flags & 0x80` set | `[doc: HIGH §3.1]` | `cell.role == BridgeCellRole::Anchor`. Detected from anchor-overlay class match on existing `ResolvedTerrainCell.bridge_layer.overlay_id` — no new cell field required (see Architectural Decisions). |
| 59 | Anchor pattern: up to 6 cells (anchor + 3 walked +dir + 1 –dir + optional 1 fixed-offset when `param_2==6`) | `[verified live 0x47E040]` | `AnchorSpan.cells: [Option<(u16, u16)>; 6]` (anchor + 5 slots, slot indices fixed by walker order). |
| 60 | Axis derived from existing `ResolvedTerrainCell.bridge_layer.direction: BridgeDirection { EastWest, NorthSouth, Low }` | `[verified — see resolved_terrain.rs:48]` | `AnchorSpan.axis` set at walker time from `bridge_layer.direction`; no new `bridge_axis` field needed on ResolvedTerrainCell. |

### Render display table

| # | Detail | Source | Home |
|---|---|---|---|
| 61 | Body overlay 0xCD–0xD2 (high) = healthy EW variants (6 frames, map-load-deterministic) | `[doc: HIGH §2]` | `BridgeDisplayTable`: built at map load from `(base_tile, axis, damage_state) → display_tile`. Healthy variants come from baked `ResolvedTerrainCell` (not gameplay-driven). |
| 62 | Body overlay 0xD3–0xD5 (high) = damaged EW (3 variants) | `[doc: HIGH §2]` | Same table, `damage_state == Damaged`. |
| 63 | Body overlay 0xD6–0xDB (high) = healthy NS variants | `[doc: HIGH §2]` | Same. |
| 64 | Body overlay 0xDC–0xDE (high) = damaged NS (3 variants) | `[doc: HIGH §2]` | Same. |
| 65 | Endpoint stubs 0xDF/0xE1 = damaged EW endpoint, 0xE0/0xE2 = destroyed EW; 0xE3/0xE5 = damaged NS, 0xE4/0xE6 = destroyed NS | `[doc: HIGH §2]` | Endpoint stub lookups in display table. |
| 66 | 0xE7 = fully destroyed body EW; 0xE8 = NS | `[doc: HIGH §2]` | `damage_state == Destroyed` → final tile. |
| 67 | Healthy frame variants 0–5/9–14 are MAP-DATA-DRIVEN | `[verified vs binary §11.5]` | Already baked into `ResolvedTerrainCell.final_tile_index`. Display table reads it through. |

### Tick ordering + determinism

| # | Detail | Source | Home |
|---|---|---|---|
| 68 | Per-event flow: gate → per-path RNG draws → retry loop → state advance \| collapse | `[GHIDRA 0x4894B0]` | `BridgeRuntimeState::apply_area_damage` mirror. |
| 69 | Collapse fallout order: ground kill → bridge-deck Limbo → ring-buffer append → debris (outer gate → jitter → MetallicDebris → BridgeExplosion) | `[verified live 0x47DD70]` | `resolve_bridge_state_changes` ordering, with `spawn_bridge_debris` per cell. |
| 70 | Zone refresh AFTER cell mutation, before next tick's pathfinding | `[doc: HIGH §11.8]` | At end of `resolve_bridge_state_changes`. |
| 71 | RNG draws for non-IonCannon Wall warhead: 1 per matching dispatch path + per-cell collapse draws (6 or 7 per collapsed cell) | `[GHIDRA 0x4894B0 + 0x47DD70]` | Confirmed via state-hash test. |
| 72 | RNG draws for IonCannon: 0 BridgeStrength draws (bypassed) + per-cell collapse draws | `[GHIDRA 0x4894B0 + 0x47DD70]` | Confirmed via state-hash test. |
| 73 | State hash includes per-cell DamageState, axis, role, anchor_span_id, bridgehead_step, AnchorSpan registry | `[determinism contract]` | Extends `BridgeRuntimeState::hash_into` (or `world_hash::hash_world`). |

### INI

| # | Detail | Source | Home |
|---|---|---|---|
| 74 | `[General] MetallicDebris=` — 20-entry list, default `DBRIS1LG..DBRS10SM` | `[ini: rulesmd.ini:528]` | `GeneralRules.metallic_debris: Vec<String>` parsed in `ruleset.rs`. |
| 75 | `[CombatDamage] IonCannonWarhead=`, default `IonCannonWH` | `[ini: rulesmd.ini:874]` | `RuleSet.bridge_warheads.ion_cannon: WarheadId` resolved at world init (new sub-struct, see Components). |
| 76 | `[CombatDamage] C4Warhead=`, default `Super` | `[ini: rulesmd.ini:818]` | `RuleSet.bridge_warheads.c4: WarheadId` resolved at world init. |
| 77 | `BridgeVoxelMax` (Tier 1 parsed) gates MetallicDebris count in step 4a | `[verified live 0x47DD70]` | Existing `BridgeRules.voxel_max` read in `spawn_bridge_debris`. |
| 78 | `SimRng::next_range_u32_inclusive(low, high)` mirrors binary `RandomRanged(low, high)` (both inclusive) | `[binary calling convention]` | New helper on `SimRng` (Components). Adopt at all bridge gate + retry sites. |

## Design

### Components

```rust
// src/sim/bridge_state.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Axis { NS, EW }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DamageState {
    Healthy { variant: u8 },     // 0..=5 per axis
    Damaged,                      // state 6 (NS) / 15 (EW)
    PartialCollapseA,             // state 7 (NS) / 17 (EW)
    PartialCollapseB,             // state 8 (NS) / 16 (EW)
    Destroyed,                    // post-collapse
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BridgeCellRole {
    Anchor,         // flag 0x80 set; primary cell of an anchor span
    Body,           // flag 0x100 set, not anchor; follows BridgeAnchorPtr
    Bridgehead,     // flag 0x100 not set, overlay in bridgehead range
    Tail,           // cell 5 of anchor pattern (-direction)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BridgeRuntimeCell {
    pub deck_present: bool,
    pub destroyable: bool,
    pub deck_level: u8,
    pub bridge_group_id: Option<u16>,
    pub damage_state: DamageState,
    pub axis: Option<Axis>,
    pub role: BridgeCellRole,
    pub anchor_span_id: Option<u16>,
    pub bridgehead_step: u8,        // 0..=3, only meaningful when role == Bridgehead
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AnchorSpan {
    pub id: u16,
    pub anchor: (u16, u16),
    pub cells: Vec<(u16, u16)>,    // 4–5 cells in walk order
    pub axis: Axis,
    pub direction: u8,             // 0..=7 compass index
    pub damage_state: DamageState, // mirror of anchor cell's state
    pub bridge_group_id: u16,
}

pub struct BridgeRuntimeState {
    width: u16,
    height: u16,
    cells: Vec<Option<BridgeRuntimeCell>>,
    group_cells: BTreeMap<u16, Vec<(u16, u16)>>,
    strength: u16,
    anchor_spans: BTreeMap<u16, AnchorSpan>,
    endpoint_records: Vec<BridgeEndpointRecord>,
    bridge_destroyable_flag: bool,  // SpecialFlags 0x8000
}
```

```rust
// src/sim/combat/mod.rs (extension)

pub struct BridgeDamageEvent {
    pub rx: u16,
    pub ry: u16,
    pub damage: u16,
    pub warhead_ref: WarheadId,
    pub is_ion_cannon: bool,        // pre-resolved at combat boundary
}

// Combat emits:
if warhead.wall && weapon.damage > 0 && !cell_has_wall_overlay(...) {
    bridge_damage_events.push(BridgeDamageEvent {
        rx: target_rx,
        ry: target_ry,
        damage: weapon.damage as u16,
        warhead_ref: interner.intern(&warhead.id),
        is_ion_cannon: warhead_ref == rules.bridge_warheads.ion_cannon,
    });
}
```

```rust
// src/rules/ruleset.rs (extension)

pub struct RuleSet {
    // ... existing fields ...
    pub bridge_warheads: BridgeWarheads,
    pub general: GeneralRules,        // extended with metallic_debris
}

#[derive(Debug, Clone, Default)]
pub struct BridgeWarheads {
    /// Pre-resolved `[CombatDamage] IonCannonWarhead=` (default "IonCannonWH").
    /// Bypasses BridgeStrength RNG gate; enables 3-retry loop in
    /// Apply_area_damage. Mirrors `Rules+0xFF0`.
    pub ion_cannon: WarheadId,
    /// Pre-resolved `[CombatDamage] C4Warhead=` (default "Super").
    /// Used as killing warhead in BlowUpBridge ground-occupant kill
    /// (force_kill=1, damage=0). Mirrors `Rules+0xFA8`.
    pub c4: WarheadId,
}

pub struct GeneralRules {
    // ... existing ...
    pub metallic_debris: Vec<String>,  // default: 20-entry DBRIS1LG..DBRS10SM
}
```

### Interfaces / Contracts

**`BridgeRuntimeState::from_resolved_terrain_with_anchors`** (replaces existing
constructor):

```rust
pub fn from_resolved_terrain_with_anchors(
    terrain: &ResolvedTerrainGrid,
    bridge_destroyable_flag: bool,
    strength: u16,
) -> Self;
```

Runs the anchor walker over every cell whose `bridge_layer.overlay_id`
matches an anchor-overlay class (high: 0x18 / 0x19; low: 0xED / 0xEE). For
each anchor, derive `Axis` from the cell's existing `bridge_layer.direction`
(`EastWest` → `Axis::EW`, `NorthSouth` → `Axis::NS`), walk the 6-cell pattern
(anchor + 3 walked +dir + 1 –dir + optional fixed-offset for direction 6),
register one `AnchorSpan` per anchor with the cell list and axis. Bridgehead
cells (role == Bridgehead, no anchor membership) get their own
`bridgehead_step` initialized to 0. Body cells get `anchor_span_id` linked to
their containing span.

**`BridgeRuntimeState::apply_area_damage`** (replaces `apply_damage`):

```rust
pub struct BridgeDamageContext<'a> {
    pub damage: u16,
    pub is_ion_cannon: bool,
    pub bridge_strength: u16,
    pub rng: &'a mut LockstepRng,
}

pub fn apply_area_damage(
    &mut self,
    rx: u16,
    ry: u16,
    ctx: &mut BridgeDamageContext,
) -> Vec<BridgeStateChange>;
```

Mirrors `Apply_area_damage` body. Returns one `BridgeStateChange` per dispatch
path that triggered a collapse.

**`BridgeRuntimeState::display_tile`** (renderer query):

```rust
pub fn display_tile(&self, rx: u16, ry: u16, base_tile: u32) -> u32;
```

Looks up `(base_tile, cell.axis, cell.damage_state)` in the
`BridgeDisplayTable` built at map load. Falls through to `base_tile` for
non-bridge cells or bridges in healthy state.

**`World::apply_bridge_damage_events`** (orchestrator, extended):

```rust
pub(crate) fn apply_bridge_damage_events(
    &mut self,
    events: &[BridgeDamageEvent],
) -> Vec<BridgeStateChange>;
```

Per event: outer gate (SpecialFlags 0x8000 + warhead.wall via `is_ion_cannon
\|\| warhead_ref maps to a Wall=yes warhead`), then `apply_area_damage` per
path with retry semantics (only IonCannon retries up to 3 times).

**`World::resolve_bridge_state_changes`** (extended):

Adds, after existing snap/despawn logic:

```rust
fn kill_ground_occupants_at(&mut self, rx: u16, ry: u16);
fn spawn_bridge_debris(&mut self, destroyed_cells: &BTreeSet<(u16, u16)>);
fn update_adjacent_bridges(&mut self, changed_cells: &BTreeSet<(u16, u16)>);
fn refresh_bridge_zones_if_dirty(&mut self, any_record_changed: bool);
```

`kill_ground_occupants_at` walks entities at `(rx, ry)` on ground layer,
applies damage through the standard death pipeline using `c4_warhead` ref —
gives correct InfDeath selection for the kill.

### Data Flow

```
Map load:
  ResolvedTerrainGrid (existing — bridge_layer.direction + bridge_layer.overlay_id)
    → BridgeRuntimeState::from_resolved_terrain_with_anchors
        → anchor walker: for each anchor cell, walk 4–5 pattern
        → register AnchorSpan; mark each cell's role + anchor_span_id
        → bridgehead cells get role + bridgehead_step=0
    → BridgeDisplayTable built from terrain overlay ranges

Tick:
  Combat phase:
    warhead.wall && damage > 0 && !cell_has_wall_overlay
      → emit BridgeDamageEvent {
            rx, ry, damage, warhead_ref,
            is_ion_cannon: warhead_ref == rules.bridge_warheads.ion_cannon
        }

  World::advance_tick (after combat, before ore growth — current order):
    apply_bridge_damage_events(events):
      per event:
        outer gate (SpecialFlags 0x8000 + warhead.Wall) — skip if false
        for each of 4 dispatch paths matching the cell:
          if !is_ion_cannon:
            roll = rng.next_range_u32_inclusive(1, strength)
            if !(roll < damage): continue (skip this path)
          loop 1..=4 attempts (only continues for IonCannon):
            outcome = apply_to_path(rx, ry, path, ctx)
            if outcome != Absorbed: break
          if outcome == Collapsed: collect into changes

    resolve_bridge_state_changes(changes):
      for each destroyed cell:
        kill_ground_occupants_at(rx, ry)        — C4Warhead force_kill
      for each on_bridge entity at destroyed cell:
        snap to ground if walkable, else despawn  — existing logic
      spawn_bridge_debris(destroyed_cells)      — REWRITES existing spawn_bridge_explosions:
                                                  per cell: 95% outer gate → 2 jitter draws →
                                                  50%-MetallicDebris (no delay) →
                                                  1-always-BridgeExplosion (delay 1–5)
      update_adjacent_bridges(changed_cells)    — new rim re-eval
      refresh_bridge_zones_if_dirty()           — new zone hook

Render (per frame):
  for each visible cell:
    if BridgeRuntimeState::cell(rx, ry).is_some():
      tile = BridgeRuntimeState::display_tile(rx, ry, base_tile)
    else:
      tile = base_tile
    draw(tile)
```

### Error Handling

- INI parser: `Option`-returning getters with documented defaults
  (`unwrap_or_default()`). No new failure modes vs Tier 1.
- Anchor walker at map load: if a bridge cell can't be classified into a
  span (e.g., orphan cell), fall back to `role = BridgeCellRole::Body` with
  `anchor_span_id = None`. Log warn, do not panic. Map data on user-supplied
  maps may contain edge cases.
- `apply_area_damage`: cell out of bounds or no `BridgeRuntimeCell` → no-op.
- Render `display_tile`: missing `BridgeRuntimeCell` → return `base_tile`
  unchanged.

### Testing Strategy

**Unit tests** (in `bridge_state.rs` and `bridge_specs.rs`):

1. `state_machine_healthy_to_damaged_first_hit_returns_absorbed` — verify
   state advances 0→6 (or 9→15) and outcome is `Absorbed`.
2. `state_machine_damaged_to_collapsed_second_hit_returns_collapsed` — verify
   state 6→0 (collapse triggers `SetBridgeDirection_NESW`).
3. `state_machine_partial_collapse_a_collapses_remaining_ramp` — for partial
   states 7/16.
4. `state_machine_partial_collapse_b_collapses_remaining_ramp` — for 8/17.
5. `bridgehead_progression_steps_0_through_2_absorb_damage` — verify
   bridgehead step counter advances without collapse.
6. `bridgehead_step_3_triggers_full_collapse_with_3_perpendicular_blowups`.
7. `apply_ramp_transition_writes_correct_overlay_via_table` — for each of
   the 16 transitions × phase × axis × damage_set permutations.
8. `set_bridge_direction_walker_visits_4_cells_plus_optional_5th` — 5-cell
   walk on `param_2 == 0`, 4-cell on others.
9. `set_bridge_direction_destruction_emits_blowup_per_cell`.

**Anchor walker tests**:

10. `anchor_walker_5x1_horizontal_bridge_one_anchor_one_span` — 5-cell wide
    horizontal bridge produces 1 span with 4 walked + 1 tail.
11. `anchor_walker_long_bridge_splits_into_multiple_spans` — bridge of 12
    cells produces 3 anchor spans (per binary's 4-cell-per-anchor pattern).
12. `anchor_walker_axis_detection_from_resolved_terrain_orientation`.

**Gate + retry tests** (in `world_tests.rs`):

13. `gate_skips_processing_when_destroyable_flag_false`.
14. `gate_skips_processing_when_warhead_not_wall`.
15. `gate_passes_when_both_flags_set` — exercises full state advance.
16. `non_ion_cannon_consumes_one_rng_draw_per_path` — RNG counter.
17. `ion_cannon_consumes_zero_rng_draws_for_strength_gate` — RNG counter.
18. `ion_cannon_retries_up_to_3_times_on_apply_failure` — verify 4 attempts.
19. `non_ion_cannon_does_not_retry`.
20. `rng_gate_uses_strict_less_than_with_inclusive_low_bound_1`.

**Collapse fallout tests**:

21. `bridge_collapse_kills_ground_occupants_with_c4_warhead` — entity at
    ground layer under collapsing bridge cell dies, death pipeline fires with
    `c4_warhead` as killing warhead, InfDeath selects from C4Warhead's slot.
22. `bridge_collapse_limbos_bridge_deck_occupants` — existing snap/despawn.
23. `bridge_collapse_spawns_metallic_debris_50_percent_per_cell` — RNG-gated.
24. `bridge_collapse_metallic_debris_skipped_when_voxel_max_zero`.
25. `bridge_collapse_runs_zone_refresh_after_state_mutation`.
26. `bridge_collapse_two_list_traversal_order_ground_then_bridge_deck`.

**Determinism / state hash**:

27. `bridge_runtime_state_round_trips_through_serde` — snapshot test.
28. `same_input_same_rng_yields_same_state_hash` — replay determinism.
29. `bridge_state_hash_includes_per_cell_damage_state_and_anchor_spans`.

**Integration test** (`tests/` directory):

30. End-to-end: map with bridge, fire IonCannon at bridge cell, verify (a)
    state machine advances, (b) collapse triggers, (c) ground occupants die,
    (d) state hash deterministic across runs.

## Architectural Decisions

- **First-class structs over flag-bit faithful storage.** Same playbook as
  `EntityStore = BTreeMap<u64, GameEntity>` from CLAUDE.md: replace the
  storage shape when typed structs improve testability/serializability/
  scalability without changing observable behavior. `AnchorSpan`, `DamageState`,
  `Axis`, `BridgeCellRole` carry the parity contract; the binary's flag-bit
  representation is internal mechanism, not surface.
- **Single `apply_ramp_transition` table-dispatched over 16 parallel functions.**
  Lookup table is a `static [[u8; 16]; 4]` per family. Equivalent to binary's
  16 `UpdateRamp_*` functions, smaller and easier to test. No behavior change.
- **Renderer queries `BridgeRuntimeState`; `ResolvedTerrainGrid` stays
  immutable.** Mirrors how the binary mutates `IsoTileTypeIndex` per cell, but
  achieves the same display result without making terrain data mutable. Wins:
  zone build, save/load, ResolvedTerrainCell tests are unaffected.
- **Gate + RNG + retry live at the world boundary, not in combat.** Combat
  pre-resolves `is_ion_cannon` and emits the typed event; world consumes RNG
  in the same order as `Apply_area_damage`. Keeps the RNG draw count exact
  for lockstep parity.
- **Pre-resolved warhead refs on `RuleSet`.** `IonCannonWarhead` and
  `C4Warhead` resolved once at world init from `[CombatDamage]`. Combat reads
  `rules.bridge_warheads.ion_cannon` directly. Mirrors binary's `Rules+0xFF0` /
  `Rules+0xFA8` storage.
- **Anchor walker mirrors `SetBridgeDirection_NESW` step-for-step.** Touches
  up to 6 cells (anchor + 3 walked +dir + 1 –dir + optional fixed-offset);
  on destruction path emits exactly **4** BlowUpBridge calls (cells 1, 2, 3,
  5). Cell 4 is flag-only (no BlowUpBridge, no `+0x2C`, no state-byte reset).
  Cell 6 (when `param_2 == 6`) is flag-only. Cell-set parity is the
  correctness foundation — every downstream behavior (kill counts, debris
  counts, RNG draws) depends on which cells belong to which span.
- **`spawn_bridge_explosions` rewritten, not preserved.** The existing
  function at world/mod.rs:851-919 spawns a wrong-vs-binary structure (1
  immediate BridgeExplosion + 50%-delayed second BridgeExplosion). Replaced
  with `spawn_bridge_debris` matching binary: 95% outer gate → 2 jitter draws
  → 50%-MetallicDebris (no delay) → 1-always-BridgeExplosion (delay 1–5).
  Existing immediate-spawn behavior was a parity drift caught by review and
  closed as part of this tier.
- **`Axis` derived from existing `bridge_layer.direction`, not added as a
  new field.** `ResolvedTerrainCell.bridge_layer: Option<BridgeLayer>` already
  carries `direction: BridgeDirection { EastWest, NorthSouth, Low }`. No new
  cell field. Anchor cells classified from `bridge_layer.overlay_id` matched
  against the BridgeSet anchor-overlay class — no `bridge_anchor_overlay`
  flag needed either.
- **`SimRng::next_range_u32_inclusive(low, high)` mirrors binary
  `RandomRanged(low, high)`.** Both ends inclusive. Adopt at all bridge gate
  + retry sites for clean translation from binary.
- **Single landing on `dev`, not Tier 2a/2b.** User priority is parity, not
  diff size. A split would introduce a documented temporary parity gap on
  bridgeheads between landings; `dev` should never sit at a known parity gap
  for parity-critical work.
- **No tech debt introduced.** All 78 ledger items have a designated home.
  No deferred follow-ups.

## Alternatives Considered

- **Approach Beta — flag-bit faithful storage.** Mirror `CellClass+0x140`
  bits as a `u32 flags` field, derive anchor pattern from flag walks, keep
  16 separate `update_ramp_*` functions. Rejected: representational proximity
  to binary doesn't buy parity beyond Alpha; harder to test, harder to
  serialize, more LOC for identical behavior.
- **Approach Gamma — hybrid (per-cell state + on-demand anchor span).**
  Skip the AnchorSpan registry struct, compute span membership on demand
  via flag walks. Rejected: marginal LOC win; loses Alpha's testability and
  snapshot ergonomics.
- **(a-medium) — same as Alpha but skip mutable damaged frame.** State byte
  exists in `BridgeRuntimeState`, but renderer keeps showing baked tile.
  Rejected: damaged-state visual is player-observable; defeats the parity
  bar.
- **(a-tight) — body cells + anchor spans + damaged frame, defer
  bridgeheads.** Rejected: bridgehead damage progression is player-visible,
  documented as Tier 2.5 deferral would be a known parity gap.
- **(b) — tri-state per-cell, no anchor spans, group-level destruction.**
  Rejected: anchor-span granularity is the largest player-visible parity gap
  in the bridge surface. Ships a parity-shaped lie.
- **Tier 2a/2b sequencing split.** Rejected: introduces a documented
  temporary parity gap on bridgeheads between landings. The user's stated
  priority is parity, not commit size.
- **Pre-resolve `is_ion_cannon` on `BridgeDamageEvent` vs lookup at world
  boundary.** Resolved at combat boundary (event time). Trade-off: one less
  field on the event struct vs slightly less work in the world orchestrator.
  Chose pre-resolve because it makes the event self-contained and avoids
  threading the warhead registry through `apply_bridge_damage_events`.
- **Read MapClass+0x54 BridgeRecord array from the .map file** (Q4 option c).
  Rejected: the array gives endpoint pairs, not full anchor pattern data;
  parsing it doesn't replace the walker.
