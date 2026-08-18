# Adversarial verdicts — render-warhead-sound facet

**Date:** 2026-05-29
**Auditor stance:** adversarial skeptic — DRIFT only if observable AND finder's live gamemd reading holds.
**Live gamemd this session:** `Apply_area_damage @ 0x00489280`, `FUN_00547230 @ 0x00547230`
(railings), `CellClass__DrawOverlay_Shadow @ 0x0047F510`, `CellClass__SetBridgeDirection_NESW @ 0x0047E040`,
`CreateRadarEvent @ 0x0065FA70`, `TacticalClass_Draw @ 0x006D3D10`. Static reads of `0x00abc210`/`0x00abc2d0`.
**Rust re-read:** `bridge_orchestrator.rs:1397-1474`, `bridge_state/mod.rs:836-970`,
`bridge_railing_atlas.rs:125-160`, `app_render/draw_passes.rs:63-230`, `app_instances/bridges.rs:255-296`,
`world_orders.rs:250-379`, `app_sim_tick.rs:564-612`.

---

D1: **REFUTED** — finder's RNG-draw-count fixture is wrong on BOTH sides. The finder
(D2 UNCHECKED) never read `path_matches_cell`; I read it (`bridge_state/mod.rs:836-905`).
For a high-body cell with overlay `0xCE`: `HighStateMachine` is explicitly **rejected**
at lines 854-857 (overlay in `0xCD..=0xE6` direct window → false), `LowStateMachine` and
`LowDirect` also false, only `HighDirect` matches → exactly ONE RNG draw. Live
`Apply_area_damage @ 0x00489280` routes the SAME `0xCE` plain-body cell to exactly ONE
block too: the high-SM block (`LAB_00489f27`) is gated by a ramp-tile-class test that
FAILS for a plain body (no ramp class), the low-SM block (`LAB_0048a0a5`) tile-class also
fails, low-direct needs `OverlayTypeIndex < 100` (206 is not), and only the high-direct
window (`0xcc < 0xCE < 0xe7`) fires → one `Random__RandomRanged`. Both sides: one draw,
one path. The finder asserted gamemd's high-SM block fires for a bare `0xCE` (it does NOT
without a ramp tile-class) and that Rust's HighStateMachine matches `0xCE` (the code
explicitly rejects it). The per-path scan structure is benign because the SM-vs-direct
match sets are made disjoint by the same overlay-window exclusions gamemd uses. No
double-draw exists in the cited fixture.

D2: **REFUTED** — `path_matches_cell` (read this session, `bridge_state/mod.rs:847-905`)
DOES mirror gamemd's split: SM paths reject the raw direct-overlay windows (`0xCD..=0xE6`
high, `0x4A..=0x63` low) before considering role, so plain body cells carrying a live
direct-destroy overlay route to the direct walker — exactly as `Apply_area_damage`'s
ramp-tile-class gate at `LAB_00489f27` shunts a non-ramp body overlay past the SM blocks
to high-direct `LAB_0048a214`. The Rust uses `deck_level >= 4` + role + a `±1` impact-z
window as its SM discriminator rather than reading the live ramp-tile-class globals
(`DAT_00aa0e28`/`DAT_00abad30`/`DAT_00aa1028`); that is a different internal mechanism, but
for the routing decision (does this cell go SM or direct?) it produces the same bucket on
the overlay windows the finder's own fixture used. No proven output divergence; the
finder's D2 was self-labeled UNCHECKED and the unread helper turns out correct.

D3: **REFUTED** (finder already self-downgraded to PARITY) — confirmed live. SM blocks at
`LAB_00489f77`/`LAB_0048a0a5` wrap `ApplyDamageToCell()` in `iVar=3; while(cVar7=='\0'){if
(bVar21||iVar<1)break; ApplyDamageToCell(); iVar--;}` (4 attempts on non-IonCannon path,
`bVar21 = param_4 != Rules+0xff0`); direct blocks `LAB_0048a214` call
`DestroyBridge_Low()`/`DestroyBridge_High()` once, no `while`. Rust `max_attempts = if
is_ion_cannon && path.is_state_machine() {4} else {1}` (`bridge_orchestrator.rs:1429`)
matches. NOT a disparity.

D4: **REAL** (conclusion stands; finder's mechanism description is WRONG and corrected
here) — `FUN_00547230 @ 0x00547230` does NOT use a separate wood value-table at
`DAT_00aa1098`. `DAT_00aa1098` is a tile-index *range base*; the else-branch maps BOTH the
concrete range (`DAT_00abc1f8`) and the wood range (`DAT_00aa1098`) into the SAME value
table `DAT_00abc210`/`+4`/`+8`/`+c` (`iVar2 = (iVar4 - iVar2) * 0x10`). gamemd's actual
second railing table is `DAT_00abc2d0`, reached by a DIFFERENT path (the `+0x2e1` flag +
`DAT_00aa102c` 16-entry range scan), which the finder did not identify. The Rust
`CONCRETE_RAILING_VALUES == WOOD_RAILING_VALUES` (bridge_railing_atlas.rs:158-160) is
therefore actually FAITHFUL to the else-branch (one table for both ranges) — that is NOT
the bug. The REAL drift: static reads of `0x00abc210` AND `0x00abc2d0` are both
**zero-filled** (theater-loaded at runtime), so the Rust hardcoded slot-4 `(13,6,48,12)`
and slot-6 `(14,1,48,12)` values are invented placeholders that cannot equal the real
runtime table; and the `DAT_00abc2d0`-path railings are entirely unmodeled. Corrected
delta: **Rust placeholder 10-entry table (2 slots) → gamemd two distinct runtime-loaded
tables (`DAT_00abc210` else-branch shared by concrete+wood ranges, `DAT_00abc2d0` for the
`0x2e1`/`DAT_00aa102c` path), both zero in the static image.** Values not statically
recoverable; needs live-debugger theater-load capture.

D5: **REAL** — confirmed live. `TacticalClass_Draw @ 0x006D3D10` runs the terrain bundle
(`Tactical_layer_base_terrain → _smudges → _building_overlays → _overlays → _animations`)
and only AFTERWARD (the `param_3==2||3` block) calls `Tactical_ObjectRenderingLoop()`.
Railings emit inside `Tactical_layer_overlays` → before the object loop. Rust submits
railings at draw_passes.rs Step 7.5 (line 224) AFTER `draw_merged_object_pass` (Step 5,
line 156). Real emission-order inversion. Cite drift: finder's `:174` is stale; the object
merge is at `:156`, railing submit at `:224`. Delta unchanged: **Rust railings after
object merge → gamemd railings in terrain bundle before objects.**

D6: **REAL** — confirmed live. `CellClass__DrawOverlay_Shadow @ 0x0047F510` ends in
`CC_Draw_Shape(piVar5, frame, ..., 0x4601, 0, cVar1*-0xf-2, 0, 1000, ...)` — a real shadow
blit on the bridge shadow pass. Rust builds `build_bridge_shadow_instances`
(bridges.rs:210-291) but draw_passes.rs Step 2.5 (lines 72-79) is explicitly DISABLED
("bridge body shadow (DISABLED)"). Bridges render with no cast shadow. Delta unchanged.

D7: **REAL** — confirmed live. Shadow Z param to `CC_Draw_Shape` is `cVar1 * -0xf + -2`
where `cVar1 = *(param_1 + 0x11b)` = cell level → `level*-15-2`, NO `+4` bridge bonus.
Rust shadow builder (bridges.rs:273) uses `z.saturating_add(BRIDGE_HEIGHT_BONUS=4)` (the
+4 body bonus), wrong for shadows. Latent until D6 re-enabled. Delta unchanged: **Rust
shadow depth_z = z+4 → gamemd shadow Z basis = level (no +4).**

D8: **REAL** — confirmed live. `CellClass__SetBridgeDirection_NESW @ 0x0047E040` calls
`RadarClass__MarkTerrainDirty(&cell->MapCoord_X)` for the anchor and each stepped neighbor
(4 sites in the body), and for `param_3==0` (collapse) also `BlowUpBridge` per cell. Rust
collapse (`bridge_orchestrator.rs:332-337`) does fallout + `update_adjacent_bridges` +
`refresh_bridge_zones` but no radar/minimap-dirty channel; minimap.rs keeps the static
intact-bridge color. Delta unchanged: **collapsed cells keep intact-bridge minimap color
indefinitely → gamemd recolors them via MarkTerrainDirty on next radar refresh.**

D9: **REFUTED** — finder read the WRONG file. The sim-side repair logic at
`world_orders.rs:356-372` DOES `self.radar_events.push(RadarEventType::BridgeRepaired,
brx, bry, ...)` BEFORE bridge mutation (Step A0), captures the dedup boolean into
`eva_allowed`, and threads `eva_allowed` into the sound event; `app_sim_tick.rs:588` then
gates the EVA cue on `eva_allowed && local-human-owner`. This mirrors live
`CreateRadarEvent @ 0x0065FA70` (decompiled: returns 0 when within the type's dedup radius
of a live same-type event, else 1 — the `AL` dedup return) created before mutation with
EVA gated on the return. Dedicated tests at `world_orders_bridge_repair_tests.rs:592-660`
assert the push, the CABHUT-cell dedup, and the `eva_allowed` gate. The finder cited only
`app_sim_tick.rs:564-607` (the playback consumer) and missed the producer in
`world_orders.rs`. Already correct.

---

## NEW disparities the finder missed

MISS: **None confirmed as a new REAL disparity this pass.** Spot-checks of the areas the
finder marked PARITY-CONFIRMED (Latin-square gate, body Y-offset, IonCannon bypass/retry,
BridgeStrength roll direction `roll < damage`, effective_render_state overlay→state map at
bridge_state/mod.rs:944-970) held up against the live `Apply_area_damage`/`DrawOverlay_*`
reads. Two finder-internal corrections worth noting (not new disparities, but they
invalidate finder reasoning): the D1/D2 "double RNG draw" risk does not exist because
`path_matches_cell`'s SM paths exclude the direct-overlay windows; and the D4 "separate
wood table at `DAT_00aa1098`" mechanism is a misread — `DAT_00aa1098` is a tile-index
range base sharing the `DAT_00abc210` value table, the real second table is `DAT_00abc2d0`.

UNCERTAIN-CARRYOVER: exact theater-loaded railing values (D4) remain statically
unrecoverable (both table roots zero in the image) — agrees with finder; needs live capture.
