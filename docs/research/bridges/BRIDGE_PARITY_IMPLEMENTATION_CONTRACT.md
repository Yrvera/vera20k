# Bridge System — Parity Implementation Contract

**Date:** 2026-05-29
**Goal:** bring the Rust bridge system to 100% observable parity with gamemd.exe.
**Method:** 16-facet finder + adversarial-skeptic scan (loop round 1 + adversarial verify).
Every gamemd claim below was re-decompiled live this session **except** where tagged `[DOC]`
(the Ghidra MCP dropped partway through the repair-walker / debris-explosion re-run; those rest
on `[ghidra/verified]` corpus docs and must be re-confirmed live before code lands).

This is a **research deliverable only — no Rust was edited.** The fix phase is opt-in and gated
on user approval (see §7 disjoint-ownership plan).

## Confidence legend
- `[LIVE]` — gamemd side re-decompiled live this run by both finder and skeptic; Rust side read at cited line.
- `[DOC]` — gamemd side corroborated by a verified-Ghidra doc only (MCP outage); re-decode before fixing.
- `[UNCERTAIN]` — equivalence/divergence not provable this run; needs a specific live read first (listed in §5).
- `[BSS]` — exact numeric constant is runtime-initialized (zero in the static image); needs a post-map-load debugger read.

Per-hole evidence cites the function `@addr` and the skeptic's verify call. Rust cites `file:line`.

## Live re-verification (2026-05-29 session, Ghidra restored via `connect_instance` TCP)

The `[DOC]`/`[UNCERTAIN]`/`[BSS]` determinism holes were live re-decoded. New reports under `docs/research/`:
- **BR-01 / BR-17 / BR-18** — `APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW_GHIDRA_REPORT.md` (COMPLETE). 4 blocks A/B/C/D, no early-out, one `RandomRanged(1,BridgeStrength)` from `Scenario+0x218` per eligible non-Ion block; lepton Z-window `(Level-2)·step+base < z ≤ (Level+1)·step+base`, lower exclusive, `Flags&0x100`-gated; `step = DAT_0089E870` (nominal 104, **debugger-deferred**), `base = 2·step`.
- **BR-02 / BR-19** — `APPLY_DAMAGE_TO_CELL_OVERLAY_FIRST_ROUTING_GHIDRA_REPORT.md` (COMPLETE). `ApplyDamageToCell` overlay-first (`0x4A..=0x63`→Low, `0xCD..=0xE6`→High) then SM; A/B fall through to C/D (duplicate draw site). High/Low routing by iso-tile-index set + neighbor overlay `0x18/0x19`÷`0xED/0xEE`, NOT `deck_level>=4`. Tile-base globals **debugger-deferred**.
- **BR-06 / BR-07** — `FUN_00598030` live: rejection-sampled `RandomRanged` (high bits, not `draw&3`) drawing from `g_MapGenRng @ 0x00abe890` (callers = `RepairBridgeWalker_*`). Repair variant must use a SEPARATE map-gen stream (Rust has only `scenario_rng`+`main_rng`).
- **BR-10** — `HIGH_BRIDGE_RIM_REFRESH_ALGORITHM_GHIDRA_REPORT.md` (PARTIAL: algorithm verified, runtime tile-table values deferred).
- **BR-15** — `BRIDGE_RAMP_PERPENDICULAR_HIGH_LOW_FAMILIES_GHIDRA_REPORT.md` (COMPLETE).
- **`[BSS]` sweep** — `BRIDGE_BSS_RUNTIME_CONSTANT_SWEEP_GHIDRA_REPORT.md` (PARTIAL: needs a post-map-load debugger capture; static init formulas recovered, e.g. `DAT_0089E864 = 2·DAT_0089E870`).

RNG routing truth: bridge binding is per-callsite ECX — damage dispatcher = `Scen->Random` (`scenario_rng`), repair variant = `g_MapGenRng` (new stream needed). See memory `reference_rng_instance_routing_truth`.

---

## 1. Severity ranking (fix order)

**TIER 1 — Lockstep / RNG determinism (multiplayer desync; fix first)**
- ✅ **BR-01 LANDED** (commit a05886e) dispatcher: one RNG gate + `break` vs all-4-blocks-no-early-out `[LIVE]`
- ✅ **BR-02 LANDED** (commit a05886e) dispatcher: `ApplyDamageToCell` overlay-first + duplicate Block C/D gate → 2 draws `[LIVE]`
- ✅ **BR-03 LANDED** (commit baf69a5) debris: outer 95% gate constant off by 2 `[LIVE]`
- ✅ **BR-04 LANDED** (commit baf69a5) debris: MetallicDebris 50% gate constant off by 1 `[LIVE]`
- BR-05 debris: metallic slot draw unconditional after gate+alloc (no `count==0` recheck) `[LIVE]`
- BR-06 repair: healthy-variant RNG uses low bits, gamemd uses high bits `[DOC]`
- BR-07 repair: repair variant may draw from the wrong RNG stream (`scenario_rng` vs `g_MapGenRng`) `[UNCERTAIN]`

**TIER 2 — High player-visibility**
- ✅ **BR-08 LANDED** (commit baf69a5) zones: bridge repair never re-activates the zone endpoint record (repaired bridge stays impassable cross-zone) `[LIVE]`
- ✅ **BR-09 LANDED** (commit baf69a5) body-SM: collapsed body anchor never clears `overlay_byte` → renders intact AND stays walkable `[LIVE]`
- BR-10 rim: entire rim-refresh algorithm wrong (stub-blank vs `UpdateBridgeEdgeTiles_*`) `[LIVE]`
- ✅ **BR-11 LANDED** (commit f914046) overlay-walkers: destroy walkers skip `Bridgehead`-role cells (ramps left standing) `[LIVE]`
- BR-12 bridgehead: healthy bridgehead collapses on 2nd direct hit; absorb writes slot+3 not slot+2 `[LIVE]`
- BR-13 fallout: ground force-kill bypasses death pipeline (no wreck/explosion/smudge/eject/score; voxel units vanish) `[LIVE]`
- ✅ **BR-14 LANDED** (commit baf69a5) fallout: ground-kill over-selects airborne units (kills aircraft flying over the collapse cell) `[LIVE]`

**TIER 3 — Medium**
- BR-15 body-SM: `update_ramp_perpendicular` writes no overlay/pavement, fires no `BlowUpBridge` `[LIVE]`
- ✅ **BR-16 LANDED** (commit 51295b8) collapse: minimap `MarkTerrainDirty` never fed by any collapse path (stale minimap terrain) `[LIVE]`
- BR-17 dispatcher: Z-window off-by-one + wrong unit frame `[LIVE]`/`[BSS]`
- BR-18 dispatcher: Z-window applied unconditionally vs structural-flag gated `[LIVE]`
- BR-19 dispatcher: High/Low SM routing uses `deck_level>=4` not tile-index + neighbor overlay `[LIVE]`
- BR-20 bridgehead: collapse overlay + level write (slot+3 @ `deck_height-4`) missing `[LIVE]`
- BR-21 debris: jitter offset `floor/2^31` (range −25..+24) vs `round((draw·scale−0.5)·50)` (−25..+25) `[LIVE]`
- BR-22 fallout/debris: debris/explosion anim Z = `deck_level` vs gamemd `0` `[LIVE]`
- BR-23 render: railing emission-order inverted (after object merge vs before, in terrain bundle) `[LIVE]`
- BR-24 render: bridge body shadow pass disabled `[LIVE]`
- BR-25 render: shadow depth Z = `z+4` vs gamemd `level` (no +4 bonus on shadows) `[LIVE]`
- BR-26 locomotion: on_bridge Exit keys on `src.bridge_structural` vs `src & 0x100` (body OR ramp) `[LIVE]`
- BR-27 locomotion: drive Set_Destination bridge Z-bump + approach-Z brake recompute absent `[LIVE]`
- BR-28 locomotion: `ShouldBeOnBridge` strict 3-level (lepton) threshold absent `[LIVE]`
- BR-29 locomotion: layer thresholds — crush pick (`ground+4` lep) + scatter (`>=3` lvl) vs single `>=2` `[LIVE]`
- BR-30 locomotion: TooBigToFitUnderBridge runtime crush + 10000 dmg / 20 self-dmg absent `[LIVE]`
- BR-31 locomotion: `Set_Height_On_Bridge` attached-anim Z rebase absent `[LIVE]`
- BR-32 render: unit sprites not lifted to deck Z; `Draw_Sprite_With_BridgeFudge` split-blit absent `[LIVE]`
- BR-33 tube: tube movement 1-cell/tick vs speed-gated fractional sub-cell slide `[LIVE]`
- BR-34 tube: exit facing `(tube.dir<<13)-0x8000` vs last-move-delta `[LIVE]`
- BR-35 tube: exit occupant scatter + retry vs unconditional snap (stacks) `[LIVE]`
- BR-36 tube: tube Z ramp `(GH(exit)-GH(entry))/path_len` vs per-cell stair-step `[LIVE]`
- BR-37 pathfinding: A* doesn't snap bridge source/dest to record endpoint (`ResolvePathCoord_BridgeAware`) `[LIVE]`
- BR-38 pathfinding: A*-tube-edge restricted to explicit tubes; `UpdateBridgePassability` peer-replay absent `[LIVE]`

**TIER 4 — Low / boundary / cleanup**
- BR-39 map-load: `walk_anchor_pattern` dir-6 slot 5 one cell short (`anchor+1E` dup of slot 4 vs `anchor+2E`) `[LIVE]`
- BR-40 zones: deactivation whole-group vs per-record geometric tol-3 `[LIVE]`
- BR-41 zones: endpoint pair = max-Manhattan heuristic vs own-coord + opposite-step `[LIVE]`
- BR-42 zones: 1 record per BFS group vs 1 per bridge tile (+ records frozen at init vs self-healing) `[LIVE]`
- BR-43 repair: dead `body_cell_repair_state` uses a 3rd RNG shape (cleanup/remove) `[LIVE]`
- BR-44 debris: `BRIDGE_EFFECT_FRAME_MS=67` vs 66.667 ms (15 fps logic frame) `[LIVE]`
- BR-45 hut: empty `BridgeExplosions` still draws 2 jitter/cell in gamemd (modded-only) `[LIVE]`
- BR-46 hut: `MAX_EXTENT_PROBE=64` cap has no gamemd counterpart (unreachable >64-cell span) `[LIVE]`
- BR-47 hut: EW bias X-underflow wraps in gamemd, aborts in Rust (column-0 boundary) `[LIVE]`
- BR-48 fallout: ground loop also `Take_Damage`s `TerrainClass` occupants; Rust only `EntityStore` `[LIVE]`
- BR-49 neighbor: EW classifier X=0/X=511 row-wrap reads `(511,Y-1)`/`(0,Y+1)` vs Rust 0 `[UNCERTAIN]`

**SPECIAL — see §4:** CABHUT-C4 "does nothing" bug — root cause UNIDENTIFIED (the ally-gate
hypothesis was refuted this run); needs a dedicated investigation, not a one-line fix.

**Clean (PARITY-CONFIRMED, do NOT touch) — see §6.**

---

## 2. TIER 1 — Lockstep / RNG determinism

### BR-01 — Dispatcher fires one RNG gate then `break`s `[LIVE]` — ✅ LANDED (a05886e)
- **Current:** `run_dispatch_loop` (`bridge_orchestrator.rs:1407-1474`) evaluates the 4 paths and
  `break`s at `:1473` after the first match → at most **1** `R(1,BridgeStrength)` draw per event.
- **Correct:** `Apply_area_damage @ 0x00489280` runs Blocks A,B,C,D **with no early-out**; each block
  that matches its own tile-index/overlay predicate independently rolls its own
  `Random__RandomRanged(1, Rules+0x1740)` from the **Scenario RNG** (`LEA ECX,[EDX+0x218]` before each
  call). A cell satisfying ≥2 block predicates rolls ≥2 draws.
- **Evidence:** `disassemble_function 0x00489280` (Block A `LAB_00489f77`, B `0x0048a0a5`,
  C `0x0048a214`, D `0x0048a26a`; no break between blocks). Rust `:1407-1474`.
- **Acceptance test:** feed one `BridgeDamageEvent` to a cell whose state matches two blocks; assert the
  SimRng advances by exactly N draws matching the binary's block-match count (oracle vector via
  `bridge-oracle-compare.rs`), and that a 2-block cell consumes 2 BridgeStrength rolls, not 1.
- **Owner:** `bridge_orchestrator.rs`. Determinism-critical → **one serial implementer + world_hash regression test.**

### BR-02 — `ApplyDamageToCell` is an overlay-first dispatcher with a duplicate Block C/D gate `[LIVE]` — ✅ LANDED (a05886e)
- **Current:** an in-band overlay cell takes exactly 1 Rust Direct path (`HighDirect`/`LowDirect`), 1 draw, `break`.
- **Correct:** `ApplyDamageToCell @ 0x00587180` checks overlay FIRST (`0x49<ov<100 → DestroyBridge_Low`;
  `0xcc<ov<0xe7 → DestroyBridge_High`) before any tile-index SM branch. Block A's match test is on
  `IsoTileTypeIndex (EDI+0x38)`, NOT overlay — so a raw in-band cell can pass Block A's gate + roll, enter
  `ApplyDamageToCell`, route to `DestroyBridge_High`, then **fall through to Block D** whose overlay gate
  also matches → 2nd roll → 2nd destroy attempt. Up to 2 draws / 2 attempts for one cell.
- **Evidence:** `decompile_function 0x00587180`; `disassemble_function 0x00489280` (Block D `0x0048a26a`).
- **Caveat:** the 2nd path requires the cell's iso-tile index ∈ Block A/B tile set (`DAT_*` `[BSS]`); the
  Block C/D duplicate overlay gate is standalone-proven.
- **Acceptance test:** oracle vector for a structural in-band-overlay cell whose iso-tile index lands in the
  Block A/B set → assert 2 BridgeStrength draws + 2 `DestroyBridge_*` invocations.
- **Owner:** `bridge_orchestrator.rs` (model `ApplyDamageToCell` as the real dispatcher, not 4 sibling break-paths). **Pairs with BR-01; same serial implementer.**

### BR-03 — Debris outer 95% gate constant off by 2 `[LIVE]` — ✅ LANDED (baf69a5)
- **Current:** `BRIDGE_DEBRIS_OUTER_GATE_EXCLUSIVE = 2_040_109_466` (`bridge_orchestrator.rs:371`),
  used `outer_draw >= ... continue` (`:1180`) → passes iff `draw ≤ 2_040_109_465`.
- **Correct:** **`2_040_109_464`** (pass iff `draw ≤ 2_040_109_463`). Decoded from `(double)draw·scale < 0.95`
  with `scale = 0x007e3570 = 2^-31 + 2^-61` and `0.95 = 0x007e4f58`; largest passing integer = 2_040_109_463.
- **Evidence:** `decompile_function 0x0047DD70` + `read_memory 0x007e3570 / 0x007e4f58`; boundary recomputed from the exact decoded scale.
- **Impact:** at `draw ∈ {2_040_109_464, 2_040_109_465}` Rust spawns full debris (6–7 extra draws), gamemd spawns nothing → **hard lockstep desync.**
- **Acceptance test:** unit test asserting the boundary: `draw=2_040_109_463` passes, `2_040_109_464` fails; world_hash regression over a scripted collapse.
- **Owner:** `bridge_orchestrator.rs`.

### BR-04 — MetallicDebris 50% gate constant off by 1 `[LIVE]` — ✅ LANDED (baf69a5)
- **Current:** `BRIDGE_METALLIC_GATE_EXCLUSIVE = 0x4000_0000` (`:372`), `metallic_draw < ...` (`:1198`) → passes iff `draw ≤ 0x3FFF_FFFF`.
- **Correct:** **`0x3FFF_FFFF`** (pass iff `draw < 0x3FFF_FFFF`, i.e. `≤ 0x3FFF_FFFE`). `draw=0x3FFFFFFF` maps to exactly 0.5 → strict `<` FAILS in gamemd.
- **Evidence:** `decompile_function 0x0047DD70` + `0.5 = 0x007e1738`.
- **Impact:** at `draw=0x3FFFFFFF` Rust draws a metallic slot the binary does not → extra draw → desync.
- **Acceptance test:** boundary unit test (`0x3FFFFFFE` passes, `0x3FFFFFFF` fails).
- **Owner:** `bridge_orchestrator.rs`.

### BR-05 — Metallic slot draw is unconditional after gate+alloc `[LIVE]`
- **Current:** slot draw gated on `metallic_pass && metallic_count > 0` (`:1202`).
- **Correct:** once the metallic gate + `operator_new` succeed, gamemd unconditionally calls
  `RandomRanged(0, count-1)` — there is NO `count==0` branch (asm `0x0047df61/0x0047df74 → CALL 0x0047df91`).
  With a modded empty `MetallicDebris=` gamemd consumes a `RandomRanged(0,-1)` draw (then UB-indexes); Rust skips it.
- **Impact:** modded-empty list only (stock `MetallicDebris` non-empty). Determinism divergence, not stock-visible.
- **Acceptance test:** with `metallic_count==0` forced, assert the draw is consumed to match the binary (or document the deliberate divergence + log the dropped behavior per "no silent caps").
- **Owner:** `bridge_orchestrator.rs`. **LOW priority** (unreachable in stock); fix alongside BR-03/04.

### BR-06 — Repair healthy-variant RNG: low bits vs high bits `[LIVE-CONFIRMED 2026-05-29]`
- **Current:** `repair_variant_offset` → `next_rejection_sampled_u8(rng,3)` (`walker.rs:412-425`); with
  `max_inclusive=3` the reject branch is dead → returns `draw % 4` = **low 2 bits** of one `next_u32`.
- **Correct:** `FUN_00598030` = `Random__Next + Math__ftol` → `floor(draw·4·2^-32)` = **high 2 bits** (multiply-high).
  Same 1-draw count; the value differs ~75% of the time → wrong repaired-tile variant + diverged downstream RNG state.
- **Evidence:** `disassemble_function 0x00598030` (2026-05-29) — `FMUL [0x007ed898]` + `Math__ftol` (`CALL 0x007c5f00`) confirm the multiply-high `RandomRanged` shape; high-bit extraction verified.
- **Acceptance test:** for a fixed seed, assert the chosen variant byte equals `(draw>>30)` not `(draw&3)`; oracle vector.
- **Owner:** `walker.rs`. Determinism-relevant → serial.

### BR-07 — Repair variant draws from the wrong RNG stream `[LIVE-CONFIRMED 2026-05-29]`
- **Current:** repair uses `self.scenario_rng` (`world_orders.rs:381`).
- **gamemd (CONFIRMED):** `disassemble_function 0x00598030` shows `MOV ECX,0xabe890` immediately before the
  `Random__Next` call (`CALL 0x0065c780`) — the repair variant draws from **`g_MapGenRng @ 0x00abe890`**, a
  separate stream from `Scen->Random`. Callers = `RepairBridgeWalker_{NS,EW}_{High,Low}` / `SelectBridgeTile*`.
  Drawing it from `scenario_rng` both picks the wrong value AND **pollutes the scenario stream** → systemic desync.
- **Function shape:** `0x00598030` is a rejection-sampled `RandomRanged(lo,hi)` on `g_MapGenRng`:
  `lo + floor(draw · range · [0x007ed898]) ` via `Math__ftol`, loop while result > hi. For the `(0,3)` repair
  call this is the **high 2 bits** (BR-06).

### APPROVED FIX (next session) — BR-06 + BR-07 together, serial + RNG regression test
1. Add `map_gen_rng: SimRng` to `Simulation` (sibling of `scenario_rng`/`main_rng` in `world/mod.rs`); seed it
   deterministically from the scenario seed in `new()` + `reseed_both()`. Add a `map_gen_rng()` accessor.
2. Route the repair variant (`walker.rs` `repair_variant_offset` and `world_orders.rs:381` repair call) to
   `map_gen_rng` instead of `scenario_rng` (BR-07 — kills the scenario-stream pollution).
3. BR-06: make `repair_variant_offset` extract the **high 2 bits** (`RandomRanged(0,3)` = `floor(draw·4·2^-32)`),
   not `draw & 3`. Mirror the existing `RandomRanged` shape used elsewhere.
4. Regression test: fixed seed, assert (a) `scenario_rng` is UNCHANGED by a repair (no pollution), (b) the repaired
   variant byte equals the high-bits value, (c) `world_hash`/`map_gen_rng` state stable over a scripted repair.
- **DEFERRED sub-item (cross-engine, not lockstep):** the *absolute* `g_MapGenRng` seed gamemd sets during
  map-load is NOT yet RE'd, so the repaired tile **variant value** will be our-engine-lockstep-deterministic but
  may not bit-match gamemd. Needs map-load RNG-seeding research before claiming exact cross-engine variant parity.
- **Owner:** `walker.rs` + `world/mod.rs` (new stream) + `world_orders.rs`. **Determinism → serial, no swarm.**

---

## 3. TIER 2 / 3 / 4 — deltas

### BR-08 — Bridge repair never re-activates the zone endpoint record `[LIVE]` — ✅ LANDED (baf69a5)
- **Current:** `refresh_endpoint_active_flags` (`bridge_state/mod.rs:1587-1603`) is deactivate-only
  (`record.active=false` at `:1600`); `bridge_state` is built once (`app_init_helpers.rs:368`) and never reconstructed.
  `bridge_record_matches` (`zone_build.rs:65`) gates the long-range A* edge on `record.active`.
- **Correct:** `ProcessBridgeDestruction_High @ 0x00573540` (the repair/restore walker; sole caller of
  `RepairBridge_High @ 0x0057f440`) calls `ValidateBridgeZones @ 0x0056db70` which, per record with `+0x08==0`,
  sets `+0x08=1` and calls `AddBridgeZoneEdges` — re-inserting the cross-zone edge (then a conditional
  `UpdateBridgeZonesHelper` rebuild gated on a `Can_Reach_Zone` test of the just-validated record).
- **Evidence:** `decompile_function 0x00573540 / 0x0056db70`; `get_function_callers 0x0057f440`.
- **Acceptance test:** destroy a bridge (record→inactive), confirm cross-zone path fails; repair via engineer;
  assert `record.active==true` AND a cross-bridge A* path now succeeds.
- **Owner:** `bridge_state/mod.rs` (+ orchestrator repair tail). **HIGH — repaired bridges are currently unusable.**

### BR-09 — Collapsed body anchor never clears `overlay_byte` `[LIVE]` — ✅ LANDED (baf69a5)
- **Current:** body collapse (`mod.rs:1090-1093`) sets `damage_state=Destroyed` but never touches `overlay_byte`;
  `update_adjacent_bridges` `continue`s past `Destroyed` (`:1091`); `effective_render_state` (`:945-970`) maps the
  stale loaded byte (e.g. `0xD6`) → `Some(Healthy)` → `is_bridge_walkable` (`:972`) returns true.
- **Correct:** body branch `LAB_0057778a` of `0x00576ba0` writes `[0x11e]=0; *(+0x44)=0xffffffff` on final collapse
  (overlay cleared to −1). `+0x44` is the visible overlay byte (`ApplyDamageToCell` dispatches `0xCD..0xE6 →
  DestroyBridge_High`).
- **Evidence:** `decompile_function 0x00576ba0` (LAB_0057778a) + `0x00587180`.
- **Acceptance test:** drive the body SM to collapse on an anchor; assert `overlay_byte == 0xFF`,
  `effective_render_state == None`, `is_bridge_walkable == false`.
- **Owner:** `bridge_state/mod.rs::body_cell_advance_state`. **HIGH** (collapsed cell stays walkable + renders intact).

### BR-10 — Entire rim-refresh algorithm is wrong `[LIVE]`
- **Current:** `update_adjacent_bridges` (`bridge_orchestrator.rs:1035-1108`) does a per-cell **stub-blank**
  (`overlay_byte=0xFF`, `damage_state=Healthy`, `bridge_group_id=None`, `deck_present=false`), capped at
  `WALK_LIMIT=30`, skipping `Destroyed` / breaking on `!deck_present`.
- **Correct:** `UpdateAdjacentBridges_High @ 0x00576770` writes ZERO cell fields — it 8-dir-scans for a `0x140 & 0x500`
  head, follows the `+0x2c` anchor back-pointer / walks to the bridge head, then calls
  `UpdateBridgeEdgeTiles_High @ 0x00576200` (`mode∈{2,4}`) + `DirtyScreenRect`. `UpdateBridgeEdgeTiles_*` does the real
  state writes: 30-cell forward ramp-class search (`local_44<0x1e`), dirty-rect union, a back-walk over the run
  inspecting `flags&0x80` set→clear transitions that writes `[0x11e]=0`, `+0x44=-1`, `MarkTerrainDirty`, and FIRST
  calls `SetBridgeDirection_NESW(uVar17∈{0,6}, 0)` — a **multi-cell group clear** (anchor + 3 fwd + 1 opposite,
  flag-mask AND, per-cell `BlowUpBridge` when `param_3==0`). It also latches `RepairBridgeSegment` (cap re-stamp) on
  the was-clear→now-set transition. NESW (High) vs NWSE (Low) split. `DestroyBridge_Low_OnHutDeath @ 0x00574c20`
  invokes the **High** rim refresh (vanilla "High-on-Low" quirk).
- **Evidence:** `decompile_function 0x00576770 / 0x00576200 / 0x00570ae0 / 0x0047e040 / 0x00575ee0`;
  `get_function_callers 0x00576770 / 0x00571050`.
- **Phase-A predicate (MISS):** key on flag bits `0x100|0x400` (structural-anchor OR bridgehead), NOT Rust's
  `role==Bridgehead || damage_state==Destroyed`; start-coord is a `+0x2c` pointer chase, not a walk.
- **Acceptance test:** collapse a multi-cell span; assert (a) no cell is blanked to `0xFF`/`!deck_present` by the rim
  pass, (b) the bridge-end iso-tiles are re-stamped (edge-tile variant changes), (c) the 30-cap applies to the
  ramp-search loop. Event-31 broadcast stays a skirmish no-op (TS/campaign-only — keep).
- **Owner:** `bridge_orchestrator.rs::update_adjacent_bridges` + new `bridge_specs` edge-tile helpers. **Large rewrite.**

### BR-11 — Destroy walkers skip `Bridgehead`-role cells `[LIVE]` — ✅ LANDED (f914046)
- **Current:** all 4 walker bodies + 4 cascade leaves do `if matches!(c.role, Bridgehead) { continue; }`
  before the overlay/damage write (`walker.rs:879,972,1236,1325` and `:753,806,1118,1170`).
- **Correct:** `DestroyBridgeWalker_*` and `ApplyBridgeDestruction_*` write the per-case overlay to **all three**
  triple cells unconditionally — gamemd has no `BridgeCellRole` concept; it writes purely on the overlay band.
  A bridgehead/ramp cell in the triple gets the destroy overlay and a `BlowUpBridge` when the write is final.
- **Evidence:** `decompile_function 0x0057cf60 / 0x0057d530 / 0x0057bcf0 / 0x0057c2b0 / 0x0057e7a0` (no role guard anywhere).
- **Acceptance test:** collapse a span whose end neighbor is a pass-3 bridgehead; assert that bridgehead cell gets the
  destroyed overlay + (if final) appears in the BlowUpBridge set, instead of being left standing.
- **Owner:** `walker.rs` (remove the role-skip in walker bodies + cascade leaves).

### BR-12 — Healthy bridgehead collapses on the 2nd hit; absorb writes slot+3 `[LIVE]`
- **Current:** `bridgehead_advance_state` (`mod.rs:1449-1545`) collapses when `input_is_final||anchor_is_final`
  (`is_final ≡ AboutToFall`), and the absorb path (`:1543-1544`) writes the **anchor → AboutToFall (slot+3)** → a 2nd
  hit collapses (`tests.rs:1282`).
- **Correct:** `0x00576ba0`/`0x00571490` decide from the INPUT cell's own tile class `(puVar9[0x38]-base)+1`; only
  `class == base+3` collapses (`return 1`); slots +0/+1/+2 take the absorb branch which writes the anchor to
  **slot+2 (Damaged)** and `return 0` **every hit** — a healthy bridgehead never collapses via this path.
- **Evidence:** `decompile_function 0x00576ba0 / 0x00571490 / 0x00572230 / 0x00572330` (DamageB caps at slot+2).
- **Acceptance test:** hit a healthy bridgehead via the SM N times; assert it NEVER collapses and the anchor reaches at
  most `Damaged` (slot+2), not `AboutToFall`. (Authored slot+3 bridgeheads still collapse first-hit — keep BR-D2-refuted.)
- **Owner:** `bridge_state/mod.rs::bridgehead_advance_state`.

### BR-13 — Ground force-kill bypasses the death pipeline `[LIVE]`
- **Current:** `kill_ground_occupants_at` (`bridge_orchestrator.rs:998-1024`) sets `health=0; dying=true` + anim switch;
  the combat death pipeline (`handle_entity_deaths`, fed only by combat `damage_events`, `combat/mod.rs:2160-2186`) never
  sees these victims → no explosion/wreck/smudge/die_sound/death-weapon/passenger-eject/score; non-animated voxel units
  despawn silently (`animation.rs:403-406`).
- **Correct:** `BlowUpBridge @ 0x0047dd70` calls `vtable[+0x16c] Take_Damage(&Health, dmg=0, C4Warhead, 0, force=1, 1, 0)`
  per ground occupant → the full death pipeline.
- **Evidence:** `disassemble_function 0x0047dd70` (`0047dd84..0047ddae`).
- **Acceptance test:** kill a tank + a soldier via bridge collapse; assert a wreck/husk spawns, the death AnimList plays,
  a smudge is placed, passengers eject, and the killer (if any) scores — same as a C4Warhead kill.
- **Owner:** `bridge_orchestrator.rs` (route the kill through the C4Warhead damage path, not a bespoke health-zero).

### BR-14 — Ground-kill over-selects airborne units `[LIVE]` — ✅ LANDED (baf69a5)
- **Current:** filter is `rx==rx && ry==ry && !is_on_bridge_layer() && health>0` (`bridge_orchestrator.rs:1001-1011`);
  `is_on_bridge_layer()` returns only `self.on_bridge` (`game_entity.rs:604`). An aircraft over the cell
  (`on_bridge==false`, live `rx/ry`) is force-killed.
- **Correct:** `BlowUpBridge` walks only the cell ground list `+0xE4` — in-flight aircraft (air layer) are not on it.
- **Evidence:** `disassemble_function 0x0047dd70` (`0047dd84 MOV ECX,[ESI+0xe4]`).
- **Fix bound:** exclude air AND underground layers — gate on `occupancy_list_layer().is_some()`
  (`game_entity.rs:582-601` already returns `None` for Air|Underground), not just `!on_bridge`.
- **Acceptance test:** park an aircraft over a collapsing bridge cell; assert it survives. A grounded/landed unit on the cell still dies.
- **Owner:** `bridge_orchestrator.rs::kill_ground_occupants_at`.

### BR-15 — `update_ramp_perpendicular` writes no overlay/pavement, fires no `BlowUpBridge` `[LIVE]`
- **Current:** `update_ramp_perpendicular` (`bridge_specs.rs:537-641`) mutates only an `Anchor` target's `damage_state`
  + abstract `bridgehead_anchor_class`; no overlay write, no pavement toggle, no recursion, no `BlowUpBridge`.
  `is_high_bridge` is discarded (`:542`).
- **Correct:** the `UpdateRamp_*_High` family (`0x00572230` DamageA, `0x00572330` DamageB, `0x00572440` CollapseA,
  `0x005727e0` CollapseB; EW at `0x00572b80/c90/da0`/`0x00573170`) writes the perpendicular target's **visible overlay**
  via `SetOverlayAndPropagate(+0/+1/+2 BridgeSet class)` or `ToggleBridgePavement`, gated by the target's `+0x38` slot;
  CollapseA/B additionally **recurse** + fire **3× `BlowUpBridge`** on the body-axis triple + `SetOverlayAndPropagate(+3)`.
  High uses concrete-bridge constants, Low uses wood constants → `is_high_bridge` MUST branch (BR-D5 of body-SM).
- **Evidence:** `decompile_function 0x00572230 / 0x00572330 / 0x00572440` (Low family `0x0056ed40…` not re-decoded — assumed to mirror; confirm before Low fix).
- **Acceptance test:** DamageA on an NS anchor → assert the perpendicular target's `overlay_byte` advances per the slot
  table (not just an enum); CollapseA on a slot-+3 perpendicular → assert 3 BlowUpBridge cells + recursion.
- **Owner:** `bridge_specs.rs::update_ramp_perpendicular`.

### BR-16 — Minimap `MarkTerrainDirty` never fed by any collapse path `[LIVE]` — ✅ LANDED (51295b8)
- **Current:** `StateOutcome::Collapsed` (`mod.rs:381-401`) has no radar field; the orchestrator never pushes into
  `sim.radar_terrain_dirty_cells`. The minimap terrain refresh (`render/minimap.rs:225-243`) is fed only by the engineer
  repair path (`world_orders.rs:391`) + combat smudge — destruction feeds nothing → stale minimap over a collapsed span.
- **Correct:** the direct walkers' final branch + `SetBridgeDirection_NESW` + the bridgehead collapse call
  `RadarClass__MarkTerrainDirty` on the collapsed triple (+ cascade-leaf perpendicular neighbors).
- **Evidence:** `decompile_function 0x0057cf60 / 0x0057d530 / 0x0047e040`; Rust trace `mod.rs:381-401/513`,
  `world_orders.rs:391`, `render/minimap.rs:225-243`.
- **Acceptance test:** collapse a span; assert the collapsed cells are pushed into `radar_terrain_dirty_cells`
  (the same channel the repair path uses).
- **Owner:** add a `radar_cells` field to `StateOutcome::Collapsed`; feed it in `bridge_orchestrator.rs`.

### BR-17 / BR-18 / BR-19 — dispatcher Z-window + High/Low routing `[LIVE]`/`[BSS]`
- **BR-17 Z-window unit/offset:** Rust `path_matches_cell` (`mod.rs:895-901`) accepts `level-1 ≤ impact_z ≤ level+1` in
  raw level units; gamemd accepts `(Level-2)·LevelHeight+BridgeHeight < z ≤ (Level+1)·LevelHeight+BridgeHeight` in lepton Z
  (`disassemble_function 0x00489280`, `0x00489f82..0x00489fba`). Lower bound is `-2`-exclusive (vs Rust `-1`-inclusive) and
  the whole window is leptons, not levels. Numeric lepton gap `[BSS]` (`0x0089e870/0x0089e864` runtime-init).
- **BR-18 Z-window gating:** Rust applies the Z gate to every SM candidate; gamemd runs it only when `Flags & 0x100` set
  (`0x00489f7d TEST AH,0x1; JZ` skips the window for non-structural cells).
- **BR-19 High/Low routing:** Rust uses `is_high = deck_level>=4` (`mod.rs:884-890`); gamemd
  (`decompile_function 0x00587180`) routes by iso-tile-type-index set membership + `0x18/0x19` vs `0xed/0xee`
  perpendicular-neighbor overlay — no deck-level test. In-skirmish trigger frequency `[BSS]`/UNCHECKED; latent for modded maps + 20k scale.
- **Acceptance test (combined):** oracle vectors at the Z boundaries (`z = (Level-2)·LH+BH ± 1`, `(Level+1)·LH+BH ± 1`) and
  for a non-structural anchor-zone cell (assert Z gate skipped); a high-vs-low routing vector keyed on tile-index not level.
- **Owner:** `bridge_state/mod.rs::path_matches_cell` + `combat/combat_aoe.rs::bridge_adjusted_impact_z`. Determinism-relevant.

### BR-20 — Bridgehead collapse overlay + level write missing `[LIVE]`
- **Current:** bridgehead collapse (`mod.rs:1461-1535`) sets only `bridgehead_anchor_class=AboutToFall` + `damage_state=Destroyed`; no overlay/level write.
- **Correct:** `SetOverlayAndPropagate(base+3, …, level = (cell deck-height byte +0x11B) − 4, 0)`.
- **Evidence:** `decompile_function 0x00576ba0 / 0x00571490`.
- **Acceptance test:** collapse a bridgehead; assert the anchor overlay = slot+3 tile at `level=deck_height-4`. (Visibility magnitude depends on renderer — see §5.)
- **Owner:** `bridge_state/mod.rs`.

### BR-21 — Debris/explosion jitter offset uses floor/2^31 `[LIVE]`
- **Current:** `bridge_jittered_subcells` (`bridge_orchestrator.rs:1293-1304`) = `floor(draw·50/2^31) − 25` → range −25..+24.
- **Correct:** `round((draw·scale − 0.5)·50)` with `scale = 0x007e3570` and `Math__ftol` round-to-nearest → range −25..+25; diverges by 1 lepton on most draws and Rust never reaches +25.
- **Evidence:** `disassemble_function 0x0047dd70` (`0x0047decf..0x0047dee9`) + `Math__ftol @ 0x007C5F00` = FISTP round-to-nearest.
- **Acceptance test:** golden-value table: `0x66666666→15`, `0x7FFFFFFE→25`; assert no draw yields outside −25..+25. Render-only (no draw-count change).
- **Owner:** `bridge_orchestrator.rs`. Also applies to `spawn_bridge_explosion_effect` (hut walker) — same helper (BR-D4).

### BR-22 — Debris/explosion anim Z = `deck_level` vs gamemd `0` `[LIVE]`
- **Current:** `WorldEffect.z = bridge_deck_level_if_any().unwrap_or(level)` (`:1215,1240`).
- **Correct:** gamemd anim Z = `Level·DAT_0089e7c0 + DAT_0089e7b4`; both constants read **0.0** live → Z = 0 regardless of level.
- **Evidence:** `read_memory 0x0089e7c0 / 0x0089e7b4` (both all-zero doubles); `disassemble 0x0047de78..0x0047dea6`.
- **Acceptance test:** assert spawned debris/explosion `WorldEffect.z == 0` (rendered-pixel magnitude unverified — flag for render review).
- **Owner:** `bridge_orchestrator.rs::spawn_bridge_debris`.

### BR-23 / BR-24 / BR-25 — render railing order + bridge shadows `[LIVE]`
- **BR-23 order:** Rust submits railings at `draw_passes.rs:224` AFTER `draw_merged_object_pass` (`:156`); gamemd
  `TacticalClass_Draw @ 0x006D3D10` emits railings inside the terrain bundle (`Tactical_layer_overlays`) BEFORE the object loop.
- **BR-24 shadow:** `build_bridge_shadow_instances` exists (`bridges.rs:210-291`) but the Step 2.5 pass (`draw_passes.rs:72-79`)
  is explicitly DISABLED; gamemd `CellClass__DrawOverlay_Shadow @ 0x0047F510` blits the shadow (`CC_Draw_Shape … 0x4601`). Enable.
- **BR-25 shadow Z:** Rust shadow builder uses `z + BRIDGE_HEIGHT_BONUS(4)` (`bridges.rs:273`); gamemd shadow Z basis = `level·-15-2`, NO +4 bonus.
- **Evidence:** `decompile_function 0x006D3D10 / 0x0047F510`.
- **Acceptance test:** visual — railings draw under objects standing on the deck; bridge casts a shadow; shadow elevation matches ground-level basis. (Render-layer; verify in-app.)
- **Owner:** `app_render/draw_passes.rs` + `app_instances/bridges.rs`.

### BR-26..BR-32 — locomotion / height / render-fudge `[LIVE]`
- **BR-26 on_bridge Exit predicate:** `compute_bridge_transition` Exit keys on `src.bridge_structural` (body-only,
  `movement_bridge.rs:88-102`); gamemd clears on `src & 0x100` (body OR ramp). At a body→ramp step Rust fires Clear a tick
  early. Fix: Exit on a `src` predicate that includes ramp/bridgehead `0x100` cells.
  Test: step a unit body→ramp; assert `on_bridge` stays set for that tick.
- **BR-27 drive Z-bump + brake:** `Set_Destination @ 0x004afd40` unconditionally `dest.Z += g_BridgeZOffset_Drive` when
  dest `0x100`; `Process_Drive_Track @ 0x004b0f20` adds the deck offset into the brake-distance `Sqrt`. Rust drive has no
  bridge Z math. Determinism (brake ramp) + sub-frame timing. Owner: `drive_locomotion.rs`.
- **BR-28 ShouldBeOnBridge:** `ObjectClass__ShouldBeOnBridge @ 0x005f6a70` gates on `LeptonsPerLevel·3 < |groundDest−groundCur|`
  (strict 3-level lepton threshold). Rust has no such predicate (uses `>=2` layer pick + `==4` transition). Owner: `movement_bridge.rs`.
- **BR-29 layer thresholds:** gamemd uses three distinct sites — A* `>=2` (PARITY), TooBig crush pick `unitZ >= ground+~4 lvl (leptons)`,
  scatter case-6 `>=3 levels`; Rust uses a single `>=2` (`movement_occupancy.rs:152`). In the 2–3-level band the occupancy
  layer / scatter decision diverges. Owner: `movement_occupancy.rs` + crush.
- **BR-30 TooBig crush:** `Process_Drive_Track` crush block calls `vtable+0x16c` `10000` dmg (warhead `Rules+0xfa8`) per
  non-crushable occupant + `20` self-dmg; Rust has no oversized-deck-crush (grep: none). Owner: `drive_locomotion.rs`/`bump_crush.rs`.
- **BR-31 anim-Z rebase:** `Set_Height_On_Bridge @ 0x005f5fa0` does `+= DAT_00ac13bc` under `+0x23` then GetGroundHeight rebase
  for attached anims on bridge units; Rust has no equivalent. Narrow (muzzle/attachment elevation). Owner: anim attach.
- **BR-32 render BridgeFudge:** `Draw_Sprite_With_BridgeFudge @ 0x0073b140` split-blits oversized units on the deck; Rust render
  consumes none of `on_bridge`/deck-Z. Owner: render (consumes the deck `position.z` this facet produces).
- All `[LIVE]` (mechanism); the `*4`/`*3`/`360` numeric constants are `[BSS]` (see §5: D4/D8/D10/ship-360).

### BR-33..BR-38 — tube + bridge-aware pathfinding `[LIVE]`
NOTE: the production tube path is `tick_low_bridge_tube_movement` (`movement_tick.rs:851`); `begin_drive_tube_traversal`
/ `finish_unit_tube_movement` are `#[cfg(test)]`-dead — fix the live path, not the dead one.
- **BR-33 step rate:** `UnitClass__TubeMovement @ 0x007359f0` interpolates fractionally (`Math__ftol` per-tick move) and advances
  a cell only when `tick_move >= remaining lepton distance`; Rust advances exactly 1 cell/tick (`tube_movement.rs:219-269`). Owner: `tube_movement.rs`.
- **BR-34 exit facing:** gamemd sets facing `(tube.dir<<13) − 0x8000` read at the exit cell; Rust uses last-move-delta. Owner: `tube_movement.rs`.
- **BR-35 exit occupant scatter:** gamemd scatters RTTI-1/0xf exit occupants (`vtable+0x174`) and retries until clear before finalizing;
  Rust snaps onto the exit unconditionally (stacks). Owner: `tube_movement.rs`.
- **BR-36 tube Z ramp:** gamemd `(GetGroundHeight(exit) − GetGroundHeight(entry)) / path_len` applied every tick; Rust snaps per-cell deck/ground level (stair-step). Owner: `tube_movement.rs`.
- **BR-37 A* endpoint snap:** `ResolvePathCoord_BridgeAware @ 0x00583180` snaps a bridge-flagged source/dest to the nearer bridge-record
  endpoint (Sqrt-approx tie-break, strict `<` → far endpoint on tie) before seeding A* + zone-cost; callers pass nonzero `param_3` on
  bridge cells (`AStar_pathfind_search @ 0x0042c900`, `EstimateZoneCost @ 0x0042d170`). Rust `astar_search` takes start/goal literally. Owner: `pathfinding/core.rs`.
- **BR-38 passability replay:** `UpdateBridgePassability @ 0x0042acf0` + `FindNearbyBridgePeer @ 0x0042b080` do lower-id-peer path
  replay with dir-8 tube jumps + a 3×3 `0x40000` neighborhood toggle; Rust models only a passive caller-supplied marker overlay. Latent
  (no current Rust producer emits a bare dir-8 over an auto tube). Owner: `pathfinding/core.rs`.
- Acceptance tests: tube traversal frame-count vs Speed; exit facing == `(dir<<13)-0x8000`; exit onto an occupied cell scatters the occupant;
  A* across a bridge snaps endpoints (golden path). Carry the Sqrt-approx + strict-`<` tie direction (fixed-point hazard).

### BR-39..BR-49 — TIER 4 low / boundary / cleanup
- **✅ BR-39 LANDED (1b7592a) anchor slot-5 (dir 6):** `walk_anchor_pattern` slot 5 = `(anchor.x+1, anchor.y)` (dup of slot 4); gamemd extra cell =
  `opposite_cell + offset[2] = anchor+2E` (`0x0047e040`). Fix slot 5 = `(anchor.x+2, anchor.y)` for dir W (match `bridge_facts::stamp_slots`).
  Also (MISS): the collision flips the opposite cell's role Tail→Body (last-write-wins in pass-2 tagging, `mod.rs:668-683`), and the true
  extra cell `(7,5)` gets no `anchor_span_id`/role though `bridge_facts` attaches it via `+0x2c`. Owner: `bridge_state/mod.rs::walk_anchor_pattern`.
- **BR-40 deactivation granularity:** Rust deactivates the whole BFS group; gamemd deactivates only records within `FindBridgeRecord` tol-3 of
  the impact (`InvalidateBridgeZones @ 0x0056dae0`). Diverges only on shared-deck junctions. Owner: `mod.rs`.
- **BR-41 endpoint pair:** Rust max-Manhattan over adjacent ground cells; gamemd `endpoint_a=own MapCoord`, `endpoint_b=opposite-step` via
  orientation tables (`ComputeBridgeZones @ 0x0056d6e0`). Owner: `mod.rs::compute_bridge_endpoints`.
- **BR-42 record cardinality + self-heal:** Rust 1 record/BFS-group built once at init; gamemd 1 record/bridge-tile, and
  `Validate/InvalidateBridgeZones` lazily call `ComputeBridgeZones` to rebuild when `FindBridgeRecord` returns -1 (self-healing). Owner: `mod.rs`.
- **BR-43 dead repair RNG:** `body_cell_repair_state` (`mod.rs:1279-1373`) uses a 3rd RNG shape (`draw & 3`), tests-only — remove or align to BR-06. Owner: `mod.rs` (cleanup).
- **BR-44 effect frame ms:** `BRIDGE_EFFECT_FRAME_MS=67` vs 66.667 ms (15 fps logic frame); ~1.7 ms drift at 5 frames. Owner: anim-timing.
- **BR-45 empty BridgeExplosions jitter:** gamemd draws 2 jitter/cell even when `BridgeExplosions` empty; Rust early-returns. Modded-only. Owner: `bridge_orchestrator.rs`.
- **BR-46 extent cap:** `MAX_EXTENT_PROBE=64` has no gamemd counterpart (uncapped); unreachable >64-cell span. Owner: `bridge_orchestrator.rs` (document, likely leave).
- **BR-47 EW bias X-wrap:** gamemd wraps X mod 65536 on the bias-subtraction underflow and runs a short off-map walk; Rust aborts (0 cells). Column-0 boundary only. Owner: `bridge_orchestrator.rs`.
- **BR-48 terrain occupants:** gamemd ground loop `Take_Damage`s `TerrainClass` occupants (trees) too; Rust kills only `EntityStore` entities. LOW. Owner: `bridge_orchestrator.rs`.
- **BR-49 EW classifier edge-wrap `[UNCERTAIN]`:** at X=0/X=511 the EW west/east probe reads the wrapped linear cell `(511,Y-1)`/`(0,Y+1)`;
  Rust reads 0. NS axis does not wrap. Reachability (can a bridge body/sibling sit at column 0/511?) unproven — needs the map-authoring constraint check. Owner: `walker.rs` (only if reachable).

---

## 4. SPECIAL — CABHUT C4 "does nothing" bug (root cause UNIDENTIFIED)

The known port-side bug (SEAL/Tanya C4 on the bridge repair hut does nothing — project memory
`project_c4_bridge_hut_followup`) was investigated this run. **The finder's root-cause hypothesis (the
`are_houses_friendly` ally gate dropping the plant) was REFUTED:** `are_houses_friendly` (`map/houses.rs:89-101`)
returns true only on name-equality or an explicit alliance-map entry, and the skirmish alliance build
(`app_skirmish.rs:359-375`) never auto-allies Neutral with the player — so a Neutral CABHUT classifies as
`EnemyStructure` and all three C4 gates PASS. The prior Immune-gate hypothesis was already refuted.

gamemd side confirmed (`InfantryClass__PerCellProcess @ 0x00519630`, `What_Action_OnObject @ 0x0051e3b0`): the
Mission-0x11 sabotage marker branch has **no ally / owner / Immune check** — only `building.Mission != 0x13`
(not Selling) + `vtable[0x160]()==0` + the `+0x6DF` already-marked guard.

**The real upstream blocker is still unknown.** Candidates to investigate next (in order):
1. **CABHUT runtime ownership** — owner comes from the map `[Structures]` line (`map/entities.rs:188,245`),
   not special-cased; if the test maps own it by the player/an ally, hover reads `FriendlyStructure` and all C4 gates reject.
2. **1×1 foundation hit-test** for a Neutral CABHUT in `hover_target_at_point` (`app_entity_pick.rs:96-145`, `click_hits_foundation`).
3. **`is_cell_revealed` / `is_cell_gap_covered`** (`app_entity_pick.rs:137-145`) downgrading the hover to `HiddenEnemy` (also C4-rejected).

The integration test (`world_orders_bridge_repair_tests.rs:793`) **sidesteps the bug** — it sets `c4_plant`
directly and spawns the CABHUT as `"Soviets"` (enemy), never exercising the Neutral-ownership / hit-test path.

**Also (cabhut MISS-2):** gamemd requires `building.Mission != 0x13` (not Selling) to accept the plant; Rust has a
`TODO(parity)` (`world_commands.rs:984-986`) that does NOT reject a mid-sell building. Low frequency; real gate gap.

**Recommendation:** this is a dedicated end-to-end diagnosis plus `/trace-action` target ("SEAL C4 on a Neutral CABHUT,
click→cursor→order→plant→hut-death→collapse"), NOT part of the mechanical fix batch. Do not attempt a one-line fix.

---

## 5. UNCERTAIN — needs a specific live read before acting

Ghidra MCP is currently disconnected; these cannot be resolved until it returns:
- **BR-07** repair RNG stream: `disassemble_function 0x00598030` → confirm `MOV ECX, 0xabe890` (`g_MapGenRng`) vs scenario stream; audit `scenario_rng` seeding.
- **BR-27 railing values:** real tables `0x00abc210` / `0x00abc2d0` are zero in the static image — need a post-theater-load debugger capture; the `0x00abc2d0` second-table path (`+0x2e1` flag + `DAT_00aa102c` range) is unmodeled.
- **debris D6** (hut-walker perp loop unconditional): re-decode `0x00575BA0` to confirm the fixed 3-iteration loop + scratch-cell fallback always consume 4 draws.
- **zone D3** (low-bridge tube records never deactivated): decode the low-collapse cell-zone-type recompute sever path + a Rust fixture showing a stale `active=true`.
- **bridgehead D5** (collapse destroyed-set aggregation): Rust appends already-Destroyed E/W/N/S neighbors; binary uses a fixed 10-cell recalc rect — trace consumers to prove output effect.
- **body 0x80-vs-role gate:** binary gates the perpendicular STATE-byte write on target `[0x140] & 0x80`; Rust gates on `role==Anchor`. Prove `{0x80 cells} == {role==Anchor}` or find the counterexample. (Overlay write of BR-15 is `0x80`-independent and unconditionally REAL.)
- **cabhut D4:** tile-index window `DAT_00ABAD1C` (low/high hut family select) — runtime-init; needs a post-map-load read to confirm Rust `is_wood_bridge_repair_tile` covers `[base, base+0x10)`.
- **`[BSS]` numeric constants:** Z-window lepton gap (`0x0089e870/0x0089e864`), drive/ship/walk bridge-Z `*4`/`*3` and rounding split, ship under-bridge `360` vs `ftol(g_ShipHeightStep*4)` — all need post-map-load debugger reads to pin the exact lepton values.
- **locomotion D4/D8/D10:** rounding split (round-up vs round-down 1-lepton), `IsOnBridgeRamp`/`IsOnBridgeSurface` tile-index ranges vs the Rust terrain resolver, ship Z constant — gamemd side verified; Rust-resolver equivalence unconfirmed.

---

## 6. PARITY-CONFIRMED — do NOT touch (verified matching this run)

- **Neighbor classifiers + `pick_destruction_overlay` table:** all 14 checks (`CheckBridgeNeighbors_{NS,EW}_{High,Low}` bit
  assignments + all 4 `ApplyBridgeDestruction_*` 16-entry tables) byte-identical to Rust (`walker.rs` / `bridge_specs.rs`). Proven over the full 256-byte input space.
- **Direct-walker case values + transition overlays** (all 4 axes/levels): match (`walker.rs:852-869` etc.).
- **Sibling-cascade perpendicular shift + cascade-leaf two-stage progression + `n!=cur` no-op guard + `is_final`/`zones_dirty`:** match.
- **Dispatch overlay→direct routing** (`0x4A..0x63`/`0xCD..0xE6`) + RNG inclusivity (`[lo,hi]`) + roll direction (`roll < damage`) + IonCannon
  bypass (all paths) + 3-retry/4-attempt SM-only + `DestroyableBridges` outer gate: all match.
- **Fallout order** kill(`+0xE4`)→DropIn(`+0xE8`, survives, no drown)→debris; C4Warhead dmg=0 force-kill; InfDeath lookup; 4 BlowUpBridge
  cells; next-object snapshot before mutate; map-editor early-out: all match.
- **Debris draw COUNT + ORDER** (outer, jitter×2, metallic gate, metallic slot only on pass, delay `R(1,5)`, explosion slot): match — only
  the gate CONSTANTS (BR-03/04) and the jitter ARITHMETIC (BR-21) are off.
- **Hut collapse:** 5×5 X-major scan, `local_2c=4` step count, 3-retry break-on-success, step/bias (signed-truncating div), extent off-by-one
  cancellation, RandomRanged equal-bounds early-out, OnHutDeath callers live in YR (not TS): all match.
- **Repair:** `repair_transition` table (all 4 families), 1-draw-per-strip written to 3 cells, no-draw on Fixed/NoChange, `zones_dirty` only on
  the RandomHealthy arm, prior-final radar gate, MarkBridgesForRepair = map-init (not engineer), per-entry low/high select, radar 3-cell set (BR-D4 refuted): all match.
- **Body SM:** state-byte switch ranges + case→phase map, perpendicular direction args, anchor-pointer follow (`+0x2c`),
  DamageA-before-B / CollapseA-before-B order: all match. (Overlay/BlowUpBridge side effects are the gap — BR-15.)
- **Bridgehead:** BlowUpBridge 3-cell geometry, start-cell gates, walk targets (NS=4/EW=2), Low-returns-0/High-returns-1, IonCannon attempts: match.
- **Locomotion:** A* `>=2` threshold, deck=`level+4` invariant, layer-tagged occupancy (Ground/Bridge), JumpJet/Hover no Z-bump,
  `CheckBridgeTraversal` (near-1:1 faithful transcription): match.
- **Render/sound:** EVA_BridgeRepaired producer + dedup + local-human gate (`world_orders.rs:356-372`), Latin-square variant gate, body Y-offset,
  `effective_render_state` overlay→state map: match.
- **Tube:** `IsLowBridgeCell`, `GetTubeAtCell` bounds, `[2,4,6,0]` table, dir-8 sentinel, compass deltas: match.

---

## 7. Fix-phase plan (opt-in, after approval) — disjoint file ownership

Determinism-sensitive holes get ONE serial implementer + a world_hash regression test — never a swarm.

| Worker | Files (disjoint) | Holes |
|---|---|---|
| **W1 (serial, determinism)** | `bridge_orchestrator.rs` (dispatcher + debris RNG) | BR-01, 02, 03, 04, 05, 21, 22, 45, 47 |
| **W2 (serial, determinism)** | `walker.rs` (repair variant) — gated on BR-07 live read | BR-06, 43 |
| W3 | `bridge_state/mod.rs` (body-SM collapse, bridgehead, zones, anchor walk) | BR-09, 12, 20, 39, 40, 41, 42, 16(field) |
| W4 | `bridge_specs.rs` (UpdateRamp overlay/pavement/BlowUpBridge) | BR-15 |
| W5 | `bridge_orchestrator.rs::update_adjacent_bridges` + new edge-tile helpers | BR-10 |
| W6 | `bridge_orchestrator.rs::kill_ground_occupants_at` + combat death route | BR-13, 14, 48 |
| W7 | `walker.rs` (destroy walkers — role-skip removal) | BR-11 |
| W8 | `bridge_state/mod.rs::path_matches_cell` + `combat_aoe.rs` | BR-17, 18, 19 |
| W9 | zone repair re-activation (`mod.rs` + orchestrator tail) | BR-08 |
| W10 | `movement_bridge.rs` / `drive_locomotion.rs` / `movement_occupancy.rs` | BR-26..31 |
| W11 | `tube_movement.rs` | BR-33, 34, 35, 36 |
| W12 | `pathfinding/core.rs` | BR-37, 38 |
| W13 | `app_render/draw_passes.rs` + `app_instances/bridges.rs` + `bridge_railing_atlas.rs` | BR-23, 24, 25, 32 |

**Contention note:** `bridge_orchestrator.rs` is touched by W1/W5/W6 and `mod.rs` by W3/W8/W9 — these must NOT run
concurrently on the same file. Serialize per-file (one worker owns the file at a time) or split by function with explicit
line-range ownership. W1/W2 (determinism) run first and alone, each followed by a world_hash regression gate.

**Blocked until §5 live reads:** BR-07, BR-27, and the `[BSS]` numeric constants (Z-window lepton gap, drive/ship/walk Z).
**Separate track:** the CABHUT-C4 bug (§4) — investigation, not a mechanical fix.

---

## Provenance
Per-facet detail (full fixtures, decompile excerpts, every refuted/uncertain claim) lives in
`docs/research/bridges/_parity_scan/<facet>_findings.md` and `<facet>_verdicts.md` (16 facets).
This contract synthesizes the **adversarially-confirmed REAL** verdicts only.
