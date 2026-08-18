# Adversarial Verdicts — Locomotion-Height (on-bridge vs under-bridge locomotion, height, render/Z offsets)

Auditor stance: refute until the live binary + current Rust prove the disparity. Default to DRIFT only when the
finder's gamemd reading holds live AND equivalence is unproven. Mark UNCERTAIN when I cannot independently confirm.

**Live gamemd re-decompiles this session (all addresses re-confirmed to resolve to the named function):**
- `ObjectClass__ShouldBeOnBridge @ 0x005f6a70` — `DAT_00ac13c8 * 3 < (groundDest - groundCur)` twice (set + clear).
- `FootClass__ShouldBeOnBridge @ 0x004ddc40` — gate on `+0x684` sign, delegate to ObjectClass.
- `DriveLocomotionClass__ComputeBridgeZOffset @ 0x004af4a0` — `g_BridgeZOffset_Drive = ftol(g_DriveHeightStep * 4)`.
- `ShipLocomotionClass__Compute_BridgeZOffset @ 0x0069ebb0` — `g_BridgeZ_Offset = ftol(g_ShipHeightStep * 4)` (separate global).
- `FootClass__Set_Height_On_Bridge @ 0x005f5fa0` — `+= DAT_00ac13bc` under +0x23 (on_bridge) guard, GetGroundHeight rebase.
- `CheckBridgeTraversal @ 0x004d9c60` — diff abs 0/1/4-else-7 branch tree, `+0x11b+4` seed, `+0x11c` slope, `0x100/0x200`.
- `IsOnBridgeRamp @ 0x00578d80` — tile-index range membership + directional (`param_2 in {0,1,3,4}`) endpoint sub-checks.
- `CellClass__IsOnBridgeSurface @ 0x00485060` — `DAT_00aa0738 <= cell+0x38 < +0xe` (14-tile window).
- `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073b140` — TooBig + IsOnBridge_ForFiring + neighbor==0 → split-blit (upper -5 / lower 16x16 mode-2).
- `DriveLocomotionClass__Set_Destination @ 0x004afd40` — unconditional `dest.Z += g_BridgeZOffset_Drive` when dest cell `0x100`.
- `DriveLocomotionClass__Process_Drive_Track @ 0x004b0f20` (body 0x4b0f20-0x4b2607) — contains ALL of: approach-Z recompute,
  on_bridge transition (LAB_004b1837/183f), TooBig crush layer-pick + 10000/20 dmg, scatter case-6 `>= 3` pick.

**Current Rust re-read:** `movement_bridge.rs` (full), `movement_occupancy.rs` (full), `occupancy.rs` (full),
`pathfinding/core.rs:399-592, 1483-1527` (helpers, A* threshold, check_bridge_traversal), `components.rs:30-47` (Position.z u8),
`bump_crush.rs` + whole `src/sim/movement/` (grep for 10000/RulesClass/TooBig crush), `src/render/` (grep for bridge fields).

---

## D1: on_bridge transition predicate — VERDICT=REAL (downgraded scope; gamemd reading holds, but the residual is narrower than "MED")

Live `Process_Drive_Track @ 0x004b0f20` LAB_004b1837/LAB_004b183f confirms the finder's gamemd table exactly:
```
if (cell+0x11b == other+0x11b - 4) {           // descending diff == -4
    if (cell+0x140 & 0x100) { +0x8c = 1; }     // new IS bridge -> set on_bridge, SKIP clear
    else if (other+0x140 & 0x100) { +0x8c = 0; }
} else {                                        // any other diff
    if (cell+0x140 & 0x100) { /* new IS bridge: no change */ }
    else if (other+0x140 & 0x100) { +0x8c = 0; } // old WAS bridge, new not -> clear
}
```
So: set fires only on diff==-4 & new-bridge; clear fires for `old-bridge & new-NOT-bridge` at ANY diff. Current Rust
`compute_bridge_transition` (`movement_bridge.rs:88-102`): Enter = `dst_h == src_h-4 && dst.has_structural_bridge()`;
Exit = `!dst.has_structural_bridge() && src.has_structural_bridge()`. Enter matches the set row. Exit's `src.has_structural_bridge()`
is the load-bearing re-derivation of `old & 0x100`: gamemd keys on the raw `0x100` cell flag, while Rust keys on
`bridge_structural` (= `bridge_walkable && !transition`, per `movement_bridge.rs:238` test ctor and core.rs:1496). A high-bridge
RAMP tile carries `0x100` in gamemd (CheckBridgeTraversal `+0x11b+4` deck math) but has `bridge_structural=false` in Rust. So at a
body->ramp step gamemd sees new-IS-bridge (no clear) and Rust's Exit also does not fire (dst ramp `has_structural_bridge`=false ->
`!false`=true... but src body `has_structural_bridge`=true -> Exit WOULD fire). This is the genuine divergence the finder flagged:
ramp `0x100` vs Rust `bridge_structural` are NOT the same predicate at ramp cells.
**Corrected delta:** Rust Exit keys on `src.bridge_structural` (body-only); gamemd clears on `src & 0x100` (body OR ramp/bridgehead).
At a body->ramp transition, gamemd does NOT clear (ramp still `0x100`) but Rust Exit fires Clear (src body structural, dst ramp not).
Observable: 1-tick premature on_bridge=0 / layer flip when a unit steps from bridge body onto a connected ramp. REAL but narrow —
fires only at the body<->ramp boundary, not every bridge crossing.

## D2: Set_Destination bridge Z-bump + approach-Z recompute — VERDICT=REAL

`Set_Destination @ 0x004afd40` (live): unconditional `dest.Z(+0x38) += g_BridgeZOffset_Drive` when `dest cell+0x140 & 0x100`,
no height-diff guard, only `0x100` consulted. `Process_Drive_Track @ 0x004b0f20` top: `uVar20 = -(cell&0x100!=0) & g_BridgeZOffset_Drive;
uStack_e0 = GroundHeight + uVar20;` then `Sqrt(dx^2+dy^2+dz^2)` for the brake-distance / deceleration ramp. Both present live.
Rust `drive_locomotion.rs` has no bridge Z math; `update_drive_speed_fraction` uses flat distance_to_goal with no Z term; Z is set
only at the cell boundary by `movement_bridge.rs`. **REAL.** Mechanism absent. Observable effect is sub-frame brake-timing on bridge
approaches (deck offset is small), but it is a real determinism delta in the brake ramp on every bridge approach.

## D3: ShouldBeOnBridge `*3` lepton threshold — VERDICT=REAL

`ObjectClass__ShouldBeOnBridge @ 0x005f6a70` (live): both arms gate on `DAT_00ac13c8 * 3 < |groundDest - groundCur|`
(LeptonsPerLevel * 3 = a strict 3-LEVEL lepton threshold). `FootClass__ShouldBeOnBridge @ 0x004ddc40` delegates after a `+0x684`
sign gate. Rust has no ShouldBeOnBridge predicate; nearest gates are `is_at_bridge_level` (`core.rs:411`, `>= 2`) and the runtime
layer pick (`movement_occupancy.rs:152`, `>= 2`), plus the D1 Enter `== -4`. None reproduce the strict `> 3-level` (leptons)
classifier. **REAL.** Delta: gamemd's "am I high enough to be on the deck" = `unit/dest ground diff > 3 levels (in leptons)`;
Rust uses `>= 2 levels` for the layer pick and `== 4` for the transition. Boundary differs in the 2-3-level band on ramp traversal.

## D4: Bridge Z-offset rounding / per-locomotor leptons — VERDICT=UNCERTAIN

The Drive `*4` and Ship `*4` are confirmed live (`0x004af4a0`, `0x0069ebb0`) and write SEPARATE globals from distinct height_step
values — that part of the finder's reading holds. BUT the decompiles do NOT show the `+0.5` round-half-up term (it is folded into
`Math__ftol`), and I did not independently verify the Walk `FUN_006D2120` round-half-DOWN (`-0.5`) claim live this session — that
rests on the finder's cited WALK doc, not a re-decompile. Rust uses exact `+4` LEVELS (u8) everywhere with no leptons and no rounding
(`movement_occupancy.rs:39`, `core.rs:450`). A units-and-rounding drift is plausible and the Drive/Ship separate-global fact is real,
but the specific round-up-vs-round-down 1-lepton split is doc-sourced, not binary-confirmed this session, and the runtime height_step
values are BSS-zero in the static dump. **UNCERTAIN** — the unit mismatch (levels vs leptons) is real but the 1-lepton rounding-split
delta is not independently proven live.

## D5: runtime layer thresholds (three distinct gamemd sites) — VERDICT=REAL

`Process_Drive_Track @ 0x004b0f20` confirms all three live:
- A* closed-list `>= 2` — matches Rust `BRIDGE_HEIGHT_THRESHOLD=2` (`core.rs:401,411`). PARITY for this site (finder agrees).
- TooBig crush layer-pick: at the `iVar8+0xc94` block, when on_bridge byte (`+0x23`)==0 it tests
  `(GetGroundHeight + g_BridgeZOffset_Drive) <= unitZ(+0x29)` -> bridge list `+0xE8`, else ground list `+0xE4`. This is a
  `>= ground + ~4-levels (leptons)` compare, NOT `>= 2`. Confirmed live.
- Scatter case-6: `uVar20 = unitZ / g_DriveHeightStep - cell+0x11b; if (abs(uVar20) < 3) skip-scatter`. Fires at `>= 3 levels`.
  Confirmed live.
Rust uses a single `>= 2` for layer selection (`movement_occupancy.rs:152`) and lacks the `>= 4-lepton` crush pick (it lives only
inside the absent crush, see D6) and the `>= 3` scatter pick. **REAL.** Delta: in the 2-3-level band a tall/scatter unit is assigned
the opposite occupancy layer vs gamemd. (A* `>= 2` site is parity, correctly excluded.)

## D6: TooBigToFitUnderBridge runtime crush + 10000/20 self-damage — VERDICT=REAL

`Process_Drive_Track @ 0x004b0f20`, TooBig block (`iVar8+0xc94 != 0`, `+0x1b4 == 0`): selects layer list by the D5 crush pick,
walks it, and for each non-crushable occupant calls vtable+0x16c with `iStack_c4 = 10000` (warhead `g_RulesClass_Instance+0xfa8`),
then `uStack_e0 = 0x14` (20) self-damage on the crusher. Confirmed live exactly as the finder stated. Rust: grep of the entire
`src/sim/movement/` for `10000`/`RulesClass`/`TooBig` crush returned NO matches; `movement_path.rs:46` explicitly comments gamemd
does not gate movement on TooBig, and bump_crush carries the flag only for blocker/neighbor logic. **REAL.** The signature
oversized-unit deck-crush (10000 dmg) + 20 self-dmg is absent. Render BridgeFudge split-blit (`0x0073b140`) is also absent (D11).

## D7: CheckBridgeTraversal branch equivalence — VERDICT=REAL (but much closer than finder's "structurally different")

Re-read `CheckBridgeTraversal @ 0x004d9c60` live AND Rust `check_bridge_traversal` (`core.rs:506-592`). Contrary to the finder's
"structurally different" framing, the Rust port is a near-1:1 transcription: same diff-abs 0/1/4-else-blocked tree, same
`signed_level()+4` seeds (`+0x11b+4`), same directional slope gate (`diff<1 -> parent.slope_type` / else `candidate.slope_type`,
mirroring `uVar3<1 -> param_5+0x11c` / `param_1+0x11c`), same `0x100/0x200` -> `has_structural_bridge`/`has_bridgehead_transition`,
same `*param_4=1` -> `force_bridge_list`. I walked the diff==4 arm both ways: gamemd `if (param_5+0x11b == iVar5-4) {...} if (iVar5 ==
param_5+0x11b-4) { require param_1 0x100&0x200; *param_4=1; return 0; }` maps to Rust's two `if parent.signed_level()==candidate-4` /
`candidate==parent-4` branches. The residual REAL concern: gamemd's diff==1 ramp gate reads the raw `+0x11c` slope byte of the LOWER
cell, and Rust's `slope_type` is set upstream at terrain-resolve time — if the resolver mis-maps `+0x11c` for any tile, the gate
diverges. That is an upstream-resolver risk, not a transcription bug in this function. **REAL but LOW** — keep as a tile-data
verification item, not a logic disparity. The function-level port is faithful.

## D8: IsOnBridgeRamp / IsOnBridgeSurface tile-index ranges — VERDICT=UNCERTAIN

`IsOnBridgeRamp @ 0x00578d80` (live): exactly the tile-index range tests over `DAT_00aa1020..+0x28`, `DAT_00aa073c..+4`,
`DAT_00abb110..+4`, `DAT_00aa1050..+4`, `DAT_00aa10a0..+4`, `DAT_00abbebc..+0x14` with directional (`param_2 in {0,1,3,4}`) endpoint
sub-checks. `IsOnBridgeSurface @ 0x00485060` (live): `DAT_00aa0738 <= cell+0x38 < +0xe` (14-tile window). Finder's reading is exact.
Rust classifies via `transition`/`bridge_structural`/`slope_type` set at terrain resolve, NOT a tile-index window. Whether the Rust
terrain resolver reproduces these exact ranges + directional endpoint sub-checks is OUTSIDE the audited files and not traced this
session. **UNCERTAIN** — gamemd side verified; Rust equivalence unconfirmed (resolver not inspected). Cannot mark REAL without showing
a concrete tile whose Rust flag diverges, nor REFUTED without proving the resolver covers all ranges.

## D9: Set_Height_On_Bridge anim-Z rebase (`DAT_00ac13bc`) — VERDICT=REAL

`FootClass__Set_Height_On_Bridge @ 0x005f5fa0` (live): `if (+0x23 != 0) param_2 += DAT_00ac13bc;` then rebases
`+0x29 = GetGroundHeight(coord) + param_2` (with a `+0x1d`-gated vtable+0x124 reposition variant). `DAT_00ac13bc` is a distinct global
from LeptonsPerLevel `0x00ac13c8`. Rust has no Set_Height_On_Bridge equivalent and no bridge-add for attached anims. **REAL** but
narrow — fires only for an anim attached to a foot unit on a bridge deck (muzzle/attachment positioning), not the per-tick locomotion
Z. Observable as a slightly-wrong attachment-anim elevation on bridge-standing infantry.

## D10: Ship under-bridge Z constant `360` vs `ftol(g_ShipHeightStep*4)` — VERDICT=UNCERTAIN

`ShipLocomotionClass__Compute_BridgeZOffset @ 0x0069ebb0` (live) confirms Ship's offset is `ftol(g_ShipHeightStep * 4)`, a SEPARATE
global from Drive's `ftol(g_DriveHeightStep * 4)` (`0x004af4a0`). Rust hardcodes `BRIDGE_Z_OFFSET = 360` (=90*4) in
`movement_bridge.rs:74`. The finder could not obtain the runtime `g_ShipHeightStep` value (BSS-zero in static dump) and neither can I
this session. If `g_ShipHeightStep == 90` leptons/level at the standard view angle, 360 is exact; if not, it drifts. The
separate-global fact is real, but the numeric mismatch is unproven. **UNCERTAIN** — needs a post-map-load runtime read of
`g_ShipHeightStep` (and `g_DriveHeightStep`) to settle.

## D11: render does not consume on_bridge / bridge_occupancy / deck Z; BridgeFudge split-blit absent — VERDICT=REAL

`UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073b140` (live) confirms the TooBig + `IsOnBridge_ForFiring` + neighbor-count==0 trigger
and the split-blit (upper full pass priority-5 via vtable+0x2ec(0,...)-5; lower 16x16 strip mode-2 via vtable+0x2ec(2,...)), gated on
`DAT_00b1cfcc > 0x10`. Grep of `src/render/` for on_bridge/bridge_occupancy/deck_level/BridgeFudge/split-blit/ZFudge returned NO
matches. **REAL** — render does not lift unit sprites to deck height, has no split-blit, no ZFudge family. Per CLAUDE.md this is a
render-facet item; logged here because the deck `position.z` this facet produces is the input render should consume and doesn't.

---

## NEW disparities the finder missed in this facet

- **MISS [LOW/REAL]:** `Set_Height_On_Bridge @ 0x005f5fa0` has a `+0x1d`-gated branch that, before rebasing Z, calls vtable+0x124(0)
  (un-mark / mark-for-redraw reposition) and re-reads the coord via the relocated position, then calls vtable+0x124(1) to re-mark.
  The finder described only the Z math (`+= DAT_00ac13bc`, GetGroundHeight rebase) and not this mark/unmark-redraw bracket. If `+0x1d`
  is the in-cell-relink/redraw state, the anim attach on a bridge unit also triggers a redraw-region update that Rust does not. Minor,
  same trigger frequency as D9 (anim on bridge-standing foot unit).
- **MISS [LOW/REAL]:** `Process_Drive_Track @ 0x004b0f20` arrival path (`iStack_7c==0 && iStack_78==0 && iVar7!=0`) contains a
  bridge-aware re-target abort: `if (abs(destZ(+0x3c) - cellZ) < g_DriveHeightStep * 2) clear destination`. This is a `2-height-step`
  (leptons) Z-tolerance on track-chain arrival when re-targeting via the `+0x5a4` helper. Rust arrival/track-chain logic does not model
  a Z-tolerance abort. Narrow (fires on track-chain re-target near a bridge), but it is a distinct lepton threshold (`* 2`) not in the
  finder's three D5 thresholds.
- **MISS [INFO]:** D5/D6 crush pick reads on_bridge byte (`+0x23`) FIRST — `if on_bridge: use +0xE8 (bridge) directly`, only the
  `on_bridge==0` path runs the `unitZ >= ground+offset` compare. The finder's D5 summary collapsed this to the height compare; the
  on_bridge-byte short-circuit means a unit already flagged on_bridge always crushes the bridge layer regardless of Z. Folds into D6
  (crush absent) so no separate fix, but the layer-pick precedence (on_bridge byte before Z compare) should be reproduced when D6 lands.

---

## Summary of changes to finder's verdicts
- D1: kept REAL, corrected the delta to the precise body->ramp Exit case (`src.bridge_structural` vs `src & 0x100`); the finder's
  LIKELY-DRIFT hedging is resolved to a concrete REAL divergence at the body<->ramp boundary only.
- D4: downgraded PROVEN-DRIFT -> UNCERTAIN (round-up/round-down 1-lepton split is doc-sourced, not re-decompiled live; the
  levels-vs-leptons unit mismatch is real but the specific rounding delta is unproven this session).
- D7: kept REAL but corrected the finder's "structurally different" claim — the Rust port is a faithful near-1:1 transcription; the
  only residual is the upstream `+0x11c` slope_type resolver, demoted to LOW.
- D2, D3, D5, D6, D9, D11: confirmed REAL with live binary evidence.
- D8, D10: kept UNCERTAIN (gamemd verified; Rust/runtime side not confirmable this session).
- PARITY-CONFIRMED items (A* `>=2`, deck=+4 levels invariant, layer-tagged occupancy, JumpJet/Hover no Z-bump, DropPod/Tunnel absent)
  spot-checked against `occupancy.rs` and `core.rs:411` and the live `*4` ComputeBridgeZOffset — consistent, no objection.
