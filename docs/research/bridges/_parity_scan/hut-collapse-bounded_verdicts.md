# Adversarial Verdicts — hut-collapse-bounded (Hut/CABHUT bounded collapse walker)

Auditor: adversarial skeptic. Method: re-decompiled the cited gamemd functions live this
session, re-read current Rust at the cited lines, applied burden-of-proof (default DRIFT;
downgrade only on proof). Read-only; no Rust edits.

Functions re-confirmed live this session:
- `MapClass__CollapseBridge_NS_High` @ 0x00575ba0 (`get_function_by_address` confirms name+entry).
- `MapClass__CollapseBridge_EW_High` @ 0x00575870 (confirmed name+entry).
- `MapClass__DestroyBridge_High_OnHutDeath` @ 0x00574000 (confirmed name+entry).
- `Random__RandomRanged` @ 0x0065c7e0 (decompiled: `if (param_2 != param_3)` equal-bounds early-out).
- `DAT_00abde88` xrefs (`get_xrefs_to`): WRITE site `FUN_005617e0` @ 0x005617e0 =
  `Sin_Lookup_Table4096(...) -> Math__ftol -> DAT_00abde88` (runtime-init, not static 0).
- Callers of 0x00574000 (`get_function_callers`): `BombClass__Detonate`, `BuildingClass__Update` only.

---

## D1: VERDICT=REAL
Empty `BridgeExplosions` list still draws 2 jitter RNG per perpendicular cell in gamemd; Rust
skips all RNG. Live decompile @ 0x00575ba0: inside the `iVar10 = 3` perpendicular triplet loop,
both `Random__RandomRanged(0,0x7ffffffe)` calls (X then Y jitter) fire UNCONDITIONALLY, BEFORE
`operator_new(0x1c8)` and BEFORE the `if (pvVar6 != 0)` gate. The ONLY guard is `pvVar6 != 0`
around `Random(1,5)` and `Random(0, RulesClass+0x168 -1)`. There is no `BridgeExplosions.Count > 0`
test before the jitter draws. Current Rust `spawn_bridge_explosion_effect` (bridge_orchestrator.rs:1258)
and `spawn_hut_walker_pre_destroy_effects` (:810) both `if presentation.bridge_explosions.is_empty()
{ return; }` BEFORE `bridge_jittered_subcells` (the 2 jitter draws). Finder's reading holds.
Corrected delta: Rust draws 0 RNG when `BridgeExplosions` empty -> gamemd draws 2 jitter RNG per
perpendicular cell (2 x 3 cells x up-to-4 steps = up to 24 draws), desyncing the lockstep stream.
Severity LOW: unreachable in stock YR (`BridgeExplosions=TWLT026,...` non-empty); modded/empty-list only.

## D2: VERDICT=UNCERTAIN
Pre-destroy effect Z. Live @ 0x00575ba0 confirms `local_4 = (char)puVar5[0x11b] * DAT_00abde88`
(cell height byte x runtime lepton-scale), and the AnimClass ctor receives `&local_c` (the
local_c/local_8/local_4 coord triple, so local_4=Z is passed). `DAT_00abde88` is statically 0 but
`get_xrefs_to` shows a WRITE in `FUN_005617e0` (`Sin_Lookup_Table4096 -> ftol`), so it is
runtime-initialized nonzero — the gamemd Z is a real lepton value, not 0. Current Rust
(bridge_orchestrator.rs:828-831) passes `bridge_deck_level_if_any().unwrap_or(level)` — an integer
deck/cell level, not `height_byte * scale`. The gamemd reading holds (different derivation), BUT I
cannot prove an OBSERVABLE render-height difference this session: the runtime value of DAT_00abde88
and how the Rust renderer maps the level field to screen-Z are not resolved. Per burden of proof
(cannot independently confirm OBSERVABLE divergence), this stays UNCERTAIN, not REAL. Finder's
own confidence (UNCHECKED) is consistent.

## D3: VERDICT=REAL
Extent-probe cap. Live @ 0x00575ba0: both extent `do {...} while` loops have NO iteration counter
— they terminate only on overlay `< 0xcd` (break) or `>= 0xe9` (loop exit). Current Rust
`measure_extent` (bridge_orchestrator.rs:858) is `for _ in 0..MAX_EXTENT_PROBE` with
`MAX_EXTENT_PROBE = 64` (:381). The cap exists in Rust and has no gamemd counterpart — a real
structural difference. Corrected delta: Rust stops counting an in-band span at 64 cells -> gamemd
counts uncapped; an asymmetric >64 span yields different `back`/`fwd` -> different bias/step/footprint.
Severity LOW: unreachable on real YR maps (no single straight in-band span > 64 cells). REAL but
unreachable in stock YR.

## D4: VERDICT=REAL
EW bias subtraction wraps X mod 65536 in gamemd, aborts in Rust. Live @ 0x00575870 (EW_High):
`uVar11 = iVar4 - (iVar13 - iVar12)/2` where `iVar4 = *param_1` is the FULL packed coord (X low16,
Y high16); then `local_1c = (ushort)uVar11` truncates to the low 16 — an X underflow borrow is
masked to 16 bits (X wraps mod 65536), Y unaffected, walker proceeds (off-map cells hit the
`&DAT_00abdc50` sentinel, overlay +0x44 reads 0 < 0xcd -> break). Confirmed EW walks X (extent
loops and walk step both mutate `local_1c`=X). Current Rust uses `step_axis_by(seed, axis, -bias)`
(bridge_orchestrator.rs:745) which returns `None` when X leaves `0..=u16::MAX`, and the walker then
`return Vec::new()` (0 cells collapsed). Finder's reading holds. Corrected delta: at the X=0 edge
with `seed_X < (back-fwd)/2`, Rust collapses 0 cells -> gamemd wraps X to ~0xFFFE and runs a short
mostly-off-map walk that breaks on the sentinel. Severity LOW: requires bridge+hut against column 0.

## D5: VERDICT=UNCERTAIN
Fallback ramp-walk primitive. Live @ 0x00574000 confirms the fallback arm (after the X-major 5x5
overlay scan misses): reads cell flags `+0x140 & 0x500`, runs an 8-direction `g_DirectionOffsets`
sweep (up to 3 cells/dir), branches on anchor flags `0x100`/`0x400`/`0x800`/`0x80`, then ramp-walks
via `MapClass__IsBridgeRampTile` / `MapClass__IsLowBridgeEndpointTile` calling
`ApplyDamageToCell(&...)` up to 3x per ramp hit (`do { ApplyDamageToCell; if !=0 break; } while
(iVar9 < 3)`), and the `0x400`-anchor walk bails at `if (3 < local_20) return;` (4 steps). This is
structurally the FULL damage state machine (`ApplyDamageToCell`), distinct from Rust
`run_hut_fallback_plan`/`apply_hut_damage_to_cell` (bridge_orchestrator.rs:553,648). Rust's
`resolve_pure_bridgehead_anchor` (:538) bails at `continuations >= 4`, matching the 4-step cap
direction. The primitive mismatch and the divergent flag-mapping (`+0x140 & 0x500` <-> Rust
`BRIDGE_FLAG_STRUCTURAL|DESTROYED_OR_RAMP`) are real and identified, BUT per-cell output equality
was NOT walked to a single divergent fixture this session — so I cannot assert REAL output drift.
Stays UNCERTAIN (finder's own UNCHECKED). Trigger frequency: only no-overlay-in-5x5 hut placements
(rare; stock CABHUT sits on/beside the deck overlay -> overlay-seed path, not fallback).

## D6: VERDICT=REFUTED (not a disparity — PARITY)
The finder itself recorded D6 as PARITY-CONFIRMED. Live verification agrees: @ 0x00575ba0 the
per-cell pre-destroy RNG order is [jitterX, jitterY, then (gated on operator_new!=0) Random(1,5),
Random(0, RulesClass+0x168 -1)]; no metallic-debris draw in the walker (metallic is only in
`CellClass::BlowUpBridge`, a different facet). For stock YR `BridgeExplosions.Count = 4`,
`Random(0,3)` draws in both; the `Count==1` degenerate is `Random(0,0)` which @ 0x0065c7e0
returns early consuming NO draw, matching Rust `next_range_u32_inclusive` lo==hi early-out
(rng.rs:137). Output-identical for the reachable stock cases. No drift.

---

## Spot-checks of PARITY-CONFIRMED items (independent re-confirmation)
- #1 5x5 X-major scan: @0x00574000 outer `iVar9=-2..<3` mutates X (`*param_2 + iVar9`), inner
  `iVar8=-2..<3` mutates Y. X-major, first-match-return. Matches Rust `hut_destroy_5x5_scan`. OK.
- #5 `local_2c = 4` step count: confirmed `local_2c = 4` in NS_High and EW_High. Matches
  `MAX_HUT_SWEEP_STEPS = 4`. OK.
- #6 3-retry break-on-success: confirmed `do { cVar = DestroyBridge_High; if (cVar != 0) break;
  iVar10++; } while (iVar10 < 3)`. Rust breaks only on `Collapsed{binary_success:true}` via
  `apply_damage_success` (mod.rs:417). OK.
- #7/#8 step tiebreak + bias: confirmed `if (iVar10 < iVar11) local_14 = -1;` (default +1, fwd==back
  -> +1) and start = `(uVar9>>0x10) - (iVar11 - iVar10)/2` (NS) / `iVar4 - (iVar13 - iVar12)/2` (EW),
  signed truncating div. Matches Rust `step`/`bias` (bridge_orchestrator.rs:742-745). OK.
- #9 extent off-by-one absorbed: confirmed both extent loops `iVar++` BEFORE the band check, so each
  count includes the terminator cell; the +1 cancels in `(back-fwd)/2` and `fwd<back`. Algebraic
  parity holds. OK.
- #12 RandomRanged equal-bounds early-out: confirmed @0x0065c7e0. OK.
- #13 OnHutDeath callers live YR: `get_function_callers 0x00574000` = `BombClass__Detonate` +
  `BuildingClass__Update` only; no SpecialFlags/TS gate. Not TS-legacy. OK.

## NEW disparities the finder missed
- MISS (NONE-confirmed at REAL level): No new player-observable disparity found in this facet that
  the finder did not already enumerate (the U1-U4 UNCHECKED items remain open but are correctly
  flagged). Specifically re-examined: the walk-step termination's `local_1c = (short)param_1`
  bookkeeping inside the `||` short-circuit (NS_High) is pure caching, no output effect — NOT a miss.
- MISS (low, latent): in `spawn_hut_walker_pre_destroy_effects` the Rust early-return on empty list
  (D1) ALSO skips the `terminal-cap` and `perpendicular triplet` traversal entirely, but since the
  only side effects in that traversal are the RNG draws + WorldEffect pushes (both empty-list-gated
  in gamemd's *visible* output too — gamemd still draws RNG but constructs no anim), the only
  divergence is the RNG-stream one already captured by D1. No separate disparity.
