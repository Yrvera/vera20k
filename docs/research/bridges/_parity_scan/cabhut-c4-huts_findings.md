# CABHUT C4 trigger + repair-hut death path — Parity Scan Findings

**Facet:** cabhut-c4-huts — SEAL/Tanya C4 on bridge repair hut (CABHUT) → hut "death" → bridge collapse.
**Date:** 2026-05-29
**Authority:** live Ghidra (gamemd.exe) > labels > docs. Every load-bearing gamemd claim below was decompiled this session.

## Executive root-cause

The downstream half of the loop is implemented correctly: once a building's
`pending_c4_detonation` fires, `apply_c4_damage_to_building`
(`src/sim/world/world_orders.rs:760`) detects `bridge_repair_hut` BEFORE any
damage/invuln gate, leaves the hut at full HP, and dispatches
`dispatch_bridge_collapse_from_hut` (`src/sim/world/bridge_orchestrator.rs:151`),
which runs the bounded 4-step `CollapseBridge_*_*` walker. That all matches the binary.

**The loop is broken at the FRONT: a SEAL/Tanya is prevented from ever setting
`c4_plant` on a CABHUT, so `pending_c4_detonation` is never set and the dispatch
never runs.** The Rust order/cursor pipeline gates the C4 plant on the target
being a non-friendly (`HoverTargetKind::EnemyStructure`) building and explicitly
rejects ally-owned targets. gamemd's plant gate
(`InfantryClass::PerCellProcess` Mission 0x11 @ `0x00519630`) and its cursor
action resolver (`InfantryClass::What_Action_OnObject` action `0x10`
@ `0x0051e3b0` → `TechnoClass::What_Action_OnObject` @ `0x006ffec0`) have **NO
ownership/ally check and NO Immune check** on the C4-on-building path. A CABHUT
is a Neutral/Special-owned bridge hut, so the Rust ally/enemy gate is the exact
upstream blocker that the prior research
(`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §15.2) predicted "must be
upstream of PerCellProcess."

---

### D1: C4-plant order gated on enemy/ally ownership; gamemd has no such gate

- Rust now: The player's right-click → PlantC4 path requires the target be a
  non-friendly structure and rejects allies in three places:
  - `src/app_cursor.rs:263` — Demolish cursor requires
    `matches!(hover.kind, HoverTargetKind::EnemyStructure)`.
  - `src/app_context_order.rs:304` — the C4 order branch returns `None` unless
    `matches!(target.kind, HoverTargetKind::EnemyStructure)`.
  - `src/sim/world/world_commands.rs:1009-1016` — the authoritative `PlantC4`
    command handler returns `false` (silently drops the command) when
    `are_houses_friendly(command_owner, target_owner)` is true.
  Because a CABHUT is owned by the Neutral/Special civilian house, whether the
  plant is allowed depends entirely on `are_houses_friendly(player, "Neutral")`
  (`src/map/houses.rs:89`), which is decided by the map's `[Houses]`/alliance
  data — not by any CABHUT-specific rule. When the map's roster treats Neutral
  as not-enemy (or the CABHUT is otherwise classified non-`EnemyStructure`), the
  plant is rejected and nothing happens — the observed bug.
- gamemd: the C4 plant marker is set in `InfantryClass::PerCellProcess`
  (`0x00519630`) Mission `0x11` (Sabotage) block. Decompiled gate, verbatim
  conditions: `mission == 0x11` AND `infantry.Type+0xEC2 != 0` (C4=yes) AND
  `Look_up_building_in_cell() == infantry.Target` AND `building.Mission != 0x13`
  (not Selling) AND `building.vtable[0x160]() == 0` (NOT Iron-Curtained). If
  `building+0x6DF == 0` it sets `building+0x6DF = 1` (the C4 marker). **There is
  no `Is_Ally_ByObject` / owner comparison and no `Type+0x233` (Immune) read on
  this branch.** The cursor that drives this order, `What_Action_OnObject` action
  `0x10`, is likewise gated only on `Type+0xEC2` (C4), target `RTTI==6`
  (building), `vtable[0x80]()==0` (not IC), `Type+0x1577` (CanC4 default true),
  `Type+0x1701==0` — again no ally/owner/Immune check.
- Fixture: SEAL (`GHOST`, `C4=yes`, owner = player house `Americans`) right-clicks
  a CABHUT at cell (50,50) owned by `Neutral`, bridge intact, not IC'd, visible.
  - gamemd: cursor resolves action `0x10` (no owner check) → order = Mission
    Sabotage → SEAL walks onto the hut cell → `PerCellProcess` sets
    `CABHUT+0x6DF=1`, stamps timer fields. After `C4Delay` frames
    `BuildingClass::Update` sees `+0x6DF && Type[0x16B6]` → dispatches bridge
    collapse. Bridge falls. CABHUT survives.
  - Rust: `are_houses_friendly("Americans","Neutral")` → if the map roster does
    NOT list Neutral as an enemy of Americans (common for the civilian house),
    `world_commands.rs:1010` treats it as friendly → `PlantC4` returns `false`.
    `c4_plant` is never set; `tick_c4_plants` never claims;
    `pending_c4_detonation` never set; dispatch never runs. SEAL does nothing
    (or just move-orders). Even when the gate passes, the requirement is a
    fabricated precondition not present in gamemd.
- Player sees: SEAL/Tanya C4 on a bridge repair hut does nothing — the bridge
  never collapses. Triggers every time a player attempts the standard
  "blow the bridge with a commando" tactic on any map whose Neutral house is not
  explicitly enemy-aligned to the player. This is the headline reported bug.
- Severity: HIGH (a stock, well-known YR tactic is fully non-functional; fires on
  every attempt on typical maps).
- Confidence: PROVEN-DRIFT (gamemd plant gate and cursor action decompiled; both
  lack the ally/owner check the Rust port requires).
- Verify-call: `decompile_function 0x00519630` (Mission 0x11 block, marker set at
  `+0x6DF`); `decompile_function 0x0051e3b0` (action 0x10 branch);
  `decompile_function 0x006ffec0` (attack-action resolution, C4-forces-5 branch);
  `get_function_by_address 0x00519630` (confirms `InfantryClass__PerCellProcess`).

### D2: C4-plant requires Chebyshev-adjacency to the building anchor before claim; gamemd claims when the infantry's OWN cell resolves to the target building

- Rust now: `tick_c4_plants` Phase 1 (`src/sim/world/world_orders.rs:443-491`)
  only claims the plant once `target_footprint.contains(&attacker_cell)`; before
  that it issues an "enter target cell" move when `adjacent_to_target_footprint`
  is true (Chebyshev ≤ 1). The claim is keyed off the infantry actually standing
  inside the building footprint cell.
- gamemd: `PerCellProcess` is the per-cell callback that fires when the infantry
  finishes moving INTO a cell; the marker is set when
  `Look_up_building_in_cell() == infantry.Target`, i.e. the infantry's current
  cell hosts the target building. For a `Foundation=1x1` CABHUT this is the same
  single cell, so the net "must be on the hut cell" requirement matches.
- Fixture: SEAL on cell (50,49) adjacent to a 1x1 CABHUT at (50,50). Both
  versions: SEAL must step onto (50,50) before the marker is set; the adjacency
  branch only drives movement, not the claim.
- Player sees: no observable difference for the stock 1x1 CABHUT (the only
  BridgeRepairHut in YR). Listed for completeness because the Rust footprint
  logic would diverge for a hypothetical multi-cell hut (none ship in YR).
- Severity: LOW (no stock multi-cell bridge hut exists; trigger frequency 0 in
  normal YR).
- Confidence: PARITY for stock content; UNCHECKED for non-stock multi-cell huts.
- Verify-call: `decompile_function 0x00519630` (`Look_up_building_in_cell() ==
  param_1[0x169]` test in the Mission 0x11 block).

### D3: "already-claimed" second-attacker handling matches, but the marker is on the BUILDING in gamemd vs a per-attacker `c4_plant` mirror in Rust — re-target window differs

- Rust now: a second SEAL on an already-`pending_c4_detonation` CABHUT hovers/no-ops
  (`world_orders.rs:474-477`). The first attacker keeps its own `c4_plant`
  component; the building holds `pending_c4_detonation`.
- gamemd: the marker is a single byte on the building (`+0x6DF`); the
  Mission-0x11 block early-returns (re-issues movement toward target) when
  `building+0x6DF != 0`, never re-stamping the timer. Single source of truth on
  the building.
- Fixture: two SEALs ordered onto the same CABHUT one tick apart. Both versions:
  only the first stamps the timer; the second is a no-op redirect. Observable
  result identical (one timer, one collapse).
- Player sees: no difference in the common case. A divergence is only reachable
  if the Rust per-attacker `c4_plant` and the building `pending_c4_detonation`
  desynchronize (e.g. attacker retasked mid-timer) — not exercised by the headline
  bug.
- Severity: LOW (behaviorally equivalent for the standard flow).
- Confidence: LIKELY-DRIFT in edge re-target ordering; PARITY for the common path.
- Verify-call: `decompile_function 0x00519630` (`if (building+0x6DF != '\0')`
  redirect-without-restamp).

### D4: hut-death collapse is bounded 4-step (matches), but gamemd's low/high pre-scan reads CellClass tile-index range `[DAT_00ABAD1C, +0x10)` that the Rust port approximates with overlay/`is_wood_bridge_repair_tile`

- Rust now: `choose_hut_bridge_family` (`bridge_orchestrator.rs:408`) picks Low
  vs High by scanning the 5x5 for a low destroy-overlay OR
  `is_wood_bridge_repair_tile`. The decisive low test is overlay-band/wood-tile.
- gamemd: `BuildingClass::Update`'s hut branch (and the second 5x5 inside
  `DestroyBridge_*_OnHutDeath`) selects Low if any scanned cell has tile index in
  `[DAT_00ABAD1C, DAT_00ABAD1C+0x10)` OR low overlay in `(0x49,0x66)`; High
  otherwise (per `InfantryClass::PerCellProcess` 5x5 in the decompile:
  `(DAT_00abad1c <= iVar4 && iVar4 < DAT_00abad1c+0x10) || (0x49 < cell+0x44 &&
  cell+0x44 < 0x66)`).
- Fixture: a CABHUT next to a low wooden bridge with cleared overlays but the
  low-bridge tile-index range present. gamemd: tile-index test fires → Low. Rust:
  relies on `is_wood_bridge_repair_tile`/overlay; if that flag is set on the same
  cells, equivalent — but the two are not proven byte-identical across all low
  tile variants.
- Player sees: on a CABHUT adjacent to a low bridge whose overlays are already
  cleared/damaged, a wrong Low/High choice would route the collapse through the
  wrong walker family (wrong overlay band → possible no-collapse or wrong tiles).
  Rare; only on partially-damaged low bridges.
- Severity: LOW-MED (low-bridge-only, requires pre-damaged overlay state; once D1
  is fixed this becomes the next thing to verify).
- Confidence: UNCHECKED (tile-index range `DAT_00ABAD1C` not enumerated this
  session; the Rust `is_wood_bridge_repair_tile` mapping to that range is unproven).
- Verify-call: `decompile_function 0x00519630` (the `DAT_00abad1c` 5x5 in the
  Mission-0x11 / repair path) — needs follow-up `inspect_memory_content
  0x00ABAD1C` to enumerate the tile-index window.

---

## PARITY-CONFIRMED (checked and matching)

- **Hut survives the C4 (no HP damage).** `apply_c4_damage_to_building`
  (`world_orders.rs:775-794`) branches on `bridge_repair_hut` and returns
  `killed_building: false` with `consumed_pending_marker: true`, never touching
  HP. Matches gamemd `BuildingClass::Update`: the `Type[0x16B6]` branch calls the
  hut-destruction entry and clears `+0x6DF`/`+0x540` WITHOUT calling the building
  damage vtable `+0x16C`. (`CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md` §C4 timer
  expiry; re-read `0x00519630` confirms no damage on hut branch.)
- **Immune=yes is NOT a gate.** Rust never reads an Immune flag on this path
  (no `Immune`/`+0x233` consult in the C4 path). Matches gamemd: Immune
  (`ObjectTypeClass+0x233`) is read at `0x005F9510` for parse but is not on the
  plant or collapse branch; the hut survives because the branch skips damage, not
  because of Immune. (`TECH_CABHUT` §4.6, §5.2.)
- **Iron Curtain DOES gate the plant.** Both `app_cursor.rs:265`,
  `app_context_order.rs:315`, and `world_commands.rs:998` reject IC'd targets at
  issue time. Matches gamemd `vtable[0x160]()==0` (IsIronCurtainActive) in the
  Mission-0x11 block and `vtable[0x80]()==0` in the cursor resolver.
- **Bounded 4-step collapse (not full-span flood).** `run_hut_collapse_bounded`
  (`bridge_orchestrator.rs:726`) implements extent-measure → signed bias start →
  `MAX_HUT_SWEEP_STEPS=4` axial steps → `MAX_HUT_ATTEMPTS_PER_STEP=3` retries →
  break on leaving the overlay band. Matches the four `CollapseBridge_*_*`
  walkers (`local_2c = 4`, ≤3 `DestroyBridge_*` retries, break on band exit) per
  `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md` §3 (verified slice).
- **Overlay bands.** Rust `in_bridge_band` uses high `[0xCD..=0xE8]`, low
  `[0x4A..=0x65]` (`bridge_orchestrator.rs:874-879`); terminal caps `0xE7/0xE8`
  high, `0x64/0x65` low (`hut_walker_terminal_cap`). Matches the documented bands
  and terminal caps.
- **5x5 hut scan is X-major, -2..=2 inclusive (25 cells).** `hut_destroy_5x5_scan`
  (`bridge_orchestrator.rs:222`) flat-maps `dx=-2..=2` then `dy=-2..=2`. Matches
  gamemd `for(iVar3=-2; iVar3<3) for(sStack_40=-2; ...<3)` (25 cells, `<3` = include
  +2, no off-by-one).
- **C4Delay = 27 ticks (0.03 min @ 15fps).** `ruleset.rs:1354-1357,1519-1524`
  parses `[CombatDamage] C4Delay` minutes → ticks, default 27. Matches the
  documented retail default; `tick_c4_plants` Phase 2 fires at
  `elapsed >= c4_delay_ticks` (`world_orders.rs:549`), matching the binary's
  `>= delay` (not `>`).
- **GHOST/TANY are C4-capable.** `ini/rulesmd.ini:4027` (`[GHOST] C4=yes`),
  `4078` (`[TANY] C4=yes`); parsed at `object_type.rs:1127` (`c4`). CABHUT has
  `BridgeRepairHut=yes` (`16348`) parsed at `object_type.rs:1073`; `CanC4` defaults
  true for buildings (`object_type.rs:1128-1130`), CABHUT does not opt out.
- **Marker clears after hut dispatch.** `world_orders.rs:588-598` clears
  `pending_c4_detonation` and the attacker's `c4_plant` when
  `consumed_pending_marker` is true. Matches gamemd clearing `+0x6DF`/`+0x540`
  after the hut branch.

## UNCHECKED

- **The exact map-condition that makes the enemy-only gate reject a given
  CABHUT.** D1 proves the Rust gate is a fabricated precondition vs gamemd, but I
  could not run the game to confirm which maps' `[Houses]` alliance state makes
  `are_houses_friendly(player,"Neutral")` true vs false. The bug fires whenever
  Neutral is not enemy-classified for the player; enumerating that requires the
  map roster at runtime. (Static read of `map/houses.rs:89` shows the gate; the
  actual alliance contents are map data.)
- **D4 tile-index window `DAT_00ABAD1C`.** Not enumerated; needs
  `inspect_memory_content 0x00ABAD1C` to confirm the low-bridge tile-index range
  and whether Rust `is_wood_bridge_repair_tile` covers exactly that set.
- **Whether a Neutral CABHUT is reliably `is_visible` and hit-tested at the hut
  cell.** `hover_target_at_point` (`app_entity_pick.rs:96`) requires the cell
  revealed and the click to hit the foundation; not exercised end-to-end here.
- **Demo-truck (DTRUCK) C4-equivalent path to the same hut dispatch.** Out of
  this facet's order scope; `apply_c4_damage_to_building` is noted in-code as the
  intended future demo-truck entry but the demo-truck order path was not traced.
