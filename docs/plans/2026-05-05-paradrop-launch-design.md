# Paradrop Superweapon Launch Pipeline Design

## Goal

Implement the full gamemd.exe paradrop superweapon launch pipeline — from a player click on the target cell to infantry calling `begin_parachute_descent` at the right altitude — at 99% observable parity with the original game.

**Scope: full Scope C** (per brainstorm 2026-05-05). Carrier aircraft spawned at house edge, two-phase mission state machine (Approach → Overfly), ROF-driven Drop_Payload with V-pattern, silent despawn at opposite edge. Hooks into the already-shipped descent state machine in [src/sim/movement/parachute_descent.rs](src/sim/movement/parachute_descent.rs).

---

## Architecture Context

### Existing infrastructure this design builds on

**Superweapon dispatch** ([src/sim/superweapon/mod.rs](src/sim/superweapon/mod.rs), [src/sim/world/world_commands.rs:917-996](src/sim/world/world_commands.rs#L917)):
- `Command::LaunchSuperWeapon { sw_type_id, target_rx, target_ry }` arrives via the standard command pipeline.
- Per-handler dispatch already exists for IronCurtain, LightningStorm, GeneticConverter, ForceShield, PsychicReveal — same shape: `pub fn launch(sim, rules, owner, rx, ry) -> bool`.
- `ParaDrop` and `AmerParaDrop` (`SuperWeaponKind` indices 5 and 6, [src/rules/superweapon_type.rs:37-40](src/rules/superweapon_type.rs#L37)) currently fall through to the "not yet implemented" arm at [world_commands.rs:982-985](src/sim/world/world_commands.rs#L982).
- SW lifecycle (charge / suspend / building-grant / ready) is fully implemented; the `[CAAIRP]` and `[AMRADR]` definitions just need their `SuperWeapon=` lines and grants land via `refresh_super_weapons_for_owner`.

**Aircraft mission FSM** ([src/sim/aircraft/mod.rs](src/sim/aircraft/mod.rs)):
- `AircraftMission` enum currently: `Idle`, `Move`, `Attack`, `Guard`, `ReturnToBase`, `Docking`, `DockedIdle`.
- `tick_aircraft_missions(sim, rules)` runs in Phase 2, after `air_movement` and before combat. Uses snapshot → process → apply → issue-commands pattern.
- Snapshot/mutation pattern lets new variants slot in cleanly without restructuring the tick.
- `air_movement::issue_air_move_command(entities, id, (rx, ry), speed)` is the destination-setting helper used by all current missions.

**Aircraft spawn at altitude** ([src/sim/world/world_spawn.rs:288-416](src/sim/world/world_spawn.rs#L288)):
- `spawn_object_at_height(type_id, owner, rx, ry, facing, z, rules)` creates aircraft with locomotor + altitude. Reusable.

**Passenger cargo** ([src/sim/passenger.rs:30-44](src/sim/passenger.rs#L30)):
- `PassengerCargo { passengers: Vec<u64>, capacity, size_limit, total_size, garrison_fire_index }`.
- Methods `board`, `disembark`, `unload_first` (FIFO via `Vec.remove(0)`).
- For paradrop N≤9 the O(n) cost is irrelevant.

**Parachute descent** ([src/sim/movement/parachute_descent.rs](src/sim/movement/parachute_descent.rs)):
- `begin_parachute_descent(entities, entity_id, drop_altitude) -> bool` is the existing entry point.
- `tick_parachute_descent` already wired into `World::advance_tick` Phase 2.
- 16 unit tests pass against gamemd Round 4 timeline.

**Tick ordering** ([src/sim/world/mod.rs:1008-1395](src/sim/world/mod.rs#L1008)):
- Phase 2 (air movement → parachute descent) runs *before* aircraft mission FSM (also Phase 2). The same tick can drop a passenger and tick its descent → so a freshly dropped infantry's first-tick `rate=0` (no movement) lines up with gamemd's first-frame ramp.

### What does NOT exist today (must build)

- ParaDrop / AmerParaDrop launch handlers.
- ParaDropApproach / ParaDropOverfly mission variants on `AircraftMission`.
- V-pattern Drop_Payload tick + math.
- `[General] ParadropRadius=`, `AmerParaDropInf/Num=`, `AllyParaDropInf/Num=`, `SovParaDropInf/Num=`, `YuriParaDropInf/Num=` parsing.
- `HouseClass.WaypointEdge` field (closest-edge selector).
- `[ParaDropWeapon] ROF` exposure on the carrier aircraft (verify what the weapon parser already produces).
- Map-edge passable cell finder (gamemd `FUN_004AA440` analog).
- Fixed-point sin/cos LUT for V-pattern trig (verify `util/fixed_math`; add 256-entry binary-angle LUT if missing).
- Silent-spawn hook for the carrier aircraft (stub now, audio/radar/AI suppression deferred).

---

## Impact Analysis

### New files (4)

- `src/sim/superweapon/paradrop.rs` — SW launch entry point.
- `src/sim/aircraft/paradrop_mission.rs` — `tick_paradrop_approach` + `tick_paradrop_overfly`.
- `src/sim/aircraft/drop_payload.rs` — V-pattern math + per-tick drop trigger.
- `src/sim/world/edge_cell.rs` — map-edge passable cell finder, N/E/S/W modes.

### Modified files (~8)

- [src/sim/aircraft/mod.rs](src/sim/aircraft/mod.rs) — `AircraftMission::ParaDropApproach` and `::ParaDropOverfly` variants; mission tick dispatch.
- [src/sim/world/world_commands.rs:982](src/sim/world/world_commands.rs#L982) — replace fall-through with `paradrop::launch`.
- [src/rules/general_rules.rs](src/rules/general_rules.rs) — parse paradrop INI keys.
- [src/sim/house_state.rs](src/sim/house_state.rs) — add `waypoint_edge: u8` field.
- [src/sim/world/world_spawn.rs](src/sim/world/world_spawn.rs) — `spawn_aircraft_silent` (stub variant).
- [src/sim/superweapon/mod.rs](src/sim/superweapon/mod.rs) — register `pub mod paradrop;`.
- `src/util/fixed_math.rs` (or sibling) — fixed-point sin/cos LUT if missing.
- [src/sim/world/mod.rs](src/sim/world/mod.rs) — no order changes; the mission tick already runs in Phase 2.

### Determinism risks

- **Edge-cell finder mode 2 (south)**: gamemd builds a candidate list (≤10 cells) then picks **random** if alternate cell is sentinel, **closest to alternate cell** otherwise. The alternate cell IS the target for paradrop, so we always hit the closest-to-target path → deterministic without needing RNG. Document explicitly.
- **V-pattern trig**: must use fixed-point LUT, not `f32::cos`. f32 desyncs across OS / compiler / glibc in lockstep MP.
- **ROF cadence**: `drop_cooldown: u16` countdown — pure integer, deterministic.
- **Multi-aircraft launch**: when one launch spawns multiple PDPLANEs (e.g. AmerParaDropInf=E1,GHOST,ENGINEER), iteration order over the `(inf_type, num)` list determines aircraft stable_id assignment order. Use the parsed array order — already deterministic.
- **Cargo retry on impassable cell**: re-insert at `passengers.insert(0, id)` to keep "same passenger retried next tick" parity. Vec ordering is deterministic.

### Snapshot/state hash impact

- New `AircraftMission` variants auto-serialize via serde derive on the enum.
- `waypoint_edge: u8` on HouseClass adds 1 byte per house — included in state hash.
- No back-compat issues (no shipped saves).

### Lockstep MP correctness

- All sim-side math is fixed-point (LUT for trig).
- Iteration order via `EntityStore::keys_sorted()` (BTreeMap-backed) is deterministic.
- Cargo `Vec<u64>` ordering preserved across save/load.
- No floats anywhere on the sim path.

---

## Chosen Approach

**Scope C, in one design**, mirroring gamemd's three-stage pipeline:

1. **Launch dispatch** (`paradrop::launch`): validate target → bridge rejection → per-side branch on `HouseClass.side_index` → for each `(inf_type, num)` in the side's lists, spawn one PDPLANE at the house's `waypoint_edge`, load `num` infantry into cargo, set initial mission to `ParaDropApproach`.

2. **Approach → Overfly transition** (mission tick): per-tick distance to target. When `distance ≤ ParadropRadius`, fire fog-reveal + ChuteSound (once per launch via `has_revealed_fog` latch) and switch to `ParaDropOverfly`. When `distance < 0x301` (very close), redirect destination to opposite-edge cell.

3. **Drop_Payload** (inside Overfly tick): when `drop_cooldown == 0 && landing_state == 0 && cargo non-empty`: pop FIFO passenger, compute V-pattern offset (±90° from heading × 128 leptons via fixed-point LUT), spawn infantry at offset cell + aircraft.altitude, call `begin_parachute_descent`, reset `drop_cooldown = 130` and `landing_state = 5`. On impassable cell or descent-attach failure → re-insert passenger at front of cargo, retry next tick.

4. **Despawn**: when cargo empty, redirect destination to opposite-edge cell. When aircraft crosses playfield boundary, remove silently (no death anim, mirroring `Landable=no` path).

The design follows existing patterns throughout: SW handler shape from `iron_curtain.rs`, mission FSM extension pattern from existing variants, snapshot/apply mutation pattern from `tick_aircraft_missions`.

---

## Tiny-Detail Ledger

**Each item must have a home in the implementation.** Sources: `[GHIDRA addr]`, `[doc §X]` for `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`, `[ini]` for INI defaults, `[L#]` for parachute-descent ledger items.

### Launch dispatch

| # | Detail | Source | Implementation home |
|---|---|---|---|
| **P1** | Bridge-target rejection: 24-cell radius search via `Find_Nearby_Passable_Cell(bridge_filter=0)`; if alternative is also a bridge, abort entire launch | [GHIDRA 0x6CC390], [doc §3.1] | `paradrop::launch` calls bridge-rejection helper before per-side dispatch |
| **P2** | Case 5 (ParaDrop) side branch: `Side==0→Allies`, `Side==2→Yuri`, `else→Soviet` (Soviet is fallback for any non-{0,2}) | [GHIDRA case 5] | `paradrop::launch` matches on `HouseClass.side_index` |
| **P3** | Case 6 (AmerParaDrop) is single-path; American gating done by `RequiredHouses=Americans` on `[AMRADR]`, NOT in dispatcher | [GHIDRA case 6], [doc §1] | Dispatcher takes a `kind: ParaDropKind { Generic, American }` param; American skips side branch |
| **P4** | Per-side lists are parallel: spawn one PDPLANE per `Inf[i]`, carrying `Num[i]` of that one infantry type. Default `AmerParaDropInf=E1, AmerParaDropNum=8` | [doc §4], [ini rulesmd:235-251] | `paradrop::launch` iterates `zip(inf_list, num_list)` |
| **P5** | Soviet branch has no count-equality assert (other 3 do); preserve as-is | [GHIDRA case 5], [doc §5 R1] | Skip assert on Soviet path; vanilla rules satisfy it |

### Carrier aircraft spawn

| # | Detail | Source | Implementation home |
|---|---|---|---|
| **P6** | PDPLANE created at `HouseClass.WaypointEdge` (N/E/S/W). Edge cell from map-edge finder | [GHIDRA 0x65E660] | `edge_cell::find_passable_at_edge(edge, target)` |
| **P7** | Edge encoding: `0=N, 1=E, 2=S, 3=W` | [doc §12] | `edge_cell::Edge` enum |
| **P8** | Edge mode 2 (south) is asymmetric: candidate list ≤10 cells, **closest-to-alternate** (paradrop always passes target as alternate, so always closest-to-target — deterministic, no RNG needed in our context) | [doc §15] | `edge_cell::find_passable_at_edge` mode 2 path |
| **P9** | Spawn coord: `(edge_cell.x*256 + 128, edge_cell.y*256 + 128, z=0)`. Aircraft ascends to cruise altitude via locomotor | [GHIDRA 0x65E660] | `paradrop::spawn_pdplane` passes z=0 to spawner |
| **P10** | Mission set BEFORE Unlimbo: `mission = ParaDropApproach`, `destination = target_cell` | [GHIDRA 0x65E660] | Set both fields on entity then call `air_movement::issue_air_move_command` |
| **P11** | Spawn wrapped in `g_MapEditorMode++/--` to suppress: spawn voice, radar ping, fog "newly seen", AI hooks | [doc §24] | `spawn_aircraft_silent` — stub for v1 (logs intent, hookup deferred); flagged in known parity drift |
| **P12** | After Unlimbo: load N infantry into cargo. Each carries `IsParachuted=1` flag (gamemd) — parity-equivalent in our model is "passenger in PDPLANE cargo" — no separate flag needed | [GHIDRA 0x65E660] | `PassengerCargo::board(infantry_id, size)` per loaded passenger |

### Mission_ParaDropApproach

| # | Detail | Source | Implementation home |
|---|---|---|---|
| **P13** | 3D distance to target via building-padded distance | [GHIDRA 0x4155F0] | `paradrop_mission::approach_distance(aircraft, target_rx, target_ry)` — start with Chebyshev × 256 leptons; flag if parity drift visible |
| **P14** | When `distance ≤ ParadropRadius` (1024 leptons default): fire fog-reveal + ChuteSound; ONCE per launch via latch | [GHIDRA 0x4155F0], [doc §3.3] | `ParaDropApproach { ..., has_revealed_fog: bool }` latches after first trigger |
| **P15** | When `distance < 0x301` (=769 leptons ≈ 3 cells): also redirect destination to opposite-edge cell | [GHIDRA 0x4155F0] | `paradrop_mission::tick_approach` checks both thresholds; the close threshold sets `exit_redirect = true` |
| **P16** | At `distance ≤ ParadropRadius`: transition to `ParaDropOverfly`. P14 (fog/sound) and P15 (close-exit-redirect) are distinct triggers within the broader overfly threshold | [doc §3.3] | One transition: ParaDropApproach → ParaDropOverfly with `exit_rx/ry` populated and `drop_cooldown = 0` (ready to drop on next tick) |
| **P17** | If target invalid or cargo empty mid-approach: clear destination, transition to Idle | [GHIDRA 0x4155F0] | `tick_approach` early-exit |
| **P18** | Reschedule cadence in gamemd is `Rules+0x290` ticks; in our tick-every-frame model this is a no-op (we run the handler every tick) | [doc §3.3] | No-op — full-tick cadence |

### Mission_ParaDropOverfly

| # | Detail | Source | Implementation home |
|---|---|---|---|
| **P19** | When cargo empty: redirect destination to `opposite_edge` cell. Aircraft flies out via locomotor | [GHIDRA 0x4157C0] | `tick_overfly` checks cargo, calls `air_movement::issue_air_move_command` to exit cell |
| **P20** | Despawn: silent at opposite edge (`Landable=no` path skips crash anim) | [doc §20] | When PDPLANE crosses playfield boundary AND mission == ParaDropOverfly AND cargo empty: `entity.dying = true` without explosion anim |
| **P21** | Drop trigger NOT in this handler — it's in the `Fire_At` gate, driven by `[ParaDropWeapon] ROF=130` | [GHIDRA 0x415EF8] | `tick_overfly` calls `drop_payload::try_drop` when `drop_cooldown == 0 && landing_state == 0 && cargo non-empty` |

### Drop cadence

| # | Detail | Source | Implementation home |
|---|---|---|---|
| **P22** | Cadence = `[ParaDropWeapon] ROF = 130` frames between drops; weapon is dummy (Damage/Range/Projectile unused) | [doc §3.5], [ini] | Read from `weapon.rof` if parser exposes it; verify in /write-plan |
| **P23** | `LandingState = 5` after drop; decremented per tick by sibling missions; gates back-to-back drops within 5 ticks. With ROF=130 ≫ 5 it's mostly a no-op safety; mirror it for parity | [doc §18] | `ParaDropOverfly { ..., landing_state: u8 }` — set to 5 on drop, decrement each tick, gate Fire_At on `== 0` |

### Drop_Payload (V-pattern)

| # | Detail | Source | Implementation home |
|---|---|---|---|
| **P24** | `passenger = Pop_Passenger(aircraft)` — FIFO from cargo head | [GHIDRA 0x473430] | `cargo.unload_first()` (already exists) |
| **P25** | `PayloadCount--` (post-decrement); side parity: even → CW 90° (RIGHT), odd → CCW 90° (LEFT). Initial 8 → drop seq: L, R, L, R, L, R, L, R (first drop is LEFT) | [GHIDRA 0x415C60], [doc §3.6] | `payload_count: u8` on overfly state; decrement BEFORE testing parity; `if (payload_count & 1) == 0 → CW` |
| **P26** | V-pattern radius = **128 leptons** (= 0.5 cell). Constant at `0x7E2808` | [doc §14 R2] | `const V_PATTERN_RADIUS_LEPTONS: i32 = 128;` |
| **P27** | Angle conversion: `theta_rad = (drop_angle - 0x3FFF) × (-2π/65536)`. `drop_angle = facing ± 0x3FFF`. The constant at `0x7E2810` is `-2π/65536` | [doc §4 R1, §14 R2] | Fixed-point binary-angle directly indexed into 256-entry sin LUT — no need for the radian conversion in our impl as long as the LUT input is binary-angle |
| **P28** | Drop offset: `dx = sin(theta) × 128`, `dy = -cos(theta) × 128` (engine inverts Y) | [GHIDRA 0x415C60] | `drop_payload::v_offset(facing: u16, side: V_Side) -> (dx, dy)` returning leptons |
| **P29** | If drop cell impassable: re-add passenger to cargo HEAD, restore `payload_count`, return without consuming `drop_cooldown`. Same passenger retried next Fire_At tick with new heading | [GHIDRA 0x415C60], [doc §3.6] | `cargo.passengers.insert(0, passenger_id); payload_count += 1;` and skip cooldown reset |
| **P30** | On successful drop: passenger `Unlimbo(drop_pos)`, then ChuteSound, `LandingState = 5`, `LastDropFrame = current_frame` | [GHIDRA 0x415C60] | After `begin_parachute_descent` returns true: emit ChuteSound event, set `landing_state = 5`, store `last_drop_frame` (optional) |
| **P31** | After Unlimbo: paradropped infantry use base Walk locomotor + parachute-descent override (per 2026-05-05 CORRECTION). Visual chute is `Object+0x88` PARACH Anim — out of scope (D4) | [doc CORRECTION] | Call `begin_parachute_descent(entities, infantry_id, drop_altitude)` — the existing `OverrideKind::Parachute` already swaps locomotor layer to Air for descent |

### Descent attach (link to existing module)

| # | Detail | Source | Implementation home |
|---|---|---|---|
| **P32** | Drop altitude = aircraft's current `locomotor.altitude` at the moment Drop_Payload fires (~`flight_level` = 1500 leptons default) | [parachute-descent design] | `let drop_altitude = aircraft.locomotor.altitude;` then `begin_parachute_descent(entities, id, drop_altitude)` |
| **P33** | Descent ramp 0,−1,−2,−3,−3 → integer-tick math; landing inclusive `altitude ≤ 0`. Already implemented and tested | [L1–L13, L16] | No work — just call the existing entry point |
| **P34** | **L17 quirk decision**: gamemd's `InfantryClass::Unlimbo` always returns success even on internal failure. **DEVIATE.** Our `begin_parachute_descent` returns bool; on `false`, treat as P29 path: re-insert passenger to cargo head, restore `payload_count`, retry next tick. Cleaner, semantically tighter, player-equivalent | [doc CORRECTION + Round 4 L17] | `drop_payload::try_drop` returns `DropResult::Success | DropResult::ImpassableRetry | DropResult::AttachFailedRetry`; both retry variants share the re-insert-to-front path |

### Per-side dispatch + INI parsing

| # | Detail | Source | Implementation home |
|---|---|---|---|
| **P35** | Multi-type form: `AmerParaDropInf=E1,GHOST,ENGINEER` + `AmerParaDropNum=6,6,6` spawns 3 PDPLANEs (one per type) | [GHIDRA loop], [ini comment] | Parse as `Vec<(InternedId, u32)>` after zip |
| **P36** | Default per-side counts: Amer=8, Allies=6, Sov=9, Yuri=6 | [ini] | `general_rules` defaults |
| **P37** | `ParadropRadius` default = 1024 leptons | [ini], [doc §4] | `general_rules.paradrop_radius: i32`, default 1024 |
| **P38** | Each PDPLANE launches independently — gamemd loops `for i: spawn_pdplane(inf[i], num[i])`. They depart same edge same tick → arrive at slightly different times due to per-aircraft tick scheduling. Our impl is deterministic; jitter source differs but observable spread is similar | [GHIDRA case 5] | `paradrop::launch` loops over `(inf_type, num)` and spawns N independent PDPLANEs |

### Known parity drifts (deferred / accepted)

| # | Drift | Why deferred | Frequency / visibility |
|---|---|---|---|
| **D1** | Mission 31 (`0x1F`, post-paradrop exit) absorbed into ParaDropOverfly's cargo-empty branch — loses the IsStrafe forced-flyby nuance during the very-close-distance exit phase | Separate mission adds complexity for a sub-second visual nuance | Triggers every paradrop launch; visible only if zoomed on a single PDPLANE during last-second exit. LOW visibility |
| **D2** | Multi-PDPLANE launches: gamemd has slight per-aircraft scheduling jitter; ours is deterministic ordering | Lockstep MP requirement | Triggers when launch list has >1 entry; only AmerParaDrop default is 1-entry (`E1`). Most launches: zero impact |
| **D3** | Carryall sibling paradrop paths (`Mission_Open`, `Mission_Rescue`) — out of scope | Separate feature (Carryall delivery) | N/A — different system |
| **D4** | Visible chute sprite (PARACH Anim above body) — out of scope | Needs attached-anim infrastructure brainstorm | Triggers every paradrop; HIGH visibility — players will see infantry falling without a parachute. **Must address before "shipping-quality"** |
| **D5** | Silent spawn (audio/radar/AI suppression on PDPLANE creation) — stubbed via `spawn_aircraft_silent` but suppression not yet wired | Cross-cutting refactor across audio/radar/AI systems | Triggers every paradrop launch; player hears "VEHICLE READY" voice + sees radar ping for the carrier plane. **Must address before "shipping-quality"** — track as separate g_MapEditorMode-equivalent task |
| **D6** | `RequiredHouses=Americans` on `[AMRADR]` not enforced — out of scope | Cross-cutting SW grant gate, affects all American-locked SWs | Triggers when non-American player builds AMRADR; vanilla rules prevent it via faction-locked tech tree, so vanilla play is unaffected. Modders may notice |

**The 99%-parity bar is met by P1–P38 with D1, D2, D3 accepted as known drifts. D4 and D5 are flagged as MUST-ADDRESS before this feature is "shipping-quality" — this design lands the launch pipeline; D4/D5 are tracked as immediate follow-ups.**

---

## Design

### Components

#### 1. `src/sim/superweapon/paradrop.rs` — Launch handler

```rust
//! Paradrop SW launch handler — dispatches to per-side infantry lists,
//! spawns one PDPLANE per (infantry type, count) entry, loads cargo,
//! sets initial mission to ParaDropApproach.
//!
//! Mirrors gamemd.exe SuperClass::Launch cases 5 (ParaDrop, side-branched)
//! and 6 (AmerParaDrop, always American config).

use crate::rules::ruleset::RuleSet;
use crate::sim::intern::InternedId;
use crate::sim::world::Simulation;

#[derive(Debug, Clone, Copy)]
pub enum ParaDropKind {
    Generic,   // case 5 — branches on HouseClass.side_index
    American,  // case 6 — always uses Amer config
}

/// Launch entry point. Returns true if any aircraft was spawned.
pub fn launch(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    target_rx: u16,
    target_ry: u16,
    kind: ParaDropKind,
) -> bool {
    // 1. Bridge rejection (P1)
    let (target_rx, target_ry) = match resolve_drop_target(sim, target_rx, target_ry) {
        Some(t) => t,
        None => return false,  // bridge-only target, no passable alternative
    };

    // 2. Pick per-side list (P2, P3)
    let lists = match kind {
        ParaDropKind::American => &rules.general.amer_paradrop_list,
        ParaDropKind::Generic => match sim.house_side(owner) {
            HouseSide::Allies => &rules.general.ally_paradrop_list,
            HouseSide::Yuri   => &rules.general.yuri_paradrop_list,
            _                  => &rules.general.sov_paradrop_list,  // fallback (P2)
        },
    };

    // 3. Find house's waypoint edge + edge cell
    let edge = sim.house_waypoint_edge(owner);
    let edge_cell = match crate::sim::world::edge_cell::find_passable_at_edge(
        &sim.map, edge, (target_rx, target_ry),
    ) {
        Some(c) => c,
        None => return false,
    };

    // 4. Spawn one PDPLANE per (inf_type, num) entry (P4, P35, P38)
    let mut spawned = 0;
    for (inf_type, num) in lists.iter() {
        if spawn_pdplane(sim, rules, owner, edge_cell, target_rx, target_ry, *inf_type, *num) {
            spawned += 1;
        }
    }
    spawned > 0
}

fn spawn_pdplane(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: InternedId,
    edge_cell: (u16, u16),
    target_rx: u16, target_ry: u16,
    inf_type: InternedId,
    num: u32,
) -> bool {
    // Silent spawn at edge cell (P11 — stubbed for v1)
    let pdplane_type = rules.general.paradrop_aircraft_type;  // PDPLANE
    let pdplane_id = match crate::sim::world::world_spawn::spawn_aircraft_silent(
        sim, pdplane_type, owner, edge_cell.0, edge_cell.1, /*facing*/ 0, /*z*/ 0,
    ) {
        Some(id) => id,
        None => return false,
    };

    // Load cargo (P12)
    let inf_size = rules.object(sim.interner.resolve(inf_type)).map(|o| o.size).unwrap_or(1);
    let pdplane = sim.entities.get_mut(pdplane_id).unwrap();
    let cargo = pdplane.passenger_role.transport_cargo_mut().expect("PDPLANE must have cargo");
    for _ in 0..num {
        let inf_id = match crate::sim::world::world_spawn::spawn_passenger_into_cargo(
            sim, inf_type, owner,
        ) {
            Some(id) => id,
            None => break,
        };
        cargo.board(inf_id, inf_size);
    }

    // Set initial mission (P10)
    pdplane.aircraft_mission = Some(AircraftMission::ParaDropApproach {
        target_rx, target_ry,
        has_revealed_fog: false,
    });
    crate::sim::movement::air_movement::issue_air_move_command(
        &mut sim.entities, pdplane_id, (target_rx, target_ry), pdplane_speed,
    );
    true
}
```

#### 2. `src/sim/aircraft/mod.rs` — AircraftMission extension

```rust
pub enum AircraftMission {
    // ... existing variants ...

    /// Carrier aircraft flying in toward paradrop target.
    /// Transitions to ParaDropOverfly when distance <= ParadropRadius.
    ParaDropApproach {
        target_rx: u16,
        target_ry: u16,
        /// Latched true after fog-reveal + ChuteSound fire (P14).
        has_revealed_fog: bool,
    },

    /// Carrier aircraft over the drop zone, dispensing payload.
    /// Transitions to silent despawn when cargo empty + at exit cell.
    ParaDropOverfly {
        /// Opposite-edge cell to fly to once cargo is empty.
        exit_rx: u16,
        exit_ry: u16,
        /// Ticks until next drop allowed (ROF=130 cadence, P22).
        drop_cooldown: u16,
        /// 5-tick mutex between drops (LandingState, P23).
        landing_state: u8,
        /// Decrements per drop; parity drives V-pattern side (P25).
        payload_count: u8,
    },
}
```

#### 3. `src/sim/aircraft/paradrop_mission.rs` — Per-tick handlers

```rust
//! Paradrop carrier-aircraft mission handlers — Approach + Overfly.

pub fn tick_approach(
    snap: &MissionSnap,
    sim: &Simulation,
    rules: &RuleSet,
    target_rx: u16, target_ry: u16,
    has_revealed_fog: bool,
) -> ParaDropMissionMutation {
    let aircraft = sim.entities.get(snap.id).unwrap();
    let dist = approach_distance(aircraft, target_rx, target_ry);
    let radius = rules.general.paradrop_radius;  // 1024 default

    let mut m = ParaDropMissionMutation::keep();

    // P14: fog reveal + sound (latched once)
    if dist <= radius && !has_revealed_fog {
        m.fire_fog_reveal = true;
        m.play_chute_sound = true;
        m.new_mission_inplace = Some(AircraftMission::ParaDropApproach {
            target_rx, target_ry, has_revealed_fog: true,
        });
    }

    // P16: transition to Overfly at the same threshold
    if dist <= radius {
        let exit = sim.house_opposite_edge_cell(aircraft.owner);
        let payload = aircraft.transport_cargo_count() as u8;
        m.new_mission_inplace = Some(AircraftMission::ParaDropOverfly {
            exit_rx: exit.0, exit_ry: exit.1,
            drop_cooldown: 0,
            landing_state: 0,
            payload_count: payload,
        });
    }

    // P17: target invalid or cargo empty mid-approach
    if aircraft.transport_cargo_count() == 0 {
        m.new_mission_inplace = Some(AircraftMission::Idle);
        m.clear_destination = true;
    }

    m
}

pub fn tick_overfly(
    snap: &MissionSnap,
    sim: &Simulation,
    rules: &RuleSet,
    state: OverflyState,
) -> ParaDropMissionMutation {
    let aircraft = sim.entities.get(snap.id).unwrap();
    let mut m = ParaDropMissionMutation::keep();

    // P23: landing_state countdown
    let landing_state = state.landing_state.saturating_sub(1);
    let drop_cooldown = state.drop_cooldown.saturating_sub(1);

    let cargo_empty = aircraft.transport_cargo_count() == 0;

    // P19: cargo empty → redirect to exit
    if cargo_empty {
        if !aircraft_at(aircraft, state.exit_rx, state.exit_ry) {
            m.move_to = Some((state.exit_rx, state.exit_ry));
        } else {
            // P20: silent despawn
            m.silent_despawn = true;
        }
    }

    // P21: drop trigger
    let can_drop = !cargo_empty
        && drop_cooldown == 0
        && landing_state == 0;
    if can_drop {
        m.try_drop = true;  // signals apply phase to call drop_payload::try_drop
    }

    m.new_mission_inplace = Some(AircraftMission::ParaDropOverfly {
        exit_rx: state.exit_rx, exit_ry: state.exit_ry,
        drop_cooldown, landing_state,
        payload_count: state.payload_count,
    });
    m
}
```

#### 4. `src/sim/aircraft/drop_payload.rs` — V-pattern math

```rust
//! Drop_Payload — V-pattern math for paratroop ejection.

use crate::util::fixed_math::{SimFixed, sin_binary_angle, cos_binary_angle};

pub const V_PATTERN_RADIUS_LEPTONS: i32 = 128;  // P26
pub const PARADROP_DROP_INTERVAL_TICKS: u16 = 130;  // P22 — fallback if weapon.rof unavailable
pub const LANDING_STATE_RESET: u8 = 5;  // P23

pub enum DropResult {
    Success,
    ImpassableRetry,       // P29
    AttachFailedRetry,     // P34
}

/// V-pattern offset in leptons. side derived from payload_count parity.
pub fn v_offset(facing_binary: u16, payload_count_post_dec: u8) -> (i32, i32) {
    // P25: even → CW 90° (RIGHT), odd → CCW 90° (LEFT)
    let drop_angle: u16 = if (payload_count_post_dec & 1) == 0 {
        facing_binary.wrapping_add(0x3FFF)  // CW 90°
    } else {
        facing_binary.wrapping_sub(0x3FFF)  // CCW 90°
    };
    // P27, P28: theta = (drop_angle - 0x3FFF) × (-2π/65536)
    // Equivalent in binary-angle LUT space: directly index sin/cos with drop_angle - 0x3FFF.
    let lookup_angle = drop_angle.wrapping_sub(0x3FFF);
    let dx = sin_binary_angle(lookup_angle).saturating_mul(V_PATTERN_RADIUS_LEPTONS);
    let dy = -cos_binary_angle(lookup_angle).saturating_mul(V_PATTERN_RADIUS_LEPTONS);  // Y inverted
    (dx, dy)
}

/// Pop one passenger, compute V-offset, attempt drop. Returns DropResult.
pub fn try_drop(
    sim: &mut Simulation,
    rules: &RuleSet,
    aircraft_id: u64,
) -> DropResult {
    let aircraft = sim.entities.get(aircraft_id).unwrap();
    let facing = aircraft.facing as u16 * 256;  // u8 facing → binary u16
    let altitude = aircraft.locomotor.as_ref().map(|l| l.altitude).unwrap_or(SIM_ZERO);
    let aircraft_rx = aircraft.position.rx;
    let aircraft_ry = aircraft.position.ry;

    // P24: pop FIFO
    let cargo = aircraft.transport_cargo_mut();
    let passenger_id = match cargo.unload_first() {
        Some(id) => id,
        None => return DropResult::Success,  // shouldn't reach — gated by cargo_empty
    };

    // P25: post-decrement parity (caller decrements payload_count after)
    let payload_count_post = aircraft.payload_count() - 1;
    let (dx, dy) = v_offset(facing, payload_count_post);

    // Compute drop cell
    let drop_rx = (aircraft_rx as i32 + dx / 256).clamp(0, MAP_MAX_RX as i32) as u16;
    let drop_ry = (aircraft_ry as i32 + dy / 256).clamp(0, MAP_MAX_RY as i32) as u16;

    // P29: impassable check
    if !sim.map.cell_passable(drop_rx, drop_ry) {
        cargo.passengers.insert(0, passenger_id);  // re-insert at HEAD
        return DropResult::ImpassableRetry;
    }

    // Spawn infantry at drop cell + altitude (P31, P32)
    let inf_type = sim.entities.get(passenger_id).unwrap().type_ref;
    let owner = aircraft.owner;
    sim.spawn_at_cell_with_altitude(passenger_id, drop_rx, drop_ry, altitude);

    // P33, P34: attach descent
    if !crate::sim::movement::parachute_descent::begin_parachute_descent(
        &mut sim.entities, passenger_id, altitude,
    ) {
        // L17 deviation: re-insert + retry
        cargo.passengers.insert(0, passenger_id);
        return DropResult::AttachFailedRetry;
    }

    // P30: ChuteSound
    sim.sound_events.push(SimSoundEvent::ChuteSound {
        rx: drop_rx, ry: drop_ry,
    });
    DropResult::Success
}
```

#### 5. `src/sim/world/edge_cell.rs` — Map-edge passable cell finder

```rust
//! Map-edge passable cell finder. Mirrors gamemd FUN_004AA440.
//! Modes: 0=N, 1=E, 2=S, 3=W (P7).

#[derive(Debug, Clone, Copy)]
pub enum Edge { North, East, South, West }

/// Find a passable cell along the given map edge, biased toward `target`.
/// Mode 2 (south) is asymmetric in gamemd (P8): builds candidate list ≤10,
/// picks closest-to-target. We always pass target as the alternate, so
/// always closest-to-target → no RNG needed.
pub fn find_passable_at_edge(
    map: &MapBounds,
    edge: Edge,
    target: (u16, u16),
) -> Option<(u16, u16)> {
    match edge {
        Edge::North => scan_linear_first(map, edge, target),
        Edge::East  => scan_linear_first(map, edge, target),
        Edge::West  => scan_linear_first(map, edge, target),
        Edge::South => scan_candidates_closest(map, target),  // mode 2 special
    }
}
```

#### 6. `src/rules/general_rules.rs` — INI parsing extension

```rust
pub struct GeneralRules {
    // ... existing ...
    pub paradrop_radius: i32,                              // P37 default 1024
    pub paradrop_aircraft_type: InternedId,                // PDPLANE
    pub amer_paradrop_list: Vec<(InternedId, u32)>,        // (inf_type, num) pairs
    pub ally_paradrop_list:  Vec<(InternedId, u32)>,
    pub sov_paradrop_list:   Vec<(InternedId, u32)>,
    pub yuri_paradrop_list:  Vec<(InternedId, u32)>,
    pub parachute_max_fall_rate: i32,  // already exists, default -3
}
```

Parser zips `XxxParaDropInf` and `XxxParaDropNum` (per side) — assert equal length except on Soviet (P5).

#### 7. `src/sim/house_state.rs` — WaypointEdge

```rust
pub struct HouseState {
    // ... existing ...
    pub waypoint_edge: u8,  // 0=N, 1=E, 2=S, 3=W (P7)
}
```

Computed at game start by closest-edge-of-bounds algorithm (gamemd `HouseClass::DetermineEdge`, [doc §12]).

### Interfaces / Contracts

**Public API:**
- `paradrop::launch(sim, rules, owner, rx, ry, kind) -> bool` — entry from `world_commands.rs` dispatch.
- `AircraftMission::ParaDropApproach { ... }` and `::ParaDropOverfly { ... }` — extend existing enum.
- `edge_cell::find_passable_at_edge(map, edge, target) -> Option<(u16, u16)>`.
- `drop_payload::try_drop(sim, rules, aircraft_id) -> DropResult`.

**Required upstream:**
- `general_rules` parses paradrop INI keys.
- `HouseState.waypoint_edge` populated at game start.
- `util/fixed_math::sin_binary_angle / cos_binary_angle` — fixed-point binary-angle trig LUT (verify exists; add if not).
- `[ParaDropWeapon] ROF=130` reachable as `weapon.rof` for the aircraft's primary weapon (verify; fall back to `PARADROP_DROP_INTERVAL_TICKS` const).
- `world_spawn::spawn_aircraft_silent` (stub: same as `spawn_object_at_height` but tagged for later suppression hookup).
- `world_spawn::spawn_passenger_into_cargo` (factory that creates an infantry entity in "limboed" state — present but not at any position; follows existing transport boarding pattern).

**Downstream consumers:**
- `tick_aircraft_missions` calls `paradrop_mission::tick_approach` / `tick_overfly` for the new variants.
- Snapshot serializer auto-includes new mission variants and `waypoint_edge`.
- `parachute_descent::tick_parachute_descent` (already wired) handles dropped infantry's descent.

### Data Flow

```
PLAYER CLICK on tactical map with charged Paradrop SW cursor
  ↓
Command::LaunchSuperWeapon { sw_type_id, target_rx, target_ry }
  ↓ (queued, executes on its assigned tick)
world_commands.rs dispatch: SuperWeaponKind::ParaDrop | AmerParaDrop
  ↓
paradrop::launch(sim, rules, owner, rx, ry, kind)
  ├→ resolve_drop_target()        [P1 bridge rejection]
  ├→ pick per-side list           [P2, P3]
  ├→ edge_cell::find_passable_at_edge(map, waypoint_edge, target)  [P6, P8]
  └→ for each (inf_type, num):
       └→ spawn_pdplane()
            ├→ world_spawn::spawn_aircraft_silent()   [P11 — stubbed]
            ├→ for num passengers: spawn_passenger_into_cargo() + cargo.board()  [P12]
            ├→ entity.aircraft_mission = ParaDropApproach { target, has_revealed_fog: false }  [P10]
            └→ air_movement::issue_air_move_command(target)

TICK LOOP (every tick):
  Phase 2: air_movement → parachute_descent → tick_aircraft_missions
                                                ↓
       ParaDropApproach handler:
         ├→ approach_distance()                     [P13]
         ├→ if dist <= ParadropRadius && !has_revealed_fog:
         │    fog_reveal + ChuteSound, latch fog    [P14]
         ├→ if dist <= ParadropRadius:
         │    transition to ParaDropOverfly         [P16]
         └→ if cargo_empty: → Idle                  [P17]

       ParaDropOverfly handler:
         ├→ drop_cooldown -= 1                       [P22]
         ├→ landing_state -= 1                       [P23]
         ├→ if cargo_empty:
         │    move toward exit_rx/ry                 [P19]
         │    if at exit: silent_despawn             [P20]
         └→ if can_drop (cooldown=0, landing=0, cargo non-empty):
              drop_payload::try_drop()
                ├→ pop FIFO passenger                [P24]
                ├→ payload_count -= 1                [P25]
                ├→ v_offset() → (dx, dy)             [P26-P28]
                ├→ if impassable: re-insert head, retry  [P29]
                ├→ spawn infantry at offset cell + altitude  [P31, P32]
                ├→ begin_parachute_descent()         [P33]
                │    └→ if false: re-insert head, retry  [P34]
                └→ ChuteSound + landing_state=5      [P30]

  Phase 2 (next tick):
       parachute_descent::tick_parachute_descent
         └→ rate ramp 0,-1,-2,-3, altitude integrates  [L1-L13, L16]

  ... infantry lands when altitude <= 0 ...
       parachute_descent cleanup:
         ├→ end_override (locomotor → Walk)
         └→ animation.sequence: Paradrop → Stand   [L11]
```

### Error Handling

- **Bridge-only target with no passable alternative**: `paradrop::launch` returns false; SW dispatch caller handles (does NOT consume charge — match LightningStorm pattern).
- **No valid edge cell**: same as above.
- **PDPLANE spawn fails** (e.g., entity store full): `spawn_pdplane` returns false; partial launches (some PDPLANEs spawned, others failed) consume charge — single-failure case acceptable.
- **Cargo board fails** (capacity exceeded): break loop; aircraft launches with partial cargo. Should not happen if `[PDPLANE] Passengers=` is set high enough (verify in /write-plan).
- **Drop_Payload impassable cell**: re-insert passenger to cargo HEAD, retry next tick (P29). No charge re-consumption.
- **`begin_parachute_descent` returns false**: same as impassable — re-insert + retry (P34).
- **PDPLANE destroyed mid-mission**: standard entity death cleanup; passengers in cargo die with carrier (matches gamemd `Passenger_Death_On_Transport_Destruction`).

### Testing Strategy

**Unit tests (in each new module, no engine spinup):**

| Test | Verifies | Ledger |
|---|---|---|
| `test_v_pattern_alternates` | Sequence L,R,L,R for payload_count 8→0 | P25 |
| `test_v_pattern_radius_128` | Drop offset magnitude ≈ 128 leptons regardless of facing | P26 |
| `test_v_pattern_facing_north` | Drop with facing=0 (north): even count → +X, odd → -X | P25, P28 |
| `test_v_pattern_facing_east` | Drop with facing=0x4000 (east): even count → +Y, odd → -Y | P25, P28 |
| `test_drop_retry_on_impassable` | Cargo head restored, payload_count restored after impassable hit | P29 |
| `test_drop_retry_on_attach_fail` | Same as above for attach-fail path | P34 |
| `test_paradrop_dispatch_side_branch` | Side=0 picks Ally, =2 picks Yuri, =1 picks Sov, =3 picks Sov (fallback) | P2 |
| `test_paradrop_amer_skips_side_branch` | Kind=American always picks AmerParaDropList regardless of Side | P3 |
| `test_paradrop_bridge_rejection_finds_alternative` | Click on bridge cell → drop target shifts to nearby non-bridge | P1 |
| `test_paradrop_bridge_only_aborts` | Click on bridge with no non-bridge nearby → launch returns false | P1 |
| `test_edge_cell_north_linear` | North edge mode finds first passable cell on top row | P7 |
| `test_edge_cell_south_closest_to_target` | South edge mode picks closest-to-target from candidates | P8 |
| `test_paradrop_full_flow_e2e` | End-to-end: launch → PDPLANE spawn → cargo loaded → mission set | All launch ledger items |
| `test_paradrop_overfly_drops_at_cooldown_zero` | Drop fires when drop_cooldown reaches 0 | P22 |
| `test_paradrop_overfly_landing_state_gates_back_to_back` | Two drop attempts in 5 ticks → only first fires | P23 |
| `test_paradrop_overfly_exit_when_cargo_empty` | Cargo empty → destination redirected to opposite edge | P19 |
| `test_paradrop_silent_despawn_at_exit` | At exit cell with empty cargo → entity.dying = true, no explosion anim | P20 |
| `test_paradrop_descent_attach_via_existing_module` | Dropped infantry has parachute_state, locomotor override = Parachute | P31, P33 |

**Integration test:**
- `test_paradrop_landing_full_descent` — spawn paradrop, advance N ticks, verify infantry lands on ground at correct cells with V-pattern offset, with the descent ramp matching gamemd timeline. Combines launch pipeline + existing parachute_descent module.

**Determinism test:**
- `test_paradrop_lockstep_two_runs` — run paradrop launch twice with same seed, verify identical entity positions, mission states, cargo contents at every tick of the descent.

---

## Architectural Decisions

### Patterns followed

- **SW handler shape**: matches `iron_curtain.rs`, `lightning_storm.rs`, `force_shield.rs` — `pub fn launch(sim, rules, owner, rx, ry) -> bool`.
- **Mission FSM extension**: new variants on existing `AircraftMission` enum, dispatched in `tick_aircraft_missions` snapshot/apply pattern. No parallel FSM.
- **Cargo reuse**: existing `PassengerCargo` instead of paradrop-specific cargo type.
- **Snapshot serialization**: auto via serde derive on enum + struct fields.
- **Determinism**: integer math + fixed-point trig LUT + `EntityStore::keys_sorted()` iteration.
- **Module size**: each new file <500 lines including tests.
- **Dependency direction**: sim → rules + map only; never depends on render/audio/UI.

### Patterns deviated from (with reason)

- **L17 quirk**: gamemd's always-success Unlimbo silent-failure NOT mirrored. Our path returns `bool` from `begin_parachute_descent` and retries on false. Cleaner, semantically tighter, player-equivalent. (P34, decision S10.)
- **Mission 31 (post-paradrop exit)**: gamemd has a separate post-drop exit mission with `IsStrafe=1` flyby behavior; we absorb its semantics into ParaDropOverfly's cargo-empty branch. (D1 — accepted parity drift.)
- **`Mission_Open` / `Mission_Rescue` (Carryall paths)**: out of scope. (D3.)

### Tech debt introduced

- **Silent spawn stub** (D5): `spawn_aircraft_silent` is structurally identical to normal spawn for v1; audio/radar/AI suppression deferred to a separate `g_MapEditorMode-equivalent` task. Tracked as MUST-ADDRESS-BEFORE-SHIPPING.
- **Visible chute sprite missing** (D4): infantry descend without a visible parachute sprite. Tracked as MUST-ADDRESS-BEFORE-SHIPPING; needs attached-anim infrastructure brainstorm first.

### Tech debt acknowledged (out of scope)

- D1 (mission 31 IsStrafe nuance) — LOW visibility, accepted.
- D2 (multi-PDPLANE scheduling jitter) — zero impact for single-entry default lists.
- D3 (Carryall sibling paths) — separate feature.
- D6 (`RequiredHouses=Americans` enforcement) — cross-cutting SW concern, separate task.

---

## Alternatives Considered

1. **Sub-scope A (no carrier aircraft, infantry materialize at altitude)**. Rejected: violates the parity bar visibly. Player sees infantry rain from a clear sky with no plane, no V-pattern, no ChuteSound cadence, no fog reveal. Multiple distinguishable artifacts in the first match.

2. **Sub-scope B (aircraft + drop, no missions)**. Rejected: structurally awkward. The `ParadropRadius`-gated approach→overfly transition is what times the first drop. Without it, "fly to target → dump all" cadence is wrong. Implementing B realistically requires half-implementing the missions, then ripping out for C.

3. **Separate `ParaDropMission` FSM (parallel to AircraftMission)**. Rejected: cuts against the project convention. Existing `AircraftMission` enum already covers Idle/Move/Attack/Guard/RTB/Docking/DockedIdle in one type; adding two more variants is the established pattern. Parallel FSMs would require separate dispatch loops and risk diverging.

4. **Paradrop-specific cargo type (instead of reusing `PassengerCargo`)**. Rejected: `PassengerCargo` already supports board/disembark/unload_first with FIFO order. The `Vec.remove(0)` cost is irrelevant at N≤9. Avoids parallel cargo abstraction.

5. **`f32` cos/sin for V-pattern offset**. Rejected: `f32` desyncs across OS / compiler / glibc in lockstep MP even when single-machine deterministic. The CLAUDE.md "no floats in sim" rule is ironclad. Fixed-point binary-angle LUT is a one-time ~50-line addition reused everywhere.

6. **Mirror gamemd's always-success Unlimbo quirk (L17)**. Rejected: it's an internal binary artifact, not observable. Retry-on-fail is cleaner and player-equivalent.

7. **Hardcode `ROF=130` instead of parsing weapon**. Rejected: violates the INI-as-source-of-truth rule. The hookup is small (resolve `[PDPLANE] Primary=ParaDropWeapon`'s rof). Done correctly day one, modders can tune.

---

## Status

**Design approved 2026-05-05.** Ready for `/write-plan` to break into commit-sized tasks.

**Build order suggestion (for /write-plan):**
1. INI parsing — paradrop lists + `ParadropRadius` + `paradrop_aircraft_type`.
2. `HouseState.waypoint_edge` + closest-edge selector.
3. `edge_cell::find_passable_at_edge` (N/E/W linear + S closest-to-target).
4. Fixed-point sin/cos LUT (if missing in `util/fixed_math`).
5. `AircraftMission::ParaDropApproach` + `::ParaDropOverfly` variants + tick handlers (no Drop_Payload yet — just FSM transitions).
6. `drop_payload::try_drop` + V-pattern math + descent attach.
7. `paradrop::launch` SW handler + dispatch wiring in `world_commands.rs`.
8. `spawn_aircraft_silent` stub.
9. End-to-end integration test.

**Follow-up tasks (post-merge, not blocking this design):**
- D4: Visible chute sprite — needs attached-anim infrastructure brainstorm first.
- D5: Silent spawn audio/radar/AI suppression — `g_MapEditorMode-equivalent` cross-cutting task.
- D6: `RequiredHouses=Americans` enforcement — cross-cutting SW grant gate.
