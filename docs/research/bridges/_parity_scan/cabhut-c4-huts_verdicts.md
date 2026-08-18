# CABHUT C4 trigger + repair-hut death path — Adversarial Verdicts

**Facet:** cabhut-c4-huts
**Date:** 2026-05-29
**Auditor stance:** adversarial skeptic — every claimed disparity REFUTED until the live binary + current Rust prove it real.
**Authority:** live Ghidra (gamemd.exe) > labels > docs. Each gamemd claim re-decompiled this session.

---

## D1 — VERDICT=REFUTED (as stated); the underlying bug is REAL but mis-attributed

**Finder's gamemd reading: CONFIRMED.** Re-decompiled `InfantryClass__PerCellProcess`
@ `0x00519630` (`get_function_by_address 0x00519630` confirms the name). The Mission
0x11 (Sabotage) block:
`if ((iVar4 == 0x11) && (Type+0xEC2 != 0)) { piVar10 = Look_up_building_in_cell();
if (piVar10 == param_1[0x169]) { ... if ((building.Mission != 0x13) &&
(building.vtable[0x160]() == 0)) { if (building+0x6DF != 0) redirect-return; else
*(building+0x6DF) = 1; ... } } }`. There is NO `Is_Ally_ByObject`, NO owner compare,
NO Immune/`+0x233` read on the marker-set branch — exactly as the finder claimed. The
cursor side at `0x0051e3b0` (`InfantryClass__What_Action_OnObject`) likewise returns
action `0x10` gated only on `IsHumanPlayer && (Type+0xEC2 C4 || HasWeaponAbility 0xE)
&& action==5 && RTTI==6 && vtable[0x80]==0 (not IC) && Type+0x1577 (CanC4) &&
Type+0x1701==0` — no ally/owner/Immune. Both gamemd readings hold up live.

**Finder's Rust ROOT-CAUSE claim: REFUTED.** The finder asserts the bug fires because
`are_houses_friendly(player,"Neutral")` returns **true** "for the civilian house," so the
`world_commands.rs:1010` ally gate drops the plant. That is the OPPOSITE of what the code
does. `are_houses_friendly` (`src/map/houses.rs:89-101`) returns true ONLY for
case-insensitive name-equality OR an explicit entry in the alliance map. The alliance map
is built from `[Houses]` allies (`houses.rs:74-83`) and, on the skirmish path, only
team-shared slots (`app_skirmish.rs:359-375` `launch_alliance_map`) — Neutral is NEVER
auto-allied with the player. Therefore `are_houses_friendly(player,"Neutral") == false`,
the `world_commands.rs:1010` gate does NOT reject, and the hover classifier
(`app_entity_pick.rs:136-158` via `fog.is_friendly` → `are_houses_friendly`,
`vision/mod.rs:276`) tags a Neutral CABHUT as `HoverTargetKind::EnemyStructure`. So both
the cursor gate (`app_cursor.rs:262-273`) and the order gate
(`app_context_order.rs:302-322`) PASS for a Neutral enemy CABHUT, and `PlantC4` is
accepted. The finder's named gate is not the blocker.

Net: the finder's gamemd half is correct, but its Rust causal mechanism is provably wrong
(`are_houses_friendly` semantics + skirmish alliance build refute "Neutral reads as
friendly"). The headline bug (SEAL/Tanya C4 on CABHUT does nothing — known, see
project memory `project_c4_bridge_hut_followup.md`) is REAL but its root cause is NOT the
ally gate D1 names. Marking D1 REFUTED-as-stated; the true upstream blocker is unidentified
(see MISS below).

## D2 — VERDICT=REFUTED (Rust already output-identical for stock content)

gamemd `0x00519630` Mission-0x11 sets the marker when
`Look_up_building_in_cell() == infantry.Target` (current cell hosts the target building).
Rust `tick_c4_plants` (`world_orders.rs:479`) claims when
`target_footprint.contains(&attacker_cell)`, footprint = `c4_base_foundation_cells`
(`world_orders.rs:618`). For the stock `Foundation=1x1` CABHUT — the only YR
BridgeRepairHut — both reduce to the single hut cell; the adjacency branch
(`adjacent_to_target_footprint`, `world_orders.rs:625`, Chebyshev<=1) only drives the
enter-move, not the claim. No observable difference for any shipping content. (Divergence
would require a hypothetical multi-cell hut that does not exist in YR.)

## D3 — VERDICT=REFUTED (common-path output-identical; edge unproven but unreached)

gamemd marker is a single byte on the building (`building+0x6DF`); the Mission-0x11 block
early-returns with a movement redirect WITHOUT restamping when `building+0x6DF != 0`
(verified in the `0x00519630` decompile: `if (*(char *)(piVar10+0x6df) != '\0') {...
return;}`). Rust mirrors this with the building's `pending_c4_detonation` plus a per-
attacker `c4_plant`, and the `already_claimed` guard (`world_orders.rs:470-477`) early-
continues a second attacker. Observable common-path result is identical (one timer, one
collapse). The finder's "LIKELY-DRIFT in edge re-target ordering" is speculative and not
exercised by the headline flow; no concrete scenario produces different output, so by the
burden of proof it is not a REAL disparity. (If a future scenario desyncs the per-attacker
mirror from the building marker, re-open — but unproven today.)

## D4 — VERDICT=UNCERTAIN (gamemd tile-index window not enumerable; equivalence unproven)

gamemd low/high select confirmed live in `0x00519630`:
`((DAT_00abad1c <= iVar4) && (iVar4 < DAT_00abad1c + 0x10)) ||
((0x49 < cell+0x44) && (cell+0x44 < 0x66))` where `iVar4 = *(int*)(CellClass+0x38)`
(tile-index field) and `cell+0x44` is the overlay field. Rust `choose_hut_bridge_family`
→ `is_low_hut_scan_evidence` (`bridge_orchestrator.rs:408-426`) tests
`is_low_destroy_overlay` OR `is_wood_bridge_repair_tile`. The overlay half matches: Rust
`in_bridge_band` Low = `0x4A..=0x65` (`bridge_orchestrator.rs:877`) == gamemd
`0x49 < x < 0x66`. But the tile-index window `DAT_00ABAD1C` is a runtime-initialized
global — `read_memory 0x00ABAD1C` returns `00000000` statically (populated at map/theater
load), so the actual window cannot be enumerated from a static read this session, and
whether Rust `is_wood_bridge_repair_tile` covers exactly `[base, base+0x10)` is unproven.
Cannot independently confirm equivalence -> UNCERTAIN (not REAL: the gamemd value is
unknown, so a divergence is unproven; not REFUTED: equivalence is also unproven). Needs a
live-debugger read of `DAT_00ABAD1C` after map load to resolve. Finder's UNCHECKED label
is appropriate; promoting to UNCERTAIN since the comparison axis is identified but unverifiable.

---

## MISS (new disparity the finder did not surface)

**MISS-1 [the actual D1 root cause is unidentified].** The finder proved gamemd has no
ally/Immune gate and assumed the Rust ally gate is the blocker — but that gate does NOT
reject a Neutral CABHUT (D1 above). For the known bug to still fire, the real upstream
blocker must be one of: (a) the runtime CABHUT owner being the local player or a
player-allied house on the maps in question (then hover = `FriendlyStructure`, all three
C4 gates reject) — owner comes straight from the map `[Structures]` line
(`map/entities.rs:188,245`) and is not special-cased; (b) the foundation hit-test for a
Neutral 1x1 CABHUT failing in `hover_target_at_point` (`app_entity_pick.rs:96-145`,
`click_hits_foundation`); or (c) `is_cell_revealed`/`is_cell_gap_covered`
(`app_entity_pick.rs:137-145`) downgrading the hover to `HiddenEnemy` (also rejected by
the C4 gate). None of these is the ally gate D1 names. The integration test
(`world_orders_bridge_repair_tests.rs:793`) sidesteps the entire issue by setting
`c4_plant` directly AND spawning the CABHUT as `"Soviets"` (an enemy house), so it never
exercises the Neutral-ownership or hit-test path — the bug is invisible to the suite. This
is the unresolved root cause and should be the next investigation target (consistent with
project memory `project_c4_bridge_hut_followup.md`: "the bug is port-side," prior
Immune-gate hypothesis already refuted, now the ally-gate hypothesis refuted too).

**MISS-2 [Mission-state gate not modeled — acknowledged in code, not in finding].**
gamemd's plant gate also requires `building.Mission != 0x13` (not Selling); Rust
`world_commands.rs:984-986` has a `TODO(parity)` that it does NOT reject selling-in-
progress buildings because building Mission state is unmodeled. A SEAL could plant C4 on a
CABHUT that gamemd would reject because it is mid-sell. Low frequency (CABHUTs are not
sold by the player), but it is a real, unflagged gate difference. The finder's
PARITY-CONFIRMED list omits it.
