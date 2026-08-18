# Rim-Refresh Parity Scan — Adversarial Verdicts

Facet: rim refresh (`UpdateAdjacentBridges` + edge-tile re-selection + stub reset).
Rust under audit: `src/sim/world/bridge_orchestrator.rs::update_adjacent_bridges` (lines 1024–1109).
Auditor stance: refute each disparity; REAL only if the gamemd reading holds live AND output differs.

Live re-confirmation done this session:
- `get_function_by_address 0x00576770` → `MapClass__UpdateAdjacentBridges_High`, body `0x576770–0x576b99`. OK.
- `get_function_by_address 0x00571050` → `MapClass__UpdateAdjacentBridges` (Low), body `0x571050–0x57148f`. OK.
- `get_function_by_address 0x00576200` → `MapClass__UpdateBridgeEdgeTiles_High`, body `0x576200–0x576764`. OK.
- `get_function_by_address 0x00570ae0` → `MapClass__UpdateBridgeEdgeTiles_Low`, body `0x570ae0–0x571044`. OK.
- `decompile_function 0x00576770`, `0x00576200`, `0x00570ae0`, `0x0047e040`, `0x00575ee0`, `0x00574c20`.
- `get_function_callers 0x00576770` and `0x00571050`.

---

## D1: Wrong algorithm entirely (stub-blank vs edge-tile re-eval) — VERDICT=REAL

Live `decompile_function 0x00576770`: `UpdateAdjacentBridges_High` does Phase A (8-dir walk over
`g_DirectionOffsets`, break on `*(puVar6+0x140) & 0x500 != 0`), then classifies the matched cell's
flag bits (`0x100`/`0x400`/`0x80`/`0x800`) to pick a START COORD, then Phase C walks that coord via
direction offset, bounded by map-rect tests (`DAT_0087f8dc`/`DAT_0087f8e0`), and on the first
matching ramp-class pattern calls `MapClass__UpdateBridgeEdgeTiles_High(&param_2, mode∈{2,4},
&local_10)`, then `TacticalClass__DirtyScreenRect`, and returns. The function writes ZERO cell
fields — it only mutates `local_10..local_4` (the dirty rect) and reads `puVar6[0x11a]`/`+0x140`.
Confirmed independently: there is NO `overlay=0xFF`, NO `deck_present`, NO span-blanking write
anywhere in the body. Rust (`bridge_orchestrator.rs:1052–1108`) instead walks and writes
`overlay_byte=0xFF`, `damage_state=Healthy`, `bridge_group_id=None`, `deck_present=false` on dead-span
cells. Different algorithm, different observable surviving-stub art on every span collapse.
Corrected delta: Rust `per-cell stub-blank pass` → gamemd `coord-classify + UpdateBridgeEdgeTiles_*
+ DirtyScreenRect; the function itself writes no cell fields`.

## D2: No edge-tile re-selection — `UpdateBridgeEdgeTiles_*` missing — VERDICT=REAL

Live `decompile_function 0x00576200` / `0x00570ae0`: `UpdateBridgeEdgeTiles_{High,Low}` is the real
state writer. Forward ramp search `do {…} while (local_44 < 0x1e)`; `param_3==2` matches
`DAT_00abc1e8`/`DAT_00aa0e38`/`DAT_00abad30[..+3]` with `puVar15[0x11a]=='\x04'`; `param_3==4` matches
`DAT_00abc1d0`/`DAT_00aa1540`/`DAT_00aa1028[..+3]` with `puVar15[0x11a]=='\x02'`; accumulates a dirty
rect via `TacticalClass__CoordsToClient2`; then a back-walk over `local_44` cells inspecting
`*(puVar15+0x140) & 0x80` transitions. Rust has no analog — `update_adjacent_bridges` blanks stubs and
stops; orchestrator header (line 9, 122) calls it a stub. Confirmed missing. REAL.
Corrected delta: Rust `(absent)` → gamemd `30-cell ramp-class forward search + dirty-rect union +
flag-transition back-walk that re-stamps cap / clears one edge cell`.

## D3: Reset condition + field set diverge — VERDICT=REAL

Live `decompile_function 0x00576200`: when the back-walk hits a `flags&0x80` set→clear transition
(`uVar13==0 && bVar10==0`), gamemd writes `puVar15[0x11e]=0`, `*(puVar15+0x44)=0xffffffff` (overlay
−1, full i32), `RadarClass__MarkTerrainDirty`, and FIRST calls
`CellClass__SetBridgeDirection_NESW(uVar17, 0)` with `uVar17=(uVar3==2 ? 0 : 6)`.
Live `decompile_function 0x0047e040`: `SetBridgeDirection_NESW(param_2, 0)` is a MULTI-cell group
edit — AND-masks `Flags` (`& 0xfffee07f` etc.) on the anchor, then with `param_2<8` walks the anchor
+ 3 forward neighbors via `g_DirectionOffsets[param_2]` and the `(param_2-4)&7` opposite neighbor,
writing `field_0x11e` and (when `param_3==0`/`cVar14=='\0'`) calling `CellClass__BlowUpBridge` per
cell. gamemd never sets a "deck_present" field; it clears structural/head flag bits via the AND-mask.
Rust (`bridge_orchestrator.rs:1101–1106`) edits ONE cell (`overlay_byte=0xFF`, `deck_present=false`,
`damage_state`, `bridge_group_id`) and leaves neighbor structural flags untouched. REAL.
Note: finder's generic "param_2≠0" framing is loose (the call passes `uVar17∈{0,6}`, and with
`uVar17=0` the anchor's `+0x11e` is set to 0 not 9), but its cited fixture (mode 2 ⇒
`SetBridgeDirection_NESW(0,0)`) is correct and the multi-cell-group-clear conclusion holds.
Corrected delta: Rust `1-cell overlay-byte+deck_present blank` → gamemd `multi-cell flag-mask
group-clear (anchor + 3 fwd + 1 opposite) + per-cell +0x11e + overlay=−1(i32) + MarkTerrainDirty`.

## D4: No `RepairBridgeSegment` cap re-stamp — VERDICT=REAL (re-stamp) / TS-legacy (event 31)

Live `decompile_function 0x00576200`: `RepairBridgeSegment(uVar1, local_40)` is called once,
latched by `bVar2`, on the back-walk's was-clear→now-set `flags&0x80` transition (`bVar11 & 1 != 0`).
Live `decompile_function 0x00575ee0`: `RepairBridgeSegment` walks the span between two endpoints and,
for every cell with `*(puVar4+0x3c)!=0` (occupant), fires `TechnoClass__ProcessCellAction(0x1f, …)`
= event 31. Rust `notify_bridge_span_collapse` (line 1120) is an explicit no-op `let _=(sim,cells)`.
The cap-re-stamp side IS a real player-visible gap; the event-31 broadcast is correctly classified
TS/campaign-only (no bound trigger in skirmish — matches CLAUDE.md filter). REAL for the re-stamp.
Corrected delta: Rust `(no-op)` → gamemd `back-walk latched RepairBridgeSegment over endpoint span
(cap re-stamp), which also broadcasts event 31 on occupied cells (event 31 itself skirmish-invisible)`.

## D5: Rust skips Destroyed + breaks on !deck_present; binary has no such control flow — VERDICT=REAL

Live `decompile_function 0x00576770`: Phase C is `if (occupied cell) break; else goto LAB_00576a74`
(advance one cell via `g_DirectionOffsets[uVar8]`) with the only termination being map-rect bounds or
a ramp-pattern match → `UpdateBridgeEdgeTiles_High`. No "skip Destroyed and continue", no
`deck_present` break. The forward search in `UpdateBridgeEdgeTiles_High` terminates at `local_44<0x1e`
or a ramp match; the back-walk terminates on the `flags&0x80` transition. Rust
(`bridge_orchestrator.rs:1086–1093`) `break`s on `!cell.deck_present` and `continue`s past
`DamageState::Destroyed`. No binary counterpart; visits/mutates a different cell set. REAL (subsumed
under D1's wrong-algorithm but a distinct concrete control-flow divergence).

## D6: 30-cap on the wrong walk — VERDICT=REAL

Live `decompile_function 0x00576200`/`0x00570ae0`: `local_44 < 0x1e` (=30) is the bound of the FORWARD
ramp-search loop inside `UpdateBridgeEdgeTiles_*`. Live `decompile_function 0x00576770`:
`UpdateAdjacentBridges_High`'s Phase-C walk has NO 30-cap — it's bounded only by the map-rect tests
(`DAT_0087f8dc`, `DAT_0087f8dc + DAT_0087f8e0*2`) and a pattern match. Rust applies `WALK_LIMIT=30`
(line 1040) to its stub-blank walk. Numerically right value, attached to a loop that doesn't exist in
the same function. On a >30-cell linear bridge the touched region differs. REAL (low frequency).

## D7: Caller set + High-on-Low quirk — VERDICT=REAL

Live `get_function_callers 0x00576770`: `UpdateAdjacentBridges_High` callers =
`DestroyBridge_High_OnHutDeath @ 0x574000`, `DestroyBridge_Low_OnHutDeath @ 0x574c20`,
`ProcessBridgeDamageStateMachine_High @ 0x576ba0`. `get_function_callers 0x00571050`:
`UpdateAdjacentBridges` (Low) caller = `ProcessBridgeDamageStateMachine_Low @ 0x571490` ONLY.
Live `decompile_function 0x00574c20`: `DestroyBridge_Low_OnHutDeath` ends at `LAB_005751c9` with
`MapClass__UpdateAdjacentBridges_High(&local_30)` — the High rim refresh invoked from a LOW
hut-death path. Vanilla "High-on-Low" quirk confirmed. Note: finder labels the hut-death funcs
`_MapInit @ 0x574000/0x574c20`; the live Ghidra names are `_OnHutDeath` and the plate comment states
the `_MapInit` suffix was a mis-label (the path is runtime hut-death, C4-timer / demo-truck). Same
addresses, same behavior — finder used a stale label name only. Rust `update_adjacent_bridges` is
family-agnostic, masking rather than reproducing the quirk; latent until D1/D2 are implemented. REAL
(fix-time constraint).

---

## PARITY-CONFIRMED items — re-checked, all hold

- **Rim-cell seed = 2 perpendicular neighbors.** Not re-verified against the binary's "×2" seed this
  session beyond the StateMachine caller existing; finder's claim consistent with the live caller set
  and not contradicted. (Seed origin lives in `compute_adjacent_bridges_dirty`, mod.rs:2028, not this
  facet's function — accepted as PARITY, not independently REAL/REFUTED here.)
- **8-dir Phase-A order.** Live `0x00576770` indexes `g_DirectionOffsets` via `uVar8 & 7` starting at
  0 — N,NE,E,SE,S,SW,W,NW. Matches Rust `DIRECTIONS` (line 1041). Stop condition `flags & 0x500`
  (bit 8 OR bit 10) vs Rust `role==Bridgehead || Destroyed` is same-intent but NOT bit-proven equal —
  see MISS below.
- **30 (0x1e) numeric value.** Verified `local_44 < 0x1e` in both edge-tiles funcs. Correct number,
  wrong loop (D6).
- **NESW (High) vs NWSE (Low) split.** Live `0x00576200` calls `SetBridgeDirection_NESW`; `0x00570ae0`
  calls `SetBridgeDirection_NWSE`. Split confirmed. Rust has neither yet.

## MISS (auditor-found, not raised by finder)

- **MISS [LOW, latent]:** Phase-A stop condition is `flags & 0x500` = bit 0x100 (anchor/structural)
  **OR** bit 0x400 (bridgehead) — NOT "Bridgehead OR Destroyed". The Rust analog uses
  `role==Bridgehead || damage_state==Destroyed` (line 1064–1065). The binary's bit 0x100 is the
  structural/anchor head flag (see `SetBridgeDirection_NESW` mask comments at `0x0047e040`), and
  Destroyed-state is tracked via `+0x11e` / overlay band, NOT a `0x500` flag bit. So the Rust
  "Destroyed" arm of the Phase-A predicate has no direct binary counterpart, and the binary's
  "0x100 structural anchor" arm is not the same set as Rust's `Bridgehead` role. The finder folded
  this into PARITY ("same intent, not separately verified"); it is in fact a concrete predicate
  divergence that selects a different START cell, hence a different downstream coord. Latent under D1
  (whole algorithm is being replaced) but should be captured in the eventual fix so Phase-A keys on
  the `0x100|0x400` flag union, not role+damage_state.
- **MISS [INFO]:** `UpdateAdjacentBridges_High` Phase A, when the matched head has bit `0x100` set
  but bit `0x80` clear, takes `param_2 = *(short**)(*(int*)(puVar6+0x2c)+0x24)` — i.e. it follows the
  cell's `+0x2c` link (the anchor back-pointer set by `SetBridgeDirection`) to get the start coord,
  rather than walking. The Rust port has no `+0x2c` anchor-link traversal in this path. Subsumed by
  D1 but worth capturing for the fix: the start-coord selection is a pointer chase, not a walk.
