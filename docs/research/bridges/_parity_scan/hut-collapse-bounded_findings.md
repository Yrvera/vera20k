# Parity Scan — hut-collapse-bounded (Hut/CABHUT bounded collapse walker)

Facet: `dispatch_bridge_collapse_from_hut`, `run_hut_collapse_bounded`, `measure_extent`,
`run_hut_fallback_plan` in `src/sim/world/bridge_orchestrator.rs`.
DETERMINISM-SENSITIVE: RNG draw count/order feeds `world_hash` / lockstep.

Authority: live Ghidra decompiles this session of
`CollapseBridge_{NS,EW}_{High,Low}` @ 0x00575ba0 / 0x00575870 / 0x00575540 / 0x00575220,
`DestroyBridgeFromCell_{High,Low}` @ 0x005749c0 / 0x00574780,
`DestroyBridge_{High,Low}_OnHutDeath` @ 0x00574000 / 0x00574c20,
`DestroyBridge_High` @ 0x0057ccf0, `Random__RandomRanged` @ 0x0065c7e0,
plus `read_memory` on the off-map sentinel cell `DAT_00abdc50+0x44` (=0).

IMPORTANT: the trace doc `BRIDGE_DEEP_SLOT2_CABHUT_C4_COLLAPSE_WALKER_TRACE.md` is STALE for
this facet — its Stage 5 (canonical-seed FAIL), Stage 7 (pre-destroy NOT-IMPLEMENTED), and
Stage 9/10/11 (debris RNG ranges) describe an OLDER Rust state. Current code implements
`canonicalize_hut_destroy_seed`, `spawn_hut_walker_pre_destroy_effects`, and normalized
`RandomRanged(0,0x7FFFFFFE)` gates. Findings below are vs the CURRENT code.

---

### D1: Empty `BridgeExplosions` list skips RNG jitter draws that gamemd still consumes
- Rust now: `spawn_hut_walker_pre_destroy_effects` (bridge_orchestrator.rs:810-811) early-returns
  `if presentation.bridge_explosions.is_empty()` BEFORE any RNG draw. Same in
  `spawn_bridge_explosion_effect` (:1258-1260).
- gamemd: the per-step 3-cell pre-destroy loop in `CollapseBridge_*` (e.g. 0x00575ba0) draws
  `Random__RandomRanged(0,0x7ffffffe)` twice (X/Y jitter) UNCONDITIONALLY per cell, then
  `operator_new(0x1c8)`; only the `Random(1,5)` delay and `Random(0,BridgeExplosions.Count-1)`
  index draws are gated on `pvVar6 != 0`. There is NO guard on `BridgeExplosions.Count > 0`
  before the jitter draws — gamemd draws 2 jitter values even when the list is empty.
- Fixture: 4-step walker, 3 cells/step, `BridgeExplosions` list empty (modded/test data).
  gamemd draws `2 jitter × 3 cells × 4 steps = 24` draws (the index draw `Random(0,-1)`
  with Count=0 is a degenerate negative span; the jitter draws still fire). Rust draws 0.
  Streams desync by ≥24 draws → divergent `world_hash` next tick.
- Player sees: lockstep desync / replay divergence ONLY when `[General] BridgeExplosions=`
  is empty. Stock YR ships `BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070` (non-empty),
  so this never triggers in a stock skirmish.
- Severity: LOW (cannot trigger in stock YR; only modded/empty `BridgeExplosions`).
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00575ba0` (jitter draws precede the operator_new null
  gate; no Count>0 guard) + Rust bridge_orchestrator.rs:810.

### D2: Pre-destroy effect Z derivation differs (cosmetic height frame)
- Rust now: pre-destroy effect Z = `terrain.cell(rx,ry).bridge_deck_level_if_any().unwrap_or(level)`
  (bridge_orchestrator.rs:828-833), an integer cell/deck level.
- gamemd: `local_4 = (char)puVar5[0x11b] * DAT_00abde88` — the cell's height byte (`+0x11B`)
  times a runtime lepton-scale constant (`DAT_00abde88`, statically 0, runtime-initialized),
  producing a lepton-space Z passed to `AnimClass::Constructor`.
- Fixture: a high-bridge deck cell at deck level 4. gamemd composes `height_byte * scale`
  (lepton Z); Rust passes the raw level 4. The two Z values land at different render heights
  unless `scale` exactly maps level→Rust's deck-level units.
- Player sees: bridge-collapse explosion sprites drawn at a slightly different vertical
  offset on the deck. Cosmetic; fires every hut collapse next to a high bridge.
- Severity: LOW (render-height offset only; no sim/RNG effect).
- Confidence: UNCHECKED (DAT_00abde88 reads 0 statically — runtime-initialized; exact
  scale + how Rust's renderer consumes the Z not resolved this session).
- Verify-call: `read_memory 0x00abde88` (=0, runtime-init) + `decompile_function 0x00575ba0`
  (`local_4 = (char)puVar5[0x11b] * DAT_00abde88`).

### D3: Extent-probe safety cap (64) has no gamemd counterpart (theoretical-only)
- Rust now: `measure_extent` caps the walk at `MAX_EXTENT_PROBE = 64` (bridge_orchestrator.rs:381,
  :858). gamemd's extent walk has NO iteration cap — it terminates only on the off-band /
  off-map overlay check.
- gamemd: `CollapseBridge_*` first/second `do{}while` loops run until overlay `< 0xCD` or
  `>= 0xE9` (high) with no counter ceiling.
- Fixture: a continuous in-band bridge span > 64 cells in one axial direction from the seed.
  Rust stops counting at 64; gamemd keeps counting. If asymmetric (one side >64, other <64),
  `back`/`fwd` diverge → different `step`/bias → different collapsed footprint + RNG.
- Player sees: nothing in stock YR (no map has a single straight bridge span > 64 cells).
- Severity: LOW (unreachable on any real YR map).
- Confidence: PROVEN-DRIFT (the cap exists in Rust and not gamemd) but UNREACHABLE in YR.
- Verify-call: `decompile_function 0x00575ba0` (no loop counter on the extent do-while).

### D4: Bias subtraction on EW walk can wrap into Y / clamp differently at map edge X=0
- Rust now: `step_axis_by(seed, Axis::EW, -bias)` (bridge_orchestrator.rs:745, :894) returns
  the new X only if it lands in `0..=u16::MAX`, else `None` (walker aborts, returns empty).
- gamemd: `CollapseBridge_EW_*` computes `uVar11 = iVar4 - (iVar13 - iVar12)/2` on the FULL
  packed coord (X in low 16, Y in high 16), then `local_1c = (ushort)uVar11` truncates to the
  low 16 — a borrow from X underflow wraps within the low 16 bits (X wraps mod 65536), Y
  unaffected; the walker then proceeds.
- Fixture: EW high bridge with seed_X = 0, `back=0`, `fwd=4` → `bias = (0-4)/2 = -2` →
  start_X = 0 - (-2) = 2 (no underflow here). Underflow needs seed_X < (fwd-back)/2; e.g.
  seed_X=0, back=4,fwd=0 → bias=+2 → start_X = 0-2 = -2. gamemd wraps to 0xFFFE and proceeds
  (off-map cells return sentinel overlay 0 → break early). Rust `step_axis_by` returns None
  → walker returns empty (no collapse at all).
- Player sees: a hut collapse whose canonical seed sits within 2 cells of the X=0 map edge
  could collapse 0 cells in Rust vs a (short, mostly off-map) walk in gamemd. Extremely rare
  (bridge hut at the absolute map border).
- Severity: LOW (requires bridge + hut against cell column 0/row 0).
- Confidence: LIKELY-DRIFT (wrap-vs-abort confirmed in code; exact gamemd post-wrap behavior
  at the literal edge not single-stepped).
- Verify-call: `decompile_function 0x00575870` (`uVar11 = iVar4 - (iVar13-iVar12)/2`;
  `local_1c = (ushort)uVar11`) + bridge_orchestrator.rs:894-901.

### D5: Fallback ramp-walk path is structurally divergent and not bit-verified
- Rust now: when `find_destroy_overlay_seed` returns None, `run_hut_fallback_plan`
  (bridge_orchestrator.rs:553-596) runs an 8-direction starter search + anchor resolve +
  ramp-walk using `BRIDGE_FLAG_*` terrain facts and `apply_hut_damage_retries` →
  `apply_hut_damage_to_cell` (which routes by overlay sub-band to `destroy_bridge_*` /
  `bridgehead_advance_state` / `body_cell_advance_state`).
- gamemd: the fallback arm of `DestroyBridge_*_OnHutDeath` (0x00574000 / 0x00574c20) reads
  cell flags `+0x140 & 0x500`, does an 8-direction `g_DirectionOffsets` sweep up to 3 cells,
  branches on anchor flags (`0x100`/`0x400`/`0x800`/`0x80`), then walks forward via
  `IsBridgeRampTile` / `IsLowBridgeEndpointTile` calling `ApplyDamageToCell` up to 3× per ramp
  hit in the reversed direction. This is a different primitive (`ApplyDamageToCell`, the full
  damage state machine) than Rust's `apply_hut_damage_to_cell`.
- Fixture: not walked to a single divergent cell this session (the flag semantics
  `+0x140 & 0x500` ↔ Rust `BRIDGE_FLAG_STRUCTURAL|DESTROYED_OR_RAMP` mapping needs its own
  trace). The continuation cap differs: gamemd pure-bridgehead loop bails at `local_20 > 3`
  (4 steps); Rust `resolve_pure_bridgehead_anchor` bails at `continuations >= 4`.
- Player sees: when a CABHUT is NOT directly over/adjacent to a bridge overlay cell (rare —
  e.g. hut on a pure ramp/bridgehead approach), the collapsed cells / damage may differ.
  Stock CABHUT placement is on/beside the deck, so the overlay-seed path (not fallback) is
  the common case.
- Severity: MED (player-visible footprint difference, but low trigger frequency — only the
  no-overlay-in-5x5 placements).
- Confidence: UNCHECKED (primitive mismatch identified; exact per-cell equality not proven).
- Verify-call: `decompile_function 0x00574000` (fallback arm after the 5x5 scan) +
  bridge_orchestrator.rs:553.

### D6: Pre-destroy anim-index draw uses `RulesClass+0x168` count; metallic absent from walker
- Rust now: `spawn_bridge_explosion_effect` draws index via
  `next_range_u32(bridge_explosions.len())` and NO metallic-debris draw in the pre-destroy
  walker stage (bridge_orchestrator.rs:1261-1266).
- gamemd: the walker pre-destroy loop draws `Random(1,5)` then
  `Random(0, RulesClass+0x168 - 1)` (= BridgeExplosions.Count-1) and constructs
  `RulesClass+0x15C[idx]` (BridgeExplosions array). It does NOT draw metallic debris in the
  walker — metallic is only in `CellClass::BlowUpBridge`. So the walker RNG order is exactly
  [jitterX, jitterY, delay(1,5), animIdx] per cell.
- Fixture: stock YR, BridgeExplosions.Count = 4 → `Random(0,3)`, span 3, draw consumed in
  both. Per step (center != cap): 3 cells × [2 jitter + delay + index] = 12 draws in both.
  Per 4-step walk: up to 48 draws in both. MATCHES.
- Player sees: nothing — this sub-aspect MATCHES for stock YR.
- Severity: — (recorded as PARITY-CONFIRMED, see below).
- Confidence: PROVEN-DRIFT only in the degenerate `Count==1` case where `Random(0,0)` draws
  nothing in BOTH (verified `RandomRanged` @ 0x0065c7e0 returns early on low==high, matching
  Rust `next_range_u32_inclusive` lo==hi early return) — i.e. still PARITY.
- Verify-call: `decompile_function 0x00575ba0` + `decompile_function 0x0065c7e0` +
  Rust rng.rs:131-139.

---

## PARITY-CONFIRMED (checked, found matching)

1. **5x5 hut scan order (X-major).** gamemd `DestroyBridge_*_OnHutDeath` (0x00574000/0x00574c20):
   outer `iVar9 = -2..<3` varies X (`*param_2 + iVar9`), inner `iVar8 = -2..<3` varies Y
   (`param_2[1] + iVar8`) → X-major. Rust `hut_destroy_5x5_scan` (:222-235):
   `(-2..=2).flat_map(|dx| (-2..=2).filter_map(|dy| ...))` = X-major. First-match-and-return
   in both. PROVEN.
2. **Low/high family choice.** gamemd selects Low if any 5x5 cell is a low overlay
   `[0x4A..0x65]` or low wood-bridge tile, else High. Rust `choose_hut_bridge_family` (:408)
   mirrors via `is_low_destroy_overlay` / `is_wood_bridge_repair_tile`. (Literal tile-index
   range equality is the one UNCHECKED sub-point — see below.)
3. **Canonical seed adjustment (DestroyBridgeFromCell_*).** VERIFIED current Rust
   `canonicalize_hut_destroy_seed` (:258-287) MATCHES gamemd 0x005749c0/0x00574780:
   probe axis = walker-family axis (= perpendicular to physical span); if probe(-1) off-band
   → seed = matched + 1; else if probe(-2) in-band → seed = matched - 1; else seed = matched.
   (Trace doc Stage 5 "FAIL" is STALE.) Also verified the NS-subrange→EW-walker / EW-subrange
   →NS-walker name swap is correctly handled by `physical_span_axis_for_destroy_overlay` (:670).
4. **Overlay sub-range classification.** Rust `is_ns/ew_walker_overlay_high/low`
   (walker.rs:597-619) byte ranges exactly match the gamemd `DestroyBridgeFromCell_*` and
   `DestroyBridge_High` (0x0057ccf0) ranges: high NS `[0xCD..0xD5]∪[0xDF..0xE2]∪0xE7`, high EW
   `[0xD6..0xDE]∪[0xE3..0xE6]∪0xE8`; low NS `[0x4A..0x52]∪[0x5C..0x5F]∪0x64`, low EW
   `[0x53..0x5B]∪[0x60..0x63]∪0x65`.
5. **`local_2c = 4` step count.** `MAX_HUT_SWEEP_STEPS = 4` (:367) == gamemd `local_2c = 4`
   in all four CollapseBridge twins.
6. **3-retry per step.** `MAX_HUT_ATTEMPTS_PER_STEP = 3` (:368) == gamemd
   `do{ DestroyBridge_*; if !=0 break; } while(iVar10 < 3)`. Break-on-success semantics match
   (Rust breaks on Collapsed-with-success; retries on Absorbed and NoChange — gamemd breaks
   only on `DestroyBridge_* != 0` = full-destroy, retries on intermediate writes/no-op).
7. **Step direction tiebreak.** `step = if fwd < back { -1 } else { 1 }` (:742) == gamemd
   `if (iVar10 < iVar11) local_14 = -1;` (default +1). Identical including the fwd==back → +1
   tiebreak.
8. **Bias / start formula.** `bias = (back - fwd)/2; cur = seed - bias` (:743-745) ==
   gamemd `seed - (iVar11 - iVar10)/2` (iVar11=back, iVar10=fwd) with signed truncating
   division. ALGEBRAIC: Rust `i32 / 2` truncates toward zero == x86 `idiv` semantics. PROVEN.
9. **Extent-count off-by-one is ABSORBED (algebraic proof).** gamemd increments the count
   BEFORE the band check so `back`/`fwd` each include their terminator cell (off-band or the
   off-map sentinel whose overlay `+0x44 = 0`, verified via read_memory). Rust counts only
   in-band cells (breaks before increment). Let gamemd back=B+1, fwd=F+1 and Rust back=B,
   fwd=F. Then `(back-fwd)/2`: gamemd `((B+1)-(F+1))/2 = (B-F)/2` == Rust `(B-F)/2`. And
   `fwd<back`: gamemd `F+1<B+1 ⟺ F<B` == Rust. The +1 cancels in BOTH consumers. PARITY
   (sole exception is the MAX_EXTENT_PROBE asymmetry — see D3, unreachable).
10. **Walk-step band gate.** Rust loop break `if !in_bridge_band(family, overlay)` (:793)
    after stepping == gamemd tail `if (overlay < 0xCD || 0xE8 < overlay) break;` (high). Off-map
    step → break in both (Rust `step_axis`→None; gamemd sentinel overlay 0 < 0xCD).
11. **Pre-destroy terminal-cap gate + perpendicular triplet.** Rust skips anims when center
    overlay == `hut_walker_terminal_cap` (:819, caps High/NS=0xE8, High/EW=0xE7, Low/NS=0x65,
    Low/EW=0x64) == gamemd `if (overlay != 0xE8/0xE7/0x65/0x64)` per twin. Triplet is the 3
    perpendicular cells in order -1,0,+1 (NS-walk → X triplet; EW-walk → Y triplet) in both.
12. **Pre-destroy RNG order [jitterX, jitterY, delay(1,5), animIdx].** Matches per cell in
    both (D6). `RandomRanged` equal-bounds early-return verified == Rust rng.rs:137.
13. **OnHutDeath callers are live YR.** `get_function_callers 0x00574000` = `BuildingClass::Update`
    (CABHUT C4 timer) + `BombClass::Detonate` (demo truck). No SpecialFlags / TS gate. Not
    TS-legacy.
14. **`DestroyBridge_High` (0x0057ccf0) draws no RNG itself** — only its `DestroyBridgeWalker_*`
    callees (scatter, a separate facet) can. The walker's per-step RNG accounting (D6) is the
    pre-destroy anim loop only.

## UNCHECKED

- **U1: Low family literal tile-index range.** gamemd low-family pre-scan also matches a wood
  bridge tile-index in `[WoodBridgeSet, WoodBridgeSet+0x10)`; Rust uses resolved-terrain
  `is_wood_bridge_repair_tile` metadata, not a literal tile-index compare. Equivalence of the
  two not proven here (needs the WoodBridgeSet base + Rust tile-resolution trace).
- **U2: Fallback ramp-walk per-cell equality (D5).** Flag mapping `+0x140 & 0x500` ↔ Rust
  `BRIDGE_FLAG_*` and the `ApplyDamageToCell`-vs-`apply_hut_damage_to_cell` primitive
  substitution not bit-verified.
- **U3: Pre-destroy effect Z scale (D2).** `DAT_00abde88` is runtime-initialized (static 0);
  the lepton-per-level constant and the Rust renderer's consumption of the level value not
  resolved.
- **U4: DestroyBridgeWalker_* scatter RNG inside the retry loop.** Each `DestroyBridge_*` retry
  may run the per-cell overlay walker, which (at full-destroy) does a 3×3 scatter that can draw
  RNG. That RNG is shared with the direct-damage path (a different facet) and was not counted
  here; if it diverges it is not hut-specific.
