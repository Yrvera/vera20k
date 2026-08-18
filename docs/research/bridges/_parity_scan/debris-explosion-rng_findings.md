# Parity Scan — debris-explosion-rng (Bridge debris/explosion spawn RNG)

Facet: BlowUpBridge debris spawn site + CollapseBridge hut-walker explosion spawn.
DETERMINISM-SENSITIVE — RNG draw count/order feed the shared scenario stream and world_hash.

Rust under test: `src/sim/world/bridge_orchestrator.rs`
  - `spawn_bridge_debris` (line 1167)
  - `bridge_jittered_subcells` / `bridge_jittered_subcell` (lines 1295 / 1304)
  - `spawn_bridge_explosion_effect` (line 1254)
  - `spawn_hut_walker_pre_destroy_effects` (line 804)
  - gate constants (lines 371-377)

gamemd anchors verified live this session:
  - `CellClass__BlowUpBridge @ 0x0047DD70` (get_function_by_address confirmed name+body 0x0047dd70-0x0047e036)
  - `MapClass__CollapseBridge_NS_High @ 0x00575BA0` (decompile + disassemble)
  - `Random__RandomRanged @ 0x0065C7E0` (decompile)
  - `Math__ftol @ 0x007C5F00` (decompile — it is ROUND, not truncate)
  - constants `0x007e3570` (scale), `0x007e1738` (0.5), `0x007e4f50` (span 50.0), `0x007e4f58` (0.95) read via read_memory

## Verified binary RNG draw order (BlowUpBridge @ 0x0047DD70), per cell that passes the count gate

1. `Random__RandomRanged(0, 0x7FFFFFFE)` → `r0`; outer gate: pass iff `r0 * scale < 0.95` (FCOMP `0x007e4f58`, strict `<`). [0x0047de54 .. 0x0047de72]
2. `Random__RandomRanged(0, 0x7FFFFFFE)` → jitter X → `ftol((r1*scale - 0.5)*50 + cellX*256+128)` (ROUND). [0x0047dec6 .. 0x0047dee9]
3. `Random__RandomRanged(0, 0x7FFFFFFE)` → jitter Y → `ftol(...)`. [0x0047df04 .. 0x0047df27]
4. `Random__RandomRanged(0, 0x7FFFFFFE)` → `r3`; metallic gate: pass iff `r3 * scale < 0.5` (FCOMP `0x007e1738`, strict `<`). [0x0047df43 .. 0x0047df61]
5. (only if metallic gate passed AND alloc != 0) `Random__RandomRanged(0, MetallicDebris.count - 1)` → metallic slot; AnimClass__Constructor delay arg = 0. [0x0047df91]
6. `Random__RandomRanged(1, 5)` → explosion delay/start-frame `uVar6`. [0x0047dfe1]
7. `Random__RandomRanged(0, BridgeExplosions.count - 1)` → explosion slot; AnimClass__Constructor delay arg = `uVar6`. [0x0047e004]

Order/count of draws (1,2,3,4, [5], 6, 7) — the Rust `spawn_bridge_debris` reproduces this order exactly (outer, jitterX, jitterY, metallic gate, [metallic slot], delay, explosion slot). Order is PARITY-CONFIRMED; the disparities below are in the gate *threshold constants* and the *jitter arithmetic*, not the draw order.

---

### D1: Outer 95% gate threshold off by 2 (lockstep desync at boundary draws)
- Rust now: `BRIDGE_DEBRIS_OUTER_GATE_EXCLUSIVE = 2_040_109_466` (line 373); gate fails iff `outer_draw >= 2_040_109_466`, i.e. PASSES for `outer_draw <= 2_040_109_465` (line 1182).
- gamemd: `BlowUpBridge @ 0x0047DD70`, `FILD; FMUL[0x007e3570]; FCOMP[0x007e4f58]; FNSTSW; TEST AH,1; JZ skip` — passes iff `(double)r0 * scale < 0.95`. With the actual double `scale = 4.656612877414201e-10` and `0.95`, the largest passing integer is **2_040_109_463** (`2040109463*scale = 0.94999999967 < 0.95` PASS; `2040109464*scale = 0.95000000014 >= 0.95` FAIL). So the correct Rust fail-threshold constant is `2_040_109_464` (pass iff `draw <= 2_040_109_463`).
- Fixture: `outer_draw = 2_040_109_464`. Binary: `* scale = 0.9500000001 >= 0.95` → FAIL → spawns NO debris/explosion, consumes only this 1 draw. Rust: `2040109464 < 2040109466` → PASS → proceeds to draw jitterX/jitterY/metallic-gate/(slot)/delay/explosion-slot (6-7 more draws) and spawns anims. Same divergence at `outer_draw = 2_040_109_465`.
- Player sees: at exactly these two draw values, one client spawns a full debris+explosion burst and the other spawns nothing; in lockstep the divergent draw counts (1 vs 7) desync the shared scenario RNG for the rest of the match. Trigger frequency: ~2 in 2.1e9 draws per collapsed cell — rare, but every bridge collapse rolls this gate, and a single hit cascades into a full desync.
- Severity: MED (rare trigger, but a hit is a hard multiplayer desync, not a cosmetic glitch)
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x0047DD70`; `read_memory 0x007e3570` (scale) + `0x007e4f58` (0.95); boundary computed `2040109463*scale < 0.95 < 2040109464*scale`.

### D2: MetallicDebris 50% gate threshold off by 1 (lockstep desync at boundary draw)
- Rust now: `BRIDGE_METALLIC_GATE_EXCLUSIVE = 0x4000_0000` (= 1_073_741_824, line 374); `metallic_pass = metallic_draw < 0x4000_0000`, i.e. passes for `metallic_draw <= 1_073_741_823` (line 1200).
- gamemd: `FCOMP[0x007e1738]` (= exactly 0.5), strict `<`. Pass iff `r3 * scale < 0.5`. Largest passing integer is **1_073_741_822** (`1073741822*scale = 0.49999999953 < 0.5` PASS; `1073741823 (=0x3FFFFFFF) * scale = 0.5 exactly` → `0.5 < 0.5` FALSE → FAIL). Correct Rust constant is `0x3FFF_FFFF` (= 1_073_741_823) so pass iff `draw < 0x3FFFFFFF`.
- Fixture: `metallic_draw = 0x3FFF_FFFF (1_073_741_823)`. Binary: `0.5 < 0.5` is false → metallic gate FAILS → NO metallic slot draw (step 5 skipped). Rust: `1073741823 < 0x40000000` → PASS → draws `next_range_u32(metallic_count)` for the slot AND spawns a MetallicDebris anim.
- Player sees: at this exact draw value one client spawns a metallic-debris sprite + consumes an extra RNG draw the other does not → lockstep desync. Trigger frequency: 1 in 2.1e9 metallic-gate rolls; every collapsed cell that passes the outer gate rolls it.
- Severity: MED (rare trigger; a hit is an RNG-stream desync via the extra slot draw)
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x0047DD70`; `read_memory 0x007e1738` (= 0.5) + `0x007e3570` (scale); boundary `1073741822*scale < 0.5`, `0x3FFFFFFF*scale == 0.5`.

### D3: Jitter sub-cell offset uses floor + wrong divisor instead of round-to-nearest (sub-cell render drift, asymmetric range)
- Rust now: `bridge_jittered_subcell` (line 1304): `offset = ((draw * 50) / 0x8000_0000) as i32 - 25`, result added to `CELL_CENTER_LEPTON (128)`. This is integer FLOOR of `draw*50/2^31`, then `-25`. Offset range = `[-25, +24]` (max draw 0x7FFFFFFE → `floor(49.99…)=49 → +24`). Divisor is `2^31`, the offset is never rounded.
- gamemd: `FILD r; FMUL scale(=1/(2^31-1)); FSUB 0.5; FMUL 50.0; FIADD (cell*256+128); ftol(=ROUND-to-nearest)`. Offset = `round((r/(2^31-1) - 0.5) * 50)`, range `[-25, +25]`. (`Math__ftol @ 0x007C5F00` = `ROUND(ST0)`, not truncate.) Same arithmetic in CollapseBridge walker (verified disassembly 0x00575d3f-0x00575d98).
- Fixture A: `draw = 0x66666666 (1_717_986_918)`. Binary: `r/(2^31-1) = 0.80000000019`, `(0.8-0.5)*50 = 15.0000000093`, round → 15, sub = 143. Rust: `1717986918*50 = 85_899_345_900`, `/2^31 = 39.9999… → floor 39`, `39-25 = 14`, sub = 142. **1-lepton off.**
- Fixture B (max boundary): `draw = 0x7FFFFFFE`. Binary: `0.99999999953*50 - 25 = 24.99999998` → round 25 → sub = 153. Rust: `floor(49.99…) - 25 = 24` → sub = 152. Binary can emit +25; Rust's range tops out at +24. **Systematic asymmetry: Rust offset ∈ [-25,+24], binary ∈ [-25,+25].**
- Player sees: every spawned MetallicDebris / BridgeExplosion sprite sits at a sub-cell position up to ~1 lepton (sub-pixel to ~1px isometric) off from gamemd for most draws, and never reaches the +25-lepton extreme. Render-only — does NOT consume extra draws, so no lockstep impact. Fires on every debris/explosion spawn of every collapse.
- Severity: LOW (sub-pixel placement, every collapse, not lockstep-affecting)
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x007C5F00` (ftol=ROUND); `disassemble_function 0x0047DD70` (0x0047decf-0x0047dee9 jitter X chain); `read_memory 0x007e4f50` (= 50.0), `0x007e3570` (scale), `0x007e1738` (0.5).

### D4: CollapseBridge hut-walker explosion jitter shares D3's floor-vs-round drift
- Rust now: `spawn_hut_walker_pre_destroy_effects` (line 804) → `spawn_bridge_explosion_effect` (line 1254) calls the same `bridge_jittered_subcells` (line 1263), so it inherits the D3 floor/divisor/asymmetry error for every hut-collapse walker explosion.
- gamemd: `CollapseBridge_NS_High @ 0x00575BA0` inner 3-perp loop uses the identical `FILD;FMUL scale;FSUB 0.5;FMUL 50;FIADD center;ftol(ROUND)` chain at 0x00575d3f-0x00575d59 (X) and 0x00575d7e-0x00575d98 (Y).
- Fixture: any `draw` where D3 diverges (e.g. 0x66666666 → 1-lepton off) on each of the 3 perpendicular explosion sprites per walker step.
- Player sees: same sub-pixel placement drift as D3, on the per-cell explosion sprites spawned during CABHUT-triggered bridge collapse. Render-only, no draw-count change.
- Severity: LOW (sub-pixel, only on hut-death collapses)
- Confidence: PROVEN-DRIFT
- Verify-call: `disassemble_function 0x00575BA0` (0x00575d3f-0x00575d98).

### D5: MetallicDebris slot draw guarded by `count > 0` (binary does not guard — modded empty list)
- Rust now: line 1204 — metallic slot draw + spawn only when `metallic_pass && metallic_count > 0`.
- gamemd: `BlowUpBridge` calls `Random__RandomRanged(0, Rules+0x14c - 1)` (= `RandomRanged(0, metallic_count-1)`) inside `if (metallic_gate_pass && operator_new != 0)` — there is NO `count > 0` precheck (0x0047df7c reads count, 0x0047df82 DEC, 0x0047df91 CALL). With `metallic_count == 0` the call becomes `RandomRanged(0, -1)` which (params differ → sorted to (-1,0), span 1) still consumes a draw.
- Fixture: modded `MetallicDebris=` empty (count 0), `BridgeExplosions` non-empty, `metallic_draw` passes the 50% gate. Binary: consumes a `RandomRanged(0,-1)` draw (then indexes a 0-length vector — crash/UB territory). Rust: skips the draw entirely. Stock YR `MetallicDebris` has 20 entries, so this never fires in stock play.
- Player sees: nothing in stock YR. Only a modded empty `MetallicDebris=` diverges (and the binary itself is unsafe there). Matches deferred OQ-10 in `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md`.
- Severity: LOW (stock YR never triggers; binary behavior is itself UB)
- Confidence: LIKELY-DRIFT (binary draw-on-empty path not runtime-confirmed; static read shows no count guard)
- Verify-call: `disassemble_function 0x0047DD70` (0x0047df7c-0x0047df91, no count test between gate-pass and slot CALL).

### D6: Hut-walker perpendicular spawn skips off-map-coordinate cells; binary always spawns 3 (consuming draws)
- Rust now: `spawn_hut_walker_pre_destroy_effects` (line 828) — `for delta in [-1,0,1] { if let Some((rx,ry)) = step_axis(...) { spawn_bridge_explosion_effect(...) } }`. `step_axis` returns `None` only at u16 numeric bounds (coord 0 or 65535), in which case NO spawn and NO draws for that perpendicular cell.
- gamemd: `CollapseBridge_NS_High` always runs the inner `iVar10=3` loop 3 times; an off-map cell falls back to the `&DAT_00abdc50` sentinel cell (0x00575cd6) and STILL consumes jitterX+jitterY+R(1,5)+slot = 4 draws and spawns the anim against the sentinel.
- Fixture: a bridge cell at column 0 (rx=0), perpendicular axis EW: `step_axis((0,ry), EW, -1)` → `None` in Rust → 1 perp skipped, 4 fewer draws than the binary for that walker step → RNG desync.
- Player sees: only when a collapsing bridge cell sits at absolute map coordinate 0 or 65535 — i.e. the literal map corner, which playable bridges never occupy. Effectively unreachable in normal YR maps.
- Severity: LOW (unreachable coordinate boundary in practice)
- Confidence: LIKELY-DRIFT (boundary reachability not exhaustively proven for all map layouts)
- Verify-call: `decompile_function 0x00575BA0` (inner 3-iteration loop with sentinel fallback `0x00575cd6`/`0x00575cdc`).

---

## PARITY-CONFIRMED

- **Draw order** in `spawn_bridge_debris`: outer gate → jitterX → jitterY → metallic gate → [metallic slot] → explosion delay → explosion slot. Matches BlowUpBridge 0x0047DD70 exactly (1,2,3,4,[5],6,7). VERIFIED via decompile.
- **Draw count** per cell: 4 unconditional draws (outer, jitterX, jitterY, metallic gate) + 1 conditional (metallic slot iff gate passes) + 2 always (delay, explosion slot when explosion_count>0). Matches binary. The two jitter draws are taken even though only used for render offset — Rust takes them too (lines 1187 / 1296-1297). VERIFIED.
- **Outer gate is on BridgeExplosions count, not BridgeVoxelMax**: Rust gates whole helper on `explosion_count == 0` early-return (line 1173); binary gates on `Rules+0x168 = BridgeExplosions.ActiveCount > 0` (0x0047de33). Match (BridgeVoxelMax correctly NOT consulted). VERIFIED.
- **Metallic vs Explosion ordering**: MetallicDebris (delay 0, immediate) spawns BEFORE BridgeExplosion (delayed) — Rust lines 1204 then 1228; binary metallic block 0x0047df63 then explosion block 0x0047dfbf. Match. VERIFIED.
- **Explosion delay range** `R(1,5)` and **slot** `R(0,count-1)`: Rust `next_range_u32_inclusive(1,5)` (line 1229) + `next_range_u32(explosion_count)` (line 1230). Binary `RandomRanged(1,5)` (0x0047dfe1) + `RandomRanged(0, count-1)` (0x0047e004). Match. VERIFIED.
- **Metallic delay = 0 / explosion delay = R(1,5)**: AnimClass__Constructor 3rd arg is 0 for metallic, `uVar6` for explosion (0x0047df9c push 0, 0x0047e009 push ESI). Rust `delay_ms: 0` for metallic, `delay_frames*ms` for explosion. Match (delay→ms unit conversion at 67ms/frame ≈ 1000/15fps; reasonable, not bit-checked).
- **`next_range_u32_inclusive` rejection sampling** matches `Random__RandomRanged` (mask = top-set-bit+1 bits, reject if `> span`): for span 0x7FFFFFFE both mask 0x7FFFFFFF and reject only 0x7FFFFFFF; for span 19 (metallic count 20) both mask 0x1F and reject 20..31; for span 3 (explosion count 4) both mask 3, no rejection. So draw counts stay in lockstep across rejections. VERIFIED (rng.rs:131-162 vs Random__RandomRanged 0x0065C7E0).
- **Shared RNG stream**: Rust uses `sim.bridge_rng()` for the BlowUpBridge path and `sim.scenario_rng` (the same stream) for the hut path; binary uses `Scenario+0x218` (one stream) everywhere. No separate stream. VERIFIED (no second RNG object).
- **Hut-walker explosion draw sequence** (jitterX, jitterY, R(1,5) delay, R(0,count-1) slot — NO outer gate, NO metallic) matches `spawn_bridge_explosion_effect` (lines 1263-1267). VERIFIED via CollapseBridge_NS_High 0x00575BA0.
- **Hut-walker perpendicular visit order** `-1, 0, +1` matches binary (DEC then INC, 0x00575c93/0x00575e17). VERIFIED.
- **Hut-walker terminal-cap skip**: center overlay == 0xE8 (NS_High) skips the 3-perp explosion block; Rust `hut_walker_terminal_cap(High,NS)=0xE8` returns early. Match for NS_High. VERIFIED.

## UNCHECKED

- **Z/level value for spawned anims.** Binary `local_4 = (char)cell[0x11b] * leptons_per_level`; Rust uses `bridge_deck_level_if_any().unwrap_or(c.level)`. The exact Z lepton value vs Rust `z: deck_level` (a `u8`) is render-only and not cross-walked here. Likely a separate render-Z facet.
- **Explosion delay frame→ms conversion exactness.** `BRIDGE_EFFECT_FRAME_MS = 67` vs gamemd anim delay measured in logic frames (15fps → 66.667ms). Off by 0.333ms/frame accumulating to ~1.7ms at 5 frames — observable timing question deferred to an anim-timing facet; the *draw* (R(1,5)) itself matches.
- **EW/Low hut-walker terminal caps** (`hut_walker_terminal_cap` returns 0xE7/0x64/0x65). Only NS_High (0xE8) cross-checked against the binary here; EW_High/NS_Low/EW_Low caps belong to the hut-collapse-walker facet and were not decompiled this session.
- **D5 modded-empty-list runtime behavior** (RandomRanged(0,-1) then 0-length index) — static-only; would need a debugger run on a modded INI (matches OQ-10 DEFERRED). Stock YR unaffected.
