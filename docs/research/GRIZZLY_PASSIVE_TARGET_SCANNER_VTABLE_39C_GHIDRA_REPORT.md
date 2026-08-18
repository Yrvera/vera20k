# Grizzly Passive Target Scanner vtable+0x39C - Ghidra Research Report

**Address(es):** `vtable__UnitClass+0x39C @ 0x007F600C -> 0x00709820`, `UnitClass+0x3C4 @ 0x007F6034 -> 0x00743190`, `TechnoClass__Greatest_Threat @ 0x006F8DF0`, `TechnoClass__Scan_Cell_For_Target @ 0x006F8960`, `TechnoClass__Evaluate_Candidate @ 0x006F7CA0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** passive target scanner reached after `OpportunityFire=yes` opens the common gate for stock Grizzly/`[MTNK]`.  
**Non-Scope:** weapon damage, projectile flight, FLH, burst cadence, and full non-Grizzly target scoring policy.  
**Confidence:** High for concrete virtual resolution, scanner call chain, range/hostility/visibility gates, and minimal ranking behavior.
**Active in YR:** Yes.

## Target Question

After `OpportunityFire=yes` lets a moving stock Grizzly run passive acquisition, what concrete scanner virtual is called, what target legality checks does it apply, and what minimal ranking behavior must Rust reproduce for move-by target selection?

## Non-goals

- Do not investigate direct firing timing after acquisition.
- Do not investigate projectile, damage, burst, warhead, or FLH origin.
- Do not generalize all `GreatestThreat` flags for non-Grizzly special weapons.
- Do not write Rust code.

## Evidence Needed to Mark COMPLETE

- Resolve the UnitClass `vtable+0x39C` entry used by `TechnoClass__AI_Update` and `TechnoClass__Passive_Target_Acquire`.
- Show the concrete scanner chain behind that virtual.
- Verify stock MTNK input data for weapon/range and `OpportunityFire`.
- Identify range, hostility, visibility/cloak, and minimal target ranking behavior needed for implementation.
- Check current Rust acquisition shape enough to name handoff surfaces.

## Stop Conditions

- Stop once the stock MTNK passive scan path is resolved to concrete functions and filters.
- Stop before weapon damage/projectile/FLH internals.
- Stop before exhaustively naming every `GreatestThreat` flag unrelated to ordinary Grizzly move-by targeting.

## 1. Overview

The `vtable+0x39C` scanner reached after the `OpportunityFire` gate is concrete for stock Grizzly/UnitClass: `vtable__UnitClass+0x39C @ 0x007F600C` contains `0x00709820`, decompiled as `TechnoClass__Retaliate_And_Scan`. The passive callers pass current coordinates from `vtable+0x48`; `0x00709820` schedules the next passive scan timer, clears invalid stale targets if `+0x50C` says the target changed, and if no current `TarCom` remains calls UnitClass `vtable+0x3C4`.

For UnitClass, `vtable+0x3C4 @ 0x007F6034` is `0x00743190`. That wrapper gathers candidate target-mask bits from current weapons (`vtable+0x3F8` plus `FUN_00772A90`) and then calls `FUN_004D9920`, which calls `TechnoClass__Greatest_Threat @ 0x006F8DF0`. `Greatest_Threat` performs the nearby-cell scan and uses `TechnoClass__Evaluate_Candidate @ 0x006F7CA0` for legality plus scoring. If a non-null candidate returns, `0x00709820` sets `TarCom` through `vtable+0x3C8` (`TechnoClass__Set_ArchiveTarget @ 0x006FCDB0`) and returns whether `TarCom` is now non-null.

For stock MTNK, the relevant INI values are `[MTNK] Primary=105mm`, `OpportunityFire=yes`, `ThreatPosed=15`, and `[105mm] Range=5`. The scanner is generic UnitClass/TechnoClass behavior; no Grizzly hardcoded branch was found.

## 2. Concrete Call Chain

| Stage | Evidence | Behavior | Active in YR |
|---|---|---|---|
| Passive caller | `TechnoClass__AI_Update @ 0x006FA6B7..0x006FA6EE`, `TechnoClass__Passive_Target_Acquire @ 0x00709492..0x007094C8` | Writes `g_CurrentFrameCounter` to `+0x4FC`, gets current coords via `vtable+0x48`, calls `vtable+0x39C`, sets `+0x50C` if `TarCom` changed | Yes |
| UnitClass virtual | PE/vtable read: `0x007F600C -> 0x00709820`; xrefs from UnitClass vtable data | Concrete `vtable+0x39C` target is `0x00709820` | Yes |
| Retaliate/scan wrapper | `0x00709820..0x007099CA`; calls `vtable+0x3C4` at `0x00709932`, then `vtable+0x3C8` at `0x00709960` | Schedules next scan, clears bad stale target, asks scanner for best object, sets `TarCom` | Yes |
| UnitClass scanner wrapper | PE/vtable read: `0x007F6034 -> 0x00743190`; decompile `0x00743190..0x00743263` | Adds weapon-derived target mask bits, then calls `FUN_004D9920` | Yes |
| Greatest threat | `FUN_004D9920 @ 0x004D9920..0x004D995C` calls `TechnoClass__Greatest_Threat @ 0x006F8DF0` | Runs target search and returns the best candidate pointer | Yes |

## 3. Range and Scan Area

The passive stock-Grizzly path enters the `Greatest_Threat` nearby-cell branch, not the all-techno global branch, because the call carries the ordinary passive scan selector. In that branch:

- `Greatest_Threat @ 0x006F8FC8..0x006F9016` asks UnitClass `vtable+0x31C` for an override range. UnitClass `vtable+0x31C @ 0x007F5F8C -> 0x00707E60`.
- For no override, `0x006F9064..0x006F913D` falls back to weapon range via `vtable+0x168`. UnitClass `vtable+0x168 @ 0x007F5DD8 -> 0x007012C0`, which reads the selected weapon's range from `weapon+0xB4`.
- The cell loop uses a radius in cells derived from the lepton range (`range >> 8`) plus one and a type radius term, but `Evaluate_Candidate` still performs exact coordinate distance gating before accepting a target.
- For stock `[105mm] Range=5`, the acceptance range is 5 cells in RA2 fixed-point/leptons, not `GuardRange`. `[MTNK]` does not define `GuardRange`.

## 4. Candidate Filters

For normal stock Grizzly targeting a normal enemy ground unit/building, a candidate must pass these verified filters:

- **Object is in the scanned cell contents.** `TechnoClass__Scan_Cell_For_Target @ 0x006F8960` reads `CellClass+0xE8`, falling back to `+0xE4`, then walks object links through `object+0x30`.
- **Not self and live/selectable enough for combat.** `Evaluate_Candidate @ 0x006F7CA0` rejects `param_1 == this`, zero health, dying/limbo-like state bytes, and target types whose type flag at `type+0x231` is false.
- **Hostility/ally rules.** `Scan_Cell_For_Target` and `Evaluate_Candidate` both call `HouseClass__Is_Ally_ByObject`; normal allies are rejected for weapon-bearing MTNK unless special repair/capture/mind-control style branches apply. Those special branches are not stock Grizzly move-by attack behavior.
- **Weapon compatibility and range.** `Evaluate_Candidate` calls attacker `vtable+0x2E4`/`SelectWeaponAgainst` and `vtable+0x3F8` to obtain the usable weapon, rejects impossible weapon/armor cases, computes 3D coordinate distance, and rejects when `weapon_range < distance`.
- **Cloak/sensor visibility.** If the target `CloakState == 2`, `Evaluate_Candidate` checks the target cell's `CellClass__SensorCountForHouse`; without sensors, an enemy cloaked target is rejected.
- **Human-player visibility gate.** `Evaluate_Candidate` contains a human-player/game-mode visibility check using target visibility bytes around `target+0x41A/+0x41B`; non-aircraft targets that are not visible to the human player in the relevant mode are rejected.
- **Bridge-layer consistency.** If source and target cells are both bridge-marked, `Evaluate_Candidate` rejects when attacker `OnBridge` differs from target `OnBridge`.

## 5. Minimal Ranking Behavior

Rust should not model this passive Grizzly scan as "nearest target wins." The binary keeps a best `(candidate, score)` pair where score is written by `Evaluate_Candidate` and derived from `TechnoClass__Calculate_Threat_Score @ 0x0070CD10`, then modified by health/state, owner-threat settings, special flags, and `TechnoClass__ThreatAvoidance_Modifier`.

The crucial local behavior:

- `Greatest_Threat` initializes best score to `-1`.
- Each accepted cell candidate returns a score through the out-param; `Greatest_Threat` updates only when `new_score > best_score`.
- Equal scores do not replace the prior candidate, so scan order is the tie-break.
- The nearby-cell scan walks rings around the Grizzly's current cell and can early-return once a candidate exists at approximately quarter/half of the scan radius (`0x006F9B4D..0x006F9B68` in decompile structure), so high-scoring nearby candidates may stop later distant scans.
- Distance affects the score/range gate, but the rank key is not `(distance, class, stable_id)`.

## 6. Current Rust Delta

Current Rust `src/sim/combat/combat_targeting.rs::acquire_best_target` filters alive/hostile/visible/weapon-compatible/in-range targets, but ranks by `(dist_sq, threat_class, stable_id)`, so nearest target wins before threat value. Current `src/sim/world/world_orders.rs::tick_order_intents_pre_combat` only runs acquisition for entities with `OrderIntent` such as `AttackMove`/`Guard`; the prior report already established stock MTNK `OpportunityFire=yes` should permit ordinary move passive acquisition.

The implementation should add the `OpportunityFire` ordinary-move scan and should either reuse or extend `acquire_best_target` with a YR-style threat-score ranking mode. A minimal Grizzly parity pass should at least include `ThreatPosed`/threat score over nearest-distance ordering for two legal move-by enemies.

## Negative Facts / Do Not Do

- Do not treat `OpportunityFire` as direct firing or damage.
- Do not make passive move-by acquisition require `AttackMove`.
- Do not rank passive Grizzly targets by nearest distance alone.
- Do not use `GuardRange` for stock MTNK passive scan; `[MTNK]` has no `GuardRange`, and the path falls back to weapon range.
- Do not acquire cloaked enemies without sensor visibility.
- Do not acquire human-player-invisible normal ground targets just because they are geometrically in range.
- Do not replace equal-score scan-order tie behavior with stable-id sorting if exact parity is needed.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|
| UnitClass passive `vtable+0x39C` sets `TarCom` from `Greatest_Threat`, not from nearest-distance search | `0x007F600C -> 0x00709820`; `0x00709932` calls `+0x3C4`; `0x004D9942` calls `Greatest_Threat`; `Greatest_Threat` updates only on greater score | Rust ranks by `(dist_sq, threat_class, stable_id)` | `src/sim/combat/combat_targeting.rs` | Moving MTNK passes two legal enemies: farther higher-threat enemy is chosen over nearer low-threat enemy | `grizzly_passive_scan_prefers_higher_threat_over_nearest` | nearest-first behavior will pick visibly wrong targets |
| Stock MTNK passive scan falls back to primary weapon range | `[MTNK] Primary=105mm`, `[105mm] Range=5`; `0x00707E60`/`0x007012C0`; no `[MTNK] GuardRange` | Rust uses `guard_range.unwrap_or(weapon.range)`, but ordinary move scan not wired | `src/rules/object_type.rs`, `src/sim/world/world_orders.rs`, `src/sim/combat/combat_targeting.rs` | Ordinary moving Grizzly acquires an enemy at 5 cells, does not acquire at beyond-5-cell weapon range | `grizzly_opportunity_fire_passive_scan_uses_105mm_range` | accidentally using sight/guard range changes move-by pickups |
| Passive scanner rejects invisible/cloaked/out-of-layer candidates | `Evaluate_Candidate @ 0x006F7CA0` cloak sensor branch, human-player visibility branch, bridge `OnBridge` branch | Rust has fog visibility but no sensor/bridge parity in this helper | `src/sim/combat/combat_targeting.rs`, fog/sensor/terrain bridge surfaces | Moving Grizzly ignores cloaked unsensed enemy and ignores target on mismatched bridge layer | `grizzly_passive_scan_rejects_unsensed_cloaked_target` / `grizzly_passive_scan_respects_bridge_layer` | over-acquisition exposes hidden or unreachable targets |

## Open Questions - Final State

- `[RESOLVED] OQ-1 - What concrete virtual is UnitClass vtable+0x39C? -> 0x00709820.` Evidence: PE/vtable read `0x007F600C -> 0x00709820`; data xref to UnitClass vtable.
- `[RESOLVED] OQ-2 - What scanner is reached? -> UnitClass +0x3C4 wrapper 0x00743190 -> FUN_004D9920 -> TechnoClass__Greatest_Threat 0x006F8DF0.`
- `[RESOLVED] OQ-3 - Does stock MTNK use 105mm range? -> Yes, `[MTNK] Primary=105mm`, `[105mm] Range=5`, no `[MTNK] GuardRange`, and scanner falls back to selected weapon range.`
- `[RESOLVED] OQ-4 - Is ranking nearest-first? -> No, accepted candidates compete by calculated threat score; equal scores keep prior scan-order candidate.`
- `[DEFERRED] OQ-5 - Exact numeric threat-score formula for every object class?` Out of scope; only minimal Grizzly move-by ranking was required.

## Stale-Doc Replacement Wording

`OpportunityFire=yes` for stock Grizzly opens the passive-acquire gate, then UnitClass `vtable+0x39C` resolves to `0x00709820`, which calls UnitClass scanner `0x00743190` and ultimately `TechnoClass__Greatest_Threat @ 0x006F8DF0`. For `[MTNK] Primary=105mm`, the ordinary move-by scan uses the selected weapon range (`Range=5`) rather than `GuardRange`, filters out illegal allies, dead/limbo targets, unsensed cloaked targets, invisible human-player targets, bridge-layer mismatches, and weapon-incompatible or out-of-range objects. Selection is threat-score based (`TechnoClass__Calculate_Threat_Score` via `Evaluate_Candidate`), not nearest-distance first; equal scores preserve scan order.

## Sources

- Ghidra decompiled/read: `0x006F9E50`, `0x00709480`, `0x00709820`, `0x00743190`, `0x004D9920`, `0x006F8DF0`, `0x006F8960`, `0x006F7CA0`, `0x0070CD10`, `0x00707E60`, `0x007012C0`, `0x006FCDB0`, `0x00746CD0`.
- Assembly/context evidence: `0x006FA6B7..0x006FA6EE`, `0x00709492..0x007094C8`, `0x00709820..0x007099CA`, `0x00743190..0x00743263`, `0x004D9920..0x004D995C`.
- Vtable reads from retail `gamemd.exe`: `0x007F600C -> 0x00709820`, `0x007F6034 -> 0x00743190`, `0x007F5F8C -> 0x00707E60`, `0x007F5DD8 -> 0x007012C0`, `0x007F6038 -> 0x006FCDB0`.
- INI checked: `ini/rulesmd.ini` `[MTNK]`; `ini/rules.ini` `[105mm]`.
- Rust scanned: `src/sim/combat/combat_targeting.rs`, `src/sim/world/world_orders.rs`, `src/sim/components.rs`.

## Status

COMPLETE.
