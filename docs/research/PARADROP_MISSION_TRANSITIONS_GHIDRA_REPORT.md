# Paradrop Mission Transitions - Ghidra Research Report

**Address(es):** `0x006CC390` (SuperClass::Launch), `0x0065E660` (paradrop spawner), `0x004158E0` (Mission_Open), `0x00415960` (Mission_Rescue), `0x004155F0` (Mission_ParaDropApproach), `0x004157C0` (Mission_ParaDropOverfly), `0x0041BEE0` (checked negative), `0x00414BB0` (AircraftClass::AI off-map removal)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard YR superweapon paradrop carrier mission transitions and the adjacent ParaDropApproach/ParaDropOverfly handlers named in prior docs.
**Non-Scope:** low-level Drop_Payload offset math, passenger construction details, bridge target replacement internals, parachuted infantry descent/render, full object cleanup/crash internals.
**Confidence:** High for launch mission ID, Open/Rescue transition loop, Approach/Overfly handler identities, `ParadropRadius` and `0x300` thresholds, and `0x0041BEE0` negative finding. Medium for final cleanup naming because this report stops at the generic AircraftClass::AI removal call.
**Active in YR:** Yes for the standard superweapon path through `Mission_Open`/`Mission_Rescue`. `Mission_ParaDropApproach`/`Mission_ParaDropOverfly` are live AircraftClass handlers but are not reached by standard stock ParaDrop/AmerParaDrop launch.

## 0. Working Notes / Stop Gate

Target question: Verify the standard YR paradrop carrier mission state machine, including mission IDs/handlers, `Rules.ParadropRadius`, hardcoded `0x301`/`0x300`, target/exit destination updates, and carrier removal.

Non-goals: Do not redo Drop_Payload math, spawner cargo construction, bridge target validation, or parachute descent except where needed to prove transition conditions.

Evidence needed to mark COMPLETE: launch-to-spawner assembly showing initial mission, dispatch-table xrefs for handlers, decompile plus assembly context for threshold comparisons and mission/destination writes, INI/default evidence for active stock YR, and Rust scan for deltas.

Stop conditions: Stop when standard SW launch, Open, Rescue, ParaDropApproach, ParaDropOverfly, possible `0x0041BEE0`, edge helper calls, distance helper, and out-of-playfield removal are each resolved or explicitly deferred.

## 1. Overview

The standard stock YR superweapon paradrop carrier does **not** start on `Mission_ParaDropApproach` (`0x1E`). `SuperClass::Launch` pushes mission `0x1A` into the paradrop spawner, and the spawner queues that mission on the created PDPLANE. That makes the standard SW chain:

```text
SuperClass::Launch case 5/6
  -> FUN_0065E660 spawner queues mission 0x1A
  -> Mission_Open (0x1A, 0x004158E0)
  -> Mission_Rescue (0x1B, 0x00415960)
  -> repeat Open/Rescue while payload/cooldown remains
  -> clear destination/target and queue mission 4 when payload path is done
  -> generic AircraftClass::AI removes the carrier when it flies out of playfield
```

`Mission_ParaDropApproach` (`0x1E`, `0x004155F0`) and `Mission_ParaDropOverfly` (`0x1F`, `0x004157C0`) are real AircraftClass handlers, but this pass found no standard ParaDrop/AmerParaDrop launch path that starts the carrier there. They should not be used as the stock SW state machine without a separate caller proving that path.

## 2. Key Offsets And Fields

| Field / table | Meaning | Evidence | Active in YR |
|---|---|---|---|
| Launch call arg `0x1A` | Initial mission passed to `FUN_0065E660` for ParaDrop/AmerParaDrop | assembly contexts `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655` all push `0x1A` before the spawner call | Yes, standard stock SW |
| Spawner vtable `+0x1E8` call | Queues the passed mission on the spawned aircraft | `FUN_0065E660` decompile; assembly `0x0065E6FF..0x0065E70C` pushes mission arg then calls `[EDX+0x1E8]` | Yes |
| AircraftClass vtable slot xref `0x007E2504` | `Mission_Open @ 0x004158E0` | vtable/data xref reached by `MissionClass::Mission_Dispatch` | Yes |
| AircraftClass vtable slot xref `0x007E2508` | `Mission_Rescue @ 0x00415960` | vtable/data xref reached by `MissionClass::Mission_Dispatch` | Yes |
| AircraftClass vtable slot xref `0x007E2510` | `Mission_ParaDropApproach @ 0x004155F0` | vtable/data xref reached by `MissionClass::Mission_Dispatch` | Conditional - live handler, not standard SW launch |
| AircraftClass vtable slot xref `0x007E2514` | `Mission_ParaDropOverfly @ 0x004157C0` | vtable/data xref reached by `MissionClass::Mission_Dispatch` | Conditional - live handler, not standard SW launch |
| `Aircraft+0x2B4` | Target pointer used by distance helper | calls to `FUN_005F6440` from Open/Rescue/Approach/Overfly | Yes |
| `Aircraft+0x5A4` | Payload/current target gate checked by Open/Overfly | Open decompile; Overfly assembly `0x00415859` | Yes |
| `Aircraft+0x118` | Payload/drop gate checked by Rescue and Fire_At | Rescue decompile; Fire_At decompile | Yes |
| `Aircraft+0x6D2` | Strafe/flyby byte | Rescue sets/clears it; Approach sets it before `0x1F` | Yes |
| `Aircraft+0x6D3` | LandingState/drop spacing byte | Open decrements it; Rescue reads it; Drop_Payload write is slot-1 scope | Yes |
| `Rules+0x54C` | `[General] ParadropRadius` | `RULESCLASS_FIELDS.csv`; Open/Rescue assembly reads | Yes, standard stock SW |
| `0x300` compare | ParaDropApproach -> ParaDropOverfly hardcoded threshold | Approach assembly `0x00415700` | Conditional - only if mission `0x1E` path is assigned |

## 3. Core Logic

### 3.1 Standard SW launch starts at mission `0x1A`

Material finding: `SuperClass::Launch` cases 5 and 6 pass mission `0x1A` to the paradrop spawner, not `0x1E`.

Evidence:
- ParaDrop side branches call `FUN_0065E660` at `0x006CD421`, `0x006CD493`, and `0x006CD4EB`; each assembly context pushes `0x1A` immediately before the call.
- AmerParaDrop calls `FUN_0065E660` at `0x006CD655`; its assembly context also pushes `0x1A`.
- `FUN_0065E660` queues the mission argument through vtable `+0x1E8` at `0x0065E6FF..0x0065E70C`.

Active in YR: Yes. Stock `rulesmd.ini` defines `[ParaDropSpecial] Type=ParaDrop` and `[AmericanParaDropSpecial] Type=AmerParaDrop`; both use this launch switch.

### 3.2 `Mission_Open` (`0x1A`, `0x004158E0`)

Material findings:

1. If target is null, Open sets destination/target fallback and queues mission `4`.
   Evidence: decompile `0x004158E0`.
   Active in YR: Yes, target-expired edge path.

2. If `Aircraft+0x5A4 == 0`, Open sets destination toward the current target and returns `3`; it does not queue Rescue in that branch.
   Evidence: decompile `0x004158E0`.
   Active in YR: Yes, payload/current-target edge path.

3. If payload/current target exists, Open computes distance to `Aircraft+0x2B4` and compares it against `Rules+0x54C` (`ParadropRadius`).
   Evidence: assembly context `0x0041592F..0x00415940`.
   Active in YR: Yes, standard stock SW. Stock value is `[General] ParadropRadius=1024`.

4. When distance is within `ParadropRadius`, Open queues mission `0x1B` and decrements `Aircraft+0x6D3`.
   Evidence: assembly context `0x00415942..0x00415956`; decompile `0x004158E0`.
   Active in YR: Yes.

### 3.3 `Mission_Rescue` (`0x1B`, `0x00415960`)

Material findings:

1. Rescue sets `Aircraft+0x6D2 = 1` at entry, then clears it if target/payload prerequisites fail.
   Evidence: decompile `0x00415960`.
   Active in YR: Yes.

2. Rescue requires both target (`+0x2B4`) and payload/drop gate (`+0x118`) before attempting the in-range drop path.
   Evidence: decompile branch checks target and `param_1[0x46]` (`0x46 * 4 = +0x118`).
   Active in YR: Yes.

3. Rescue compares distance against `Rules+0x54C` (`ParadropRadius`). If in range and current coords are in playfield, it calls `Drop_Payload` and returns `5`.
   Evidence: assembly context `0x0041598C..0x004159C8`; decompile `0x00415960`.
   Active in YR: Yes.

4. If out of range, Rescue clears `+0x6D2`; if `+0x6D3 > 0`, it queues mission `0x1A` and returns `5`. Otherwise it clears archive target/destination and queues mission `4`.
   Evidence: decompile `0x00415960`; assembly around `0x0041599F..0x00415A0A`.
   Active in YR: Yes.

### 3.4 ParaDropApproach / ParaDropOverfly are live sibling handlers, not standard SW launch

`Mission_ParaDropApproach @ 0x004155F0`:
- Uses weapon range (`weapon+0xB4` from vtable `+0x3F8`) for reveal/fire handling.
- Transitions to mission `0x1F` only when distance is `<= 0x300` (`distance < 0x301` in decompile).
- On transition, it queues `0x1F`, writes `+0x6D2 = 1`, computes opposite edge through `HouseClass::GetOppositeEdge`, calls edge finder `FUN_004AA440`, and sets destination if the result is not invalid.
- Evidence: assembly contexts `0x0041565A`, `0x00415700`, `0x0041570C..0x0041577F`.
- Active in YR: Conditional. The handler is in the AircraftClass dispatch table, but this report did not find it on the standard stock ParaDrop/AmerParaDrop launch path.

`Mission_ParaDropOverfly @ 0x004157C0`:
- Uses weapon range for reveal/fire handling.
- If `Aircraft+0x5A4 == 0`, computes opposite-edge destination and sets it if valid.
- Returns fixed delay `3`.
- Evidence: assembly contexts `0x004157E3`, `0x00415859..0x004158C1`.
- Active in YR: Conditional. Live handler, not standard stock SW launch from cases 5/6.

### 3.5 `0x0041BEE0` is not mission 31

Material finding: the suspected "mission 31/post-drop exit" at `0x0041BEE0` is not the standard paradrop overfly handler.

Evidence:
- `0x0041BEE0` has data xrefs (including `0x007E2528`) but not a standard stock SW paradrop handler xref.
- `search_functions Mission_ParaDrop` returns only `0x004155F0` and `0x004157C0`.
- `read_memory 0x0041BEE0` shows tiny return stubs, not a decompiler-recognized mission function.
- Mission `0x1F`/31 table xref points to `0x004157C0`, not `0x0041BEE0`.

Active in YR: No for standard paradrop.

### 3.6 Carrier removal / leaving map

Material finding: the mission handlers do not directly delete the carrier. The verified removal point in this slice is generic `AircraftClass::AI`.

Evidence:
- `AircraftClass::AI @ 0x00414BB0` checks projected/current cell playfield state through `MapClass__Is_Cell_In_Playfield` / `Cell_in_bounds_check`.
- When outside and the vtable `+0x4DC` predicate allows it, AircraftClass::AI calls vtable `+0xF8`.

Active in YR: Yes. Exact semantic names of `+0x4DC/+0xF8` are deferred to generic cleanup docs.

## 4. INI Keys

| Key | Stock YR value | Use in this slice | Active in YR |
|---|---:|---|---|
| `[General] ParadropRadius` | `1024` | Read by Open/Rescue through `Rules+0x54C`; standard SW range gate. | Yes |
| `[ParaDropWeapon] Range` | `4` | Used by ParaDropApproach/Overfly sibling handlers for reveal/fire range. | Conditional - only if those missions are assigned |
| `[ParaDropWeapon] ROF` | `130` | Parsed, but not a mission-transition source in this slice. Cadence belongs to slot 1. | Yes as weapon data |
| `[PDPLANE] Primary` | `ParaDropWeapon` | Supplies weapon range for sibling Approach/Overfly. | Yes |
| `[PDPLANE] MoveToShroud` | `yes` | Relevant to off-map handling; exact consumer outside this slice. | Yes |
| `[PDPLANE] Landable` | `no` | Relevant to final cleanup/crash behavior; exact effect outside this slice. | Yes |

## 5. Current Rust Implementation Status

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/sim/superweapon/paradrop.rs` | Starts the carrier in the existing `ParaDropApproach` enum variant, now used as an Open-equivalent for standard SW. | Name remains historically misleading, but behavior now models the standard Open-to-Rescue path. |
| `src/sim/aircraft/paradrop_mission.rs::tick_approach` | Suppresses threshold chute sound/fog and enters Rescue-equivalent overfly after the verified Open delay. | Rust status refreshed after the verified fix; the sibling binary Approach `0x1E` path remains a separate nonstandard path if ever modeled. |
| `src/sim/aircraft/paradrop_mission.rs::tick_overfly` | Handles Rescue-equivalent payload dropping and cooldown. | Direct boundary despawn remains a cleanup-model delta; cadence no longer relies on `ParaDropWeapon` ROF. |
| `src/sim/aircraft/mod.rs::AircraftMission` | Keeps `ParaDropApproach`/`ParaDropOverfly` variant names for compatibility. | Treat these as local names for Open/Rescue-equivalent behavior in the standard SW path, not as literal binary `0x1E/0x1F` handlers. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `SuperClass::Launch` cases 5/6 | verified | decompile `0x006CC390`; assembly call sites `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655` | none for mission arg |
| `FUN_0065E660` mission queue | verified | decompile; assembly `0x0065E6FF..0x0065E70C` | passenger construction outside scope |
| `Mission_Open @ 0x004158E0` | verified | decompile; assembly `0x0041592F..0x00415956`; xref `0x007E2504` | Drop_Payload cadence interaction belongs to slot 1 |
| `Mission_Rescue @ 0x00415960` | verified | decompile; assembly `0x0041598C..0x00415A0A`; xref `0x007E2508` | Drop_Payload internals outside scope |
| `Mission_ParaDropApproach @ 0x004155F0` | verified as live sibling | decompile; xref `0x007E2510`; assembly `0x0041565A`, `0x00415700`, `0x0041570C..0x0041577F` | standard caller not found in stock SW path |
| `Mission_ParaDropOverfly @ 0x004157C0` | verified as live sibling | decompile; xref `0x007E2514`; assembly `0x004157E3`, `0x00415859..0x004158C1` | standard caller not found in stock SW path |
| `0x0041BEE0` possible mission 31 | verified-negative for standard SW | data xrefs exist, including `0x007E2528`; memory shows tiny return stubs; standard mission `0x1F` xref points to `0x004157C0` | none |
| `FUN_005F6440` distance helper | verified | decompile; xrefs from Open/Rescue/Approach/Overfly/Attack | exact coordinate naming outside scope |
| `HouseClass::GetOppositeEdge` | verified | decompile `0x0050DAC0` | only relevant to sibling Approach/Overfly here |
| `AircraftClass::AI` off-map removal | touched-not-exhausted | decompile `0x00414BB0`; playfield-check callees | exact cleanup vtable targets |
| Current Rust | verified scan | Codegraph + `rg`/file reads | implementation not changed |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - What mission does stock SW launch give the carrier? -> Mission `0x1A`, not `0x1E`.` (evidence: `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655`)
- `[RESOLVED] OQ-02 - Does the spawner queue that mission on the aircraft? -> Yes, through vtable `+0x1E8`.` (evidence: `0x0065E6FF..0x0065E70C`)
- `[RESOLVED] OQ-03 - What is mission `0x1A`? -> `Mission_Open @ 0x004158E0`.` (evidence: xref `0x007E2504`)
- `[RESOLVED] OQ-04 - What is mission `0x1B`? -> `Mission_Rescue @ 0x00415960`.` (evidence: xref `0x007E2508`)
- `[RESOLVED] OQ-05 - Does standard SW use `ParadropRadius`? -> Yes, Open/Rescue compare distance against `Rules+0x54C`.` (evidence: `0x0041592F..0x00415940`, `0x0041598C..0x0041599D`)
- `[RESOLVED] OQ-06 - Where does `0x300` apply? -> Only in `Mission_ParaDropApproach` sibling transition to `0x1F`.` (evidence: `0x00415700`)
- `[RESOLVED] OQ-07 - Is `0x0041BEE0` mission 31? -> No for standard SW paradrop; mission `0x1F` xrefs to `0x004157C0`. `0x0041BEE0` has data xrefs but resolves to tiny return stubs, not the standard paradrop overfly handler.` (evidence: `0x007E2514`, `0x007E2528`)
- `[RESOLVED] OQ-08 - Does Open transition to Rescue? -> Yes, within `ParadropRadius` it queues `0x1B` and decrements `+0x6D3`.` (evidence: `0x00415942..0x00415956`)
- `[RESOLVED] OQ-09 - Does Rescue transition back to Open? -> Yes, when out of range and `+0x6D3 > 0`, it queues `0x1A`.` (evidence: decompile `0x00415960`)
- `[RESOLVED] OQ-10 - Does Rescue drop payload directly? -> Yes, if in range and coords are in playfield it calls Drop_Payload.` (evidence: decompile `0x00415960`)
- `[RESOLVED] OQ-11 - Do Approach/Overfly assign exit destination? -> Yes, but that is the conditional sibling path, not standard SW launch.` (evidence: `0x00415727..0x0041577F`, `0x00415859..0x004158C1`)
- `[RESOLVED] OQ-12 - How does carrier leave/despawn? -> Verified transition-level removal is generic AircraftClass::AI off-playfield cleanup call, not a mission-specific delete.` (evidence: `0x00414BB0`)
- `[RESOLVED] OQ-13 - Is current Rust aligned? -> No; Rust currently starts the SW carrier on Approach/Overfly.` (evidence: `src/sim/superweapon/paradrop.rs`, `src/sim/aircraft/paradrop_mission.rs`)
- `[DEFERRED] OQ-14 - Exact vtable names and side effects for AircraftClass::AI `+0x4DC/+0xF8`.` (category: out-of-scope; reason: generic cleanup lifecycle, not mission transition; next-step-if-pursued: use generic cleanup report or decompile vtable targets)
- `[DEFERRED] OQ-15 - Exact Drop_Payload cadence and `+0x6D3` write/read interplay.` (category: out-of-scope; reason: assigned to slot 1; next-step-if-pursued: reconcile with `PARADROP_DROP_CADENCE_GHIDRA_REPORT.md`)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock ParaDrop/AmerParaDrop spawner queues mission `0x1A` on PDPLANE. | launch assembly `0x006CD421/493/4EB/655`; spawner queue `0x0065E6FF..0x0065E70C` | mismatch | `src/sim/superweapon/paradrop.rs`; `src/sim/aircraft/mod.rs` | Start standard SW carrier in an Open-equivalent mission, not Approach. | `paradrop_launch_starts_carrier_in_open_mission_0x1a` | Do not trust the stale "initial approach mission" claim. |
| Open uses `Rules.ParadropRadius` and queues Rescue `0x1B` when in range. | `0x0041592F..0x00415956` | missing | new/updated aircraft paradrop mission code | Add Open state behavior: target/null handling, payload gate, distance <= `ParadropRadius`, decrement `+0x6D3`, queue Rescue. | `paradrop_open_queues_rescue_at_paradropradius` | Do not use hardcoded `0x300` for Open. |
| Rescue uses `Rules.ParadropRadius`, drops payload in range, and may queue Open while `+0x6D3 > 0`. | `0x0041598C..0x00415A0A` | missing/partial | aircraft paradrop mission + drop apply phase | Add Rescue state behavior and its in-range/out-of-range branches. | `paradrop_rescue_drops_in_range_and_returns_to_open_while_landingstate_positive` | Do not route standard SW drops through Overfly-only logic. |
| `Mission_ParaDropApproach` live sibling transition is hardcoded `distance <= 0x300`. | `0x00415700` | current Rust incorrectly uses `ParadropRadius` for its Approach state | `tick_approach` if kept for nonstandard path | If keeping this mission, model it separately from standard SW Open/Rescue. | `paradrop_approach_sibling_uses_0x300_not_rules_radius` | Do not merge Open and Approach thresholds. |
| `0x0041BEE0` is not the standard mission 31/post-drop exit handler. | data xrefs exist, but actual standard `0x1F` xref is `0x007E2514 -> 0x004157C0` | none if no such mission exists | `AircraftMission` enum | Do not add a post-drop mission from this address. | `paradrop_no_0041bee0_postdrop_state` | Do not create a fake exit mission from stale docs. |
| Carrier cleanup is generic AircraftClass::AI off-playfield handling. | `0x00414BB0` | Rust directly despawns in Overfly | aircraft tick/removal pipeline | Prefer generic aircraft off-map cleanup once available; document direct despawn as temporary if retained. | `paradrop_carrier_removed_by_generic_aircraft_offmap_check` | Do not attach crash/delete side effects to mission handler without generic cleanup evidence. |

### Stale Docs / Follow-up Docs

- Replace "PDPLANE is initially put on paradrop approach mission" with: "Stock ParaDrop/AmerParaDrop launch passes mission `0x1A` to the spawner; PDPLANE starts on `Mission_Open`, then cycles with `Mission_Rescue`."
- Replace "Mission 31 handler at `0x0041BEE0`" with: "Mission `0x1F` dispatch-table xref points to `Mission_ParaDropOverfly @ 0x004157C0`; `0x0041BEE0` is not a standard paradrop mission."
- Clarify: "`0x300` is the `Mission_ParaDropApproach` sibling transition threshold. Standard stock SW Open/Rescue uses `Rules.ParadropRadius`."

## 9. Negative Facts / Do Not Do

- Do not start stock SW PDPLANE on `Mission_ParaDropApproach` unless a different caller than SuperClass cases 5/6 is being modeled.
- Do not use `0x300` as the standard SW Open/Rescue drop radius.
- Do not add `0x0041BEE0` as a post-drop mission.
- Do not collapse Open/Rescue and Approach/Overfly into one state pair; their thresholds and branch structure differ.
- Do not model carrier deletion as a direct mission return; verified removal is generic AircraftClass::AI off-playfield handling.

## 10. Remaining Uncertainty

- Exact names and side effects of AircraftClass::AI vtable `+0x4DC/+0xF8` remain outside this slice.
- Exact Drop_Payload cadence belongs to slot 1 and should be reconciled with this Open/Rescue state-machine report.
- Full `FUN_004AA440` candidate ordering is not repeated here; use the spawner/edge report for that.

## 11. Proposed Rust Tests

- `paradrop_launch_starts_carrier_in_open_mission_0x1a`
- `paradrop_open_queues_rescue_at_paradropradius`
- `paradrop_rescue_drops_in_range_and_returns_to_open_while_landingstate_positive`
- `paradrop_approach_sibling_uses_0x300_not_rules_radius`
- `paradrop_no_0041bee0_postdrop_state`
- `paradrop_carrier_removed_by_generic_aircraft_offmap_check`

## Sources

- Ghidra decompile/read-only: `0x006CC390`, `0x0065E660`, `0x004158E0`, `0x00415960`, `0x004155F0`, `0x004157C0`, `0x00415A50`, `0x00415EF8`, `0x00414BB0`, `0x005F6440`, `0x0050DAC0`, `0x004AA440`.
- Ghidra xrefs/data: `0x004158E0 -> 0x007E2504`, `0x00415960 -> 0x007E2508`, `0x004155F0 -> 0x007E2510`, `0x004157C0 -> 0x007E2514`; `0x0041BEE0` has data xrefs including `0x007E2528` but is not the standard paradrop overfly handler.
- Ghidra assembly contexts: `0x006CD421`, `0x006CD493`, `0x006CD4EB`, `0x006CD655`, `0x0065E6FF..0x0065E70C`, `0x0041592F`, `0x0041598C`, `0x0041565A`, `0x00415700`, `0x004157E3`, `0x00415859`.
- Existing docs referenced: `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`, `AIRCRAFTCLASS_GHIDRA_REPORT.md`, `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`, `RULESCLASS_FIELDS.csv`, `RULESCLASS_GHIDRA_REPORT.md`, `PARADROP_DROP_CADENCE_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/aircraft/paradrop_mission.rs`, `src/sim/aircraft/mod.rs`, `src/sim/superweapon/paradrop.rs`.
