# Bridge Hut Fallback Flags/Ramp-Only - Ghidra Research Report

**Address(es):** `0x00574000` (`MapClass__DestroyBridge_High_OnHutDeath`), `0x00574C20` (`MapClass__DestroyBridge_Low_OnHutDeath`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** no-overlay hut fallback inside `DestroyBridge_{High,Low}_OnHutDeath` after the inner 5x5 overlay scan fails.
**Non-Scope:** overlay-found fast path, full `CollapseBridge_*_*` walker bodies, exact bridge-collapse sound, and trigger action `0x1F`.
**Confidence:** High for binary control flow; Medium for stock-map incidence.
**Active in YR:** Conditional. The functions are live in standard YR, but this fallback branch requires a hut-centered 5x5 with no matching bridge overlay and at least one `CellClass+0x140 & 0x500` cell at the input or within 3 cells.

## 0. Working Notes

- **Target question:** What exactly happens in the rare no-overlay bridge-hut fallback when only cell flags/ramp evidence exists?
- **Non-goals:** Do not re-investigate overlay-found collapse, bridge repair, sound, or event `0x1F`.
- **Evidence needed to mark COMPLETE:** direct decompile of both OnHutDeath bodies; helper decompile for ramp/endpoint tests and `ApplyDamageToCell`; caller liveness from C4/demo-truck paths; current Rust comparison and test handoff.
- **Stop conditions:** stop after no-overlay fallback starter selection, anchor math, ramp walk, retry counts, side effects, and Rust-facing deltas are resolved or explicitly deferred.

## 1. Overview

When the inner hut-centered 5x5 scan finds no bridge overlay in the selected bridge family, gamemd does not search for another overlay elsewhere. It uses cell flags at `CellClass+0x140` to pick a single starter cell, resolves an anchor from that starter, walks along a ramp/search axis, and calls `ApplyDamageToCell` in two bounded retry groups.

This is not a list-building or flood-fill fallback. It is a single-starter, single-walk path with specific early returns and tail side effects.

## 2. Class Layout / Key Offsets

| Field | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0x24` | cell coordinate used for starter/anchor | `0x00574000`, `0x00574C20` decompile | Yes |
| `CellClass+0x2C` | peer/owning bridge cell pointer used when `0x100` is set but `0x80` is clear | `0x00574000`, `0x00574C20` decompile | Conditional |
| `CellClass+0x38` | tile index used by ramp and endpoint probes | `0x005746C0`, `0x00574600` decompile | Conditional |
| `CellClass+0x44` | overlay index; fast path already failed in this scope | `0x00574000`, `0x00574C20` decompile | Yes |
| `CellClass+0x11A` | ramp/endpoint class byte used by `IsBridgeRampTile` / `IsLowBridgeEndpointTile` | `0x005746C0`, `0x00574600` decompile | Conditional |
| `CellClass+0x140` | flag word; fallback accepts `0x100` or `0x400` through mask `0x500`, then branches on `0x80` and `0x800` | `0x0057409D..0x00574231`, `0x00574CBD..0x00574E51` decompile/disassembly range | Conditional |
| `MapClass+0x124/+0x128/+0x12C/+0x130` | map left/top/width/height bounds for the ramp walk | `0x00574350..0x005745AD` decompile/disassembly range | Yes |
| `MapClass+0x13C` | per-cell pointer/index array gate before ramp probe | `0x005743B0..0x00574422` decompile/disassembly range | Conditional |

## 3. Core Logic

### 3.1 Inner 5x5 overlay scan, for contrast only

Both functions first run an overlay-only scan:

- High: overlay `0xCD..=0xE8`, calls `DestroyBridgeFromCell_High`, returns.
- Low: overlay `0x4A..=0x65`, calls `DestroyBridgeFromCell_Low`, returns.
- Loop order is column-major: `dx = -2..2` outside, `dy = -2..2` inside.

**Active in YR:** Yes. Verified in `BuildingClass__Update @ 0x0043FB20` and `BombClass__Detonate @ 0x00438720`, which call these OnHutDeath bodies when the target type has `BridgeRepairHut`.

### 3.2 Fallback starter selection

The no-overlay path starts from the original hut input cell:

1. Fetch the input cell. If out of map or null, use sentinel `DAT_00ABDC50` and write the requested coordinate to `DAT_00ABDC74`.
2. If `(flags & 0x500) != 0`, use that input cell as the starter. No 8-direction search runs.
3. Otherwise iterate direction index `0..7`; for each direction, check exactly 1, 2, then 3 steps from the original input coordinate.
4. The first probed cell whose `(flags & 0x500) != 0` becomes the starter. Search stops immediately.
5. After the search, if neither `0x100` nor `0x400` is present, return immediately. This early return does not call `UpdateBridgeZonesHelper`.

The direction table is `g_DirectionOffsets @ 0x0089F688`; prior direction-table research maps index order to `N, NE, E, SE, S, SW, W, NW`. The loop uses direction indices numerically `0..7`, and distance order is unrolled as 1, 2, 3 in each direction.

**Active in YR:** Conditional. The code is live in YR, but this branch only runs on no-overlay hut topologies. Loose local retail file search did not prove a stock loose map that forces this exact fallback; custom maps can force it by placing a `BridgeRepairHut` so the inner 5x5 sees no matching body overlay but nearby cell flags still identify bridge/ramp structure.

### 3.3 Accepted flags

The search acceptance mask is exactly `0x500`, meaning `0x100` or `0x400`:

- `0x100` is accepted for starter selection.
- `0x400` is accepted for starter selection.
- `0x80` is not accepted by itself; it only chooses the anchor source after a `0x100` starter is found.
- `0x800` is not accepted by itself; it only flips the pure-bridgehead and forward-walk directions.
- If the starter has neither `0x100` nor `0x400` after search, the function returns.

**Active in YR:** Conditional. Verified by decompile and disassembly range `0x0057409D..0x00574244` and low twin `0x00574CBD..0x00574E64`.

### 3.4 Anchor resolution

After a starter is selected:

| Starter flags | Anchor behavior | Evidence | Active in YR |
|---|---|---|---|
| `0x100` set and `0x80` set | anchor is starter cell's own `+0x24` coord | `0x00574332..0x00574346` / low twin | Conditional |
| `0x100` set and `0x80` clear | anchor is `(*(cell+0x2C)+0x24)` | `0x00574324..0x0057432E` / low twin | Conditional |
| `0x100` clear and `0x400` set | pure bridgehead branch; see below | `0x00574255..0x0057431B` / low twin | Conditional |

Pure bridgehead branch:

- Start from the starter cell's own `+0x24` coord.
- Compute `base = (flags & 0x800) ? 2 : 0`.
- Walk direction is `base + 2`: clear `0x800` walks direction `2` (E); set `0x800` walks direction `4` (S).
- Step one cell at a time while the next cell still has `0x400`.
- Count only cells that still have `0x400`; if four consecutive stepped cells still have `0x400`, return early.
- When the first non-`0x400` cell is reached after 0..3 flagged cells, anchor becomes that break cell plus two cells in the opposite direction: clear `0x800` uses direction `6` (W), set `0x800` uses direction `0` (N).

This corrects an attractive but wrong reading: the final two-cell offset is not "two more in the same direction." It is opposite the pure-bridgehead scan direction.

**Active in YR:** Conditional. The branch is live code in standard YR; it requires a starter with `0x400` and no `0x100`.

### 3.5 Ramp-search walk and retry counts

After anchor resolution:

1. Compute forward direction from starter flags: `(flags & 0x800) ? 6 : 0`.
2. Walk from `anchor` in that forward direction while inside map bounds.
3. For each cell, first require `MapClass+0x13C[cell_index] != 0`; only then call `MapClass__IsBridgeRampTile`.
4. On the first ramp tile, reverse direction with `(forward - 4) & 7`.
5. Call `ApplyDamageToCell(&ramp_cell)` up to three times, stopping early if it returns true.
6. Continue stepping in the reversed direction until `MapClass__IsLowBridgeEndpointTile` returns true or bounds fail.
7. If bounds fail after the first ramp, call `UpdateAdjacentBridges_High(anchor)`, set `g_Tactical+0xD7C = 1`, then call `UpdateBridgeZonesHelper`.
8. If an endpoint is found and `endpoint_tile_index - selected_tile_base != -2`, reverse direction again and call `ApplyDamageToCell` up to three times on the cell one step beyond the endpoint in the original forward direction. Stop early on true.
9. Then call `UpdateAdjacentBridges_High(anchor)`, set `g_Tactical+0xD7C = 1`, and call `UpdateBridgeZonesHelper`.

If no ramp tile is found before the initial forward walk exits bounds, the function still calls `UpdateBridgeZonesHelper`, but does not call `UpdateAdjacentBridges_High` and does not set `g_Tactical+0xD7C`.

**Active in YR:** Conditional. Verified by high decompile `0x00574350..0x005745CA`, low decompile `0x00574F6C..0x005751E6`, helper decompile `0x005746C0`, `0x00574600`, and `ApplyDamageToCell @ 0x00587180`.

### 3.6 `ApplyDamageToCell` dispatch relevance

`ApplyDamageToCell` does not simply "damage the selected coordinate." It re-fetches the cell and dispatches by overlay and bridge/ramp evidence:

- Low body overlays accepted by direct low destruction are `0x4A..=0x63` in this helper.
- High body overlays accepted by direct high destruction are `0xCD..=0xE6` in this helper.
- Otherwise it may route to high or low bridge damage state machines based on tile/ramp evidence and bridge flags.
- After state mutation it iterates the accumulated cell list and calls the C4 splash/occupant walker (`FUN_00487720`) for each listed cell.

**Active in YR:** Yes when fallback ramp walk reaches a ramp or endpoint damage site.

## 4. INI Keys

| Key | Default / source | Effect in this slice | Active in YR |
|---|---|---|---|
| `[CABHUT] BridgeRepairHut=yes` | `ini/rulesmd.ini:16348`; base `rules.ini:9460` | enables C4/demo-truck target to enter hut bridge destruction call path | Yes |
| `[CABHUT] TechLevel=-1` | `ini/rulesmd.ini` / `rules.ini` | stock type exists but is map-placed, not buildable | Yes |
| `[CABHUT] Immune=yes` | `ini/rules.ini:9452`; inherited unless patched | does not block the hut bridge destruction dispatch in verified caller paths | Yes |
| `[CombatDamage] DestroyableBridges=yes` | `ini/rulesmd.ini:804`; decorative for hut fallback | not read by the hut-death OnHutDeath bodies | Yes as INI data; No as a gate here |

## 5. Integration Points

Verified callers:

- `BuildingClass__Update @ 0x0043FB20`: when `BuildingClass+0x6DF` C4 marker is set, timer has elapsed, and `BuildingTypeClass+0x16B6` (`BridgeRepairHut`) is true, it runs the outer 5x5 low/high classifier and calls `DestroyBridge_Low_OnHutDeath` or `DestroyBridge_High_OnHutDeath`. Active in YR: Yes.
- `BombClass__Detonate @ 0x00438720`: after demo-truck area damage and explosion anim, if target RTTI is building and type `+0x16B6` is true, it runs the same outer classifier and calls the same OnHutDeath bodies. Active in YR: Yes.

Direct callees in fallback:

- `MapClass__IsBridgeRampTile @ 0x005746C0`
- `MapClass__IsLowBridgeEndpointTile @ 0x00574600`
- `ApplyDamageToCell @ 0x00587180`
- `MapClass__UpdateAdjacentBridges_High @ 0x00576770`
- `MapClass__UpdateBridgeZonesHelper @ 0x0056C510`

## 6. Current Rust Implementation Status

Current Rust surface:

- `src/sim/world/bridge_orchestrator.rs:176` precomputes `fallback_cells_lazy = find_hut_fallback_cells(...)`.
- `src/sim/world/bridge_orchestrator.rs:341` defines direction order as N, NE, E, SE, S, SW, W, NW; this matches prior direction-table research.
- `src/sim/world/bridge_orchestrator.rs:442` first checks the hut center, then directions and distances 1..=3.
- `src/sim/world/bridge_orchestrator.rs:472` traces multiple contiguous evidence cells along a direction, capped at `HUT_FALLBACK_TRACE_LIMIT = 12`.
- `src/sim/world/bridge_orchestrator.rs:685` applies generic per-cell state-machine damage to every returned fallback cell until a collapse appears.

Rust likely matches the high-level ability to collapse rare fallback layouts, but it is not exact. The binary chooses one starter cell and derives one anchor/ramp walk. Rust may over-walk by returning a trace of evidence cells, may use a destroy overlay outside the 5x5 fallback list as a direct collapse seed, and does not model the binary's pure `0x400` anchor math or two specific `ApplyDamageToCell` retry groups.

Existing tests include `c4_on_cabhut_bridgehead_fallback_collapses_bridge`, which is useful but too broad: it proves "some fallback collapses," not the exact starter, anchor, retry, or side-effect behavior.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| High OnHutDeath no-overlay fallback | verified | `0x00574000` decompile; disassembly range `0x00574000..0x0057463F` refreshed | none |
| Low OnHutDeath no-overlay fallback | verified | `0x00574C20` decompile | none |
| Starter selection 8 dirs x 3 steps | verified | `0x0057409D..0x00574231`; low twin | none |
| Accepted flag mask `0x500` | verified | `0x005740BA`, repeated probes in decompile | none |
| Pure `0x400` branch direction math | verified | `0x00574255..0x0057431B`; low twin | none |
| `0x100`/`0x80` anchor branches | verified | `0x00574324..0x00574346`; low twin | none |
| Ramp tile predicate | verified | `0x005746C0` decompile | semantic names of tile globals deferred |
| Endpoint predicate | verified | `0x00574600` decompile | semantic names of tile globals deferred |
| `ApplyDamageToCell` retry target behavior | verified | `0x00587180`; call sites in `0x0057447A`, `0x0057457C`, low twins | none |
| Standard-map incidence | touched-not-exhausted | loose retail `rg` found no CABHUT map evidence outside editor config | MIX-packed campaign/multiplayer maps not unpacked in this slot |
| Trigger event `0x1F` | deferred | explicitly outside slot | slot 3 target |
| Collapse sound | deferred | explicitly outside slot | slot 1 target |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does no-overlay fallback search overlays outside the 5x5? -> No; it searches `flags & 0x500`, not overlay bands.` (evidence: `0x0057409D..0x00574231`, `0x00574CBD..0x00574E51`)
- `[RESOLVED] OQ-02 - What order does the fallback starter search use? -> input cell first; if not accepted, direction indices 0..7 and distances 1,2,3 per direction.` (evidence: `0x0057409D..0x00574231`)
- `[RESOLVED] OQ-03 - Which flags are accepted? -> starter acceptance is `0x100|0x400`; `0x80` and `0x800` are modifiers only.` (evidence: `0x005740BA`, `0x00574231..0x00574346`)
- `[RESOLVED] OQ-04 - What happens if no accepted flag is found? -> return; no adjacent update, no tactical dirty, no zone helper.` (evidence: early return at `0x00574244` in high decompile; low twin)
- `[RESOLVED] OQ-05 - What is pure bridgehead direction math? -> clear `0x800`: walk E through up to 3 flagged bridgehead cells then anchor two W from the first non-flag cell; set `0x800`: walk S then anchor two N.` (evidence: `0x00574255..0x0057431B`; direction table report `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` section 11.7)
- `[RESOLVED] OQ-06 - How many `ApplyDamageToCell` retries? -> up to 3 on the first ramp cell, then conditionally up to 3 on one cell beyond the endpoint; each group stops on first true return.` (evidence: `0x0057447A..0x0057458E`; low twin)
- `[RESOLVED] OQ-07 - Does no-ramp bounds exit still rebuild zones? -> Yes, it reaches `UpdateBridgeZonesHelper`; it skips `UpdateAdjacentBridges_High` and `g_Tactical+0xD7C`.` (evidence: `LAB_005745CA` / `LAB_005751E6` decompile)
- `[RESOLVED] OQ-08 - Are these functions active in YR? -> Yes for C4/demo-truck on `BridgeRepairHut`; fallback branch conditional on topology.` (evidence: `BuildingClass__Update @ 0x0043FB20`; `BombClass__Detonate @ 0x00438720`; `rulesmd.ini:16348`)
- `[DEFERRED] OQ-09 - Which stock packed maps exercise fallback rather than overlay fast path?` (category: requires-different-system-context; reason: retail map content is packed; loose file grep did not prove incidence; next-step-if-pursued: unpack MIX maps and scan CABHUT + nearby overlay/tile data)
- `[DEFERRED] OQ-10 - Exact semantic names of tile-base globals used by ramp/endpoint predicates.` (category: out-of-scope; reason: control flow and constants are sufficient for fallback handoff; next-step-if-pursued: extend terrain tile global naming report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Fallback picks one starter cell: hut center if `flags&0x500`, else first direction 0..7 and distance 1..3 with `flags&0x500`; it does not trace a list of evidence cells. | `0x0057409D..0x00574231`, `0x00574CBD..0x00574E51` | mismatch likely: Rust `append_hut_fallback_trace` returns a contiguous evidence list up to 12 cells | `src/sim/world/bridge_orchestrator.rs:442`, `:472` | Make fallback semantics starter/anchor driven; avoid applying damage to arbitrary traced cells beyond the chosen binary starter/walk result. | Hut has no overlay in 5x5, center has no bridge flags, N distance 2 and E distance 1 both have flags; binary chooses N distance 2 because direction index 0 beats index 2. | Do not implement fallback as BFS, radius scan, or contiguous evidence trace. |
| Pure `0x400` starter anchors by walking E/S based on `0x800`, stops at first non-`0x400` after 0..3 flagged cells, then offsets two cells opposite. | `0x00574255..0x0057431B`; low twin | unchecked/mismatch likely: Rust routes bridgehead cells through `bridgehead_advance_state`, not this anchor math | `apply_hut_damage_to_cell`, bridgehead fallback fixtures/tests | Add a deterministic hut fallback scenario that exercises pure bridgehead-only flags and asserts the selected anchor/cell damage matches the binary branch. | Pure `0x400` bridgehead chain of 0, 1, 3, and 4 flagged continuation cells; 4 continuation cells no-op/returns. | Do not use bridgehead direct-damage rules as a substitute for hut fallback pure-flag anchor resolution. |
| Ramp fallback has two retry groups: <=3 `ApplyDamageToCell` calls on first ramp; then endpoint walk; if endpoint relative tile != -2, <=3 calls one cell past endpoint in original forward direction; side effects differ for no-ramp vs post-ramp bounds exits. | `0x00574350..0x005745CA`, `0x005746C0`, `0x00574600`, `0x00587180` | mismatch likely: Rust applies per-cell damage across fallback list and uses collapse outcome aggregation | `dispatch_bridge_collapse_from_hut`, `apply_hut_damage_to_cell`, test fixtures | Model/verify exact retry count and side-effect flags for no-ramp, ramp+endpoint, and ramp+OOB cases. | No-ramp fallback triggers zone rebuild but no bridge dirty/tactical dirty; ramp+OOB triggers adjacent update + tactical dirty + zone rebuild. | Do not always emit adjacent bridge dirty just because fallback was entered; do not skip zone rebuild on no-ramp exit. |

### Negative Facts / Do Not Do

- Do not search for matching bridge overlays outside the inner 5x5 as part of fallback. Evidence: fallback probes only `CellClass+0x140 & 0x500` after the overlay scan fails.
- Do not let `0x80` alone qualify a fallback starter. Evidence: search mask is `0x500`; `0x80` is only tested inside the `0x100` anchor branch.
- Do not interpret pure `0x400` final anchor offset as "two more in the same direction." Evidence: after walking E/S, decompile computes `(base - 2) & 7`, producing W/N for the final two-cell anchor offset.
- Do not treat all fallback exits as equivalent. Evidence: no accepted flags returns with no tail; no-ramp/bounds exit calls only `UpdateBridgeZonesHelper`; post-ramp exits also call `UpdateAdjacentBridges_High` and set `g_Tactical+0xD7C`.
- Do not implement fallback as full-span collapse directly from a flag cell. Evidence: the flag fallback uses `IsBridgeRampTile`, `IsLowBridgeEndpointTile`, and bounded `ApplyDamageToCell` groups; full collapse lives in the overlay-found/collapse walker subtree.

### Stale Docs / Follow-up Docs

- `docs/research/DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md`: replace the pure bridgehead row wording "else step 2 more cells in the same dir to land on anchor" with "else step two cells in the opposite direction from the first non-0x400 cell: clear 0x800 walks E then offsets W; set 0x800 walks S then offsets N."
- `docs/research/DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md`: replace "Global side effects (always, on the slow path): UpdateAdjacentBridges_High, Tactical+0xD7C, UpdateBridgeZonesHelper" with "On the slow path, `UpdateBridgeZonesHelper` runs after the ramp walk setup even when no ramp is found; `UpdateAdjacentBridges_High` and `g_Tactical+0xD7C=1` run only after a ramp was found or the post-ramp reverse walk exits through the adjacent-update label."
- `docs/research/BRIDGE_HUT_DESTRUCTION_ENTRY_DECODE_GHIDRA_REPORT.md`: replace the pure bridgehead pseudocode line `anchor = coord + 2 more steps in (dir2 - 2) & 7` with the explicit E->W / S->N wording above.

## Sources

- Ghidra decompile: `0x00574000` `MapClass__DestroyBridge_High_OnHutDeath`
- Ghidra decompile: `0x00574C20` `MapClass__DestroyBridge_Low_OnHutDeath`
- Ghidra decompile: `0x005746C0` `MapClass__IsBridgeRampTile`
- Ghidra decompile: `0x00574600` `MapClass__IsLowBridgeEndpointTile`
- Ghidra decompile: `0x00587180` `ApplyDamageToCell`
- Ghidra decompile: `0x0043FB20` `BuildingClass__Update`
- Ghidra decompile: `0x00438720` `BombClass__Detonate`
- Ghidra disassembly refresh: `0x00574000..0x0057463F`
- Prior direction table: `docs/research/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` section 11.7
- Prior body report: `docs/research/DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md`
- Prior hut entry report: `docs/research/BRIDGE_HUT_DESTRUCTION_ENTRY_DECODE_GHIDRA_REPORT.md`
- Current Rust: `src/sim/world/bridge_orchestrator.rs`
- Current tests: `src/sim/world/world_orders_bridge_repair_tests.rs`
