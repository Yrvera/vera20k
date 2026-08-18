# Parity scan — dispatcher-rng-gate (4-path damage dispatcher + BridgeStrength RNG gate)

Facet: `src/sim/world/bridge_orchestrator.rs::run_dispatch_loop` + `apply_bridge_damage_events`;
`src/sim/bridge_state/mod.rs::path_matches_cell`.

Authority: live decompile/disassembly of `Apply_area_damage @ 0x00489280`,
`ApplyDamageToCell @ 0x00587180`, `Random__RandomRanged @ 0x0065C7E0` (all anchors re-confirmed
this session via `get_function_by_address`). Cross-checked against
`BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md`, `BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md §3`,
`BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md §5.1`, `WEAPON_AOE_BRIDGE_DAMAGE_ENTRY_GHIDRA_REPORT.md §3.4`.

DETERMINISM-SENSITIVE facet: per-cell RNG draw COUNT and ORDER drive `world_hash`/lockstep.

---

### D1: First-match-wins `break` collapses 4 independent RNG branches into 1 — wrong draw count
- Rust now: `run_dispatch_loop` (`bridge_orchestrator.rs:1407-1474`) iterates the 4 paths
  `[HighStateMachine, LowStateMachine, LowDirect, HighDirect]`, and for the FIRST path whose
  `path_matches_cell` returns true it rolls ONE BridgeStrength gate draw, runs the driver, then
  `break`s out of the path loop (`:1473`). At most ONE BridgeStrength RNG draw is consumed per
  event (plus retries only on Ion state-machine paths).
- gamemd: `Apply_area_damage @ 0x00489280` runs FOUR sequential, independent bridge blocks with
  NO early-out. The control flow falls through every block:
  - Block A (tile-index `DAT_00aa0e28`/`DAT_00abad30`/`DAT_00aa1028` ranges + structural-flag/`0x18/0x19`
    neighbor) → RNG gate at `0x00489FE0..0x00489FFE` → `ApplyDamageToCell` (+3 retries on Ion) → falls to Block B.
  - Block B (tile-index `DAT_00abad1c` ranges + `0xed/0xee` neighbor, `0x0048A0A5`) → RNG gate
    `0x0048A165..0x0048A182` → `ApplyDamageToCell` (+3 on Ion) → falls to Block C.
  - Block C (overlay `0x4A..=0x63`, `0x0048A214`) → RNG gate `0x0048A231..0x0048A24E` →
    `DestroyBridge_Low @ 0x0057baa0` → falls to Block D.
  - Block D (overlay `0xCD..=0xE6`, `0x0048A26A`) → RNG gate `0x0048A28B..0x0048A2A8` →
    `DestroyBridge_High @ 0x0057ccf0`.
  Each block re-evaluates its own gate and calls `Random__RandomRanged(1, BridgeStrength)` from the
  shared `Scenario+0x218` RNG. Deep-dive §3.1 (lines 206-209): "There is no shared HP pool. Each of
  the four branches has its own RNG call. A single weapon impact rolls independently against the same
  BridgeStrength for the low-zone, high-zone, low-body, and high-body branches."
- Fixture: structural high-bridge body cell whose `overlay_byte = 0xC0` (already transitioned out of
  the body range, i.e. `ApplyDamageToCell` would route via `ProcessBridgeDamageStateMachine_High`),
  impact in Z window, non-Ion warhead, `BridgeStrength=1500`, `damage=80`.
  - Binary: Block A gate rolls draw #1 `R(1,1500)`; if `>=80` the block does nothing but flow
    FALLS THROUGH. Block B is then evaluated; its tile-index test fails for this cell so Block B
    rolls 0 draws. Block C overlay test (`0x4A..=0x63`) fails → 0 draws. Block D overlay test
    (`0xCD..=0xE6`) fails (cell is `0xC0`) → 0 draws. Net: exactly 1 draw, but the draw belongs to
    whichever of A/B matched the cell's tile index — and if BOTH A and B match a cell (their
    tile-index sets are not provably disjoint), the binary rolls TWO draws while Rust rolls one.
  - Rust: HighStateMachine matches → 1 draw `next_range_u32_inclusive(1,1500)` → `break`. Net 1 draw.
  The divergence is exposed any time more than one binary block's gate is reachable for the same
  cell: e.g. a cell counted in both Block A's and Block B's tile-index range, OR a cell that the
  binary routes through both a tile-index block AND would also satisfy an overlay block. Rust's single
  shared `path_matches_cell` + `break` cannot reproduce 2 draws.
- Player sees: post-collapse RNG stream advances by a different number of draws → desyncs every
  downstream RNG consumer (debris jitter, scatter, crit, miner spawn) for the rest of the match.
  In a multiplayer lockstep game this is an immediate `world_hash` mismatch → desync. Triggers on
  the first weapon impact that overlaps a structural bridge cell whose tile-index lands in more than
  one of the binary's bridge-block ranges — happens whenever players fight over a bridge.
- Severity: HIGH (lockstep desync; bridges are a routine combat objective).
- Confidence: PROVEN-DRIFT (binary fall-through has no `break`; Rust has explicit `break` at :1473;
  deep-dive §3.1 confirms independent per-branch RNG).
- Verify-call: `decompile_function 0x00489280` + `disassemble_function 0x00489f00` (4 RNG sites at
  `0x00489FF5`, `0x0048A179`, `0x0048A245`, `0x0048A29F`, each followed by `JGE` fall-through, no break).

---

### D2: Z-height window lower bound off-by-one and wrong unit frame
- Rust now: `path_matches_cell` (`bridge_state/mod.rs:895-901`) computes
  `level_i32 = terrain.cell.level` and rejects when `impact_z < level_i32 - 1 || impact_z > level_i32 + 1`.
  Accepted window is the closed interval `[level-1, level+1]` in raw level units. `impact_z` is built
  in `combat_aoe.rs:54-58` as `cell.level + bridge_height_for_selector(cell)` (level units), where the
  selector height is `(deck_level - level).max(BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS)`.
- gamemd: Block A `0x00489F82..0x00489FBC` and Block B `0x0048A10D..0x0048A141` compute, only when
  `Flags & 0x100` (structural) is set:
  - upper: skip if `impact_z > (Level + 1) * LevelHeight + BridgeHeight`  (`SAR`-free `IMUL` by
    `DAT_0089e870`=LevelHeight then `ADD DAT_0089e864`=BridgeHeight; `CMP ECX,ESI; JG`).
  - lower: skip if `impact_z <= (Level - 2) * LevelHeight + BridgeHeight`  (`ADD EAX,-0x2`; `IMUL`;
    `ADD`; `CMP ECX,EAX; JLE`).
  Accepted window: `(Level-2)*LevelHeight + BridgeHeight  <  impact_z  <=  (Level+1)*LevelHeight + BridgeHeight`,
  where `impact_z` (`param_1[2]`) is the detonation Z in lepton/CoordStruct units, NOT raw level units.
  (`BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md §5.1`; `WEAPON_AOE_BRIDGE_DAMAGE_ENTRY_GHIDRA_REPORT.md §3.4`.)
- Fixture: structural high bridge at `Level=2`, deck two levels up. Take a hit one level BELOW the
  deck level.
  - Binary lower bound is `Level-2 = 0` (exclusive): a hit at the equivalent of `Level-1` is still
    ACCEPTED (well above the `Level-2` floor). The window spans roughly 3 LevelHeight-units below to
    1 above the cell level.
  - Rust window `[level-1, level+1]` = `[1,3]`. With Rust's `impact_z = level + selectorHeight`
    (e.g. `2 + 2 = 4`), `4 > 3` → Rust REJECTS the on-deck hit that the binary would ACCEPT
    (binary upper bound is `(2+1)*LevelHeight + BridgeHeight`, comfortably above a deck-height Z).
  The lower bound differs by one level (`-2` exclusive vs `-1` inclusive) and the window is expressed
  in incompatible units: binary scales each bound by `LevelHeight` and adds `BridgeHeight`; Rust
  compares raw level integers. Because Rust's `impact_z` is `level + height-in-levels` while the
  binary's `impact_z` is a lepton Z, the `±1` clamp does not correspond to the binary's
  `(Level±k)*LevelHeight + BridgeHeight` bounds at any LevelHeight value.
- Player sees: state-machine bridge cells (already-transitioned overlays) accept or reject damage at
  wrong impact heights — a shell landing on the deck of a tall bridge may be silently ignored (no
  collapse) or a near-miss below the bridge may wrongly collapse it. Triggers on every state-machine
  hit on a HIGH bridge (the only family where deck-vs-ground Z separation is large). Direct-overlay
  paths (C/D) are unaffected — they have no Z gate in either engine (matches).
- Severity: MED (visible wrong collapse/no-collapse on high bridges; lower frequency than D1 because
  it only bites the SM sub-path and only on tall bridges).
- Confidence: PROVEN-DRIFT (binary bounds and unit scaling read from disassembly; Rust raw-level clamp
  read from source). The exact LevelHeight (`DAT_0089e870`) / BridgeHeight (`DAT_0089e864`) values are
  runtime-initialized (read back 0 from static memory) so the numeric magnitude of the gap is
  UNCHECKED, but the `-2`-vs-`-1` lower bound and the unit-frame mismatch are structural.
- Verify-call: `disassemble_function 0x00489f00` (`0x00489F82 MOVSX [EDI+0x11b]`=Level; `0x00489F8D LEA [EAX+1]`;
  `0x00489F90 IMUL [0x0089e870]`; `0x00489F97 ADD [0x0089e864]`; `0x00489FA8 ADD EAX,-0x2`).

---

### D3: High/Low state-machine routing uses `deck_level>=4`, not the binary's tile-index + flag test
- Rust now: `path_matches_cell` (`bridge_state/mod.rs:884-891`) discriminates HighSM vs LowSM purely
  by `is_high = cell.deck_level >= 4`, and requires `is_high == want_high`. So a single cell can match
  at most ONE of the two SM paths.
- gamemd: there are no "HighSM" / "LowSM" entry paths in `Apply_area_damage`. Blocks A and B both call
  the SAME function `ApplyDamageToCell @ 0x00587180`, which performs its OWN internal dispatch:
  1. overlay `0x4A..=0x63` → `DestroyBridge_Low`; overlay `0xCD..=0xE6` → `DestroyBridge_High`
     (these short-circuit, return);
  2. else tile-index test `(Overlay/IsoTile - DAT_00aa0e28)+1` vs ranges + `Flags&0x100` +
     `0x18/0x19` neighbor → `ProcessBridgeDamageStateMachine_High`;
  3. else second tile-index test `(... - DAT_00abad1c)+1` + `0xed/0xee` neighbor →
     `ProcessBridgeDamageStateMachine_Low`.
  The high/low selection is driven by ISO-TILE-TYPE-INDEX membership in the high vs low bridge tile
  sets and by the `0x18/0x19` (low anchor) vs `0xed/0xee` (high anchor) neighbor overlays — NOT by a
  `deck_level >= 4` threshold.
- Fixture: a LOW (wood) bridge whose runtime `deck_level` happens to be >= 4 (e.g. wood bridge built
  over a deep gully so the deck sits 4 levels above the gully floor).
  - Binary: `ApplyDamageToCell` routes by the wood tile-index set → `ProcessBridgeDamageStateMachine_Low`.
  - Rust: `deck_level >= 4` → classifies as HighSM, `want_high` true → matches `HighStateMachine`,
    then drives `body_cell_advance_state(.., is_high=true ..)`. Wrong family selected.
  Conversely a HIGH concrete bridge in a shallow map with `deck_level < 4` routes to LowSM in Rust.
- Player sees: wrong state-machine family advances the cell → the collapse animation/overlay band and
  the perpendicular `UpdateRamp_*` writes use the wrong (low vs high) tile family, producing a visibly
  mismatched destroyed-bridge sprite, and potentially a different number of RNG/effect draws on the
  cascade side. Triggers whenever a bridge's runtime deck level crosses the hardcoded `4` threshold in
  the "wrong" direction relative to its tile family — map-dependent but deterministic per map.
- Severity: MED (wrong-family collapse visuals; depends on map deck-level layout vs the `4` constant).
- Confidence: LIKELY-DRIFT (binary routing by tile-index/neighbor is proven; the Rust `deck_level>=4`
  proxy is a known approximation. Whether any stock YR map actually puts a wood bridge deck at >=4 or
  a concrete bridge deck at <4 is UNCHECKED, so player-visibility frequency is not pinned.)
- Verify-call: `decompile_function 0x00587180` (overlay-direct first, then two tile-index/flag SM branches;
  no deck-level-4 test anywhere).

---

### D4: `ApplyDamageToCell` is itself a 4-way dispatcher (overlay-direct first) — Rust splits this across two layers, changing intra-block precedence
- Rust now: the orchestrator treats HighDirect/LowDirect (overlay `0xCD..=0xE6` / `0x4A..=0x63`) and
  HighSM/LowSM as four SIBLING paths at the same level, ordered `HighSM, LowSM, LowDirect, HighDirect`
  (`bridge_orchestrator.rs:1407-1412`). With first-match-`break`, a body cell that is BOTH overlay-in-band
  AND role-bridge can only ever take the SM path first because SM is listed before Direct AND the
  SM-path matcher explicitly rejects in-band overlays (`bridge_state/mod.rs:854-863`), so in practice a
  raw-overlay cell takes the Direct path and a transitioned cell takes the SM path.
- gamemd: within a SINGLE block (A or B), `ApplyDamageToCell` checks overlay-direct FIRST
  (`0x4A..=0x63` → `DestroyBridge_Low`, then `0xCD..=0xE6` → `DestroyBridge_High`), and only if neither
  matches does it fall to the tile-index SM branches. AND separately, the outer `Apply_area_damage`
  Blocks C and D ALSO call `DestroyBridge_Low`/`DestroyBridge_High` directly for the same overlay ranges.
  So for a raw-overlay cell the binary can reach `DestroyBridge_*` through TWO routes: (a) inside Block
  A/B's `ApplyDamageToCell` after passing the Block A/B gate + Z window, and (b) Block C/D after passing
  the Block C/D gate. Each route is a separate RNG draw.
- Fixture: high bridge body cell `overlay_byte = 0xD0` (raw in-band), structural, impact in Block A's
  Z window, non-Ion, `damage=80`, `BridgeStrength=1500`.
  - Binary: Block A gate rolls draw #1; if it passes, `ApplyDamageToCell` sees overlay `0xD0 ∈ 0xCD..=0xE6`
    → calls `DestroyBridge_High` immediately (NOT the SM). Flow then falls through to Block D, whose
    overlay gate (`0xCD..=0xE6`) ALSO matches `0xD0` → rolls draw #2 → calls `DestroyBridge_High` AGAIN.
    Up to 2 draws and 2 destroy attempts for this one cell.
  - Rust: HighSM matcher rejects `0xD0` (in-band, `:854`); LowSM rejects (not low band); LowDirect
    rejects (`0xD0 ∉ 0x4A..=0x63`); HighDirect matches → 1 draw → `destroy_bridge_high` → `break`.
    Exactly 1 draw, 1 attempt.
  - Net: binary 2 draws / Rust 1 draw for a structural in-band high cell whose tile index also lands in
    Block A's range.
- Player sees: same lockstep-desync class as D1 (RNG draw-count divergence) plus a possible double
  destroy attempt (harmless to state if already destroyed, but the SECOND attempt's RNG draw still
  advances the stream). Triggers on raw-overlay structural cells hit by a Bridge-capable warhead in the
  Z window.
- Severity: HIGH (compounds D1; RNG draw-count divergence → desync).
- Confidence: PROVEN-DRIFT (`ApplyDamageToCell` overlay-first dispatch read from decompile; Block C/D
  duplicate overlay gates read from disassembly at `0x0048A214`/`0x0048A26A`).
- Verify-call: `decompile_function 0x00587180` (overlay `0x49<x<100` and `0xcc<x<0xe7` checked before SM)
  + `disassemble_function 0x00489f00` (`0x0048A217 CMP 0x4a`, `0x0048A26D CMP 0xcd`).

---

### D5: Rust gates Block-A/B equivalent on `path_matches_cell` BEFORE the RNG roll, but skips the structural-flag precondition for the Z window
- Rust now: the Z-window gate in `path_matches_cell` (`:892-901`) is applied UNCONDITIONALLY to every
  SM-path candidate cell (any cell whose `role` is bridge and overlay transitioned).
- gamemd: the Z-window (`0x00489F77 TEST AH,0x1`) is applied ONLY when `Flags & 0x100` (structural) is
  set. If the cell is NOT structural (the `0x18/0x19`-neighbor-anchor case that reached Block A via the
  neighbor test, not via its own structural flag), the binary JUMPS PAST the Z window straight to the
  `warhead+0x144` + RNG gate (`0x00489FC2`). I.e. a non-structural anchor-adjacent cell gets NO Z gate.
- Fixture: a low-bridge ANCHOR-ZONE cell that is itself non-structural but sits next to a `0x18/0x19`
  overlay (the binary's "Low-bridge anchor zone" branch, deep-dive §3.1 item 1). Impact Z far outside
  `[level-1, level+1]`.
  - Binary: `Flags & 0x100 == 0` for the anchor cell → `JZ 0x00489FC2` skips the whole Z window → rolls
    the RNG gate and may damage.
  - Rust: the same cell, if it reaches an SM path, is rejected by the unconditional Z gate → 0 draws,
    no damage.
- Player sees: anchor-zone cells of bridges may fail to take damage at impact heights the binary would
  accept (or, if Rust never routes anchor-zone cells to SM at all, this manifests as missing-behavior
  rather than wrong-Z). Triggers on hits near a bridge's land anchor at off-deck Z.
- Severity: LOW-MED (narrow: only the anchor-zone sub-branch at unusual impact Z; anchor cells are a
  small fraction of a bridge span, but a bridge has exactly two of them and they are common aim points).
- Confidence: LIKELY-DRIFT (binary structural-flag gating of the Z window is proven; whether Rust's
  combat boundary ever emits an SM event for a non-structural anchor-zone cell is UNCHECKED — depends on
  how `role`/`overlay` are populated for anchor cells, which is outside this facet's files).
- Verify-call: `disassemble_function 0x00489f00` (`0x00489F7D TEST AH,0x1` / `0x00489F80 JZ 0x00489FC2`
  bypasses the Z window for non-structural cells).

---

## PARITY-CONFIRMED (checked, found matching)

- **BridgeStrength RNG range + strictness.** Binary `Random__RandomRanged(1, Rules+0x1740)` with
  inclusive `[1, BridgeStrength]` and `CMP EAX,[damage]; JGE skip` (strict `roll < damage`, equality
  fails). Rust `next_range_u32_inclusive(1, bridge_strength)` then `!((roll as u16) < damage)` continue
  — identical bounds and strictness. (`bridge_orchestrator.rs:1419-1423`; binary `0x00489FF5/0x00489FFE`.)
  Verified `decompile_function 0x0065C7E0` (inclusive both-ends range generator).
- **IonCannon bypass.** Binary jumps around the RNG call when `warhead == Rules+0xFF0`
  (`0x00489FD8 CMP / JZ 0x0048A004`). Rust `if !ctx.is_ion_cannon { roll … }` (`:1418`) — Ion consumes
  ZERO gate draws on every path. Match.
- **IonCannon 3-retry (4-attempt total) on state-machine paths only.** Binary retry loop
  `MOV ESI,3; while(cVar7==0){ if(!is_ion||ESI<1) break; ApplyDamageToCell(); ESI-- }`
  (`0x0048A015..0x0048A03A`, mirrored at `0x0048A199..0x0048A1BE`). Rust
  `max_attempts = if is_ion_cannon && path.is_state_machine() { 4 } else { 1 }` (`:1429-1433`) — same
  4-total, same SM-only restriction. Direct paths (`DestroyBridge_*`) are single-shot in both
  (binary Block C/D have no retry loop). Match. (Deep-dive §3.2 confirms C4/CrushWarhead get exactly 1.)
- **Direct-overlay ranges.** Binary Block C `0x4A..=0x63` (`CMP 0x4a JL / CMP 0x63 JG`), Block D
  `0xCD..=0xE6` (`CMP 0xcd JL / CMP 0xe6 JG`); `ApplyDamageToCell` internal `0x49<x<100` and
  `0xcc<x<0xe7`. Rust `LowDirect=(0x4A..=0x63)`, `HighDirect=(0xCD..=0xE6)`
  (`bridge_state/mod.rs:848-849`) and the SM matcher rejects these same bands (`:854-863`). Ranges match.
- **Gate-before-driver ordering for structural/Z checks.** Rust front-loads role/overlay/Z checks into
  `path_matches_cell` (returns false → no draw) before rolling RNG, mirroring the binary's structural +
  tile-index + Z-window tests that precede each `Random__RandomRanged` call. The ORDER of (structural
  check → roll → driver) within a single block matches; only the bound/unit details (D2) and the
  multi-block independence (D1/D4) diverge.
- **Outer DestroyableBridges gate.** Rust bails when `!is_destroyable()` before any work
  (`bridge_orchestrator.rs:66-69`); binary `Scenario & 0x8000` master gate (`0x00489EAB TEST CH,0x80`).
  Match (the `warhead+0x144`=Bridge= per-warhead half of the gate is pre-resolved at the Rust combat
  boundary, outside this facet).
- **`path_matches_cell` is RNG-free.** No `rng` access inside `path_matches_cell`; the bridgehead
  axis=None rejection (`:881-883`) correctly avoids burning a draw on a cell that would NoChange — this
  is a deliberate parity-preserving guard, consistent with the binary not rolling for cells its
  tile-index/flag tests reject.

---

## UNCHECKED

- **Exact LevelHeight (`DAT_0089e870`) and BridgeHeight (`DAT_0089e864`) magnitudes.** Both read back
  `0x00000000` from static memory via `read_memory` — they are runtime-initialized (BSS, set from the
  cell-height tables at scenario load). The numeric size of the D2 Z-window gap therefore can't be
  computed offline; the structural `-2`-vs-`-1` lower-bound and unit-frame mismatch stand regardless,
  but the per-fixture lepton boundary value is unverified. Next step: read these via the debugger with
  a scenario loaded, or trace `BridgeShadowTable_StaticInit @ 0x00543f10` / `BridgeSlopeTable_StaticInit
  @ 0x00544691` for the init writers.
- **Whether stock YR maps actually trigger D3.** Whether any retail YR map places a wood (low) bridge
  deck at runtime `deck_level >= 4`, or a concrete (high) bridge deck at `< 4`, is not verified. If no
  stock map crosses the `4` threshold in the wrong direction, D3 has zero in-skirmish frequency (still a
  latent drift for modded/custom maps and for the 20k-scale target).
- **Whether Block A's and Block B's tile-index sets overlap for any real cell** (the precise condition
  that makes D1 roll 2 binary draws vs Rust's 1). The `DAT_00aa0e28/DAT_00abad30/DAT_00aa1028` (Block A)
  vs `DAT_00abad1c` (Block B) tile-index bases are distinct globals but their numeric values (and thus
  whether the ranges can co-cover a cell) are runtime-initialized and were not read this session.
  Regardless, D4's overlay-block duplication (Block A/B `ApplyDamageToCell` → `DestroyBridge_*` AND
  Block C/D → `DestroyBridge_*`) already proves a 2-draw-vs-1-draw case independent of A/B overlap.
- **Whether the Rust combat boundary ever emits an SM-routable event for a non-structural anchor-zone
  cell** (the precondition for D5 to be observable rather than latent). Depends on `role`/`overlay`
  population in `from_resolved_terrain`, outside this facet's two files.
